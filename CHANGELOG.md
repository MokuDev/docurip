# Changelog

## v0.6.3 (unreleased)

### Added
- **Sitemap import & auto-discovery**: New Crawl now checks `robots.txt` and the well-known `/sitemap.xml` / `/sitemap_index.xml` locations 700 ms after a valid URL is entered (toggleable in Settings → Sitemap Discovery). When a sitemap is found, a banner offers to open a picker that fetches, parses (both `<urlset>` and `<sitemapindex>`, incl. gzipped `.xml.gz` and CDATA `<loc>`), filters (free-text + path-prefix), and imports selected URLs. Safety caps: 10 k URLs (result truncates rather than errors), 50 sub-sitemaps, depth 2, 50 MB response body, 30 s timeout, SSRF-protected. Selecting one URL fills the single-URL field; selecting many switches into Batch mode with the picks pre-filled.
- **Batch crawl mode**: New Crawl has a Single/Batch toggle. Batch mode takes one URL per line (with live count, duplicate/invalid detection, and a 500-URL cap), an optional name, and a per-batch on-failure override. `start_batch` spawns child crawls sequentially, each tagged with a `batchId` so history, export, and persistence work unchanged. The default on-failure behavior — Continue with next URL vs. Stop the batch — is set in Settings → Batch Crawls and can be overridden at launch. Backend adds `BatchJob`, `BatchRunner`, and `start_batch` / `stop_batch` / `get_batch` / `list_batches` / `delete_batch` commands. The Live Monitor grows a Batch Progress bar over the child-crawl progress; batches survive a page reload via `sessionStorage`.
- **Batch grouping in History**: child jobs are collapsed under their parent batch card (name, `M/N URLs`, status, progress, on-failure mode). Deleting a batch also drops its child jobs. New "Batches only" filter option.

### Changed
- **`JsonStore<T>` unifies persistence for jobs, templates, and batches**: extracts the per-type `save_*_to_disk` / `delete_*_from_disk` / `RwLock<HashMap>` / init-from-directory machinery into a single generic helper keyed by a `HasId` trait. `AppState.persisted_jobs` was renamed to `AppState.jobs` (a `JsonStore<CrawlJob>`); `state.templates` and the new `state.batches` follow the same shape. The `persist_job` / `persist_template` methods stay as thin wrappers so orchestrator call sites don't need to move. Closes the "Template/job persistence duplication" cleanup item from the ROADMAP.
- **`start_crawl` internals extracted into a `spawn_crawl` helper** shared by the single-URL command and the batch runner, so both paths produce identical bookkeeping.

## v0.6.2 (2026-07-11)

### Added
- **Job templates**: save the current New Crawl form (URL + full config) as a named template from a new `TemplateBar`, then re-apply or delete it later. Persisted as JSON files under `templates/` in the app data dir, mirroring the existing job-persistence pattern. New `list_templates` / `save_template` / `delete_template` commands.
- **Re-crawl with same settings**: History gets a "Crawl again" action on completed, failed, and cancelled jobs — jumps to New Crawl with the job's original URL and full config pre-filled via a new `prefillConfig` prop.
- **Auto-export after crawl**: new `autoExportFormat` setting (Settings → Auto-Export) runs the existing export pipeline automatically against a job's `formats/` directory the moment it completes. Triggered from the same terminal-event handler that already fires desktop notifications, reusing one `get_settings`/`get_job` round trip for both.
- **Configurable keyboard shortcuts**: shortcuts are now driven by a central action registry (`SHORTCUT_ACTIONS`) instead of being hardcoded. Added four new default bindings for tab navigation — `Ctrl/Cmd+D` Dashboard, `Ctrl/Cmd+H` History, `Ctrl/Cmd+,` Settings, `Ctrl/Cmd+I` Import — alongside the existing New/Active Crawl and Search shortcuts. Every nav item now shows its shortcut hint (previously only "New Crawl" did).
- **Keyboard Shortcuts settings section**: new Settings → Keyboard Shortcuts panel lists every shortcut with its current binding. Click a binding to rebind it — press any key combination to capture it live. Conflicting bindings are detected and rejected with an inline message naming the other action; Escape cancels editing without side effects. A reset button reverts a rebound action to its default. Bindings persist via the existing `AppSettings` store (`shortcutOverrides`) and take effect app-wide immediately on save.
- **Settings sub-navigation**: replaced the single endless column of Settings sections with a category sidebar — General (Appearance + Notifications), Shortcuts, Crawling & Export (Crawl Defaults + Auto-Export), Network & Storage (Network + Output + Window). Only the selected category renders, so the page no longer requires scrolling to reach settings near the bottom. No behavior changes — same fields, same handlers, purely a layout restructure informed by 3 mocked-up alternatives (sub-nav, grouped grid, quick-bar + accordion).

### Fixed
- **Shortcut capture leaking to global handlers**: while capturing a new key combination in the Settings rebind UI, the keypress was also being picked up by the app-wide shortcut listener (e.g. capturing `Ctrl+N` would both flag the conflict *and* navigate to New Crawl, closing the settings page). The row's key-capture handler now calls `stopPropagation()` so the global `document`-level listener never sees the event.
- **`CrawlJob.config` typed with a `url` field it never has at runtime**: the frontend `CrawlConfig` type includes `url`, but the backend's `CrawlConfig` struct — and therefore every `job.config` returned over IPC — never carries one (a job's URL only ever lives at `job.url`). Introduced a `TemplateConfig` type (`Omit<CrawlConfig, 'url'>`) matching what the backend actually returns and retyped `CrawlJob.config` to it, so any future `job.config.url` access is now a compile error instead of a silent `undefined`.
- **Low-contrast micro-labels app-wide**: the `charcoal` text token (used for ~124 uppercase labels — `TopStatusBar`, `SystemStatusBar`, Dashboard stat cards, Settings field labels, etc.) only cleared ~3.7:1 contrast against card surfaces in dark mode and ~4.6:1 in light mode, both at or below WCAG AA's 4.5:1 minimum for the 10-11px text it's typically used at. Lightened dark-mode `charcoal` to `rgb(130 146 168)` (~5.6:1 vs. card surfaces, ~6.2:1 vs. the page background) and darkened light-mode `charcoal` to `rgb(80 96 116)` (~6.4:1 vs. white, ~6.1:1 vs. the page background) — comfortably past AA, while staying visibly more muted than the `secondary` token (~7:1) so the two-tier text hierarchy still reads. Also bumped the smallest (10px) status-bar labels — Session/Uptime/Jobs, CPU/RAM/Output — to `font-medium` for extra legibility at that size.
- **Result Browser crashes to a white screen when clicking a folder**: `ResultTree` tracked *expanded* paths in a set where "empty set" meant "everything expanded" as a special case. The moment any single folder was toggled, that special case stopped applying, so every other branch — including the clicked folder's own ancestors — collapsed at once. The resulting sudden, large shrink in visible rows left `focusedIndex` pointing far past the new list length; a `useEffect` then called react-window's `scrollToRow` with that stale, out-of-bounds index, which throws a `RangeError`. With no `ErrorBoundary` anywhere in the app, the uncaught error unmounted the entire UI. Fixed by inverting the model to track *collapsed* paths instead (toggling one folder no longer affects unrelated branches — normal file-explorer behavior) and by merging the clamp-and-scroll effects into one, so an out-of-bounds `focusedIndex` is always corrected before `scrollToRow` can be called with it.

### Changed
- **`start_crawl` / `save_template` payload construction unified**: both now go through a shared `toBackendConfig()` helper in `NewCrawl.tsx` instead of duplicating the field-by-field mapping (trim/filter patterns, normalize `pathPrefix`), closing out a standing ROADMAP item.

## v0.6.1 (2026-07-09)

### Added
- **Include patterns & path prefix filter**: new `includePatterns` (regex list) and `pathPrefix` fields on `CrawlConfig` allow whitelisting URLs during a crawl. When any include constraint is set, only URLs matching at least one include pattern or the path prefix are enqueued. Exclude patterns still override includes. UI fields added to New Crawl view; backend validation rejects malformed regex before the crawl starts.
- **Keyboard shortcuts**: `Ctrl/Cmd+N` opens New Crawl (or Active Crawl when running), `Ctrl/Cmd+F` focuses the search input, `Escape` closes the topmost modal or Live Console. Shortcuts suppress inside text inputs (except Escape). New `useKeyboardShortcuts` hook and `EscapeStack` context for coordinating Escape across nested modals.
- **Desktop notifications**: system notifications fire when a crawl completes or fails. Gated by a new `notificationsEnabled` setting (default on) with a toggle in Settings → Notifications. Uses `tauri-plugin-notification` with permission request on first use.

### Fixed
- **ResultTree focusedIndex out of range**: keyboard-focused index could exceed the visible node count after filtering or collapsing a folder, causing undefined access. Now clamped via `useEffect` whenever `visibleNodes` shrinks.
- **Duplicate desktop notifications**: if the backend emitted multiple terminal events for the same job (e.g. race between completed/failed), the notification fired more than once. A ref-backed `Set<string>` now deduplicates per jobId.
- **Notification plugin errors unhandled**: `sendNotification` was not wrapped in try/catch — if the plugin threw after permission was granted, the rejection propagated as unhandled. Now caught and logged.
- **Whitespace-only pattern lines sent to backend**: include/exclude/selector textareas split on newlines but only filtered with `.filter(Boolean)`, so a line of spaces passed through as a non-empty (invalid) regex. Lines are now `.trim()`'d before filtering.
- **Keyboard shortcuts case-sensitive**: `Ctrl+N`/`Ctrl+F` matched `e.key === 'n'`/`'f'` literally, so they failed with CapsLock on. Now normalized via `.toLowerCase()`.
- **ResultTree rowProps invalidating memoization**: `rowProps={{}}` created a fresh object every render, defeating react-window's row memoization. Hoisted to a module-level constant.
- **`headlessStrategy` cast to `any`**: select onChange used `as any` to bypass TypeScript, hiding potential type mismatches. Now uses the proper `CrawlConfig['headlessStrategy']` union type.
- **`pathPrefix` submitted with query/fragment**: user-entered path prefixes containing `?` or `#` would never match (the backend compares against `url.path()` only). Now normalized on both frontend submit and backend `start_crawl`: trim whitespace, strip query/fragment, enforce leading `/`.
- **`listRef` typed as `any` with wrong API calls**: ResultTree used `useRef<any>` and react-window v1's `scrollToItem` method. Replaced with react-window v2's `useListRef` hook (properly typed `ListImperativeAPI`) and `scrollToRow` API. Fixes the only TypeScript error in the codebase.

## v0.6.0 (2026-07-08)

### Added
- **Dark / Light / System theme toggle**: the app was previously dark-only with no theme infrastructure. Semantic Tailwind tokens (`deepVoid`, `surface`, `abyssal`, `ghost`, `smooth`, `secondary`, `charcoal`) now resolve through CSS variables that flip based on a `.dark`/`.light` class on `<html>`, so existing component styling didn't need to change. New `ThemeProvider`/`useTheme()` hook persists the preference via a dedicated `set_theme` command; `system` mode tracks `prefers-color-scheme` live. Quick-access toggle in the top status bar; full picker in Settings → Appearance.

### Fixed
- **Unreadable button text in light mode**: `text-deepVoid` was reused as guaranteed-dark label text on `accentGreen`/`amber` buttons (Save, Start Crawl, dashboard CTA); since `deepVoid` now flips to near-white in light mode, those labels went nearly invisible. Switched to a fixed dark color for button labels instead of a theme-aware token.
- **Low-contrast secondary text/badges in light mode**: darkened the light-mode text scale so `charcoal` (the most-muted token, used widely for hints and status badges) clears WCAG AA contrast against the near-white backgrounds.
- **Theme-save race**: `useTheme`'s `setTheme` previously did its own `get_settings`→`update_settings` round trip, which could race with the Settings page's own save/reset flow and silently clobber either the theme or unrelated field edits (the backend does a full overwrite, not a merge). Replaced with a dedicated `set_theme` command that only touches the `theme` key; Settings now reads the live theme from context at save time instead of a locally-mirrored copy, and persistence failures now surface a toast instead of failing silently.

## v0.5.2 (2026-07-04)

### Added
- **HTML export format**: Export crawled documentation as styled HTML files (individual or merged). Uses pulldown-cmark to convert Markdown to HTML with embedded CSS styling. Added `HtmlFiles` and `MergedHtml` variants to `ExportFormat`.
- **Virtualized ResultTree**: Tree view in ResultBrowser now uses react-window for efficient rendering of large result sets. Only visible nodes are rendered, improving performance with thousands of pages.
- **Lazy-loaded MarkdownPreview**: MarkdownPreview component is now code-split and loaded on demand, reducing initial bundle size by ~31KB.

## v0.5.1 (2026-07-01)

### Added
- **Advanced Markdown cleaning pipeline**: Comprehensive pre- and post-processing for cleaner output:
  - Pre-processing: strips `<script>` and `<style>` tags with content, removes empty `<a href="#"></a>` links before HTML-to-Markdown conversion
  - Extended boilerplate detection: filters cookie banners ("we use cookies"), newsletter signups ("Subscribe to our newsletter"), and copy-code buttons in addition to existing UI elements
  - Post-processing: collapses excessive blank lines (3+ → 2), removes empty link syntax `[text]()`, and strips broken image references `![alt]()`

## v0.5.0 (2026-06-28)

### Added
- **PDF/EPUB import**: Import PDF and EPUB files into Markdown with automatic image extraction. New Import view with drag & drop file picker. Backend modules `importer/pdf.rs` and `importer/epub.rs` handle extraction via `pdf_extract` and `epub` crates.
- **JSON export format**: Export crawled documentation as structured JSON files (individual or merged). Each entry includes `title`, `url`, `content`, and `meta` fields. Added `JsonFiles` and `MergedJson` variants to `ExportFormat`.
- **Text cleaner for imports**: Configurable pipeline that strips headers, footers, page numbers, footnotes, and boilerplate from imported PDFs/EPUBs. Uses cross-page frequency analysis for header/footer detection, sequential number detection for page numbers, and zone-restricted pattern matching. Toggle in Import UI, enabled by default.
- **Tauri native drag & drop**: Import view uses Tauri's native file drop event instead of HTML5 drag & drop for more reliable file handling.
- **Auto content extraction in crawler**: When no CSS selectors are configured, the crawler now tries common content selectors (`main`, `article`, `[role="main"]`, `#content`, `.content`, etc.) before falling back to the full HTML body. Prevents nav, sidebar, and footer content from polluting the Markdown output.
- **Markdown deduplication**: Post-processing step in `HtmlToMarkdown` removes duplicate text blocks (>80 chars) that appear multiple times in converted output, eliminating repeated content from pages with duplicated DOM structures.

### Fixed
- **JSON export title extraction**: Now detects both ATX-style (`# Heading`) and setext-style (`Heading\n===`) Markdown headings. Previously only ATX headings were detected, causing filenames like `getting-started` to appear as the title instead of the actual heading text.
- **Clean text toggle not working**: Fixed click area (onClick was only on the small track, not the label) and stale closure in drag & drop handler (useEffect captured initial state; now uses useRef to always read current value).
- **TextCleaner ineffective on real PDFs**: Pre-trims excessive blank lines from `pdf_extract` output, uses non-blank-line-aware zone indexing, expanded detection zones to 5 lines, added sequential page number detection across pages, and caps effective zone at half page height to avoid false positives on short pages.
- **UI boilerplate in crawled output**: Strips "Copy page", "Open markdown", "Edit page" text that leaks from documentation site UI buttons into the Markdown/JSON output.
- **TOC navigation polluting output**: Detects and removes anchor-link table-of-contents sections (lists of `[Section](#anchor)` links), including full-path variants like `/docs/page#section`.
- **Trailing heading stubs**: Removes repeated heading-only blocks (no content between them) that appear at the end of pages from sidebar/mobile navigation elements. Handles both ATX (`## Heading`) and setext (`Heading\n----------`) formats, plus short fragments like "💡Tip" interspersed between stubs.

## v0.4.2 (2026-06-28)

### Added
- **"Active Crawl" navigation item**: New sidebar entry appears when a crawl is running, allowing access to crawl controls (pause/cancel) from any view. Dashboard remains accessible during active crawls.
- **Default settings update**: `default_page_limit` increased from 50 to 1000, `request_delay` reduced from 1000ms to 750ms (25% faster).

### Fixed
- **`activeJobIds` tracking bug**: Fixed stale "RUNNING" badge issue — `activeJobIds` now only updates on `jobStatusChanged` events. Jobs are correctly removed from active set when status is `completed`, `failed`, or `cancelled`. Previously, all non-status events incorrectly added the jobId to the active set.
- **`NewCrawlView` state loss**: Active crawl state is now persisted in `sessionStorage` and restored on mount — switching between Dashboard and Active Crawl no longer loses the Live Monitor or controls. Paused jobs are now correctly restored (previously only `running`/`queued` were handled).
- **Duplicate sidebar entries**: "New Crawl" and "Active Crawl" are now mutually exclusive — only one appears at a time based on whether a crawl is running.
- **LiveConsole close button removed**: Removed non-functional "X" close button, leaving only "Clear" and "Minimize" controls.
- **Vitest test suite expanded**: 11 frontend unit tests now pass (useToasts: 6 tests for push, dismiss, auto-dismiss, error persistence, multiple types, correct removal; useCrawlEvents: 5 tests for initial state, event emission, active job tracking, 500-event cap, and non-status event isolation).
- **LiveConsole close button removed**: Removed the "X" close button from the LiveConsole header — only "Clear" and "Minimize" remain, as closing the console while a crawl is active was confusing and rarely useful.

## v0.4.1 (2026-06-28)

### Added
- **Queue backpressure**: `MAX_QUEUE_SIZE = 50_000` limit with warning event when queue reaches capacity — prevents unbounded memory growth during aggressive crawls.
- **Vitest test suite**: 8 passing frontend unit tests covering `useToasts` (push, dismiss, auto-dismiss, error persistence) and `useCrawlEvents` (event handling, active job tracking, 500-event cap). Includes `vitest.config.ts`, `src/test/setup.ts`, and test files for both hooks.

### Fixed
- **Phosphor icon `title` prop removed**: `title` attribute is not supported by `@phosphor-icons/react` — removed from all icon instances in `LiveConsole.tsx` to fix TypeScript build error.

## v0.4.0 (2026-06-28)

### Added
- **Full-text search in ResultBrowser**: backend-powered content search via `search_job_results` — reads `.md` files from disk with relevance scoring and preview snippets. Triggered at 3+ characters with 300ms debounce.
- **ErrorKind icons in LiveConsole**: error events now display visual indicators for error type — red disk icon for `Disk`, orange cloud for `Network`, file-x for `Parse`, stop sign for `RobotsBlocked`, and generic warning for `Unknown`.

### Changed
- **`job.results` keeps only metadata in RAM**: `PageMeta` now stores URL, title, HTTP status, and link count — Markdown content no longer sits on the heap. Eliminates O(n·content) RAM growth during large crawls.
- **Persist throttling**: `persist_job` is no longer invoked after every page; it runs at most every 50 pages or 10 seconds, whichever comes first.
- **Introduced `JobStatus::Cancelled`**: cancelled jobs now report status `Cancelled` instead of `Failed`.
- **`ErrorKind` for typed error classification**: `CrawlEvent::Error` now carries `kind: ErrorKind` (`Network` / `Disk` / `Unknown`), laying the groundwork for differentiated error display in the frontend.

### Fixed
- **Blocking `tokio::select!` in crawl loop**: `select! { _ = persist_interval.tick() => true, else => false }` blocked each loop iteration for up to 10 seconds because `_ = future` always matches and `else` never fires. Replaced with a non-blocking `Instant::elapsed()` check — resolves a ~7× throughput regression.


## v0.3.4 (2026-06-27)

### Added
- **MIME-Type validation for asset downloads**: `HttpFetcher::fetch_bytes` checks `Content-Type` against an allow-list (images, fonts, CSS, JS, JSON, PDF, audio/video, octet-stream) and rejects `text/html`/`application/xhtml+xml` so that error pages or login redirects served at asset URLs no longer get persisted as broken images/stylesheets.
- **SSRF check on the start URL**: `validate_crawl_input` now runs `crawler::ssrf::is_private_target` on the submitted URL when `ssrf_protection` is enabled, returning an actionable error before the crawl is even spawned. Previously SSRF was only enforced on follow-up links during the crawl.
- **`useUpdater.error` rendered in the update banner**: the update banner now shows the captured error message and switches the action button label from "Install & Restart" to "Retry" when a previous install attempt failed.

### Changed
- **User-Agent unified to `Docurip/0.3.3`** in `HttpFetcher` (`fetcher/http.rs`) and the default `AppSettings.user_agent` (`settings/config.rs`); previously both still advertised `Docurip/0.3.1`.
- **Dashboard stats polling throttled**: stats refresh every 3 s while at least one crawl is active, otherwise every 4th tick (~12 s). Job list and recent exports continue to poll every 3 s. Keeps the live-stats UX from v0.3.2 during crawls while reducing idle backend load.
- **NewCrawl logs migrated to `useRef`**: log entries are appended into a ref-backed array (mutated in place, capped at 500); a `logTick` counter drives re-renders. Avoids per-append array-copy that grew quadratic on long crawls.
- **`walk_dir` is now `pub` in `export.rs`** and `commands::export_job_zip` calls `export::zip_directory` instead of its own inline `add_dir_to_zip` recursion. Eliminates ~25 lines of duplicated ZIP-walking logic.
- **Error classification uses typed downcasts**:
  - `crawler::orchestrator::is_disk_error` now walks the `anyhow::Error` chain looking for `std::io::Error` and matches on `ErrorKind::PermissionDenied` / `StorageFull` / `ReadOnlyFilesystem`. The original substring-based logic is kept as `is_disk_error_str` and used as a fallback only when no `io::Error` is present in the chain.
  - `HttpFetcher::is_transient_error` now downcasts to `reqwest::Error` and uses `is_timeout()` / `is_connect()` / `is_request()`. Substring matching remains as a fallback for non-reqwest errors.

### Fixed
- **Headless feature build**: `tab.close()` now passes the required `fire_unload: bool` argument (`tab.close(false)`) so `cargo check --features headless` succeeds against headless_chrome 1.x. Previously failed to compile in the headless build.

### Tests
- 2 new tests for `is_disk_error`: classifies `io::ErrorKind::PermissionDenied`/`StorageFull`/`ReadOnlyFilesystem` via cause chain; falls back to string matching when no `io::Error` is present.
- 1 new test for `is_allowed_asset_mime` covering images, fonts, CSS, JS, JSON, PDF, audio/video, octet-stream, charset suffixes, empty content-type, and rejection of `text/html`/`application/xhtml+xml`.


## v0.3.3 (2026-06-14)

### Added
- **Window size setting**: new dropdown in Settings → Window with 5 presets (1280×900 Compact, 1600×1000 Standard, 1920×1080 Full HD, 2560×1440 QHD, 3840×2160 UHD/4K). Selection applies immediately — the window resizes and centers on the current monitor without restart. Persisted across sessions via `tauri-plugin-store`. On startup, the saved size is applied before the window becomes visible. Oversized selections (e.g. UHD on a 1080p display) are clamped to the available monitor dimensions with a toast notice. Minimum window size constraint of 1280×900 enforced via `tauri.conf.json`.

### Fixed
- **Dashboard stats showing zero**: `DashboardStats` struct was missing `#[serde(rename_all = "camelCase")]`, so the backend sent `pages_saved` but the frontend expected `pagesSaved` — all fields were `undefined` and fell back to `0`.
- **Recent Exports always empty**: `list_recent_exports` scanned the nonexistent `app_data_dir/exports/` directory. Exports are actually written to `{outputDir}/zip/`. Rewrote the function to accept a list of job output dirs and scan each `{dir}/zip/` subfolder. `list_exports` command now collects unique output dirs from all active + persisted jobs.

### Changed
- Removed unused `Manager` import from `commands.rs`.


## v0.3.2 (2026-06-14)

### Added
- **Auto-organized output folders**: every crawl now creates three subfolders under the global output directory: `{outputDir}/{targetName}/main/` for crawled content, `{outputDir}/{targetName}/zip/` for exported ZIPs, and `{outputDir}/{targetName}/formats/` for format exports (MD files, PDF files, merged variants). Folders are created automatically when a crawl starts — no manual setup required.
- **targetName extraction**: the subfolder name is derived from the crawl URL's domain (e.g. `docs.example.com`), keeping results organized by site.
- **Simplified ExportModal**: export destination is now fully automatic — the format picker is all you need. ZIP exports land in the job's `zip/` subfolder; all other formats (Markdown files, PDF files, merged MD, merged PDF) land in the `formats/` subfolder. No more manual folder picker step.
- **"Open folder" opens main/ subfolder**: History and ResultBrowser "Open output folder" buttons now open the `main/` subfolder directly, showing the crawled content instead of the parent directory.
- **Live dashboard stats**: dashboard stats (pages saved, total size, crawl velocity, fail rate) now update in real-time during active crawls, not just after completion.
- **Animated stat counters**: stat cards use a smooth count-up animation with ease-out cubic interpolation when values change.

### Changed
- **Dashboard stats cache removed**: the 30-second cache TTL that served stale zeros during active crawls has been eliminated. Stats are computed fresh on every 3-second poll.
- **`collect_all_jobs` now async**: uses `.read().await` instead of `try_read()` on both `active_jobs` and `persisted_jobs` RwLocks, so active jobs are no longer silently skipped when the orchestrator holds a write lock.
- **Crawl velocity includes active jobs**: velocity is now computed from wall-clock time (`Utc::now() - start_time`) for running jobs, falling back to `end_time - start_time` for completed jobs.
- **Total size includes active job output**: `total_size_bytes` now sums output directories for all jobs (active + completed), not just completed ones.
- **`compute_velocity` extracted**: velocity logic moved to a dedicated function that handles both running and completed jobs.
- **Output directory setting moved to Settings only**: the per-crawl output directory picker in New Crawl has been removed. The output directory is configured once in Settings and applies to all crawls, reducing configuration friction and ensuring a consistent folder structure.
- **Export commands use subfolder structure**: `export_job` and `export_job_zip` now read crawled content from `{outputDir}/main/` and write ZIPs to `{outputDir}/zip/`. `export_job_v2` auto-derives the destination to `{outputDir}/formats/` when no explicit destination is provided, with the `destination` parameter becoming optional.
- **`resolve_output_dir` simplified**: generates `{baseDir}/{domain}` (no date/id suffix), matching the cleaner subfolder structure.


## v0.3.1 (2026-06-14)

### Added
- Tests: 3 regression tests for URL-to-path query-string stripping in `writer/fs.rs` (verifies `?query` and `#fragment` are stripped from filenames)
- **Logs memory cap**: `NewCrawl.tsx` now caps log entries at 500, preventing unbounded memory growth during long crawls
- **Search debounce**: `ResultSearch.tsx` debounces input by 200ms to reduce re-rendering of ResultTree and MarkdownPreview during typing
- **Dashboard error logging**: empty `catch` blocks in `Dashboard.tsx` now emit `console.warn` with context for debugging

### Fixed
- **Startup crash**: `AppState::init()` called `Handle::current().block_on()` before Tauri starts its Tokio runtime, causing an immediate panic; reverted to synchronous `std::fs` for the one-time startup load
- **prefillUrl re-trigger**: removed the `if (prev.url) return prev` guard in `NewCrawl.tsx` so quick-start URLs are always applied, even after the user has manually edited the URL field


## v0.3.0 (2026-06-14)

### Added
- **Domain filtering (`stay_within_domain`)**: new config option (default `true`) that restricts the crawler to links within the same domain as the start URL; implemented in `orchestrator.rs` URL queue with checkbox in New Crawl and Settings views
- **robots.txt enforcement (`respect_robots_txt`)**: new `robots.rs` module that fetches and parses `robots.txt` from the target site, honoring `User-agent`, `Disallow`, `Allow`, and `Crawl-delay` directives; enforced as the first URL-level filter in the crawl loop
- **SSRF protection (`ssrf_protection`)**: new `ssrf.rs` module that detects and blocks requests to private/internal IP addresses (loopback, link-local, RFC 1918, IPv6 ULA) via IP-literal checks, known-local hostname patterns, and DNS resolution; configurable per-crawl with checkbox UI
- 10 unit tests for SSRF detection covering IPv4 private ranges, IPv6 loopback/ULA, localhost variants, `.local` TLD, and public-host passthrough

### Security
- **XSS prevention in MarkdownPreview**: added DOMPurify sanitization after Markdown-to-HTML rendering and after search-query highlighting; restricted allowed tags/attributes and blocked `javascript:` URIs in links
- **CSP hardened**: removed `'unsafe-inline'` from `script-src`; set `withGlobalTauri: false` so the Content Security Policy now actually blocks injected inline scripts

### Changed
- `useCrawlEvents`: migrated from `window.__TAURI__.event.listen` back to typed `listen()` from `@tauri-apps/api/event` with proper async cleanup and TypeScript-safe event payloads
- **Async disk I/O**: `save_job_to_disk`, `load_job_from_disk`, `load_all_jobs`, and `delete_job_from_disk` in `state.rs` now use `tokio::fs` instead of blocking `std::fs`, preventing the crawl runtime from stalling on disk operations; `AppState::init()` bridges the sync/async boundary via `Handle::current().block_on()`
- `persist_job` and `remove_persisted_job` now `.await` the underlying async disk operations
- **System stats caching**: `system.rs` now uses a `LazyLock<Mutex<System>>` singleton instead of creating a new `sysinfo::System::new_all()` on every 2-second poll, reducing allocation overhead and improving accuracy of CPU readings
- **Asset download size limit**: `fetch_bytes` now checks the `content-length` header before consuming the response body, rejecting assets larger than 50 MB to prevent memory exhaustion from adversarial or oversized downloads
- **Dashboard polling merged**: three separate `useEffect` intervals (jobs 3s, stats 3s, exports 5s) in `Dashboard.tsx` consolidated into a single 3s interval
- **Parallel asset downloads**: `orchestrator.rs` asset-download loop replaced with `tokio::task::JoinSet` so all assets per page download concurrently instead of sequentially
- **Shared StatusBadge**: extracted `StatusIcon` and `StatusBadge` into `src/components/StatusBadge.tsx`; `Dashboard.tsx`, `History.tsx`, and `NewCrawl.tsx` now import the shared component instead of duplicating local versions
- **LiveConsole event processing**: fixed event loss by switching from `events[events.length - 1]` to index-based tracking with `lastProcessedIdx` ref, ensuring all events between renders are processed
- **History loading flicker**: `loadJobs` now accepts a `showSpinner` parameter; background polls skip `setLoading(true)` so the spinner only appears on initial load

### Fixed
- **UTF-8 panic in search preview**: `extract_preview` now uses `char_safe_start`/`char_safe_end` helpers to find valid UTF-8 character boundaries before slicing, preventing panics on non-ASCII content (e.g. German umlauts, CJK characters)
- **Panic-safe CSS selector**: replaced `unwrap()` on hardcoded `a[href]` selector in `DomParser` with `expect()` carrying an explanatory message
- **ZIP export path error**: `export_job_zip` now propagates an explicit error when `output_dir` has no parent directory instead of silently falling back via `unwrap_or`
- **Silent polling failure in New Crawl**: `get_job` polling now tracks consecutive errors via a ref; after 3 consecutive failures the interval is cleared and the job status is set to `failed` so the UI reflects the broken state instead of freezing silently
- **Invalid exclude patterns ignored silently**: `validate_crawl_input` now validates each exclude pattern before the crawl starts and returns an actionable error if any pattern is malformed; `Orchestrator::new` propagates the error via `?` instead of discarding it with `.ok()`
- **`stay_within_domain` was defined but never enforced**: `CrawlConfig.stay_within_domain` existed in the struct since v0.2.x but had no filtering logic — links to external domains were silently followed; now correctly skipped when enabled
- **`respect_robots_txt` was defined but never enforced**: same issue — the config field existed without any parsing or checking; now actively fetched and consulted


## v0.2.4 

### Added
- Dashboard stats expanded to 4 metric cards: Pages Saved, Total Size, Crawl Velocity (pages/min), Fail Rate
- Recent Exports panel on Dashboard — lists exported ZIPs (name, size, date), click-to-reveal in file manager
- `list_exports` command + `exports.rs` helper scanning `app_data_dir/exports/`
- Top status bar: session ID (short hex) + live uptime counter
- Bottom system status bar: live CPU%, RAM used/total, active output path
- `get_system_stats` / `get_session_info` commands via new `system.rs` (sysinfo-based)
- Global toast container (bottom-left, max 3 visible, auto-dismiss after 6s except errors)
- Dependencies: `sysinfo = "0.31"`, `uuid` (v4 feature)

### Changed
- `useCrawlEvents` now pushes `error` events into the global toast system
- Main layout adjusted (`h-[calc(100vh-44px)]`) to accommodate top/bottom bars


## v0.2.3 

### Added
- Pause/Resume for active crawl jobs: `pause_crawl` / `resume_crawl` commands, `should_pause` flag + `Notify`-based resume signal on `CrawlHandle`
- Pause/Resume buttons in New Crawl view (orange Pause / green Resume / red Cancel)
- Disk-error auto-pause: write failures (permission denied, no space, read-only) now pause the job with an actionable hint to fix the output path and resume, instead of marking it failed
- E2E test (`tests/e2e_crawl.rs`) using a wiremock static-site fixture (index + 2 sub-pages + image), verifying full crawl writes Markdown + assets with rewritten relative paths
- Dependency: `wiremock = "0.6"` (dev)

### Changed
- Orchestrator main loop now checks `should_pause` before `should_stop`, persists job state as `Paused`, aborts in-flight tasks gracefully, and resumes cleanly back to `Running`


## v0.2.2 

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


## v0.2.1 

### Added
- `dirs` crate for cross-platform home directory resolution
- `event_bus.register()` typed wrapper returning `(&Self, broadcast::Receiver)` with `Receiver::start()`


## v0.2.0 

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


## v0.1.0

### Initial
- Tauri v2 desktop app
- Documentation crawler with HTTP and headless Chrome fetcher
- DOM parsing, HTML-to-Markdown conversion, filesystem writer
- Dashboard, New Crawl, History, Settings views
- Job persistence (disk-backed JSON)
- ZIP export
