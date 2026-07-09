use std::collections::HashMap;

use dirs::home_dir;
use serde::{Deserialize, Serialize};

use super::profiles::CrawlProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub output_dir: String,
    pub concurrency: u32,
    pub request_delay: u32,
    pub timeout: u32,
    pub user_agent: String,
    pub default_max_depth: u32,
    pub default_page_limit: u32,
    pub default_download_assets: bool,
    pub default_headless_strategy: String,
    pub default_respect_robots_txt: bool,
    pub default_stay_within_domain: bool,
    pub default_ssrf_protection: bool,
    pub window_width: u32,
    pub window_height: u32,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    /// User-customized keyboard shortcut bindings, keyed by action id (e.g. "new-crawl").
    /// A value of "" means the action is explicitly unbound; a missing key falls back
    /// to the action's built-in default binding.
    #[serde(default)]
    pub shortcut_overrides: HashMap<String, String>,
    /// When set, this export format runs automatically after every crawl completes.
    #[serde(default)]
    pub auto_export_format: Option<crate::export::ExportFormat>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let output_dir = home_dir()
            .map(|h| h.join(".docurip").to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("./output"));
        Self {
            output_dir,
            concurrency: 3,
            request_delay: 750,
            timeout: 30000,
            user_agent: String::from("Docurip/0.6.2 (Documentation Crawler)"),
            default_max_depth: 2,
            default_page_limit: 1000,
            default_download_assets: false,
            default_headless_strategy: String::from("auto"),
            default_respect_robots_txt: true,
            default_stay_within_domain: true,
            default_ssrf_protection: true,
            window_width: 1280,
            window_height: 900,
            notifications_enabled: true,
            theme: default_theme(),
            shortcut_overrides: HashMap::new(),
            auto_export_format: None,
        }
    }
}

fn default_theme() -> String {
    String::from("system")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlConfig {
    pub output_dir: String,
    pub max_depth: u32,
    pub page_limit: u32,
    pub download_assets: bool,
    pub headless_strategy: String,
    pub content_selectors: Vec<String>,
    pub exclude_patterns: Vec<String>,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default)]
    pub path_prefix: String,
    pub respect_robots_txt: bool,
    #[serde(default = "default_true")]
    pub stay_within_domain: bool,
    #[serde(default = "default_true")]
    pub ssrf_protection: bool,
    #[serde(default)]
    pub profile: Option<CrawlProfile>,
}

fn default_true() -> bool {
    true
}
