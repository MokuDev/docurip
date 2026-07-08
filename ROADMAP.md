# Docurip Roadmap to v1.0

## Goal

End state: a stable, single-user desktop productivity tool for crawling documentation sites up to **5,000–10,000 pages** and converting documentation (including PDF and ePub) into offline Markdown, accessible both through the GUI and a CLI.

## Scope

- Personal, offline-first usage.
- Local data and settings only; no cloud services or user accounts.
- v1.0 is a polished desktop app, not an enterprise multi-user server.

## Decisions Made

- **v0.4** – Foundations: stability, test coverage, scale groundwork, developer experience.
- **v0.5** – Import & Export: **PDF/ePub → Markdown** is the headline feature.
- **v0.6** – UX & Automation: recurring crawls, filters/rules, templates; **OCR** for scanned PDFs/images as a nice-to-have.
- **v0.7** – Platforms & Distribution: installer, updater, cross-platform build preparation.
- **v1.0** – Productivity-Ready: **CLI mode** (forces clean separation between backend logic and the Tauri app handle), final polish, stable release.

## Phases

### v0.4 – Foundations - Done! [X]

- Stabilize the existing crawler, event system, and export pipeline.
- Expand Rust and frontend test coverage for critical paths.
- Introduce safeguards for larger crawls: memory bounds, queue backpressure, and smarter persistence.
- Improve developer experience: logging, build times, typed error classification, and debugging helpers.

### v0.5 – Import & Export

- Add **PDF → Markdown** import (text extraction; image-based PDFs handled later via OCR). - Done! [X]
- Add **ePub → Markdown** import by unpacking the archive and converting HTML content. - Done! [X]
- Improve export UX: better format selection, naming, and destination control.
- Validate import quality with a representative set of real-world PDFs and ePubs.

### v0.6 – UX & Automation

Broken into incremental sub-releases, each building on the previous:

#### v0.6.0 – Theme System ✅
- Dark / Light / System theme toggle with CSS variable infrastructure.
- WCAG AA contrast fixes for light mode.

#### v0.6.1 – Filtering & Foundations
- **Include-patterns + path-prefix filter**: whitelist URLs by regex or simple path prefix (e.g. `/docs/api/` only), complementing the existing exclude-patterns.
- **Keyboard shortcuts**: power-user navigation (`Ctrl+N` New Crawl, `Ctrl+F` search, `Esc` close modal, arrow keys in ResultTree).
- **Desktop notifications**: system-level notification on crawl completion/failure via `tauri-plugin-notification`. Essential for batch and scheduled crawls.

#### v0.6.2 – Templates & Re-Crawl
- **Job templates**: save a named crawl configuration (URL + all settings) and re-apply it later. Extends the existing `CrawlProfile` system with user-defined templates.
- **Re-crawl with same settings**: one-click "crawl again" on completed jobs, pre-filling all original settings.
- **Auto-export after crawl**: configure a default export format (ZIP, Merged MD, etc.) that runs automatically when a crawl completes.

#### v0.6.3 – Batch & Sitemap
- **Multi-URL queue (batch crawl)**: enter multiple URLs (textarea or dynamic input list) that are crawled sequentially with shared or per-URL settings.
- **Sitemap import as URL source**: fetch and parse `sitemap.xml` from a target domain, present URLs as a selectable list to seed the crawl queue.

#### v0.6.4 – Result Browser Upgrade
- **Bookmarks**: mark/favorite individual pages in the result browser for quick access.
- **Search highlighting in preview**: highlight matched terms in the MarkdownPreview pane when searching.
- **Annotations**: attach user notes to crawled pages, persisted alongside the job data.

#### v0.6.5 – Scheduling & Diff
- **Scheduled / recurring crawls**: cron-style repeat (daily/weekly/monthly) with timer persistence and startup check. Builds on templates, batch queue, and notifications.
- **Crawl diff / change detection**: when re-crawling a previously crawled site, detect and display new, deleted, and modified pages.

#### v0.6.6 – OCR (Nice-to-have)
- **OCR for scanned PDFs and images**: extract text from image-based PDF pages and embedded images so they become searchable Markdown. Optional feature due to heavy dependencies (Tesseract or Rust-native engine). Kept as a separate release to isolate the dependency footprint.

### v0.7 – Platforms & Distribution

- Robust installer packaging and auto-updater flow.
- Prepare macOS/Linux builds where feasible (Windows remains primary).
- Signed releases and clean uninstall/upgrade behavior.

### v1.0 – Productivity-Ready

- **CLI mode** exposing backend commands independently of the Tauri app handle.
- Final documentation and user-facing guides.
- Settings migration path from earlier versions.
- Stable release with defined support policy.

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| PDF/ePub parsing is fragmented and file-dependent | High | Use established, well-maintained Rust crates; test against a diverse corpus; gracefully degrade on unsupported layouts. |
| OCR adds heavy dependencies and language packs | Medium | Make OCR an optional feature/plugin; start with Tesseract or a Rust-native engine; ship language data separately. |
| CLI refactor breaks existing Tauri commands | Medium | Keep Tauri commands as thin wrappers over reusable backend functions; introduce the CLI only after v0.4/v0.5 stabilize the API. |
| Scaling past 10k pages strains JSON persistence | Medium | Monitor in v0.4; if JSON becomes a bottleneck, evaluate a local embedded DB in a later version. |
| Cross-platform build issues | Low | Build and test on target platforms early in v0.7; keep platform-specific code isolated. |

## Validation Plan

- Each phase ends with a demo milestone and passing tests.
- v0.4 acceptance: existing workflows remain stable; `cargo test` and `npm run build` pass; a 1,000-page crawl completes without failure.
- v0.5 acceptance: representative PDF and ePub files convert to usable Markdown.
- v0.6.1 acceptance: include-patterns filter URLs correctly; keyboard shortcuts work across all views; desktop notifications fire on crawl completion.
- v0.6.2 acceptance: user can save, list, and apply job templates; re-crawl pre-fills settings; auto-export produces correct output.
- v0.6.3 acceptance: batch crawl processes multiple URLs sequentially; sitemap import fetches and displays selectable URL list.
- v0.6.4 acceptance: bookmarks persist across sessions; search terms are highlighted in preview; annotations save and load correctly.
- v0.6.5 acceptance: scheduled crawl fires on time after app restart; diff view correctly identifies new/changed/deleted pages.
- v0.6.6 acceptance: OCR, if included, produces useful text on sample scanned pages.
- v0.7 acceptance: installer/updater works on a clean Windows machine.
- v1.0 acceptance: 5,000-page crawl succeeds; CLI can start a crawl and export results; all tests pass on the release branch.

## Open Questions

- ~~Which exact crates should be used for PDF and ePub parsing?~~ Resolved in v0.5: `pdf_extract` and `epub` crates.
- Which OCR engine and language-pack strategy should be used? (Tesseract vs. Rust-native; decide during v0.6.6 planning if OCR is pursued.)
- Should macOS/Linux be first-class v1.0 targets or deferred to a post-1.0 release?
- How should job templates be persisted? (JSON files in app data dir vs. tauri-plugin-store; decide during v0.6.2.)
- What scheduling backend for recurring crawls? (In-process timer with on-startup catch-up vs. OS-level scheduler; decide during v0.6.5.)
- How granular should crawl diffs be? (Page-level new/deleted/changed vs. line-level content diff; decide during v0.6.5.)
