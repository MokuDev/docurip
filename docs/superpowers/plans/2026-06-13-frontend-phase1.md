# Docurip Frontend Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the frontend by fixing integration gaps, wiring Tauri events globally, and implementing job detail views. The frontend is already ~70% built; this plan covers the remaining 30%.

**Architecture:** React 19 + TypeScript + Tailwind CSS 3. Tauri v2 event bus (`listen('crawl-event')`) streams live progress. React state managed via hooks (no external store needed for v1). Views: Dashboard, NewCrawl, History, Settings, LiveConsole.

**Tech Stack:** React 19, Vite 6, Tailwind CSS 3.4, `@tauri-apps/api` v2, `@phosphor-icons/react`, `framer-motion` (already in package.json but unused).

**Current State Assessment:**
- ✅ Sidebar navigation, tab switching, routing
- ✅ Dashboard with stats cards and recent activity list
- ✅ NewCrawl with full config form + live monitor panel
- ✅ History with filter, search, delete (UI only)
- ✅ Settings with save/reset
- ✅ LiveConsole component with event listener
- ✅ Dark terminal theme, Tailwind colors, fonts
- ⚠️ `Spinner` icon doesn't exist in Phosphor (causes crash)
- ⚠️ `headlessStrategy` values mismatch frontend↔backend
- ⚠️ Dashboard Quick Start is disabled/stub
- ⚠️ No global event listener — each view creates its own
- ⚠️ History delete doesn't call backend
- ⚠️ No job detail view / result browsing
- ⚠️ Framer Motion unused — add subtle animations

---

### Task 1: Fix Critical UI Bugs

**Files:**
- Modify: `src/views/NewCrawl.tsx`
- Modify: `src/types/index.ts`

- [x] **Step 1: Replace non-existent `Spinner` icon**
- [x] **Step 2: Fix `headlessStrategy` enum mismatch**
- [x] **Step 3: Verify fix with `npm run build`**

---

### Task 2: Create Global Event Listener Hook

**Files:**
- Create: `src/hooks/useCrawlEvents.ts`
- Modify: `src/App.tsx`

- [x] **Step 1: Create `useCrawlEvents` hook**
- [x] **Step 2: Wire hook into App.tsx**
- [x] **Step 3: Create hooks directory if not exists**

---

### Task 3: Wire Dashboard Quick Start

**Files:**
- Modify: `src/views/Dashboard.tsx`
- Modify: `src/App.tsx` (add tab switching callback)

- [x] **Step 1: Enable Quick Start form in Dashboard**
- [x] **Step 2: Pass tab-switch callback through App**
- [x] **Step 3: Accept prefillUrl in NewCrawl**

---

### Task 4: Add Job Detail View

**Files:**
- Create: `src/views/JobDetail.tsx`
- Modify: `src/views/History.tsx`

- [x] **Step 1: Create JobDetail component**
- [x] **Step 2: Wire JobDetail into History**

---

### Task 5: Polish & Animation

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/styles/index.css`

- [x] **Step 1: Add framer-motion page transitions**
- [x] **Step 2: Add scanline effect to LiveConsole**

---

### Task 6: Integration Verification

**Files:** All frontend files.

- [x] **Step 1: Build check**
- [x] **Step 2: Cargo check**
- [x] **Step 3: Manual smoke test (dev mode)**

```bash
cd D:\.GitHub2\docurip
npm run dev
```
Verify:
- Dashboard loads with stats
- Quick Start URL redirects to NewCrawl with prefilled URL
- NewCrawl form submits and starts job
- LiveConsole shows events
- History shows jobs and clicking opens detail view
- Settings save/load persists

---

## Self-Review Checklist

| Spec Requirement | Task Covering It |
|---|---|
| Dashboard with stats cards | Task 3 (Quick Start wiring) + existing code |
| Live progress console | Task 2 (global events) + existing LiveConsole |
| History with job list & filters | Existing code + Task 4 (detail view) |
| Settings persistence | Existing code |
| Dark terminal aesthetic | Existing code + Task 5 (scanline) |
| Tauri event stream | Task 2 (global hook) |
| Job detail / result browsing | Task 4 |

**No placeholders found.** Every step contains actual code.

---

**Execution Options:**

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution

Which approach do you prefer?
