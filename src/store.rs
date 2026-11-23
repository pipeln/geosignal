use crate::ai::aggregate_leaderboard;
use crate::types::*;
use crate::util::{new_id, now_iso};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_GUIDANCE: &str = "This business is a small local service provider. Prefer fixes that need copy edits and schema changes over new tooling. Always name the city, and never suggest ranking guarantees.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredScan {
    pub scan: ScanResult,
    pub report: Report,
    pub saved_at: String,
}

fn data_dir() -> PathBuf {
    crate::util::data_dir()
}

fn scans_dir() -> PathBuf {
    data_dir().join("scans")
}

fn compares_path() -> PathBuf {
    data_dir().join("compares.json")
}

fn overrides_path() -> PathBuf {
    data_dir().join("overrides.json")
}

fn ensure_dirs() {
    let _ = fs::create_dir_all(scans_dir());
}

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), String> {
    ensure_dirs();
    let text = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

pub fn save_scan(scan: &ScanResult, report: &Report) -> Result<(), String> {
    let stored = StoredScan {
        scan: scan.clone(),
        report: report.clone(),
        saved_at: now_iso(),
    };
    write_json(&scans_dir().join(format!("{}.json", scan.id)), &stored)
}

pub fn get_scan(id: &str) -> Option<StoredScan> {
    if !safe_id(id) {
        return None;
    }
    let path = scans_dir().join(format!("{}.json", id));
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn list_scans() -> Vec<ScanListEntry> {
    ensure_dirs();
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(scans_dir()) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else { continue };
            let Ok(stored) = serde_json::from_str::<StoredScan>(&text) else {
                continue;
            };
            entries.push(ScanListEntry {
                id: stored.scan.id.clone(),
                url: stored.scan.url.clone(),
                host: stored.scan.host.clone(),
                finished_at: stored.scan.finished_at.clone(),
                pages: stored.scan.pages.len(),
                score: stored.report.score,
            });
        }
    }
    entries.sort_by(|a, b| b.finished_at.cmp(&a.finished_at));
    entries.truncate(30);
    entries
}

pub fn delete_scan(id: &str) -> bool {
    if !safe_id(id) {
        return false;
    }
    fs::remove_file(scans_dir().join(format!("{}.json", id))).is_ok()
}

pub fn list_compares() -> Vec<CompareResult> {
    let Ok(text) = fs::read_to_string(compares_path()) else {
        return Vec::new();
    };
    let mut entries: Vec<CompareResult> = serde_json::from_str(&text).unwrap_or_default();
    entries.sort_by(|a, b| b.at.cmp(&a.at));
    entries
}

pub fn save_compare(result: &CompareResult) -> Result<(), String> {
    let mut history = list_compares();
    history.retain(|entry| entry.id != result.id);
    history.insert(0, result.clone());
    history.truncate(120);
    write_json(&compares_path(), &history)
}

pub fn get_compare(id: &str) -> Option<CompareResult> {
    list_compares().into_iter().find(|entry| entry.id == id)
}

pub fn get_leaderboard() -> Vec<LeaderboardRow> {
    aggregate_leaderboard(&list_compares())
}

pub fn default_overrides() -> Overrides {
    Overrides {
        disabled_rules: Vec::new(),
        weights: HashMap::new(),
        custom_rules: Vec::new(),
        ai_guidance: DEFAULT_GUIDANCE.to_string(),
        sources: Vec::new(),
    }
}

fn valid_category(value: &str) -> bool {
    matches!(value, "entity" | "onpage" | "linking" | "technical" | "trust")
}

pub fn sanitize_overrides(raw: Overrides) -> Overrides {
    let mut clean = default_overrides();

    clean.disabled_rules = raw
        .disabled_rules
        .into_iter()
        .filter(|id| safe_id(id))
        .take(200)
        .collect();

    for (id, value) in raw.weights {
        if safe_id(&id) && value.is_finite() && (0.0..=5.0).contains(&value) {
            clean.weights.insert(id, value);
        }
    }

    for rule in raw.custom_rules.into_iter().take(40) {
        let pattern = rule.pattern.trim().to_string();
        if pattern.is_empty() || pattern.chars().count() > 500 || Regex::new(&pattern).is_err() {
            continue;
        }
        clean.custom_rules.push(CustomRule {
            id: if safe_id(&rule.id) {
                rule.id
            } else {
                new_id("custom")
            },
            title: rule.title.chars().take(120).collect(),
            category: if valid_category(&rule.category) {
                rule.category
            } else {
                "trust".to_string()
            },
            weight: if rule.weight.is_finite() {
                rule.weight.clamp(0.5, 5.0)
            } else {
                1.0
            },
            why: rule.why.chars().take(600).collect(),
            fix: rule.fix.chars().take(600).collect(),
            rule_type: if rule.rule_type == "must_not_contain" {
                "must_not_contain".to_string()
            } else {
                "must_contain".to_string()
            },
            pattern,
            scope: if rule.scope == "site" {
                "site".to_string()
            } else {
                "page".to_string()
            },
        });
    }

    clean.ai_guidance = if raw.ai_guidance.trim().is_empty() {
        DEFAULT_GUIDANCE.to_string()
    } else {
        raw.ai_guidance.chars().take(2000).collect()
    };

    clean.sources = raw
        .sources
        .into_iter()
        .filter(|source| {
            let url = source.url.to_ascii_lowercase();
            url.starts_with("http://") || url.starts_with("https://")
        })
        .take(30)
        .map(|source| SourceRef {
            label: source.label.chars().take(120).collect(),
            url: source.url.chars().take(500).collect(),
            note: source.note.chars().take(300).collect(),
        })
        .collect();

    clean
}

pub fn get_overrides() -> Overrides {
    let Ok(text) = fs::read_to_string(overrides_path()) else {
        return default_overrides();
    };
    match serde_json::from_str::<Overrides>(&text) {
        Ok(raw) => sanitize_overrides(raw),
        Err(_) => default_overrides(),
    }
}

pub fn save_overrides(raw: Overrides) -> Result<Overrides, String> {
    let clean = sanitize_overrides(raw);
    write_json(&overrides_path(), &clean)?;
    Ok(clean)
}
