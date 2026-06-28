pub mod pdf;
pub mod epub;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub markdown_path: String,
    pub images_dir: Option<String>,
    pub page_count: usize,
    pub image_count: usize,
    pub title: String,
}

pub fn ensure_output_dir(output_dir: &Path) -> anyhow::Result<PathBuf> {
    let images_dir = output_dir.join("images");
    std::fs::create_dir_all(&images_dir)?;
    Ok(images_dir)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}
