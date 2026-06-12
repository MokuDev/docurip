# Docurip Design Spec

## Overview
Docurip is a high-performance documentation site crawler packaged as a Tauri v2 desktop application. The backend is written entirely in Rust (Approach A: all-in-one) and the frontend is a React/TypeScript SPA styled with Tailwind CSS. The app downloads docs sites, converts HTML pages to Markdown, persists assets locally, and streams live progress to the UI.

## Goals
- Crawl public documentation sites recursively starting from a root URL.
- Convert each crawled HTML page to Markdown.
- Download and rewrite links for local assets (images, CSS, JS).
- Provide live progress updates (pages fetched, assets downloaded, errors) via an event bus.
- Persist crawl jobs and results locally for history and replay.
- Allow configuration of crawl depth, domain restrictions, and output directory.

## Non-Goals
- Authentication or private content crawling (v1).
- Real-time collaboration or cloud sync.
- Headless-browser rendering for JS-heavy SPAs beyond basic fetch.
- Built-in Markdown editor.

## Architecture

### Backend (Rust)
The Rust side is organized into single-responsibility modules:

| Module | Responsibility |
|--------|----------------|
| `crawler` | Orchestrates a crawl job: maintains a URL queue, tracks seen URLs, coordinates fetch/parse/convert/write phases. |
| `fetcher::http` | Synchronous/async HTTP fetch using `reqwest`. Handles retries, timeouts, and status checking. |
| `fetcher::headless` | Optional headless fetch placeholder for future JS-rendered pages. |
| `parser::dom` | Parses HTML with `scraper`/`html5ever`, extracts links, assets, title, main content area. |
| `converter::html_to_md` | Converts extracted HTML content to Markdown using `html2md` or similar. |
| `writer::fs` | Writes Markdown files and assets to disk, preserving URL-derived folder structure. |
| `asset_dl::downloader` | Downloads images/CSS/JS assets and rewrites URLs in Markdown to local relative paths. |
| `events::bus` | Broadcasts crawl progress events (page completed, asset downloaded, error) to the frontend via Tauri events. |
| `settings::config` | Loads and saves user preferences (default output dir, concurrency, timeout). |

### Data Model
```rust
pub struct CrawlConfig {
    pub start_url: String,
    pub max_depth: u32,
    pub stay_within_domain: bool,
    pub output_dir: String,
    pub concurrency: u32,
    pub timeout_secs: u32,
}

pub struct CrawlJob {
    pub id: String,
    pub config: CrawlConfig,
    pub status: JobStatus, // Pending | Running | Paused | Completed | Failed
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub pages_total: u32,
    pub pages_done: u32,
    pub assets_total: u32,
    pub assets_done: u32,
    pub errors: Vec<String>,
}

pub struct PageResult {
    pub url: String,
    pub status_code: u16,
    pub markdown_path: String,
    pub title: String,
    pub links_found: Vec<String>,
    pub assets_found: Vec<String>,
}
```

### Event System
The backend emits Tauri events on a per-job channel:
- `crawl:progress` — periodic summary (pages_done, pages_total, assets_done, assets_total).
- `crawl:page` — single page completed with `PageResult` payload.
- `crawl:error` — fetch/parse/write error with URL and message.
- `crawl:completed` — job finished or failed.

### Frontend (React / TypeScript)
Single-page application with route-based views:

| View | Purpose |
|------|---------|
| **Dashboard** | Quick stats (total crawls, last crawl), start-new-crawl form, recent jobs preview. |
| **Crawls** | List of all crawl jobs with status filters, pause/resume/delete actions. |
| **History** | Detailed view of a completed job: browsable tree of saved Markdown files, search within results. |
| **Settings** | Edit `CrawlConfig` defaults, pick default output directory, set concurrency limits. |
| **Live Console** | Real-time event log for the active crawl job, showing fetch/convert/write progress and errors. |

State management: React hooks + Tauri event listeners. No external state library (Zustand/Redux) needed for v1 scope.

### Visual Design
Docurip uses a **dark terminal/cyberpunk** aesthetic:
- **Background:** near-black (`#0a0a0f` / `bg-[#0a0a0f]`)
- **Accent:** bright neon green (`#00ff88` / `text-emerald-400`)
- **Typography:** system sans for UI, monospace for logs and data readouts
- **Log/console output:** color-coded by level — green (`[OK]`, `[SUCCESS]`), yellow (`[WARN]`), red (`[ERROR]`, `[FATAL]`)
- **Dashboard layout:**
  - Top stats bar with 4 metric cards (Pages Saved, Total Size, Crawl Velocity, Fail Rate)
  - Active Crawls area with per-job cards showing progress bars, scan-line separators, and status badges
  - Live Output terminal panel (scrollable, monospace, timestamped)
  - Recent Exports panel on the right
- **Crawls list view:** per-job progress bars, action buttons (pause, stop, download report)
- **Live Monitor view:** split-pane with target hierarchy tree (left) and real-time log stream (right)
- **Global chrome:** sidebar nav (Dashboard, Crawls, History, Settings), top status bar (session ID, uptime), bottom system bar (CPU, RAM, path), system alert toasts in bottom-left
- **Logo:** pixelated green ASCII/wordmark style

## Error Handling
- **Transient errors** (network timeouts, 5xx): retry with exponential backoff up to 3 times, then emit `crawl:error` and continue.
- **Permanent errors** (4xx, parse failures): emit `crawl:error`, skip the URL, do not retry.
- **Disk errors** (write failures): pause the job, emit error, allow user to resume after fixing the path.
- All errors are collected in `CrawlJob.errors` and shown in the Live Console.

## Concurrency Model
- The crawler uses a bounded async worker pool (Tokio) sized by `config.concurrency`.
- The orchestrator maintains an in-memory queue and a `DashSet` of seen URLs for deduplication.
- Pause/resume is implemented by toggling a cancellation token; active futures finish gracefully.

## Settings Persistence
- Configuration is stored in Tauri’s `app_config_dir()` as a JSON file (`settings.json`).
- Loaded on app startup and saved on every settings change.

## Testing Strategy
- **Unit tests** for `parser::dom` (HTML extraction) and `converter::html_to_md` (Markdown output correctness).
- **Integration tests** for `fetcher::http` against a local `httpmock` server.
- **End-to-end test** for a full crawl of a small static site fixture.
- Frontend tests are out of scope for v1.

## Security & Safety
- URLs are validated before fetching (must be `http:` or `https:`).
- Output paths are sanitized to prevent directory traversal (`..` disallowed).
- Only public content is fetched; no credentials or cookies are forwarded.

## Constraints
- Rust toolchain: 1.95+ (with `time = "=0.3.35"` pinned to avoid `cookie` crate E0119).
- Tauri v2 with `reqwest`, `tokio`, `scraper`, `pulldown-cmark` or `html2md`.
- Frontend: React 18+, Vite, Tailwind CSS 3+, TypeScript 5+.

## Future Considerations (Out of Scope)
- Headless browser fetch for JS-heavy SPAs.
- Authentication (cookies, basic auth, bearer tokens).
- Incremental/differential crawls based on etag/last-modified.
- Built-in Markdown viewer/editor.
