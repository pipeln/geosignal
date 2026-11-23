use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default)]
pub struct Anchor {
    pub href: String,
    pub text: String,
    pub rel: String,
}

#[derive(Debug, Clone, Default)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ImageRef {
    pub alt: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptRef {
    pub src: Option<String>,
    pub is_async: bool,
    pub defer: bool,
    pub json_ld: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LinkRef {
    pub rel: String,
    pub href: String,
    pub hreflang: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedDoc {
    pub title: String,
    pub lang: String,
    pub metas: HashMap<String, String>,
    pub links: Vec<LinkRef>,
    pub anchors: Vec<Anchor>,
    pub headings: Vec<Heading>,
    pub images: Vec<ImageRef>,
    pub scripts: Vec<ScriptRef>,
    pub json_ld: Vec<Value>,
    pub text: String,
    pub word_count: usize,
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?is)</?([a-zA-Z][a-zA-Z0-9:-]*)((?:[^>"']|"[^"]*"|'[^']*')*?)\s*/?>"#).unwrap())
}

fn script_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<script\b([^>]*)>(.*?)</script\s*>").unwrap())
}

fn attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)([^\s"'<>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#).unwrap()
    })
}

fn body_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<body[\s>]").unwrap())
}

fn entity_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"&(#x?[0-9a-fA-F]+|[a-zA-Z][a-zA-Z0-9]*);").unwrap())
}

pub fn decode_entities(input: &str) -> String {
    entity_re()
        .replace_all(input, |caps: &regex::Captures| {
            let group = &caps[1];
            if let Some(rest) = group.strip_prefix('#') {
                let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    rest.parse::<u32>().ok()
                };
                if let Some(code) = code {
                    if let Some(ch) = char::from_u32(code) {
                        return ch.to_string();
                    }
                }
                return caps[0].to_string();
            }
            let named = match group.to_ascii_lowercase().as_str() {
                "amp" => "&",
                "lt" => "<",
                "gt" => ">",
                "quot" => "\"",
                "apos" => "'",
                "nbsp" => " ",
                "ndash" => "–",
                "mdash" => "—",
                "hellip" => "…",
                "rsquo" => "’",
                "lsquo" => "‘",
                "ldquo" => "“",
                "rdquo" => "”",
                "copy" => "©",
                "reg" => "®",
                "trade" => "™",
                "deg" => "°",
                "middot" => "·",
                "bull" => "•",
                _ => return caps[0].to_string(),
            };
            named.to_string()
        })
        .into_owned()
}

pub fn normalize_space(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse_attrs(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if raw.is_empty() {
        return out;
    }
    for caps in attr_re().captures_iter(raw) {
        let name = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let value = caps
            .get(2)
            .or_else(|| caps.get(3))
            .or_else(|| caps.get(4))
            .map(|m| decode_entities(m.as_str()))
            .unwrap_or_default();
        out.entry(name).or_insert(value);
    }
    out
}

fn clean_for_parsing(html: &str) -> String {
    let script = Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").unwrap();
    let style = Regex::new(r"(?is)<style\b[^>]*>.*?</style\s*>").unwrap();
    let template = Regex::new(r"(?is)<template\b[^>]*>.*?</template\s*>").unwrap();
    let comment = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let step1 = script.replace_all(html, " ");
    let step2 = style.replace_all(&step1, " ");
    let step3 = template.replace_all(&step2, " ");
    comment.replace_all(&step3, " ").into_owned()
}

pub fn parse_html(html: &str) -> ParsedDoc {
    let mut doc = ParsedDoc::default();

    if let Some(caps) = Regex::new(r"(?is)<html\b([^>]*)>").unwrap().captures(html) {
        let attrs = parse_attrs(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        doc.lang = attrs.get("lang").cloned().unwrap_or_default().trim().to_string();
    }

    for caps in script_re().captures_iter(html) {
        let attrs = parse_attrs(caps.get(1).map(|m| m.as_str()).unwrap_or(""));
        let script_type = attrs.get("type").cloned().unwrap_or_default().to_lowercase();
        let is_json_ld = script_type.contains("ld+json");
        let body = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        doc.scripts.push(ScriptRef {
            src: attrs.get("src").cloned(),
            is_async: attrs.contains_key("async"),
            defer: attrs.contains_key("defer"),
            json_ld: is_json_ld,
        });
        if !is_json_ld {
            continue;
        }
        let trimmed = body
            .trim()
            .trim_start_matches("<!--")
            .trim_end_matches("-->")
            .trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            if let Some(array) = value.as_array() {
                for item in array {
                    doc.json_ld.push(item.clone());
                }
            } else {
                doc.json_ld.push(value);
            }
        }
    }

    let clean = clean_for_parsing(html);
    let text_start = body_re()
        .find(&clean)
        .map(|m| m.start())
        .unwrap_or(0);

    let mut pending_title: Option<String> = None;
    let mut pending_anchor: Option<Anchor> = None;
    let mut pending_heading: Option<Heading> = None;
    let mut text_parts: Vec<String> = Vec::new();
    let mut last_index = 0usize;

    for caps in tag_re().captures_iter(&clean) {
        let whole = caps.get(0).unwrap();
        let between = &clean[last_index..whole.start()];
        last_index = whole.end();

        if !between.is_empty() {
            if let Some(title) = pending_title.as_mut() {
                title.push_str(&decode_entities(between));
            }
            if whole.start() >= text_start {
                let text = decode_entities(between);
                text_parts.push(text.clone());
                if let Some(anchor) = pending_anchor.as_mut() {
                    anchor.text.push_str(&text);
                }
                if let Some(heading) = pending_heading.as_mut() {
                    heading.text.push_str(&text);
                }
            }
        }

        let name = caps.get(1).map(|m| m.as_str().to_lowercase()).unwrap_or_default();
        let closing = whole.as_str().starts_with("</");
        let attrs = if closing {
            HashMap::new()
        } else {
            parse_attrs(caps.get(2).map(|m| m.as_str()).unwrap_or(""))
        };

        if closing {
            match name.as_str() {
                "a" => {
                    if let Some(anchor) = pending_anchor.take() {
                        doc.anchors.push(Anchor {
                            href: anchor.href.trim().to_string(),
                            text: normalize_space(&anchor.text),
                            rel: anchor.rel,
                        });
                    }
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    if let Some(heading) = pending_heading.take() {
                        doc.headings.push(Heading {
                            level: heading.level,
                            text: normalize_space(&heading.text),
                        });
                    }
                }
                "title" => {
                    if let Some(title) = pending_title.take() {
                        doc.title = normalize_space(&title);
                    }
                }
                _ => {}
            }
            continue;
        }

        match name.as_str() {
            "a" => {
                if let Some(anchor) = pending_anchor.take() {
                    doc.anchors.push(Anchor {
                        href: anchor.href.trim().to_string(),
                        text: normalize_space(&anchor.text),
                        rel: anchor.rel,
                    });
                }
                pending_anchor = Some(Anchor {
                    href: attrs.get("href").cloned().unwrap_or_default(),
                    text: String::new(),
                    rel: attrs.get("rel").cloned().unwrap_or_default(),
                });
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if let Some(heading) = pending_heading.take() {
                    doc.headings.push(Heading {
                        level: heading.level,
                        text: normalize_space(&heading.text),
                    });
                }
                let level = name[1..2].parse::<u8>().unwrap_or(2);
                pending_heading = Some(Heading { level, text: String::new() });
            }
            "title" => {
                if pending_title.is_none() {
                    pending_title = Some(String::new());
                }
            }
            "meta" => {
                let key = attrs
                    .get("name")
                    .or_else(|| attrs.get("property"))
                    .or_else(|| attrs.get("itemprop"))
                    .map(|value| value.to_lowercase())
                    .unwrap_or_default();
                if !key.is_empty() {
                    doc.metas
                        .entry(key)
                        .or_insert_with(|| attrs.get("content").cloned().unwrap_or_default());
                }
            }
            "link" => {
                let rel = attrs.get("rel").cloned().unwrap_or_default().to_lowercase();
                if !rel.is_empty() {
                    doc.links.push(LinkRef {
                        rel,
                        href: attrs.get("href").cloned().unwrap_or_default(),
                        hreflang: attrs.get("hreflang").cloned(),
                    });
                }
            }
            "img" => {
                doc.images.push(ImageRef {
                    alt: attrs.get("alt").cloned(),
                    width: attrs.get("width").cloned(),
                    height: attrs.get("height").cloned(),
                });
            }
            _ => {}
        }
    }

    if let Some(anchor) = pending_anchor.take() {
        doc.anchors.push(Anchor {
            href: anchor.href.trim().to_string(),
            text: normalize_space(&anchor.text),
            rel: anchor.rel,
        });
    }
    if let Some(heading) = pending_heading.take() {
        doc.headings.push(Heading {
            level: heading.level,
            text: normalize_space(&heading.text),
        });
    }
    if let Some(title) = pending_title.take() {
        doc.title = normalize_space(&title);
    }

    doc.text = normalize_space(&text_parts.join(" "));
    doc.word_count = doc.text.split_whitespace().count();
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_elements() {
        let html = r#"<html lang="en-AU"><head><title>Plumber in Bondi | ABC</title><meta name="description" content="Emergency plumbing"><link rel="canonical" href="https://x.test/"><script type="application/ld+json">{"@type":"LocalBusiness","name":"ABC"}</script></head><body><h1>Plumber in Bondi</h1><h2>Areas we serve</h2><a href="/contact">Contact</a><a href="tel:+61123456789">Call</a><img src="a.jpg"></body></html>"#;
        let doc = parse_html(html);
        assert_eq!(doc.title, "Plumber in Bondi | ABC");
        assert_eq!(doc.lang, "en-AU");
        assert_eq!(doc.metas.get("description").map(String::as_str), Some("Emergency plumbing"));
        assert_eq!(doc.headings.len(), 2);
        assert_eq!(doc.headings[0].level, 1);
        assert_eq!(doc.headings[0].text, "Plumber in Bondi");
        assert_eq!(doc.headings[1].level, 2);
        assert_eq!(doc.anchors.len(), 2);
        assert_eq!(doc.images.len(), 1);
        assert!(doc.images[0].alt.is_none());
        assert_eq!(doc.json_ld.len(), 1);
        assert!(doc.text.contains("Plumber in Bondi"));
    }

    #[test]
    fn extracts_entities_and_resolves_links() {
        assert_eq!(decode_entities("Fish &amp; Chips &#39;n&#39; &#x27;rolls&#x27;"), "Fish & Chips 'n' 'rolls'");
        assert_eq!(
            resolve_url("/contact", "https://example.com/about").as_deref(),
            Some("https://example.com/contact")
        );
        assert_eq!(resolve_url("tel:+123", "https://example.com"), None);
        assert_eq!(resolve_url("#top", "https://example.com"), None);
    }
}

pub fn resolve_url(href: &str, base: &str) -> Option<String> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.to_ascii_lowercase().starts_with("mailto:")
        || trimmed.to_ascii_lowercase().starts_with("tel:")
        || trimmed.to_ascii_lowercase().starts_with("javascript:")
        || trimmed.to_ascii_lowercase().starts_with("data:")
        || trimmed.to_ascii_lowercase().starts_with("sms:")
    {
        return None;
    }
    let base_url = url::Url::parse(base).ok()?;
    base_url.join(trimmed).ok().map(|url| url.to_string())
}
