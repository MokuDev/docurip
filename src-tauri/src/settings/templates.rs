use serde::{Deserialize, Serialize};

use super::config::CrawlConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrawlTemplate {
    pub id: String,
    pub name: String,
    pub url: String,
    pub config: CrawlConfig,
    pub created_at: String,
}
