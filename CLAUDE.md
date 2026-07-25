# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Docurip?

A Tauri v2 desktop app that crawls documentation websites and converts them into offline Markdown archives. Rust backend handles parallel fetching, HTML→Markdown conversion, and asset downloads. React frontend provides live progress streaming, result browsing, and export.

## Build & Development Commands

```bash
npm install                          # Install frontend dependencies
npm run tauri dev                    # Dev mode with hot-reload (frontend + backend)
npm run tauri build                  # Production build
npm run tauri build -- --features headless  # With headless Chrome (PDF export, JS-rendered pages)

cd src-tauri && cargo test           # Run all Rust tests (181 lib tests; e2e test is --ignored)
cd src-tauri && cargo test export    # Run tests matching "export"
cd src-tauri && cargo check          # Fast type-check without building

npm run lint                         # Lint frontend
npx tsc --noEmit                     # TypeScript type-check only
```

**Note:** There are pre-existing TS errors in `src/views/Settings.tsx` (missing React namespace) — these exist on main and don't block Vite builds.

**Linux build prereqs:** `libgtk-3-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev libayatana-appindicator3-dev`

## Architecture

### Two-process Tauri model

- **Frontend** (`src/`): React 19 + TypeScript + Vite. Communicates with backend via `invoke()` IPC calls and receives real-time events via Tauri's `emit`/`listen`.
- **Backend** (`src-tauri/src/`): Rust. All crawl logic, file I/O, and export runs here. Commands are registered in `lib.rs` via `tauri::generate_handler![]`.

### Backend module map

| Module | Purpose |
|--------|---------|
| `commands.rs` | All `#[tauri::command]` handlers — the IPC surface between frontend and backend |
| `crawler/orchestrator.rs` | Main crawl loop: BFS queue, semaphore-bounded concurrency, pause/resume via atomics. Filter helpers (`build_regex_set`, `passes_include_rules`) are kept as free fns so they're unit-testable without an `Orchestrator`. |
| `crawler/job.rs` | `CrawlJob`, `JobStatus`, `PageMeta` data models. `CrawlJob` also carries `bookmarks: Vec<String>` and `annotations: HashMap<String,String>` — both serde-defaulted so on-disk jobs from older versions load unchanged. |
| `crawler/batch.rs` | `BatchJob` + `BatchRunner`: sequential child crawls tagged with a `batchId`, per-batch on-failure override (Continue/Stop). |
| `crawler/robots.rs` | robots.txt parser |
| `crawler/ssrf.rs` | SSRF protection (blocks private IPs) — also enforced on redirect hops by the sitemap fetcher. |
| `fetcher/http.rs` | reqwest-based HTTP fetcher with retry logic |
| `fetcher/headless.rs` | headless_chrome wrapper (behind `headless` feature flag) |
| `parser/dom.rs` | Scraper-based DOM extraction (titles, links, assets, content via CSS selectors) |
| `converter/html_to_md.rs` | `html2md` wrapper — HTML→Markdown |
| `writer/fs.rs` | Async filesystem writer with URL→path mapping and path sanitization |
| `asset_dl/downloader.rs` | Asset fetcher with MIME allow-list and 50MB cap |
| `export.rs` / `exports.rs` | Export pipeline: copy MD, merge MD, PDF (headless), JSON, ZIP; `exports.rs` tracks recent-export history. |
| `importer/` | PDF/EPUB→Markdown import with image extraction (`pdf.rs`, `epub.rs`, `text_cleaner.rs`) |
| `sitemap/` | Sitemap auto-discovery & import: probes `robots.txt` + well-known locations, parses `<urlset>` / `<sitemapindex>` (incl. gzip + CDATA), enforces caps (10 k URLs, 50 sub-sitemaps, depth 2, 50 MB body, 30 s timeout). |
| `events/bus.rs` | `EventBus` — broadcasts `CrawlEvent` variants to frontend via Tauri emit |
| `state.rs` | `AppState` — `active_jobs` (in-memory `RwLock<HashMap>`) plus three JSON-file-backed stores unified by the generic `JsonStore<T>`: `jobs`, `templates`, `batches`. `persist_job` / `persist_template` remain as thin back-compat wrappers. |
| `settings/config.rs` | `AppSettings` and `CrawlConfig` structs (serde, persisted via tauri-plugin-store) |
| `settings/profiles.rs` | `CrawlProfile` enum with per-variant defaults (max depth, page limit, content selectors, exclude patterns, robots policy). |
| `settings/templates.rs` | `CrawlTemplate` data model — user-defined named crawl configurations. |
| `system.rs` | `SystemStats` sampling for the dashboard (CPU / memory / uptime). |

### Crawl pipeline flow

```
URL → Orchestrator (BFS + semaphore) → HttpFetcher/HeadlessFetcher
  → DomParser (extract content, links, assets)
  → HtmlToMarkdown (html2md crate)
  → FsWriter (write .md + assets to disk)
  → EventBus (stream progress to frontend)
```

### Frontend structure

- `src/App.tsx` — Shell with sidebar nav, tab routing (no react-router — state-based)
- `src/views/` — Dashboard, NewCrawl, History, Settings, ImportView, ResultBrowser
- `src/components/` — ExportModal, LiveConsole, MarkdownPreview (with search-highlight nav via a `TreeWalker`-based `highlightMatches`), ResultTree, ResultSearch, StatusBadge, EmptyState, TemplateBar, SitemapPickerModal, BatchUrlList, ToggleRow, FilterField, AnnotationPanel, ShortcutRow, TopStatusBar, SystemStatusBar, ToastContainer
- `src/hooks/` — useCrawlEvents (event listener; parallelizes `get_settings` + `get_job` on terminal events), useToasts, useUpdater, useSystemStats, useTheme, useNotifications, useKeyboardShortcuts, useShortcutOverrides
- `src/types/index.ts` — All shared TypeScript interfaces and the `EXPORT_OPTIONS` constant

### Key conventions

- **Serde naming:** Rust uses `snake_case` with `#[serde(rename_all = "camelCase")]` for IPC — frontend sees camelCase.
- **Feature flags:** `headless` feature gates headless_chrome dependency and PDF export. Non-headless builds stub PDF functions with `anyhow::bail!`.
- **Job persistence:** Jobs serialize as one JSON file per entry in `%APPDATA%/com.docurip.app/jobs/`. Active jobs live in `AppState.active_jobs` (RwLock<HashMap>); completed ones live in the generic `AppState.jobs: JsonStore<CrawlJob>` (same pattern is reused for `templates` and `batches`). `toggle_bookmark` and `set_annotation` mutate the active job under its write lock (or the persisted copy via `JsonStore::get`), then re-persist through `persist_job`.
- **Backward-compatible job fields:** New fields added to `CrawlJob` (currently `batch_id`, `bookmarks`, `annotations`) use `#[serde(default)]` so older on-disk jobs still deserialize cleanly. Preserve this pattern when extending the struct.
- **Output layout:** `~/.docurip/{domain}/main/` (crawled content), `formats/` (exports), `zip/` (archives).
- **Event streaming:** Backend emits `CrawlEvent` variants (Progress, Log, PageComplete, JobStatusChanged, Error) — frontend listens on `"crawl-event"` channel.
- **Styling:** Tailwind CSS with custom color tokens (deepVoid, abyssal, ghost, charcoal, accentGreen, crimson, etc.) defined in `tailwind.config.js`.

## Testing

- **Rust:** `cd src-tauri && cargo test --lib` (181 tests). The integration test `tests/e2e_crawl.rs` spins up a local HTTP server and drives a full `Orchestrator::spawn` — it is `#[ignore]` on Windows (tauri DLL load failure) and normally skipped; run explicitly with `cargo test --test e2e_crawl -- --ignored`. Whenever you add a required field to `AppSettings` or `CrawlJob`, update its literal there too or the whole test target stops compiling.
- **Frontend:** `npx vitest run` (jsdom + Testing Library). Component tests colocate as `<Name>.test.tsx` next to the source. When testing debounced flows (`AnnotationPanel`), prefer `vi.advanceTimersByTimeAsync` and short microtask flushes over `waitFor` — `waitFor` polls via `setInterval` and hangs under fake timers.
- **Screenshots without Rust:** every `invoke()` call throws in a plain Vite dev server. To grab UI screenshots the shell in that state renders empty views; for feature shots (Result Browser etc.) mount the target component from a temporary demo entry that stubs `window.__TAURI_INTERNALS__.invoke` with a per-command mock, then delete the demo files.
