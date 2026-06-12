use std::path::PathBuf;
use tokio::fs;

#[derive(Clone)]
pub struct FsWriter {
    base_dir: PathBuf,
}

impl FsWriter {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub async fn write_page(&self, url: &str, markdown: &str) -> anyhow::Result<PathBuf> {
        let path = self.url_to_page_path(url);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, markdown).await?;
        Ok(path)
    }

    pub async fn write_asset(&self, url: &str, data: &[u8]) -> anyhow::Result<PathBuf> {
        let path = self.url_to_asset_path(url);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, data).await?;
        Ok(path)
    }

    fn url_to_page_path(&self, url: &str) -> PathBuf {
        let parsed =
            url::Url::parse(url).unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
        let host = parsed.host_str().unwrap_or("unknown");
        let path = parsed.path();

        let mut file_path = self.base_dir.clone();
        file_path.push(host);

        if path == "/" || path.is_empty() {
            file_path.push("index.md");
            return file_path;
        }

        let clean = path.strip_prefix('/').unwrap_or(path);
        let segs: Vec<&str> = clean.split('/').collect();

        for (i, seg) in segs.iter().enumerate() {
            if i == segs.len() - 1 {
                if seg.is_empty() {
                    file_path.push("index.md");
                } else if let Some((stem, _)) = seg.rsplit_once('.') {
                    file_path.push(format!("{stem}.md"));
                } else {
                    file_path.push(format!("{seg}.md"));
                }
            } else {
                file_path.push(seg);
            }
        }

        file_path
    }

    fn url_to_asset_path(&self, url: &str) -> PathBuf {
        let parsed =
            url::Url::parse(url).unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
        let host = parsed.host_str().unwrap_or("unknown");
        let path = parsed.path();

        let mut file_path = self.base_dir.clone();
        file_path.push(host);

        if path == "/" || path.is_empty() {
            file_path.push("index.asset");
            return file_path;
        }

        let clean = path.strip_prefix('/').unwrap_or(path);
        let segs: Vec<&str> = clean.split('/').collect();

        for seg in segs {
            file_path.push(seg);
        }

        file_path
    }
}
