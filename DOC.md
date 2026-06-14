# Docurip — Full Documentation

> **v0.2.5** — Offline documentation crawler for Tauri v2 desktop.
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
8. [Export System (v0.2.0)](#export-system-v020)
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
cargo test                 # All Rust tests (54 tests)
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
| `lib.rs` | Tauri app setup, plugin init, command registration |
| `commands.rs` | All Tauri commands: `start_crawl`, `stop_crawl`, `pause_crawl`, `resume_crawl`, `list_jobs`, `get_job`, `delete_job`, `export_job_zip`, `export_job_v2`, `search_job_results`, `get_settings`, `update_settings`, `get_system_stats`, `check_headless_support`, `get_session_info`, `list_exports` |
| `state.rs` | `AppState` with `active_jobs` (in-memory `DashMap`) + `persisted_jobs` (disk-backed JSON at `%APPDATA%`) |
| `crawler/orchestrator.rs` | Main crawl loop: parallel fetch with `Semaphore`, parse, convert, write. Supports pause/resume/cancel via `AtomicBool` + `Notify` |
| `crawler/job.rs` | `CrawlJob`, `CrawlProgress`, `PageResult`, `JobStatus` types |
| `fetcher/http.rs` | `reqwest`-based HTTP fetcher with retry + exponential backoff, configurable concurrency and timeout |
| `fetcher/headless.rs` | `headless_chrome` fetcher for JS-rendered pages (feature-gated: `#[cfg(feature = "headless")]`) |
| `parser/dom.rs` | DOM parsing with `scraper`: titles, links, assets, content extraction, URL rewriting |
| `converter/html_to_md.rs` | HTML-to-Markdown via `html2md` crate |
| `writer/fs.rs` | Filesystem writer with path sanitization, traversal prevention |
| `asset_dl/downloader.rs` | Asset download + local path rewriting in Markdown |
| `events/bus.rs` | Tauri event emission (`crawl-event`) via `tokio::sync::broadcast` |
| `settings/config.rs` | `AppSettings`, `CrawlConfig` types with defaults |
| `export.rs` | Multi-format export: `copy_md_files`, `merge_md_files`, `md_to_html`, `export_pdf_files`, `export_merged_pdf` |
| `system.rs` | System stats (CPU, memory) via `sysinfo` crate |

### Frontend (React) — `src/`

| Module | Responsibility |
|--------|---------------|
| `App.tsx` | Main layout: sidebar nav, tab switching, framer-motion page transitions, system status bars |
| `main.tsx` | Entry point: `ToastProvider` → `CrawlEventsProvider` → `App` |
| `hooks/useCrawlEvents.tsx` | Global Tauri event listener for crawl progress, exposes `CrawlContextType` |
| `hooks/useToasts.tsx` | Toast notification system (bottom-left, auto-dismiss 6s) |
| `hooks/useSystemStats.ts` | Polls system stats (CPU, RAM) every 2s via Tauri command |
| `views/Dashboard.tsx` | Stats cards (Pages Saved, Total Size, Crawl Velocity, Fail Rate), quick start form, recent activity, recent exports |
| `views/NewCrawl.tsx` | Crawl config form + live monitor panel with colored log stream, pause/resume/cancel buttons |
| `views/History.tsx` | Job list with filter/search, detail view, result browser, delete/export actions |
| `views/JobDetail.tsx` | Full job detail with PageResults list and stats |
| `views/Settings.tsx` | App settings form with inline validation, save/reset |
| `components/ResultBrowser.tsx` | Split-pane result browser (tree + markdown preview + search + export) |
| `components/ResultTree.tsx` | Collapsible tree of crawled pages with file-type icons |
| `components/MarkdownPreview.tsx` | Dark-themed Markdown rendering |
| `components/ResultSearch.tsx` | Search input that filters the result tree by title/content/URL |
| `components/LiveConsole.tsx` | Real-time crawl log drawer with scanline effect |
| `components/EmptyState.tsx` | Reusable empty-state placeholder |
| `components/ExportModal.tsx` | Multi-format export modal (MD files, PDF files, Merged MD, Merged PDF) |
| `components/TopStatusBar.tsx` | Session ID + live uptime counter |
| `components/SystemStatusBar.tsx` | CPU%, RAM used/total, active output path |
| `components/ToastContainer.tsx` | Bottom-left global toast renderer with framer-motion animations |
| `types/index.ts` | TypeScript types: `CrawlJob`, `PageResult`, `JobStatus`, `CrawlConfig`, `ExportFormat`, `ExportOption` |

### Key Data Flow

```
User starts crawl → commands::start_crawl()
  → creates CrawlJob → persists to disk
  → spawns Orchestrator::spawn() on tokio runtime
    → parallel fetch loop (Semaphore-limited by config.concurrency)
      → HttpFetcher/HeadlessFetcher → DomParser → HtmlToMarkdown → FsWriter
      → emits crawl-event via EventBus after each page
    → persists job state after each page + on completion
Frontend listens via useCrawlEvents → updates UI in real-time
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust 1.95+, Tauri v2, tokio, reqwest, scraper, html2md, pulldown-cmark, headless_chrome, zip |
| Frontend | React 19, TypeScript 5+, Vite 6, Tailwind CSS 3.4, framer-motion |
| Icons | @phosphor-icons/react |
| Tauri Plugins | tauri-plugin-shell, tauri-plugin-dialog, tauri-plugin-store |
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
}
```

### TypeScript Types

```typescript
type JobStatus = 'queued' | 'running' | 'paused' | 'completed' | 'failed'

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

interface PageResult {
  url: string
  title: string
  content: string
  links: string[]
  assets: string[]
  status: string
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

Frontend receives events via the global `useCrawlEvents` hook and distributes to views via React Context.

---

## Features

### Crawl Engine

- **Parallel fetching** with configurable concurrency (Semaphore-based, default: 5)
- **Rate limiting** via `request_delay_ms` (configurable in Settings)
- **Exponential backoff** retry for transient network errors (up to 3 attempts)
- **Domain scoping** — `stay_within_domain` option restricts to same origin
- **Depth control** — `max_depth` limits recursion depth (1-10)
- **Page limit** — `default_page_limit` caps total pages per crawl (1-1000)
- **Headless Chrome** for JS-rendered pages (feature-gated behind `--features headless`)
- **Asset downloading** — images, CSS, JS are fetched and paths rewritten to local
- **Cancel** — `AtomicBool` flag checked in main loop, cancels in-flight futures
- **Pause/Resume** — soft pause via `Notify`, persists state, resume continues where left off

### Persistence

- Jobs saved as JSON at `%APPDATA%/com.docurip.app/jobs/<job_id>.json`
- Persisted after every page completion + on job end
- Survives app restart: `AppState::init()` loads all saved jobs on startup
- Settings stored via `tauri-plugin-store` at `%APPDATA%/com.docurip.app/settings.json`

### Result Browser

- **Split-pane** layout: collapsible tree (left) + Markdown preview (right)
- **Search** within all crawled pages by title, path, or content
- **Export** — see Export System below

---

## Views & UI

### Visual Design

Docurip uses a **dark terminal/cyberpunk** aesthetic:
- **Background:** near-black (`#0a0a0f`)
- **Accent:** neon green (`#00ff88`)
- **Typography:** system sans for UI, monospace for logs and data
- **Log output:** color-coded — green (`✅`), yellow (`⚠️`), red (`❌`)
- **Animations:** framer-motion page transitions and modal animations

### Dashboard (`views/Dashboard.tsx`)

4 metric cards: **Pages Saved**, **Total Size** (MB), **Crawl Velocity** (pages/min), **Fail Rate** (%).
Quick Start form for entering a URL and starting a crawl immediately.
Recent Activity list of last 5 jobs with status badges.
Recent Exports panel (ZIP files).

### New Crawl (`views/NewCrawl.tsx`)

Full configuration form:
- **Start URL** (required, validated)
- **Max Depth** (1-10)
- **Page Limit** (1-1000)
- **Stay Within Domain** toggle
- **Output Directory** picker
- **Concurrency** slider
- **Concurrency** slider
- **Live Monitor** panel with real-time colored log stream
- **Pause/Resume/Cancel** buttons during active crawl
- Error banner for backend validation failures

### History (`views/History.tsx`)

- Job list with **status filter** (All, Running, Completed, Failed)
- **Search** by URL or job ID
- Per-job actions: **Browse Results**, **Export** (multi-format), **Delete**
- **Job Detail** overlay with full PageResults list and stats

### Settings (`views/Settings.tsx`)

- **Output Directory** (with folder picker)
- **Concurrency** (1-20)
- **Request Delay** (0-30000 ms)
- **Timeout** (1000-120000 ms)
- **User Agent** string
- **Default Max Depth** (1-10)
- **Default Page Limit** (1-1000)
- **Default Download Assets** (toggle)
- **Default Headless Strategy** (auto/http/always)
- **Default Respect Robots.txt** (toggle)
- **Inline validation** with red borders and error messages per field
- Save/Reset buttons

### System Chrome

- **Sidebar:** Logo (centered), nav icons (Dashboard, Crawls, History, Settings), version `v0.2.5`, "made with love by moku" link
- **Top Status Bar:** Session ID (first 8 chars), live uptime counter (HH:MM:SS)
- **Bottom Status Bar:** CPU%, RAM used/total, active output path (or "idle")
- **Toast Container:** Bottom-left, slide-in animations, auto-dismiss (6s), dismissible

---

## Export System (v0.2.5)

### Export Formats

| Format | Description |
|--------|-------------|
| **MD Files** | Copy `.md` files to destination, preserving folder structure |
| **PDF Files** | Per-page MD→HTML→PDF via headless Chrome |
| **Merged MD** | All pages concatenated into one `.md` with `---` separators |
| **Merged PDF** | All pages in one HTML doc → single PDF via headless Chrome |

### Implementation

**Backend Command:**
```
export_job_v2(job_id: String, format: ExportFormat, destination: String) → Result<String, String>
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
- `MdFiles` → walks output_dir, copies all `.md` to destination
- `PdfFiles` → for each `.md`: converts MD→HTML via `pulldown-cmark`, opens headless Chrome, prints to PDF
- `MergedMd` → reads all `.md` in sorted order, concatenates with `\n\n---\n\n`
- `MergedPdf` → builds one HTML document from all pages, headless Chrome → single PDF

**PDF Dependencies:**
- Headless Chrome (feature-gated: `#[cfg(feature = "headless")]`)
- Without feature: returns clear error `"PDF export requires headless Chrome support. Rebuild with --features headless."`
- `check_headless_support` command for runtime detection in frontend

**Frontend:**
- `ExportModal` component with format radio group, destination picker, progress state
- PDF options grayed out + tooltip when headless unavailable
- ZIP export kept for backward compatibility

### Error Cases

| Condition | Error Message |
|-----------|--------------|
| Job not found | `"Job not found: {id}"` |
| Output dir missing | `"Output directory not found for job {id}"` |
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
- **Rate limiting** via `request_delay_ms` applied per-page fetch

---

## Error Handling

| Error Type | Behavior |
|-----------|----------|
| **Transient** (network timeouts, 5xx) | Retry with exponential backoff up to 3 times, then emit `crawl:error` and continue |
| **Permanent** (4xx, parse failures) | Emit `crawl:error`, skip URL, do not retry |
| **Disk errors** (write failures, no space, permission denied) | Pause job, emit error, allow user to resume after fixing path |
| **Validation errors** | Return error string from Tauri command, display in frontend |

All errors are collected in `CrawlJob.errors` and shown in the Live Console.

---

## Settings & Persistence

- **Settings:** Stored via `tauri-plugin-store` at `%APPDATA%/com.docurip.app/settings.json`
- **Jobs:** Disk-backed JSON at `%APPDATA%/com.docurip.app/jobs/<job_id>.json`
- **Startup:** `AppState::init()` loads all persisted jobs on app launch
- **Persistence trigger:** After every `page_complete` event + on job completion/failure

---

## Security & Safety

- **URL validation:** Must be `http:` or `https:` before fetching
- **Path sanitization:** Directory traversal prevented (`..` disallowed in output paths)
- **Public content only:** No credentials or cookies forwarded
- **No secrets in code:** Zero API keys, tokens, or passwords in source
- **Dependency auditing:** `cargo audit` and `npm audit` run as part of pre-launch
- **Input validation:** All form fields validated client-side and server-side before crawl

---

## Testing

### Rust Tests (54 tests, all passing)

| Module | Coverage |
|--------|----------|
| `state.rs` | Job persist/load/delete |
| `fetcher/http.rs` | Retry logic, transient error handling |
| `fetcher/headless.rs` | Headless Chrome lifecycle |
| `parser/dom.rs` | Title/link/asset extraction, URL rewriting |
| `converter/html_to_md.rs` | Markdown conversion correctness |
| `writer/fs.rs` | Path sanitization, traversal prevention |
| `export.rs` | File copy, merge concatenation, ExportFormat serde |
| E2E test | Full crawl against wiremock static site fixture |

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

Docurip was built across 4 implementation phases + 1 polish phase:

### Phase 1: Core Crawling Pipeline
- **Backend:** All 8 modules implemented (fetcher, parser, converter, writer, orchestrator, events, settings, state)
- **Frontend:** Dashboard, NewCrawl, History, Settings, LiveConsole views
- **Critical fixes:** `Spinner` → `SpinnerGap` icon, `headlessStrategy` enum fix
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
- **Frontend:** Error display in crawl monitor, Settings validation, Empty States
- **Backend:** Parallel fetching with Semaphore, connection pooling, rate limiting
- **Headless:** Feature flag gating, `cargo check --features headless`

### Phase 4: Spec Gap Coverage
- **Pause/Resume:** Full implementation with persistence and frontend buttons
- **Disk-Error Recovery:** Automatic pause on disk errors, user can resume after fixing path
- **Dashboard Stats:** Pages Saved, Total Size, Crawl Velocity, Fail Rate
- **Recent Exports Panel:** List of recent ZIP exports with metadata
- **System Status Bars:** Top (session + uptime) and Bottom (CPU + RAM)
- **Toast Container:** Bottom-left global toast notifications
- **E2E Test:** Full crawl against wiremock static site fixture

---

## Changelog

### v0.2.5 (2026-06-14)

**Added:**
- Comprehensive codebase analysis and bug documentation
- Missing commands: `get_job`, `search_job_results`, `get_session_info`, `list_exports`
- Missing settings: `defaultDownloadAssets`, `defaultHeadlessStrategy`, `defaultRespectRobotsTxt`
- Updated data model to match actual implementation

**Known Issues:**
- See `docs/checks/PROBLEMS.md` for detailed bug/security/performance analysis

### v0.2.0 (2026-06-13)

**Added:**
- Multi-format export: Markdown, Merged MD, PDF, Merged PDF
- ExportModal UI with format picker, headless detection, directory picker
- `export_job_v2` command with `ExportFormat` enum
- Headless Chrome PDF export (feature-gated: `--features headless`)
- `check_headless_support` command for runtime feature detection
- `copy_md_files` and `merge_md_files` functions
- `pulldown-cmark` for MD→HTML conversion
- Footer: "made with love by moku" link to https://moku.cx

**Changed:**
- Version: v0.1.0-alpha → v0.2.0
- Logo: 20% larger, centered in sidebar
- ExportModal: fixed centering (ported to `document.body`, `inset-0 m-auto`)

**Fixed:**
- ExportModal centering: framer-motion `transform` conflict resolved
- `md_to_html` gated behind `headless` feature
- PDF export: tab leak fixed with `drop(tab)`, files sorted, early exit on error

### v0.1.0 (unreleased)

**Initial:**
- Tauri v2 desktop app
- Documentation crawler with HTTP and headless Chrome fetcher
- DOM parsing, HTML-to-Markdown conversion, filesystem writer
- Dashboard, New Crawl, History, Settings views
- Job persistence (disk-backed JSON)
- ZIP export
- Live crawl monitoring with progress events
- Parallel fetching with configurable concurrency
- Pause/Resume/Cancel support
- System status bars (CPU, RAM, uptime)
- Toast notification system
- Result browser with search and Markdown preview

---

## Future Roadmap

| Feature | Status |
|---------|--------|
| Headless browser fetch for JS-heavy SPAs | Feature-gated, ready |
| Multi-format export (MD, PDF, Merged) | Implemented v0.2.0 |
| Pause/Resume with persistence | Implemented v0.2.0 |
| Dashboard stats (Pages, Size, Velocity, Fail Rate) | Implemented v0.2.0 |
| System status bars (CPU, RAM, Uptime) | Implemented v0.2.0 |
| Authentication (cookies, basic auth, bearer tokens) | Planned |
| Incremental/differential crawls (etag/last-modified) | Planned |
| Custom PDF styling/theming | Planned |
| Progress reporting per-page during PDF export | Planned |
| Batch export of multiple jobs | Planned |
| Built-in Markdown editor | Planned |
| Frontend automated tests | Planned |
| SSRF protection (optional IP blacklist) | Planned |
| robots.txt enforcement | Planned |
| stay_within_domain enforcement | Planned |

---

## Usage Guides

### Guide 1: Crawling Large Documentation Sites Efficiently

**Goal:** Mirror a large docs site (500+ pages) as fast as possible without getting rate-limited.

**Step-by-step:**

1. **Open Settings** and configure global defaults:
   - **Concurrency:** `8` — 8 parallel fetches are safe for most public docs hosts
   - **Request Delay:** `200` ms — adds 200ms between each parallel batch, prevents 429s
   - **Timeout:** `30000` ms — 30s per page is generous for slow servers
   - **User Agent:** Set to something identifiable like `Docurip/0.2.0` — some hosts block generic user agents
   - **Default Max Depth:** `3` — most docs are 2-3 levels deep
   - **Default Page Limit:** `500` — cap to prevent runaway crawls

2. **Start a new crawl** from Dashboard or NewCrawl tab:
   - Enter the docs root URL (e.g., `https://docs.example.com`)
   - Enable **Stay Within Domain** — prevents following outbound links
   - Click **Start Crawl**

3. **Monitor in real time:**
   - The **Live Monitor** shows each page as it arrives: `✅` green for success, `❌` red for errors
   - Bottom status bar shows real-time CPU/RAM usage — bump concurrency down if CPU hits 100%
   - Use **Pause** if you see a spike of 429/503 errors, wait 30s, then **Resume**

4. **After crawl completes:**
   - Go to **History** → click **Browse Results** to inspect the captured pages
   - Use the **search bar** to find specific topics across all pages
   - Export: **Merged MD** is best for feeding into LLMs; **MD Files** preserves folder structure for Obsidian/Notion

**Pro tip:** For sites behind Cloudflare or aggressive rate limits, set concurrency to `2`, request delay to `1000` ms. Slower but zero blocked requests.

---

### Guide 2: Converting Doc Sites to Offline PDF

**Prerequisite:** Build with headless Chrome support. The first time you click "PDF Files" in the export modal, if it's grayed out, you need:
```bash
cd src-tauri
cargo build --features headless
```

**Step-by-step:**

1. Crawl the docs site normally (see Guide 1).
2. After crawl completes, go to **History** → find the completed job → click **Export**.
3. In the **Export Modal**:
   - Select **PDF Files** (one PDF per page) or **Merged PDF** (all pages as one PDF)
   - Click **Choose Folder** to pick where to save
   - Click **Export**
4. **Merged PDF** is ideal for archiving — one searchable PDF with all content.
5. **PDF Files** preserves the original page structure — open individual pages by filename.

**Limitations:** Complex JavaScript-rendered content may look basic in PDF. Use the headless Chrome fetcher during crawl (`HeadlessStrategy`) to capture JS-rendered pages before exporting.

---

### Guide 3: Offline Knowledge Base for LLM / RAG

**Goal:** Build a complete offline Markdown archive of a documentation site to feed into an LLM or RAG pipeline.

**Configuration:**

- **Max Depth:** `3` — captures most documentation
- **Page Limit:** `High (500-1000)` — you want everything
- **Stay Within Domain:** `ON`
- **Concurrency:** `4` — gentle on the server, steady progress

**Process:**

1. **Crawl** the target docs.
2. **Export** as **Merged MD** — produces a single `.md` file with all pages separated by `---`.
3. **Feed** the merged `.md` into:
   - **ChatGPT/Claude Code** — paste into context or upload as file
   - **RAG pipelines** — split on `---` boundaries for natural document chunks
   - **Obsidian / Notion** — import and search locally
4. **Keep fresh:** Re-crawl weekly/monthly. Job history persists across restarts — compare page counts over time.

**Pro tip:** Export as **MD Files** (not merged) if you want Obsidian-style backlinks. The folder structure mirrors the site's URL hierarchy.

---

### Guide 4: Crawling SPA / JavaScript-Heavy Documentation

Some modern docs sites (VitePress, Docusaurus, Nextra) render content client-side. For these:

1. **In Settings**, set **Fetch Strategy** to **Headless Chrome** (requires `--features headless` build).
2. When starting a crawl, the orchestrator uses `HeadlessFetcher` which launches Chrome, renders the page including JS, then extracts the final DOM.
3. **Performance note:** Headless crawling is ~5-10x slower than HTTP fetch. Keep concurrency at `2-3` and be patient.
4. **Fallback:** If headless isn't available, the HTTP fetcher still works for most docs — it just won't execute JavaScript. Many docs sites serve pre-rendered HTML for SEO, so HTTP fetch often captures 90%+ of content.

---

### Guide 5: Troubleshooting Common Issues

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| **"Invalid URL" error** | URL missing `https://` prefix | Always include full URL: `https://docs.example.com` |
| **Many 403/429 errors** | Rate limiting | Lower concurrency to `2`, increase request delay to `500-1000` ms |
| **Crawl stops with "no space"** | Output disk full | Check bottom bar RAM/disk. Change output directory in Settings, then Resume |
| **Export button grayed out** | Job still running or no job selected | Wait for job to complete, or go to History and select a completed job |
| **PDF export grayed out** | Headless Chrome not built | Rebuild with `cargo build --features headless` |
| **Black screen on startup** | Tauri event listener fails outside Tauri | This is handled by try-catch in `useCrawlEvents`. Restart the app. |
| **Dashboard stats show `NaN`** | Stats not loaded yet / empty state | Normal on first launch. Start a crawl to populate. `?? 0` fallback used internally. |
| **Crawl produces 0 pages** | URL redirects or is a 404 | Check the URL in a browser first. Some sites redirect `/docs` to `/docs/` — test both. |

---

### Guide 6: Keyboard Shortcuts & Power-User Tips

| Shortcut / Action | Effect |
|-------------------|--------|
| **Tab** | Switch between views (Dashboard → NewCrawl → History → Settings) |
| **Dashboard Quick Start** | Paste a URL and hit Enter — jumps to NewCrawl with the URL prefilled |
| **History Status Filter** | Click "Running" / "Completed" / "Failed" to filter jobs in real-time |
| **Result Browser Search** | Type to filter the page tree — results update as you type |
| **Double-click page** in Result Browser | Opens the Markdown preview on the right |
| **System Bars** | Top bar shows session ID + uptime — copy session ID for support requests. Bottom bar shows live CPU/RAM — useful when tuning concurrency. |

**Data locations:**
```
Windows: %APPDATA%\com.docurip.app\
  ├── jobs\          ← All crawl jobs as JSON
  ├── settings.json  ← App settings
  └── exports\       ← ZIP/PDF exports
```

---

## Constraints

- Rust toolchain: 1.95+
- Tauri v2 with `reqwest`, `tokio`, `scraper`, `html2md`, `pulldown-cmark`
- Frontend: React 19+, Vite 6, Tailwind CSS 3.4+, TypeScript 5+
- Windows: WebView2 runtime required (bundled with modern Windows/Tauri)
- Job persistence: synchronous `std::fs` (not `tokio::fs`)
- Headless Chrome: feature-gated (`cargo check --features headless` to verify)
- No `Spinner` icon in Phosphor Icons — use `SpinnerGap` instead
- `Browser::close()` removed in `headless_chrome` v1.x — use `drop(h)` instead

---

## Known Issues & Technical Debt

> Detailed analysis available in `docs/checks/PROBLEMS.md`

### Critical

| ID | Issue | Location |
|----|-------|----------|
| B1 | `stay_within_domain` config exists but is never enforced | orchestrator.rs |
| B2 | `respect_robots_txt` config exists but is never enforced | orchestrator.rs, config.rs |
| P1 | All disk I/O is synchronous (`std::fs`), blocks tokio runtime | state.rs, commands.rs |

### High Priority

| ID | Issue | Location |
|----|-------|----------|
| B3 | Cancel sets status to `Paused` instead of `Failed` | orchestrator.rs:299 |
| B4 | `timeout` setting not passed to HttpFetcher (hardcoded 30s) | http.rs, config.rs |
| B5 | Version strings inconsistent (0.1.0 / 0.2.0 / 0.2.5) | Multiple files |
| B6 | Query strings not stripped from filenames (invalid on Windows) | writer/fs.rs |
| B7 | Double update check in useUpdater | useUpdater.ts |
| P2 | Headless browser created per fetch (should reuse) | headless.rs |
| P3 | System stats not cached (new sysinfo::System every 2s) | system.rs |
| S2 | No content-type validation on asset downloads | downloader.rs |
| S3 | No file size limits on asset downloads | downloader.rs |

### Medium Priority

| ID | Issue | Location |
|----|-------|----------|
| B8 | LiveConsole only processes last event, misses events between renders | LiveConsole.tsx |
| B9 | History loading spinner flickers every 3s poll | History.tsx |
| B10 | prefillUrl useEffect not re-triggerable after manual clear | NewCrawl.tsx |
| B11 | AppSettings TypeScript type missing 3 fields | types/index.ts |
| B12 | walk_dir duplicated with different implementations | export.rs, commands.rs |
| P4 | Dashboard polls 3× separately (wasteful) | Dashboard.tsx |
| P5 | Assets downloaded sequentially instead of parallel | orchestrator.rs |
| P6 | Logs array copy-on-write on every event | NewCrawl.tsx |
| P7 | No virtualization in ResultTree (slow for 500+ pages) | ResultTree.tsx |
| C1 | `is_disk_error` via string matching (fragile) | orchestrator.rs |
| C2 | `is_transient_error` via string matching (fragile) | http.rs |

### Low Priority

| ID | Issue | Location |
|----|-------|----------|
| B13 | LiveConsole doesn't update with multiple events | LiveConsole.tsx |
| B14 | useUpdater error state never displayed in UI | useUpdater.ts |
| C3 | `collect_all_jobs` uses try_read (silently drops data) | commands.rs |
| C4 | DashboardStats cache uses std::sync::Mutex (blocks async) | commands.rs |
| C5 | rewrite_asset_urls does string replacement (not DOM-aware) | dom.rs |
| C6 | merge_md_files reads all into single String (OOM risk) | export.rs |
| C7 | walk_dir reads entire dir into memory (no streaming) | commands.rs |
| C8 | StatusIcon/StatusBadge duplicated in 3 components | Multiple TSX |
| C9 | Regex-based Markdown rendering (fragile) | MarkdownPreview.tsx |
| C10 | Redundant log storage (local + global) | NewCrawl.tsx |
| C11 | Dashboard catch blocks are empty (errors silently ignored) | Dashboard.tsx |

### Security Notes

| ID | Issue | Mitigation |
|----|-------|------------|
| S1 | No SSRF protection (can crawl internal IPs) | Low risk for desktop app; add optional IP blacklist |
| S4 | Regex DoS via exclude_patterns | Use fancy-regex with timeout |
| S5 | Path traversal via query strings | Strip query params from filenames |
| S6 | Output path not sanitized on frontend | Backend validation exists via Tauri |
| S7 | No user input sanitization | Typed Tauri commands limit attack surface
