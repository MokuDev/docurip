use serde::{Deserialize, Serialize};
use crate::settings::config::CrawlConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageMeta {
    pub url: String,
    pub title: String,
    pub status: u16,
    pub links_count: usize,
}

/// Full page data used only during crawl processing; not stored in CrawlJob.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub links: Vec<String>,
    pub assets: Vec<String>,
    pub status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlProgress {
    pub pages_crawled: usize,
    pub page_limit: usize,
    pub current_url: String,
    pub depth: u32,
    pub max_depth: u32,
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlJob {
    pub id: String,
    pub url: String,
    pub status: JobStatus,
    pub config: CrawlConfig,
    pub results: Vec<PageMeta>,
    pub progress: CrawlProgress,
    pub error: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    /// Set when this job is a child of a batch. Present for History
    /// grouping and does not affect crawl behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}
