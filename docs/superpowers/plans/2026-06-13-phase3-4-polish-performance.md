# Phase 3+4: Frontend Polish + Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve UX with error display, settings validation, empty states; improve performance with parallel fetching, rate limiting, and safe request batching.

**Architecture:** Add global error handling in React components; backend-side settings validation + optimized fetch pipeline with configurable concurrency and rate limiting.

**Tech Stack:** Rust (tokio, reqwest, regex), React (TypeScript, Tailwind), Tauri v2

---

## File Structure

- `src/hooks/useCrawlEvents.tsx` — Add error state + error display logic
- `src/views/NewCrawl.tsx` — Add settings validation, error banner, empty-state checks
- `src/views/History.tsx` — Add empty state illustration/message
- `src/views/Settings.tsx` — Add inline validation for form fields
- `src-tauri/src/commands.rs` — Add settings validation before crawl start
- `src-tauri/src/crawler/orchestrator.rs` — Add concurrency limit + rate limiter + parallel fetch loop
- `src-tauri/src/fetcher/http.rs` — Add request timeout override + expose per-request config

---

## Task 1: Frontend — Global Error Banner in NewCrawl

**Files:**
- Modify: `src/hooks/useCrawlEvents.tsx`
- Modify: `src/views/NewCrawl.tsx`

Add a global `error` field to the `CrawlContextType` emitted by `useCrawlEvents`.
Display it in `NewCrawl` as a dismissible red banner at the top of the live monitor panel.

**Steps:**

- [ ] **Step 1: Add `error` and `clearError` to the context type/interface**

```typescript
export interface CrawlContextType {
  jobId: string | null;
  phase: CrawlPhase;
  progress: CrawlProgress | null;
  lastEvent: CrawlEvent | null;
  error: string | null;
  clearError: () => void;
}
```

- [ ] **Step 2: Implement `error` state in `useCrawlEvents`**
Set `error` when an event with `error` payload arrives; expose `clearError` via `useCallback`.

- [ ] **Step 3: Render error banner in `NewCrawl`**
Above the spinner/monitor panel:
```tsx
{error && (
  <div className="bg-red-50 border-l-4 border-red-500 text-red-700 p-4 mb-4 rounded flex justify-between">
    <span>{error}</span>
    <button onClick={clearError} className="font-bold">&times;</button>
  </div>
)}
```

- [ ] **Step 4: Ensure `clearError` is called automatically when a new crawl starts**

---

## Task 2: Frontend — Empty State for History & Settings Validation UI

**Files:**
- Modify: `src/views/History.tsx`
- Modify: `src/views/Settings.tsx`

Add an empty state to `History` when `jobs.length === 0`.
Add inline validation styling (`red border`, helper text) to `Settings` inputs (max depth must be >= 1, delay must be >= 0, format must be non-empty).

**Steps:**

- [ ] **Step 1: Add empty state to `History`**
```tsx
{jobs.length === 0 && (
  <div className="flex flex-col items-center justify-center h-64 text-gray-500">
    <DocumentTextIcon className="w-12 h-12 mb-2" />
    <p>No saved crawls yet.</p>
  </div>
)}
```

- [ ] **Step 2: Add `isValid` helper and validation state in `Settings`**
Track `urlError`, `depthError`, `delayError`, `formatError` booleans.
Render inputs with conditional `border-red-500` class and `<p className="text-red-500 text-sm">` messages.

- [ ] **Step 3: Prevent save/submit when validation fails**
Disable the "Save" / "Start Crawl" button if any error field is set.

---

## Task 3: Backend — Settings validation in `start_crawl` command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: backend integration in existing test suite (if any, else manual)

Validate `CrawlSettings` inside `start_crawl` before spawning the crawler:
- `url` must be valid per `reqwest::Url::parse`
- `max_depth` must be >= 1
- `request_delay_ms` must be >= 0
- `output_format` must not be empty

**Steps:**

- [ ] **Step 1: Add `validate_settings` helper fn**
```rust
fn validate_settings(s: &CrawlSettings) -> Result<(), String> {
    reqwest::Url::parse(&s.url).map_err(|_| "Invalid URL".to_string())?;
    if s.max_depth < 1 { return Err("max_depth must be >= 1".into()); }
    if s.request_delay_ms > 0 && s.request_delay_ms < 100 { return Err("delay too short".into()); }
    if s.output_format.is_empty() { return Err("output_format required".into()); }
    Ok(())
}
```

- [ ] **Step 2: Call it at top of `start_crawl`**
Return `Err(validate_settings(&payload).err().unwrap())` (wrapped via `tauri::Error` or custom error enum) before spawning.

- [ ] **Step 3: Ensure frontend displays the returned error string**

---

## Task 4: Backend — Parallel fetching with concurrency limit & rate limiting

**Files:**
- Modify: `src-tauri/src/crawler/orchestrator.rs`
- Modify: `src-tauri/src/fetcher/http.rs`
- Test: add concurrency/rate-limit unit test

Current orchestrator fetches pages sequentially. Refactor the URL queue worker to:
1. Use a `tokio::sync::Semaphore` to limit concurrent fetches (default: 5).
2. Use a `tokio::time::interval` or `tokio::time::sleep` with `request_delay_ms` to enforce rate limiting.
3. Still respect `is_cancelled` checks.
4. Keep event emission order deterministically correct (events fire per-page as each future completes).

**Steps:**

- [ ] **Step 1: Add `concurrency: usize` field to `Orchestrator`**
Default to `5` in builder.

- [ ] **Step 2: Refactor page-processing loop to use bounded concurrency**
Replace sequential `while let Some(url) = urls.pop() { ... }` with:
```rust
let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
let mut handles = vec![];
for url in urls {
    let sem = semaphore.clone();
    let handle = tokio::spawn(async move {
        let _permit = sem.acquire().await.unwrap();
        // ... fetch, parse, emit events ...
    });
    handles.push(handle);
}
```
Wait for all handles.  Ensure cancel sets a shared `AtomicBool` that each task checks before/after acquiring permit.

- [ ] **Step 3: Add `tokio::time::sleep` before each fetch for rate limiting**
```rust
if self.request_delay_ms > 0 {
    tokio::time::sleep(Duration::from_millis(self.request_delay_ms.into())).await;
}
```
Place this inside the spawned task, after acquiring permit.

- [ ] **Step 4: Ensure `http::fetch_page` accepts optional timeout override**
Expose `timeout: Duration` param on `fetch_page` (default 30 s); update callers.

- [ ] **Step 5: Add unit test verifying concurrency <= limit**
Use a mock fetcher or counter to ensure max N requests in-flight at once.

---

## Task 5: Backend — CLI / Cargo feature flags for headless chrome

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/fetcher/headless.rs`
- Modify: `src-tauri/src/crawler/orchestrator.rs`

Ensure `headless-chrome` feature flag compiles correctly and the fallback path (HTTP fetcher) is active when the feature is off.

**Steps:**

- [ ] **Step 1: Verify `[features]` block in `Cargo.toml`**
```toml
[features]
default = []
headless-chrome = ["dep:headless_chrome"]
```

- [ ] **Step 2: Gate `mod headless` and its usage with `#[cfg(feature = "headless-chrome")]`**
In `fetcher/mod.rs` and `orchestrator.rs`.

- [ ] **Step 3: Verify `cargo check` and `cargo test` pass with and without feature**
```bash
cargo check
cargo check --features headless-chrome
cargo test
cargo test --features headless-chrome
```

---

## Task 6: Build verification and final React smoke test

**Files:**
- All modified frontend files
- All modified backend files

**Steps:**

- [ ] **Step 1: `cargo test`** — expect all existing + new tests passing.
- [ ] **Step 2: `cargo check`** — no warnings.
- [ ] **Step 3: `npm run build`** — no TS/build errors.
- [ ] **Step 4: `npm run tauri dev` if available, else verify `tauri build` prep step.**

---

## Spec Coverage Checklist

- [x] Errors from backend appear in frontend ✅ Task 1
- [x] Settings validated before crawl ✅ Task 2 + 3
- [x] Empty state when no history ✅ Task 2
- [x] Parallel fetch with concurrency limit ✅ Task 4
- [x] Rate limiting via request_delay ✅ Task 4
- [x] Headless feature flag safe ✅ Task 5
- [x] Build/test green ✅ Task 6
