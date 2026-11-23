//! Embedded inference: llama.cpp is compiled into this binary, GGUF weights are
//! downloaded once into `data/models/` and then loaded from disk. No daemon,
//! no Ollama, no network calls after the weights arrive.

use crate::models;
use crate::types::ModelPreset;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_TOKENS: usize = 900;
const CTX_TOKENS: u32 = 8192;
const BATCH_TOKENS: usize = 512;

/// GBNF grammar for the audit answer. Constrained decoding is what makes a
/// 1.5B model usable here: the shape is guaranteed even when the prose is not.

pub struct GenResult {
    pub ok: bool,
    pub content: String,
    pub error: Option<String>,
    pub latency_ms: u64,
    pub tokens: Option<u64>,
}

impl GenResult {
    fn failed(message: String, latency_ms: u64) -> Self {
        GenResult {
            ok: false,
            content: String::new(),
            error: Some(message),
            latency_ms,
            tokens: None,
        }
    }
}

fn backend() -> Result<&'static LlamaBackend, String> {
    static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
    BACKEND
        .get_or_init(|| LlamaBackend::init().map_err(|error| format!("Could not start llama.cpp: {error}")))
        .as_ref()
        .map_err(|error| error.clone())
}

pub fn models_dir() -> PathBuf {
    crate::util::data_dir().join("models")
}

#[derive(Clone)]
struct ModelTarget {
    id: String,
    tag: String,
    label: String,
    size: String,
    url: String,
    file: String,
    bytes: u64,
    sha256: String,
    custom: bool,
}

fn builtin_target(preset: &ModelPreset) -> ModelTarget {
    ModelTarget {
        id: preset.id.to_string(),
        tag: preset.tag.to_string(),
        label: preset.label.to_string(),
        size: preset.size.to_string(),
        url: format!(
            "https://huggingface.co/{}/resolve/main/{}?download=true",
            preset.gguf_repo, preset.gguf_file
        ),
        file: preset.gguf_file.to_string(),
        bytes: preset.gguf_bytes,
        sha256: preset.gguf_sha256.to_string(),
        custom: false,
    }
}

fn custom_target(custom: &models::CustomModel) -> ModelTarget {
    ModelTarget {
        id: custom.id.clone(),
        tag: custom.tag.clone(),
        label: custom.label.clone(),
        size: custom.size.clone(),
        url: custom.url.clone(),
        file: custom.file.clone(),
        bytes: custom.bytes,
        sha256: custom.sha256.clone(),
        custom: true,
    }
}

fn target_for_id(id: &str) -> Option<ModelTarget> {
    crate::ai::PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .map(builtin_target)
        .or_else(|| models::find_by_id(id).map(|model| custom_target(&model)))
}

fn target_for_tag(tag: &str) -> Option<ModelTarget> {
    crate::ai::PRESETS
        .iter()
        .find(|preset| preset.tag == tag)
        .map(builtin_target)
        .or_else(|| models::find_by_tag(tag).map(|model| custom_target(&model)))
}

fn target_path(target: &ModelTarget) -> PathBuf {
    models_dir().join(&target.file)
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifiedFile {
    url: String,
    sha256: String,
}

fn verification_path(target: &ModelTarget) -> PathBuf {
    target_path(target).with_file_name(format!(".{}.verified.json", target.file))
}

fn write_verification(path: &Path, url: &str, sha256: &str) -> Result<(), String> {
    let record = VerifiedFile {
        url: url.to_string(),
        sha256: sha256.to_ascii_lowercase(),
    };
    let text = serde_json::to_string(&record).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| format!("Could not record model verification: {error}"))
}

fn partial_path(target: &ModelTarget) -> PathBuf {
    models_dir().join(format!("{}.part", target.file))
}

fn partial_metadata_path(target: &ModelTarget) -> PathBuf {
    models_dir().join(format!(".{}.part.json", target.file))
}

fn partial_matches(metadata: &Path, url: &str, sha256: &str) -> bool {
    let Ok(text) = fs::read_to_string(metadata) else {
        return false;
    };
    let Ok(record) = serde_json::from_str::<VerifiedFile>(&text) else {
        return false;
    };
    record.url == url && record.sha256.eq_ignore_ascii_case(sha256)
}

fn discard_stale_partial(target: &ModelTarget, part: &Path, metadata: &Path) -> u64 {
    let existing = part.metadata().map(|meta| meta.len()).unwrap_or(0);
    if existing == 0 {
        let _ = fs::remove_file(metadata);
        return 0;
    }
    if target.custom && !partial_matches(metadata, &target.url, &target.sha256) {
        let _ = fs::remove_file(part);
        let _ = fs::remove_file(metadata);
        return 0;
    }
    existing
}

fn custom_file_ready(path: &Path, verification: &Path, url: &str, sha256: &str) -> bool {
    let installed = path
        .metadata()
        .map(|meta| meta.len() > 1_000_000)
        .unwrap_or(false);
    if !installed {
        return false;
    }
    let Ok(text) = fs::read_to_string(verification) else {
        return false;
    };
    let Ok(record) = serde_json::from_str::<VerifiedFile>(&text) else {
        return false;
    };
    record.url == url && record.sha256.eq_ignore_ascii_case(sha256)
}

fn target_installed(target: &ModelTarget) -> bool {
    let path = target_path(target);
    if target.custom {
        return custom_file_ready(&path, &verification_path(target), &target.url, &target.sha256);
    }
    path.metadata()
        .map(|meta| meta.len() > 1_000_000)
        .unwrap_or(false)
}

pub fn model_path(preset: &ModelPreset) -> PathBuf {
    models_dir().join(preset.gguf_file)
}

pub fn is_installed(preset: &ModelPreset) -> bool {
    target_installed(&builtin_target(preset))
}

#[derive(Clone)]
struct Download {
    received: u64,
    total: u64,
    error: Option<String>,
    done: bool,
}

fn downloads() -> &'static Mutex<HashMap<String, Download>> {
    static DOWNLOADS: OnceLock<Mutex<HashMap<String, Download>>> = OnceLock::new();
    DOWNLOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn start_download(id: &str) -> Result<(), String> {
    let target = target_for_id(id).ok_or_else(|| format!("Unknown model: {id}"))?;
    if target_installed(&target) {
        return Ok(());
    }
    {
        let mut map = downloads().lock().unwrap();
        if let Some(entry) = map.get(id) {
            if entry.error.is_none() && !entry.done {
                return Ok(());
            }
        }
        map.insert(
            id.to_string(),
            Download {
                received: 0,
                total: target.bytes,
                error: None,
                done: false,
            },
        );
    }

    std::thread::spawn(move || {
        let result = download(&target);
        let mut map = downloads().lock().unwrap();
        let entry = map.entry(target.id.clone()).or_insert(Download {
            received: 0,
            total: target.bytes,
            error: None,
            done: false,
        });
        match result {
            Ok(bytes) => {
                entry.received = bytes;
                entry.done = true;
                entry.error = None;
            }
            Err(error) => {
                entry.error = Some(error);
            }
        }
    });
    Ok(())
}

fn download(target: &ModelTarget) -> Result<u64, String> {
    let dir = models_dir();
    fs::create_dir_all(&dir).map_err(|error| format!("Could not create {}: {error}", dir.display()))?;
    let final_path = target_path(target);
    let part_path = partial_path(target);
    let part_metadata = partial_metadata_path(target);
    let existing = discard_stale_partial(target, &part_path, &part_metadata);
    if target.custom {
        write_verification(&part_metadata, &target.url, &target.sha256)?;
    }

    let public = crate::util::public_mode();
    let builder = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120));
    let builder = if public {
        builder.resolver(crate::util::public_resolver)
    } else {
        builder
    };
    let agent = builder.build();
    let mut request = agent.get(&target.url);
    if existing > 0 {
        request = request.set("range", &format!("bytes={existing}-"));
    }
    let response = request
        .call()
        .map_err(|error| format!("Download failed: {error}"))?;

    let resume = response.status() == 206 && existing > 0;
    let start_at = if resume { existing } else { 0 };
    if response.status() != 200 && response.status() != 206 {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }
    {
        let mut map = downloads().lock().unwrap();
        if let Some(entry) = map.get_mut(&target.id) {
            entry.received = start_at;
            entry.total = response
                .header("content-length")
                .and_then(|value| value.parse::<u64>().ok())
                .map(|length| length + start_at)
                .unwrap_or(target.bytes);
        }
    }

    let mut file = if resume {
        OpenOptions::new()
            .append(true)
            .open(&part_path)
            .map_err(|error| format!("Could not resume download: {error}"))?
    } else {
        fs::File::create(&part_path).map_err(|error| format!("Could not write model file: {error}"))?
    };

    let mut reader = response.into_reader();
    let mut buffer = vec![0u8; 256 * 1024];
    let mut received = start_at;
    let mut last_report = Instant::now();
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Download interrupted: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("Could not write model file: {error}"))?;
        received += read as u64;
        if last_report.elapsed() > Duration::from_millis(250) {
            last_report = Instant::now();
            if let Some(entry) = downloads().lock().unwrap().get_mut(&target.id) {
                entry.received = received;
            }
        }
    }
    file.flush().ok();
    drop(file);

    let size = part_path
        .metadata()
        .map(|meta| meta.len())
        .map_err(|error| format!("Downloaded file is missing: {error}"))?;
    if target.bytes > 0 && size + 1024 * 1024 < target.bytes {
        return Err(format!(
            "Download stopped early ({:.0} MB of {:.0} MB). Try again to resume.",
            size as f64 / 1e6,
            target.bytes as f64 / 1e6
        ));
    }
    let digest = sha256_file(&part_path)?;
    if digest != target.sha256 {
        let _ = fs::remove_file(&part_path);
        let _ = fs::remove_file(&part_metadata);
        return Err("The model download failed its integrity check and was removed. Try again."
            .to_string());
    }
    fs::rename(&part_path, &final_path)
        .map_err(|error| format!("Could not finish model file: {error}"))?;
    if target.custom {
        let _ = fs::remove_file(&part_metadata);
        if let Err(error) = write_verification(&verification_path(target), &target.url, &target.sha256) {
            let _ = fs::remove_file(&final_path);
            return Err(error);
        }
    }
    Ok(size)
}

fn sha256_file(path: &PathBuf) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Could not read downloaded model: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not verify downloaded model: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn target_status_json(
    target: &ModelTarget,
    custom: bool,
    family: &str,
    notes: &str,
    license: &str,
    repo: &str,
    map: &HashMap<String, Download>,
) -> Value {
    let path = target_path(target);
    let installed = target_installed(target);
    let local_bytes = path.metadata().map(|meta| meta.len()).unwrap_or(0);
    let progress = map.get(&target.id);
    json!({
        "id": &target.id,
        "tag": &target.tag,
        "label": &target.label,
        "family": family,
        "notes": notes,
        "size": &target.size,
        "license": license,
        "repo": repo,
        "file": &target.file,
        "expectedBytes": target.bytes,
        "bytes": if installed { local_bytes } else { progress.map(|p| p.received).unwrap_or(0) },
        "installed": installed,
        "downloading": progress.map(|p| p.error.is_none() && !p.done).unwrap_or(false),
        "done": progress.map(|p| p.done).unwrap_or(false),
        "error": progress.and_then(|p| p.error.clone()),
        "path": if installed { json!(path.display().to_string()) } else { Value::Null },
        "custom": custom,
    })
}

pub fn status_json() -> Value {
    let map = downloads().lock().unwrap().clone();
    let mut models: Vec<Value> = crate::ai::PRESETS
        .iter()
        .map(|preset| {
            target_status_json(
                &builtin_target(preset),
                false,
                preset.family,
                preset.notes,
                preset.license,
                preset.gguf_repo,
                &map,
            )
        })
        .collect();
    for custom in models::list() {
        models.push(target_status_json(
            &custom_target(&custom),
            true,
            &custom.family,
            &custom.notes,
            &custom.license,
            "",
            &map,
        ));
    }
    json!({
        "runtime": "llama.cpp (built in)",
        "modelsDir": models_dir().display().to_string(),
        "models": models,
    })
}

pub fn has_embedded(tag: &str) -> bool {
    target_for_tag(tag)
        .map(|target| target_installed(&target))
        .unwrap_or(false)
}

pub fn generate(tag: &str, system: &str, user: &str) -> GenResult {
    let started = Instant::now();
    let Some(target) = target_for_tag(tag) else {
        return GenResult::failed(
            format!("No model is mapped to \"{tag}\". Pick a preset or use an Ollama tag."),
            started.elapsed().as_millis() as u64,
        );
    };
    let path = target_path(&target);
    if !target_installed(&target) {
        return GenResult::failed(
            format!(
                "{} is not downloaded yet. Open the Model lab and download it once ({}).",
                target.label, target.size
            ),
            started.elapsed().as_millis() as u64,
        );
    }

    match infer(&path, system, user) {
        Ok((text, tokens)) => GenResult {
            ok: true,
            content: text,
            error: None,
            latency_ms: started.elapsed().as_millis() as u64,
            tokens: Some(tokens as u64),
        },
        Err(error) => GenResult::failed(error, started.elapsed().as_millis() as u64),
    }
}

fn fallback_prompt(system: &str, user: &str) -> String {
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
    )
}

fn infer(path: &PathBuf, system: &str, user: &str) -> Result<(String, usize), String> {
    let backend = backend()?;
    let model_params = LlamaModelParams::default().with_n_gpu_layers(1000);
    let model = LlamaModel::load_from_file(backend, path, &model_params)
        .map_err(|error| format!("Could not load {}: {error}", path.display()))?;

    let n_ctx = NonZeroU32::new(CTX_TOKENS).unwrap();
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_batch(BATCH_TOKENS as u32);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|error| format!("Could not create inference context: {error}"))?;

    let prompt = match model.chat_template(None) {
        Ok(template) => {
            let messages = [
                LlamaChatMessage::new("system".to_string(), system.to_string())
                    .map_err(|error| format!("Invalid system prompt: {error}"))?,
                LlamaChatMessage::new("user".to_string(), user.to_string())
                    .map_err(|error| format!("Invalid user prompt: {error}"))?,
            ];
            model
                .apply_chat_template(&template, &messages, true)
                .unwrap_or_else(|_| fallback_prompt(system, user))
        }
        Err(_) => fallback_prompt(system, user),
    };

    let tokens = model.vocab().tokenize(prompt.as_bytes(), true, true);
    let prompt_tokens = tokens.len();
    if prompt_tokens + MAX_TOKENS + 8 > CTX_TOKENS as usize {
        return Err(format!(
            "The prompt is {prompt_tokens} tokens, too long for the {CTX_TOKENS}-token context."
        ));
    }

    let mut batch = LlamaBatch::new(BATCH_TOKENS, 1);
    for (chunk_index, chunk) in tokens.chunks(BATCH_TOKENS).enumerate() {
        batch.clear();
        let base = chunk_index * BATCH_TOKENS;
        for (offset, token) in chunk.iter().enumerate() {
            let position = (base + offset) as i32;
            let is_last = base + offset + 1 == prompt_tokens;
            batch
                .add(*token, position, &[0], is_last)
                .map_err(|error| format!("Could not queue the prompt: {error}"))?;
        }
        ctx.decode(&mut batch)
            .map_err(|error| format!("Prompt evaluation failed: {error}"))?;
    }

    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::temp(0.2),
        LlamaSampler::dist(42),
    ]);
    let mut next_position = prompt_tokens as i32;
    let mut bytes: Vec<u8> = Vec::new();
    let mut generated = 0usize;

    while generated < MAX_TOKENS {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.vocab().is_eog(token) {
            break;
        }
        let piece = model.vocab().token_to_piece(token, true, None);
        bytes.extend_from_slice(&piece);
        generated += 1;

        batch.clear();
        batch
            .add(token, next_position, &[0], true)
            .map_err(|error| format!("Could not queue a token: {error}"))?;
        next_position += 1;
        ctx.decode(&mut batch)
            .map_err(|error| format!("Token evaluation failed: {error}"))?;
    }

    Ok((String::from_utf8_lossy(&bytes).to_string(), generated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_has_a_gguf_target() {
        for preset in crate::ai::PRESETS {
            assert!(preset.gguf_repo.contains('/'), "{} repo", preset.id);
            assert!(preset.gguf_file.ends_with(".gguf"), "{} file", preset.id);
            assert!(preset.gguf_bytes > 100_000_000, "{} size", preset.id);
            assert_eq!(preset.gguf_sha256.len(), 64, "{} sha256", preset.id);
        }
    }

    #[test]
    #[ignore]
    fn smoke_generates() {
        let preset = &crate::ai::PRESETS[0];
        if !is_installed(preset) {
            println!("{} is not downloaded; skipping", preset.label);
            return;
        }
        let (text, tokens) = infer(
            &model_path(preset),
            "You answer with compact JSON only.",
            "Return exactly {\"ok\":true,\"note\":\"hello\"} and nothing else.",
        )
        .expect("inference failed");
        println!("GENERATED {tokens} TOKENS:\n{text}");
        assert!(!text.is_empty());
    }

    #[test]
    fn verifies_file_hashes() {
        let path = std::env::temp_dir().join(format!("geosignal-sha-{}", std::process::id()));
        fs::write(&path, b"geosignal").unwrap();
        let digest = sha256_file(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert_eq!(
            digest,
            "af8e7e8d3e6b55a30b70c1d15d1a1051e13dff7ff04744bd86a97b7a3902477f"
        );
    }

    fn verified_fixture(name: &str) -> (PathBuf, PathBuf, String) {
        let dir = std::env::temp_dir().join(format!(
            "geosignal-{name}-{}-{}",
            std::process::id(),
            crate::util::new_id("t")
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("model.gguf");
        fs::write(&file, vec![0u8; 1_000_001]).unwrap();
        let verification = dir.join(".model.gguf.verified.json");
        let sha = "a".repeat(64);
        write_verification(&verification, "https://example.com/model.gguf", &sha).unwrap();
        (dir, verification, sha)
    }

    #[test]
    fn custom_url_change_requires_a_fresh_download() {
        let (dir, verification, sha) = verified_fixture("url-change");
        let file = dir.join("model.gguf");
        assert!(custom_file_ready(
            &file,
            &verification,
            "https://example.com/model.gguf",
            &sha
        ));
        assert!(!custom_file_ready(
            &file,
            &verification,
            "https://cdn.example.com/model.gguf",
            &sha
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn custom_sha_change_requires_a_fresh_download() {
        let (dir, verification, _) = verified_fixture("sha-change");
        let file = dir.join("model.gguf");
        assert!(!custom_file_ready(
            &file,
            &verification,
            "https://example.com/model.gguf",
            &"b".repeat(64),
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    fn custom_target_for(url: &str, sha256: &str) -> ModelTarget {
        ModelTarget {
            id: "custom_test".to_string(),
            tag: "custom:custom_test".to_string(),
            label: "Test model".to_string(),
            size: "1.0 GB".to_string(),
            url: url.to_string(),
            file: "model.gguf".to_string(),
            bytes: 1_000_001,
            sha256: sha256.to_string(),
            custom: true,
        }
    }

    #[test]
    fn changed_custom_target_discards_partial_download() {
        let dir = std::env::temp_dir().join(format!(
            "geosignal-partial-{}-{}",
            std::process::id(),
            crate::util::new_id("t")
        ));
        fs::create_dir_all(&dir).unwrap();
        let part = dir.join("model.gguf.part");
        let metadata = dir.join(".model.gguf.part.json");
        let old_url = "https://example.com/model.gguf";
        let old_sha = "a".repeat(64);
        let new_url = "https://cdn.example.com/model.gguf";
        let new_sha = "b".repeat(64);

        fs::write(&part, b"stale-part").unwrap();
        write_verification(&metadata, old_url, &old_sha).unwrap();
        assert_eq!(
            discard_stale_partial(&custom_target_for(old_url, &old_sha), &part, &metadata),
            10
        );
        assert!(part.exists());

        assert_eq!(
            discard_stale_partial(&custom_target_for(new_url, &old_sha), &part, &metadata),
            0
        );
        assert!(!part.exists());
        assert!(!metadata.exists());

        fs::write(&part, b"stale-part").unwrap();
        write_verification(&metadata, old_url, &old_sha).unwrap();
        assert_eq!(
            discard_stale_partial(&custom_target_for(old_url, &new_sha), &part, &metadata),
            0
        );
        assert!(!part.exists());
        assert!(!metadata.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fallback_prompt_has_chatml_markers() {
        let prompt = fallback_prompt("sys", "hello");
        assert!(prompt.contains("<|im_start|>system\nsys<|im_end|>"));
        assert!(prompt.ends_with("<|im_start|>assistant\n"));
    }
}
