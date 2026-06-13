use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock, Semaphore};
use tokio::time::{sleep, Duration};
use url::Url;
use regex::RegexSet;

use crate::asset_dl::downloader::AssetDownloader;
use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus, PageResult};
use crate::crawler::is_valid_url;
use crate::events::bus::{CrawlEvent, EventBus};
use crate::fetcher::headless::HeadlessFetcher;
use crate::fetcher::http::HttpFetcher;
use crate::parser::dom::DomParser;
use crate::converter::html_to_md::HtmlToMarkdown;
use crate::settings::config::{AppSettings, CrawlConfig};
use crate::writer::fs::FsWriter;

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
    ) -> anyhow::Result<Self> {
        if !is_valid_url(start_url) {
            anyhow::bail!("Invalid or unsupported URL scheme: {}", start_url);
        }
        let base_url = Url::parse(start_url)?;
        let fetcher = HttpFetcher::new();
        let parser = DomParser::new();
        let converter = HtmlToMarkdown::new();
        let writer = FsWriter::new(&config.output_dir);
        let exclude_set = if config.exclude_patterns.is_empty() {
            None
        } else {
            RegexSet::new(&config.exclude_patterns).ok()
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
        config,
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

            match Self::new(&start_url, config, settings, handle.clone(), headless) {
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
                    });
                }
            }
        });
    }

    async fn run(&mut self, start_url: &str) -> anyhow::Result<()> {
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
            let job_id = job.id.clone();
            drop(job);
            self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                job_id,
                status: JobStatus::Running,
            });
        }

        let mut processed = 0usize;

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
                let _ = self.handle_task_result(result, &url, depth, max_depth, &mut processed, page_limit, &mut queue, &mut visited).await;
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
                        let _ = self.handle_task_result(result, &url, depth, max_depth, &mut processed, page_limit, &mut queue, &mut visited).await;
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
            html.clone()
        };

        let mut asset_map = HashMap::new();
        if self.config.download_assets {
            let asset_downloader =
                AssetDownloader::new(self.fetcher.clone(), self.writer.clone());
            for asset_url in &assets {
                match asset_downloader.download(asset_url).await {
                    Ok(rel_path) => {
                        asset_map.insert(asset_url.clone(), rel_path);
                    }
                    Err(e) => {
                        let err_msg = format!("Asset download failed for {}: {}", asset_url, e);
                        let job_id = self.handle.job.read().await.id.clone();
                        self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id: job_id.clone(),
                            level: "WARN".into(),
                            message: err_msg.clone(),
                        });
                        if is_disk_error(&err_msg) {
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
                            });
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
            let err_msg = format!("Write error for {}: {}", url, e);
            let job_id = self.handle.job.read().await.id.clone();
            self.handle.event_bus.emit(CrawlEvent::Error {
                job_id: job_id.clone(),
                message: err_msg.clone(),
            });
            if is_disk_error(&err_msg) {
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
                });
                return;
            }
        }

        let page_result = PageResult {
            url: url.clone(),
            title: title.clone(),
            content: markdown,
            links: links.clone(),
            assets: assets.clone(),
            status: status_code,
        };

        {
            let mut job = self.handle.job.write().await;
            job.results.push(page_result.clone());
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
                page: page_result,
            });
            self.handle.event_bus.emit(CrawlEvent::Progress {
                job_id,
                progress,
            });
        }

        if let Some(ref app_state) = self.app_state {
            let job = self.handle.job.read().await.clone();
            let _ = app_state.persist_job(&job).await;
        }

        if depth < max_depth {
            for link in links {
                if !visited.contains(&link) && is_valid_url(&link) {
                    if let Some(ref set) = self.exclude_set {
                        if set.is_match(&link) {
                            continue;
                        }
                    }
                    visited.insert(link.clone());
                    queue.push_back((link, depth + 1));
                }
            }
        }
    }
}

/// Returns true if the given error message indicates a disk-related write
/// failure (permission denied, no space, read-only filesystem, or the
/// matching OS error codes for those conditions).
pub fn is_disk_error(msg: &str) -> bool {
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
    fn disk_error_matches_permission_denied() {
        assert!(is_disk_error("Permission denied (os error 5)"));
    }

    #[test]
    fn disk_error_matches_no_space_left() {
        assert!(is_disk_error("No space left on device (os error 28)"));
    }

    #[test]
    fn disk_error_matches_read_only_filesystem() {
        assert!(is_disk_error("Read-only file system (os error 30)"));
    }

    #[test]
    fn disk_error_matches_case_insensitively() {
        assert!(is_disk_error("PERMISSION DENIED"));
        assert!(is_disk_error("read-ONLY"));
    }

    #[test]
    fn disk_error_matches_bare_os_codes() {
        assert!(is_disk_error("os error 5"));
        assert!(is_disk_error("os error 28"));
        assert!(is_disk_error("os error 30"));
    }

    #[test]
    fn disk_error_does_not_match_network_error() {
        assert!(!is_disk_error("connection refused"));
        assert!(!is_disk_error("timeout"));
        assert!(!is_disk_error("http 500"));
        assert!(!is_disk_error("DNS resolution failed"));
    }

    #[test]
    fn disk_error_does_not_match_empty() {
        assert!(!is_disk_error(""));
    }
}
