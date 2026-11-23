//! User-supplied GGUF models. Built-in presets stay in `ai::PRESETS`; entries
//! here are downloaded from an explicit HTTPS URL and verified with a hash.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use url::Url;

const MAX_MODELS: usize = 100;
const MAX_BYTES: u64 = 20_000_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomModel {
    #[serde(default)]
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub tag: String,
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub license: String,
    pub url: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub source: String,
}

pub fn models_dir() -> PathBuf {
    crate::util::data_dir().join("models")
}

fn catalog_path() -> PathBuf {
    crate::util::data_dir().join("model-catalog.json")
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let slug = slug.trim_matches('_').chars().take(36).collect::<String>();
    if slug.is_empty() {
        format!("custom_{}", crate::util::new_id("m").chars().take(12).collect::<String>())
    } else {
        format!("custom_{slug}")
    }
}

fn file_from_url(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap_or("model.gguf")
        .trim()
        .to_string()
}

fn safe_file(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value.ends_with(".gguf")
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn text(value: &str, max: usize, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.chars().take(max).collect()
    }
}

/// Normalize untrusted UI or catalog input into a safe registry entry.
pub fn normalize(raw: CustomModel) -> Result<CustomModel, String> {
    let label = text(&raw.label, 80, "");
    if label.is_empty() {
        return Err("Give the custom model a name.".to_string());
    }

    let url = Url::parse(raw.url.trim())
        .map_err(|error| format!("Invalid model URL: {error}"))?;
    if url.scheme() != "https" {
        return Err("Custom model URLs must use HTTPS.".to_string());
    }
    if url.host_str().unwrap_or("").is_empty() {
        return Err("The model URL needs a host.".to_string());
    }
    if crate::util::public_mode() && !crate::util::public_url_allowed(url.as_str()) {
        return Err("Public deployments only accept public model URLs.".to_string());
    }

    let mut id = raw.id.trim().to_ascii_lowercase();
    if id.is_empty() {
        id = slug(&label);
    } else if !id.starts_with("custom_") {
        id = format!("custom_{id}");
    }
    if !safe_id(&id) {
        return Err("Model id can only use letters, numbers, underscores and hyphens.".to_string());
    }

    let mut file = raw.file.trim().to_string();
    if file.is_empty() {
        file = file_from_url(&url);
    }
    if !safe_file(&file) {
        return Err("Model file must be a plain .gguf filename.".to_string());
    }
    if crate::ai::PRESETS
        .iter()
        .any(|preset| preset.gguf_file == file)
    {
        return Err("That filename belongs to a built-in model. Use a different .gguf filename.".to_string());
    }

    let sha256 = raw.sha256.trim().to_ascii_lowercase();
    if !valid_sha256(&sha256) {
        return Err("Paste the model file's 64-character SHA-256 digest.".to_string());
    }
    if raw.bytes > MAX_BYTES {
        return Err("That model is larger than the 20 GB safety limit.".to_string());
    }

    let size = if !raw.size.trim().is_empty() {
        raw.size.trim().chars().take(32).collect::<String>()
    } else if raw.bytes > 0 {
        format!("{:.1} GB", raw.bytes as f64 / 1e9)
    } else {
        "Unknown size".to_string()
    };

    Ok(CustomModel {
        id: id.clone(),
        label,
        tag: format!("custom:{id}"),
        size,
        family: text(&raw.family, 80, "Custom GGUF"),
        notes: text(&raw.notes, 500, "User-supplied GGUF model. Verify its license before use."),
        license: text(&raw.license, 120, "See upstream model license"),
        url: url.to_string(),
        file,
        bytes: raw.bytes,
        sha256,
        source: if raw.source == "catalog" {
            "catalog".to_string()
        } else {
            "manual".to_string()
        },
    })
}

pub fn list() -> Vec<CustomModel> {
    let Ok(text) = fs::read_to_string(catalog_path()) else {
        return Vec::new();
    };
    let raw: Vec<CustomModel> = serde_json::from_str(&text).unwrap_or_default();
    let mut models: Vec<CustomModel> = Vec::new();
    for item in raw {
        if let Ok(clean) = normalize(item) {
            if !models.iter().any(|model| model.id == clean.id) {
                models.push(clean);
            }
        }
    }
    models.truncate(MAX_MODELS);
    models
}

fn save(models: &[CustomModel]) -> Result<(), String> {
    let dir = crate::util::data_dir();
    fs::create_dir_all(&dir).map_err(|error| format!("Could not create data folder: {error}"))?;
    let text = serde_json::to_string_pretty(models).map_err(|error| error.to_string())?;
    fs::write(catalog_path(), text).map_err(|error| format!("Could not save custom models: {error}"))
}

pub fn find_by_id(id: &str) -> Option<CustomModel> {
    list().into_iter().find(|model| model.id == id)
}

pub fn find_by_tag(tag: &str) -> Option<CustomModel> {
    list().into_iter().find(|model| model.tag == tag)
}

pub fn file_path(model: &CustomModel) -> PathBuf {
    models_dir().join(&model.file)
}

pub fn add(raw: CustomModel) -> Result<CustomModel, String> {
    let clean = normalize(raw)?;
    let mut models = list();
    if let Some(existing) = models.iter().find(|model| model.id == clean.id) {
        if existing.file != clean.file {
            return Err("This model id already uses a different GGUF file.".to_string());
        }
    }
    if models
        .iter()
        .any(|model| model.file == clean.file && model.id != clean.id)
    {
        return Err("Another custom model already uses that GGUF filename.".to_string());
    }
    let existing_file = file_path(&clean);
    if existing_file.exists() && !models.iter().any(|model| model.file == clean.file) {
        return Err("A file with that name is already in the models folder. Choose another filename.".to_string());
    }

    models.retain(|model| model.id != clean.id);
    models.insert(0, clean.clone());
    models.truncate(MAX_MODELS);
    save(&models)?;
    Ok(clean)
}

pub fn upsert_many(raw: Vec<CustomModel>) -> Result<usize, String> {
    let mut incoming = Vec::new();
    for item in raw.into_iter().take(MAX_MODELS) {
        incoming.push(normalize(item)?);
    }
    let mut models = list();
    for clean in &incoming {
        if models
            .iter()
            .any(|model| model.file == clean.file && model.id != clean.id)
        {
            return Err(format!(
                "The update's model {} uses a filename already used by another model.",
                clean.label
            ));
        }
        if file_path(clean).exists() && !models.iter().any(|model| model.file == clean.file) {
            return Err(format!(
                "The update's model {} would overwrite an existing model file.",
                clean.label
            ));
        }
    }
    for clean in incoming.iter() {
        models.retain(|model| model.id != clean.id);
        models.insert(0, clean.clone());
    }
    models.truncate(MAX_MODELS);
    save(&models)?;
    Ok(incoming.len())
}

pub fn remove(id: &str) -> Result<bool, String> {
    if !safe_id(id) {
        return Ok(false);
    }
    let mut models = list();
    let Some(found) = models.iter().find(|model| model.id == id).cloned() else {
        return Ok(false);
    };
    models.retain(|model| model.id != id);
    save(&models)?;
    let path = file_path(&found);
    if path.starts_with(models_dir()) {
        let _ = fs::remove_file(path);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CustomModel {
        CustomModel {
            label: "My Small Model".to_string(),
            url: "https://huggingface.co/example/model.gguf".to_string(),
            file: "model.gguf".to_string(),
            sha256: "a".repeat(64),
            bytes: 1_000_000_000,
            ..Default::default()
        }
    }

    #[test]
    fn accepts_https_gguf_models() {
        let model = normalize(sample()).unwrap();
        assert_eq!(model.id, "custom_my_small_model");
        assert_eq!(model.tag, "custom:custom_my_small_model");
        assert_eq!(model.file, "model.gguf");
    }

    #[test]
    fn rejects_unsafe_or_unverified_models() {
        let mut insecure = sample();
        insecure.url = "http://example.com/model.gguf".to_string();
        assert!(normalize(insecure).is_err());

        let mut bad_hash = sample();
        bad_hash.sha256 = "not-a-hash".to_string();
        assert!(normalize(bad_hash).is_err());

        let mut traversal = sample();
        traversal.file = "../model.gguf".to_string();
        assert!(normalize(traversal).is_err());

        let mut builtin = sample();
        builtin.file = crate::ai::PRESETS[0].gguf_file.to_string();
        assert!(normalize(builtin).is_err());
    }
}
