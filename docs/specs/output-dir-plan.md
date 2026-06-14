# Implementation Plan: Output-Dir-System-Redesign

## Overview

Rework the output directory system so each crawl gets an organized subfolder (`{domain}/{date}-{id}`), the UI uses native folder pickers instead of text inputs, and exports pre-fill from the job's output path.

## Task List

### Phase 1: Backend — Subfolder Generation

- [ ] **Task 1:** Add `resolve_output_dir` helper to orchestrator.rs that generates `{domain}/{date}-{id}` subfolder from the URL + job_id when `config.output_dir` is empty. Write it into `CrawlJob.config.output_dir` before persisting.
  - Acceptance: Orchestrator creates `docs.example.com/2026-06-14-abc123def/` under global default when output_dir is empty
  - Verify: `cargo test` passes
  - Files: `src-tauri/src/crawler/orchestrator.rs`
  - Scope: S

### Phase 2: Frontend — Folder Pickers

- [ ] **Task 2:** Replace text input in NewCrawl.tsx with folder picker button + path display. Use `@tauri-apps/plugin-dialog` `open({ directory: true })`. Empty state shows "Auto-organized" with globe icon.
  - Acceptance: Clicking folder icon opens native picker. Selected path is shown. Empty state shows auto-organized preview.
  - Verify: `npm run build` succeeds, manual check
  - Files: `src/views/NewCrawl.tsx`
  - Scope: S

- [ ] **Task 3:** Replace text input in Settings.tsx with folder picker button + path display. Same pattern as Task 2.
  - Acceptance: Clicking folder icon opens native picker. Selected path is shown. "Reset to default" option exists.
  - Verify: `npm run build` succeeds
  - Files: `src/views/Settings.tsx`
  - Scope: S

### Phase 3: ExportModal Pre-fill

- [ ] **Task 4:** Update ExportModal to pre-fill destination from `job.config.outputDir`. Remove mandatory folder picker dialog. Add "Save here" button that exports directly.
  - Acceptance: ExportModal opens with destination pre-filled. "Save here" exports without picker dialog. User can still change destination.
  - Verify: `npm run build` succeeds, manual check
  - Files: `src/components/ExportModal.tsx`
  - Scope: S

### Phase 4: History Polish

- [ ] **Task 5:** Show truncated output path in History detail view. Verify "Open folder" works for all completed jobs (use resolved output_dir from persisted job).
  - Acceptance: Output path visible in history detail. "Open folder" button opens correct directory.
  - Verify: `npm run build` succeeds, manual check
  - Files: `src/views/History.tsx`
  - Scope: S

### Checkpoint: Complete
- [ ] All 75 tests pass: `cargo test`
- [ ] Frontend builds: `npm run build`
- [ ] App runs: `npm run tauri dev` — new crawl creates subfolder, folder pickers work, export pre-fills
