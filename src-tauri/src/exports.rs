use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentExport {
    pub path: String,
    pub job_id: String,
    pub created_at: String,
    pub size_bytes: u64,
}

pub fn list_recent_exports(app_data_dir: &Path, n: usize) -> Vec<RecentExport> {
    let dir = app_data_dir.join("exports");
    if !dir.exists() {
        return Vec::new();
    }

    let mut entries: Vec<RecentExport> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("zip") {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                if !meta.is_file() {
                    return None;
                }
                let filename = path.file_name()?.to_str()?.to_string();
                let job_id = filename.strip_suffix(".zip")?.to_string();
                let created_at = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                Some(RecentExport {
                    path: path.to_string_lossy().to_string(),
                    job_id,
                    created_at,
                    size_bytes: meta.len(),
                })
            })
            .collect(),
        Err(_) => return Vec::new(),
    };

    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    entries.truncate(n);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn touch_zip(dir: &Path, name: &str) {
        let p = dir.join(name);
        let mut f = File::create(&p).unwrap();
        f.write_all(b"PK\x03\x04fake").unwrap();
    }

    fn touch_zip_at(dir: &Path, name: &str, mtime: std::time::SystemTime) {
        let p = dir.join(name);
        let mut f = File::create(&p).unwrap();
        f.write_all(b"PK\x03\x04fake").unwrap();
        f.set_modified(mtime).unwrap();
    }

    #[test]
    fn missing_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let result = list_recent_exports(tmp.path(), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn filters_zips_and_sorts_by_modified() {
        let tmp = TempDir::new().unwrap();
        let exports = tmp.path().join("exports");
        fs::create_dir_all(&exports).unwrap();
        let t0 = std::time::SystemTime::now();
        touch_zip_at(&exports, "job-a.zip", t0);
        touch_zip_at(&exports, "job-b.zip", t0 + std::time::Duration::from_secs(60));
        fs::write(exports.join("notes.txt"), b"not a zip").unwrap();

        let result = list_recent_exports(tmp.path(), 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].job_id, "job-b");
        assert_eq!(result[1].job_id, "job-a");
        assert!(result[0].size_bytes > 0);
    }

    #[test]
    fn limits_to_n() {
        let tmp = TempDir::new().unwrap();
        let exports = tmp.path().join("exports");
        fs::create_dir_all(&exports).unwrap();
        for i in 0..10 {
            touch_zip(&exports, &format!("job-{i}.zip"));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let result = list_recent_exports(tmp.path(), 3);
        assert_eq!(result.len(), 3);
    }
}
