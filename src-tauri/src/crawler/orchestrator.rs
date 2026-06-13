use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
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
    pub event_bus: EventBus,
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
        })
    }

    pub fn spawn(
        start_url: String,
        config: CrawlConfig,
        settings: AppSettings,
        handle: CrawlHandle,
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
                    let result = orch.run(&start_url).await;
                    if let Some(mut h) = orch.headless_fetcher.take() {
                        let _ = tokio::task::spawn_blocking(move || {
                            h.close();
                        })
                        .await;
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

    async fn fetch_page(&self, url: &str) -> anyhow::Result<(u16, String)> {
        let job_id = self.handle.job.read().await.id.clone();
        match self.config.headless_strategy.as_str() {
            "always" => {
                if let Some(ref h) = self.headless_fetcher {
                    match h.fetch(url).await {
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
                        h.fetch(url).await.map(|h| (200, h))
                    } else {
                        Ok((status, html))
                    }
                }
                Err(_) if self.headless_fetcher.is_some() => {
                    self.headless_fetcher.as_ref().unwrap().fetch(url).await.map(|h| (200, h))
                }
                Err(e) => Err(e),
            },
            _ => self.fetcher.fetch_with_status(url).await,
        }
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

        while let Some((url, depth)) = queue.pop_front() {
            if self.handle.should_stop.load(Ordering::Relaxed) {
                let mut job = self.handle.job.write().await;
                job.status = JobStatus::Paused;
                let job_id = job.id.clone();
                drop(job);
                self.handle.event_bus.emit(CrawlEvent::JobStatusChanged {
                    job_id,
                    status: JobStatus::Paused,
                });
                return Ok(());
            }

            if processed >= page_limit {
                break;
            }

            if depth > max_depth {
                continue;
            }

            let job_id = self.handle.job.read().await.id.clone();
            let (status_code, html) = match self.fetch_page(&url).await {
                Ok(result) => result,
                Err(e) => {
                    let err_msg = format!("Fetch error for {}: {}", url, e);
                    if processed == 0 {
                        let mut job = self.handle.job.write().await;
                        job.error = Some(err_msg.clone());
                    }
                    self.handle.event_bus.emit(CrawlEvent::Error {
                        job_id,
                        message: err_msg,
                    });
                    continue;
                }
            };

            let title = self.parser.extract_title(&html).unwrap_or_default();
            let links = self.parser.extract_links(&html, &self.base_url);
            let assets = self.parser.extract_assets(&html, &self.base_url);

            let html_for_md = if !self.config.content_selectors.is_empty() {
                self.parser
                    .extract_content(&html, &self.config.content_selectors)
                    .unwrap_or(html.clone())
            } else {
                html.clone()
            };
            let markdown = self.converter.convert(&html_for_md);

            if let Err(e) = self.writer.write_page(&url, &markdown).await {
                self.handle.event_bus.emit(CrawlEvent::Error {
                    job_id: self.handle.job.read().await.id.clone(),
                    message: format!("Write error for {}: {}", url, e),
                });
            }

            if self.config.download_assets {
                let asset_downloader =
                    AssetDownloader::new(self.fetcher.clone(), self.writer.clone());
                for asset_url in &assets {
                    if let Err(e) = asset_downloader.download(asset_url).await {
                        self.handle.event_bus.emit(CrawlEvent::Log {
                            job_id: self.handle.job.read().await.id.clone(),
                            level: "WARN".into(),
                            message: format!("Asset download failed for {}: {}", asset_url, e),
                        });
                    }
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
                processed += 1;
                let start_time = job.start_time.clone();
                let progress = CrawlProgress {
                    pages_crawled: processed,
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

            if depth < max_depth {
                for link in links {
                    if !visited.contains(&link) {
                        if !is_valid_url(&link) {
                            continue;
                        }
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

            if self.settings.request_delay > 0 {
                sleep(Duration::from_millis(self.settings.request_delay as u64)).await;
            }
        }

        {
            let mut job = self.handle.job.write().await;
            if job.error.is_some() {
                job.status = JobStatus::Failed;
            } else {
                job.status = JobStatus::Completed;
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
        }

        Ok(())
    }
}
