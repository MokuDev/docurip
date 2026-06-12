use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::crawler::job::{CrawlJob, CrawlProgress, JobStatus, PageResult};
use crate::settings::config::CrawlConfig;
use crate::events::bus::{CrawlEvent, EventBus};
use crate::fetcher::http::HttpFetcher;
use crate::parser::dom::DomParser;
use crate::converter::html_to_md::HtmlToMarkdown;
use crate::writer::fs::FsWriter;

pub struct CrawlHandle {
    pub job: Arc<RwLock<CrawlJob>>,
    pub should_stop: Arc<AtomicBool>,
    pub event_bus: EventBus,
}

pub fn spawn_crawl(url: String, config: CrawlConfig, handle: CrawlHandle) {
    tokio::spawn(async move {
        if let Err(e) = run_crawl(&url, &config, &handle).await {
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
    });
}

async fn run_crawl(start_url: &str, config: &CrawlConfig, handle: &CrawlHandle) -> anyhow::Result<()> {
    let base_url = Url::parse(start_url)?;

    let fetcher = HttpFetcher::new();
    let parser = DomParser::new();
    let converter = HtmlToMarkdown::new();
    let writer = FsWriter::new(&config.output_dir);

    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();

    queue.push_back((start_url.to_string(), 0));
    visited.insert(start_url.to_string());

    let max_depth = config.max_depth as u32;
    let page_limit = config.page_limit as usize;

    {
        let mut job = handle.job.write().await;
        job.status = JobStatus::Running;
        job.start_time = Some(chrono::Utc::now().to_rfc3339());
        let job_id = job.id.clone();
        drop(job);
        handle.event_bus.emit(CrawlEvent::JobStatusChanged {
            job_id,
            status: JobStatus::Running,
        });
    }

    let mut processed = 0usize;

    while let Some((url, depth)) = queue.pop_front() {
        if handle.should_stop.load(Ordering::Relaxed) {
            let mut job = handle.job.write().await;
            job.status = JobStatus::Paused;
            let job_id = job.id.clone();
            drop(job);
            handle.event_bus.emit(CrawlEvent::JobStatusChanged {
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

        let (status_code, html) = match fetcher.fetch_with_status(&url).await {
            Ok(result) => result,
            Err(e) => {
                let err_msg = format!("Fetch error for {}: {}", url, e);
                if processed == 0 {
                    let mut job = handle.job.write().await;
                    job.error = Some(err_msg.clone());
                }
                handle.event_bus.emit(CrawlEvent::Error {
                    job_id: handle.job.read().await.id.clone(),
                    message: err_msg,
                });
                continue;
            }
        };

        let title = parser.extract_title(&html).unwrap_or_default();
        let links = parser.extract_links(&html, &base_url);
        let assets = parser.extract_assets(&html, &base_url);
        let markdown = converter.convert(&html);

        if let Err(e) = writer.write_page(&url, &markdown).await {
            handle.event_bus.emit(CrawlEvent::Error {
                job_id: handle.job.read().await.id.clone(),
                message: format!("Write error for {}: {}", url, e),
            });
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
            let mut job = handle.job.write().await;
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
            handle.event_bus.emit(CrawlEvent::PageComplete {
                job_id: job_id.clone(),
                page: page_result,
            });
            handle.event_bus.emit(CrawlEvent::Progress {
                job_id,
                progress,
            });
        }

        if depth < max_depth {
            for link in links {
                if !visited.contains(&link) {
                    visited.insert(link.clone());
                    queue.push_back((link, depth + 1));
                }
            }
        }
    }

    {
        let mut job = handle.job.write().await;
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
        handle.event_bus.emit(CrawlEvent::JobStatusChanged {
            job_id: job_id.clone(),
            status,
        });
        handle.event_bus.emit(CrawlEvent::Progress {
            job_id: job_id.clone(),
            progress,
        });
    }

    Ok(())
}
