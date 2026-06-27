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

    pub async fn write_asset(&self, url: &str, data: &[u8]) -> anyhow::Result<String> {
        let path = self.url_to_asset_path(url);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, data).await?;
        let rel = path
            .strip_prefix(&self.base_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        Ok(rel)
    }

    /// Sanitize a path segment to prevent directory traversal.
    fn sanitize_segment(seg: &str) -> Option<&str> {
        if seg == ".." || seg == "." || seg.is_empty() {
            None
        } else {
            Some(seg)
        }
    }

    pub fn url_to_page_path(&self, url: &str) -> PathBuf {
        let parsed =
            url::Url::parse(url).unwrap_or_else(|_| url::Url::parse("http://localhost/").unwrap());
        let host = parsed.host_str().unwrap_or("unknown");
        let path = parsed.path();

        let mut file_path = self.base_dir.clone();
        file_path.push(Self::sanitize_segment(host).unwrap_or("unknown"));

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
                    let safe_stem = Self::sanitize_segment(stem).unwrap_or("unnamed");
                    file_path.push(format!("{safe_stem}.md"));
                } else {
                    let safe_seg = Self::sanitize_segment(seg).unwrap_or("unnamed");
                    file_path.push(format!("{safe_seg}.md"));
                }
            } else {
                if let Some(safe) = Self::sanitize_segment(seg) {
                    file_path.push(safe);
                }
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
        file_path.push(Self::sanitize_segment(host).unwrap_or("unknown"));

        if path == "/" || path.is_empty() {
            file_path.push("index.asset");
            return file_path;
        }

        let clean = path.strip_prefix('/').unwrap_or(path);
        let segs: Vec<&str> = clean.split('/').collect();

        for seg in segs {
            if let Some(safe) = Self::sanitize_segment(seg) {
                file_path.push(safe);
            }
        }

        file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_page_path_basic() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_page_path("https://example.com/docs/guide");
        assert_eq!(path, PathBuf::from("/output/example.com/docs/guide.md"));
    }

    #[test]
    fn test_url_to_page_path_root() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_page_path("https://example.com/");
        assert_eq!(path, PathBuf::from("/output/example.com/index.md"));
    }

    #[test]
    fn test_url_to_page_path_with_extension() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_page_path("https://example.com/page.html");
        assert_eq!(path, PathBuf::from("/output/example.com/page.md"));
    }

    #[test]
    fn test_url_to_page_path_traversal_blocked() {
        let writer = FsWriter::new("/output");
        // ".." in path should be sanitized out
        let path = writer.url_to_page_path("https://example.com/docs/../etc/passwd");
        assert!(!path.to_string_lossy().contains(".."));
        // Should resolve within base_dir
        assert!(path.starts_with("/output"));
    }

    #[test]
    fn test_url_to_page_path_traversal_in_host() {
        let writer = FsWriter::new("/output");
        // ".." in host should be sanitized
        let path = writer.url_to_page_path("https://../etc/passwd");
        // Host ".." becomes "unknown" after sanitization fails
        assert!(path.starts_with("/output"));
        assert!(!path.to_string_lossy().contains("../"));
    }

    #[test]
    fn test_url_to_asset_path_basic() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_asset_path("https://example.com/images/logo.png");
        assert_eq!(
            path,
            PathBuf::from("/output/example.com/images/logo.png")
        );
    }

    #[test]
    fn test_url_to_asset_path_traversal_blocked() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_asset_path("https://example.com/images/../secret.txt");
        assert!(!path.to_string_lossy().contains(".."));
        assert!(path.starts_with("/output"));
    }

    #[test]
    fn test_sanitize_segment() {
        assert_eq!(FsWriter::sanitize_segment("safe"), Some("safe"));
        assert_eq!(FsWriter::sanitize_segment(".."), None);
        assert_eq!(FsWriter::sanitize_segment("."), None);
        assert_eq!(FsWriter::sanitize_segment(""), None);
    }

    #[test]
    fn test_url_with_query_string_stripped() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_page_path("https://example.com/docs?page=1&lang=en");
        assert_eq!(path, PathBuf::from("/output/example.com/docs.md"));
    }

    #[test]
    fn test_url_with_fragment_stripped() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_page_path("https://example.com/guide#section-2");
        assert_eq!(path, PathBuf::from("/output/example.com/guide.md"));
    }

    #[test]
    fn test_asset_url_with_query_string() {
        let writer = FsWriter::new("/output");
        let path = writer.url_to_asset_path("https://example.com/img/logo.png?v=2");
        assert_eq!(path, PathBuf::from("/output/example.com/img/logo.png"));
    }
}
