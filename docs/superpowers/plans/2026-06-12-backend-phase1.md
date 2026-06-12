# Docurip Backend Phase 1 Implementation Plan

> **Goal:** Implement the core Rust crawling pipeline: fetch → parse → convert → write, orchestrated via Tauri commands with a live event stream.

**Architecture:** Approach A (all-in-one Rust). Single-threaded async crawler per job using `tokio`, `reqwest`, `html2md`, and `headless_chrome`. State managed via `Arc<RwLock<AppState>>`. Events broadcast to frontend via Tauri's `Listener` / `emit`.

**Tech Stack:** Tauri v2, Rust 1.95, `tokio`, `reqwest`, `scraper`, `html2md`, `headless_chrome`, `tauri-store`.

### Task 1: Implement `settings/config`
**Files:** `src-tauri/src/settings/config.rs`, `src-tauri/Cargo.toml`
- [ ] Define `Settings` struct (download_assets, headless, output_path).
- [ ] Integrate `tauri-plugin-store` to persist settings.
- [ ] Add commands `get_settings` and `update_settings`.

### Task 2: Implement `events/bus`
**Files:** `src-tauri/src/events/bus.rs`
- [ ] Define `CrawlEvent` enum (Progress, Log, PageComplete, Error).
- [ ] Implement `EventBus` using `tokio::sync::broadcast`.
- [ ] Ensure thread-safe access from the orchestrator.

### Task 3: Implement `crawler/job`
**Files:** `src-tauri/src/crawler/job.rs`
- [ ] Define `CrawlJob` struct with `id`, `status`, `config`, `results`, `start_time`.
- [ ] Define `CrawlConfig` struct (URL, max_depth, page_limit, selectors, output_strategy).
- [ ] Define `PageResult` struct (url, status, title, markdown_content, links, assets).

### Task 4: Implement `fetcher/http`
**Files:** `src-tauri/src/fetcher/http.rs`
- [ ] Create `HttpFetcher` struct wrapping `reqwest::Client`.
- [ ] Implement `fetch(url) -> Result<String, FetchError>`.
- [ ] Add user-agent header and basic timeout/retry config.

### Task 5: Implement `fetcher/headless`
**Files:** `src-tauri/src/fetcher/headless.rs`
- [ ] Create `HeadlessFetcher` struct wrapping `headless_chrome::Browser`.
- [ ] Implement `fetch_dynamic(url) -> Result<String, FetchError>`.
- [ ] Handle browser lifecycle (launch/close).

### Task 6: Implement `parser/dom`
**Files:** `src-tauri/src/parser/dom.rs`
- [ ] Create `HtmlParser` struct.
- [ ] Implement `extract_content(html, selector) -> Vec<String>`.
- [ ] Implement `extract_links(html) -> Vec<String>`.
- [ ] Implement `extract_title(html) -> Option<String>`.

### Task 7: Implement `converter/html_to_md`
**Files:** `src-tauri/src/converter/html_to_md.rs`
- [ ] Create `HtmlToMarkdown` struct.
- [ ] Implement `convert(html) -> String` using `html2md` crate.
- [ ] Inject custom handlers for specific HTML tags if needed.

### Task 8: Implement `writer/fs`
**Files:** `src-tauri/src/writer/fs.rs`
- [ ] Create `FileWriter` struct.
- [ ] Implement `write_markdown(path, content) -> Result<(), WriteError>`.
- [ ] Implement `write_assets(path, assets) -> Result<(), WriteError>`.
- [ ] Ensure directory creation logic.

### Task 9: Implement `crawler/orchestrator`
**Files:** `src-tauri/src/crawler/orchestrator.rs`
- [ ] Create `Orchestrator` struct holding `Job`, `EventBus`, `Fetchers`.
- [ ] Implement `start_job() -> Result<JobId, CrawlError>`.
- [ ] Implement the main crawl loop: dequeue URL, fetch, parse, convert, write.
- [ ] Add progress tracking and throttling.

### Task 10: Implement `state` and `commands`
**Files:** `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/main.rs`
- [ ] Define `AppState` with `RwLock<HashMap<JobId, Arc<Job>>>`.
- [ ] Register Tauri commands: `start_crawl`, `get_job_status`, `get_job_logs`, `stop_crawl`.
- [ ] Update `main.rs` to initialize state, plugins, and commands.

### Task 11: Update `tauri.conf.json`
- [ ] Add permissions for filesystem, network, and store.
- [ ] Configure CSP for local assets and external API access.
