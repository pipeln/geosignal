//! Content and release checks. Updates are data-only: a manifest describes a
//! help-content pack and an app release page. The application never executes
//! remote code, and every content pack must match a pinned SHA-256 digest.

use crate::models::{self, CustomModel};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

const SCHEMA: u32 = 1;
const MANIFEST_LIMIT: u64 = 512 * 1024;
const CONTENT_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub last_check: String,
    #[serde(default)]
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppRelease {
    #[serde(default)]
    pub latest: String,
    #[serde(default)]
    pub release_url: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContentRelease {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub app: Option<AppRelease>,
    #[serde(default)]
    pub content: Option<ContentRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContentPack {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub topics: Value,
    #[serde(default)]
    pub guides: Value,
    #[serde(default)]
    pub models: Vec<CustomModel>,
}

fn data_dir() -> PathBuf {
    crate::util::data_dir().join("updates")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

fn content_path() -> PathBuf {
    data_dir().join("content.json")
}

fn ensure_dir() -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|error| format!("Could not create updates folder: {error}"))
}

pub fn load_config() -> UpdateConfig {
    let mut config: UpdateConfig = fs::read_to_string(config_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if config.source.trim().is_empty() {
        config.source = std::env::var("GEOSIGNAL_UPDATE_MANIFEST").unwrap_or_default();
    }
    config
}

fn save_config(config: &UpdateConfig) -> Result<(), String> {
    ensure_dir()?;
    let text = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    fs::write(config_path(), text).map_err(|error| format!("Could not save update settings: {error}"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn validate_url(value: &str, label: &str) -> Result<Url, String> {
    let url = Url::parse(value.trim()).map_err(|error| format!("Invalid {label} URL: {error}"))?;
    if url.scheme() != "https" {
        return Err(format!("{label} must use HTTPS."));
    }
    if url.host_str().unwrap_or("").is_empty() {
        return Err(format!("{label} needs a host."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!("{label} must not contain credentials."));
    }
    if crate::util::public_mode() && !crate::util::public_url_allowed(url.as_str()) {
        return Err(format!("{label} must use a public host in this deployment."));
    }
    Ok(url)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn fetch_limited(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let public = crate::util::public_mode();
    let builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .redirects(if public { 0 } else { 10 });
    let builder = if public {
        builder.resolver(crate::util::public_resolver)
    } else {
        builder
    };
    let agent = builder.build();
    let response = agent
        .get(url)
        .call()
        .map_err(|error| format!("Update request failed: {error}"))?;
    if response.status() != 200 {
        return Err(format!("Update request returned HTTP {}.", response.status()));
    }
    let mut bytes = Vec::new();
    response
        .into_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Could not read update response: {error}"))?;
    if bytes.len() as u64 > limit {
        return Err("Update response is larger than the allowed size.".to_string());
    }
    Ok(bytes)
}

fn validate_manifest(raw: Manifest) -> Result<Manifest, String> {
    if raw.schema != SCHEMA {
        return Err(format!(
            "This update manifest uses schema {}, but this app supports schema {}.",
            raw.schema, SCHEMA
        ));
    }
    if let Some(app) = &raw.app {
        if !app.latest.trim().is_empty() {
            let url = validate_url(&app.release_url, "App release page")?;
            if app.release_url != url.to_string() {
                return Err("App release page URL is malformed.".to_string());
            }
        }
    }
    if let Some(content) = &raw.content {
        if content.version.trim().is_empty() || content.url.trim().is_empty() {
            return Err("The content update is missing its version or URL.".to_string());
        }
        validate_url(&content.url, "Content pack")?;
        if !valid_sha256(&content.sha256) {
            return Err("The content update is missing a valid SHA-256 digest.".to_string());
        }
    }
    Ok(raw)
}

fn validate_content(raw: ContentPack) -> Result<ContentPack, String> {
    if raw.schema != SCHEMA {
        return Err(format!(
            "This content pack uses schema {}, but this app supports schema {}.",
            raw.schema, SCHEMA
        ));
    }
    if raw.version.trim().is_empty() {
        return Err("The content pack has no version.".to_string());
    }
    for (name, value) in [("topics", &raw.topics), ("guides", &raw.guides)] {
        if !value.is_null() && !value.is_object() {
            return Err(format!("Content pack {name} must be an object."));
        }
    }
    Ok(raw)
}

pub fn status() -> Value {
    let config = load_config();
    let content = load_content();
    let (version, notes, topics, guides, model_count) = match content {
        Some((_, pack)) => (
            pack.version,
            pack.notes,
            pack.topics.as_object().map(|items| items.len()).unwrap_or(0),
            pack.guides.as_object().map(|items| items.len()).unwrap_or(0),
            pack.models.len(),
        ),
        None => (String::new(), String::new(), 0, 0, 0),
    };
    json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "source": config.source,
        "lastCheck": config.last_check,
        "lastError": config.last_error,
        "content": {
            "installed": !version.is_empty(),
            "version": version,
            "notes": notes,
            "topics": topics,
            "guides": guides,
            "models": model_count,
        }
    })
}

pub fn set_source(source: &str) -> Result<Value, String> {
    let source = source.trim().to_string();
    if source.len() > 500 {
        return Err("Update source URL is too long.".to_string());
    }
    if !source.is_empty() {
        validate_url(&source, "Update source")?;
    }
    let mut config = load_config();
    config.source = source;
    config.last_check = String::new();
    config.last_error = String::new();
    save_config(&config)?;
    Ok(status())
}

pub fn check() -> Result<Value, String> {
    let config = load_config();
    if config.source.trim().is_empty() {
        return Err("Set an HTTPS update manifest URL first.".to_string());
    }
    validate_url(&config.source, "Update source")?;
    let result = (|| {
        let bytes = fetch_limited(&config.source, MANIFEST_LIMIT)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Update manifest is not valid JSON: {error}"))?;
        validate_manifest(manifest)
    })();

    let mut next = config;
    next.last_check = crate::util::now_iso();
    next.last_error = match &result {
        Ok(_) => String::new(),
        Err(error) => error.chars().take(500).collect(),
    };
    save_config(&next)?;
    let manifest = result?;
    Ok(json!({
        "manifest": manifest,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "contentVersion": load_content().map(|(_, pack)| pack.version).unwrap_or_default(),
    }))
}

pub fn apply(content: ContentRelease) -> Result<Value, String> {
    validate_url(&content.url, "Content pack")?;
    if content.version.trim().is_empty() {
        return Err("Content pack has no version.".to_string());
    }
    if !valid_sha256(&content.sha256) {
        return Err("Content pack has no valid SHA-256 digest.".to_string());
    }
    let bytes = fetch_limited(&content.url, CONTENT_LIMIT)?;
    let actual = sha256_bytes(&bytes);
    if actual != content.sha256.to_ascii_lowercase() {
        return Err("Content pack failed its SHA-256 integrity check. Nothing was applied.".to_string());
    }
    let mut pack: ContentPack = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Content pack is not valid JSON: {error}"))?;
    pack = validate_content(pack)?;
    for model in &mut pack.models {
        model.source = "catalog".to_string();
    }
    let model_count = models::upsert_many(pack.models.clone())?;
    ensure_dir()?;
    let text = serde_json::to_string_pretty(&pack).map_err(|error| error.to_string())?;
    fs::write(content_path(), text)
        .map_err(|error| format!("Could not write content pack: {error}"))?;
    Ok(json!({
        "ok": true,
        "version": pack.version,
        "topics": pack.topics.as_object().map(|items| items.len()).unwrap_or(0),
        "guides": pack.guides.as_object().map(|items| items.len()).unwrap_or(0),
        "models": model_count,
    }))
}

pub fn load_content() -> Option<(Value, ContentPack)> {
    let text = fs::read_to_string(content_path()).ok()?;
    let pack: ContentPack = serde_json::from_str(&text).ok()?;
    if pack.version.trim().is_empty() {
        return None;
    }
    Some((serde_json::to_value(&pack).ok()?, pack))
}

pub fn content_json() -> Value {
    load_content()
        .map(|(json, _)| json)
        .unwrap_or_else(|| json!({ "schema": SCHEMA, "version": "", "topics": {}, "guides": {}, "models": [] }))
}

pub fn clear_content() -> Result<(), String> {
    match fs::remove_file(content_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Could not remove content pack: {error}")),
    }
}

pub fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |value: &str| -> Vec<u64> {
        value
            .trim()
            .trim_start_matches('v')
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let latest = parse(latest);
    let current = parse(current);
    for index in 0..latest.len().max(current.len()) {
        let left = latest.get(index).copied().unwrap_or(0);
        let right = current.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_plain_http_update_sources() {
        assert!(validate_url("http://example.com/manifest.json", "Update source").is_err());
        assert!(validate_url("https://example.com/manifest.json", "Update source").is_ok());
    }

    #[test]
    fn compares_versions_without_a_semver_dependency() {
        assert!(version_is_newer("1.2.0", "1.1.9"));
        assert!(!version_is_newer("v1.0.0", "1.0.0"));
        assert!(!version_is_newer("0.9.9", "1.0.0"));
    }

    #[test]
    fn rejects_content_with_the_wrong_shape() {
        let mut pack = ContentPack {
            schema: 1,
            version: "2026.11.1".to_string(),
            ..Default::default()
        };
        assert!(validate_content(pack.clone()).is_ok());
        pack.topics = json!(["not", "an", "object"]);
        assert!(validate_content(pack).is_err());
    }
}
