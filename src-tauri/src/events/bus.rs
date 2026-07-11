use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast;
use crate::crawler::batch::BatchStatus;
use crate::crawler::job::{CrawlProgress, JobStatus, PageMeta};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Network,
    Disk,
    Parse,
    RobotsBlocked,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
// `rename_all` only renames variant names for the `tag`; struct fields
// inside variants need `rename_all_fields`. Without it, emitted JSON
// carries snake_case field names (`job_id`, `batch_id`) even though the
// tag is camelCase — silently breaking every `event.jobId` read in the
// frontend (`useCrawlEvents`, `LiveConsole`).
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "type")]
pub enum CrawlEvent {
    Progress { job_id: String, progress: CrawlProgress },
    Log { job_id: String, level: String, message: String },
    PageComplete { job_id: String, page: PageMeta },
    JobStatusChanged { job_id: String, status: JobStatus },
    Error { job_id: String, message: String, kind: ErrorKind },
    BatchProgress {
        batch_id: String,
        current_index: usize,
        total: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_job_id: Option<String>,
    },
    BatchStatusChanged { batch_id: String, status: BatchStatus },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawler::batch::BatchStatus;

    fn v(e: CrawlEvent) -> serde_json::Value {
        serde_json::to_value(&e).unwrap()
    }

    #[test]
    fn job_status_changed_emits_camelcase_jobid() {
        let p = v(CrawlEvent::JobStatusChanged {
            job_id: "abc".into(),
            status: JobStatus::Running,
        });
        assert_eq!(p["type"], "jobStatusChanged");
        assert_eq!(p["jobId"], "abc");
        assert_eq!(p["status"], "running");
        assert!(p.get("job_id").is_none(), "must not emit snake_case");
    }

    #[test]
    fn progress_emits_camelcase_jobid() {
        let p = v(CrawlEvent::Progress {
            job_id: "abc".into(),
            progress: CrawlProgress {
                pages_crawled: 1,
                page_limit: 10,
                current_url: String::new(),
                depth: 0,
                max_depth: 1,
                start_time: None,
            },
        });
        assert_eq!(p["type"], "progress");
        assert_eq!(p["jobId"], "abc");
    }

    #[test]
    fn batch_progress_emits_camelcase_fields() {
        let p = v(CrawlEvent::BatchProgress {
            batch_id: "b1".into(),
            current_index: 2,
            total: 5,
            current_job_id: Some("c1".into()),
        });
        assert_eq!(p["type"], "batchProgress");
        assert_eq!(p["batchId"], "b1");
        assert_eq!(p["currentIndex"], 2);
        assert_eq!(p["total"], 5);
        assert_eq!(p["currentJobId"], "c1");
    }

    #[test]
    fn batch_status_changed_emits_camelcase_batchid() {
        let p = v(CrawlEvent::BatchStatusChanged {
            batch_id: "b1".into(),
            status: BatchStatus::Running,
        });
        assert_eq!(p["type"], "batchStatusChanged");
        assert_eq!(p["batchId"], "b1");
    }
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<CrawlEvent>,
    app: Option<AppHandle>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self { tx, app: None }
    }

    pub fn with_app(app: AppHandle) -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self { tx, app: Some(app) }
    }

    pub fn set_app(&mut self, app: AppHandle) {
        self.app = Some(app);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CrawlEvent> {
        self.tx.subscribe()
    }

    pub fn emit(&self, event: CrawlEvent) {
        let _ = self.tx.send(event.clone());
        if let Some(ref app) = self.app {
            let payload = match serde_json::to_value(&event) {
                Ok(v) => v,
                Err(_) => return,
            };
            let _ = app.emit("crawl-event", payload);
        }
    }
}
