# Docurip Documentation

## General Information

Docurip is a Tauri v2 desktop application for crawling documentation websites and converting them into offline Markdown archives. It combines a Rust backend (high-performance crawling, security filtering, and disk I/O) with a React 19 / TypeScript / Tailwind CSS frontend.

- **Current version:** v0.6.2
- **Platforms:** Windows (primary target), built with Tauri v2
- **Architecture:** Rust backend + React 19 frontend
- **Default output directory:** `~/.docurip` (falls back to `./output` if the home directory cannot be resolved)
- **Data storage:** All job metadata and application settings are stored locally on your machine. No cloud service is used.

## Detailed Features

### Crawling Engine

Docurip’s crawler is built around a parallel, queue-based fetch loop:

| Capability | Description |
|------------|-------------|
| **HTTP fetcher** | `reqwest`-based fetcher with a configurable timeout, connection pooling, and a User-Agent header. |
| **Automatic retry** | Retries transient failures (HTTP 5xx, 408, timeouts, connection errors, DNS issues) up to 3 times with exponential backoff (1 s base delay). Permanent errors (4xx, malformed responses) fail immediately. |
| **Headless Chrome** | Optional feature-gated fetcher using `headless_chrome`. Strategies: `never` (HTTP only), `auto` (fallback to headless when HTTP returns empty/non-success), `always`. PDF exports also require headless support. |
| **Parallel execution** | Pages are fetched concurrently up to the configured concurrency limit using a semaphore and a `JoinSet` for task management. |
| **Pause / Resume / Cancel** | Active jobs can be paused, resumed, or cancelled from the live monitor. Paused jobs persist their state to disk. |
| **Disk-error auto-pause** | If a write fails because of permission denied, read-only filesystem, or no space left, the crawl is paused automatically with an actionable error so you can fix the output path and resume. |
| ** robots.txt support** | Fetches and parses `/robots.txt`, honoring `User-agent`, `Disallow`, `Allow`, and `Crawl-delay` directives for the configured user agent. |
| **SSRF protection** | Blocks private/internal targets: loopback IPs, RFC 1918 ranges, link-local, IPv6 ULA, `localhost`, `*.local`, and hostnames that resolve to private addresses. |

### Crawl Profiles

Since v0.5.0, Docurip includes pre-configured crawl profiles that set sensible defaults for common documentation types. Select a profile in the **New Crawl** view to auto-fill `maxDepth`, `pageLimit`, `respectRobotsTxt`, content selectors, and exclude patterns.

| Profile | Max Depth | Page Limit | Robots.txt | Best for |
|---------|-----------|------------|------------|----------|
| **API Docs** | 3 | 500 | Yes | Swagger, OpenAPI, typedoc, Redoc |
| **Wiki** | 4 | 2,000 | Yes | MediaWiki, GitBook, Notion-published wikis |
| **Blog** | 2 | 100 | No | Static blogs, article collections |
| **Documentation** | 3 | 1,000 | Yes | Docusaurus, MkDocs, Sphinx, VitePress |
| **Custom** | 2 | 1,000 | Yes | Manual configuration — no presets applied |

Selecting a profile sets the initial values; you can still adjust any field before starting the crawl.

### Crawl Configuration

Each crawl job has its own configuration. Defaults are taken from **Settings** or a **Crawl Profile** unless overridden when starting a crawl.

| Field | Type / Range | Default | Description |
|-------|--------------|---------|-------------|
| `url` | URL | — | Start URL. Must use `http://` or `https://`. |
| `maxDepth` | 1–10 | 2 | Maximum link depth to follow from the start URL. |
| `pageLimit` | 1–10,000 | 1,000 | Hard cap on pages to crawl. The crawler stops spawning new tasks once the limit is reached; in-flight tasks may finish, so the final count can slightly exceed the limit. |
| `downloadAssets` | boolean | false | Download images, stylesheets, and scripts referenced by crawled pages and rewrite their URLs to local relative paths. |
| `headlessStrategy` | `never` / `auto` / `always` | `auto` | When to use headless Chrome for fetching pages. |
| `contentSelectors` | string[] | `["main", "article", ".content"]` | CSS selectors used to extract only the relevant page content before Markdown conversion. If no selector matches, the full page HTML is converted. |
| `excludePatterns` | string[] | `[]` | Regex patterns. Any link matching one of these patterns is skipped. Invalid patterns are rejected when the crawl starts. Exclude patterns always take precedence over include patterns. |
| `includePatterns` | string[] | `[]` | Regex patterns. Since v0.6.1: when any include pattern (or `pathPrefix`) is set, only links matching at least one of them are enqueued — everything else is skipped. Leave empty to crawl all non-excluded links. |
| `pathPrefix` | string | `""` | Since v0.6.1: a simple path prefix filter (e.g. `/docs/api/`) that admits URLs the same way `includePatterns` does, without needing a regex. Normalized on submit — a leading `/` is enforced and any query string or fragment is stripped. |
| `respectRobotsTxt` | boolean | true | Honor the target site’s `robots.txt`. |
| `stayWithinDomain` | boolean | true | Only follow links whose host matches the start URL. |
| `ssrfProtection` | boolean | true | Block private/internal addresses. |
| `outputDir` | path | `~/.docurip` | Root output directory for all crawls (configured globally in Settings). |

### Application Settings

Settings are persisted across sessions in `%APPDATA%/com.docurip.app/settings.json` via `tauri-plugin-store`.

| Setting | Default | Range / Notes |
|---------|---------|---------------|
| Output directory | `~/.docurip` | Directory where all crawl subfolders are created. |
| Concurrency | 3 | 1–20 parallel fetch tasks. |
| Request delay | 750 ms | 0–30,000 ms delay before each request. |
| Timeout | 30,000 ms | 1,000–120,000 ms per HTTP request. |
| User agent | `Docurip/0.6.2 (Documentation Crawler)` | Sent with every request. |
| Default max depth | 2 | Default for new crawls. |
| Default page limit | 1,000 | Default for new crawls. |
| Default download assets | false | Default for new crawls. |
| Default headless strategy | `auto` | Default for new crawls. |
| Default respect robots.txt | true | Default for new crawls. |
| Default stay within domain | true | Default for new crawls. |
| Default SSRF protection | true | Default for new crawls. |
| Window size | 1280×900 | Presets: 1280×900, 1600×1000, 1920×1080, 2560×1440, 3840×2160. Applied immediately and centered on the current monitor, clamped if larger than the display. |
| Theme | `system` | Since v0.6.0: `dark`, `light`, or `system` (follows the OS preference live). Quick-toggle in the top status bar; full picker in Settings → General. |
| Desktop notifications | on | Since v0.6.1: shows a system notification when a crawl completes or fails. Toggle in Settings → General. |
| Auto-export format | disabled | Since v0.6.2: when set, this export format runs automatically to the job's `formats/` directory the moment a crawl completes. Configured in Settings → Crawling & Export. |
| Keyboard shortcut bindings | built-in defaults | Since v0.6.1: every shortcut can be rebound via a live key-capture UI in Settings → Shortcuts. Conflicting bindings are rejected with an inline message; a reset button restores the default per action. |

Settings are organized into four categories in the UI: **General** (theme, notifications), **Shortcuts** (keyboard bindings), **Crawling & Export** (crawl defaults, auto-export), and **Network & Storage** (timeout, user agent, output directory, window size).

### Security & Safety

Docurip includes several protections so it can be run safely on public or internal networks:

- **SSRF protection** – Prevents the crawler from requesting loopback, RFC 1918, link-local, IPv6 ULA, `localhost`, `*.local`, and hostnames resolving to private IPs. Applied to the start URL at validation time and to every discovered link during the crawl.
- **robots.txt enforcement** – Respects the target site’s crawler rules before following links.
- **Domain restriction** – `stayWithinDomain` limits crawling to the start URL’s host.
- **Asset MIME-type allow-list** – Downloaded assets are checked against an allow-list (`image/*`, `font/*`, `text/css`, `text/javascript`, `application/javascript`, `application/json`, `application/pdf`, `application/octet-stream`, `audio/*`, `video/*`, etc.). `text/html` and `application/xhtml+xml` are rejected so login pages or error pages are not saved as broken assets.
- **Asset size cap** – Assets larger than 50 MB are rejected before the response body is consumed.
- **Path sanitization** – Filenames and paths are sanitized to block `.` and `..` segments, preventing directory traversal. Query strings and fragments are stripped from page filenames.

### Output Organization

Since v0.3.2, every crawl creates a subfolder named after the target domain:

```
{outputDir}/
  {domain}/
    main/      # Crawled Markdown files
    zip/       # ZIP exports
    formats/   # Markdown files, merged Markdown, PDF files, merged PDF, HTML files, merged HTML
```

For example, crawling `https://docs.example.com` with the default output directory produces `~/.docurip/docs.example.com/main/` containing the Markdown pages.

### Exports

Completed jobs can be exported from the **History** view or the **Result Browser**.

| Format | Description | Requirements |
|--------|-------------|--------------|
| **ZIP** | Bundles the entire `main/` Markdown archive into one `.zip` file. | Always available. |
| **Markdown Files** | Copies the `main/` Markdown tree to `formats/`. | Always available. |
| **Merged Markdown** | Concatenates all Markdown pages into a single file with `---` separators. | Always available. |
| **PDF Files** | Renders each Markdown page to an individual PDF. | Requires a build with `--features headless`. |
| **Merged PDF** | Renders all pages into one PDF document. | Requires a build with `--features headless`. |
| **HTML Files** | Converts each Markdown page to a standalone HTML file with basic styling. | Always available. |
| **Merged HTML** | Concatenates all pages into a single HTML document with `---` separators. | Always available. |

The app detects headless support at runtime and disables PDF options when the feature is not compiled in.

### Markdown Cleaning

Since v0.5.0, the HTML-to-Markdown pipeline includes automatic cleaning steps to produce cleaner output:

- **Script/style removal** — `<script>` and `<style>` tags with their content are stripped before Markdown conversion.
- **Empty link removal** — `<a href="#">` tags without meaningful content are removed before conversion.
- **Boilerplate filtering** — After conversion, common UI noise is removed: "Copy page" / "Edit page" links, cookie banners, newsletter signups, and "Copy code" buttons.
- **Post-processing** — Excessive blank lines are collapsed, empty link URLs (`[text]()`) are unwrapped to plain text, and broken images (`![alt]()`) are removed.
- **Trailing heading stubs** — Orphaned heading lines at the end of a document (often from navigation fragments) are stripped.

### User Interface

| View | Purpose |
|------|---------|
| **Dashboard** | Overview cards (pages saved, total size, crawl velocity, fail rate), quick-start buttons, recent jobs, recent exports, and live session/system status bars. |
| **New Crawl** | Configure and start a crawl with profile selection, include/exclude filters, and a `TemplateBar` (since v0.6.2) to save the current configuration as a reusable named template or apply/delete a saved one. A live monitor shows progress bar, status badge, log drawer, and pause/resume/cancel controls. |
| **History** | Browse all jobs, filter and search, inspect details, open output folders, delete jobs, export results, and — since v0.6.2 — click **Crawl again** on any completed, failed, or cancelled job to re-run it with the exact same settings. |
| **Result Browser** | Split-pane virtualized file tree + lazy-loaded Markdown preview with a debounced search panel that highlights matches across all pages. Arrow-key navigation in the tree since v0.6.1. |
| **Settings** | Since v0.6.2, organized into a category sidebar (General, Shortcuts, Crawling & Export, Network & Storage) instead of one long scrolling page. Configure theme, notifications, keyboard shortcuts, crawl defaults, auto-export, network behavior, output directory, and window size. |
| **Import** | Drag and drop a PDF or EPUB to convert it into a Markdown archive using the same output pipeline as crawled content. |

### Job Persistence

- Job metadata is saved as JSON files in `%APPDATA%/com.docurip.app/jobs/{jobId}.json`.
- Jobs are persisted after each page is processed and on pause / stop / completion.
- Restarting the app reloads all persisted jobs into the History view.
- Deleting a job removes its JSON file and output is left untouched.

### Job Templates

Since v0.6.2, a crawl configuration (URL + every setting — profile, depth, page limit, selectors, include/exclude patterns, path prefix, robots.txt/domain/SSRF toggles) can be saved as a named template and re-applied later.

- **Save** — in New Crawl, fill in the URL and settings you want, then use the `TemplateBar`'s "Save current" control and give it a name.
- **Apply** — click a saved template's chip to instantly replace the current form with its saved configuration.
- **Delete** — click the small delete control on a template chip. Deletion is immediate; there is no confirmation prompt.
- **Storage** — templates are saved as individual JSON files in `%APPDATA%/com.docurip.app/templates/{templateId}.json`, mirroring how jobs are persisted.

### Re-crawl

Since v0.6.2, any completed, failed, or cancelled job in **History** has a **Crawl again** action (circular-arrow icon). It navigates to New Crawl with the job's original URL and its entire configuration pre-filled, so you can re-run a job — after a site update, a fix to your excludes, or a transient failure — without reconfiguring anything.

### Auto-Export

Since v0.6.2, Settings → Crawling & Export includes an **Auto-Export** dropdown. When set to a format other than "Disabled", that export runs automatically the moment any crawl completes — no need to open the Export dialog manually. The export lands in the job's `formats/` directory, same as a manual export. Auto-export only fires on successful completion, not on failed or cancelled jobs.

### Keyboard Shortcuts

Since v0.6.1, docurip is fully navigable by keyboard, and every binding is configurable.

| Action | Default Binding |
|--------|------------------|
| New / Active Crawl | `Ctrl/Cmd+N` |
| Focus Search | `Ctrl/Cmd+F` |
| Dashboard | `Ctrl/Cmd+D` |
| History | `Ctrl/Cmd+H` |
| Settings | `Ctrl/Cmd+,` |
| Import | `Ctrl/Cmd+I` |
| Close Modal / Panel | `Escape` |

Shortcuts are suppressed while typing in a text input or textarea (except `Escape`, which always fires so you can close a dialog from anywhere). Arrow keys navigate the Result Browser's file tree; `Enter` opens the focused page.

**Rebinding:** in Settings → Shortcuts, click any binding to enter capture mode, then press the new key combination. If it's already used by another action, an inline message names the conflicting action and the rebind is rejected — press a different combination or `Escape` to cancel. A reset button restores an action's built-in default. `Escape` itself is not rebindable.

### Desktop Notifications

Since v0.6.1, docurip can show a native system notification when a crawl completes or fails, via `tauri-plugin-notification`. Enabled by default; toggle in Settings → General. On first use, the OS may prompt for notification permission — if denied, notifications are silently skipped rather than erroring.

### Themes

Since v0.6.0, docurip supports **Dark**, **Light**, and **System** (follows your OS light/dark preference live, no restart needed) themes, meeting WCAG AA contrast in both. Switch via the quick-toggle in the top status bar or the full picker in Settings → General.

### Live Events & Stats

- Crawl progress, page completion, logs, errors, and status changes are streamed to the UI in real time via Tauri events.
- Dashboard stats are polled every 3 seconds while a crawl is active and roughly every 12 seconds otherwise to reduce idle load.
- Animated stat counters show updated values smoothly.

### Updates

Docurip checks for application updates on startup. When an update is available, a banner appears with the new version and release notes. If the install fails, the banner displays the error and switches the action to **Retry**.

## Best-Practice Guides

### Crawling Large Documentation Sites (500+ Pages)

1. **Scope the crawl** first with a low `pageLimit` (e.g., 50) and shallow `maxDepth` (2) to understand the site structure before running a full crawl.
2. **Use `stayWithinDomain`** to avoid accidentally following external links.
3. **Add `excludePatterns`** to skip noise such as `/blog/`, `/changelog/`, version-switcher pages, search result pages, or tags/categories. Example patterns:
   - `.*/tag/.*`
   - `.*/search.*`
   - `.*/changelog/.*`
4. **Increase `pageLimit`** to the expected number of pages (up to 10,000). The crawler will stop cleanly once the limit is reached.
5. **Set a polite `requestDelay`** (1,000–2,000 ms) and keep `concurrency` moderate (3–5) to avoid overwhelming the documentation server.
6. **Use headless `auto`** so JavaScript-rendered pages fall back to Chrome automatically without slowing down simple static pages.
7. **Disable `downloadAssets`** unless you need offline images or stylesheets; this significantly reduces disk usage and crawl time.
8. **Respect `robots.txt`** unless the site explicitly blocks useful documentation paths. If it does, consider contacting the site owner rather than disabling the check.
9. **Split huge sites** into multiple jobs by starting from different sections (e.g., `/docs/api/`, `/docs/guides/`) instead of one enormous crawl.
10. **Monitor the live console** for repeated errors or SSRF blocks, and adjust exclude patterns accordingly.

### Preparing Docs for LLM / RAG Use

Docurip’s Markdown output is ideal for feeding into retrieval-augmented generation (RAG) pipelines, embedding models, or large language model (LLM) context windows.

1. **Use content selectors** to strip navigation, headers, footers, and sidebars. Good selectors:
   - `main`
   - `article`
   - `.content`
   - `.markdown-body`
   - `article div.prose`
2. **Disable asset downloads** unless the model needs images or the layout must be preserved. Most text-only pipelines do not need CSS, images, or scripts.
3. **Export as Merged Markdown** for a single file that is easy to split into chunks. The `---` separators between pages make natural chunk boundaries.
4. **Exclude noisy pages** such as:
   - Changelogs and release notes with many version-specific tables
   - Search result pages
   - Tag/category indexes
   - Login / account pages
5. **Keep `respectRobotsTxt` enabled** to stay a good citizen while building training or retrieval corpora.
6. **For very large corpora**, export individual Markdown files and process them with your own chunking script. File paths preserve the original URL structure, making citations easier.
7. **Use a sensible crawl velocity** (`concurrency` 3, `requestDelay` 1000–2000 ms) so the source site is not burdened while you build your dataset.
8. **Sanitize before ingestion** – Markdown preview in Docurip is already DOMPurify-sanitized, but you should run your own cleanup if you convert Markdown back to HTML for model consumption.

## Troubleshooting

| Symptom | Likely Cause | Fix |
|---------|--------------|-----|
| Crawl fails immediately with “SSRF protection blocked the start URL” | The URL resolves to a private/internal address (localhost, 192.168.x.x, etc.). | Use a public URL, or disable **SSRF protection** only for trusted internal documentation. |
| Pages are missing or mostly empty | The documentation is rendered by JavaScript after load. | Set **Headless strategy** to `auto` or `always`. |
| Many assets fail to download | Asset URLs return HTML (login page, redirect, or 404). | This is expected for protected assets. Ensure **Download assets** is enabled and the assets are publicly accessible. Check the MIME-type rejection message in the live log. |
| PDF export option is disabled | Docurip was built without the `headless` feature. | Rebuild the backend with `cargo build --features headless` or download a release that includes headless Chrome support. |
| Output folder cannot be created / permission denied | The configured output directory is read-only or protected. | Change **Output directory** in Settings to a writable location (e.g., `C:\Users\<you>\Documents\Docurip`). If a disk error occurs mid-crawl, fix the path and click **Resume**. |
| “Job not found” when opening History | The job JSON file was deleted or corrupted. | Deleted jobs cannot be recovered. Ensure antivirus or cleanup tools are not removing files from `%APPDATA%/com.docurip.app/jobs/`. |
| Dashboard stats stay at zero | No jobs have completed or the output directory is empty. | Run a crawl and wait a few seconds. Stats refresh every 3 seconds while a crawl is active. |
| Update banner shows an error | The updater could not download or install the release. | Read the error message in the banner and click **Retry**, or download the latest installer manually from the project releases page. |
| Crawl is slower than expected | `requestDelay` is high or `concurrency` is low. | Lower `requestDelay` or raise `concurrency` in Settings, but be respectful to the target server. |
| Repeated 429 / “Too Many Requests” errors | The site is rate-limiting the crawler. | Increase `requestDelay` (e.g., 2,000–5,000 ms), lower `concurrency` to 1–2, and consider crawling during off-peak hours. |
| Result search shows no matches | Query has no hits or the job has no results. | Try a broader keyword. Search matches titles (weight 10), URLs (weight 5), and content. |
| Crawl finds very few or zero pages | An `includePatterns` regex or `pathPrefix` filter is too strict, or an `excludePatterns` entry unintentionally matches everything. | Clear the include/path-prefix filters to confirm the site crawls normally, then re-add them one at a time. Remember exclude patterns always win over includes. |
| Rebinding a shortcut shows "Already used by…" | The key combination you pressed is already bound to another action. | Press a different combination, or first reset/rebind the other action to free it up. |

## Data & Privacy

- Docurip does not send documentation content, job metadata, or usage statistics to any remote service.
- Update checks contact the Tauri updater endpoint configured for the app; no crawl data is included.
- All crawling, conversion, and storage happens locally on your computer.

## Version Notes

This documentation reflects **Docurip v0.6.2**.

### v0.6.x Highlights

- **Dark / Light / System Theme** (v0.6.0) — Full theme infrastructure with WCAG AA contrast in both modes; a follow-up pass further increased the contrast of muted/label text app-wide.
- **Include Patterns & Path Prefix** (v0.6.1) — Whitelist URLs by regex or a simple path prefix, complementing exclude patterns.
- **Configurable Keyboard Shortcuts** (v0.6.1) — Full keyboard navigation with a live rebind UI, conflict detection, and per-action reset.
- **Desktop Notifications** (v0.6.1) — Native system notification on crawl completion/failure.
- **Job Templates** (v0.6.2) — Save a named crawl configuration (URL + full settings) and re-apply it later.
- **Re-crawl** (v0.6.2) — One-click "Crawl again" on any past job, pre-filling every original setting.
- **Auto-Export** (v0.6.2) — Automatically run a chosen export format the moment a crawl completes.
- **Settings Sub-Navigation** (v0.6.2) — Settings reorganized into a category sidebar (General, Shortcuts, Crawling & Export, Network & Storage) instead of one long scrolling page.

### v0.5.x Highlights

- **Crawl Profiles** — Pre-configured presets for API docs, wikis, blogs, and documentation sites.
- **HTML Export** — New export formats: individual HTML files and merged HTML.
- **Markdown Cleaning** — Automatic removal of script/style tags, UI boilerplate, empty links, broken images, and excessive blank lines.
- **Virtualized Result Tree** — The Result Browser file tree uses windowed rendering for large crawl results.
- **Lazy-loaded Markdown Preview** — The preview component is code-split and loaded on demand.
- **Debounced Search** — Search input in the Result Browser is debounced (300 ms) to reduce redundant filtering.

### v0.4.x Highlights

- **Disk-error auto-pause** — Crawls pause automatically on permission denied, read-only filesystem, or no space left.
- **Typed error classification** — All crawl errors are classified by kind (Network, Http, Disk, Parse, Config, Cancelled, Unknown).
- **Queue backpressure** — Internal queue depth is bounded to prevent unbounded memory growth on large crawls.
- **Throttled job persistence** — Active job state is persisted at most every 5 seconds to reduce disk I/O.
- **Active crawl navigation** — Navigate directly to a running crawl from the Dashboard.

### v0.3.x Highlights

- MIME-type validation for downloaded assets
- SSRF checks on start URLs
- Throttled dashboard polling
- Headless Chrome compatibility fixes for `headless_chrome` 1.x
