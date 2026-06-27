# Docurip — Full Documentation

> **v0.3.4** — Offline documentation crawler for Tauri v2 desktop.
> Crawls public doc sites, converts HTML to Markdown, persists assets locally, streams live progress.
> Made with love by [moku](https://moku.cx).

---

## Table of Contents

1. [Overview](#overview)
2. [Quick Start](#quick-start)
3. [Architecture](#architecture)
4. [Data Model](#data-model)
5. [Event System](#event-system)
6. [Features](#features)
7. [Views & UI](#views--ui)
8. [Export System](#export-system)
9. [Concurrency Model](#concurrency-model)
10. [Error Handling](#error-handling)
11. [Settings & Persistence](#settings--persistence)
12. [Security & Safety](#security--safety)
13. [Testing](#testing)
14. [Development Plan History](#development-plan-history)
15. [Changelog](#changelog)
16. [Future Roadmap](#future-roadmap)
17. [Usage Guides](#usage-guides)

---

## Overview

Docurip is a high-performance documentation site crawler packaged as a Tauri v2 desktop application. The backend is written entirely in Rust and the frontend is a React/TypeScript SPA styled with Tailwind CSS (dark terminal/cyberpunk aesthetic).

**Goals:**
- Crawl public documentation sites recursively starting from a root URL
- Convert each crawled HTML page to Markdown
- Download and rewrite links for local assets (images, CSS, JS)
- Provide live progress updates via an event bus
- Persist crawl jobs and results locally for history and replay
- Export results in multiple formats (MD, PDF, ZIP)

**Non-Goals:**
- Authentication or private content crawling (v1)
- Real-time collaboration or cloud sync
- Built-in Markdown editor

---

## Quick Start

```bash
# Prerequisites
# - Rust 1.95+
# - Node.js 22+
# - Windows: WebView2 runtime (bundled with Tauri)

# Clone & install
git clone <repo-url>
cd docurip
npm install

# Development
npm run tauri dev          # Full Tauri dev mode (backend + frontend)
npm run dev                # Vite frontend only (localhost:1420)

# Build
npm run build              # Production frontend build
npm run tauri build        # Full Tauri production build

# Testing
cd src-tauri
cargo test                 # All Rust tests (79 tests)
cargo test --package docurip --lib -- state::tests  # Specific module
cargo check                # Type-check (no codegen)
cargo check --features headless  # With headless Chrome support

# Format/Lint
npm run lint               # Frontend lint
```

---

## Architecture

### Backend (Rust) — `src-tauri/src/`

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Tauri app setup, plugin init (shell, fs, dialog, store, updater), command registration, startup window-size restore |
| `commands.rs` | All Tauri commands: `start_crawl`, `stop_crawl`, `pause_crawl`, `resume_crawl`, `get_job`, `list_jobs`, `get_dashboard_stats`, `delete_job`, `get_settings`, `update_settings`, `open_output_folder`, `export_job`, `export_job_v2`, `export_job_zip`, `check_headless_support`, `search_job_results`, `list_exports`, `get_system_stats`, `get_session_info`, `set_window_size` |
| `state.rs` | `AppState` with `active_jobs` (in-memory `DashMap`) + `persisted_jobs` (disk-backed JSON at `%APPDATA%`); async disk I/O via `tokio::fs` |
| `crawler/orchestrator.rs` | Main crawl loop: parallel fetch with `Semaphore`, parse, convert, write. Supports pause/resume/cancel via `AtomicBool` + `Notify`. Enforces `stay_within_domain`, `respect_robots_txt`, and `ssrf_protection`. Parallel asset downloads via `JoinSet`. |
| `crawler/job.rs` | `CrawlJob`, `CrawlProgress`, `PageResult`, `JobStatus` types |
| `crawler/robots.rs` | `robots.txt` fetcher and parser; honors `User-agent`, `Disallow`, `Allow`, `Crawl-delay` |
| `crawler/ssrf.rs` | SSRF guard: blocks loopback, link-local, RFC 1918, IPv6 ULA, `.local` TLD, known-local hostnames; resolves DNS before fetch |
| `fetcher/http.rs` | `reqwest`-based HTTP fetcher with retry + exponential backoff, configurable concurrency, timeout, 50 MB asset cap, content-type allow-list for `fetch_bytes` |
| `fetcher/headless.rs` | `headless_chrome` fetcher for JS-rendered pages (feature-gated: `#[cfg(feature = "headless")]`) |
| `parser/dom.rs` | DOM parsing with `scraper`: titles, links, assets, content extraction, URL rewriting |
| `converter/html_to_md.rs` | HTML-to-Markdown via `html2md` crate |
| `writer/fs.rs` | Filesystem writer with path sanitization, traversal prevention, query/fragment stripping |
| `asset_dl/downloader.rs` | Asset download + local path rewriting in Markdown |
| `events/bus.rs` | Tauri event emission (`crawl-event`) via `tokio::sync::broadcast` |
| `settings/config.rs` | `AppSettings`, `CrawlConfig` types with defaults |
| `export.rs` | Multi-format export: `copy_md_files`, `merge_md_files`, `md_to_html`, `export_pdf_files`, `export_merged_pdf`, public `walk_dir` + `zip_directory` |
| `exports.rs` | `list_recent_exports` helper scanning each job's `{outputDir}/zip/` folder |
| `system.rs` | System stats (CPU, memory) via `sysinfo` crate; cached `LazyLock<Mutex<System>>` singleton |

### Frontend (React) — `src/`

| Module | Responsibility |
|--------|---------------|
| `App.tsx` | Main layout: sidebar nav, tab switching, framer-motion page transitions, system status bars, update banner |
| `main.tsx` | Entry point: `ToastProvider` → `CrawlEventsProvider` → `App` |
| `hooks/useCrawlEvents.tsx` | Global Tauri event listener for crawl progress, exposes `CrawlContextType`; pushes `error` events into toast system |
| `hooks/useToasts.tsx` | Toast notification system (bottom-left, auto-dismiss 6s) |
| `hooks/useSystemStats.ts` | Polls system stats (CPU, RAM) every 2s via Tauri command |
| `hooks/useUpdater.ts` | Tauri updater integration: checks for updates, downloads, install + restart; surfaces `error` for the update banner |
| `views/Dashboard.tsx` | Stats cards (Pages Saved, Total Size, Crawl Velocity, Fail Rate) with animated count-up, quick-start form, recent activity, recent exports |
| `views/NewCrawl.tsx` | Crawl config form + live monitor panel with colored log stream, pause/resume/cancel buttons; logs in `useRef` capped at 500 |
| `views/History.tsx` | Job list with filter/search, detail view, result browser, delete/export actions |
| `views/ResultBrowser.tsx` | Split-pane result browser (tree + markdown preview + search + export) |
| `views/Settings.tsx` | App settings form with inline validation, save/reset, window-size dropdown |
| `components/ResultTree.tsx` | Collapsible tree of crawled pages with file-type icons |
| `components/MarkdownPreview.tsx` | Dark-themed Markdown rendering, DOMPurify-sanitized |
| `components/ResultSearch.tsx` | Search input (200 ms debounce) that filters the result tree by title/content/URL |
| `components/LiveConsole.tsx` | Real-time crawl log drawer with scanline effect; index-based event tracking |
| `components/EmptyState.tsx` | Reusable empty-state placeholder |
| `components/ExportModal.tsx` | Multi-format export modal (MD files, PDF files, Merged MD, Merged PDF); auto-destination |
| `components/TopStatusBar.tsx` | Session ID + live uptime counter |
| `components/SystemStatusBar.tsx` | CPU%, RAM used/total, active output path |
| `components/ToastContainer.tsx` | Bottom-left global toast renderer with framer-motion animations |
| `components/StatusBadge.tsx` | Shared `StatusIcon` + `StatusBadge` used by Dashboard/History/NewCrawl |
| `components/AnimatedCounter.tsx` | Ease-out cubic count-up animation for stat cards |
| `types/index.ts` | TypeScript types: `CrawlJob`, `PageResult`, `JobStatus`, `CrawlConfig`, `AppSettings`, `ExportFormat`, `ExportOption` |

### Key Data Flow

```
User starts crawl → commands::start_crawl()
  → validate_crawl_input() (URL + SSRF preflight + exclude-pattern compile check)
  → creates CrawlJob → persists to disk (async)
  → spawns Orchestrator::spawn() on tokio runtime
    → fetch robots.txt (if enabled)
    → parallel fetch loop (Semaphore-limited by config.concurrency)
      → URL filter: robots → stay_within_domain → ssrf → exclude_patterns
      → HttpFetcher/HeadlessFetcher → DomParser → HtmlToMarkdown → FsWriter
      → parallel asset download via JoinSet
      → emits crawl-event via EventBus after each page
    → persists job state after each page + on completion
Frontend listens via useCrawlEvents → updates UI in real-time
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust 1.95+, Tauri v2, tokio, reqwest, scraper, html2md, pulldown-cmark, headless_chrome, zip |
| Frontend | React 19, TypeScript 5+, Vite 6, Tailwind CSS 3.4, framer-motion, DOMPurify |
| Icons | @phosphor-icons/react |
| Tauri Plugins | tauri-plugin-shell, tauri-plugin-fs, tauri-plugin-dialog, tauri-plugin-store, tauri-plugin-updater |
| System | sysinfo 0.31, uuid v4 |

---

## Data Model

### Rust Types

```rust
pub struct CrawlConfig {
    pub output_dir: String,
    pub max_depth: u32,
    pub page_limit: u32,
    pub download_assets: bool,
    pub headless_strategy: String,
    pub content_selectors: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub respect_robots_txt: bool,
    pub stay_within_domain: bool,   // default: true
    pub ssrf_protection: bool,      // default: true
}

pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
}

pub struct CrawlJob {
    pub id: String,
    pub url: String,
    pub config: CrawlConfig,
    pub status: JobStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub pages_crawled: u32,
    pub page_limit: u32,
    pub current_url: Option<String>,
    pub depth: u32,
    pub max_depth: u32,
    pub results: Vec<PageResult>,
    pub errors: Vec<String>,
}

pub struct PageResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub links: Vec<String>,
    pub assets: Vec<String>,
    pub status: String,
}

pub struct AppSettings {
    pub output_dir: String,
    pub concurrency: u32,
    pub request_delay: u32,
    pub timeout: u32,
    pub user_agent: String,
    pub default_max_depth: u32,
    pub default_page_limit: u32,
    pub default_download_assets: bool,
    pub default_headless_strategy: String,
    pub default_respect_robots_txt: bool,
    pub default_stay_within_domain: bool,
    pub default_ssrf_protection: bool,
    pub window_width: u32,
    pub window_height: u32,
}
```

### TypeScript Types

```typescript
type JobStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed'

interface CrawlConfig {
  outputDir: string
  maxDepth: number
  pageLimit: number
  downloadAssets: boolean
  headlessStrategy: string
  contentSelectors: string[]
  excludePatterns: string[]
  respectRobotsTxt: boolean
  stayWithinDomain: boolean
  ssrfProtection: boolean
}

interface CrawlJob {
  id: string
  url: string
  config: CrawlConfig
  status: JobStatus
  createdAt: string
  startedAt?: string
  completedAt?: string
  pagesCrawled: number
  pageLimit: number
  currentUrl?: string
  depth: number
  maxDepth: number
  results: PageResult[]
  errors: string[]
}

interface AppSettings {
  outputDir: string
  concurrency: number
  requestDelay: number
  timeout: number
  userAgent: string
  defaultMaxDepth: number
  defaultPageLimit: number
  defaultDownloadAssets: boolean
  defaultHeadlessStrategy: string
  defaultRespectRobotsTxt: boolean
  defaultStayWithinDomain: boolean
  defaultSsrfProtection: boolean
  windowWidth: number
  windowHeight: number
}
```

> **Note:** Serialization uses `camelCase` via serde. TypeScript ↔ Rust field names match automatically.

---

## Event System

The backend emits Tauri events on a per-job broadcast channel (`crawl-event`):

| Event | Payload | Trigger |
|-------|---------|---------|
| `pageComplete` | `{ jobId, page: PageResult }` | After each page fetch + parse + write |
| `log` | `{ jobId, level, message }` | Informational messages during crawl |
| `jobStatusChanged` | `{ jobId, status: JobStatus }` | Status transitions (running → paused → completed/failed) |
| `error` | `{ jobId, message }` | Fetch/parse/write/disk errors |
| `progress` | `{ jobId, pagesDone, pagesTotal }` | Periodic progress summary |

Frontend receives events via the global `useCrawlEvents` hook and distributes to views via React Context. `error` events are also pushed into the global toast system.

---

## Features

### Crawl Engine

- **Parallel fetching** with configurable concurrency (Semaphore-based, default: 3)
- **Rate limiting** via `request_delay` (configurable in Settings)
- **Exponential backoff** retry for transient network errors (up to 3 attempts; uses typed `reqwest::Error` downcasts)
- **Domain scoping** — `stay_within_domain` restricts crawling to the start URL's origin (enforced)
- **robots.txt enforcement** — `respect_robots_txt` honors `User-agent`, `Disallow`, `Allow`, `Crawl-delay` (enforced)
- **SSRF protection** — `ssrf_protection` blocks private/internal IPs (loopback, link-local, RFC 1918, IPv6 ULA, `.local`, known-local hostnames); preflight check on the start URL and per-link check during the crawl
- **Depth control** — `max_depth` limits recursion depth (1–10)
- **Page limit** — `page_limit` caps total pages per crawl (1–1000)
- **Headless Chrome** for JS-rendered pages (feature-gated behind `--features headless`)
- **Asset downloading** — images, CSS, JS fetched in parallel via `JoinSet`; paths rewritten to local; size cap 50 MB; content-type allow-list (rejects HTML served at asset URLs)
- **Cancel** — `AtomicBool` flag checked in main loop, in-flight tasks aborted via `JoinSet::abort_all()`
- **Pause/Resume** — soft pause via `Notify`, persists state, resume continues where left off
- **Disk-error auto-pause** — write failures (permission denied, no space, read-only) pause the job with an actionable hint; classified via `io::ErrorKind` downcasts with string fallback

### Persistence

- Jobs saved as JSON at `%APPDATA%/com.docurip.app/jobs/<job_id>.json` (async `tokio::fs`)
- Persisted after every page completion + on job end
- Survives app restart: `AppState::init()` loads all saved jobs on startup (synchronous one-time load)
- Settings stored via `tauri-plugin-store` at `%APPDATA%/com.docurip.app/settings.json`

### Result Browser

- **Split-pane** layout: collapsible tree (left) + Markdown preview (right)
- **Search** within all crawled pages by title, path, or content (200 ms debounce)
- **UTF-8 safe** preview slicing (no panics on umlauts/CJK)
- **Export** — see Export System below

### Auto-Organized Output Folders

Each crawl creates three subfolders under the global output directory:
- `{outputDir}/{domain}/main/` — crawled Markdown + assets
- `{outputDir}/{domain}/zip/` — ZIP exports
- `{outputDir}/{domain}/formats/` — format exports (MD files, PDF files, merged variants)

---

## Views & UI

### Visual Design

Docurip uses a **dark terminal/cyberpunk** aesthetic:
- **Background:** near-black (`#0a0a0f`)
- **Accent:** neon green (`#00ff88`)
- **Typography:** system sans for UI, monospace for logs and data
- **Log output:** color-coded — green (`✅`), yellow (`⚠️`), red (`❌`)
- **Animations:** framer-motion page transitions, modal animations, animated stat counters

### Dashboard (`views/Dashboard.tsx`)

4 metric cards: **Pages Saved**, **Total Size** (MB), **Crawl Velocity** (pages/min), **Fail Rate** (%) — with animated count-up.
Quick-start form for entering a URL and starting a crawl immediately.
Recent Activity list of last 5 jobs with status badges.
Recent Exports panel listing ZIP files from each job's `zip/` subfolder.
Stats refresh every 3 s while a crawl is active, otherwise ~12 s; job list and exports poll every 3 s.

### New Crawl (`views/NewCrawl.tsx`)

Full configuration form:
- **Start URL** (required, validated)
- **Max Depth** (1–10)
- **Page Limit** (1–1000)
- **Stay Within Domain** toggle (default on)
- **Respect Robots.txt** toggle (default on)
- **SSRF Protection** toggle (default on)
- **Output Directory** is configured globally in Settings (per-crawl picker removed); crawls auto-create `{outputDir}/{domain}/main/`, `zip/`, and `formats/` subfolders
- **Live Monitor** panel with real-time colored log stream
- **Pause/Resume/Cancel** buttons during active crawl
- Error banner for backend validation failures
- Direct `get_job` polling every 2 s; after 3 consecutive errors the poll is cleared and the job is surfaced as failed

### History (`views/History.tsx`)

- Job list with **status filter** (All, Running, Completed, Failed)
- **Search** by URL or job ID
- Per-job actions: **Browse Results**, **Export** (multi-format), **Delete**, **Open Output Folder** (opens `main/` directly)
- **Job Detail** overlay with full PageResults list and stats
- Background polling does not flicker the loading spinner

### Settings (`views/Settings.tsx`)

- **Output Directory** native folder picker (default: `~/.docurip`)
- **Concurrency** (1–20, default 3)
- **Request Delay** (0–30000 ms, default 1000)
- **Timeout** (1000–120000 ms, default 30000)
- **User Agent** string (default `Docurip/0.3.3 (Documentation Crawler)`)
- **Default Max Depth** (1–10, default 2)
- **Default Page Limit** (1–1000, default 50)
- **Default Download Assets** (toggle)
- **Default Headless Strategy** (auto/http/always)
- **Default Respect Robots.txt** (toggle)
- **Default Stay Within Domain** (toggle)
- **Default SSRF Protection** (toggle)
- **Window Size** dropdown (1280×900 Compact, 1600×1000 Standard, 1920×1080 Full HD, 2560×1440 QHD, 3840×2160 UHD/4K) — applied live via `set_window_size`, clamped to monitor dimensions
- **Inline validation** with red borders and error messages per field
- Save/Reset buttons

### System Chrome

- **Sidebar:** Logo (centered), nav icons (Dashboard, Crawls, History, Settings), version, "made with love by moku" link
- **Top Status Bar:** Session ID (first 8 chars), live uptime counter (HH:MM:SS)
- **Bottom Status Bar:** CPU%, RAM used/total, active output path (or "idle")
- **Toast Container:** Bottom-left, slide-in animations, auto-dismiss (6 s), dismissible
- **Update Banner:** shown when `useUpdater` reports an available update; surfaces error text and switches the action button to "Retry" after a failed install attempt

---

## Export System

### Export Formats

| Format | Description |
|--------|-------------|
| **MD Files** | Copy `.md` files to destination, preserving folder structure |
| **PDF Files** | Per-page MD→HTML→PDF via headless Chrome |
| **Merged MD** | All pages concatenated into one `.md` with `---` separators |
| **Merged PDF** | All pages in one HTML doc → single PDF via headless Chrome |
| **ZIP** | Full output directory archived via `zip_directory` |

### Implementation

**Backend Commands:**
```
export_job_zip(job_id) → writes ZIP to {outputDir}/zip/
export_job_v2(job_id, format, destination?) → writes to {outputDir}/formats/ when destination is None/empty
```

**ExportFormat Enum:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    MdFiles,
    PdfFiles,
    MergedMd,
    MergedPdf,
}
```

**Strategy:**
- `MdFiles` → walks `{outputDir}/main/`, copies all `.md` to `{outputDir}/formats/`
- `PdfFiles` → for each `.md` in `main/`: converts MD→HTML via `pulldown-cmark`, opens headless Chrome, prints to PDF in `formats/`
- `MergedMd` → reads all `.md` in `main/` sorted, concatenates with `\n\n---\n\n`, writes to `formats/`
- `MergedPdf` → builds one HTML document from all pages in `main/`, headless Chrome → single PDF in `formats/`
- `export_job_zip` → uses shared `export::zip_directory` (no duplicated walk logic)

**PDF Dependencies:**
- Headless Chrome (feature-gated: `#[cfg(feature = "headless")]`)
- Without feature: returns clear error `"PDF export requires headless Chrome support. Rebuild with --features headless."`
- `check_headless_support` command for runtime detection in frontend

**Frontend:**
- `ExportModal` component with format radio group, auto-destination (no manual picker), progress state
- PDF options grayed out + tooltip when headless unavailable

### Error Cases

| Condition | Error Message |
|-----------|--------------|
| Job not found | `"Job not found: {id}"` |
| Output dir missing | `"Output directory not found for job {id}"` |
| ZIP parent missing | propagated explicit error (no silent `unwrap_or`) |
| No headless feature | `"PDF export requires headless Chrome support"` |
| Chrome start failure | `"Failed to start Chrome: {err}"` |
| Write failure | `"Failed to write export: {err}"` |

---

## Concurrency Model

- **Bounded async worker pool** via `tokio::sync::Semaphore` sized by `config.concurrency`
- **URL queue** maintained in-memory by orchestrator
- **Seen URLs** tracked in `HashSet` for deduplication
- **Pause/Resume** via `AtomicBool` + `tokio::sync::Notify` — active futures finish gracefully
- **Cancel** via `AtomicBool` — in-flight tasks aborted via `JoinSet::abort_all()`
- **Connection pooling** via `reqwest::Client` with configurable pool limits
- **Rate limiting** via `request_delay` applied per-page fetch
- **Parallel asset downloads** via `tokio::task::JoinSet`
- **Async disk persistence** via `tokio::fs` (startup load is the only synchronous I/O)

---

## Error Handling

| Error Type | Behavior |
|-----------|----------|
| **Transient** (network timeouts, 5xx) | Retry with exponential backoff up to 3 times (typed `reqwest::Error` checks: `is_timeout` / `is_connect` / `is_request`), then emit `error` and continue |
| **Permanent** (4xx, parse failures) | Emit `error`, skip URL, do not retry |
| **Disk errors** (write failures, no space, permission denied, read-only) | Pause job (classified via `io::ErrorKind` with string fallback), emit error, allow user to resume after fixing path |
| **Validation errors** | Return error string from Tauri command, display in frontend (e.g. SSRF preflight, invalid exclude pattern) |

All errors are collected in `CrawlJob.errors` and shown in the Live Console; surfaced as toasts via `useCrawlEvents`.

---

## Settings & Persistence

- **Settings:** Stored via `tauri-plugin-store` at `%APPDATA%/com.docurip.app/settings.json`
- **Jobs:** Disk-backed JSON at `%APPDATA%/com.docurip.app/jobs/<job_id>.json` (async `tokio::fs`)
- **Startup:** `AppState::init()` loads all persisted jobs on app launch
- **Persistence trigger:** After every `page_complete` event + on job completion/failure
- **Window size:** restored on startup before the window becomes visible

---

## Security & Safety

- **URL validation:** Must be `http:` or `https:` before fetching
- **SSRF protection:** preflight on start URL + per-link enforcement; blocks loopback, link-local, RFC 1918, IPv6 ULA, `.local`, known-local hostnames; resolves DNS before fetch
- **Path sanitization:** Directory traversal prevented (`..` disallowed in output paths); query strings and fragments stripped from filenames
- **MIME-type validation:** asset downloads rejected when `Content-Type` is `text/html`/`application/xhtml+xml`; allow-list covers images, fonts, CSS, JS, JSON, PDF, audio/video, `application/octet-stream`
- **Asset size cap:** 50 MB enforced via `content-length` check before consuming the response body
- **XSS prevention:** `MarkdownPreview` sanitizes with DOMPurify after MD→HTML and after search-query highlighting; restricted tags/attributes; `javascript:` URIs blocked
- **CSP hardened:** `unsafe-inline` removed from `script-src`; `withGlobalTauri: false`
- **Public content only:** No credentials or cookies forwarded
- **No secrets in code:** Zero API keys, tokens, or passwords in source
- **Dependency auditing:** `cargo audit` and `npm audit` run as part of pre-launch
- **Input validation:** all form fields validated client-side and server-side; exclude patterns compiled before crawl start

---

## Testing

### Rust Tests (79 tests)

| Module | Coverage |
|--------|----------|
| `state.rs` | Job persist/load/delete (async) |
| `fetcher/http.rs` | Retry logic, transient error classification (typed downcasts), MIME-type allow-list, size-limit enforcement |
| `fetcher/headless.rs` | Headless Chrome lifecycle (feature-gated) |
| `parser/dom.rs` | Title/link/asset extraction, URL rewriting |
| `converter/html_to_md.rs` | Markdown conversion correctness |
| `writer/fs.rs` | Path sanitization, traversal prevention, query/fragment stripping |
| `crawler/orchestrator.rs` | Disk-error classification (`io::ErrorKind` chain + string fallback) |
| `crawler/robots.rs` | robots.txt parsing and matching |
| `crawler/ssrf.rs` | IPv4 private ranges, IPv6 loopback/ULA, localhost variants, `.local` TLD, public passthrough |
| `export.rs` | File copy, merge concatenation, ExportFormat serde |
| E2E test | Full crawl against wiremock static-site fixture (index + sub-pages + image) |

```bash
cd src-tauri
cargo test                                    # All tests
cargo test --package docurip --lib -- state::tests  # Specific module
cargo test --features headless                # Including headless tests
```

### Frontend

Manual testing via `npm run tauri dev`. No automated frontend tests yet (out of scope for v1).

---

## Development Plan History

Docurip was built across 4 implementation phases + ongoing polish/hardening:

### Phase 1: Core Crawling Pipeline
- **Backend:** All 8 modules implemented (fetcher, parser, converter, writer, orchestrator, events, settings, state)
- **Frontend:** Dashboard, NewCrawl, History, Settings, LiveConsole views
- **Global event listener:** `useCrawlEvents` hook wired into `App.tsx`
- **Quick Start:** Dashboard form redirects to NewCrawl with prefilled URL

### Phase 2: Persistence, History, Cancel, Export
- **Job persistence:** Disk-backed JSON at `app_data_dir/jobs/`, survives restart
- **History:** Delete jobs, full detail view with PageResults
- **Cancel:** Graceful shutdown via AtomicBool + JoinSet abort
- **ZIP Export:** `export_job` command using `zip` crate

### Phase 3: Crawl Result Browser
- **Split-pane browser:** ResultTree (left) + MarkdownPreview (right)
- **Search:** Filter pages by title/content/URL
- **Export ZIP:** Download output as ZIP from result browser

### Phase 3+4: Polish & Performance
- Parallel fetching with Semaphore, connection pooling, rate limiting
- Headless: Feature flag gating, `cargo check --features headless`

### Phase 4: Spec Gap Coverage
- Pause/Resume with persistence
- Disk-error recovery (auto-pause)
- Dashboard stats + Recent Exports panel
- System status bars (CPU + RAM, session + uptime)
- Toast container, E2E test

### Phase 5 (v0.3.x): Hardening
- Domain filtering, robots.txt, SSRF protection actually enforced
- Async disk I/O across persistence layer
- Parallel asset downloads, system-stats caching, shared StatusBadge
- XSS sanitization, CSP hardening, content-type + size limits on assets
- Typed error classification (io::ErrorKind, reqwest::Error)
- Auto-organized output folders per domain (`main/` / `zip/` / `formats/`)
- Live dashboard stats with animated counters; in-app updater banner; window-size setting

---

## Changelog

See `CHANGELOG.md` for the full history. Recent versions:

### v0.3.4 (2026-06-27)
- MIME-type validation for asset downloads (`HttpFetcher::fetch_bytes`)
- SSRF preflight on the start URL inside `validate_crawl_input`
- Update banner shows `useUpdater.error` and offers "Retry" after failed install
- User-Agent unified to `Docurip/0.3.3` across fetcher + defaults
- Dashboard stats polling throttled to ~12 s when idle, 3 s during active crawls
- NewCrawl logs migrated to a `useRef`-backed array (cap 500) with a `logTick` counter
- `export::walk_dir` now `pub`; `export_job_zip` uses shared `zip_directory` (no duplicated recursion)
- Typed error classification: `is_disk_error` walks `anyhow::Error` chain for `io::ErrorKind`; `is_transient_error` downcasts to `reqwest::Error`
- Headless build fix: `tab.close(false)` signature

### v0.3.3 (2026-06-14)
- Window-size setting with 5 presets, live apply + monitor clamping
- Fixed dashboard stats (added `#[serde(rename_all = "camelCase")]` to `DashboardStats`)
- Fixed Recent Exports (scan each job's `{outputDir}/zip/` instead of nonexistent `app_data_dir/exports/`)

### v0.3.2 (2026-06-14)
- Auto-organized output folders: `{outputDir}/{targetName}/{main,zip,formats}/`
- ExportModal: destination fully automatic
- "Open folder" opens `main/` subfolder
- Live dashboard stats during active crawls; animated counters
- `collect_all_jobs` async (no silent skips); `compute_velocity` extracted
- Per-crawl output picker removed; configured globally in Settings

### v0.3.1 (2026-06-14)
- Tests for query-string stripping in `writer/fs.rs`
- NewCrawl logs capped at 500
- 200 ms debounce on result search
- Dashboard catch blocks now log via `console.warn`
- Fixed startup crash (synchronous one-time load in `AppState::init()`)
- Fixed `prefillUrl` re-trigger after manual edit

### v0.3.0 (2026-06-14)
- **Enforced** `stay_within_domain`, `respect_robots_txt`, `ssrf_protection` (previously defined but never enforced)
- New modules: `crawler/robots.rs`, `crawler/ssrf.rs`
- XSS prevention via DOMPurify in `MarkdownPreview`; CSP hardened (no `unsafe-inline`)
- Async disk I/O via `tokio::fs`
- System-stats caching with `LazyLock<Mutex<System>>`
- 50 MB asset download cap
- Dashboard polling merged into one 3 s interval
- Parallel asset downloads via `JoinSet`
- Shared `StatusBadge` extracted; LiveConsole event-loss fix; History flicker fix
- UTF-8-safe preview slicing; panic-safe selectors; explicit ZIP-export error propagation
- Invalid exclude patterns now error out before crawl start

### v0.2.4 — v0.2.0
See `CHANGELOG.md` for the full v0.2.x history (Dashboard stats expansion, Recent Exports, system status bars, toast container, Pause/Resume, disk-error auto-pause, E2E test, multi-format export, headless PDF, etc.).

### v0.1.0
Initial Tauri v2 desktop app with crawler, persistence, ZIP export, live monitoring.

---

## Future Roadmap

| Feature | Status |
|---------|--------|
| Headless browser fetch for JS-heavy SPAs | Feature-gated, ready |
| Multi-format export (MD, PDF, Merged) | Implemented v0.2.0 |
| Pause/Resume with persistence | Implemented v0.2.3 |
| Dashboard stats (Pages, Size, Velocity, Fail Rate) | Implemented v0.2.4 |
| System status bars (CPU, RAM, Uptime) | Implemented v0.2.4 |
| `stay_within_domain` enforcement | Implemented v0.3.0 |
| `respect_robots_txt` enforcement | Implemented v0.3.0 |
| SSRF protection | Implemented v0.3.0 (+ preflight v0.3.4) |
| Live dashboard stats during crawls | Implemented v0.3.2 |
| Window-size setting | Implemented v0.3.3 |
| MIME-type asset validation | Implemented v0.3.4 |
| Authentication (cookies, basic auth, bearer tokens) | Planned |
| Incremental/differential crawls (etag/last-modified) | Planned |
| Custom PDF styling/theming | Planned |
| Progress reporting per-page during PDF export | Planned |
| Batch export of multiple jobs | Planned |
| Built-in Markdown editor | Planned |
| Frontend automated tests | Planned |
| Virtualization in ResultTree (500+ pages) | Planned |
| Reusable headless browser instance | Planned |

---

## Usage Guides

### Guide 1: Crawling Large Documentation Sites Efficiently

**Goal:** Mirror a large docs site (500+ pages) as fast as possible without getting rate-limited.

**Step-by-step:**

1. **Open Settings** and configure global defaults:
   - **Concurrency:** `8` — 8 parallel fetches are safe for most public docs hosts
   - **Request Delay:** `200` ms — adds 200 ms between each parallel batch, prevents 429s
   - **Timeout:** `30000` ms — 30 s per page is generous for slow servers
   - **User Agent:** something identifiable like `Docurip/0.3.4` — some hosts block generic UAs
   - **Default Max Depth:** `3` — most docs are 2–3 levels deep
   - **Default Page Limit:** `500` — cap to prevent runaway crawls
   - Keep **Respect Robots.txt**, **Stay Within Domain**, **SSRF Protection** on (defaults)

2. **Start a new crawl** from Dashboard or NewCrawl tab:
   - Enter the docs root URL (e.g., `https://docs.example.com`)
   - Click **Start Crawl**

3. **Monitor in real time:**
   - The **Live Monitor** shows each page: `✅` for success, `❌` for errors
   - Bottom status bar shows real-time CPU/RAM — bump concurrency down if CPU hits 100%
   - Use **Pause** if you see a spike of 429/503 errors, wait 30 s, then **Resume**

4. **After crawl completes:**
   - **History** → **Browse Results** to inspect the captured pages
   - Use the **search bar** to find specific topics
   - Export: **Merged MD** is best for feeding into LLMs; **MD Files** preserves folder structure

**Pro tip:** For sites behind Cloudflare or aggressive rate limits, set concurrency to `2`, request delay to `1000` ms.

---

### Guide 2: Converting Doc Sites to Offline PDF

**Prerequisite:** Build with headless Chrome support:
```bash
cd src-tauri
cargo build --features headless
```

**Step-by-step:**

1. Crawl the docs site normally (see Guide 1).
2. **History** → completed job → **Export**.
3. In the **Export Modal**:
   - Select **PDF Files** (one PDF per page) or **Merged PDF** (all pages as one PDF)
   - Click **Export** — destination is auto-derived to `{outputDir}/{domain}/formats/`
4. **Merged PDF** is ideal for archiving — one searchable PDF with all content.
5. **PDF Files** preserves the original page structure.

**Limitations:** Complex JavaScript-rendered content may look basic in PDF. Use the headless Chrome fetcher during the crawl to capture JS-rendered pages first.

---

### Guide 3: Offline Knowledge Base for LLM / RAG

**Goal:** Build a complete offline Markdown archive to feed into an LLM or RAG pipeline.

**Configuration:**

- **Max Depth:** `3`
- **Page Limit:** `500–1000`
- **Stay Within Domain:** `ON`
- **Concurrency:** `4`

**Process:**

1. **Crawl** the target docs.
2. **Export** as **Merged MD** — single `.md` with all pages separated by `---`.
3. **Feed** into:
   - **ChatGPT/Claude Code** — paste into context or upload as file
   - **RAG pipelines** — split on `---` boundaries for natural chunks
   - **Obsidian / Notion** — import and search locally
4. **Keep fresh:** Re-crawl periodically.

**Pro tip:** Export as **MD Files** (not merged) for Obsidian-style backlinks. The folder structure mirrors the site's URL hierarchy.

---

### Guide 4: Crawling SPA / JavaScript-Heavy Documentation

For docs sites that render content client-side (VitePress, Docusaurus, Nextra):

1. **In Settings**, set **Default Headless Strategy** to **always** (requires `--features headless` build).
2. The orchestrator uses `HeadlessFetcher`: launches Chrome, renders the page including JS, then extracts the final DOM.
3. **Performance note:** Headless crawling is ~5–10× slower than HTTP fetch. Keep concurrency at `2–3`.
4. **Fallback:** Without headless, the HTTP fetcher still works — many docs sites pre-render for SEO, so HTTP fetch often captures 90%+ of content.

---

### Guide 5: Troubleshooting Common Issues

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| **"Invalid URL" error** | URL missing `https://` prefix | Include full URL: `https://docs.example.com` |
| **"Blocked: private/internal address"** | SSRF preflight | Disable SSRF Protection only for explicitly trusted internal sites |
| **Many 403/429 errors** | Rate limiting | Lower concurrency to `2`, increase request delay to `500–1000` ms |
| **Crawl pauses with "no space"** | Output disk full | Free space or change output directory in Settings, then Resume |
| **Crawl pauses with "permission denied"** | Output dir not writable | Fix permissions or pick another folder, then Resume |
| **Export button grayed out** | Job still running / no job selected | Wait for completion or select a completed job |
| **PDF export grayed out** | Headless Chrome not built | Rebuild with `cargo build --features headless` |
| **Dashboard stats show `NaN` or `0`** | First launch / no completed crawls | Normal — start a crawl |
| **Crawl produces 0 pages** | URL redirects, 404, or blocked by robots.txt | Verify URL in browser; try toggling **Respect Robots.txt** off for sites you own |
| **Update banner shows "Retry"** | A previous update install failed | Click Retry; check console / banner error text |

---

### Guide 6: Keyboard Shortcuts & Power-User Tips

| Shortcut / Action | Effect |
|-------------------|--------|
| **Sidebar nav** | Switch between Dashboard / NewCrawl / History / Settings |
| **Dashboard Quick Start** | Paste a URL and submit — jumps to NewCrawl with the URL prefilled |
| **History Status Filter** | Filter jobs by Running / Completed / Failed |
| **Result Browser Search** | 200 ms debounce; filters the page tree live |
| **System Bars** | Top: session ID (copy for support) + uptime. Bottom: live CPU/RAM. |

**Data locations:**
```
Windows: %APPDATA%\com.docurip.app\
  ├── jobs\           ← All crawl jobs as JSON
  └── settings.json   ← App settings

Output (configured in Settings, default ~/.docurip):
  {outputDir}/{domain}/
    ├── main/         ← Crawled Markdown + assets
    ├── zip/          ← ZIP exports
    └── formats/      ← MD / PDF / merged exports
```

---

## Constraints

- Rust toolchain: 1.95+
- Tauri v2 with `reqwest`, `tokio`, `scraper`, `html2md`, `pulldown-cmark`
- Frontend: React 19+, Vite 6, Tailwind CSS 3.4+, TypeScript 5+, DOMPurify
- Windows: WebView2 runtime required (bundled with modern Windows/Tauri)
- Job persistence: async `tokio::fs` at runtime; synchronous one-time load on startup
- Headless Chrome: feature-gated (`cargo check --features headless` to verify); `tab.close(false)` required since headless_chrome 1.x
- `Browser::close()` removed in `headless_chrome` v1.x — use `drop(h)` instead
- No `Spinner` icon in Phosphor Icons — use `SpinnerGap` instead

---

## Known Issues & Technical Debt

> Detailed analysis available in `docs/checks/PROBLEMS.md`. Many issues from earlier audits have since been resolved across v0.3.0–v0.3.4 (domain/robots/SSRF enforcement, async disk I/O, system-stats caching, parallel asset downloads, typed error classification, shared StatusBadge, query-string stripping, NewCrawl polling, etc.).

### Remaining / Lower Priority

| Area | Issue |
|------|-------|
| Performance | Headless browser instance created per fetch (should reuse) — `headless.rs` |
| Performance | No virtualization in `ResultTree` — slow for 500+ pages |
| Memory | `merge_md_files` reads all files into a single `String` (OOM risk for very large crawls) |
| Memory | `walk_dir` reads entire directory list into memory (no streaming) |
| Robustness | `rewrite_asset_urls` does string replacement (not DOM-aware) |
| UX | `MarkdownPreview` uses regex-based rendering (fragile vs. a real MD renderer) |
| Security | Optional IP-blacklist for outbound (SSRF is enforced for private ranges already) |
| Security | Regex DoS via `exclude_patterns` (consider `fancy-regex` with timeout) |
