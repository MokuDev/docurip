# Docurip — Agent Guide

## Project Overview

Tauri v2 desktop app for crawling documentation sites and converting them to offline Markdown. Rust backend + React 19/TypeScript/Tailwind frontend.

## Essential Commands

### Backend (Rust)
```bash
cd src-tauri
cargo check          # Type-check (no codegen)
cargo build          # Full build
cargo test           # Run all tests
cargo test --package docurip --lib -- state::tests  # Run specific test module
cargo check --features headless  # Check with headless Chrome support
```

### Frontend (React)
```bash
npm run dev          # Vite dev server (localhost:1420)
npm run build        # Production build
npm run preview      # Preview production build
npm run tauri dev    # Full Tauri dev mode (backend + frontend)
```

### Combined
```bash
npm run tauri dev    # Starts both Vite and Tauri backend
```

| Skill | Wann verwenden |
|-------|---------------|
| `superpowers:brainstorming` | **Vor jeder kreativen Arbeit** — Features bauen, Komponenten erstellen, Verhalten ändern. Klärt Intent und Design vor Implementierung |
| `superpowers:writing-plans` | Wenn Spec oder Anforderungen vorliegen und es sich um eine mehrstufige Aufgabe handelt — vor dem ersten Code |
| `superpowers:executing-plans` | Wenn ein schriftlicher Implementierungsplan vorliegt und ausgeführt werden soll |
| `superpowers:subagent-driven-development` | Wenn Implementierungsplan unabhängige Aufgaben enthält, die parallel bearbeitbar sind |
| `superpowers:dispatching-parallel-agents` | Bei 2+ unabhängigen Aufgaben ohne geteilten State oder sequentielle Abhängigkeiten |
| `superpowers:test-driven-development` | **Vor jeder Feature- oder Bugfix-Implementierung** — Tests zuerst schreiben |
| `superpowers:systematic-debugging` | Bei jedem Bug, fehlgeschlagenem Test oder unerwartetem Verhalten — vor Fixes |
| `superpowers:verification-before-completion` | Bevor Arbeit als "fertig", "gefixt" oder "passing" deklariert wird |
| `superpowers:requesting-code-review` | Nach Implementierung, vor Merge, wenn Aufgaben abgeschlossen sind |
| `superpowers:receiving-code-review` | Wenn Code-Review-Feedback eingeht — vor Umsetzung der Vorschläge |
| `superpowers:finishing-a-development-branch` | Wenn Implementierung abgeschlossen ist und Integration entschieden werden muss |
| `superpowers:using-git-worktrees` | Bei Feature-Arbeit die Isolation vom aktuellen Workspace braucht |
| `superpowers:writing-skills` | Wenn neue Skills erstellt oder bestehende bearbeitet werden |

---

## Regeln

1. Skill-Check vor jeder Aktion — keine Ausnahmen
2. Skills vollständig befolgen — nicht selektiv anwenden
3. Agenten delegieren nicht an andere Agenten
4. Messung vor Behauptung — `verification-before-completion` bevor "fertig" gesagt wird
5. Brainstorming vor Implementierung bei kreativen Aufgaben
6. Tests vor Code bei Features und Bugfixes

## Architecture

### Backend (`src-tauri/src/`)

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Tauri app setup, plugin init, command registration |
| `commands.rs` | All Tauri commands (start_crawl, stop_crawl, list_jobs, etc.) |
| `state.rs` | `AppState` with `active_jobs` (in-memory) + `persisted_jobs` (disk-backed JSON) |
| `crawler/orchestrator.rs` | Main crawl loop: parallel fetch with Semaphore, parse, convert, write |
| `crawler/job.rs` | `CrawlJob`, `CrawlProgress`, `PageResult`, `JobStatus` types |
| `fetcher/http.rs` | `reqwest`-based HTTP fetcher with retry + exponential backoff |
| `fetcher/headless.rs` | `headless_chrome` fetcher (feature-gated) |
| `parser/dom.rs` | DOM parsing with `scraper`: titles, links, assets, content extraction, URL rewriting |
| `converter/html_to_md.rs` | HTML-to-Markdown via `html2md` crate |
| `writer/fs.rs` | Filesystem writer with path sanitization |
| `asset_dl/downloader.rs` | Asset download + local path rewriting |
| `events/bus.rs` | Tauri event emission (`crawl-event`) |
| `settings/config.rs` | `AppSettings`, `CrawlConfig` types |
| `export.rs` | ZIP export via `zip` crate |
| `system.rs` | System stats (CPU, memory) via `sysinfo` |

### Frontend (`src/`)

| Module | Responsibility |
|--------|---------------|
| `App.tsx` | Main layout: sidebar nav, tab switching, live console drawer |
| `main.tsx` | Entry point: `ToastProvider` → `CrawlEventsProvider` → `App` |
| `hooks/useCrawlEvents.tsx` | Global Tauri event listener for crawl progress |
| `hooks/useToasts.tsx` | Toast notification system |
| `views/Dashboard.tsx` | Stats cards, quick start, recent activity |
| `views/NewCrawl.tsx` | Crawl config form + live monitor panel |
| `views/History.tsx` | Job list with filter/search, detail view, result browser |
| `views/Settings.tsx` | App settings form with inline validation |
| `components/ResultBrowser.tsx` | Split-pane result browser (tree + markdown preview + search) |
| `components/LiveConsole.tsx` | Real-time crawl log drawer |

### Key Data Flow

```
User starts crawl → commands::start_crawl()
  → creates CrawlJob → persists to disk
  → spawns Orchestrator::spawn()
    → parallel fetch loop (Semaphore-limited)
      → HttpFetcher/HeadlessFetcher → DomParser → HtmlToMarkdown → FsWriter
      → emits crawl-event via EventBus after each page
    → persists job state after each page + on completion
Frontend listens via useCrawlEvents → updates UI in real-time
```

## Testing

### Rust Tests
```bash
cd src-tauri
cargo test
```

Tests cover:
- `state.rs`: Job persist/load/delete
- `fetcher/http.rs`: Retry logic, transient error handling
- `parser/dom.rs`: Title/link/asset extraction, URL rewriting
- `converter/html_to_md.rs`: Markdown conversion
- `writer/fs.rs`: Path sanitization, traversal prevention

### Frontend
No automated tests yet. Manual testing via `npm run tauri dev`.

## Important Constraints

1. **No `Spinner` icon** — Phosphor Icons uses `SpinnerGap` instead
2. **Headless Chrome is feature-gated** — `cargo check --features headless` to verify
3. **`Browser::close()` removed in headless_chrome v1.x** — use `drop(h)` instead
4. **Tauri event listener can fail in browser** — wrapped in try-catch in `useCrawlEvents`
5. **Dashboard stats can be undefined** — use `?? 0` fallback for `toFixed` calls
6. **`AppState` uses `Arc`** — commands receive `State<'_, Arc<AppState>>`
7. **Job persistence is synchronous** — `std::fs` not `tokio::fs` in `state.rs`
8. **Parallel fetch uses `JoinSet`** — not manual `tokio::spawn` + `join_all`

## Common Pitfalls

| Symptom | Cause | Fix |
|---------|-------|-----|
| Blackscreen on startup | `listen` fails outside Tauri | Try-catch in `useCrawlEvents` |
| `Cannot read properties of undefined (reading 'toFixed')` | Stats not loaded | `?? 0` fallback |
| `cargo test` fails with "Zugriff verweigert" | Tauri dev process holds `.exe` | Stop dev server first |
| `Spinner` not found | Wrong Phosphor icon name | Use `SpinnerGap` |
| `Browser::close` not found | headless_chrome v1.x API change | Use `drop(h)` |

## File Locations

- Job persistence: `%APPDATA%/com.docurip.app/jobs/*.json`
- Settings: `%APPDATA%/com.docurip.app/settings.json` (via `tauri-plugin-store`)
- Crawl output: User-configurable (default: user-selected directory)

## Type Consistency

Rust ↔ TypeScript types must match:
- `CrawlJob` / `CrawlJob` interface
- `PageResult` / `PageResult` interface
- `JobStatus` enum variants: `queued`, `running`, `paused`, `completed`, `failed`
- `CrawlConfig` / `CrawlConfig` interface (note: `output_dir` in Rust → `outputDir` in TS via serde `camelCase`)
