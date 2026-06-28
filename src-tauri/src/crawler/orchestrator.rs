use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock, Semaphore};
use std::time::Instant;
use tokio::time::{sleep, Duration};
use url::Url;
use regex::RegexSet;

use crate::asset_dl::downloader::AssetDownloader;
use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus, PageMeta};
use crate::events::bus::ErrorKind;
use crate::crawler::is_valid_url;
use crate::crawler::robots::{self, RobotsTxt};
use crate::crawler::ssrf;
use crate::events::bus::{CrawlEvent, EventBus};
use crate::fetcher::headless::HeadlessFetcher;
use crate::fetcher::http::HttpFetcher;
use crate::parser::dom::DomParser;
use crate::converter::html_to_md::HtmlToMarkdown;
use crate::settings::config::{AppSettings, CrawlConfig};
use crate::writer::fs::FsWriter;

pub fn resolve_output_dir(base_dir: &str, url: &str) -> String {
    let domain = Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    format!("{}/{}", base_dir, domain)
}

#[derive(Clone)]
pub struct CrawlHandle {
    pub job: Arc<RwLock<CrawlJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub should_pause: Arc<AtomicBool>,
    pub resume_notify: Arc<Notify>,
    pub event_bus: EventBus,
}

#[derive(Clone)]
struct FetchContext {
    handle: CrawlHandle,
    fetcher: HttpFetcher,
    headless_fetcher: Option<Arc<tokio::sync::Mutex<HeadlessFetcher>>>,
    headless_strategy: String,
    settings: AppSettings,
}

impl FetchContext {
    async fn fetch_page(&self, url: &str) -> anyhow::Result<(u16, String)> {
        let job_id = self.handle.job.read().await.id.clone();
        match self.headless_strategy.as_str() {
            "always" => {
                if let Some(ref h) = self.headless_fetcher {
                    match h.lock().await.fetch(url).await {
                        Ok(html) => return Ok((200, html)),
                        Err(e) => {
                            let _ = self.handle.event_bus.emit(CrawlEvent::Log {
                                job_id,
                                level: "WARN".into(),
                                message: format!("Headless fetch failed for {}: {}", url, e),
                            });
                        }
                    }
                }
                self.fetcher.fetch_with_status(url).await
            }
            "auto" => match self.fetcher.fetch_with_status(url).await {
                Ok((status, html)) => {
                    if status >= 200 && status < 300 && !html.trim().is_empty() {
                        Ok((status, html))
                    } else if let Some(ref h) = self.headless_fetcher {
                        h.lock().await.fetch(url).await.map(|h| (200, h))
                    } else {
                        Ok((status, html))
                    }
                }
                Err(_) if self.headless_fetcher.is_some() => {
                    self.headless_fetcher.as_ref().unwrap().lock().await.fetch(url).await.map(|h| (200, h))
                }
                Err(e) => Err(e),
            },
            _ => self.fetcher.fetch_with_status(url).await,
        }
    }
}

pub struct Orchestrator {
    handle: CrawlHandle,
    base_url: Url,
    fetcher: HttpFetcher,
    headless_fetcher: Option<HeadlessFetcher>,
    parser: DomParser,
    converter: HtmlToMarkdown,
    writer: FsWriter,
    exclude_set: Option<RegexSet>,
    robots: RobotsTxt,
    config: CrawlConfig,
    settings: AppSettings,
    app_state: Option<Arc<crate::state::AppState>>,
}

impl Orchestrator {
    pub fn new(
        start_url: &str,
        config: CrawlConfig,
        settings: AppSettings,
        handle: CrawlHandle,
        headless_fetcher: Option<HeadlessFetcher>,
        _job_id: &str,
    ) -> anyhow::Result<Self> {
        if !is_valid_url(start_url) {
            anyhow::bail!("Invalid or unsupported URL scheme: {}", start_url);
        }
        let base_url = Url::parse(start_url)?;
        let timeout_secs = ((settings.timeout / 1000).max(1)) as u64;
        let fetcher = HttpFetcher::new(timeout_secs);
        let parser = DomParser::new();
        let converter = HtmlToMarkdown::new();
        let base_output = if config.output_dir.is_empty() {
            resolve_output_dir(&settings.output_dir, start_url)
        } else {
            config.output_dir.clone()
        };
        let main_dir = format!("{}/main", base_output);
        let zip_dir = format!("{}/zip", base_output);
        let formats_dir = format!("{}/formats", base_output);
        for dir in [&main_dir, &zip_dir, &formats_dir] {
            if let Err(e) = std::fs::create_dir_all(dir) {
                anyhow::bail!("Failed to create output directory {}: {}", dir, e);
            }
        }
        let writer = FsWriter::new(&main_dir);
        let resolved_config = CrawlConfig { output_dir: base_output.clone(), ..config };
        let exclude_set = if resolved_config.exclude_patterns.is_empty() {
            None
        } else {
            let patterns: Vec<&str> = resolved_config.exclude_patterns
                .iter()
                .filter(|p| !p.is_empty())
                .map(|p| p.as_str())
                .collect();
            if patterns.is_empty() {
                None
            } else {
                Some(RegexSet::new(&patterns)
                    .map_err(|e| anyhow::anyhow!("Invalid exclude pattern: {}", e))?)
            }
        };

    Ok(Self {
        handle,
        base_url,
        fetcher,
        headless_fetcher,
        parser,
        converter,
        writer,
        exclude_set,
        robots: RobotsTxt::default(),
        config: resolved_config,
        settings,
        app_state: None,
    })
    }

    pub fn spawn(
        start_url: String,
        config: CrawlConfig,
        settings: AppSettings,
        handle: CrawlHandle,
        app_state: Option<Arc<crate::state::AppState>>,
    ) {
        tokio::spawn(async move {
            let job_id = handle.job.read().await.id.clone();
            let headless = if config.headless_strategy != "never" {
                match HeadlessFetcher::new() {
                    Ok(h) => Some(h),
                    Err(e) => {
                        handle.event_bus.emit(CrawlEvent::Log {
                            job_id: job_id.clone(),
                            level: "WARN".into(),
                            message: format!("Headless fetcher unavailable: {}", e),
                        });
                        None
                    }
                }
            } else {
                None
            };

            match Self::new(&start_url, config, settings, handle.clone(), headless, &job_id) {
                Ok(mut orch) => {
                    orch.app_state = app_state;
                    let result = orch.run(&start_url).await;
                    if let Some(h) = orch.headless_fetcher.take() {
                        drop(h);
                    }
                    if let Err(e) = result {
                        let mut job = handle.job.write().await;
                        job.status = JobStatus::Failed;
                        job.error = Some(format!("{}", e));
                        job.end_time = Some(chrono::Utc::now().to_rfc3339());
                        let job_id = job.id.clone();
                        drop(job);
                        handle.event_bus.emit(CrawlEvent::Error {
                            job_id,
                            message: format!("{}", e),
                            kind: ErrorKind::Unknown,
                        });
                    }
                }
                Err(e) => {
                    let mut job = handle.job.write().await;
                    job.status = JobStatus::Failed;
                    job.error = Some(format!("{}", e));
                    job.end_time = Some(chrono::Utc::now().to_rfc3339());
                    let job_id = job.id.clone();
                    drop(job);
                    handle.event_bus.emit(CrawlEvent::Error {
                        job_id,
                        message: format!("{}", e),
                        kind: ErrorKind::Unknown,
                    });
                }
            }
        });
    }

    async fn run(&mut self, start_url: &str) -> anyhow::Result<()> {
        if self.config.respect_robots_txt {
            let job_id = self.handle.job.read().await.id.clone();
            self.handle.event_bus.emit(CrawlEvent::Log {
                job_id,
                level: "INFO".into(),
                message: "Fetching robots.txt...".into(),
            });
            self.robots = robots::fetch_robots_txt(&self.base_url, &self.settings.user_agent).await;
            if self.robots.was_fetched() {
                let job_id = self.handle.job.read().await.id.clone();
                self.handle.event_bus.emit(CrawlEvent::Log {
                    job_id,
                    level: "INFO".into(),
                    message: "robots.txt loaded".into(),
                });
            }
        }

        let mut queue: VecDeque<(String, u32)> = VecDeque::new();
        let mut visited: HashSet<String> = HashSet::new();

        queue.push_back((start_url.to_string(), 0));
        visited.insert(start_url.to_string());

        let max_depth = self.config.max_depth;
        let page_limit = self.config.page_limit as usize;

        {
            let mut job = self.handle.job.write().await;
            job.status = JobStatus::Running;
            job.start_time = Some(chrono::Utc::now().to_rfc3339());
            job.config.output_dir = self.config.output_dir.clone();
            let job_id = job.id.clone();
            drop(job);
            self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                job_id,
                status: JobStatus::Running,
            });
        }

        let mut processed = 0usize;
        let mut pages_since_persist: usize = 0;
        const PERSIST_EVERY_N: usize = 50;
        const PERSIST_INTERVAL: Duration = Duration::from_secs(10);
        let mut last_persist = Instant::now();

        let semaphore = Arc::new(Semaphore::new(self.settings.concurrency as usize));
        let mut pending = tokio::task::JoinSet::new();

        let fetch_ctx = FetchContext {
            handle: self.handle.clone(),
            fetcher: self.fetcher.clone(),
            headless_fetcher: self.headless_fetcher.take().map(|h| Arc::new(tokio::sync::Mutex::new(h))),
            headless_strategy: self.config.headless_strategy.clone(),
            settings: self.settings.clone(),
        };

        loop {
            while let Some(res) = pending.try_join_next() {
                let (result, url, depth) = match res {
                    Ok(Ok((url, depth, status_code, html))) => (Ok((status_code, html)), url, depth),
                    Ok(Err(e)) => (Err(e), String::new(), 0),
                    Err(join_err) => (Err(anyhow::anyhow!("Task panicked: {}", join_err)), String::new(), 0),
                };
                let advanced = self.handle_task_result(result, &url, depth, max_depth, &mut processed, page_limit, &mut queue, &mut visited).await.unwrap_or(false);
                if advanced {
                    pages_since_persist += 1;
                }
            }

            // Throttled persist: every 10s OR every 50 pages, whichever comes first.
            if last_persist.elapsed() >= PERSIST_INTERVAL || pages_since_persist >= PERSIST_EVERY_N {
                if let Some(ref app_state) = self.app_state {
                    let job = self.handle.job.read().await.clone();
                    let _ = app_state.persist_job(&job).await;
                }
                pages_since_persist = 0;
                last_persist = Instant::now();
            }

            if self.handle.should_pause.load(Ordering::Relaxed) {
                {
                    let mut job = self.handle.job.write().await;
                    if job.status != JobStatus::Paused {
                        job.status = JobStatus::Paused;
                        let job_id = job.id.clone();
                        drop(job);
                        self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                            job_id: job_id.clone(),
                            status: JobStatus::Paused,
                        });
                        self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id,
                            level: "INFO".into(),
                            message: "Crawl paused by user".into(),
                        });
                    }
                }
                if let Some(ref app_state) = self.app_state {
                    let job = self.handle.job.read().await.clone();
                    let _ = app_state.persist_job(&job).await;
                }
                pending.abort_all();
                while let Some(_) = pending.join_next().await {}
                self.handle.resume_notify.notified().await;
                {
                    let mut job = self.handle.job.write().await;
                    job.status = JobStatus::Running;
                    let job_id = job.id.clone();
                    drop(job);
                    self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                        job_id,
                        status: JobStatus::Running,
                    });
                }
            }

            if self.handle.should_stop.load(Ordering::Relaxed) {
                pending.abort_all();
                while let Some(_) = pending.join_next().await {}
                {
                    let mut job = self.handle.job.write().await;
                    job.status = JobStatus::Cancelled;
                    job.error = None;
                    let job_id = job.id.clone();
                    drop(job);
                    self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                        job_id: job_id.clone(),
                        status: JobStatus::Cancelled,
                    });
                    self.handle.event_bus.emit(CrawlEvent::Log {
                        job_id,
                        level: "INFO".into(),
                        message: "Crawl cancelled by user".into(),
                    });
                }
                if let Some(ref app_state) = self.app_state {
                    let job = self.handle.job.read().await.clone();
                    let _ = app_state.persist_job(&job).await;
                }
                break;
            }

            while pending.len() < self.settings.concurrency as usize {
                // Soft limit: we stop spawning new tasks once processed >= page_limit,
                // but in-flight tasks may complete, so final count can slightly exceed the limit.
                if processed >= page_limit {
                    break;
                }
                if let Some((url, depth)) = queue.pop_front() {
                    if depth > max_depth {
                        continue;
                    }
                    let sem = semaphore.clone();
                    let ctx = fetch_ctx.clone();
                    pending.spawn(async move {
                        if ctx.settings.request_delay > 0 {
                            sleep(Duration::from_millis(ctx.settings.request_delay as u64)).await;
                        }
                        let _permit = sem.acquire_owned().await.map_err(|e| anyhow::anyhow!("Semaphore error: {}", e))?;
                        match ctx.fetch_page(&url).await {
                            Ok((status, html)) => Ok((url, depth, status, html)),
                            Err(e) => Err(anyhow::anyhow!("Fetch error for {}: {}", url, e)),
                        }
                    });
                } else {
                    break;
                }
            }

            if queue.is_empty() && pending.is_empty() {
                break;
            }

            if !pending.is_empty() {
                match pending.join_next().await {
                    Some(res) => {
                        let (result, url, depth) = match res {
                            Ok(Ok((url, depth, status_code, html))) => (Ok((status_code, html)), url, depth),
                            Ok(Err(e)) => (Err(e), String::new(), 0),
                            Err(join_err) => (Err(anyhow::anyhow!("Task panicked: {}", join_err)), String::new(), 0),
                        };
                        let advanced = self.handle_task_result(result, &url, depth, max_depth, &mut processed, page_limit, &mut queue, &mut visited).await.unwrap_or(false);
                        if advanced {
                            pages_since_persist += 1;
                        }
                    }
                    None => {}
                }
            } else {
                break;
            }
        }

        if let Some(h_arc) = fetch_ctx.headless_fetcher {
            if let Ok(mutex) = Arc::try_unwrap(h_arc) {
                let h = mutex.into_inner();
                drop(h);
            }
        }

        {
            let mut job = self.handle.job.write().await;
            if job.status != JobStatus::Paused {
                if job.error.is_some() {
                    job.status = JobStatus::Failed;
                } else {
                    job.status = JobStatus::Completed;
                }
            }
            let start_time = job.start_time.clone();
            let progress = CrawlProgress {
                pages_crawled: processed,
                page_limit,
                current_url: String::new(),
                depth: max_depth,
                max_depth,
                start_time,
            };
            job.progress = progress.clone();
            job.end_time = Some(chrono::Utc::now().to_rfc3339());
            let job_id = job.id.clone();
            let status = job.status.clone();
            drop(job);
            self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                job_id: job_id.clone(),
                status,
            });
            self.handle.event_bus.emit(CrawlEvent::Progress {
                job_id: job_id.clone(),
                progress,
            });
            if let Some(ref app_state) = self.app_state {
                let job = self.handle.job.read().await.clone();
                let _ = app_state.persist_job(&job).await;
            }
        }

        Ok(())
    }

    async fn handle_task_result(
        &mut self,
        result: anyhow::Result<(u16, String)>,
        url: &str,
        depth: u32,
        max_depth: u32,
        processed: &mut usize,
        page_limit: usize,
        queue: &mut VecDeque<(String, u32)>,
        visited: &mut HashSet<String>,
    ) -> anyhow::Result<bool> {
        match result {
            Ok((status_code, html)) => {
                self.process_fetched_page(
                    url.to_string(),
                    depth,
                    status_code,
                    html,
                    processed,
                    page_limit,
                    max_depth,
                    visited,
                    queue,
                ).await;
                Ok(true)
            }
            Err(e) => {
                let job_id = self.handle.job.read().await.id.clone();
                let err_msg = format!("{}", e);
                if *processed == 0 {
                    let mut job = self.handle.job.write().await;
                    job.error = Some(err_msg.clone());
                }
                self.handle.event_bus.emit(CrawlEvent::Error {
                    job_id,
                    message: err_msg,
                    kind: ErrorKind::Network,
                });
                Ok(false)
            }
        }
    }

    async fn process_fetched_page(
        &mut self,
        url: String,
        depth: u32,
        status_code: u16,
        html: String,
        processed: &mut usize,
        page_limit: usize,
        max_depth: u32,
        visited: &mut HashSet<String>,
        queue: &mut VecDeque<(String, u32)>,
    ) {
        let title = self.parser.extract_title(&html).unwrap_or_default();
        let links = self.parser.extract_links(&html, &self.base_url);
        let assets = self.parser.extract_assets(&html, &self.base_url);

        let mut html_for_md = if !self.config.content_selectors.is_empty() {
            self.parser
                .extract_content(&html, &self.config.content_selectors)
                .unwrap_or(html.clone())
        } else {
            self.parser.auto_extract_content(&html).unwrap_or(html.clone())
        };

        let mut asset_map = HashMap::new();
        if self.config.download_assets {
            let asset_downloader =
                AssetDownloader::new(self.fetcher.clone(), self.writer.clone());
            let mut download_tasks = tokio::task::JoinSet::new();
            for asset_url in &assets {
                let dl = asset_downloader.clone();
                let url = asset_url.clone();
                download_tasks.spawn(async move { (url.clone(), dl.download(&url).await) });
            }
            while let Some(result) = download_tasks.join_next().await {
                let (asset_url, dl_result) = result.unwrap_or_else(|e| {
                    ("?".into(), Err(anyhow::anyhow!("Task join error: {}", e)))
                });
                match dl_result {
                    Ok(rel_path) => {
                        asset_map.insert(asset_url, rel_path);
                    }
                    Err(e) => {
                        let is_disk = is_disk_error(&e);
                        let err_msg = format!("Asset download failed for {}: {}", asset_url, e);
                        let job_id = self.handle.job.read().await.id.clone();
                        self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id: job_id.clone(),
                            level: "WARN".into(),
                            message: err_msg.clone(),
                        });
                        if is_disk {
                            {
                                let mut job = self.handle.job.write().await;
                                job.status = JobStatus::Paused;
                                job.error = Some(err_msg.clone());
                            }
                            self.handle.should_pause.store(true, Ordering::Relaxed);
                            if let Some(ref app_state) = self.app_state {
                                let job = self.handle.job.read().await.clone();
                                let _ = app_state.persist_job(&job).await;
                            }
                            self.handle.event_bus.emit(CrawlEvent::Error {
                                job_id,
                                message: format!(
                                    "Disk error: {}. Fix output path in Settings and resume.",
                                    err_msg
                                ),
                                kind: ErrorKind::Disk,
                            });
                            download_tasks.abort_all();
                            return;
                        }
                    }
                }
            }
        }

        if !asset_map.is_empty() {
            html_for_md = self.parser.rewrite_asset_urls(&html_for_md, &self.base_url, &asset_map);
        }

        let markdown = self.converter.convert(&html_for_md);

        if let Err(e) = self.writer.write_page(&url, &markdown).await {
            let is_disk = is_disk_error(&e);
            let err_msg = format!("Write error for {}: {}", url, e);
            let job_id = self.handle.job.read().await.id.clone();
            let kind = if is_disk { ErrorKind::Disk } else { ErrorKind::Unknown };
            self.handle.event_bus.emit(CrawlEvent::Error {
                job_id: job_id.clone(),
                message: err_msg.clone(),
                kind,
            });
            if is_disk {
                {
                    let mut job = self.handle.job.write().await;
                    job.status = JobStatus::Paused;
                    job.error = Some(err_msg.clone());
                }
                self.handle.should_pause.store(true, Ordering::Relaxed);
                if let Some(ref app_state) = self.app_state {
                    let job = self.handle.job.read().await.clone();
                    let _ = app_state.persist_job(&job).await;
                }
                self.handle.event_bus.emit(CrawlEvent::Error {
                    job_id,
                    message: format!(
                        "Disk error: {}. Fix output path in Settings and resume.",
                        err_msg
                    ),
                    kind: ErrorKind::Disk,
                });
                return;
            }
        }

        let page_meta = PageMeta {
            url: url.clone(),
            title: title.clone(),
            status: status_code,
            links_count: links.len(),
        };

        {
            let mut job = self.handle.job.write().await;
            job.results.push(page_meta.clone());
            *processed += 1;
            let start_time = job.start_time.clone();
            let progress = CrawlProgress {
                pages_crawled: *processed,
                page_limit,
                current_url: url.clone(),
                depth,
                max_depth,
                start_time,
            };
            job.progress = progress.clone();
            let job_id = job.id.clone();
            drop(job);
            self.handle.event_bus.emit(CrawlEvent::PageComplete {
                job_id: job_id.clone(),
                page: page_meta,
            });
            self.handle.event_bus.emit(CrawlEvent::Progress {
                job_id,
                progress,
            });
        }

        if depth < max_depth {
            let base_host = self.base_url.host_str();
            const MAX_QUEUE_SIZE: usize = 50_000;
            for link in links {
                if !visited.contains(&link) && is_valid_url(&link) {
                    if self.config.ssrf_protection && ssrf::is_private_target(&link) {
                        let job_id = self.handle.job.read().await.id.clone();
                        let _ = self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id,
                            level: "WARN".into(),
                            message: format!("SSRF blocked: {} resolves to a private/internal address", link),
                        });
                        continue;
                    }
                    if self.config.respect_robots_txt && !self.robots.is_allowed(&link) {
                        continue;
                    }
                    if self.config.stay_within_domain {
                        if let Ok(parsed) = Url::parse(&link) {
                            if parsed.host_str() != base_host {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if let Some(ref set) = self.exclude_set {
                        if set.is_match(&link) {
                            continue;
                        }
                    }
                    if queue.len() >= MAX_QUEUE_SIZE {
                        let job_id = self.handle.job.read().await.id.clone();
                        let _ = self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id,
                            level: "WARN".into(),
                            message: format!("Queue at capacity ({} URLs), skipping new links from {}", MAX_QUEUE_SIZE, url),
                        });
                        continue;
                    }
                    visited.insert(link.clone());
                    queue.push_back((link, depth + 1));
                }
            }
        }
    }
}

/// Returns true if the given error indicates a disk-related write failure
/// (permission denied, no space left, read-only filesystem).
///
/// Walks the anyhow error chain looking for a `std::io::Error` and classifies
/// by `ErrorKind`. Falls back to substring matching on the formatted error
/// only when no `io::Error` is present in the chain (e.g. for third-party
/// errors that stringify without a source chain).
pub fn is_disk_error(err: &anyhow::Error) -> bool {
    use std::io::ErrorKind;
    for cause in err.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            return matches!(
                io_err.kind(),
                ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem | ErrorKind::StorageFull
            );
        }
    }
    is_disk_error_str(&err.to_string())
}

/// Substring-based disk error detection. Used as a fallback when no
/// `std::io::Error` is available in the cause chain.
pub fn is_disk_error_str(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    [
        "permission denied",
        "no space",
        "read-only",
        "os error 28",
        "os error 5",
        "os error 30",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_error_str_matches_permission_denied() {
        assert!(is_disk_error_str("Permission denied (os error 5)"));
    }

    #[test]
    fn disk_error_str_matches_no_space_left() {
        assert!(is_disk_error_str("No space left on device (os error 28)"));
    }

    #[test]
    fn disk_error_str_matches_read_only_filesystem() {
        assert!(is_disk_error_str("Read-only file system (os error 30)"));
    }

    #[test]
    fn disk_error_str_matches_case_insensitively() {
        assert!(is_disk_error_str("PERMISSION DENIED"));
        assert!(is_disk_error_str("read-ONLY"));
    }

    #[test]
    fn disk_error_str_matches_bare_os_codes() {
        assert!(is_disk_error_str("os error 5"));
        assert!(is_disk_error_str("os error 28"));
        assert!(is_disk_error_str("os error 30"));
    }

    #[test]
    fn disk_error_str_does_not_match_network_error() {
        assert!(!is_disk_error_str("connection refused"));
        assert!(!is_disk_error_str("timeout"));
        assert!(!is_disk_error_str("http 500"));
        assert!(!is_disk_error_str("DNS resolution failed"));
    }

    #[test]
    fn disk_error_str_does_not_match_empty() {
        assert!(!is_disk_error_str(""));
    }

    #[test]
    fn disk_error_classifies_io_kinds_via_chain() {
        use std::io;
        let perm_err: anyhow::Error =
            io::Error::new(io::ErrorKind::PermissionDenied, "denied").into();
        assert!(is_disk_error(&perm_err));

        let full_err: anyhow::Error =
            io::Error::new(io::ErrorKind::StorageFull, "full").into();
        assert!(is_disk_error(&full_err));

        let ro_err: anyhow::Error =
            io::Error::new(io::ErrorKind::ReadOnlyFilesystem, "ro").into();
        assert!(is_disk_error(&ro_err));

        let other_io: anyhow::Error =
            io::Error::new(io::ErrorKind::ConnectionRefused, "no").into();
        assert!(!is_disk_error(&other_io));
    }

    #[test]
    fn disk_error_uses_string_fallback_when_no_io_error() {
        let plain = anyhow::anyhow!("permission denied while writing page");
        assert!(is_disk_error(&plain));

        let network = anyhow::anyhow!("connection refused");
        assert!(!is_disk_error(&network));
    }
}
