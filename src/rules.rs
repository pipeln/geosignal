use crate::types::{
    CategoryScore, CheckResult, CheckSource, CustomRule, Overrides, PageAudit, Report, ScanResult,
    SiteSignals,
};
use crate::util::pct;
use regex::Regex;
use std::collections::HashSet;

const SEARCH_ESSENTIALS: (&str, &str) = (
    "Google Search Essentials",
    "https://developers.google.com/search/docs/essentials",
);
const SEO_STARTER: (&str, &str) = (
    "Google SEO Starter Guide",
    "https://developers.google.com/search/docs/fundamentals/seo-starter-guide",
);
const LOCAL_RANKING: (&str, &str) = (
    "Google: Tips to improve local ranking",
    "https://support.google.com/business/answer/7091",
);
const BUSINESS_INFO: (&str, &str) = (
    "How Google sources business information",
    "https://support.google.com/business/answer/2721884",
);
const LOCAL_BUSINESS: (&str, &str) = (
    "LocalBusiness structured data",
    "https://developers.google.com/search/docs/appearance/structured-data/local-business",
);
const STRUCTURED_INTRO: (&str, &str) = (
    "Intro to structured data",
    "https://developers.google.com/search/docs/appearance/structured-data/intro-structured-data",
);
const CWV: (&str, &str) = (
    "Core Web Vitals & thresholds",
    "https://developers.google.com/search/docs/appearance/core-web-vitals",
);
const TITLE_LINK: (&str, &str) = (
    "Influencing title links",
    "https://developers.google.com/search/docs/appearance/title-link",
);
const SNIPPET: (&str, &str) = (
    "Snippet / meta description guidance",
    "https://developers.google.com/search/docs/appearance/snippet",
);
const CANONICAL: (&str, &str) = (
    "Consolidate duplicate URLs",
    "https://developers.google.com/search/docs/crawling-indexing/consolidate-duplicate-urls",
);
const ROBOTS: (&str, &str) = (
    "Robots.txt introduction",
    "https://developers.google.com/search/docs/crawling-indexing/robots/intro",
);
const SITEMAP: (&str, &str) = (
    "Build and submit a sitemap",
    "https://developers.google.com/search/docs/crawling-indexing/sitemaps/build-sitemap",
);
const HREFLANG: (&str, &str) = (
    "Localized versions (hreflang)",
    "https://developers.google.com/search/docs/specialty/international/localized-versions",
);
const BREADCRUMB: (&str, &str) = (
    "Breadcrumb structured data",
    "https://developers.google.com/search/docs/appearance/structured-data/breadcrumb",
);
const FAQ: (&str, &str) = (
    "FAQPage structured data",
    "https://developers.google.com/search/docs/appearance/structured-data/faqpage",
);
const REVIEW_SNIPPET: (&str, &str) = (
    "Review snippet structured data",
    "https://developers.google.com/search/docs/appearance/structured-data/review-snippet",
);
const REVIEW_POLICY: (&str, &str) = (
    "Google: making review rich results more helpful",
    "https://developers.google.com/search/blog/2019/09/making-review-rich-results-more-helpful",
);
const HELPFUL_CONTENT: (&str, &str) = (
    "Creating helpful, reliable, people-first content",
    "https://developers.google.com/search/docs/fundamentals/creating-helpful-content",
);
const MOBILE_FIRST: (&str, &str) = (
    "Mobile-first indexing best practices",
    "https://developers.google.com/search/docs/crawling-indexing/mobile/mobile-sites-mobile-first-indexing",
);
const LINKS: (&str, &str) = (
    "Link best practices",
    "https://developers.google.com/search/docs/crawling-indexing/links-crawlable",
);
const HTTPS_SIGNAL: (&str, &str) = (
    "HTTPS as a ranking signal",
    "https://developers.google.com/search/blog/2014/08/https-as-ranking-signal",
);
const GSC: (&str, &str) = (
    "Verify site ownership in Search Console",
    "https://support.google.com/webmasters/answer/9008080",
);

pub const CATEGORIES: [(&str, &str, &str); 5] = [
    (
        "entity",
        "Local entity",
        "Can Google and models confirm who you are, where you are, and how to reach you?",
    ),
    (
        "onpage",
        "On-page relevance",
        "Do titles, headings and copy tie your services to the places you serve?",
    ),
    (
        "linking",
        "Location pages & links",
        "Are location/service-area pages discoverable and internally connected?",
    ),
    (
        "technical",
        "Technical & indexability",
        "Crawlability, canonicalisation, mobile readiness and page experience.",
    ),
    (
        "trust",
        "Content & E-E-A-T",
        "Depth, evidence, accessibility and people-first usefulness.",
    ),
];

pub struct Ctx<'a> {
    pub root: &'a PageAudit,
    pub pages: &'a [PageAudit],
    pub s: &'a SiteSignals,
    pub keyword: &'a str,
    pub city: &'a str,
}

pub struct Outcome {
    pub status: &'static str,
    pub evidence: String,
    pub page: Option<String>,
}

fn pass(evidence: impl Into<String>) -> Outcome {
    Outcome { status: "pass", evidence: evidence.into(), page: None }
}
fn warn(evidence: impl Into<String>) -> Outcome {
    Outcome { status: "warn", evidence: evidence.into(), page: None }
}
fn fail(evidence: impl Into<String>) -> Outcome {
    Outcome { status: "fail", evidence: evidence.into(), page: None }
}
fn info(evidence: impl Into<String>) -> Outcome {
    Outcome { status: "info", evidence: evidence.into(), page: None }
}

fn listing(items: &[String], max: usize) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    let shown = items.iter().take(max).cloned().collect::<Vec<_>>().join(" -> ");
    if items.len() > max {
        format!("{} (+{} more)", shown, items.len() - max)
    } else {
        shown
    }
}

fn status_rank(status: &str) -> usize {
    if status == "fail" {
        0
    } else {
        1
    }
}

fn status_value(status: &str) -> Option<f64> {
    match status {
        "pass" => Some(1.0),
        "warn" => Some(0.5),
        "fail" => Some(0.0),
        _ => None,
    }
}

type RuleFn = fn(&Ctx) -> Outcome;

pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub weight: f64,
    pub why: &'static str,
    pub fix: &'static str,
    pub sources: &'static [(&'static str, &'static str)],
    pub run: RuleFn,
}

pub static RULES: &[Rule] = &[
    Rule {
        id: "https",
        title: "Site is served over HTTPS",
        category: "technical",
        weight: 3.0,
        why: "HTTPS is a documented (lightweight) ranking signal and browsers warn on plain HTTP. For a local business it is also a trust cue in the click decision.",
        fix: "Issue a certificate, redirect every http:// URL to https:// with a single 301, and update internal links, canonicals and your sitemap to the HTTPS host.",
        sources: &[HTTPS_SIGNAL, SEARCH_ESSENTIALS],
        run: |ctx| {
            if ctx.s.https {
                pass(format!("All crawled URLs use https://{}.", ctx.s.host))
            } else {
                fail(format!("The site resolved on http://{} only.", ctx.s.host))
            }
        },
    },
    Rule {
        id: "indexability",
        title: "Pages are indexable",
        category: "technical",
        weight: 3.0,
        why: "A local page cannot rank if Google is told not to index it. noindex is the most common cause of \"my new location page never appeared\".",
        fix: "Remove \"noindex\" from robots meta tags and X-Robots-Tag headers on pages that should rank, then request indexing in Search Console.",
        sources: &[SEARCH_ESSENTIALS, ROBOTS],
        run: |ctx| {
            let blocked: Vec<_> = ctx.pages.iter().filter(|page| page.noindex).collect();
            if blocked.is_empty() {
                return pass(format!(
                    "All {} crawled pages are indexable ({}/{}).",
                    ctx.pages.len(),
                    ctx.s.indexable_pages,
                    ctx.pages.len()
                ));
            }
            let paths: Vec<String> = blocked.iter().map(|page| page.path.clone()).collect();
            warn(format!(
                "{} of {} crawled pages send noindex: {}.",
                blocked.len(),
                ctx.pages.len(),
                listing(&paths, 3)
            ))
        },
    },
    Rule {
        id: "canonical",
        title: "Homepage declares a self-referencing canonical",
        category: "technical",
        weight: 2.0,
        why: "Canonical tags consolidate duplicate or parameterised URLs so ranking signals land on one address. Local sites often duplicate the same page per suburb or tracking parameter.",
        fix: "Add <link rel=\"canonical\" href=\"https://yourdomain.com/current-path\"> on every page, matching the preferred URL. Keep one canonical per page.",
        sources: &[CANONICAL],
        run: |ctx| {
            let normalize = |value: &str| {
                value
                    .trim_end_matches('/')
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .to_ascii_lowercase()
            };
            if ctx.root.canonical.is_empty() {
                return warn("No canonical link element found on the starting page.");
            }
            if normalize(&ctx.root.canonical) == normalize(&ctx.root.url) {
                pass(format!("Canonical points to itself: {}", ctx.root.canonical))
            } else {
                warn(format!("Canonical points elsewhere: {}", ctx.root.canonical))
            }
        },
    },
    Rule {
        id: "title",
        title: "Titles are present, specific and unique",
        category: "onpage",
        weight: 3.0,
        why: "The title link is the strongest on-page relevance hint in a local query (\"service + city\"). Duplicate titles across location pages make them compete with each other.",
        fix: "Write one unique title per page, roughly 30-60 characters, leading with the service then the locality, e.g. \"Emergency Plumber in Austin, TX | ABC Plumbing\".",
        sources: &[TITLE_LINK, SEO_STARTER],
        run: |ctx| {
            if ctx.root.title.is_empty() {
                return fail("The starting page has no <title> element.");
            }
            let titled: Vec<_> = ctx.pages.iter().filter(|page| !page.title.is_empty()).collect();
            let duplicates = titled.len().saturating_sub(ctx.s.unique_titles);
            if duplicates > 0 {
                return warn(format!(
                    "\"{}\" ({} chars), but {} page(s) share a title across {} pages.",
                    ctx.root.title,
                    ctx.root.title_length,
                    duplicates,
                    titled.len()
                ));
            }
            if ctx.root.title_length < 15 {
                return warn(format!(
                    "Title is very short: \"{}\" ({} chars).",
                    ctx.root.title, ctx.root.title_length
                ));
            }
            let too_long: Vec<_> = titled
                .iter()
                .filter(|page| page.title_length > 65)
                .collect();
            if !too_long.is_empty() {
                return warn(format!(
                    "{} title(s) exceed 65 characters, e.g. \"{}\" ({} chars).",
                    too_long.len(),
                    too_long[0].title,
                    too_long[0].title_length
                ));
            }
            pass(format!(
                "\"{}\" ({} chars); titles are unique across {} pages.",
                ctx.root.title,
                ctx.root.title_length,
                titled.len()
            ))
        },
    },
    Rule {
        id: "meta_description",
        title: "Meta descriptions support the location pitch",
        category: "onpage",
        weight: 2.0,
        why: "Descriptions do not change rankings directly, but Google uses them for snippets. A description naming the service, the city and a differentiator improves click-through from the same position.",
        fix: "Write 120-160 character descriptions per page: service, city/area, proof (years, licence, rating) and a call to action. Google may rewrite them, so keep them honest.",
        sources: &[SNIPPET],
        run: |ctx| {
            if ctx.root.meta_description.is_empty() {
                return fail("The starting page has no meta description.");
            }
            let missing = ctx
                .pages
                .iter()
                .filter(|page| page.meta_description.is_empty())
                .count();
            if missing > 0 {
                return warn(format!(
                    "Description present on the homepage, missing on {} of {} crawled pages.",
                    missing,
                    ctx.pages.len()
                ));
            }
            let length = ctx.root.meta_description.chars().count();
            if length < 70 {
                return warn(format!("Homepage description is short ({} chars).", length));
            }
            if length > 165 {
                return warn(format!(
                    "Homepage description is long ({} chars) and may be truncated.",
                    length
                ));
            }
            let preview: String = ctx.root.meta_description.chars().take(120).collect();
            pass(format!(
                "Homepage description is {} chars: \"{}...\"",
                length, preview
            ))
        },
    },
    Rule {
        id: "h1",
        title: "One clear H1 per page",
        category: "onpage",
        weight: 2.0,
        why: "An H1 gives crawlers and readers the page topic. One H1 containing the service or locality is easier to match to a query than a styled div or five competing H1s.",
        fix: "Use exactly one H1 per page describing the page topic, then structure sections with H2/H3. Do not use H1 for the logo or repeated taglines.",
        sources: &[SEO_STARTER],
        run: |ctx| {
            if ctx.root.h1.is_empty() {
                return fail("No H1 found on the starting page.");
            }
            if ctx.root.h1.len() > 1 {
                return warn(format!(
                    "{} H1s on the starting page: {}.",
                    ctx.root.h1.len(),
                    listing(&ctx.root.h1, 2)
                ));
            }
            pass(format!("H1: \"{}\"", ctx.root.h1[0]))
        },
    },
    Rule {
        id: "headings",
        title: "Content is structured with subheadings",
        category: "onpage",
        weight: 1.0,
        why: "H2/H3 structure lets Google (and AI answer engines) extract the specific passage that answers \"do you serve my suburb?\" or \"do you have emergency call-out?\".",
        fix: "Break long sections with descriptive H2s such as \"Areas we serve\", \"Emergency call-outs\", \"Pricing and guarantees\". Include the locality naturally.",
        sources: &[SEO_STARTER],
        run: |ctx| {
            if ctx.root.h2.len() >= 2 {
                pass(format!(
                    "{} H2 sections, e.g. \"{}\".",
                    ctx.root.h2.len(),
                    ctx.root.h2.first().cloned().unwrap_or_default()
                ))
            } else {
                warn(format!(
                    "Only {} H2 section(s) on the starting page.",
                    ctx.root.h2.len()
                ))
            }
        },
    },
    Rule {
        id: "lang",
        title: "Page language is declared",
        category: "technical",
        weight: 1.0,
        why: "A correct lang attribute helps Google serve the page to the right language/region audience and helps screen readers pronounce content.",
        fix: "Set <html lang=\"en-AU\"> (or your locale) on every page. For multi-country sites also add hreflang alternates.",
        sources: &[HREFLANG, MOBILE_FIRST],
        run: |ctx| {
            if ctx.root.lang.is_empty() {
                warn("The <html> element has no lang attribute.")
            } else {
                pass(format!("<html lang=\"{}\">", ctx.root.lang))
            }
        },
    },
    Rule {
        id: "viewport",
        title: "Mobile viewport is configured",
        category: "technical",
        weight: 3.0,
        why: "Google indexes the mobile version of a site first. Without a viewport meta tag, phones render a zoomed-out desktop layout, which hurts usability and mobile-first indexing.",
        fix: "Add <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"> and test tap targets, font sizes and sticky call buttons on a real phone.",
        sources: &[MOBILE_FIRST],
        run: |ctx| {
            if !ctx.root.viewport {
                return fail("No viewport meta tag on the starting page.");
            }
            let missing = ctx.pages.iter().filter(|page| !page.viewport).count();
            if missing > 0 {
                return warn(format!(
                    "Viewport present on the homepage but missing on {} page(s).",
                    missing
                ));
            }
            pass("Every crawled page declares a responsive viewport.")
        },
    },
    Rule {
        id: "robots",
        title: "robots.txt is present and sane",
        category: "technical",
        weight: 1.0,
        why: "robots.txt controls crawl access. A stray \"Disallow: /\" from a staging push is a classic way to vanish from local results.",
        fix: "Publish robots.txt that allows crawlable pages, blocks only admin/cart/search URLs, and references your sitemap.",
        sources: &[ROBOTS, SITEMAP],
        run: |ctx| {
            if !ctx.s.robots_found {
                return warn(format!(
                    "No robots.txt at https://{}/robots.txt (crawling still works, but you lose directive control).",
                    ctx.s.host
                ));
            }
            if ctx.s.robots_disallow.iter().any(|rule| rule == "/") {
                return fail("robots.txt contains \"Disallow: /\", which blocks the whole site.");
            }
            pass(format!(
                "robots.txt found with {} disallow rule(s){}.",
                ctx.s.robots_disallow.len(),
                if ctx.s.sitemaps.is_empty() {
                    String::new()
                } else {
                    format!(" and {} sitemap reference(s)", ctx.s.sitemaps.len())
                }
            ))
        },
    },
    Rule {
        id: "sitemap",
        title: "An XML sitemap lists the site",
        category: "technical",
        weight: 2.0,
        why: "A sitemap is how you tell Google about new location pages and get them discovered faster, especially pages that are weakly linked.",
        fix: "Generate /sitemap.xml with every indexable URL, reference it from robots.txt, and submit it in Search Console. Regenerate when you add a location page.",
        sources: &[SITEMAP],
        run: |ctx| {
            if !ctx.s.sitemap_found {
                return fail("No sitemap found at the URLs advertised in robots.txt or /sitemap.xml.");
            }
            if ctx.s.sitemap_urls == 0 {
                return warn("A sitemap responded but contained no <loc> entries.");
            }
            pass(format!(
                "{} found with {} URL entries.",
                if ctx.s.sitemap_is_index { "Sitemap index" } else { "Sitemap" },
                ctx.s.sitemap_urls
            ))
        },
    },
    Rule {
        id: "gsc",
        title: "Search Console ownership is verifiable from the page",
        category: "technical",
        weight: 1.0,
        why: "Search Console is the only first-party source for queries, impressions, indexing status and Core Web Vitals. Without it you are optimising blind.",
        fix: "Verify the domain in Search Console (DNS or HTML tag). If you use the HTML tag, keep it in the <head>. Then review the Pages and Core Web Vitals reports monthly.",
        sources: &[GSC],
        run: |ctx| {
            if ctx.s.gsc_verification {
                pass("A google-site-verification meta tag is present.")
            } else {
                warn("No google-site-verification meta tag found (DNS verification is also valid — confirm in Search Console).")
            }
        },
    },
    Rule {
        id: "cwv",
        title: "Page weight, blocking scripts and response time are in range",
        category: "technical",
        weight: 2.0,
        why: "Core Web Vitals are page-experience signals: LCP <=2.5s, INP <=200ms, CLS <=0.1 at the 75th percentile. HTML size, render-blocking scripts, unsized images and server response time are the levers you can see from a crawl.",
        fix: "Compress HTML, defer non-critical JS, set width/height (or aspect-ratio) on images, preload the LCP image, and cache HTML at the edge. Confirm with Search Console Core Web Vitals and PageSpeed Insights field data.",
        sources: &[CWV],
        run: |ctx| {
            let dimensions_ok = ctx.root.images.total == 0
                || (ctx.root.images.no_dimensions as f64 / ctx.root.images.total as f64) < 0.3;
            let points = [
                ctx.root.bytes < 150_000,
                ctx.s.blocking_scripts <= 3,
                dimensions_ok,
                ctx.s.avg_ms < 800,
            ];
            let score = points.iter().filter(|value| **value).count();
            let facts = format!(
                "{} KB HTML · {} render-blocking script(s) · {} ms avg response · {}/{} images without dimensions",
                ctx.root.bytes / 1024,
                ctx.s.blocking_scripts,
                ctx.s.avg_ms,
                ctx.root.images.no_dimensions,
                ctx.root.images.total
            );
            if score == 4 {
                pass(facts)
            } else if score >= 2 {
                warn(format!("{}/4 checks in range — {}.", score, facts))
            } else {
                fail(format!("{}/4 checks in range — {}.", score, facts))
            }
        },
    },
    Rule {
        id: "hreflang",
        title: "Multi-region versions are declared (if any)",
        category: "technical",
        weight: 1.0,
        why: "For businesses serving several countries or languages, hreflang tells Google which version to show. Single-market sites do not need it — this check is informational unless alternates exist.",
        fix: "If you run country or language variants, add reciprocal rel=\"alternate\" hreflang links (including x-default). Otherwise, ignore this check.",
        sources: &[HREFLANG],
        run: |ctx| {
            if let Some(page) = ctx.pages.iter().find(|page| !page.hreflang.is_empty()) {
                pass(format!(
                    "{} hreflang alternate(s) found, e.g. {}.",
                    page.hreflang.len(),
                    page.hreflang[0].lang
                ))
            } else {
                info("No hreflang alternates. Expected for a single-language local site.")
            }
        },
    },
    Rule {
        id: "localbusiness_schema",
        title: "LocalBusiness structured data is complete",
        category: "entity",
        weight: 3.0,
        why: "LocalBusiness markup (name, address, phone, geo, hours, sameAs) is the machine-readable version of your Google Business Profile. It disambiguates your entity and feeds rich results and AI answers.",
        fix: "Add one LocalBusiness (or the most specific subtype, e.g. Plumber) JSON-LD block with @id, name, address, telephone, geo, openingHoursSpecification, url, sameAs and areaServed. Validate with the Rich Results Test.",
        sources: &[LOCAL_BUSINESS, STRUCTURED_INTRO],
        run: |ctx| {
            let nap = &ctx.s.business;
            if nap.name.is_none() {
                return fail("No LocalBusiness/Organization JSON-LD with name + address was found in the crawled pages.");
            }
            let has_address = nap.street.is_some() || nap.locality.is_some();
            let has_phone = nap.phone.is_some();
            let detail = [
                nap.name.clone(),
                nap.types.first().cloned(),
                if has_address { Some("address".to_string()) } else { None },
                if has_phone { Some("phone".to_string()) } else { None },
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            if has_address && has_phone {
                pass(format!("Detected {}.", detail))
            } else {
                warn(format!(
                    "Detected {} but {} is missing.",
                    detail,
                    if !has_address { "address" } else { "telephone" }
                ))
            }
        },
    },
    Rule {
        id: "nap_visible",
        title: "Name, address and phone appear in visible copy",
        category: "entity",
        weight: 3.0,
        why: "Google cross-checks your site against Business Profile and other citations. Matching NAP in visible HTML (not only an image or schema) is the consistency signal local ranking guidance hinges on: prominence, relevance, distance.",
        fix: "Show the full business name, street address and phone number in the footer of every page, matching your Google Business Profile character-for-character. Use a tel: link.",
        sources: &[LOCAL_RANKING, BUSINESS_INFO],
        run: |ctx| {
            let text = ctx.root.text_sample.to_ascii_lowercase();
            let phone_digits: String = ctx
                .s
                .business
                .phone
                .clone()
                .unwrap_or_default()
                .chars()
                .filter(|ch| ch.is_ascii_digit())
                .collect();
            let page_digits: String = text.chars().filter(|ch| ch.is_ascii_digit()).collect();
            let phone_visible = phone_digits.len() >= 7 && page_digits.contains(&phone_digits);
            let locality_visible = ctx
                .s
                .business
                .locality
                .as_ref()
                .map(|locality| text.contains(&locality.to_ascii_lowercase()))
                .unwrap_or(false);
            let street_visible = ctx
                .s
                .business
                .street
                .as_ref()
                .map(|street| text.contains(&street.to_ascii_lowercase()))
                .unwrap_or(false);
            let facts = format!(
                "{} · {} · {}",
                if phone_visible { "phone visible" } else { "phone not visible" },
                if locality_visible { "locality visible" } else { "locality not visible" },
                if street_visible { "street visible" } else { "street not visible" }
            );
            if ctx.s.business.name.is_none() {
                return warn(format!("No structured NAP to compare; homepage copy: {}.", facts));
            }
            if phone_visible && (locality_visible || street_visible) {
                pass(facts)
            } else {
                fail(facts)
            }
        },
    },
    Rule {
        id: "tel_link",
        title: "Phone number is tappable on mobile",
        category: "entity",
        weight: 2.0,
        why: "Local searches are overwhelmingly mobile and often call-driven. A tel: link removes friction between a local result and a call, which is what most local businesses are actually optimising for.",
        fix: "Wrap the phone number in <a href=\"tel:+15551234567\">, keep it in the header and footer, and add a sticky \"Call\" button on mobile.",
        sources: &[LOCAL_RANKING],
        run: |ctx| {
            let total: usize = ctx.pages.iter().map(|page| page.links.tel).sum();
            if total > 0 {
                pass(format!("{} tel: link(s) across {} pages.", total, ctx.pages.len()))
            } else {
                fail("No tel: links found. The phone number is not tappable.")
            }
        },
    },
    Rule {
        id: "geo",
        title: "Coordinates or service areas are declared",
        category: "entity",
        weight: 2.0,
        why: "geo and areaServed spell out where you actually operate. They help match a service to the searcher's location without stuffing every suburb name into body copy.",
        fix: "Add \"geo\": {\"@type\":\"GeoCoordinates\",\"latitude\":...,\"longitude\":...} and areaServed entries (cities, regions or GeoCircle) to your LocalBusiness markup.",
        sources: &[LOCAL_BUSINESS],
        run: |ctx| {
            let has_geo = ctx.s.business.lat.is_some() && ctx.s.business.lng.is_some();
            let areas = &ctx.s.business.area_served;
            if has_geo && !areas.is_empty() {
                return pass(format!(
                    "Geo {}, {} and {} areaServed entr{}.",
                    ctx.s.business.lat.unwrap_or_default(),
                    ctx.s.business.lng.unwrap_or_default(),
                    areas.len(),
                    if areas.len() == 1 { "y" } else { "ies" }
                ));
            }
            if has_geo {
                warn("Coordinates present, areaServed missing.")
            } else if !areas.is_empty() {
                warn(format!("areaServed lists {} place(s), but no coordinates.", areas.len()))
            } else {
                warn("No geo coordinates or areaServed values in the LocalBusiness markup.")
            }
        },
    },
    Rule {
        id: "hours",
        title: "Opening hours are published and marked up",
        category: "entity",
        weight: 1.0,
        why: "Hours affect \"open now\" style intent, reduce wasted calls and keep your Business Profile consistent. Marked-up hours are reusable by Google and other agents.",
        fix: "Add openingHoursSpecification to your LocalBusiness JSON-LD and display the same hours on the Contact page. Update both for holidays.",
        sources: &[LOCAL_BUSINESS],
        run: |ctx| {
            if !ctx.s.business.opening_hours.is_empty() {
                return pass(format!(
                    "Hours: {}.",
                    listing(&ctx.s.business.opening_hours, 2)
                ));
            }
            let hours_re = Regex::new(
                r"(?i)\b(mon|tue|wed|thu|fri|sat|sun)[a-z]*\b[\s\S]{0,40}\d{1,2}(:\d{2})?\s?(am|pm)",
            )
            .unwrap();
            if hours_re.is_match(&ctx.root.text_sample) {
                warn("Hours appear in visible copy but are not marked up in structured data.")
            } else {
                warn("No opening hours found in visible copy or structured data.")
            }
        },
    },
    Rule {
        id: "same_as",
        title: "Profiles are linked with sameAs",
        category: "entity",
        weight: 2.0,
        why: "sameAs links your site to the authoritative profiles (Google Business Profile, Facebook, Yelp, industry registers). That is how search engines reconcile one business entity across the web.",
        fix: "Add sameAs URLs for your Google Maps/Business Profile, social profiles and major directories to your Organization/LocalBusiness markup and footer.",
        sources: &[LOCAL_BUSINESS, BUSINESS_INFO],
        run: |ctx| {
            let mut profiles: HashSet<String> = ctx
                .s
                .business
                .same_as
                .iter()
                .map(|url| url.to_ascii_lowercase())
                .collect();
            let relevant = Regex::new(
                r"(?i)google\.[a-z.]+/maps|facebook\.com|instagram\.com|linkedin\.com|yelp\.|yellowpages\.|tripadvisor\.|healthgrades\.|bbb\.org|trustpilot\.",
            )
            .unwrap();
            for page in ctx.pages {
                for anchor in &page.anchors {
                    if relevant.is_match(&anchor.href) {
                        profiles.insert(anchor.href.to_ascii_lowercase());
                    }
                }
            }
            let list: Vec<String> = profiles.iter().cloned().collect();
            if profiles.is_empty() {
                warn("No sameAs values and no links to Google Business Profile, social or directory profiles.")
            } else {
                pass(format!(
                    "{} profile link(s), e.g. {}.",
                    profiles.len(),
                    listing(&list, 2)
                ))
            }
        },
    },
    Rule {
        id: "city_title",
        title: "Primary locality appears in the title",
        category: "onpage",
        weight: 3.0,
        why: "Local queries are service + place. A title that names the city/region is the clearest relevance match a local page can make without keyword stuffing.",
        fix: "Put the locality in the title of the money pages, e.g. \"Dental Implants in Bondi | Harbour Dental\". Keep brand last.",
        sources: &[TITLE_LINK, LOCAL_RANKING],
        run: |ctx| {
            if ctx.city.is_empty() {
                return info("No locality detected. Enter the target city to unlock this check.");
            }
            if ctx.root.title.to_ascii_lowercase().contains(&ctx.city.to_ascii_lowercase()) {
                pass(format!(
                    "Title contains \"{}\": \"{}\"",
                    ctx.city, ctx.root.title
                ))
            } else {
                fail(format!(
                    "The title does not contain \"{}\": \"{}\"",
                    ctx.city, ctx.root.title
                ))
            }
        },
    },
    Rule {
        id: "city_content",
        title: "Locality is used naturally in headings and copy",
        category: "onpage",
        weight: 2.0,
        why: "Repeated, natural locality mentions (in H1/H2 and body) plus service context build topical relevance for a place. One mention in a footer is not enough; stuffing every suburb is a spam risk.",
        fix: "Mention the city/region in the H1, an \"Areas we serve\" section and 2-4 body sentences tied to real local context (landmarks, travel, local regulations).",
        sources: &[LOCAL_RANKING, HELPFUL_CONTENT],
        run: |ctx| {
            if ctx.city.is_empty() {
                return info("No locality detected. Enter the target city to unlock this check.");
            }
            let mentions = ctx.s.city_mentions;
            let in_h1 = ctx
                .root
                .h1
                .iter()
                .any(|heading| heading.to_ascii_lowercase().contains(&ctx.city.to_ascii_lowercase()));
            if in_h1 && mentions >= 2 {
                pass(format!(
                    "\"{}\" appears in the H1 and {} times across the crawl.",
                    ctx.city, mentions
                ))
            } else if mentions >= 2 {
                warn(format!(
                    "\"{}\" appears {} times but not in the H1.",
                    ctx.city, mentions
                ))
            } else {
                fail(format!(
                    "\"{}\" appears {} time(s) across {} crawled pages.",
                    ctx.city, mentions, ctx.s.pages_crawled
                ))
            }
        },
    },
    Rule {
        id: "keyword",
        title: "The primary service is expressed on-page",
        category: "onpage",
        weight: 3.0,
        why: "Local ranking needs a service intent, not just a place. Repeating the service phrase in title, H1 and body sections is how a page signals \"this is the plumber for Austin\", not \"this is a business in Austin\".",
        fix: "State the primary service in the H1, title and first paragraph, with real variants (emergency call-out, same-day, 24/7). Cover related services on dedicated pages.",
        sources: &[SEO_STARTER, HELPFUL_CONTENT],
        run: |ctx| {
            if ctx.keyword.is_empty() {
                return info("No primary service detected. Add one in the scan form for a precise check.");
            }
            let needle = ctx.keyword.to_ascii_lowercase();
            let in_title = ctx.root.title.to_ascii_lowercase().contains(&needle);
            let in_h1 = ctx
                .root
                .h1
                .iter()
                .any(|heading| heading.to_ascii_lowercase().contains(&needle));
            let mentions = ctx.s.keyword_mentions;
            if in_title && in_h1 && mentions >= 2 {
                pass(format!(
                    "\"{}\" is in the title, H1 and {} mentions.",
                    ctx.keyword, mentions
                ))
            } else if in_title || in_h1 || mentions >= 3 {
                warn(format!(
                    "\"{}\" — title: {}, H1: {}, {} total mentions.",
                    ctx.keyword,
                    if in_title { "yes" } else { "no" },
                    if in_h1 { "yes" } else { "no" },
                    mentions
                ))
            } else {
                fail(format!(
                    "\"{}\" is not expressed in the title, H1 or body copy ({} mentions).",
                    ctx.keyword, mentions
                ))
            }
        },
    },
    Rule {
        id: "location_pages",
        title: "Dedicated location or service-area pages are linked",
        category: "linking",
        weight: 3.0,
        why: "A single homepage cannot credibly rank for every suburb. Google's local guidance is relevance, distance and prominence — dedicated pages per real service area give each place a relevant landing page, and internal links make them discoverable.",
        fix: "Create one page per materially different service area (city/suburb/region) with unique copy, local proof and its own title/H1. Link them from a \"Locations\" hub and the footer. Do not clone the same text with the city swapped.",
        sources: &[LOCAL_RANKING, HELPFUL_CONTENT],
        run: |ctx| {
            if !ctx.s.location_links.is_empty() {
                let labels: Vec<String> = ctx
                    .s
                    .location_links
                    .iter()
                    .map(|link| link.text.clone())
                    .collect();
                return pass(format!(
                    "{} local link(s): {}.",
                    labels.len(),
                    listing(&labels, 3)
                ));
            }
            fail("No location, service-area or city pages were linked from the crawled pages.")
        },
    },
    Rule {
        id: "internal_links",
        title: "Pages are internally linked, not orphaned",
        category: "linking",
        weight: 2.0,
        why: "Internal links pass context and crawl priority. Orphaned service pages are discovered late or never, and anchor text tells Google what the destination is about.",
        fix: "Link every service and location page from a hub page and the footer. Use descriptive anchors (\"emergency plumber in Bondi\"), not \"click here\".",
        sources: &[LINKS],
        run: |ctx| {
            let avg = if ctx.s.pages_crawled == 0 {
                0
            } else {
                (ctx.s.internal_link_total + ctx.s.pages_crawled / 2) / ctx.s.pages_crawled
            };
            let sparse = ctx
                .pages
                .iter()
                .filter(|page| page.links.internal < 2)
                .count();
            if avg >= 5 && sparse == 0 {
                pass(format!(
                    "{} internal links per page on average ({} total).",
                    avg, ctx.s.internal_link_total
                ))
            } else if avg >= 2 {
                warn(format!(
                    "{} internal links per page on average; {} page(s) have fewer than 2.",
                    avg, sparse
                ))
            } else {
                fail(format!(
                    "{} internal links per page on average; {} page(s) have fewer than 2.",
                    avg, sparse
                ))
            }
        },
    },
    Rule {
        id: "anchor_relevance",
        title: "Anchor text carries service and place context",
        category: "linking",
        weight: 2.0,
        why: "Anchor text is a relevance hint for the linked page. \"Services\" tells Google nothing; \"roof repairs in Leeds\" defines the target page.",
        fix: "Rewrite generic navigation anchors on local pages to include service + place, and vary them naturally across the site.",
        sources: &[LINKS],
        run: |ctx| {
            if ctx.keyword.is_empty() && ctx.city.is_empty() {
                return info("Add a service and city to evaluate anchor text.");
            }
            let needles: Vec<String> = [ctx.keyword, ctx.city]
                .iter()
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase())
                .collect();
            let mut matches = 0usize;
            let mut samples: Vec<String> = Vec::new();
            for page in ctx.pages {
                for anchor in &page.anchors {
                    let text = anchor.text.to_ascii_lowercase();
                    if !text.is_empty() && needles.iter().any(|needle| text.contains(needle)) {
                        matches += 1;
                        if samples.len() < 3 {
                            samples.push(anchor.text.clone());
                        }
                    }
                }
            }
            if matches >= 3 {
                pass(format!(
                    "{} anchors contain the service/place: {}.",
                    matches,
                    listing(&samples, 3)
                ))
            } else {
                warn(format!(
                    "Only {} anchor(s) mention the service or place ({}).",
                    matches,
                    if samples.is_empty() {
                        "none".to_string()
                    } else {
                        listing(&samples, 2)
                    }
                ))
            }
        },
    },
    Rule {
        id: "breadcrumb",
        title: "Breadcrumbs are present",
        category: "linking",
        weight: 1.0,
        why: "Breadcrumbs expose your site hierarchy to users and search engines, and BreadcrumbList markup can replace the raw URL in results. On location sites they show \"Home > Locations > Bondi\".",
        fix: "Add visible breadcrumbs plus BreadcrumbList JSON-LD on service and location pages.",
        sources: &[BREADCRUMB],
        run: |ctx| {
            if let Some(page) = ctx.pages.iter().find(|page| page.has_breadcrumb) {
                return pass(format!("BreadcrumbList markup found on {}.", page.path));
            }
            if ctx.pages.iter().any(|page| page.has_breadcrumb_visible) {
                warn("Visible breadcrumb text found but no BreadcrumbList markup.")
            } else {
                warn("No breadcrumbs found on the crawled pages.")
            }
        },
    },
    Rule {
        id: "crawl_depth",
        title: "Key pages are reachable in a shallow crawl",
        category: "linking",
        weight: 1.0,
        why: "Pages deep in the click path get crawled less often and inherit less context. Local money pages should sit within two clicks of the homepage.",
        fix: "Flatten navigation, link location pages from a hub and footer, and avoid JS-only menus that hide links from crawlers.",
        sources: &[LINKS, SEARCH_ESSENTIALS],
        run: |ctx| {
            if ctx.pages.len() == 1 {
                return warn("Only one page was found. Navigation may be JavaScript-only or links may be blocked.");
            }
            let deep = ctx.pages.iter().filter(|page| page.depth >= 2).count();
            if (deep as f64 / ctx.pages.len() as f64) <= 0.5 {
                pass(format!(
                    "{} pages crawled, max depth {}.",
                    ctx.pages.len(),
                    ctx.s.max_depth
                ))
            } else {
                warn(format!(
                    "{}/{} crawled pages sit 2+ clicks from the homepage.",
                    deep,
                    ctx.pages.len()
                ))
            }
        },
    },
    Rule {
        id: "content_depth",
        title: "Money pages have enough substantive copy",
        category: "trust",
        weight: 2.0,
        why: "Thin pages struggle to demonstrate relevance or experience. Depth is not word count for its own sake; it is whether the page answers the questions a local buyer actually has.",
        fix: "Add service specifics: what is included, pricing ranges, response times, service area, guarantees, staff credentials, before/after evidence and FAQs.",
        sources: &[HELPFUL_CONTENT],
        run: |ctx| {
            let words = ctx.root.word_count;
            if words >= 600 {
                pass(format!("~{} words of visible copy on the starting page.", words))
            } else if words >= 300 {
                warn(format!(
                    "~{} words on the starting page — enough to start, thin for a competitive service.",
                    words
                ))
            } else {
                fail(format!("Only ~{} words of visible copy on the starting page.", words))
            }
        },
    },
    Rule {
        id: "helpful_content",
        title: "Trust pages and first-hand experience are evident",
        category: "trust",
        weight: 2.0,
        why: "Google's people-first content guidance rewards demonstrated experience, expertise, authoritativeness and trust (E-E-A-T). About/contact pages, named staff, credentials and real local proof are how a small business shows that.",
        fix: "Publish an About page with real people and credentials, a Contact page with NAP and a map, case studies or reviews, and clear service/pricing detail. Name the author of advice content.",
        sources: &[HELPFUL_CONTENT],
        run: |ctx| {
            let mut missing: Vec<String> = Vec::new();
            if ctx.s.about_link.is_none() {
                missing.push("About page link".to_string());
            }
            if ctx.s.contact_link.is_none() {
                missing.push("Contact page link".to_string());
            }
            let titled = ctx.pages.iter().filter(|page| !page.title.is_empty()).count();
            if titled != ctx.s.unique_titles {
                missing.push("unique page titles".to_string());
            }
            if missing.is_empty() {
                pass("About and Contact pages are linked from the crawl and page titles are unique.")
            } else {
                warn(format!("Missing: {}.", missing.join(", ")))
            }
        },
    },
    Rule {
        id: "image_alt",
        title: "Images have alternative text",
        category: "trust",
        weight: 2.0,
        why: "Alt text makes images accessible and gives image search context. For local businesses, geotagged filenames plus honest alt text (\"wheelchair ramp at the Bondi clinic\") reinforce the place.",
        fix: "Add concise, descriptive alt text to informative images. Use empty alt=\"\" for decorative images. Describe the place/object, not keyword lists.",
        sources: &[SEO_STARTER],
        run: |ctx| {
            let total: usize = ctx.pages.iter().map(|page| page.images.total).sum();
            let missing: usize = ctx.pages.iter().map(|page| page.images.missing_alt).sum();
            if total == 0 {
                return info("No <img> elements found on the crawled pages.");
            }
            let share = pct(missing, total);
            let text = format!("{}/{} images missing alt ({}%).", missing, total, share);
            if share <= 10 {
                pass(text)
            } else if share <= 35 {
                warn(text)
            } else {
                fail(text)
            }
        },
    },
    Rule {
        id: "map",
        title: "A map or directions link is embedded",
        category: "trust",
        weight: 1.0,
        why: "An embedded map or directions link confirms a physical location, reduces friction for \"near me\" visitors and mirrors your Google Business Profile location.",
        fix: "Embed a Google Maps iframe on Contact (lazy-loaded) and link \"Get directions\" to your Business Profile listing. Add hasMap to LocalBusiness markup.",
        sources: &[LOCAL_RANKING, LOCAL_BUSINESS],
        run: |ctx| {
            if ctx.s.map_embed || ctx.s.business.has_map.is_some() {
                pass("A map embed or hasMap link was found.")
            } else {
                warn("No map embed, maps link or hasMap value found.")
            }
        },
    },
    Rule {
        id: "reviews",
        title: "Review evidence is present and honest",
        category: "trust",
        weight: 1.0,
        why: "Reviews are a prominence signal and a conversion driver. Google no longer shows self-serving review stars for LocalBusiness, so the goal is visible, verifiable third-party reviews rather than markup tricks.",
        fix: "Collect reviews on your Google Business Profile (and one or two relevant directories), show a verifiable rating on-site, and add review/testimonial content. Do not mark up reviews you collected yourself as aggregateRating.",
        sources: &[REVIEW_SNIPPET, REVIEW_POLICY, LOCAL_RANKING],
        run: |ctx| {
            let has_markup = ctx.pages.iter().any(|page| page.has_rating);
            if ctx.s.review_mentions >= 3 || has_markup {
                pass(format!(
                    "{} review/testimonial mentions{} across the crawl.",
                    ctx.s.review_mentions,
                    if has_markup { " plus rating markup" } else { "" }
                ))
            } else {
                warn("Little review evidence found on-site. Third-party profile reviews still count for prominence.")
            }
        },
    },
    Rule {
        id: "faq",
        title: "Common questions are answered on-page",
        category: "trust",
        weight: 1.0,
        why: "FAQ content answers the long-tail local questions (\"do you service my suburb?\", \"how much is a call-out?\"). FAQ rich results are now limited to well-known authoritative government/health sites, so treat this as content quality, not a rich-result play.",
        fix: "Add a short FAQ to service and location pages using the exact language customers use. Answer plainly in the first sentence; add Q&A markup only if it is genuinely useful.",
        sources: &[FAQ, HELPFUL_CONTENT],
        run: |ctx| {
            if let Some(page) = ctx.pages.iter().find(|page| page.has_faq) {
                pass(format!(
                    "Q&A markup found on {} (rich results are limited, but the content still helps).",
                    page.path
                ))
            } else {
                info("No FAQ/QAPage markup. Add an FAQ section if customers ask repetitive local questions.")
            }
        },
    },
];

fn run_custom_rule(rule: &CustomRule, scan: &ScanResult) -> Option<CheckResult> {
    let regex = Regex::new(&rule.pattern).ok()?;
    let scope_pages: Vec<&PageAudit> = if rule.scope == "site" {
        scan.pages.iter().take(1).collect()
    } else {
        scan.pages.iter().collect()
    };
    let hits: Vec<&PageAudit> = scope_pages
        .into_iter()
        .filter(|page| regex.is_match(&format!("{} {}", page.title, page.text_sample)))
        .collect();
    let violated = if rule.rule_type == "must_contain" {
        hits.is_empty()
    } else {
        !hits.is_empty()
    };
    let category = if CATEGORIES.iter().any(|(key, _, _)| *key == rule.category) {
        rule.category.clone()
    } else {
        "trust".to_string()
    };
    Some(CheckResult {
        id: rule.id.clone(),
        title: rule.title.clone(),
        category,
        weight: rule.weight.clamp(0.5, 5.0),
        status: if violated { "fail".to_string() } else { "pass".to_string() },
        evidence: format!(
            "Pattern /{}/ {}.",
            rule.pattern,
            if hits.is_empty() {
                "matched no pages".to_string()
            } else {
                format!("matched {} page(s)", hits.len())
            }
        ),
        why: if rule.why.is_empty() {
            "Your own rule from the Standards tab.".to_string()
        } else {
            rule.why.clone()
        },
        fix: if rule.fix.is_empty() {
            "Adjust the page, or disable this custom rule in Standards.".to_string()
        } else {
            rule.fix.clone()
        },
        sources: Vec::new(),
        page: hits.first().map(|page| page.path.clone()),
        custom: true,
    })
}

fn band_for(score: u32) -> (&'static str, &'static str) {
    if score >= 85 {
        (
            "Excellent foundation",
            "The documented local signals are largely in place. Shift effort from fixing to measuring, publishing and review generation.",
        )
    } else if score >= 70 {
        (
            "Strong foundation",
            "Core local signals work. A handful of high-weight gaps are limiting reach — fix those before adding new pages.",
        )
    } else if score >= 55 {
        (
            "Developing",
            "The basics exist but are inconsistent. Entity clarity and location-page coverage are usually the fastest wins at this level.",
        )
    } else if score >= 40 {
        (
            "Weak",
            "Several important signals are missing. Start with indexability, NAP consistency and one strong location page rather than broad content.",
        )
    } else {
        (
            "Critical gaps",
            "Essential local signals are absent. Establish the entity (schema, NAP, profile links) and technical access before optimising copy.",
        )
    }
}

pub fn build_report(scan: &ScanResult, overrides: &Overrides) -> Report {
    let ctx = Ctx {
        root: &scan.pages[0],
        pages: &scan.pages,
        s: &scan.signals,
        keyword: &scan.signals.keyword,
        city: &scan.signals.city,
    };
    let disabled: HashSet<&str> = overrides.disabled_rules.iter().map(|id| id.as_str()).collect();

    let mut checks: Vec<CheckResult> = Vec::new();
    for rule in RULES {
        if disabled.contains(rule.id) {
            continue;
        }
        let outcome = (rule.run)(&ctx);
        let weight = overrides
            .weights
            .get(rule.id)
            .copied()
            .unwrap_or(rule.weight)
            .clamp(0.0, 5.0);
        checks.push(CheckResult {
            id: rule.id.to_string(),
            title: rule.title.to_string(),
            category: rule.category.to_string(),
            weight,
            status: outcome.status.to_string(),
            evidence: outcome.evidence,
            why: rule.why.to_string(),
            fix: rule.fix.to_string(),
            sources: rule
                .sources
                .iter()
                .map(|(label, url)| CheckSource {
                    label: label.to_string(),
                    url: url.to_string(),
                })
                .collect(),
            page: outcome.page,
            custom: false,
        });
    }

    for custom in &overrides.custom_rules {
        if disabled.contains(custom.id.as_str()) {
            continue;
        }
        if let Some(result) = run_custom_rule(custom, scan) {
            checks.push(result);
        }
    }

    let mut earned = 0.0;
    let mut scored_weight = 0.0;
    for check in &checks {
        if let Some(value) = status_value(&check.status) {
            earned += value * check.weight;
            scored_weight += check.weight;
        }
    }
    let score = if scored_weight > 0.0 {
        (earned / scored_weight * 100.0).round() as u32
    } else {
        0
    };

    let mut categories: Vec<CategoryScore> = Vec::new();
    for (key, label, _) in CATEGORIES {
        let in_category: Vec<&CheckResult> = checks
            .iter()
            .filter(|check| check.category == key && status_value(&check.status).is_some())
            .collect();
        if in_category.is_empty() {
            continue;
        }
        let total_weight: f64 = in_category.iter().map(|check| check.weight).sum();
        let category_earned: f64 = in_category
            .iter()
            .map(|check| status_value(&check.status).unwrap_or(0.0) * check.weight)
            .sum();
        categories.push(CategoryScore {
            key: key.to_string(),
            label: label.to_string(),
            score: if total_weight > 0.0 {
                (category_earned / total_weight * 100.0).round() as u32
            } else {
                0
            },
            weight: total_weight,
            passed: in_category.iter().filter(|check| check.status == "pass").count(),
            total: in_category.len(),
        });
    }

    let mut fixes: Vec<CheckResult> = checks
        .iter()
        .filter(|check| check.status == "fail" || check.status == "warn")
        .cloned()
        .collect();
    fixes.sort_by(|a, b| {
        status_rank(&a.status)
            .cmp(&status_rank(&b.status))
            .then_with(|| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal))
    });

    let working: Vec<CheckResult> = checks
        .iter()
        .filter(|check| check.status == "pass")
        .cloned()
        .collect();
    let pass_count = working.len();
    let total_checks = checks.len();
    let (band, summary) = band_for(score);

    Report {
        score,
        band: band.to_string(),
        summary: format!(
            "{} {} of {} documented checks pass (total weight {}).",
            summary,
            pass_count,
            total_checks,
            checks.iter().map(|check| check.weight).sum::<f64>().round() as i64
        ),
        categories,
        checks,
        working,
        fixes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scan() -> ScanResult {
        let mut scan = ScanResult::default();
        scan.pages = vec![PageAudit::default()];
        scan.signals = SiteSignals {
            host: "example.test".to_string(),
            ..Default::default()
        };
        scan
    }

    #[test]
    fn all_rules_run_and_score_is_bounded() {
        let report = build_report(&sample_scan(), &Overrides::default());
        assert_eq!(report.checks.len(), RULES.len());
        assert!(report.score <= 100);
        assert!(!report.categories.is_empty());
        let notes = report
            .checks
            .iter()
            .filter(|check| check.status == "info")
            .count();
        assert_eq!(
            report.working.len() + report.fixes.len() + notes,
            report.checks.len()
        );
    }

    #[test]
    fn overrides_disable_and_reweight_checks() {
        let mut overrides = Overrides::default();
        overrides.disabled_rules.push("https".to_string());
        overrides.weights.insert("title".to_string(), 5.0);
        let report = build_report(&sample_scan(), &overrides);
        assert!(!report.checks.iter().any(|check| check.id == "https"));
        let title = report
            .checks
            .iter()
            .find(|check| check.id == "title")
            .expect("title check present");
        assert_eq!(title.weight, 5.0);
    }
}
