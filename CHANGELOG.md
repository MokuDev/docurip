# Changelog

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
