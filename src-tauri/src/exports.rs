use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentExport {
    pub path: String,
    pub job_id: String,
    pub created_at: String,
    pub size_bytes: u64,
}

pub fn list_recent_exports(output_dirs: &[std::path::PathBuf], n: usize) -> Vec<RecentExport> {
    let mut entries: Vec<RecentExport> = Vec::new();

    for output_dir in output_dirs {
        let zip_dir = output_dir.join("zip");
        if !zip_dir.exists() {
            continue;
        }

        let rd = match std::fs::read_dir(&zip_dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for entry in rd.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("zip") {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) if m.is_file() => m,
                _ => continue,
            };
            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(f) => f.to_string(),
                None => continue,
            };
            let job_id = match filename.strip_suffix(".zip") {
                Some(j) => j.to_string(),
                None => continue,
            };
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
            entries.push(RecentExport {
                path: path.to_string_lossy().to_string(),
                job_id,
                created_at,
                size_bytes: meta.len(),
            });
        }
    }

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

    fn touch_zip(dir: &std::path::Path, name: &str) {
        let p = dir.join(name);
        let mut f = File::create(&p).unwrap();
        f.write_all(b"PK\x03\x04fake").unwrap();
    }

    fn touch_zip_at(dir: &std::path::Path, name: &str, mtime: std::time::SystemTime) {
        let p = dir.join(name);
        let mut f = File::create(&p).unwrap();
        f.write_all(b"PK\x03\x04fake").unwrap();
        f.set_modified(mtime).unwrap();
    }

    #[test]
    fn missing_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let result = list_recent_exports(&[tmp.path().to_path_buf()], 5);
        assert!(result.is_empty());
    }

    #[test]
    fn filters_zips_and_sorts_by_modified() {
        let tmp = TempDir::new().unwrap();
        let zip_dir = tmp.path().join("zip");
        fs::create_dir_all(&zip_dir).unwrap();
        let t0 = std::time::SystemTime::now();
        touch_zip_at(&zip_dir, "job-a.zip", t0);
        touch_zip_at(&zip_dir, "job-b.zip", t0 + std::time::Duration::from_secs(60));
        fs::write(zip_dir.join("notes.txt"), b"not a zip").unwrap();

        let result = list_recent_exports(&[tmp.path().to_path_buf()], 5);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].job_id, "job-b");
        assert_eq!(result[1].job_id, "job-a");
        assert!(result[0].size_bytes > 0);
    }

    #[test]
    fn limits_to_n() {
        let tmp = TempDir::new().unwrap();
        let zip_dir = tmp.path().join("zip");
        fs::create_dir_all(&zip_dir).unwrap();
        for i in 0..10 {
            touch_zip(&zip_dir, &format!("job-{i}.zip"));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let result = list_recent_exports(&[tmp.path().to_path_buf()], 3);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn scans_multiple_output_dirs() {
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();
        let zip1 = tmp1.path().join("zip");
        let zip2 = tmp2.path().join("zip");
        fs::create_dir_all(&zip1).unwrap();
        fs::create_dir_all(&zip2).unwrap();
        touch_zip(&zip1, "job-a.zip");
        touch_zip(&zip2, "job-b.zip");

        let dirs = vec![tmp1.path().to_path_buf(), tmp2.path().to_path_buf()];
        let result = list_recent_exports(&dirs, 5);
        assert_eq!(result.len(), 2);
    }
}
