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

- Recurring / scheduled crawls and job templates.
- Advanced filtering and URL rules (include/exclude by pattern, path prefix, content type).
- Result-browser enhancements: full-text search improvements, bookmarks, and annotations.
- **OCR nice-to-have**: extract text from scanned PDF pages and images so they become searchable Markdown. Kept optional because it adds dependencies and accuracy varies.

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
- v0.6 acceptance: automation features work end-to-end; OCR, if included, produces useful text on sample scanned pages.
- v0.7 acceptance: installer/updater works on a clean Windows machine.
- v1.0 acceptance: 5,000-page crawl succeeds; CLI can start a crawl and export results; all tests pass on the release branch.

## Open Questions

- Which exact crates should be used for PDF and ePub parsing? (Decide during v0.5 planning.)
- Which OCR engine and language-pack strategy should be used? (Tesseract vs. Rust-native; decide during v0.6 planning if OCR is pursued.)
- Should macOS/Linux be first-class v1.0 targets or deferred to a post-1.0 release?
