use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crawler::job::CrawlJob;
use crate::events::bus::EventBus;

pub struct JobHandle {
    pub job: Arc<RwLock<CrawlJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub event_bus: EventBus,
}

#[derive(Default)]
pub struct AppState {
    pub active_jobs: RwLock<HashMap<String, JobHandle>>,
}
