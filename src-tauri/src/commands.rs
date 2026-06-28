use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::RwLock;

use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus};
use crate::importer::ImportResult;
use crate::writer::fs::FsWriter;
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
    if config.ssrf_protection && crate::crawler::ssrf::is_private_target(url) {
        return Err(format!(
            "SSRF protection blocked the start URL '{}': resolves to a private/internal address. \
             Disable 'SSRF protection' in the crawl config to allow internal targets.",
            url
        ));
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
    for pattern in &config.exclude_patterns {
        if !pattern.is_empty() {
            regex::Regex::new(pattern)
                .map_err(|e| format!("Invalid exclude pattern '{}': {}", pattern, e))?;
        }
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
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub pages_saved: u32,
    pub total_size_bytes: u64,
    pub crawl_velocity: f32,
    pub fail_rate: f32,
}

const SIZE_SCAN_FILE_CAP: usize = 1000;
const SIZE_SCAN_TIME_CAP: Duration = Duration::from_secs(5);

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

async fn collect_all_jobs(state: &AppState) -> Vec<CrawlJob> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();

    {
        let active = state.active_jobs.read().await;
        for (_, handle) in active.iter() {
            let job = handle.job.read().await;
            seen.insert(job.id.clone());
            result.push(job.clone());
        }
    }

    {
        let persisted = state.persisted_jobs.read().await;
        for (_, job) in persisted.iter() {
            if !seen.contains(&job.id) {
                result.push(job.clone());
            }
        }
    }

    result
}

fn output_dir_for_job(job: &CrawlJob) -> Option<std::path::PathBuf> {
    let out = Path::new(&job.config.output_dir);
    if out.as_os_str().is_empty() {
        let settings = crate::settings::config::AppSettings::default();
        let default_out = PathBuf::from(&settings.output_dir);
        if default_out.exists() {
            return Some(default_out);
        }
    } else if out.exists() {
        return Some(out.to_path_buf());
    }
    None
}

async fn compute_dashboard_stats(state: &AppState) -> DashboardStats {
    let jobs = collect_all_jobs(state).await;
    let total = jobs.len();
    let failed = jobs.iter().filter(|j| j.status == JobStatus::Failed).count();

    let pages_saved: u32 = jobs.iter().map(|j| j.results.len() as u32).sum();

    let mut total_size_bytes: u64 = 0;
    let mut latest_completed: Option<&CrawlJob> = None;
    let mut latest_running: Option<&CrawlJob> = None;

    for job in &jobs {
        // Include output size for ALL jobs (active + completed)
        if let Some(out) = output_dir_for_job(job) {
            total_size_bytes = total_size_bytes.saturating_add(dir_size_capped(&out));
        }

        // Track latest completed job for velocity fallback
        if job.status == JobStatus::Completed {
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

        // Track latest running job for live velocity
        if job.status == JobStatus::Running {
            let is_newer = match &latest_running {
                Some(prev) => match (&prev.start_time, &job.start_time) {
                    (Some(p), Some(j)) => j.as_str() > p.as_str(),
                    _ => false,
                },
                None => true,
            };
            if is_newer {
                latest_running = Some(job);
            }
        }
    }

    // Prefer active job for velocity, fall back to latest completed
    let crawl_velocity = compute_velocity(latest_running.or(latest_completed));

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

fn compute_velocity(job: Option<&CrawlJob>) -> f32 {
    let Some(j) = job else { return 0.0 };
    let pages = j.results.len() as f32;
    if pages <= 0.0 {
        return 0.0;
    }

    let start = match &j.start_time {
        Some(s) => match parse_rfc3339(s) {
            Some(dt) => dt,
            None => return 0.0,
        },
        None => return 0.0,
    };

    let elapsed = match (&j.status, &j.end_time) {
        (JobStatus::Running, _) => {
            let now = chrono::Utc::now();
            (now - start).num_seconds().max(1) as f32
        }
        (_, Some(e)) => {
            match parse_rfc3339(e) {
                Some(end) => (end - start).num_seconds().max(1) as f32,
                None => return 0.0,
            }
        }
        _ => return 0.0,
    };

    pages / (elapsed / 60.0)
}

#[tauri::command]
pub async fn get_dashboard_stats(state: State<'_, Arc<AppState>>) -> Result<DashboardStats, String> {
    Ok(compute_dashboard_stats(state.inner().as_ref()).await)
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
    let main_dir = output_dir.join("main");
    if !main_dir.exists() {
        return Err("Output directory not found".to_string());
    }

    let zip_dir = output_dir.join("zip");
    std::fs::create_dir_all(&zip_dir).map_err(|e| e.to_string())?;
    let zip_path = zip_dir.join(format!("{}.zip", job_id));
    crate::export::zip_directory(&main_dir, &zip_path)
        .map_err(|e| e.to_string())?;

    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_job_v2(
    job_id: String,
    format: crate::export::ExportFormat,
    destination: Option<String>,
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
    let main_dir = output_dir.join("main");
    if !main_dir.exists() {
        return Err("Output directory not found for job".to_string());
    }

    let dest = match destination {
        Some(d) if !d.is_empty() => std::path::PathBuf::from(d),
        _ => output_dir.join("formats"),
    };
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    match format {
        crate::export::ExportFormat::MdFiles => {
            crate::export::copy_md_files(&main_dir, &dest)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::PdfFiles => {
            crate::export::export_pdf_files(&main_dir, &dest)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedMd => {
            let out_file = dest.join(format!("{}-merged.md", job_id));
            crate::export::merge_md_files(&main_dir, &out_file)
                .map_err(|e| format!("Export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedPdf => {
            let out_file = dest.join(format!("{}-merged.pdf", job_id));
            crate::export::export_merged_pdf(&main_dir, &out_file)
                .map_err(|e| format!("PDF export failed: {}", e))?;
        }
        crate::export::ExportFormat::JsonFiles => {
            crate::export::export_json_files(&main_dir, &dest)
                .map_err(|e| format!("JSON export failed: {}", e))?;
        }
        crate::export::ExportFormat::MergedJson => {
            let out_file = dest.join(format!("{}-merged.json", job_id));
            crate::export::merge_json_files(&main_dir, &out_file)
                .map_err(|e| format!("JSON export failed: {}", e))?;
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

fn char_safe_start(s: &str, byte_pos: usize) -> usize {
    let mut p = byte_pos.min(s.len());
    while p > 0 && !s.is_char_boundary(p) { p -= 1; }
    p
}

fn char_safe_end(s: &str, byte_pos: usize) -> usize {
    let mut p = byte_pos.min(s.len());
    while p < s.len() && !s.is_char_boundary(p) { p += 1; }
    p
}

fn extract_preview(content: &str, query: &str) -> String {
    let lower = content.to_lowercase();
    if let Some(pos) = lower.find(&query.to_lowercase()) {
        let start = char_safe_start(content, pos.saturating_sub(80));
        let end = char_safe_end(content, pos + query.len() + 120);
        let mut preview = content[start..end].to_string();
        if start > 0 { preview.insert_str(0, "…"); }
        if end < content.len() { preview.push('…'); }
        preview
    } else {
        content.chars().take(200).collect::<String>() + "…"
    }
}

#[tauri::command]
pub async fn read_page_content(
    job_id: String,
    url: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let job = get_job(job_id, state).await?;
    let main_dir = PathBuf::from(&job.config.output_dir).join("main");
    let writer = FsWriter::new(&main_dir);
    let path = writer.url_to_page_path(&url);
    tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Could not read page content: {}", e))
}

#[tauri::command]
pub async fn search_job_results(
    job_id: String,
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<SearchMatch>, String> {
    let job = get_job(job_id, state).await?;
    let q = query.to_lowercase();
    let main_dir = PathBuf::from(&job.config.output_dir).join("main");
    let writer = FsWriter::new(&main_dir);
    let mut matches = Vec::new();

    for page in &job.results {
        let title_lower = page.title.to_lowercase();
        let url_lower = page.url.to_lowercase();

        let title_score = title_lower.matches(&q).count() as u32;
        let url_score = url_lower.matches(&q).count() as u32;

        let path = writer.url_to_page_path(&page.url);
        let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let content_lower = content.to_lowercase();
        let content_score = content_lower.matches(&q).count() as u32;

        let relevance = title_score * 10 + content_score + url_score * 5;

        if relevance > 0 {
            let preview = extract_preview(&content, &q);
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

    let main_dir = output_dir.join("main");
    if !main_dir.exists() {
        return Err("Output directory does not contain main/ subfolder".into());
    }

    let zip_dir = output_dir.join("zip");
    std::fs::create_dir_all(&zip_dir).map_err(|e| e.to_string())?;
    let zip_path = zip_dir.join(format!("{}.zip", job_id));

    crate::export::zip_directory(&main_dir, &zip_path).map_err(|e| e.to_string())?;
    Ok(zip_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn list_exports(
    state: State<'_, Arc<AppState>>,
    limit: Option<usize>,
) -> Result<Vec<RecentExport>, String> {
    let n = limit.unwrap_or(5);

    let mut output_dirs: Vec<std::path::PathBuf> = Vec::new();

    {
        let active = state.active_jobs.read().await;
        for handle in active.values() {
            let job = handle.job.read().await;
            let dir = std::path::PathBuf::from(&job.config.output_dir);
            if !output_dirs.contains(&dir) {
                output_dirs.push(dir);
            }
        }
    }
    {
        let persisted = state.persisted_jobs.read().await;
        for job in persisted.values() {
            let dir = std::path::PathBuf::from(&job.config.output_dir);
            if !output_dirs.contains(&dir) {
                output_dirs.push(dir);
            }
        }
    }

    Ok(exports::list_recent_exports(&output_dirs, n))
}

#[tauri::command]
pub async fn import_file(
    file_path: String,
    output_dir: Option<String>,
    clean_text: Option<bool>,
    app: AppHandle,
) -> Result<ImportResult, String> {
    let path = std::path::PathBuf::from(&file_path);
    if !path.exists() {
        return Err("File not found".into());
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let dest = match output_dir {
        Some(d) if !d.is_empty() => std::path::PathBuf::from(d),
        _ => {
            let settings = get_settings(app).await?;
            let base = std::path::PathBuf::from(&settings.output_dir);
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("import");
            base.join(format!("import-{}", stem))
        }
    };
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let do_clean = clean_text.unwrap_or(true);

    match ext.as_str() {
        "pdf" => crate::importer::pdf::import_pdf(&path, &dest, do_clean)
            .map_err(|e| format!("PDF import failed: {}", e)),
        "epub" => crate::importer::epub::import_epub(&path, &dest, do_clean)
            .map_err(|e| format!("EPUB import failed: {}", e)),
        _ => Err(format!("Unsupported file type: .{}", ext)),
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowResizeResult {
    pub clamped: bool,
    pub applied_width: u32,
    pub applied_height: u32,
}

#[tauri::command]
pub async fn set_window_size(
    width: u32,
    height: u32,
    app: AppHandle,
) -> Result<WindowResizeResult, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    let (clamped_w, clamped_h, clamped) = match window.current_monitor() {
        Ok(Some(monitor)) => {
            let sf = monitor.scale_factor();
            let max_w = (monitor.size().width as f64 / sf) as u32;
            let max_h = (monitor.size().height as f64 / sf) as u32;
            let w = width.max(1280).min(max_w);
            let h = height.max(900).min(max_h);
            let c = w != width.max(1280) || h != height.max(900);
            (w, h, c)
        }
        _ => (width.max(1280), height.max(900), false),
    };

    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize::new(
            clamped_w as f64,
            clamped_h as f64,
        )))
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())?;

    Ok(WindowResizeResult {
        clamped,
        applied_width: clamped_w,
        applied_height: clamped_h,
    })
}
