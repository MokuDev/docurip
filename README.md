<div align="center">
  <img src="assets/docurip_logo_g.png" alt="docurip" width="460" />

  <br />
  <br />

  <strong>Turn any documentation site into a clean, offline Markdown archive.</strong>

  <br />
  <br />

  [![Version](https://img.shields.io/badge/version-0.3.4-00ff88?style=flat-square&labelColor=0a0a0f)](CHANGELOG.md)
  [![License](https://img.shields.io/badge/license-MIT-00ff88?style=flat-square&labelColor=0a0a0f)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-Windows-0078d4?style=flat-square&labelColor=0a0a0f)](https://github.com/MokuDev/docurip-src/releases)
  [![Rust](https://img.shields.io/badge/rust-1.95+-f74c00?style=flat-square&labelColor=0a0a0f)](https://www.rust-lang.org/)
  [![Tauri](https://img.shields.io/badge/tauri-v2-24c8db?style=flat-square&labelColor=0a0a0f)](https://tauri.app/)

  <br />

  [Website](https://docurip.moku.cx) · [Releases](https://github.com/MokuDev/docurip-src/releases) · [Changelog](CHANGELOG.md) · [Roadmap](ROADMAP.md)
</div>

---

<div align="center">
  <img src="assets/screenshots/dashboard.png" alt="Dashboard — 154 pages saved, 200.9 pages/min, 0.0% fail rate" width="49%" />
  <img src="assets/screenshots/new-crawl.png" alt="New Crawl — live monitor streaming progress in real time" width="49%" />
  <img src="assets/screenshots/result-browser.png" alt="Result Browser — split-pane tree and Markdown preview" width="49%" />
  <img src="assets/screenshots/export.png" alt="Export modal — Markdown Files, Merged MD, PDF options" width="49%" />
</div>

---

## What is docurip?

Docurip is a high-performance desktop app that recursively crawls documentation websites and converts them into structured, offline Markdown archives. The Rust backend handles parallel fetching, HTML-to-Markdown conversion, and asset downloads. The React frontend streams live progress and lets you browse, search, and export results — all without leaving the app.

Built for developers who want their docs available offline — for LLM context windows, RAG pipelines, air-gapped environments, or just reading without an internet connection.

---

## Features

### Crawl Engine

| | |
|---|---|
| **Parallel fetching** | Semaphore-bounded concurrency (configurable, default 3) with a shared `reqwest` connection pool |
| **Pause / Resume / Cancel** | Soft-pause via atomics — in-flight requests finish gracefully before stopping |
| **Headless Chrome** | Feature-gated fetcher for JS-rendered SPAs; strategies: `never`, `auto`, `always` |
| **Automatic retry** | Exponential backoff for transient errors (timeouts, 5xx); permanent errors (4xx) fail immediately |
| **Disk-error auto-pause** | Detects permission errors, full disks, and read-only filesystems — pauses the job so you can fix the path and resume |
| **Domain scoping** | Stays within the start URL's origin by default; configurable |
| **Depth & page limits** | `maxDepth` (1–10) and `pageLimit` (1–10,000) prevent runaway crawls |

### Safety & Compliance

| | |
|---|---|
| **SSRF protection** | Blocks loopback, RFC 1918, link-local, IPv6 ULA, `.local` TLD, and hosts that resolve to private IPs — checked before every request |
| **robots.txt enforcement** | Fetches and parses `/robots.txt`, honors `User-agent`, `Disallow`, `Allow`, and `Crawl-delay` |
| **Asset safety** | 50 MB size cap, MIME-type allow-list, path sanitization, directory-traversal prevention |
| **XSS prevention** | Markdown preview sanitized with DOMPurify; `javascript:` URIs blocked |
| **CSP hardened** | No `unsafe-inline` in `script-src`; `withGlobalTauri: false` |

### Export

| Format | Description |
|--------|-------------|
| **MD Files** | Individual `.md` files preserving the site's folder structure |
| **Merged MD** | All pages concatenated into one file — ideal for pasting into an LLM or a RAG pipeline |
| **PDF Files** | Per-page PDF export via headless Chrome |
| **Merged PDF** | All pages as a single searchable PDF |
| **ZIP** | Full output directory as an archive |

### UI

- **Live console** — real-time colored log stream during crawls (`✅` success · `⚠️` warning · `❌` error)
- **Split-pane result browser** — collapsible page tree + Markdown preview + full-text search (200 ms debounce)
- **Dashboard** — animated stats (Pages Saved, Total Size, Crawl Velocity, Fail Rate), recent activity, recent exports
- **System status bars** — live CPU%, RAM, session uptime
- **Dark terminal aesthetic** — near-black background, neon green accent, framer-motion transitions

---

## Quick Start

**Prerequisites:** Rust 1.95+, Node.js 22+, Windows with WebView2 (bundled with modern Windows)

```bash
git clone https://github.com/MokuDev/docurip-src
cd docurip-src
npm install

# Development (hot-reload)
npm run tauri dev

# Production build
npm run tauri build

# With headless Chrome — enables JS-rendered page fetching and PDF export
npm run tauri build -- --features headless
```

```bash
# Run Rust tests
cd src-tauri && cargo test

# Lint frontend
npm run lint
```

---

## Usage

### 1. Configure Settings

Before your first crawl, open **Settings** and set:
- **Output directory** — where crawled content is saved (default: `~/.docurip`)
- **Concurrency** — parallel requests (lower if you hit 429s)
- **Request delay** — milliseconds between requests (raise for rate-limited hosts)

### 2. Start a Crawl

Go to **New Crawl**, paste a docs URL, tune depth/page limits, and click **Start Crawl**.

The live console shows each page in real time. Use **Pause** if you see a spike of rate-limit errors, wait a moment, then **Resume**.

### 3. Browse & Export

In **History**, select any completed job to:
- **Browse Results** — search and preview all captured pages
- **Export** — choose a format; output lands automatically in the job's folder
- **Open Output Folder** — opens the `main/` subfolder directly in Explorer

### Output folder layout

```
~/.docurip/
└── {domain}/
    ├── main/       ← crawled Markdown + downloaded assets
    ├── zip/        ← ZIP archives
    └── formats/    ← MD files · PDF files · merged exports
```

---

## Recipes

### LLM / RAG context

```
Crawl docs site → Export as Merged MD → paste into LLM context or load into your RAG pipeline
```
The `---` separators between pages are natural chunk boundaries for text splitters.

### Large sites (500+ pages)

```
Concurrency: 8  |  Request delay: 200 ms  |  Max depth: 3  |  Page limit: 500
```
If you see 429 errors, lower concurrency to 2 and raise the delay to 1000 ms.

### JS-rendered docs (VitePress, Docusaurus, Nextra)

Build with `--features headless` and set **Headless Strategy** to `always` in Settings. Expect ~5–10× slower throughput — keep concurrency at 2–3.

### Offline PDF archive

Crawl the site, then **Export → Merged PDF**. One searchable PDF containing all pages. Requires the headless build.

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust 1.95+, Tauri v2, tokio, reqwest, scraper, html2md, pulldown-cmark |
| Frontend | React 19, TypeScript 5, Vite 6, Tailwind CSS 3.4, framer-motion |
| Tauri Plugins | shell, fs, dialog, store, updater |
| System | sysinfo, uuid, DOMPurify |
| Optional | headless_chrome (behind `--features headless`) |

---

## Roadmap

| Version | Focus |
|---------|-------|
| **v0.4** | Foundations — stability, test coverage, memory bounds, backpressure |
| **v0.5** | Import — PDF → Markdown, ePub → Markdown |
| **v0.6** | UX & Automation — scheduled crawls, URL rules, full-text search improvements, optional OCR |
| **v0.7** | Distribution — robust installer, auto-updater, macOS/Linux build preparation |
| **v1.0** | CLI mode, 5k-page crawls, stable release |

Full plan in [ROADMAP.md](ROADMAP.md).

---

## Contributing

Issues and pull requests are welcome. Please run `cargo test` and `npm run lint` before opening a PR.

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">
  Made with love by <a href="https://moku.cx">moku</a>
</div>
