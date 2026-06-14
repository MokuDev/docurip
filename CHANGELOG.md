# Changelog

## v0.3.2 (2026-06-14)

### Added
- **Auto-organized output folders**: crawls now create `{domain}/{YYYY-MM-DD}-{job_id}/` subfolders under the global output directory by default, keeping crawl results organized without manual intervention
- **Native folder pickers**: NewCrawl and Settings views use native OS directory pickers instead of text inputs for output directory selection
- **ExportModal pre-fill**: export destination now auto-fills from the job's output directory, eliminating the manual folder picker step for most exports


## v0.3.1 (2026-06-14)

### Added
- Tests: 3 regression tests for URL-to-path query-string stripping in `writer/fs.rs` (verifies `?query` and `#fragment` are stripped from filenames)
- **Logs memory cap**: `NewCrawl.tsx` now caps log entries at 500, preventing unbounded memory growth during long crawls
- **Search debounce**: `ResultSearch.tsx` debounces input by 200ms to reduce re-rendering of ResultTree and MarkdownPreview during typing
- **Dashboard error logging**: empty `catch` blocks in `Dashboard.tsx` now emit `console.warn` with context for debugging

### Fixed
- **Startup crash**: `AppState::init()` called `Handle::current().block_on()` before Tauri starts its Tokio runtime, causing an immediate panic; reverted to synchronous `std::fs` for the one-time startup load
- **prefillUrl re-trigger**: removed the `if (prev.url) return prev` guard in `NewCrawl.tsx` so quick-start URLs are always applied, even after the user has manually edited the URL field


## v0.3.0 (2026-06-14)

### Added
- **Domain filtering (`stay_within_domain`)**: new config option (default `true`) that restricts the crawler to links within the same domain as the start URL; implemented in `orchestrator.rs` URL queue with checkbox in New Crawl and Settings views
- **robots.txt enforcement (`respect_robots_txt`)**: new `robots.rs` module that fetches and parses `robots.txt` from the target site, honoring `User-agent`, `Disallow`, `Allow`, and `Crawl-delay` directives; enforced as the first URL-level filter in the crawl loop
- **SSRF protection (`ssrf_protection`)**: new `ssrf.rs` module that detects and blocks requests to private/internal IP addresses (loopback, link-local, RFC 1918, IPv6 ULA) via IP-literal checks, known-local hostname patterns, and DNS resolution; configurable per-crawl with checkbox UI
- 10 unit tests for SSRF detection covering IPv4 private ranges, IPv6 loopback/ULA, localhost variants, `.local` TLD, and public-host passthrough

### Security
- **XSS prevention in MarkdownPreview**: added DOMPurify sanitization after Markdown-to-HTML rendering and after search-query highlighting; restricted allowed tags/attributes and blocked `javascript:` URIs in links
- **CSP hardened**: removed `'unsafe-inline'` from `script-src`; set `withGlobalTauri: false` so the Content Security Policy now actually blocks injected inline scripts

### Changed
- `useCrawlEvents`: migrated from `window.__TAURI__.event.listen` back to typed `listen()` from `@tauri-apps/api/event` with proper async cleanup and TypeScript-safe event payloads
- **Async disk I/O**: `save_job_to_disk`, `load_job_from_disk`, `load_all_jobs`, and `delete_job_from_disk` in `state.rs` now use `tokio::fs` instead of blocking `std::fs`, preventing the crawl runtime from stalling on disk operations; `AppState::init()` bridges the sync/async boundary via `Handle::current().block_on()`
- `persist_job` and `remove_persisted_job` now `.await` the underlying async disk operations
- **System stats caching**: `system.rs` now uses a `LazyLock<Mutex<System>>` singleton instead of creating a new `sysinfo::System::new_all()` on every 2-second poll, reducing allocation overhead and improving accuracy of CPU readings
- **Asset download size limit**: `fetch_bytes` now checks the `content-length` header before consuming the response body, rejecting assets larger than 50 MB to prevent memory exhaustion from adversarial or oversized downloads
- **Dashboard polling merged**: three separate `useEffect` intervals (jobs 3s, stats 3s, exports 5s) in `Dashboard.tsx` consolidated into a single 3s interval
- **Parallel asset downloads**: `orchestrator.rs` asset-download loop replaced with `tokio::task::JoinSet` so all assets per page download concurrently instead of sequentially
- **Shared StatusBadge**: extracted `StatusIcon` and `StatusBadge` into `src/components/StatusBadge.tsx`; `Dashboard.tsx`, `History.tsx`, and `NewCrawl.tsx` now import the shared component instead of duplicating local versions
- **LiveConsole event processing**: fixed event loss by switching from `events[events.length - 1]` to index-based tracking with `lastProcessedIdx` ref, ensuring all events between renders are processed
- **History loading flicker**: `loadJobs` now accepts a `showSpinner` parameter; background polls skip `setLoading(true)` so the spinner only appears on initial load

### Fixed
- **UTF-8 panic in search preview**: `extract_preview` now uses `char_safe_start`/`char_safe_end` helpers to find valid UTF-8 character boundaries before slicing, preventing panics on non-ASCII content (e.g. German umlauts, CJK characters)
- **Panic-safe CSS selector**: replaced `unwrap()` on hardcoded `a[href]` selector in `DomParser` with `expect()` carrying an explanatory message
- **ZIP export path error**: `export_job_zip` now propagates an explicit error when `output_dir` has no parent directory instead of silently falling back via `unwrap_or`
- **Silent polling failure in New Crawl**: `get_job` polling now tracks consecutive errors via a ref; after 3 consecutive failures the interval is cleared and the job status is set to `failed` so the UI reflects the broken state instead of freezing silently
- **Invalid exclude patterns ignored silently**: `validate_crawl_input` now validates each exclude pattern before the crawl starts and returns an actionable error if any pattern is malformed; `Orchestrator::new` propagates the error via `?` instead of discarding it with `.ok()`
- **`stay_within_domain` was defined but never enforced**: `CrawlConfig.stay_within_domain` existed in the struct since v0.2.x but had no filtering logic — links to external domains were silently followed; now correctly skipped when enabled
- **`respect_robots_txt` was defined but never enforced**: same issue — the config field existed without any parsing or checking; now actively fetched and consulted


## v0.2.4 

### Added
- Dashboard stats expanded to 4 metric cards: Pages Saved, Total Size, Crawl Velocity (pages/min), Fail Rate
- Recent Exports panel on Dashboard — lists exported ZIPs (name, size, date), click-to-reveal in file manager
- `list_exports` command + `exports.rs` helper scanning `app_data_dir/exports/`
- Top status bar: session ID (short hex) + live uptime counter
- Bottom system status bar: live CPU%, RAM used/total, active output path
- `get_system_stats` / `get_session_info` commands via new `system.rs` (sysinfo-based)
- Global toast container (bottom-left, max 3 visible, auto-dismiss after 6s except errors)
- Dependencies: `sysinfo = "0.31"`, `uuid` (v4 feature)

### Changed
- `useCrawlEvents` now pushes `error` events into the global toast system
- Main layout adjusted (`h-[calc(100vh-44px)]`) to accommodate top/bottom bars


## v0.2.3 

### Added
- Pause/Resume for active crawl jobs: `pause_crawl` / `resume_crawl` commands, `should_pause` flag + `Notify`-based resume signal on `CrawlHandle`
- Pause/Resume buttons in New Crawl view (orange Pause / green Resume / red Cancel)
- Disk-error auto-pause: write failures (permission denied, no space, read-only) now pause the job with an actionable hint to fix the output path and resume, instead of marking it failed
- E2E test (`tests/e2e_crawl.rs`) using a wiremock static-site fixture (index + 2 sub-pages + image), verifying full crawl writes Markdown + assets with rewritten relative paths
- Dependency: `wiremock = "0.6"` (dev)

### Changed
- Orchestrator main loop now checks `should_pause` before `should_stop`, persists job state as `Paused`, aborts in-flight tasks gracefully, and resumes cleanly back to `Running`


## v0.2.2 

### Changed
- AppSettings default output dir to `~/.docurip`; `Orchestrator::new` auto-creates output directory via `std::fs::create_dir_all`
- Dashboard stats fallback: when `job.output_dir` is empty, use AppSettings default output dir
- Frontend event listener migrated from `@tauri-apps/api/event` `listen` to `window.__TAURI__.event.listen` for reliable synchronous cleanup
- EventBus `emit` now calls synchronous `app.emit` for reliable event delivery
- `useCrawlEvents` context simplified — removed `error`/`clearError` state; retains only `events` and `activeJobIds`

### Fixed
- History view polling: 3-second interval ensures job list stays in sync after navigation
- New Crawl live monitor: replaced unreliable `crawl-event` diffing with direct `get_job` polling every 2s
- Config object indentation in `start_crawl` invocation
- Removed unused `useCrawlEvents` / error-handling dead code from `NewCrawl.tsx`
- LiveConsole "Unknown event": unwrapped Tauri v2 `{ id, payload }` wrapper in `useCrawlEvents`
- App.tsx tab-switch reset: removed dynamic `key={activeTab}` to prevent view unmount/remount on navigation


## v0.2.1 

### Added
- `dirs` crate for cross-platform home directory resolution
- `event_bus.register()` typed wrapper returning `(&Self, broadcast::Receiver)` with `Receiver::start()`


## v0.2.0 

### Added
- Multi-format export: Markdown, Merged MD, PDF, Merged PDF
- ExportModal UI with format picker, headless detection, directory picker
- `export_job_v2` command with `ExportFormat` enum (Md, MergedMd, Pdf, MergedPdf)
- Headless Chrome PDF export (feature-gated: `--features headless`)
- `check_headless_support` command for runtime feature detection
- `copy_md_files` and `merge_md_files` functions in `export.rs`
- `pulldown-cmark` for MD→HTML conversion in PDF export
- Footer: "made with love by moku" link to https://moku.cx

### Changed
- Version: v0.1.0-alpha → v0.2.0
- Logo: 20% larger, centered in sidebar
- ExportModal: fixed centering (ported to `document.body`, `inset-0 m-auto`)

### Fixed
- ExportModal centering: framer-motion `transform` conflict resolved
- `md_to_html` gated behind `headless` feature
- PDF export: tab leak fixed with `drop(tab)`, files sorted, early exit on error


## v0.1.0

### Initial
- Tauri v2 desktop app
- Documentation crawler with HTTP and headless Chrome fetcher
- DOM parsing, HTML-to-Markdown conversion, filesystem writer
- Dashboard, New Crawl, History, Settings views
- Job persistence (disk-backed JSON)
- ZIP export
