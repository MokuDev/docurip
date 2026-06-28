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

cd src-tauri && cargo test           # Run all Rust tests (83 tests)
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
| `crawler/orchestrator.rs` | Main crawl loop: BFS queue, semaphore-bounded concurrency, pause/resume via atomics |
| `crawler/job.rs` | `CrawlJob`, `JobStatus`, `PageMeta` data models |
| `crawler/robots.rs` | robots.txt parser |
| `crawler/ssrf.rs` | SSRF protection (blocks private IPs) |
| `fetcher/http.rs` | reqwest-based HTTP fetcher with retry logic |
| `fetcher/headless.rs` | headless_chrome wrapper (behind `headless` feature flag) |
| `parser/dom.rs` | Scraper-based DOM extraction (titles, links, assets, content via CSS selectors) |
| `converter/html_to_md.rs` | `html2md` wrapper — HTML→Markdown |
| `writer/fs.rs` | Async filesystem writer with URL→path mapping and path sanitization |
| `asset_dl/downloader.rs` | Asset fetcher with MIME allow-list and 50MB cap |
| `export.rs` | Export pipeline: copy MD, merge MD, PDF (headless), JSON, ZIP |
| `importer/` | PDF/EPUB→Markdown import with image extraction |
| `events/bus.rs` | `EventBus` — broadcasts `CrawlEvent` variants to frontend via Tauri emit |
| `state.rs` | `AppState` — in-memory active jobs + JSON-persisted completed jobs |
| `settings/config.rs` | `AppSettings` and `CrawlConfig` structs (serde, persisted via tauri-plugin-store) |

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
- `src/components/` — ExportModal, LiveConsole, MarkdownPreview, ResultTree, ResultSearch
- `src/hooks/` — useCrawlEvents (event listener), useToasts, useUpdater, useSystemStats
- `src/types/index.ts` — All shared TypeScript interfaces and the `EXPORT_OPTIONS` constant

### Key conventions

- **Serde naming:** Rust uses `snake_case` with `#[serde(rename_all = "camelCase")]` for IPC — frontend sees camelCase.
- **Feature flags:** `headless` feature gates headless_chrome dependency and PDF export. Non-headless builds stub PDF functions with `anyhow::bail!`.
- **Job persistence:** Jobs serialize to JSON files in `%APPDATA%/com.docurip.app/jobs/`. Active jobs live in `AppState.active_jobs` (RwLock<HashMap>), completed ones in `persisted_jobs`.
- **Output layout:** `~/.docurip/{domain}/main/` (crawled content), `formats/` (exports), `zip/` (archives).
- **Event streaming:** Backend emits `CrawlEvent` variants (Progress, Log, PageComplete, JobStatusChanged, Error) — frontend listens on `"crawl-event"` channel.
- **Styling:** Tailwind CSS with custom color tokens (deepVoid, abyssal, ghost, charcoal, accentGreen, crimson, etc.) defined in `tailwind.config.js`.
