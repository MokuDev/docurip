# REVIEW.md

Guidance for reviewing changes to docurip — a Tauri v2 desktop app that
crawls documentation sites and converts them to offline Markdown.
There is no server, no multi-tenancy, no user account. What breaks a
crawler is different from what breaks a SaaS, so calibrate accordingly.

## What matters in this repository

- **Backend ↔ frontend IPC contract**. Every `#[tauri::command]` and
  every `CrawlEvent` variant is a wire boundary. Field names must
  round-trip through serde exactly as the frontend expects
  (`rename_all_fields = "camelCase"` on tagged enums, `#[serde(default)]`
  on additive struct fields). A silent snake_case leak here is invisible
  in tests that only exercise one side.
- **Filesystem path safety**. `FsWriter` derives on-disk paths from
  user-supplied URLs. Every code path that turns a URL into a `PathBuf`
  is a potential traversal or drive-root escape. See the existing
  `writer::fs::tests::test_url_to_*_traversal_*` cases for the shape.
  Empty strings, `..`, unicode NFC vs NFD, backslashes on Windows, and
  absolute-looking hostnames all belong in the test bed.
- **SSRF protection is a real security boundary**. `crawler::ssrf`
  blocks private IP literals, `.local` hostnames, and DNS-resolved
  private ranges. Any new fetch surface (sitemap, robots, headless,
  asset download, future features) must run its start URL through
  `is_private_target` when `config.ssrf_protection` is on. New checks
  belong next to the existing ones, not sprinkled inline.
- **Async cancellation and pause**. Crawls use `Arc<AtomicBool>` +
  `Notify` for stop/pause. New long-running work inside the orchestrator
  must poll these signals at the same cadence as the existing loop.
  Ignoring them leaves ghost tasks that persist past a "Cancel" click
  and drift the "active jobs" counter.
- **Resource caps**. Every fetch/parse has a numeric ceiling
  (10 000 URLs per sitemap, 50 MB body, 30 s timeout, 500 URLs per
  batch, MAX_DEPTH 2 for sitemap-index recursion, 50 MB per asset).
  Removing or raising a cap needs a written reason; adding a new
  unbounded input needs a new cap. Truncation ("`result.truncated =
  true`") is preferred over hard errors for user-facing paths.
- **User-supplied regex**. Include/exclude patterns come from a
  textarea. They must be validated (`regex::Regex::new`) before a crawl
  starts, not lazily at first match. See `validate_crawl_input` for the
  shape.
- **Persistence backward-compat**. Job/template/batch JSON files live
  on the user's disk across app versions. Adding a field to
  `CrawlJob`, `CrawlTemplate`, `BatchJob`, or any type they embed
  requires `#[serde(default)]` (or `#[serde(default = "…")]` with a
  sensible non-panicking default). A missing default silently drops
  every persisted entry on next start.
- **Event pipeline correctness**. The `crawl-event` channel drives
  auto-open of the LiveConsole, the "JOBS N" top-bar counter,
  desktop notifications, and auto-export. A change that touches the
  `CrawlEvent` enum, the `useCrawlEvents` reducer, or the frontend
  `CrawlEvent` type must update all three plus the wire tests.
- **Feature-flagged headless**. Headless Chrome sits behind the
  `headless` cargo feature. Non-headless builds must degrade gracefully
  (`anyhow::bail!` with a message the frontend can surface), not
  compile-error. Check both `cargo check` and `cargo check --features
  headless` when touching the fetcher, export, or converter.
- **Windows vs Unix path assumptions**. `format!("{}/{}", base, host)`
  works on Linux and blows up on Windows when `base` is empty (root of
  current drive) or uses `\` separators. Prefer `PathBuf::join`, and
  resolve empty/default markers at the read boundary (see
  `get_settings`'s empty-`output_dir` fallback) rather than at every
  usage site.

## Severity calibration

- **Critical** — data loss, SSRF bypass, path traversal outside the
  configured `output_dir`, secret leakage (though there are few
  secrets here — `AppSettings` is not sensitive, but the user's home
  directory location and browsing behavior are), a change that makes
  every persisted job unloadable, a panic in the orchestrator hot loop
  that kills the whole tokio task, or a fix that also silently
  changes the wire format and breaks the frontend at runtime.
- **Warning** — missing input validation on user-supplied strings,
  a new fetch call that skips SSRF or robots.txt, an unbounded input
  (no cap, no timeout), a new `CrawlEvent` field with `#[serde(default)]`
  missing, `.unwrap()` / `.expect()` in a hot path, blocking `std::fs`
  or `std::net` inside an async task, a new setting that stores an
  empty string as a "use default" marker without a matching normalizer,
  a React effect that attaches an async listener without cancellation
  handling (StrictMode reveal bug — see `useCrawlEvents`).
- **Nit** — style choices already enforced by tooling (rustfmt,
  prettier), naming preferences, refactors that fragment small helpers
  further, comment wording. Skip these unless the reader would
  genuinely misread the code.
- **Do not flag**:
  - Pre-existing TypeScript errors in `src/views/Settings.tsx` (see
    `CLAUDE.md` — these predate the current branch and Vite builds
    around them).
  - `default_include_patterns` / `default_path_prefix` on
    `CrawlProfile` returning empty values — already tracked in the
    ROADMAP's optimization list.
  - Duplication between the several `resolve_output_dir` /
    `output_dir_for_job` helpers unless the change touches them.
  - String-based error kind checks in `is_transient_error` — the
    typed-downcast path already handles the modern cases; the string
    fallback is an intentional safety net.

## Verification expectations

- **IPC / event changes**: add a serialization test in the module
  where the type is defined. `events::bus::tests::*_camelcase_*` is
  the template — assert `v["type"]`, every renamed field, and that
  the snake_case name is *not* present (`assert!(v.get("job_id").is_none())`).
  A change that touches `CrawlEvent` also updates
  `src/types/index.ts` and the `LiveConsole` message formatter in
  the same commit.
- **Orchestrator behavior**: new crawl logic gets a wiremock-driven
  integration test that starts a crawl and asserts on the visible
  outcome (pages written, events emitted, terminal status), not on
  internal state. See `fetcher::http::tests` for the wiremock pattern.
- **Path handling**: a new `url_to_*` helper needs at least four
  cases: normal, traversal (`..`), URL query/fragment stripping, and
  unicode host. Cross-platform separators (Windows `\`) if the code
  will touch `Path::join`.
- **Persistence changes**: adding a field to a persisted type is only
  landable with (a) a `#[serde(default)]` and (b) a unit test that
  deserializes a stored JSON file that predates the field. `JsonStore`'s
  `init_skips_corrupt_files` test is the shape.
- **Filter/regex validation**: any new user-supplied pattern surface
  needs to plumb through `validate_crawl_input` (or an equivalent
  helper) before a crawl starts. Invalid input must surface as a
  `Result::Err`, not a runtime panic mid-crawl.
- **Frontend UI changes** for user-visible features: at least
  screenshot the change or drive it manually and note it in the PR body
  (the PR template's Test plan checklist). Unit tests for React
  rendering are welcome but not required; `useCrawlEvents.test.tsx`
  is the shape when the logic is worth pinning.
- **Cross-platform**: if a change touches paths, environment
  variables, or the launched-from-cwd assumption, note explicitly in
  the PR body whether it was tested on Windows. The primary target
  is Windows; regressions there are release-blocking.
- **Docs**: any user-visible change updates `CHANGELOG.md`
  (`Added` / `Changed` / `Fixed` sections) and, when a mental model
  changes, `Documentation.md`. `ROADMAP.md` marks completion or
  strikes through a resolved cleanup item.
