//! End-to-end integration test for the Docurip crawler.
//!
//! **Disabled on Windows:** the `tauri` crate pulls in Windows-specific
//! runtime DLLs (WebView2 / MSVC) that cause `STATUS_ENTRYPOINT_NOT_FOUND`
//! (0xc0000139) when the standalone test executable is loaded. The test
//! compiles but is empty on `target_os = "windows"`. On Linux/macOS run
//! `cargo test --test e2e_crawl -- --ignored` to execute it.
//!
//! Setup: spins up a minimal local HTTP server via `tokio::net::TcpListener`,
//! serves 3 HTML pages + 1 PNG image, runs the full `Orchestrator::spawn` flow
//! against it, and asserts that the output files and asset rewriting are
//! correct.
//!
//! Deviations from the plan: PNG is embedded as a raw byte const (no fixture
//! file); expected output path is `{output_dir}/127.0.0.1/...` (host-prefixed
//! by `FsWriter`).

#![cfg(not(target_os = "windows"))]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Notify, RwLock};
use tokio::time::sleep;

use docurip::crawler::job::{CrawlJob, CrawlProgress, JobStatus};
use docurip::crawler::orchestrator::{CrawlHandle, Orchestrator};
use docurip::events::bus::EventBus;
use docurip::settings::config::{AppSettings, CrawlConfig};

const PNG_BYTES: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Index</title></head>
<body>
  <h1>Index Page</h1>
  <p>Welcome to the test site.</p>
  <a href="page1.html">Page 1</a>
  <img src="logo.png" alt="logo">
</body>
</html>
"#;

const PAGE1_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Page 1</title></head>
<body>
  <h1>Page 1</h1>
  <p>This is page 1.</p>
  <a href="page2.html">Page 2</a>
</body>
</html>
"#;

const PAGE2_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>Page 2</title></head>
<body>
  <h1>Page 2</h1>
  <p>This is page 2.</p>
</body>
</html>
"#;

fn make_config(output_dir: &str) -> CrawlConfig {
    CrawlConfig {
        output_dir: output_dir.to_string(),
        max_depth: 2,
        page_limit: 10,
        download_assets: true,
        headless_strategy: "never".to_string(),
        content_selectors: vec![],
        exclude_patterns: vec![],
        include_patterns: vec![],
        path_prefix: String::new(),
        respect_robots_txt: false,
        stay_within_domain: true,
        ssrf_protection: false,
        profile: None,
    }
}

fn make_settings() -> AppSettings {
    AppSettings {
        output_dir: String::from("./output"),
        concurrency: 2,
        request_delay: 0,
        timeout: 30000,
        user_agent: String::from("Docurip-Test/0.1"),
        default_max_depth: 2,
        default_page_limit: 50,
        default_download_assets: true,
        default_headless_strategy: String::from("never"),
        default_respect_robots_txt: false,
        default_stay_within_domain: true,
        default_ssrf_protection: false,
        window_width: 1280,
        window_height: 900,
        theme: String::from("dark"),
        notifications_enabled: true,
        shortcut_overrides: std::collections::HashMap::new(),
        auto_export_format: None,
    }
}

async fn spawn_static_site_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("local_addr");
    let uri = format!("http://{}", addr);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => n,
                    _ => return,
                };
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req
                    .lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .unwrap_or("/");

                let (status, content_type, body): (u16, &str, Vec<u8>) = match path {
                    "/" | "/index.html" => (200, "text/html; charset=utf-8", INDEX_HTML.as_bytes().to_vec()),
                    "/page1.html" => (200, "text/html; charset=utf-8", PAGE1_HTML.as_bytes().to_vec()),
                    "/page2.html" => (200, "text/html; charset=utf-8", PAGE2_HTML.as_bytes().to_vec()),
                    "/logo.png" => (200, "image/png", PNG_BYTES.to_vec()),
                    _ => (404, "text/plain", b"Not Found".to_vec()),
                };

                let header = format!(
                    "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status, content_type, body.len()
                );
                let _ = stream.write_all(header.as_bytes()).await;
                let _ = stream.write_all(&body).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    uri
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Windows tauri DLL load failure; run with --ignored on Linux/macOS"]
async fn end_to_end_crawl_writes_files_and_rewrites_assets() {
    let base_uri = spawn_static_site_server().await;
    let start_url = format!("{}/", base_uri);

    let temp = TempDir::new().expect("create temp dir");
    let output_dir = temp.path().to_string_lossy().to_string();

    let config = make_config(&output_dir);
    let settings = make_settings();

    let job = CrawlJob {
        id: "test-e2e".to_string(),
        url: start_url.clone(),
        status: JobStatus::Queued,
        config: config.clone(),
        results: vec![],
        progress: CrawlProgress {
            pages_crawled: 0,
            page_limit: config.page_limit as usize,
            current_url: String::new(),
            depth: 0,
            max_depth: config.max_depth,
            start_time: None,
        },
        error: None,
        start_time: None,
        end_time: None,
    };

    let handle = CrawlHandle {
        job: Arc::new(RwLock::new(job)),
        should_stop: Arc::new(AtomicBool::new(false)),
        should_pause: Arc::new(AtomicBool::new(false)),
        resume_notify: Arc::new(Notify::new()),
        event_bus: EventBus::new(),
    };

    Orchestrator::spawn(start_url, config, settings, handle.clone(), None);

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = handle.job.read().await.status.clone();
        match status {
            JobStatus::Completed | JobStatus::Failed => break,
            JobStatus::Paused => {
                let err = handle.job.read().await.error.clone();
                panic!("Crawl paused unexpectedly: {:?}", err);
            }
            _ => {}
        }
        if Instant::now() >= deadline {
            let status = handle.job.read().await.status.clone();
            let err = handle.job.read().await.error.clone();
            panic!(
                "Crawl did not complete within 30s. Status: {:?}, Error: {:?}",
                status, err
            );
        }
        sleep(Duration::from_millis(100)).await;
    }

    let final_status = handle.job.read().await.status.clone();
    let final_error = handle.job.read().await.error.clone();
    assert_eq!(
        final_status,
        JobStatus::Completed,
        "Crawl did not complete successfully. Error: {:?}",
        final_error
    );

    let host_dir = temp.path().join("127.0.0.1");
    assert!(
        host_dir.join("index.md").exists(),
        "index.md not found at {}",
        host_dir.join("index.md").display()
    );
    assert!(
        host_dir.join("page1.md").exists(),
        "page1.md not found"
    );
    assert!(
        host_dir.join("page2.md").exists(),
        "page2.md not found"
    );
    assert!(
        host_dir.join("logo.png").exists(),
        "logo.png not found at {}",
        host_dir.join("logo.png").display()
    );

    let index_md = std::fs::read_to_string(host_dir.join("index.md"))
        .expect("read index.md");
    assert!(
        index_md.contains("logo.png"),
        "Expected index.md to reference logo.png, got:\n{}",
        index_md
    );
    assert!(
        index_md.contains("127.0.0.1/logo.png"),
        "Expected rewritten path 127.0.0.1/logo.png in index.md, got:\n{}",
        index_md
    );
}
