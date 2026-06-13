use std::fs::File;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use zip::write::FileOptions;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus};
use crate::crawler::orchestrator::{CrawlHandle, Orchestrator};
use crate::events::bus::EventBus;
use crate::exports::{self, RecentExport};
use crate::settings::config::{AppSettings, CrawlConfig};
use crate::state::{AppState, JobHandle};
use crate::system::SystemStats;
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
    let should_pause = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let resume_notify = Arc::new(tokio::sync::Notify::new());
    let event_bus = EventBus::with_app(app.clone());

    let handle = CrawlHandle {
        job: job_arc.clone(),
        should_stop: should_stop.clone(),
        should_pause: should_pause.clone(),
        resume_notify: resume_notify.clone(),
        event_bus: event_bus.clone(),
    };

    let job_handle = JobHandle {
        job: job_arc.clone(),
        should_stop,
        should_pause,
        resume_notify,
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
pub async fn pause_crawl(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        handle.should_pause.store(true, Ordering::Relaxed);
        Ok(())
    } else {
        Err("Job not found".into())
    }
}

#[tauri::command]
pub async fn resume_crawl(job_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let jobs = state.active_jobs.read().await;
    if let Some(handle) = jobs.get(&job_id) {
        handle.should_pause.store(false, Ordering::Relaxed);
        handle.resume_notify.notify_one();
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

#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub pages_saved: u32,
    pub total_size_bytes: u64,
    pub crawl_velocity: f32,
    pub fail_rate: f32,
}

const STATS_CACHE_TTL: Duration = Duration::from_secs(30);
const SIZE_SCAN_FILE_CAP: usize = 1000;
const SIZE_SCAN_TIME_CAP: Duration = Duration::from_secs(5);

static STATS_CACHE: OnceLock<Mutex<Option<(Instant, DashboardStats)>>> = OnceLock::new();

fn stats_cache() -> &'static Mutex<Option<(Instant, DashboardStats)>> {
    STATS_CACHE.get_or_init(|| Mutex::new(None))
}

fn dir_size_capped(path: &Path) -> u64 {
    let start = Instant::now();
    let mut stack: Vec<std::path::PathBuf> = vec![path.to_path_buf()];
    let mut total: u64 = 0;
    let mut count: usize = 0;

    while let Some(p) = stack.pop() {
        if start.elapsed() > SIZE_SCAN_TIME_CAP {
            break;
        }
        let entries = match std::fs::read_dir(&p) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if count >= SIZE_SCAN_FILE_CAP {
                return total;
            }
            let ep = entry.path();
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_file() {
                total = total.saturating_add(meta.len());
                count += 1;
            } else if meta.is_dir() {
                stack.push(ep);
            }
        }
    }
    total
}

fn parse_rfc3339(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn collect_all_jobs(state: &AppState) -> Vec<CrawlJob> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Ok(active) = state.active_jobs.try_read() {
        for (_, handle) in active.iter() {
            if let Ok(job) = handle.job.try_read() {
                seen.insert(job.id.clone());
                result.push(job.clone());
            }
        }
    }

    if let Ok(persisted) = state.persisted_jobs.try_read() {
        for (_, job) in persisted.iter() {
            if !seen.contains(&job.id) {
                result.push(job.clone());
            }
        }
    }

    result
}

fn compute_dashboard_stats(state: &AppState) -> DashboardStats {
    let jobs = collect_all_jobs(state);
    let total = jobs.len();
    let failed = jobs.iter().filter(|j| j.status == JobStatus::Failed).count();

    let pages_saved: u32 = jobs.iter().map(|j| j.results.len() as u32).sum();

    let mut total_size_bytes: u64 = 0;
    let mut latest_completed: Option<&CrawlJob> = None;
    for job in &jobs {
        if job.status != JobStatus::Completed {
            continue;
        }
        let out = Path::new(&job.config.output_dir);
        if out.exists() {
            total_size_bytes = total_size_bytes.saturating_add(dir_size_capped(out));
        }
        let is_newer = match &latest_completed {
            Some(prev) => match (&prev.end_time, &job.end_time) {
                (Some(p), Some(j)) => j.as_str() > p.as_str(),
                _ => false,
            },
            None => true,
        };
        if is_newer {
            latest_completed = Some(job);
        }
    }

    let crawl_velocity: f32 = match latest_completed {
        Some(j) => {
            let pages = j.results.len() as f32;
            if pages <= 0.0 {
                0.0
            } else {
                let (start, end) = match (&j.start_time, &j.end_time) {
                    (Some(s), Some(e)) => (parse_rfc3339(s), parse_rfc3339(e)),
                    _ => (None, None),
                };
                match (start, end) {
                    (Some(s), Some(e)) => {
                        let secs = (e - s).num_seconds().max(1) as f32;
                        pages / (secs / 60.0)
                    }
                    _ => 0.0,
                }
            }
        }
        None => 0.0,
    };

    let fail_rate: f32 = if total == 0 {
        0.0
    } else {
        (failed as f32 / total as f32) * 100.0
    };

    DashboardStats {
        pages_saved,
        total_size_bytes,
        crawl_velocity,
        fail_rate,
    }
}

#[tauri::command]
pub async fn get_dashboard_stats(state: State<'_, Arc<AppState>>) -> Result<DashboardStats, String> {
    if let Some((ts, cached)) = stats_cache().lock().map(|g| g.clone()).unwrap_or(None) {
        if ts.elapsed() < STATS_CACHE_TTL {
            return Ok(cached);
        }
    }

    let stats = compute_dashboard_stats(state.inner().as_ref());
    if let Ok(mut g) = stats_cache().lock() {
        *g = Some((Instant::now(), stats.clone()));
    }
    Ok(stats)
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

#[tauri::command]
pub async fn export_job_v2(
    job_id: String,
    format: crate::export::ExportFormat,
    destination: String,
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
        return Err("Output directory not found for job".to_string());
    }

    let dest = std::path::PathBuf::from(&destination);

    match format {
        crate::export::ExportFormat::MdFiles => {
            crate::export::copy_md_files(&output_dir, &dest)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::PdfFiles => {
            crate::export::export_pdf_files(&output_dir, &dest)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedMd => {
            let out_file = dest.join(format!("{}-merged.md", job_id));
            crate::export::merge_md_files(&output_dir, &out_file)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedPdf => {
            let out_file = dest.join(format!("{}-merged.pdf", job_id));
            crate::export::export_merged_pdf(&output_dir, &out_file)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
    }

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_headless_support() -> bool {
    cfg!(feature = "headless")
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub url: String,
    pub title: String,
    pub preview: String,
    pub relevance: u32,
}

fn extract_preview(content: &str, query: &str) -> String {
    let lower = content.to_lowercase();
    if let Some(pos) = lower.find(&query.to_lowercase()) {
        let start = pos.saturating_sub(80);
        let end = (pos + query.len() + 120).min(content.len());
        let mut preview = content[start..end].to_string();
        if start > 0 { preview.insert_str(0, "…"); }
        if end < content.len() { preview.push('…'); }
        preview
    } else {
        content.chars().take(200).collect::<String>() + "…"
    }
}

#[tauri::command]
pub async fn search_job_results(
    job_id: String,
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<SearchMatch>, String> {
    let job = get_job(job_id, state).await?;
    let q = query.to_lowercase();
    let mut matches = Vec::new();

    for page in &job.results {
        let title_lower = page.title.to_lowercase();
        let content_lower = page.content.to_lowercase();
        let url_lower = page.url.to_lowercase();

        let title_score = title_lower.matches(&q).count() as u32;
        let content_score = content_lower.matches(&q).count() as u32;
        let url_score = url_lower.matches(&q).count() as u32;

        let relevance = title_score * 10 + content_score + url_score * 5;

        if relevance > 0 {
            let preview = extract_preview(&page.content, &q);
            matches.push(SearchMatch {
                url: page.url.clone(),
                title: page.title.clone(),
                preview,
                relevance,
            });
        }
    }

    matches.sort_by(|a, b| b.relevance.cmp(&a.relevance));
    Ok(matches)
}

#[tauri::command]
pub async fn export_job_zip(
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let job = get_job(job_id.clone(), state).await?;
    let output_dir = std::path::PathBuf::from(&job.config.output_dir);

    if !output_dir.exists() {
        return Err("Output directory does not exist".into());
    }

    let zip_path = output_dir.parent()
        .unwrap_or(&output_dir)
        .join(format!("{}-export.zip", job_id));

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<File>,
        base: &std::path::Path,
        current: &std::path::Path,
        options: FileOptions<()>,
    ) -> Result<(), String> {
        for entry in std::fs::read_dir(current).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let relative = path.strip_prefix(base).map_err(|e| e.to_string())?;

            if path.is_file() {
                let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
                zip.start_file_from_path(relative, options.clone())
                    .map_err(|e| e.to_string())?;
                std::io::copy(&mut file, zip).map_err(|e| e.to_string())?;
            } else if path.is_dir() {
                zip.add_directory_from_path(relative, options.clone())
                    .map_err(|e| e.to_string())?;
                add_dir_to_zip(zip, base, &path, options.clone())?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip, &output_dir, &output_dir, options)
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn list_exports(
    app: AppHandle,
    limit: Option<usize>,
) -> Result<Vec<RecentExport>, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(5);
    Ok(exports::list_recent_exports(&dir, n))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub uptime_secs: u64,
}

#[tauri::command]
pub async fn get_system_stats() -> Result<SystemStats, String> {
    Ok(crate::system::collect())
}

#[tauri::command]
pub async fn get_session_info(
    state: State<'_, Arc<AppState>>,
) -> Result<SessionInfo, String> {
    let s = state.inner().clone();
    Ok(SessionInfo {
        id: s.session_id.clone(),
        uptime_secs: s.uptime_secs(),
    })
}
