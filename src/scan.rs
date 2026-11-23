use crate::html::{parse_html, resolve_url, ParsedDoc};
use crate::schema::{collect_types, extract_nap, find_business};
use crate::types::*;
use crate::util::{new_id, now_iso, public_mode, public_url_allowed};
use regex::Regex;
use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use url::Url;

const UA: &str = "Mozilla/5.0 (compatible; GeoSignal/1.0; local SEO audit; runs on your machine)";
const MAX_BYTES: u64 = 3_000_000;

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub max_pages: Option<usize>,
    pub keyword: String,
    pub city: String,
}

struct FetchResult {
    status: u16,
    final_url: String,
    text: String,
    ms: u64,
    bytes: usize,
    content_type: String,
}

struct RobotsInfo {
    found: bool,
    disallow: Vec<String>,
    sitemaps: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct SitemapInfo {
    found: bool,
    urls: usize,
    is_index: bool,
}

fn skip_ext_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\.(pdf|jpe?g|png|gif|svg|webp|avif|ico|zip|gz|mp4|mov|mp3|wav|css|js|json|xml|rss|woff2?|ttf|eot|docx?|xlsx?|pptx?)$").unwrap()
    })
}

fn skip_path_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)/(wp-admin|wp-login\.php|feed|xmlrpc\.php|cart|checkout|my-account)(/|$)").unwrap()
    })
}

fn host_key(hostname: &str) -> String {
    hostname.to_ascii_lowercase().trim_start_matches("www.").to_string()
}

fn build_agent(public: bool) -> ureq::Agent {
    let builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .redirects(if public { 0 } else { 10 })
        .user_agent(UA);
    // Public mode also filters every resolved address at connect time, so a
    // hostname cannot pass the URL check and then resolve to a private IP.
    let builder = if public {
        builder.resolver(crate::util::public_resolver)
    } else {
        builder
    };
    builder.build()
}

fn into_fetch(resp: ureq::Response, status: u16, started: Instant) -> FetchResult {
    let content_type = resp.header("content-type").unwrap_or("").to_string();
    let final_url = resp.get_url().to_string();
    let mut buffer = Vec::new();
    let _ = resp
        .into_reader()
        .take(MAX_BYTES)
        .read_to_end(&mut buffer);
    let text = String::from_utf8_lossy(&buffer).to_string();
    FetchResult {
        status,
        final_url,
        text,
        ms: started.elapsed().as_millis() as u64,
        bytes: buffer.len(),
        content_type,
    }
}

fn fetch_text(agent: &ureq::Agent, url: &str, public: bool) -> Result<FetchResult, String> {
    let started = Instant::now();
    let mut current = url.to_string();
    for _ in 0..10 {
        if public && !public_url_allowed(&current) {
            return Err("Public deployments only crawl public HTTP(S) addresses.".to_string());
        }
        let request = agent.get(&current).set(
            "accept",
            "text/html,application/xhtml+xml,text/plain;q=0.8,*/*;q=0.5",
        );
        match request.call() {
            Ok(resp) => {
                let status = resp.status();
                if public && (300..400).contains(&status) {
                    let Some(location) = resp.header("location") else {
                        return Ok(into_fetch(resp, status, started));
                    };
                    let next = Url::parse(&current)
                        .and_then(|base| base.join(location))
                        .map_err(|error| format!("Invalid redirect: {error}"))?;
                    current = next.to_string();
                    continue;
                }
                return Ok(into_fetch(resp, status, started));
            }
            Err(ureq::Error::Status(code, resp)) => return Ok(into_fetch(resp, code, started)),
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Too many redirects.".to_string())
}

fn parse_robots(text: &str) -> (Vec<String>, Vec<String>) {
    let mut disallow = Vec::new();
    let mut sitemaps = Vec::new();
    let mut applies = false;
    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some(colon) = line.find(':') else { continue };
        let key = line[..colon].trim().to_ascii_lowercase();
        let value = line[colon + 1..].trim();
        match key.as_str() {
            "user-agent" => applies = value == "*",
            "disallow" if applies => {
                if !value.is_empty() {
                    disallow.push(value.to_string());
                }
            }
            "sitemap" => {
                if !value.is_empty() {
                    sitemaps.push(value.to_string());
                }
            }
            _ => {}
        }
    }
    (disallow, sitemaps)
}

fn robots_allows(pathname: &str, disallow: &[String]) -> bool {
    let path = if pathname.is_empty() { "/" } else { pathname };
    for rule in disallow {
        if rule == "/" {
            if path == "/" {
                return true;
            }
            continue;
        }
        let mut pattern = String::from("^");
        for ch in rule.chars() {
            if ch == '*' {
                pattern.push_str(".*");
            } else {
                pattern.push_str(&regex::escape(&ch.to_string()));
            }
        }
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(path) {
                return false;
            }
        }
    }
    true
}

fn load_robots(agent: &ureq::Agent, origin: &str, public: bool) -> RobotsInfo {
    match fetch_text(agent, &format!("{}/robots.txt", origin), public) {
        Ok(fetched) => {
            if !(200..400).contains(&fetched.status)
                || fetched.content_type.to_ascii_lowercase().contains("text/html")
            {
                return RobotsInfo { found: false, disallow: Vec::new(), sitemaps: Vec::new() };
            }
            let (disallow, sitemaps) = parse_robots(&fetched.text);
            RobotsInfo { found: true, disallow, sitemaps }
        }
        Err(_) => RobotsInfo { found: false, disallow: Vec::new(), sitemaps: Vec::new() },
    }
}

fn load_sitemap(agent: &ureq::Agent, origin: &str, from_robots: &[String], public: bool) -> SitemapInfo {
    let mut candidates: Vec<String> = from_robots.to_vec();
    if candidates.is_empty() {
        candidates.push(format!("{}/sitemap.xml", origin));
        candidates.push(format!("{}/sitemap_index.xml", origin));
    }
    let loc_re = Regex::new(r"(?i)<loc>").unwrap();
    let index_re = Regex::new(r"(?i)<sitemapindex").unwrap();
    let urlset_re = Regex::new(r"(?i)<urlset").unwrap();
    for candidate in candidates.iter().take(3) {
        if let Ok(fetched) = fetch_text(agent, candidate, public) {
            if !(200..400).contains(&fetched.status) {
                continue;
            }
            let urls = loc_re.find_iter(&fetched.text).count();
            if urls > 0 || index_re.is_match(&fetched.text) || urlset_re.is_match(&fetched.text) {
                return SitemapInfo {
                    found: true,
                    urls,
                    is_index: index_re.is_match(&fetched.text),
                };
            }
        }
    }
    SitemapInfo::default()
}

fn detect_city(source: &str) -> String {
    let patterns = [
        r"(?i)\b(?:in|near|serving|serving the)\s+([A-Z][a-zA-Z.'-]+(?:\s+[A-Z][a-zA-Z.'-]+)?)",
        r"\b([A-Z][a-zA-Z.'-]+(?:\s+[A-Z][a-zA-Z.'-]+)?),\s*(?:[A-Z]{2}|[A-Z][a-z]+)\b",
    ];
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(source) {
                if let Some(candidate) = caps.get(1) {
                    let value = candidate.as_str().trim();
                    if !value.is_empty()
                        && !matches!(value.to_ascii_lowercase().as_str(), "the" | "our" | "your" | "we" | "all" | "new")
                    {
                        return value.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

fn humanize_type(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    let chars: Vec<char> = value.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_uppercase() && index > 0 && chars[index - 1].is_lowercase() {
            out.push(' ');
        }
        out.push(*ch);
    }
    let lower = out.to_ascii_lowercase();
    if lower == "local business" || lower == "localbusiness" {
        String::new()
    } else {
        lower
    }
}

fn path_of(url: &str) -> String {
    match Url::parse(url) {
        Ok(parsed) => {
            let mut path = parsed.path().to_string();
            if let Some(query) = parsed.query() {
                path.push('?');
                path.push_str(query);
            }
            path
        }
        Err(_) => "/".to_string(),
    }
}

fn audit_page(doc: &ParsedDoc, url: &str, depth: usize, fetched: &FetchResult) -> PageAudit {
    let canonical_href = doc
        .links
        .iter()
        .find(|link| link.rel.split_whitespace().any(|part| part == "canonical"))
        .map(|link| link.href.clone())
        .unwrap_or_default();
    let canonical = if canonical_href.is_empty() {
        String::new()
    } else {
        resolve_url(&canonical_href, url).unwrap_or_default()
    };

    let robots_meta = doc
        .metas
        .get("robots")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let noindex = robots_meta.contains("noindex")
        || fetched
            .text
            .to_ascii_lowercase()
            .contains("<meta name=\"robots\" content=\"noindex");

    let base_host = Url::parse(url)
        .ok()
        .map(|parsed| host_key(parsed.host_str().unwrap_or("")))
        .unwrap_or_default();

    let mut anchors: Vec<AnchorRef> = Vec::new();
    let mut internal = 0usize;
    let mut external = 0usize;
    let mut tel = 0usize;
    let mut nofollow = 0usize;

    for anchor in &doc.anchors {
        if anchor.href.to_ascii_lowercase().starts_with("tel:") {
            tel += 1;
            continue;
        }
        let Some(resolved) = resolve_url(&anchor.href, url) else { continue };
        if anchor.rel.to_ascii_lowercase().contains("nofollow") {
            nofollow += 1;
        }
        let Ok(parsed) = Url::parse(&resolved) else { continue };
        if host_key(parsed.host_str().unwrap_or("")) == base_host {
            internal += 1;
            anchors.push(AnchorRef { text: anchor.text.clone(), href: resolved });
        } else {
            external += 1;
        }
    }

    let types = collect_types(&doc.json_ld);
    let lower_types: Vec<String> = types.iter().map(|t| t.to_ascii_lowercase()).collect();
    let has_business = find_business(&doc.json_ld).is_some();
    let json_ld_blocks = doc.scripts.iter().filter(|script| script.json_ld).count();
    let extended_text = &fetched.text;

    PageAudit {
        url: url.to_string(),
        path: path_of(url),
        depth,
        status: fetched.status,
        ok: (200..400).contains(&fetched.status),
        ms: fetched.ms,
        bytes: fetched.bytes,
        content_type: fetched.content_type.clone(),
        title: doc.title.clone(),
        title_length: doc.title.chars().count(),
        meta_description: doc.metas.get("description").cloned().unwrap_or_default(),
        canonical,
        noindex,
        lang: doc.lang.clone(),
        viewport: doc.metas.contains_key("viewport"),
        h1: doc
            .headings
            .iter()
            .filter(|heading| heading.level == 1)
            .map(|heading| heading.text.clone())
            .take(5)
            .collect(),
        h2: doc
            .headings
            .iter()
            .filter(|heading| heading.level == 2)
            .map(|heading| heading.text.clone())
            .take(12)
            .collect(),
        word_count: doc.word_count,
        images: ImageStats {
            total: doc.images.len(),
            missing_alt: doc.images.iter().filter(|image| image.alt.is_none()).count(),
            no_dimensions: doc
                .images
                .iter()
                .filter(|image| {
                    !(image
                        .width
                        .as_ref()
                        .map(|w| !w.is_empty())
                        .unwrap_or(false)
                        && image
                            .height
                            .as_ref()
                            .map(|h| !h.is_empty())
                            .unwrap_or(false))
                })
                .count(),
        },
        links: LinkCounts { internal, external, tel, nofollow },
        anchors: anchors.into_iter().take(80).collect(),
        hreflang: doc
            .links
            .iter()
            .filter(|link| link.rel.contains("alternate") && link.hreflang.is_some())
            .map(|link| Hreflang {
                lang: link.hreflang.clone().unwrap_or_default(),
                href: resolve_url(&link.href, url).unwrap_or_default(),
            })
            .collect(),
        json_ld_types: types,
        json_ld_blocks,
        json_ld_parsed: doc.json_ld.len(),
        has_business_schema: has_business,
        has_breadcrumb: lower_types.iter().any(|t| t == "breadcrumblist"),
        has_breadcrumb_visible: extended_text.to_ascii_lowercase().contains("breadcrumb"),
        has_faq: lower_types.iter().any(|t| t == "faqpage" || t == "qapage"),
        has_rating: lower_types.iter().any(|t| t == "aggregaterating" || t == "review"),
        has_map_embed: Regex::new(r"(?i)google\.[a-z.]+/maps|maps\.googleapis|maps\.google\.com|openstreetmap")
            .map(|re| re.is_match(extended_text))
            .unwrap_or(false),
        gsc_verification: doc.metas.contains_key("google-site-verification"),
        nap: extract_nap(&doc.json_ld),
        text_sample: doc.text.chars().take(6000).collect(),
        blocking_scripts: doc
            .scripts
            .iter()
            .filter(|script| script.src.is_some() && !script.is_async && !script.defer)
            .count(),
    }
}

fn crawl_one(
    agent: &ureq::Agent,
    url: &str,
    depth: usize,
    base_host: &str,
    disallow: &[String],
    public: bool,
) -> Option<(PageAudit, Vec<(String, usize)>)> {
    let fetched = fetch_text(agent, url, public).ok()?;
    let content_type = fetched.content_type.to_ascii_lowercase();
    if !content_type.is_empty()
        && !content_type.contains("text/html")
        && !content_type.contains("application/xhtml")
    {
        return None;
    }
    let doc = parse_html(&fetched.text);
    let audit = audit_page(&doc, &fetched.final_url, depth, &fetched);

    let mut links: Vec<(String, usize)> = Vec::new();
    if depth < 2 {
        let skip_ext = skip_ext_re();
        let skip_path = skip_path_re();
        for anchor in &audit.anchors {
            let Ok(mut parsed) = Url::parse(&anchor.href) else { continue };
            if host_key(parsed.host_str().unwrap_or("")) != base_host {
                continue;
            }
            parsed.set_fragment(None);
            let path = parsed.path().to_string();
            if skip_ext.is_match(&path) || skip_path.is_match(&path) {
                continue;
            }
            if let Some(query) = parsed.query() {
                let lower = query.to_ascii_lowercase();
                if lower.contains("utm_") || lower.contains("replytocom") || lower.contains("preview=") {
                    continue;
                }
            }
            if !robots_allows(&path, disallow) {
                continue;
            }
            links.push((parsed.to_string(), depth + 1));
        }
    }
    Some((audit, links))
}

fn pick_business(pages: &[PageAudit]) -> Nap {
    let mut best: Option<Nap> = None;
    let mut best_score: i32 = -1;
    for page in pages {
        let nap = &page.nap;
        let score = [
            nap.name.is_some(),
            nap.street.is_some(),
            nap.locality.is_some(),
            nap.region.is_some(),
            nap.postal_code.is_some(),
            nap.phone.is_some(),
            nap.lat.is_some(),
            nap.rating_value.is_some(),
        ]
        .iter()
        .filter(|present| **present)
        .count() as i32;
        if score > best_score {
            best_score = score;
            best = Some(nap.clone());
        }
    }
    best.unwrap_or_default()
}

fn count_occurrences(corpus: &str, needle: &str) -> usize {
    let value = needle.trim().to_ascii_lowercase();
    if value.is_empty() {
        return 0;
    }
    if value.contains(' ') {
        return corpus.matches(&value).count();
    }
    Regex::new(&format!(r"\b{}\b", regex::escape(&value)))
        .map(|re| re.find_iter(corpus).count())
        .unwrap_or(0)
}

pub fn scan_site(raw_url: &str, options: ScanOptions) -> Result<ScanResult, String> {
    let started = Instant::now();
    let started_iso = now_iso();
    let public = public_mode();
    let max_pages = options.max_pages.unwrap_or(10).clamp(1, 30);
    let max_pages = if public { max_pages.min(10) } else { max_pages };

    let mut input = raw_url.trim().to_string();
    if input.is_empty() {
        return Err("Enter a website address to scan.".to_string());
    }
    if !(input.starts_with("http://") || input.starts_with("https://")) {
        input = format!("https://{}", input);
    }
    let mut root = Url::parse(&input).map_err(|_| format!("\"{}\" is not a valid URL.", raw_url))?;
    root.set_fragment(None);
    let origin = format!("{}://{}", root.scheme(), root.host_str().unwrap_or(""));
    let base_host = host_key(root.host_str().unwrap_or(""));
    if public && !public_url_allowed(&root.to_string()) {
        return Err("Public deployments only crawl public HTTP(S) addresses.".to_string());
    }

    let agent = build_agent(public);
    let robots = load_robots(&agent, &origin, public);
    let sitemap = load_sitemap(&agent, &origin, &robots.sitemaps, public);

    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((root.to_string(), 0));
    let mut visited: HashSet<String> = HashSet::new();
    let mut pages: Vec<PageAudit> = Vec::new();
    let mut root_error: Option<String> = None;

    while let Some((url, depth)) = queue.pop_front() {
        if pages.len() >= max_pages {
            break;
        }
        if !visited.insert(url.clone()) {
            continue;
        }
        let result = crawl_one(&agent, &url, depth, &base_host, &robots.disallow, public);
        match result {
            Some((audit, links)) => {
                let is_root = pages.is_empty();
                pages.push(audit);
                if depth < 2 {
                    for (link, next_depth) in links {
                        if visited.contains(&link) {
                            continue;
                        }
                        if pages.len() + queue.len() >= max_pages * 6 {
                            break;
                        }
                        queue.push_back((link, next_depth));
                    }
                }
                let _ = is_root;
            }
            None => {
                if depth == 0 && root_error.is_none() {
                    root_error = Some(format!(
                        "Could not load {} as an HTML page. Check the address or try again.",
                        base_host
                    ));
                }
            }
        }
    }

    if pages.is_empty() {
        return Err(root_error.unwrap_or_else(|| {
            "Could not load the site. Check the address and that it is publicly reachable.".to_string()
        }));
    }

    let root_page = &pages[0];
    if root_page.status >= 400 {
        let bot_challenge = root_page.status == 403
            || root_page.status == 429
            || Regex::new(r"(?i)attention required|just a moment|cf-browser-verification|enable javascript and cookies|captcha")
                .map(|re| re.is_match(&root_page.text_sample))
                .unwrap_or(false);
        return Err(if bot_challenge {
            format!(
                "{} blocked the audit request (HTTP {}, likely bot protection). This is not a site problem — try again later, or audit a site you control.",
                base_host, root_page.status
            )
        } else {
            format!("The starting page returned HTTP {}. Fix access before auditing.", root_page.status)
        });
    }

    let business = pick_business(&pages);
    let keyword = if options.keyword.trim().is_empty() {
        humanize_type(business.types.first().map(|t| t.as_str()).unwrap_or(""))
    } else {
        options.keyword.trim().to_string()
    };
    let detected_city = business
        .locality
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            let source = format!(
                "{} {}",
                root_page.title,
                root_page.h1.join(" ")
            );
            detect_city(&source)
        });
    let city = if options.city.trim().is_empty() {
        detected_city.clone()
    } else {
        options.city.trim().to_string()
    };

    let corpus = pages
        .iter()
        .map(|page| format!("{} {} {}", page.title, page.h1.join(" "), page.text_sample))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    let city_mentions = count_occurrences(&corpus, &city);
    let keyword_mentions = count_occurrences(&corpus, &keyword);

    let location_re = Regex::new(
        r"(?i)/(locations?|service-areas?|areas-we-serve|areas-served|cities|near-me|local)\b",
    )
    .unwrap();
    let city_lower = city.to_ascii_lowercase();
    let mut location_links: Vec<LocalLink> = Vec::new();
    let mut seen_links: HashSet<String> = HashSet::new();
    for page in &pages {
        for anchor in &page.anchors {
            let href = anchor.href.to_ascii_lowercase();
            let text = anchor.text.to_ascii_lowercase();
            let looks_local = (!city_lower.is_empty()
                && (text.contains(&city_lower) || href.contains(&city_lower)))
                || location_re.is_match(&href);
            if !looks_local || !seen_links.insert(anchor.href.clone()) {
                continue;
            }
            location_links.push(LocalLink {
                text: if anchor.text.is_empty() {
                    anchor.href.clone()
                } else {
                    anchor.text.clone()
                },
                href: anchor.href.clone(),
            });
            if location_links.len() >= 12 {
                break;
            }
        }
    }

    let about_re = Regex::new(r"(?i)/(about|about-us|company|our-team)\b").unwrap();
    let contact_re = Regex::new(r"(?i)/(contact|contact-us|get-in-touch)\b").unwrap();
    let find_link = |pattern: &Regex| -> Option<String> {
        for page in &pages {
            for anchor in &page.anchors {
                if pattern.is_match(&anchor.href) || pattern.is_match(&anchor.text) {
                    return Some(anchor.href.clone());
                }
            }
        }
        None
    };
    let about_link = find_link(&about_re);
    let contact_link = find_link(&contact_re);

    let unique_titles: HashSet<String> = pages
        .iter()
        .map(|page| page.title.to_ascii_lowercase())
        .filter(|title| !title.is_empty())
        .collect();
    let avg_ms = if pages.is_empty() {
        0
    } else {
        (pages.iter().map(|page| page.ms).sum::<u64>() as f64 / pages.len() as f64).round() as u64
    };
    let review_re = Regex::new(r"(?i)\breviews?\b|\btestimonials?\b").unwrap();

    let signals = SiteSignals {
        host: base_host.clone(),
        https: origin.starts_with("https://"),
        pages_crawled: pages.len(),
        max_depth: pages.iter().map(|page| page.depth).max().unwrap_or(0),
        robots_found: robots.found,
        robots_disallow: robots.disallow.clone(),
        sitemaps: robots.sitemaps.clone(),
        sitemap_found: sitemap.found,
        sitemap_urls: sitemap.urls,
        sitemap_is_index: sitemap.is_index,
        gsc_verification: pages.iter().any(|page| page.gsc_verification),
        business,
        city_detected: !detected_city.is_empty() && options.city.trim().is_empty(),
        keyword_detected: options.keyword.trim().is_empty() && !keyword.is_empty(),
        city: city.clone(),
        keyword: keyword.clone(),
        city_mentions,
        keyword_mentions,
        avg_ms,
        html_bytes: root_page.bytes,
        blocking_scripts: pages.iter().map(|page| page.blocking_scripts).max().unwrap_or(0),
        images_no_dims: pages.iter().map(|page| page.images.no_dimensions).sum(),
        internal_link_total: pages.iter().map(|page| page.links.internal).sum(),
        location_links,
        about_link,
        contact_link,
        map_embed: pages.iter().any(|page| page.has_map_embed),
        review_mentions: review_re.find_iter(&corpus).count(),
        indexable_pages: pages.iter().filter(|page| !page.noindex).count(),
        unique_titles: unique_titles.len(),
    };

    let target = Target { keyword, city };
    let finished_at = now_iso();

    Ok(ScanResult {
        id: new_id("s"),
        url: root.to_string(),
        host: base_host,
        started_at: started_iso,
        finished_at,
        duration_ms: started.elapsed().as_millis() as u64,
        target,
        pages,
        signals,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_robots_directives() {
        let (disallow, sitemaps) = parse_robots(
            "User-agent: *\nDisallow: /admin\nSitemap: https://example.test/sitemap.xml\n",
        );
        assert_eq!(disallow, vec!["/admin".to_string()]);
        assert_eq!(sitemaps, vec!["https://example.test/sitemap.xml".to_string()]);
        assert!(robots_allows("/about", &disallow));
        assert!(!robots_allows("/admin/settings", &disallow));
    }

    #[test]
    fn normalizes_hosts_and_detects_cities() {
        assert_eq!(host_key("WWW.Example.com"), "example.com");
        assert_eq!(detect_city("Plumber in Austin, TX | ABC"), "Austin");
        assert_eq!(humanize_type("LocalBusiness"), "");
        assert_eq!(humanize_type("HomeAndConstructionBusiness"), "home and construction business");
    }
}
