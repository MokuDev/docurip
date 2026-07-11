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
#[serde(rename_all = "camelCase", tag = "type")]
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
