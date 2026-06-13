# Docurip Phase 3 — Crawl Result Browser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Users can browse, preview, search, and export the Markdown results of a completed crawl job.

**Architecture:** The History view gains a detail overlay that renders a split-pane Result Browser. Left pane: searchable tree of `PageResult`s from the job. Right pane: Markdown preview with syntax highlighting. A backend command zips the job's output directory for download. All page content is read from the in-memory `CrawlJob.results` vec (no filesystem scanning needed).

**Tech Stack:** React 19, Tailwind CSS, TypeScript, Rust (tauri, zip)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src/views/ResultBrowser.tsx` | Split-pane Result Browser view (tree + preview + search + export) |
| `src/components/ResultTree.tsx` | Collapsible tree/list of `PageResult`s with file-type icons |
| `src/components/MarkdownPreview.tsx` | Markdown rendering with dark theme code blocks |
| `src/components/ResultSearch.tsx` | Search input that filters the tree by title/content/url |
| `src/views/History.tsx` | Add "Browse Results" button per completed job; open ResultBrowser overlay |
| `src-tauri/src/commands.rs` | Add `search_job_results`, `export_job_zip` commands |
| `src-tauri/Cargo.toml` | Add `zip` crate dependency |
| `src/types/index.ts` | Add `PageResult` (already exists), `ResultSearchMatch` interfaces |

---

## Task 1: Backend — Search & Export Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (register new commands if needed)

- [x] **Step 1: Add `zip` dependency to Cargo.toml**
- [x] **Step 2: Add `search_job_results` command**
- [x] **Step 3: Add `export_job_zip` command**
- [x] **Step 4: Register new commands in lib.rs**
- [x] **Step 5: `cargo check`**

---

## Task 2: Frontend — Result Browser Components

**Files:**
- Create: `src/components/ResultTree.tsx`
- Create: `src/components/MarkdownPreview.tsx`
- Create: `src/components/ResultSearch.tsx`
- Create: `src/views/ResultBrowser.tsx`

- [x] **Step 1: Create `ResultSearch` component**
- [x] **Step 2: Create `ResultTree` component**
- [x] **Step 3: Create `MarkdownPreview` component**
- [x] **Step 4: Create `ResultBrowser` view**
- [x] **Step 5: Add `EmptyState` import check**
- [x] **Step 6: `npm run build`**

---

## Task 3: Frontend — History Integration

**Files:**
- Modify: `src/views/History.tsx`

- [x] **Step 1: Add "Browse Results" button to completed jobs**
- [x] **Step 2: Handle edge case — crawling in progress**
- [x] **Step 3: `npm run build`**

---

## Task 4: Add Type Definitions

**Files:**
- Modify: `src/types/index.ts`

- [x] **Step 1: Add `SearchMatch` interface**
- [x] **Step 2: Verify `PageResult` has `content` field**

---

## Integration & Verification

- [x] **Step 1: `cargo check`**
- [x] **Step 2: `cargo test`**
- [x] **Step 3: `npm run build`**
- [x] **Step 4: Manuelle Tests**

1. Einen Crawl durchführen (z.B. `https://example.com`)
2. In History auf "Browse Results" klicken
3. File-Tree zeigt alle Pages hierarchisch
4. Auf eine Page klicken → Markdown Vorschau rechts
5. In Suchleiste etwas eingeben → Tree filtert
6. "Export ZIP" klicken → ZIP wird erstellt, Pfad angezeigt
7. "Open Folder" klicken → Output-Ordner öffnet sich

---

## Spec Coverage Check

| Requirement | Task |
|------------|------|
| File-Tree der Ergebnisse | Task 2 (ResultTree) |
| Markdown-Vorschau / Reader | Task 2 (MarkdownPreview) |
| Suche innerhalb der Ergebnisse | Task 1 (search_job_results) + Task 2 (ResultSearch) |
| ZIP-Export-Button | Task 1 (export_job_zip) + Task 2 (ResultBrowser) |

## Placeholder Scan

Keine TBDs, TODOs, oder unvollständige Code-Blöcke.

## Type Consistency

- `PageResult` identisch in TS und Rust
- `SearchMatch` nur Backend → TS via invoke
- `CrawlJob` identisch in TS und Rust
- `job.config.outputDir` passt zu Rust `CrawlConfig.output_dir`
