//! Batch crawl orchestration.
//!
//! A [`BatchJob`] represents a sequential run over a list of start
//! URLs, all sharing the same [`CrawlConfig`]. Each URL becomes an
//! ordinary child [`CrawlJob`] (tagged with `batch_id`) so the rest of
//! the app — history, export, persistence — works with batches without
//! knowing they are batches.
//!
//! The runner is deliberately sequential: crawls compete for global
//! semaphore slots, throttling, and disk cache, and running them in
//! parallel would tangle the progress reporting and hide per-site
//! failures. Sequential + honest is easier to reason about.

use serde::{Deserialize, Serialize};

use crate::settings::config::{BatchFailureMode, CrawlConfig};
use crate::state::HasId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BatchStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// One batch's persistent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchJob {
    pub id: String,
    /// Optional user-supplied label. When absent, the UI can fall back
    /// to "Batch of N URLs" or the first URL's host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub urls: Vec<String>,
    pub config: CrawlConfig,
    pub on_failure: BatchFailureMode,
    /// Ids of child crawl jobs, in the order they were spawned.
    #[serde(default)]
    pub child_job_ids: Vec<String>,
    pub status: BatchStatus,
    /// Zero-based index of the URL currently being crawled. When the
    /// batch is queued this is 0; when completed this is `urls.len()`.
    #[serde(default)]
    pub current_index: usize,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

impl HasId for BatchJob {
    fn id(&self) -> &str { &self.id }
}

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::AppHandle;
use tokio::sync::RwLock;

use crate::crawler::job::JobStatus;
use crate::events::bus::{CrawlEvent, EventBus};
use crate::state::{AppState, BatchHandle};

/// How often the batch runner polls a running child job for terminal status.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Upper bound on how long the batch runner waits for a single child
/// job to reach a terminal state. If a crawl hangs (fetcher deadlock,
/// unresponsive server, orchestrator panic) the runner would otherwise
/// block the whole batch forever. After this elapses, the batch
/// abandons the child (with a fresh stop signal) and treats it as failed
/// so the batch can move on.
const CHILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Grace window after the stop signal is forwarded during a timeout
/// before we declare the child abandoned. Gives a well-behaved
/// orchestrator time to notice and update its status.
const STOP_GRACE: Duration = Duration::from_secs(5);

/// Kick off a batch. Returns immediately after spawning the runner task —
/// the batch's progress is reported through events and the persisted
/// `BatchJob` state.
pub async fn spawn_batch(
    batch: BatchJob,
    state: Arc<AppState>,
    app: AppHandle,
) -> anyhow::Result<String> {
    let batch_id = batch.id.clone();

    // Persist the queued batch before we register the handle, so a
    // frontend polling `get_batch` immediately after `start_batch`
    // returns already sees it.
    state.batches.insert(batch.clone()).await?;

    let batch_arc = Arc::new(RwLock::new(batch));
    let should_stop = Arc::new(AtomicBool::new(false));
    let event_bus = EventBus::with_app(app.clone());

    let handle = BatchHandle {
        batch: batch_arc.clone(),
        should_stop: should_stop.clone(),
        event_bus: event_bus.clone(),
    };
    {
        let mut active = state.active_batches.write().await;
        active.insert(batch_id.clone(), handle);
    }

    tokio::spawn(run_batch(batch_arc, should_stop, event_bus, state, app));

    Ok(batch_id)
}

async fn run_batch(
    batch_arc: Arc<RwLock<BatchJob>>,
    should_stop: Arc<AtomicBool>,
    event_bus: EventBus,
    state: Arc<AppState>,
    app: AppHandle,
) {
    let batch_id = { batch_arc.read().await.id.clone() };
    let (urls, config, on_failure) = {
        let b = batch_arc.read().await;
        (b.urls.clone(), b.config.clone(), b.on_failure)
    };
    let total = urls.len();

    // Mark Running.
    {
        let mut b = batch_arc.write().await;
        b.status = BatchStatus::Running;
        b.start_time = Some(chrono::Utc::now().to_rfc3339());
    }
    event_bus.emit(CrawlEvent::BatchStatusChanged {
        batch_id: batch_id.clone(),
        status: BatchStatus::Running,
    });
    let _ = state.batches.insert(batch_arc.read().await.clone()).await;

    let mut final_status = BatchStatus::Completed;
    // Every child failure and every spawn failure is recorded here so
    // a Continue-mode batch of 100 URLs where 30 fail shows the user
    // all 30 messages, not just the first — otherwise the failure
    // pattern is invisible until the user opens each child's job page.
    let mut errors: Vec<String> = Vec::new();

    for (idx, url) in urls.iter().enumerate() {
        if should_stop.load(Ordering::Relaxed) {
            final_status = BatchStatus::Cancelled;
            break;
        }

        // Spawn the child crawl.
        let spawn_result = crate::commands::spawn_crawl(
            url.clone(),
            config.clone(),
            state.clone(),
            app.clone(),
            Some(batch_id.clone()),
        )
        .await;

        let job_id = match spawn_result {
            Ok(id) => id,
            Err(e) => {
                // Treat spawn failure like a failed child job.
                errors.push(format!("Failed to spawn crawl for '{}': {}", url, e));
                if on_failure == BatchFailureMode::Stop {
                    final_status = BatchStatus::Failed;
                    break;
                } else {
                    // Continue: bump the index, record and move on.
                    let mut b = batch_arc.write().await;
                    b.current_index = idx + 1;
                    continue;
                }
            }
        };

        {
            let mut b = batch_arc.write().await;
            b.child_job_ids.push(job_id.clone());
            b.current_index = idx;
        }
        event_bus.emit(CrawlEvent::BatchProgress {
            batch_id: batch_id.clone(),
            current_index: idx,
            total,
            current_job_id: Some(job_id.clone()),
        });
        let _ = state.batches.insert(batch_arc.read().await.clone()).await;

        // Wait for the child job to hit a terminal state.
        let terminal_status = wait_for_terminal(&state, &job_id, &should_stop).await;

        match terminal_status {
            Some(JobStatus::Failed) => {
                let child_err = state
                    .jobs
                    .get(&job_id)
                    .await
                    .and_then(|j| j.error)
                    .unwrap_or_else(|| "unknown error".to_string());
                errors.push(format!("{}: {}", url, child_err));
                if on_failure == BatchFailureMode::Stop {
                    final_status = BatchStatus::Failed;
                    // Advance index one past the last attempted so UI shows N/N.
                    let mut b = batch_arc.write().await;
                    b.current_index = idx + 1;
                    break;
                }
            }
            Some(JobStatus::Cancelled) => {
                // Cancellation of a child (typically because the batch was
                // stopped) ends the batch too.
                final_status = BatchStatus::Cancelled;
                let mut b = batch_arc.write().await;
                b.current_index = idx + 1;
                break;
            }
            _ => {}
        }

        {
            let mut b = batch_arc.write().await;
            b.current_index = idx + 1;
        }
    }

    // Finalize.
    {
        let mut b = batch_arc.write().await;
        b.status = final_status.clone();
        b.end_time = Some(chrono::Utc::now().to_rfc3339());
        if !errors.is_empty() {
            // Prefix with the count so a batch with many failures is
            // immediately obvious at a glance; the joined messages
            // follow for detail.
            let n = errors.len();
            b.error = Some(format!(
                "{} child failure{}:\n{}",
                n,
                if n == 1 { "" } else { "s" },
                errors.join("\n"),
            ));
        }
        // Ensure current_index reflects completion.
        if final_status == BatchStatus::Completed {
            b.current_index = total;
        }
    }
    event_bus.emit(CrawlEvent::BatchProgress {
        batch_id: batch_id.clone(),
        current_index: total,
        total,
        current_job_id: None,
    });
    event_bus.emit(CrawlEvent::BatchStatusChanged {
        batch_id: batch_id.clone(),
        status: final_status,
    });
    let _ = state.batches.insert(batch_arc.read().await.clone()).await;

    // Drop from active_batches.
    let mut active = state.active_batches.write().await;
    active.remove(&batch_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::config::{BatchFailureMode, CrawlConfig};

    fn sample_config() -> CrawlConfig {
        CrawlConfig {
            output_dir: String::new(),
            max_depth: 2,
            page_limit: 10,
            download_assets: false,
            headless_strategy: "never".into(),
            content_selectors: vec![],
            exclude_patterns: vec![],
            include_patterns: vec![],
            path_prefix: String::new(),
            respect_robots_txt: true,
            stay_within_domain: true,
            ssrf_protection: true,
            profile: None,
        }
    }

    #[test]
    fn batch_job_has_id() {
        let batch = BatchJob {
            id: "b1".into(),
            name: None,
            urls: vec!["https://a".into()],
            config: sample_config(),
            on_failure: BatchFailureMode::Continue,
            child_job_ids: vec![],
            status: BatchStatus::Queued,
            current_index: 0,
            created_at: "2026-07-11T00:00:00Z".into(),
            error: None,
            start_time: None,
            end_time: None,
        };
        assert_eq!(batch.id(), "b1");
    }

    #[test]
    fn batch_job_roundtrips_json() {
        let batch = BatchJob {
            id: "b1".into(),
            name: Some("release notes".into()),
            urls: vec!["https://a".into(), "https://b".into()],
            config: sample_config(),
            on_failure: BatchFailureMode::Stop,
            child_job_ids: vec!["c1".into()],
            status: BatchStatus::Running,
            current_index: 1,
            created_at: "2026-07-11T00:00:00Z".into(),
            error: None,
            start_time: Some("2026-07-11T00:00:01Z".into()),
            end_time: None,
        };
        let json = serde_json::to_string(&batch).unwrap();
        let back: BatchJob = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "b1");
        assert_eq!(back.on_failure, BatchFailureMode::Stop);
        assert_eq!(back.status, BatchStatus::Running);
        assert_eq!(back.child_job_ids, vec!["c1"]);
        assert_eq!(back.current_index, 1);
    }

    #[test]
    fn batch_job_camelcase_serialization() {
        let batch = BatchJob {
            id: "b1".into(),
            name: None,
            urls: vec![],
            config: sample_config(),
            on_failure: BatchFailureMode::Continue,
            child_job_ids: vec![],
            status: BatchStatus::Queued,
            current_index: 0,
            created_at: "2026-07-11T00:00:00Z".into(),
            error: None,
            start_time: None,
            end_time: None,
        };
        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("\"onFailure\":\"continue\""));
        assert!(json.contains("\"childJobIds\""));
        assert!(json.contains("\"currentIndex\""));
        assert!(json.contains("\"createdAt\""));
    }

    #[test]
    fn is_terminal_matches_all_terminal_states() {
        assert!(is_terminal(&JobStatus::Completed));
        assert!(is_terminal(&JobStatus::Failed));
        assert!(is_terminal(&JobStatus::Cancelled));
        assert!(!is_terminal(&JobStatus::Queued));
        assert!(!is_terminal(&JobStatus::Running));
        assert!(!is_terminal(&JobStatus::Paused));
    }
}

/// Poll `active_jobs` for `job_id` until its status is terminal.
///
/// If `should_stop` is raised, forwards the stop signal to the child
/// job's handle so it can shut down cleanly, then keeps polling until
/// the child actually reaches a terminal state. If [`CHILD_TIMEOUT`]
/// elapses without a terminal status, forwards a stop signal and — if
/// the child still hasn't caught up after [`STOP_GRACE`] — abandons
/// with `JobStatus::Failed` so the batch can proceed.
async fn wait_for_terminal(
    state: &AppState,
    job_id: &str,
    batch_should_stop: &AtomicBool,
) -> Option<JobStatus> {
    let mut stop_forwarded = false;
    let mut timeout_hit_at: Option<std::time::Instant> = None;
    let started = std::time::Instant::now();
    loop {
        // Forward a batch stop to the running child once — either because
        // the user cancelled or because we hit CHILD_TIMEOUT.
        let should_forward = !stop_forwarded
            && (batch_should_stop.load(Ordering::Relaxed) || timeout_hit_at.is_some());
        if should_forward {
            let jobs = state.active_jobs.read().await;
            if let Some(handle) = jobs.get(job_id) {
                handle.should_stop.store(true, Ordering::Relaxed);
                // In case the child was paused, unblock it so it can
                // observe the stop signal.
                handle.should_pause.store(false, Ordering::Relaxed);
                handle.resume_notify.notify_one();
            }
            stop_forwarded = true;
        }

        // Read current status.
        let status = {
            let jobs = state.active_jobs.read().await;
            match jobs.get(job_id) {
                Some(handle) => Some(handle.job.read().await.status.clone()),
                None => None,
            }
        };
        match status {
            Some(s) if is_terminal(&s) => return Some(s),
            Some(_) => {
                if let Some(hit_at) = timeout_hit_at {
                    // We've already forwarded stop and given the child a
                    // grace window; if it's still not terminal, abandon.
                    if hit_at.elapsed() >= STOP_GRACE {
                        return Some(JobStatus::Failed);
                    }
                } else if started.elapsed() >= CHILD_TIMEOUT {
                    timeout_hit_at = Some(std::time::Instant::now());
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            // Job vanished from active_jobs — try the persisted store,
            // then bail out.
            None => {
                return state.jobs.get(job_id).await.map(|j| j.status);
            }
        }
    }
}

fn is_terminal(s: &JobStatus) -> bool {
    matches!(
        s,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    )
}


