use crate::rules::RULES;
use crate::types::*;
use crate::util::new_id;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub const PRESETS: [ModelPreset; 3] = [
    ModelPreset {
        id: "qwen15",
        label: "Qwen 2.5 1.5B",
        tag: "qwen2.5:1.5b",
        size: "1.1 GB",
        family: "Alibaba Qwen",
        notes: "Fastest of the three and unusually disciplined at JSON for its size. Best for quick passes and low-RAM machines.",
        gguf_repo: "Qwen/Qwen2.5-1.5B-Instruct-GGUF",
        gguf_file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
        gguf_bytes: 1_117_320_736,
        gguf_sha256: "6a1a2eb6d15622bf3c96857206351ba97e1af16c30d7a74ee38970e434e9407e",
        license: "Apache-2.0",
    },
    ModelPreset {
        id: "llama32",
        label: "Llama 3.2 3B",
        tag: "llama3.2:3b",
        size: "2.0 GB",
        family: "Meta Llama",
        notes: "Best reasoning-to-speed balance at 3B. Usually produces the clearest \"why this works\" explanations.",
        gguf_repo: "bartowski/Llama-3.2-3B-Instruct-GGUF",
        gguf_file: "Llama-3.2-3B-Instruct-Q4_K_M.gguf",
        gguf_bytes: 2_019_377_696,
        gguf_sha256: "6c1a2b41161032677be168d354123594c0e6e67d2b9227c84f296ad037c728ff",
        license: "Meta Llama 3.2 Community License",
    },
    ModelPreset {
        id: "gemma2",
        label: "Gemma 2 2B",
        tag: "gemma2:2b",
        size: "1.7 GB",
        family: "Google Gemma",
        notes: "Strong language understanding and local nuance; the JSON repair step catches its occasional formatting slips.",
        gguf_repo: "bartowski/gemma-2-2b-it-GGUF",
        gguf_file: "gemma-2-2b-it-Q4_K_M.gguf",
        gguf_bytes: 1_708_582_752,
        gguf_sha256: "e0aee85060f168f0f2d8473d7ea41ce2f3230c1bc1374847505ea599288a7787",
        license: "Gemma Terms of Use",
    },
];

fn ollama_host() -> String {
    std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
        .trim_end_matches('/')
        .to_string()
}

pub fn label_for(tag: &str) -> String {
    PRESETS
        .iter()
        .find(|preset| preset.tag == tag)
        .map(|preset| preset.label.to_string())
        .or_else(|| crate::models::find_by_tag(tag).map(|model| model.label))
        .unwrap_or_else(|| tag.to_string())
}

pub fn is_installed(tag: &str, installed: &[String]) -> bool {
    let base = tag.to_ascii_lowercase();
    installed.iter().any(|name| {
        let candidate = name.to_ascii_lowercase();
        candidate == base
            || candidate == format!("{}:latest", base)
            || candidate.split(':').next() == base.split(':').next()
    })
}

pub struct OllamaModel {
    pub name: String,
    pub size_bytes: u64,
    pub params: String,
}

pub struct OllamaStatus {
    pub online: bool,
    pub host: String,
    pub error: Option<String>,
    pub models: Vec<OllamaModel>,
}

pub fn ollama_status() -> OllamaStatus {
    let host = ollama_host();
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(2500))
        .build();
    match agent.get(&format!("{}/api/tags", host)).call() {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            let parsed: Value = serde_json::from_str(&text).unwrap_or(json!({}));
            let mut models = Vec::new();
            if let Some(list) = parsed.get("models").and_then(|value| value.as_array()) {
                for item in list {
                    let Some(name) = item.get("name").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    models.push(OllamaModel {
                        name: name.to_string(),
                        size_bytes: item.get("size").and_then(|value| value.as_u64()).unwrap_or(0),
                        params: item
                            .get("details")
                            .and_then(|details| details.get("parameter_size"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
            OllamaStatus { online: true, host, error: None, models }
        }
        Err(_error) => OllamaStatus {
            online: false,
            host: host.clone(),
            error: Some(format!(
                "No Ollama server at {}. Start it with \"ollama serve\".",
                host
            )),
            models: Vec::new(),
        },
    }
}

fn summarize_for_model(scan: &ScanResult, report: &Report, overrides: &Overrides) -> String {
    let root = &scan.pages[0];
    let pages: Vec<Value> = scan
        .pages
        .iter()
        .take(6)
        .map(|page| {
            json!({
                "path": page.path,
                "title": page.title,
                "description": page.meta_description.chars().take(160).collect::<String>(),
                "h1": page.h1.first().cloned().unwrap_or_default(),
                "words": page.word_count,
                "internalLinks": page.links.internal,
                "noindex": page.noindex,
                "schemaTypes": page.json_ld_types.iter().take(10).cloned().collect::<Vec<_>>(),
            })
        })
        .collect();
    let failing: Vec<Value> = report
        .fixes
        .iter()
        .take(14)
        .map(|check| {
            json!({
                "id": check.id,
                "status": check.status,
                "title": check.title,
                "evidence": check.evidence,
                "principle": check.why,
            })
        })
        .collect();
    let categories: Vec<Value> = report
        .categories
        .iter()
        .map(|category| json!({"key": category.key, "score": category.score}))
        .collect();
    let data = json!({
        "site": scan.host,
        "startUrl": scan.url,
        "target": {
            "service": if scan.signals.keyword.is_empty() { Value::Null } else { json!(scan.signals.keyword) },
            "city": if scan.signals.city.is_empty() { Value::Null } else { json!(scan.signals.city) },
        },
        "detectedBusiness": {
            "name": scan.signals.business.name,
            "types": scan.signals.business.types,
            "locality": scan.signals.business.locality,
            "region": scan.signals.business.region,
            "phone": scan.signals.business.phone,
            "hasGeo": scan.signals.business.lat.is_some(),
            "areaServed": scan.signals.business.area_served,
            "sameAsCount": scan.signals.business.same_as.len(),
        },
        "ruleScore": {"total": report.score, "band": report.band, "categories": categories},
        "crawl": {
            "pages": scan.pages.len(),
            "maxDepth": scan.signals.max_depth,
            "robotsFound": scan.signals.robots_found,
            "sitemapFound": scan.signals.sitemap_found,
            "sitemapUrls": scan.signals.sitemap_urls,
            "indexablePages": scan.signals.indexable_pages,
            "locationLinks": scan.signals.location_links.iter().take(8).map(|l| l.text.clone()).collect::<Vec<_>>(),
            "aboutLink": scan.signals.about_link.is_some(),
            "contactLink": scan.signals.contact_link.is_some(),
            "mapEmbed": scan.signals.map_embed,
            "avgResponseMs": scan.signals.avg_ms,
            "htmlKb": root.bytes / 1024,
        },
        "pages": pages,
        "failingChecks": failing,
        "userGuidance": if overrides.ai_guidance.is_empty() { Value::Null } else { json!(overrides.ai_guidance) },
        "userSources": overrides.sources.iter().map(|source| json!({"label": source.label, "url": source.url})).collect::<Vec<_>>(),
    });
    let text = data.to_string();
    if text.chars().count() > 9000 {
        format!("{}...", text.chars().take(9000).collect::<String>())
    } else {
        text
    }
}

fn system_prompt(allowed_keys: &[String], guidance: &str) -> String {
    let mut parts = vec![
        "You are a senior local SEO consultant who also teaches. You audit ONE website for local/geo search performance using documented Google guidance: Search Essentials, local ranking factors (relevance, distance, prominence), LocalBusiness structured data, Core Web Vitals and people-first content.".to_string(),
        "Hard rules:".to_string(),
        "- Ground every claim in the supplied crawl data. Never invent rankings, traffic numbers, review counts, competitors or Google features.".to_string(),
        "- Never promise a ranking position. Explain the mechanism of why a change helps.".to_string(),
        "- Prioritise what a small local business can actually ship this month.".to_string(),
        "- Be specific, not generic. Quote the evidence you were given.".to_string(),
        "- Return ONLY valid JSON. No markdown fences, no commentary outside the JSON.".to_string(),
        String::new(),
        "Allowed \"key\" values (use the closest match, or \"general\"):".to_string(),
        allowed_keys.join(", "),
        String::new(),
        "FIELD RULES:".to_string(),
        "- summary: two sentences in your own words about the single biggest local-search constraint on THIS site.".to_string(),
        "- suggestions: 3 to 6 items, ordered by impact. key must be one of the allowed check ids, or general.".to_string(),
        "- evidence: quote a specific fact from the crawl data (counts, missing tags, a page path).".to_string(),
        "- why_it_works: explain the mechanism, not a promise of ranking.".to_string(),
        "- fix: concrete steps a small business can ship this month.".to_string(),
        "- impact 1-5, effort low|medium|high, confidence 0-100.".to_string(),
        "- training_notes: one or two sentences teaching how to reason about local SEO for this site.".to_string(),
        String::new(),
        "SHAPE EXAMPLE — it describes a bakery in Lisbon, NOT this site. Never reuse its wording, titles or numbers:".to_string(),
        "{\"summary\":\"The bakery lists three different opening hours across the site, so Google cannot trust any of them for map results.\",\"suggestions\":[{\"key\":\"hours\",\"title\":\"Publish one set of opening hours\",\"category\":\"entity\",\"severity\":\"high\",\"evidence\":\"The footer, contact page and JSON-LD all list different Saturday hours.\",\"why_it_works\":\"Google cross-checks hours against its own business data; conflicting values weaken the entity and can show a closed storefront.\",\"fix\":\"Choose the real hours, update the footer, contact page and LocalBusiness schema, then re-verify in the Business Profile.\",\"impact\":4,\"effort\":\"low\",\"confidence\":92}],\"training_notes\":\"Hours feel like a small detail, but they are a trust signal Google can verify against the real world.\"}".to_string(),
        String::new(),
        "Give at most 6 suggestions. Prefer fewer, sharper suggestions over a long list.".to_string(),
        String::new(),
        "FINAL RULE: reply with the JSON object only. Your first character must be { and your last must be }. No headings, no markdown, no code fences, no text before or after.".to_string(),
    ];
    if !guidance.is_empty() {
        parts.push(format!(
            "The site owner added this guidance, honour it: {}",
            guidance
        ));
    }
    parts.join("\n")
}

struct RawCall {
    ok: bool,
    content: String,
    error: Option<String>,
    latency_ms: u64,
    tokens: Option<u64>,
}

fn call_ollama(model: &str, system: &str, user: &str) -> RawCall {
    let started = Instant::now();
    let host = ollama_host();
    let payload = json!({
        "model": model,
        "stream": false,
        "format": "json",
        "options": {
            "temperature": 0.2,
            "num_ctx": 8192,
            "num_predict": 900,
            "repeat_penalty": 1.05
        },
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    });
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .build();

    let result = agent
        .post(&format!("{}/api/chat", host))
        .set("content-type", "application/json")
        .send_string(&payload.to_string());

    match result {
        Ok(resp) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let text = resp.into_string().unwrap_or_default();
            let parsed: Value = serde_json::from_str(&text).unwrap_or(json!({}));
            let content = parsed
                .get("message")
                .and_then(|message| message.get("content"))
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            RawCall {
                ok: true,
                content,
                error: None,
                latency_ms,
                tokens: parsed.get("eval_count").and_then(|value| value.as_u64()),
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let latency_ms = started.elapsed().as_millis() as u64;
            let detail = resp.into_string().unwrap_or_default();
            let hint = if code == 404 {
                format!(" Run \"ollama pull {}\" first.", model)
            } else {
                String::new()
            };
            RawCall {
                ok: false,
                content: String::new(),
                error: Some(format!(
                    "HTTP {}{}{}",
                    code,
                    if detail.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", detail.chars().take(200).collect::<String>())
                    },
                    hint
                )),
                latency_ms,
                tokens: None,
            }
        }
        Err(_error) => RawCall {
            ok: false,
            content: String::new(),
            error: Some(format!(
                "Could not reach Ollama at {}. Start it with \"ollama serve\", then run \"ollama pull {}\".",
                host, model
            )),
            latency_ms: started.elapsed().as_millis() as u64,
            tokens: None,
        },
    }
}

fn trailing_comma_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r",\s*([}\]])").unwrap())
}

fn extract_json(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return None;
    }
    let mut cleaned = text.trim();
    cleaned = cleaned.strip_prefix("```json").unwrap_or(cleaned);
    cleaned = cleaned.strip_prefix("```").unwrap_or(cleaned);
    cleaned = cleaned.strip_suffix("```").unwrap_or(cleaned);
    let cleaned = cleaned.trim();
    let first = cleaned.find('{')?;
    let last = cleaned.rfind('}')?;
    if last <= first {
        return None;
    }
    let candidate = &cleaned[first..=last];
    if let Ok(value) = serde_json::from_str::<Value>(candidate) {
        return Some(value);
    }
    let repaired = trailing_comma_re().replace_all(candidate, "$1").into_owned();
    serde_json::from_str::<Value>(&repaired).ok()
}

fn clamp_u8(value: Option<&Value>, min: u8, max: u8, fallback: u8) -> u8 {
    let parsed = match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.parse::<f64>().ok(),
        _ => None,
    };
    match parsed {
        Some(number) if number.is_finite() => number.round().clamp(min as f64, max as f64) as u8,
        _ => fallback,
    }
}

fn text_of(value: Option<&Value>) -> String {
    value
        .and_then(|item| item.as_str())
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

fn trim_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn normalize_suggestions(parsed: &Value) -> (Vec<AiSuggestion>, u32) {
    let list = parsed
        .get("suggestions")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut suggestions = Vec::new();
    let mut complete = 0usize;

    for item in list {
        let Some(node) = item.as_object() else { continue };
        let key_raw = text_of(node.get("key")).to_ascii_lowercase();
        let key: String = key_raw
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch == '-' {
                    ch
                } else {
                    '_'
                }
            })
            .take(48)
            .collect();
        let key = if key.is_empty() { "general".to_string() } else { key };
        let title = text_of(node.get("title"));
        let evidence = text_of(node.get("evidence"));
        let why = text_of(node.get("why_it_works"));
        let fix = text_of(node.get("fix"));
        let category_raw = text_of(node.get("category")).to_ascii_lowercase();
        let severity_raw = text_of(node.get("severity")).to_ascii_lowercase();
        let effort_raw = text_of(node.get("effort")).to_ascii_lowercase();
        if !title.is_empty() && !evidence.is_empty() && !why.is_empty() && !fix.is_empty() {
            complete += 1;
        }
        if title.is_empty() {
            continue;
        }
        suggestions.push(AiSuggestion {
            key,
            title: trim_chars(&title, 160),
            category: if matches!(
                category_raw.as_str(),
                "entity" | "onpage" | "linking" | "technical" | "trust"
            ) {
                category_raw
            } else {
                "onpage".to_string()
            },
            severity: if matches!(
                severity_raw.as_str(),
                "critical" | "high" | "medium" | "low"
            ) {
                severity_raw
            } else {
                "medium".to_string()
            },
            evidence: trim_chars(&evidence, 400),
            why_it_works: trim_chars(&why, 600),
            fix: trim_chars(&fix, 700),
            impact: clamp_u8(node.get("impact"), 1, 5, 3),
            effort: if matches!(effort_raw.as_str(), "low" | "medium" | "high") {
                effort_raw
            } else {
                "medium".to_string()
            },
            confidence: clamp_u8(node.get("confidence"), 0, 100, 60),
        });
    }

    let compliance = if suggestions.is_empty() {
        0
    } else {
        ((complete * 100 + suggestions.len() / 2) / suggestions.len()) as u32
    };
    (suggestions, compliance)
}

fn run_model(model: &str, system: &str, user: &str) -> ModelRun {
    let embedded = crate::engine::has_embedded(model);
    let custom = crate::models::find_by_tag(model);
    let mut response = if embedded {
        let result = crate::engine::generate(model, system, user);
        RawCall {
            ok: result.ok,
            content: result.content,
            error: result.error,
            latency_ms: result.latency_ms,
            tokens: result.tokens,
        }
    } else if let Some(custom) = custom {
        RawCall {
            ok: false,
            content: String::new(),
            error: Some(format!(
                "{} is not downloaded yet. Download it in the Model lab before running it.",
                custom.label
            )),
            latency_ms: 0,
            tokens: None,
        }
    } else {
        call_ollama(model, system, user)
    };
    if !response.ok && !embedded {
        if let Some(hint) = response.error.as_mut() {
            if PRESETS.iter().any(|preset| preset.tag == model) {
                hint.push_str(" Or download the built-in model in the Model lab — no Ollama needed.");
            }
        }
    }
    let label = label_for(model);
    if !response.ok {
        return ModelRun {
            model: model.to_string(),
            label,
            ok: false,
            error: response.error,
            latency_ms: response.latency_ms,
            valid_json: false,
            tokens: response.tokens,
            suggestions: Vec::new(),
            summary: None,
            training_notes: None,
            schema_compliance: 0,
        };
    }
    let Some(parsed) = extract_json(&response.content) else {
        eprintln!(
            "[ai] {} returned {} chars without JSON: {}",
            model,
            response.content.chars().count(),
            response.content.chars().take(400).collect::<String>()
        );
        return ModelRun {
            model: model.to_string(),
            label,
            ok: false,
            error: Some("The model did not return parseable JSON. Try again or raise num_predict.".to_string()),
            latency_ms: response.latency_ms,
            valid_json: false,
            tokens: response.tokens,
            suggestions: Vec::new(),
            summary: None,
            training_notes: None,
            schema_compliance: 0,
        };
    };
    let (suggestions, compliance) = normalize_suggestions(&parsed);
    ModelRun {
        model: model.to_string(),
        label,
        ok: true,
        error: None,
        latency_ms: response.latency_ms,
        valid_json: true,
        tokens: response.tokens,
        suggestions,
        summary: Some(trim_chars(&text_of(parsed.get("summary")), 600)),
        training_notes: Some(trim_chars(&text_of(parsed.get("training_notes")), 600)),
        schema_compliance: compliance,
    }
}

fn curate(runs: &[ModelRun]) -> (Vec<CuratedSuggestion>, Vec<CuratedSuggestion>) {
    let successful: Vec<&ModelRun> = runs.iter().filter(|run| run.ok).collect();
    let mut groups: HashMap<String, Vec<(usize, AiSuggestion)>> = HashMap::new();

    for (index, run) in runs.iter().enumerate() {
        if !run.ok {
            continue;
        }
        for suggestion in &run.suggestions {
            let key = if suggestion.key.is_empty() {
                "general".to_string()
            } else {
                suggestion.key.clone()
            };
            groups.entry(key).or_default().push((index, suggestion.clone()));
        }
    }

    let mut curated: Vec<CuratedSuggestion> = Vec::new();
    for (key, items) in groups {
        let mut ordered = items.clone();
        ordered.sort_by(|a, b| {
            b.1.confidence
                .cmp(&a.1.confidence)
                .then_with(|| b.1.impact.cmp(&a.1.impact))
                .then_with(|| {
                    runs[b.0]
                        .schema_compliance
                        .cmp(&runs[a.0].schema_compliance)
                })
        });
        let canonical = ordered[0].1.clone();
        let mut voters: Vec<String> = Vec::new();
        for (index, _) in &ordered {
            let model = runs[*index].model.clone();
            if !voters.contains(&model) {
                voters.push(model);
            }
        }
        let why = ordered
            .iter()
            .find(|(_, suggestion)| suggestion.why_it_works.chars().count() > 40)
            .map(|(_, suggestion)| suggestion.why_it_works.clone())
            .unwrap_or_else(|| canonical.why_it_works.clone());
        let fix = ordered
            .iter()
            .find(|(_, suggestion)| suggestion.fix.chars().count() > 40)
            .map(|(_, suggestion)| suggestion.fix.clone())
            .unwrap_or_else(|| canonical.fix.clone());
        let avg_confidence = (ordered
            .iter()
            .map(|(_, suggestion)| suggestion.confidence as u32)
            .sum::<u32>()
            + ordered.len() as u32 / 2)
            / ordered.len() as u32;
        curated.push(CuratedSuggestion {
            key,
            title: canonical.title,
            category: canonical.category,
            severity: canonical.severity,
            evidence: canonical.evidence,
            why,
            fix,
            impact: canonical.impact,
            effort: canonical.effort,
            vote_count: voters.len(),
            agreement: if successful.is_empty() {
                0
            } else {
                (voters.len() * 100 / successful.len()) as u32
            },
            votes: voters,
            confidence: avg_confidence,
        });
    }

    curated.sort_by(|a, b| {
        b.vote_count
            .cmp(&a.vote_count)
            .then_with(|| b.impact.cmp(&a.impact))
            .then_with(|| b.confidence.cmp(&a.confidence))
    });
    let split = curated.iter().position(|item| item.vote_count == 1);
    match split {
        Some(index) => {
            let unique = curated.split_off(index);
            (curated, unique)
        }
        None => (curated, Vec::new()),
    }
}

fn score_model(
    run: &ModelRun,
    fastest: u64,
    majority_keys: &HashSet<String>,
    comparisons: usize,
) -> ModelMetric {
    if !run.ok {
        return ModelMetric {
            model: run.model.clone(),
            label: run.label.clone(),
            latency_ms: run.latency_ms,
            valid_json: false,
            suggestions: run.suggestions.len(),
            schema_compliance: 0,
            consensus_match: 0,
            score: 0,
        };
    }
    let latency_score = if run.latency_ms > 0 {
        ((fastest as f64 / run.latency_ms as f64) * 100.0).min(100.0)
    } else {
        0.0
    };
    let consensus_match = if comparisons >= 2 {
        if run.suggestions.is_empty() {
            0
        } else {
            let matches = run
                .suggestions
                .iter()
                .filter(|suggestion| {
                    majority_keys.contains(if suggestion.key.is_empty() {
                        "general"
                    } else {
                        suggestion.key.as_str()
                    })
                })
                .count();
            (matches * 100 / run.suggestions.len()) as u32
        }
    } else {
        100
    };
    let count = run.suggestions.len();
    let count_fit = if (3..=8).contains(&count) {
        100.0
    } else if count >= 1 {
        55.0
    } else {
        0.0
    };
    let valid = if run.valid_json { 100.0 } else { 0.0 };
    let score = (0.25 * valid
        + 0.2 * run.schema_compliance as f64
        + 0.2 * consensus_match as f64
        + 0.25 * latency_score
        + 0.1 * count_fit)
        .round() as u32;
    ModelMetric {
        model: run.model.clone(),
        label: run.label.clone(),
        latency_ms: run.latency_ms,
        valid_json: run.valid_json,
        suggestions: count,
        schema_compliance: run.schema_compliance,
        consensus_match,
        score,
    }
}

pub fn compare_models(
    scan: &ScanResult,
    report: &Report,
    models: &[String],
    mode: &str,
    overrides: &Overrides,
) -> Result<CompareResult, String> {
    let mut unique_models: Vec<String> = Vec::new();
    for model in models {
        let trimmed = model.trim().to_string();
        if !trimmed.is_empty() && !unique_models.contains(&trimmed) {
            unique_models.push(trimmed);
        }
    }
    if unique_models.is_empty() {
        return Err("Select at least one model.".to_string());
    }
    if unique_models.len() > 6 {
        return Err("Select no more than 6 models per run.".to_string());
    }

    let mut allowed: Vec<String> = RULES.iter().map(|rule| rule.id.to_string()).collect();
    for custom in &overrides.custom_rules {
        allowed.push(custom.id.clone());
    }
    allowed.push("general".to_string());

    let system = system_prompt(&allowed, &overrides.ai_guidance);
    let user = format!(
        "Audit this crawl data:\n{}",
        summarize_for_model(scan, report, overrides)
    );

    let runs: Vec<ModelRun> = if mode == "sequential" {
        unique_models
            .iter()
            .map(|model| run_model(model, &system, &user))
            .collect()
    } else {
        std::thread::scope(|scope| {
            let handles: Vec<_> = unique_models
                .iter()
                .map(|model| {
                    let system_ref = &system;
                    let user_ref = &user;
                    scope.spawn(move || run_model(model, system_ref, user_ref))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().unwrap_or_else(|_| ModelRun {
                        ok: false,
                        error: Some("Model worker panicked.".to_string()),
                        ..Default::default()
                    })
                })
                .collect()
        })
    };

    let successful = runs.iter().filter(|run| run.ok).count();
    let (curated, unique) = curate(&runs);
    let majority_keys: HashSet<String> = curated
        .iter()
        .filter(|item| item.vote_count >= 2)
        .map(|item| item.key.clone())
        .collect();
    let fastest = runs
        .iter()
        .filter(|run| run.ok && run.latency_ms > 0)
        .map(|run| run.latency_ms)
        .min()
        .unwrap_or(1);
    let metrics: Vec<ModelMetric> = runs
        .iter()
        .map(|run| score_model(run, fastest, &majority_keys, successful))
        .collect();

    Ok(CompareResult {
        id: new_id("c"),
        scan_id: scan.id.clone(),
        host: scan.host.clone(),
        at: crate::util::now_iso(),
        mode: if mode == "sequential" { "sequential" } else { "parallel" }.to_string(),
        runs,
        curated,
        unique,
        metrics,
        leaderboard: Vec::new(),
    })
}

pub fn aggregate_leaderboard(history: &[CompareResult]) -> Vec<LeaderboardRow> {
    struct Entry {
        label: String,
        runs: usize,
        valid: usize,
        latency: Vec<u64>,
        schema: Vec<u32>,
        consensus: Vec<u32>,
        scores: Vec<u32>,
        last: String,
    }

    let mut by_model: HashMap<String, Entry> = HashMap::new();
    for compare in history {
        for metric in &compare.metrics {
            let entry = by_model.entry(metric.model.clone()).or_insert_with(|| Entry {
                label: metric.label.clone(),
                runs: 0,
                valid: 0,
                latency: Vec::new(),
                schema: Vec::new(),
                consensus: Vec::new(),
                scores: Vec::new(),
                last: compare.at.clone(),
            });
            entry.runs += 1;
            if metric.valid_json {
                entry.valid += 1;
            }
            entry.latency.push(metric.latency_ms);
            entry.schema.push(metric.schema_compliance);
            entry.consensus.push(metric.consensus_match);
            entry.scores.push(metric.score);
            if compare.at > entry.last {
                entry.last = compare.at.clone();
            }
        }
    }

    let average = |values: &[u32]| -> u32 {
        if values.is_empty() {
            0
        } else {
            (values.iter().sum::<u32>() as f64 / values.len() as f64).round() as u32
        }
    };
    let average_u64 = |values: &[u64]| -> u64 {
        if values.is_empty() {
            0
        } else {
            (values.iter().sum::<u64>() as f64 / values.len() as f64).round() as u64
        }
    };

    let mut rows: Vec<LeaderboardRow> = by_model
        .into_iter()
        .map(|(model, entry)| LeaderboardRow {
            model,
            label: entry.label,
            runs: entry.runs,
            valid_json_rate: ((entry.valid * 100) as f64 / entry.runs as f64).round() as u32,
            avg_latency_ms: average_u64(&entry.latency),
            avg_schema: average(&entry.schema),
            avg_consensus: average(&entry.consensus),
            score: average(&entry.scores),
            last_run: entry.last,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.avg_latency_ms.cmp(&b.avg_latency_ms))
    });
    rows
}
