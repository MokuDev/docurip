use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus};
use crate::crawler::orchestrator::{CrawlHandle, Orchestrator};
use crate::events::bus::EventBus;
use crate::settings::config::{AppSettings, CrawlConfig};
use crate::state::{AppState, JobHandle};
use url::Url;

fn validate_crawl_input(url: &str, config: &CrawlConfig) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("URL scheme must be http or https".to_string());
    }
    if config.max_depth < 1 {
        return Err("max_depth must be at least 1".to_string());
    }
    if config.page_limit < 1 {
        return Err("page_limit must be at least 1".to_string());
    }
    if config.headless_strategy.is_empty() {
        return Err("headless_strategy must not be empty".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn start_crawl(
    url: String,
    config: CrawlConfig,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<String, String> {
    validate_crawl_input(&url, &config)?;
    let job_id = uuid::Uuid::new_v4().to_string();

    let job = CrawlJob {
        id: job_id.clone(),
        url: url.clone(),
        status: JobStatus::Queued,
        config: config.clone(),
        results: Vec::new(),
        progress: CrawlProgress {
            pages_crawled: 0,
            page_limit: config.page_limit as usize,
            current_url: String::new(),
            depth: 0,
            max_depth: config.max_depth as u32,
            start_time: None,
        },
        error: None,
        start_time: None,
        end_time: None,
    };

    let job_arc = Arc::new(RwLock::new(job));
    let should_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let event_bus = EventBus::with_app(app.clone());

    let handle = CrawlHandle {
        job: job_arc.clone(),
        should_stop: should_stop.clone(),
        event_bus: event_bus.clone(),
    };

    let job_handle = JobHandle {
        job: job_arc.clone(),
        should_stop,
        event_bus,
    };

    {
        let mut jobs = state.active_jobs.write().await;
        jobs.insert(job_id.clone(), job_handle);
    }

    state.persist_job(&*job_arc.read().await).await.map_err(|e| e.to_string())?;

    let settings = get_settings(app).await.map_err(|e| e)?;
    Orchestrator::spawn(url, config, settings, handle, Some(state.inner().clone()));

    Ok(job_id)
}

#[tauri::command]
pub async fn stop_crawl(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        handle.should_stop.store(true, Ordering::Relaxed);
        let job = handle.job.read().await.clone();
        drop(jobs); // read lock freigeben
        state.persist_job(&job).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Job not found".into())
    }
}

#[tauri::command]
pub async fn get_job(job_id: String, state: State<'_, Arc<AppState>>) -> Result<CrawlJob, String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        return Ok(handle.job.read().await.clone());
    }
    drop(jobs);

    let persisted = state.persisted_jobs.read().await;
    if let Some(job) = persisted.get(&job_id) {
        return Ok(job.clone());
    }

    Err("Job not found".into())
}

#[tauri::command]
pub async fn list_jobs(state: State<'_, Arc<AppState>>) -> Result<Vec<CrawlJob>, String> {
    let mut result = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let active_jobs = state.active_jobs.read().await;
    for (_, handle) in active_jobs.iter() {
        let job = handle.job.read().await.clone();
        seen_ids.insert(job.id.clone());
        result.push(job);
    }
    drop(active_jobs);

    let persisted_jobs = state.persisted_jobs.read().await;
    for (_, job) in persisted_jobs.iter() {
        if !seen_ids.contains(&job.id) {
            result.push(job.clone());
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn delete_job(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    {
        let jobs = state.active_jobs.read().await;
        if let Some(handle) = jobs.get(&job_id) {
            handle.should_stop.store(true, Ordering::Relaxed);
        }
    }
    {
        let mut jobs = state.active_jobs.write().await;
        jobs.remove(&job_id);
    }
    state.remove_persisted_job(&job_id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    let settings = store
        .get("settings")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    Ok(settings)
}

#[tauri::command]
pub async fn update_settings(settings: AppSettings, app: AppHandle) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set(
        "settings",
        serde_json::to_value(&settings).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn open_output_folder(path: String) -> Result<(), String> {
    open::that(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_job(
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let job = {
        let jobs = state.active_jobs.read().await;
        if let Some(handle) = jobs.get(&job_id) {
            handle.job.read().await.clone()
        } else {
            let jobs = state.persisted_jobs.read().await;
            jobs.get(&job_id).cloned().ok_or("Job not found")?
        }
    };

    let output_dir = std::path::PathBuf::from(&job.config.output_dir);
    if !output_dir.exists() {
        return Err("Output directory not found".to_string());
    }

    let zip_path = output_dir.with_extension("zip");
    crate::export::zip_directory(&output_dir, &zip_path)
        .map_err(|e| e.to_string())?;

    Ok(zip_path.to_string_lossy().to_string())
}
