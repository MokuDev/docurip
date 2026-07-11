use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::fs;
use tokio::sync::{Notify, RwLock, RwLockReadGuard};
use uuid::Uuid;

use crate::crawler::batch::BatchJob;
use crate::crawler::job::CrawlJob;
use crate::events::bus::EventBus;
use crate::settings::templates::CrawlTemplate;

pub struct JobHandle {
    pub job: Arc<RwLock<CrawlJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub should_pause: Arc<AtomicBool>,
    pub resume_notify: Arc<Notify>,
    pub event_bus: EventBus,
}

/// Live-batch bookkeeping. Held in `AppState.active_batches` while a
/// batch runs so `stop_batch` / status polling can reach into the
/// running task.
pub struct BatchHandle {
    pub batch: Arc<RwLock<BatchJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub event_bus: EventBus,
}

/// Types stored in a [`JsonStore`] need a stable id used for the on-disk
/// filename and as the in-memory HashMap key.
pub trait HasId {
    fn id(&self) -> &str;
}

impl HasId for CrawlJob {
    fn id(&self) -> &str { &self.id }
}

impl HasId for CrawlTemplate {
    fn id(&self) -> &str { &self.id }
}

// `BatchJob` implements `HasId` in its defining module.

/// One-JSON-file-per-entry, in-memory-cached, `RwLock`-protected store.
///
/// Used for both crawl-job snapshots and user-defined templates. `insert`
/// writes to disk and updates the cache atomically (from the caller's
/// point of view — writes are serialized behind the write lock). Files
/// live at `<dir>/<id>.json`.
pub struct JsonStore<T> {
    dir: PathBuf,
    entries: RwLock<HashMap<String, T>>,
}

impl<T> JsonStore<T>
where
    T: HasId + Serialize + DeserializeOwned + Clone + Send + Sync,
{
    /// Load all `*.json` files from `dir` into the cache. Invalid files
    /// are silently skipped so a single corrupt entry doesn't prevent
    /// startup — this now also applies to per-file IO errors
    /// (permission denied, file locked, transient read failure) which
    /// would otherwise abort `init` and prevent the app from starting.
    /// A failure to list the directory itself is still fatal — that
    /// typically means the app data dir is misconfigured or the disk
    /// is missing, and we'd rather surface that than start silently.
    pub fn init(dir: PathBuf) -> anyhow::Result<Self> {
        let mut entries: HashMap<String, T> = HashMap::new();
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let contents = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if let Ok(item) = serde_json::from_str::<T>(&contents) {
                    entries.insert(item.id().to_string(), item);
                }
            }
        }
        Ok(Self {
            dir,
            entries: RwLock::new(entries),
        })
    }

    /// Read-guard over the in-memory cache. Use when the caller needs to
    /// iterate over entries; the guard holds the read lock, so drop it
    /// promptly.
    pub async fn read(&self) -> RwLockReadGuard<'_, HashMap<String, T>> {
        self.entries.read().await
    }

    /// Clone one entry out of the cache, releasing the lock before
    /// returning. Prefer this to `read().get(id).cloned()` for one-off
    /// lookups.
    pub async fn get(&self, id: &str) -> Option<T> {
        self.entries.read().await.get(id).cloned()
    }

    /// Write `item` to disk and update the cache. Overwrites any existing
    /// entry with the same id.
    pub async fn insert(&self, item: T) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir).await?;
        let path = self.dir.join(format!("{}.json", item.id()));
        let json = serde_json::to_string_pretty(&item)?;
        fs::write(path, json).await?;
        let mut guard = self.entries.write().await;
        guard.insert(item.id().to_string(), item);
        Ok(())
    }

    /// Delete the on-disk file (if present) and remove from the cache.
    /// Returns `Ok(())` even if the id was not known.
    pub async fn remove(&self, id: &str) -> anyhow::Result<()> {
        let path = self.dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path).await?;
        }
        let mut guard = self.entries.write().await;
        guard.remove(id);
        Ok(())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

pub struct AppState {
    pub active_jobs: RwLock<HashMap<String, JobHandle>>,
    pub active_batches: RwLock<HashMap<String, BatchHandle>>,
    pub jobs: JsonStore<CrawlJob>,
    pub templates: JsonStore<CrawlTemplate>,
    pub batches: JsonStore<BatchJob>,
    pub session_id: String,
    pub start_time: Instant,
}

impl AppState {
    pub fn init(
        persist_dir: PathBuf,
        templates_dir: PathBuf,
        batches_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            active_jobs: RwLock::new(HashMap::new()),
            active_batches: RwLock::new(HashMap::new()),
            jobs: JsonStore::init(persist_dir)?,
            templates: JsonStore::init(templates_dir)?,
            batches: JsonStore::init(batches_dir)?,
            session_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        })
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    // ---- Back-compat wrappers over the `jobs` store ----

    pub async fn persist_job(&self, job: &CrawlJob) -> anyhow::Result<()> {
        self.jobs.insert(job.clone()).await
    }

    pub async fn remove_persisted_job(&self, job_id: &str) -> anyhow::Result<()> {
        self.jobs.remove(job_id).await
    }

    // ---- Back-compat wrappers over the `templates` store ----

    pub async fn persist_template(&self, template: &CrawlTemplate) -> anyhow::Result<()> {
        self.templates.insert(template.clone()).await
    }

    pub async fn remove_persisted_template(&self, template_id: &str) -> anyhow::Result<()> {
        self.templates.remove(template_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_job(id: &str) -> CrawlJob {
        CrawlJob {
            id: id.to_string(),
            url: "https://example.com".to_string(),
            status: crate::crawler::job::JobStatus::Completed,
            config: crate::settings::config::CrawlConfig {
                output_dir: "/tmp".to_string(),
                max_depth: 2,
                page_limit: 10,
                download_assets: false,
                headless_strategy: "auto".to_string(),
                content_selectors: vec![],
                exclude_patterns: vec![],
                include_patterns: vec![],
                path_prefix: String::new(),
                respect_robots_txt: true,
                stay_within_domain: true,
                ssrf_protection: true,
                profile: None,
            },
            results: vec![],
            progress: crate::crawler::job::CrawlProgress {
                pages_crawled: 0,
                page_limit: 10,
                current_url: String::new(),
                depth: 0,
                max_depth: 2,
                start_time: None,
            },
            error: None,
            start_time: None,
            end_time: None,
            batch_id: None,
        }
    }

    fn create_test_template(id: &str) -> CrawlTemplate {
        CrawlTemplate {
            id: id.to_string(),
            name: "My Template".to_string(),
            url: "https://example.com".to_string(),
            config: crate::settings::config::CrawlConfig {
                output_dir: "/tmp".to_string(),
                max_depth: 2,
                page_limit: 10,
                download_assets: false,
                headless_strategy: "auto".to_string(),
                content_selectors: vec![],
                exclude_patterns: vec![],
                include_patterns: vec![],
                path_prefix: String::new(),
                respect_robots_txt: true,
                stay_within_domain: true,
                ssrf_protection: true,
                profile: None,
            },
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn json_store_insert_and_get() {
        let dir = TempDir::new().unwrap();
        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();

        let job = create_test_job("job-1");
        store.insert(job.clone()).await.unwrap();

        assert!(dir.path().join("job-1.json").exists());
        let loaded = store.get("job-1").await.unwrap();
        assert_eq!(loaded.id, job.id);
    }

    #[tokio::test]
    async fn json_store_remove_deletes_file_and_cache() {
        let dir = TempDir::new().unwrap();
        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
        let job = create_test_job("job-1");
        store.insert(job).await.unwrap();

        store.remove("job-1").await.unwrap();
        assert!(!dir.path().join("job-1.json").exists());
        assert!(store.get("job-1").await.is_none());
    }

    #[tokio::test]
    async fn json_store_remove_unknown_id_is_ok() {
        let dir = TempDir::new().unwrap();
        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
        assert!(store.remove("does-not-exist").await.is_ok());
    }

    #[tokio::test]
    async fn json_store_init_loads_all_entries() {
        let dir = TempDir::new().unwrap();
        {
            let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
            store.insert(create_test_job("a")).await.unwrap();
            store.insert(create_test_job("b")).await.unwrap();
        }
        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
        let entries = store.read().await;
        assert_eq!(entries.len(), 2);
        assert!(entries.contains_key("a"));
        assert!(entries.contains_key("b"));
    }

    #[tokio::test]
    async fn json_store_init_skips_corrupt_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("bad.json"), "not json").unwrap();
        {
            let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
            store.insert(create_test_job("good")).await.unwrap();
        }
        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
        let entries = store.read().await;
        assert_eq!(entries.len(), 1);
        assert!(entries.contains_key("good"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn json_store_init_skips_unreadable_files_on_unix() {
        // A file we can't read (permissions) must not abort init — on
        // Windows this manifests as ERROR_SHARING_VIOLATION when another
        // process has the file open; the same tolerance is what protects
        // startup there.
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        {
            let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
            store.insert(create_test_job("readable")).await.unwrap();
        }
        // Make a second, unreadable file
        let bad_path = dir.path().join("locked.json");
        std::fs::write(&bad_path, "{}").unwrap();
        std::fs::set_permissions(&bad_path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let store: JsonStore<CrawlJob> = JsonStore::init(dir.path().to_path_buf()).unwrap();
        let entries = store.read().await;
        assert!(entries.contains_key("readable"));
        // Restore perms so tempdir cleanup can remove the file.
        std::fs::set_permissions(&bad_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    #[tokio::test]
    async fn app_state_persist_job_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let state = AppState::init(
            temp_dir.path().join("jobs"),
            temp_dir.path().join("templates"),
            temp_dir.path().join("batches"),
        )
        .unwrap();
        let job = create_test_job("job-1");

        state.persist_job(&job).await.unwrap();
        let loaded = state.jobs.get("job-1").await.unwrap();
        assert_eq!(loaded.url, job.url);

        state.remove_persisted_job("job-1").await.unwrap();
        assert!(state.jobs.get("job-1").await.is_none());
    }

    #[tokio::test]
    async fn app_state_persist_template_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let state = AppState::init(
            temp_dir.path().join("jobs"),
            temp_dir.path().join("templates"),
            temp_dir.path().join("batches"),
        )
        .unwrap();
        let template = create_test_template("tpl-1");

        state.persist_template(&template).await.unwrap();
        let templates = state.templates.read().await;
        assert_eq!(templates.get("tpl-1").unwrap().name, "My Template");
        drop(templates);

        state.remove_persisted_template("tpl-1").await.unwrap();
        assert!(state.templates.get("tpl-1").await.is_none());
    }

    #[tokio::test]
    async fn app_state_init_loads_persisted_templates() {
        let temp_dir = TempDir::new().unwrap();
        let templates_dir = temp_dir.path().join("templates");
        {
            let store: JsonStore<CrawlTemplate> =
                JsonStore::init(templates_dir.clone()).unwrap();
            store.insert(create_test_template("tpl-1")).await.unwrap();
        }
        let state = AppState::init(
            temp_dir.path().join("jobs"),
            templates_dir,
            temp_dir.path().join("batches"),
        )
        .unwrap();
        let templates = state.templates.read().await;
        assert_eq!(templates.len(), 1);
        assert!(templates.contains_key("tpl-1"));
    }
}
