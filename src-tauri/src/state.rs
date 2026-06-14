use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::sync::{Notify, RwLock};
use uuid::Uuid;

use crate::crawler::job::CrawlJob;
use crate::events::bus::EventBus;

pub struct JobHandle {
    pub job: Arc<RwLock<CrawlJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub should_pause: Arc<AtomicBool>,
    pub resume_notify: Arc<Notify>,
    pub event_bus: EventBus,
}

pub struct AppState {
    pub active_jobs: RwLock<HashMap<String, JobHandle>>,
    pub persisted_jobs: RwLock<HashMap<String, CrawlJob>>,
    pub persist_dir: PathBuf,
    pub session_id: String,
    pub start_time: Instant,
}

impl AppState {
    pub fn init(persist_dir: PathBuf) -> anyhow::Result<Self> {
        let mut jobs = Vec::new();
        if persist_dir.exists() {
            for entry in std::fs::read_dir(&persist_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let contents = std::fs::read_to_string(&path)?;
                    if let Ok(job) = serde_json::from_str::<CrawlJob>(&contents) {
                        jobs.push(job);
                    }
                }
            }
        }
        let persisted_jobs: HashMap<String, CrawlJob> =
            jobs.into_iter().map(|job| (job.id.clone(), job)).collect();

        Ok(Self {
            active_jobs: RwLock::new(HashMap::new()),
            persisted_jobs: RwLock::new(persisted_jobs),
            persist_dir,
            session_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        })
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    pub async fn save_job_to_disk(dir: &Path, job: &CrawlJob) -> anyhow::Result<()> {
        fs::create_dir_all(dir).await?;
        let path = dir.join(format!("{}.json", job.id));
        let json = serde_json::to_string_pretty(job)?;
        fs::write(path, json).await?;
        Ok(())
    }

    pub async fn load_job_from_disk(dir: &Path, job_id: &str) -> anyhow::Result<CrawlJob> {
        let path = dir.join(format!("{}.json", job_id));
        let contents = fs::read_to_string(path).await?;
        let job = serde_json::from_str(&contents)?;
        Ok(job)
    }

    pub async fn load_all_jobs(dir: &Path) -> anyhow::Result<Vec<CrawlJob>> {
        fs::create_dir_all(dir).await?;
        let mut jobs = Vec::new();
        let mut entries = fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let contents = fs::read_to_string(&path).await?;
                let job: CrawlJob = serde_json::from_str(&contents)?;
                jobs.push(job);
            }
        }
        Ok(jobs)
    }

    pub async fn delete_job_from_disk(dir: &Path, job_id: &str) -> anyhow::Result<()> {
        let path = dir.join(format!("{}.json", job_id));
        if path.exists() {
            fs::remove_file(path).await?;
        }
        Ok(())
    }

    pub async fn persist_job(&self, job: &CrawlJob) -> anyhow::Result<()> {
        Self::save_job_to_disk(&self.persist_dir, job).await?;
        let mut persisted = self.persisted_jobs.write().await;
        persisted.insert(job.id.clone(), job.clone());
        Ok(())
    }

    pub async fn remove_persisted_job(&self, job_id: &str) -> anyhow::Result<()> {
        Self::delete_job_from_disk(&self.persist_dir, job_id).await?;
        let mut persisted = self.persisted_jobs.write().await;
        persisted.remove(job_id);
        Ok(())
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
                respect_robots_txt: true,
                stay_within_domain: true,
                ssrf_protection: true,
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
        }
    }

    #[tokio::test]
    async fn test_save_and_load_job() {
        let temp_dir = TempDir::new().unwrap();
        let job = create_test_job("job-1");

        AppState::save_job_to_disk(temp_dir.path(), &job).await.unwrap();
        let loaded = AppState::load_job_from_disk(temp_dir.path(), "job-1").await.unwrap();

        assert_eq!(loaded.id, job.id);
        assert_eq!(loaded.url, job.url);
    }

    #[tokio::test]
    async fn test_load_all_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let job1 = create_test_job("job-1");
        let job2 = create_test_job("job-2");

        AppState::save_job_to_disk(temp_dir.path(), &job1).await.unwrap();
        AppState::save_job_to_disk(temp_dir.path(), &job2).await.unwrap();

        let jobs = AppState::load_all_jobs(temp_dir.path()).await.unwrap();
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let temp_dir = TempDir::new().unwrap();
        let job = create_test_job("job-1");

        AppState::save_job_to_disk(temp_dir.path(), &job).await.unwrap();
        assert!(temp_dir.path().join("job-1.json").exists());

        AppState::delete_job_from_disk(temp_dir.path(), "job-1").await.unwrap();
        assert!(!temp_dir.path().join("job-1.json").exists());
    }
}
