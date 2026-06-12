use serde::{Deserialize, Serialize};

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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            output_dir: String::from("./output"),
            concurrency: 3,
            request_delay: 1000,
            timeout: 30000,
            user_agent: String::from("Docurip/0.1.0 (Documentation Crawler)"),
            default_max_depth: 2,
            default_page_limit: 50,
            default_download_assets: false,
            default_headless_strategy: String::from("auto"),
            default_respect_robots_txt: true,
        }
    }
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
    pub respect_robots_txt: bool,
}
