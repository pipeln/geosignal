use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Nap {
    pub name: Option<String>,
    pub types: Vec<String>,
    pub street: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub opening_hours: Vec<String>,
    pub same_as: Vec<String>,
    pub area_served: Vec<String>,
    pub rating_value: Option<f64>,
    pub review_count: Option<f64>,
    pub price_range: Option<String>,
    pub has_map: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRef {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Hreflang {
    pub lang: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImageStats {
    pub total: usize,
    pub missing_alt: usize,
    pub no_dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkCounts {
    pub internal: usize,
    pub external: usize,
    pub tel: usize,
    pub nofollow: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PageAudit {
    pub url: String,
    pub path: String,
    pub depth: usize,
    pub status: u16,
    pub ok: bool,
    pub ms: u64,
    pub bytes: usize,
    pub content_type: String,
    pub title: String,
    pub title_length: usize,
    pub meta_description: String,
    pub canonical: String,
    pub noindex: bool,
    pub lang: String,
    pub viewport: bool,
    pub h1: Vec<String>,
    pub h2: Vec<String>,
    pub word_count: usize,
    pub images: ImageStats,
    pub links: LinkCounts,
    pub anchors: Vec<AnchorRef>,
    pub hreflang: Vec<Hreflang>,
    pub json_ld_types: Vec<String>,
    pub json_ld_blocks: usize,
    pub json_ld_parsed: usize,
    pub has_business_schema: bool,
    pub has_breadcrumb: bool,
    pub has_breadcrumb_visible: bool,
    pub has_faq: bool,
    pub has_rating: bool,
    pub has_map_embed: bool,
    pub gsc_verification: bool,
    pub nap: Nap,
    pub text_sample: String,
    pub blocking_scripts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalLink {
    pub text: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SiteSignals {
    pub host: String,
    pub https: bool,
    pub pages_crawled: usize,
    pub max_depth: usize,
    pub robots_found: bool,
    pub robots_disallow: Vec<String>,
    pub sitemaps: Vec<String>,
    pub sitemap_found: bool,
    pub sitemap_urls: usize,
    pub sitemap_is_index: bool,
    pub gsc_verification: bool,
    pub business: Nap,
    pub city: String,
    pub keyword: String,
    pub city_detected: bool,
    pub keyword_detected: bool,
    pub city_mentions: usize,
    pub keyword_mentions: usize,
    pub avg_ms: u64,
    pub html_bytes: usize,
    pub blocking_scripts: usize,
    pub images_no_dims: usize,
    pub internal_link_total: usize,
    pub location_links: Vec<LocalLink>,
    pub about_link: Option<String>,
    pub contact_link: Option<String>,
    pub map_embed: bool,
    pub review_mentions: usize,
    pub indexable_pages: usize,
    pub unique_titles: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub keyword: String,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub id: String,
    pub url: String,
    pub host: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub target: Target,
    pub pages: Vec<PageAudit>,
    pub signals: SiteSignals,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckSource {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub id: String,
    pub title: String,
    pub category: String,
    pub weight: f64,
    pub status: String,
    pub evidence: String,
    pub why: String,
    pub fix: String,
    pub sources: Vec<CheckSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(default)]
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    pub key: String,
    pub label: String,
    pub score: u32,
    pub weight: f64,
    pub passed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub score: u32,
    pub band: String,
    pub summary: String,
    pub categories: Vec<CategoryScore>,
    pub checks: Vec<CheckResult>,
    pub working: Vec<CheckResult>,
    pub fixes: Vec<CheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomRule {
    pub id: String,
    pub title: String,
    pub category: String,
    pub weight: f64,
    pub why: String,
    pub fix: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub pattern: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Overrides {
    pub disabled_rules: Vec<String>,
    pub weights: HashMap<String, f64>,
    pub custom_rules: Vec<CustomRule>,
    pub ai_guidance: String,
    pub sources: Vec<SourceRef>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
    pub size: &'static str,
    pub family: &'static str,
    pub notes: &'static str,
    pub gguf_repo: &'static str,
    pub gguf_file: &'static str,
    pub gguf_bytes: u64,
    pub gguf_sha256: &'static str,
    pub license: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    pub key: String,
    pub title: String,
    pub category: String,
    pub severity: String,
    pub evidence: String,
    #[serde(rename = "why_it_works", alias = "whyItWorks")]
    pub why_it_works: String,
    pub fix: String,
    pub impact: u8,
    pub effort: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelRun {
    pub model: String,
    pub label: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub latency_ms: u64,
    pub valid_json: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u64>,
    pub suggestions: Vec<AiSuggestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_notes: Option<String>,
    pub schema_compliance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CuratedSuggestion {
    pub key: String,
    pub title: String,
    pub category: String,
    pub severity: String,
    pub evidence: String,
    pub why: String,
    pub fix: String,
    pub impact: u8,
    pub effort: String,
    pub votes: Vec<String>,
    pub vote_count: usize,
    pub agreement: u32,
    pub confidence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelMetric {
    pub model: String,
    pub label: String,
    pub latency_ms: u64,
    pub valid_json: bool,
    pub suggestions: usize,
    pub schema_compliance: u32,
    pub consensus_match: u32,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardRow {
    pub model: String,
    pub label: String,
    pub runs: usize,
    pub valid_json_rate: u32,
    pub avg_latency_ms: u64,
    pub avg_schema: u32,
    pub avg_consensus: u32,
    pub score: u32,
    pub last_run: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompareResult {
    pub id: String,
    pub scan_id: String,
    pub host: String,
    pub at: String,
    pub mode: String,
    pub runs: Vec<ModelRun>,
    pub curated: Vec<CuratedSuggestion>,
    pub unique: Vec<CuratedSuggestion>,
    pub metrics: Vec<ModelMetric>,
    #[serde(default)]
    pub leaderboard: Vec<LeaderboardRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanListEntry {
    pub id: String,
    pub url: String,
    pub host: String,
    pub finished_at: String,
    pub pages: usize,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub url: String,
    #[serde(default)]
    pub max_pages: Option<usize>,
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompareRequest {
    pub scan_id: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub mode: String,
}

