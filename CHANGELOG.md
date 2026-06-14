# Changelog

## v0.2.5 (2026-06-14)

### Security
- **XSS prevention in MarkdownPreview**: added DOMPurify sanitization after Markdown-to-HTML rendering and after search-query highlighting; restricted allowed tags/attributes and blocked `javascript:` URIs in links
- **CSP hardened**: removed `'unsafe-inline'` from `script-src`; set `withGlobalTauri: false` so the Content Security Policy now actually blocks injected inline scripts

### Changed
- `useCrawlEvents`: migrated from `window.__TAURI__.event.listen` back to typed `listen()` from `@tauri-apps/api/event` with proper async cleanup and TypeScript-safe event payloads

### Fixed
- **UTF-8 panic in search preview**: `extract_preview` now uses `char_safe_start`/`char_safe_end` helpers to find valid UTF-8 character boundaries before slicing, preventing panics on non-ASCII content (e.g. German umlauts, CJK characters)
- **Panic-safe CSS selector**: replaced `unwrap()` on hardcoded `a[href]` selector in `DomParser` with `expect()` carrying an explanatory message
- **ZIP export path error**: `export_job_zip` now propagates an explicit error when `output_dir` has no parent directory instead of silently falling back via `unwrap_or`
- **Silent polling failure in New Crawl**: `get_job` polling now tracks consecutive errors via a ref; after 3 consecutive failures the interval is cleared and the job status is set to `failed` so the UI reflects the broken state instead of freezing silently
- **Invalid exclude patterns ignored silently**: `validate_crawl_input` now validates each exclude pattern before the crawl starts and returns an actionable error if any pattern is malformed; `Orchestrator::new` propagates the error via `?` instead of discarding it with `.ok()`

## v0.2.4 (2026-06-14)

### Added
- `dirs` crate for cross-platform home directory resolution
- `event_bus.register()` typed wrapper returning `(&Self, broadcast::Receiver)` with `Receiver::start()`

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

## v0.2.3 (2026-06-14)

### Added
- `dirs` crate for cross-platform home directory resolution
- `event_bus.register()` typed wrapper returning `(&Self, broadcast::Receiver)` with `Receiver::start()`

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

## v0.2.0 (2026-06-13)

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

## v0.1.0 (unreleased)

### Initial
- Tauri v2 desktop app
- Documentation crawler with HTTP and headless Chrome fetcher
- DOM parsing, HTML-to-Markdown conversion, filesystem writer
- Dashboard, New Crawl, History, Settings views
- Job persistence (disk-backed JSON)
- ZIP export
