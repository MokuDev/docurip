use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

use crate::crawler::job::{CrawlJob, JobStatus};
use crate::crawler::orchestrator::{spawn_crawl, CrawlHandle};
use crate::events::bus::EventBus;
use crate::settings::config::{AppSettings, CrawlConfig};
use crate::state::{AppState, JobHandle};

#[tauri::command]
pub async fn start_crawl(
    url: String,
    config: CrawlConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();

    let job = CrawlJob {
        id: job_id.clone(),
        url: url.clone(),
        status: JobStatus::Queued,
        config: config.clone(),
        results: Vec::new(),
        progress: 0.0,
        start_time: None,
        end_time: None,
    };

    let job_arc = Arc::new(RwLock::new(job));
    let should_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let event_bus = EventBus::with_app(app);

    let handle = CrawlHandle {
        job: job_arc.clone(),
        should_stop: should_stop.clone(),
        event_bus: event_bus.clone(),
    };

    let job_handle = JobHandle {
        job: job_arc,
        should_stop,
        event_bus,
    };

    {
        let mut jobs = state.active_jobs.write().await;
        jobs.insert(job_id.clone(), job_handle);
    }

    spawn_crawl(url, config, handle);

    Ok(job_id)
}

#[tauri::command]
pub async fn stop_crawl(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        handle.should_stop.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        Err("Job not found".into())
    }
}

#[tauri::command]
pub async fn get_job(job_id: String, state: State<'_, AppState>) -> Result<CrawlJob, String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        Ok(handle.job.read().await.clone())
    } else {
        Err("Job not found".into())
    }
}

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>) -> Result<Vec<CrawlJob>, String> {
    let jobs = state.active_jobs.read().await;
    let mut result = Vec::new();
    for (_, handle) in jobs.iter() {
        result.push(handle.job.read().await.clone());
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let path = config_dir.join("settings.json");
    if let Ok(data) = tokio::fs::read_to_string(&path).await {
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&data) {
            return Ok(settings);
        }
    }
    Ok(AppSettings::default())
}

#[tauri::command]
pub async fn update_settings(settings: AppSettings, app: AppHandle) -> Result<(), String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&config_dir).await.map_err(|e| e.to_string())?;
    let path = config_dir.join("settings.json");
    let data = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    tokio::fs::write(&path, data).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn open_output_folder(path: String) -> Result<(), String> {
    open::that(path).map_err(|e| e.to_string())
}
