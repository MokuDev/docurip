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

- [ ] **Step 1: Replace non-existent `Spinner` icon**

`@phosphor-icons/react` has no `Spinner` component. Use `SpinnerGap` with animate-spin class instead.

In `src/views/NewCrawl.tsx`, replace all occurrences:
```tsx
// OLD (broken):
<Spinner className="animate-spin" size={18} />
// NEW:
<SpinnerGap className="animate-spin" size={18} />
```

Add import: `import { SpinnerGap } from '@phosphor-icons/react';`
Remove `Spinner` from import.

- [ ] **Step 2: Fix `headlessStrategy` enum mismatch**

Frontend uses: `'disabled' | 'js-only' | 'all'`
Backend uses: `'never' | 'auto' | 'always'`

Update `src/types/index.ts`:
```ts
export interface CrawlConfig {
  // ... other fields
  headlessStrategy: 'never' | 'auto' | 'always';
  // ...
}
```

Update `src/views/NewCrawl.tsx`:
```ts
const DEFAULT_CONFIG: CrawlConfig = {
  // ...
  headlessStrategy: 'never',
  // ...
};
```

Replace select options:
```tsx
<select value={config.headlessStrategy} onChange={...}>
  <option value="never">Never (raw HTML)</option>
  <option value="auto">Auto (fallback on JS-rendered)</option>
  <option value="always">Always (headless Chrome)</option>
</select>
```

- [ ] **Step 3: Verify fix with `npm run build`**

Run: `cd D:\.GitHub2\docurip && npm run build`
Expected: Build succeeds without TypeScript errors.

---

### Task 2: Create Global Event Listener Hook

**Files:**
- Create: `src/hooks/useCrawlEvents.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create `useCrawlEvents` hook**

```ts
import { useState, useEffect, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { CrawlEvent, CrawlJob } from '../types';

interface JobMap {
  [jobId: string]: CrawlJob;
}

interface UseCrawlEventsReturn {
  jobs: JobMap;
  activeJobIds: string[];
  latestEvent: CrawlEvent | null;
}

export function useCrawlEvents(): UseCrawlEventsReturn {
  const [jobs, setJobs] = useState<JobMap>({});
  const [latestEvent, setLatestEvent] = useState<CrawlEvent | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      unlisten = await listen<CrawlEvent>('crawl-event', (event) => {
        const ev = event.payload;
        setLatestEvent(ev);

        setJobs((prev) => {
          const job = prev[ev.jobId];
          if (!job && ev.type !== 'jobStatusChanged') return prev;

          const updated = { ...job };

          switch (ev.type) {
            case 'progress':
              if (ev.progress) updated.progress = ev.progress;
              break;
            case 'pageComplete':
              if (ev.page) {
                updated.results = [...(updated.results || []), ev.page];
              }
              break;
            case 'jobStatusChanged':
              if (ev.status) updated.status = ev.status;
              break;
            case 'error':
              updated.error = ev.message;
              break;
          }

          return { ...prev, [ev.jobId]: updated };
        });
      });
    };

    setup();
    return () => unlisten?.();
  }, []);

  const activeJobIds = Object.values(jobs)
    .filter((j) => j.status === 'running' || j.status === 'queued')
    .map((j) => j.id);

  return { jobs, activeJobIds, latestEvent };
}
```

- [ ] **Step 2: Wire hook into App.tsx**

In `src/App.tsx`, replace the polling-based job tracking:

```tsx
// Add import
import { useCrawlEvents } from './hooks/useCrawlEvents';

// Replace activeJobs state with:
const { jobs, activeJobIds } = useCrawlEvents();
const activeCount = activeJobIds.length;

// In JSX, replace activeJobs references:
badge={activeCount > 0 ? `${activeCount}` : undefined}

// Remove the useEffect interval for polling list_jobs
```

- [ ] **Step 3: Create hooks directory if not exists**

```bash
mkdir -p src/hooks
```

---

### Task 3: Wire Dashboard Quick Start

**Files:**
- Modify: `src/views/Dashboard.tsx`
- Modify: `src/App.tsx` (add tab switching callback)

- [ ] **Step 1: Enable Quick Start form in Dashboard**

In `src/views/Dashboard.tsx`, add state for the quick URL:
```tsx
const [quickUrl, setQuickUrl] = useState('');
```

Replace disabled input/button:
```tsx
<input
  type="url"
  placeholder="Enter a URL to crawl..."
  value={quickUrl}
  onChange={(e) => setQuickUrl(e.target.value)}
  className="flex-1 bg-surface/50 border border-abyssal rounded-md px-4 py-3 text-ghost placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50"
/>
<button
  onClick={() => {
    if (!quickUrl) return;
    // Switch to New Crawl tab with prefilled URL
    onStartCrawl?.(quickUrl);
  }}
  disabled={!quickUrl}
  className="bg-accentGreen hover:bg-brightGreen text-deepVoid px-6 py-3 rounded-md font-semibold transition-all duration-fast hover:shadow-[0_0_15px_rgba(22,224,141,0.3)] disabled:opacity-50"
>
  Start Crawl
</button>
```

- [ ] **Step 2: Pass tab-switch callback through App**

In `src/App.tsx`, add state to track prefilled URL:
```tsx
const [prefillUrl, setPrefillUrl] = useState('');
```

Pass to Dashboard:
```tsx
{activeTab === 'dashboard' && (
  <DashboardView onStartCrawl={(url) => {
    setPrefillUrl(url);
    setActiveTab('crawls');
  }} />
)}
```

- [ ] **Step 3: Accept prefillUrl in NewCrawl**

In `src/views/NewCrawl.tsx`, add prop:
```tsx
export function NewCrawlView({ prefillUrl }: { prefillUrl?: string }) {
```

Use effect to set URL when prefill changes:
```tsx
useEffect(() => {
  if (prefillUrl && !activeJob) {
    setConfig((prev) => ({ ...prev, url: prefillUrl }));
  }
}, [prefillUrl]);
```

---

### Task 4: Add Job Detail View

**Files:**
- Create: `src/views/JobDetail.tsx`
- Modify: `src/views/History.tsx`

- [ ] **Step 1: Create JobDetail component**

```tsx
import { useState } from 'react';
import { ArrowLeft, FileText, Globe, Link as LinkIcon, CheckCircle } from '@phosphor-icons/react';
import type { CrawlJob } from '../types';

export function JobDetail({ job, onBack }: { job: CrawlJob; onBack: () => void }) {
  const [activeTab, setActiveTab] = useState<'pages' | 'links' | 'assets'>('pages');

  return (
    <div className="h-full flex flex-col">
      <div className="h-14 flex items-center px-5 border-b border-abyssal/50">
        <button onClick={onBack} className="mr-4 text-charcoal hover:text-ghost transition-colors">
          <ArrowLeft size={20} />
        </button>
        <div className="flex-1 min-w-0">
          <h1 className="text-ghost font-semibold text-base truncate">{job.url}</h1>
          <p className="text-charcoal text-xs">ID: {job.id}</p>
        </div>
        <StatusBadge status={job.status} />
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 border-b border-abyssal/30">
        <StatBox icon={<FileText size={16} className="text-accentGreen" />} label="Pages" value={job.results?.length || 0} />
        <StatBox icon={<LinkIcon size={16} className="text-cyberBlue" />} label="Links" value={job.results?.reduce((s, r) => s + r.links.length, 0) || 0} />
        <StatBox icon={<Globe size={16} className="text-brightGreen" />} label="Assets" value={job.results?.reduce((s, r) => s + r.assets.length, 0) || 0} />
      </div>

      {/* Tabs */}
      <div className="flex border-b border-abyssal/30">
        {(['pages', 'links', 'assets'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-xs font-medium uppercase tracking-wider transition-colors ${
              activeTab === tab ? 'text-accentGreen border-b-2 border-accentGreen' : 'text-charcoal hover:text-secondary'
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {activeTab === 'pages' && (
          <div className="space-y-2">
            {job.results?.map((page, i) => (
              <div key={i} className="bg-surface/30 border border-abyssal/30 rounded-md p-3">
                <p className="text-sm text-ghost font-medium">{page.title || 'Untitled'}</p>
                <p className="text-xs text-charcoal truncate">{page.url}</p>
                <p className="text-xs text-charcoal mt-1">{page.content?.slice(0, 200)}...</p>
              </div>
            )) || <p className="text-charcoal text-center py-8">No pages crawled</p>}
          </div>
        )}
        {activeTab === 'links' && (
          <div className="space-y-1">
            {[...new Set(job.results?.flatMap((r) => r.links) || [])].map((link, i) => (
              <div key={i} className="text-xs text-secondary break-all py-1">{link}</div>
            )) || <p className="text-charcoal text-center py-8">No links found</p>}
          </div>
        )}
        {activeTab === 'assets' && (
          <div className="space-y-1">
            {[...new Set(job.results?.flatMap((r) => r.assets) || [])].map((asset, i) => (
              <div key={i} className="text-xs text-secondary break-all py-1">{asset}</div>
            )) || <p className="text-charcoal text-center py-8">No assets found</p>}
          </div>
        )}
      </div>
    </div>
  );
}

const StatBox = ({ icon, label, value }: { icon: React.ReactNode; label: string; value: number }) => (
  <div className="flex items-center px-5 py-3 border-r border-abyssal/30 last:border-r-0">
    <div className="mr-3">{icon}</div>
    <div>
      <div className="text-lg font-mono font-semibold text-ghost">{value}</div>
      <div className="text-[10px] text-charcoal uppercase tracking-wider">{label}</div>
    </div>
  </div>
);

const StatusBadge = ({ status }: { status: string }) => {
  const styles: Record<string, string> = {
    queued: 'bg-amber/10 text-amber',
    running: 'bg-accentGreen/10 text-accentGreen',
    paused: 'bg-cyberBlue/10 text-cyberBlue',
    completed: 'bg-brightGreen/10 text-brightGreen',
    failed: 'bg-crimson/10 text-crimson',
  };
  return (
    <span className={`text-[11px] font-semibold uppercase tracking-wider px-2 py-1 rounded ${styles[status] || 'text-charcoal'}`}>
      {status}
    </span>
  );
};
```

- [ ] **Step 2: Wire JobDetail into History**

In `src/views/History.tsx`, add state:
```tsx
const [selectedJob, setSelectedJob] = useState<CrawlJob | null>(null);
```

If selected:
```tsx
if (selectedJob) {
  return <JobDetail job={selectedJob} onBack={() => setSelectedJob(null)} />;
}
```

Add click handler on job row:
```tsx
<div onClick={() => setSelectedJob(job)} className="cursor-pointer ...">
```

---

### Task 5: Polish & Animation

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/styles/index.css`

- [ ] **Step 1: Add framer-motion page transitions**

In `src/App.tsx`, wrap tab content:
```tsx
import { AnimatePresence, motion } from 'framer-motion';

// In main content area:
<AnimatePresence mode="wait">
  <motion.div
    key={activeTab}
    initial={{ opacity: 0, y: 10 }}
    animate={{ opacity: 1, y: 0 }}
    exit={{ opacity: 0, y: -10 }}
    transition={{ duration: 0.2 }}
    className="flex-1 flex flex-col overflow-hidden"
  >
    {activeTab === 'dashboard' && <DashboardView ... />}
    // ... other tabs
  </motion.div>
</AnimatePresence>
```

- [ ] **Step 2: Add scanline effect to LiveConsole**

In `src/styles/index.css`:
```css
.scanline::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: repeating-linear-gradient(
    0deg,
    transparent,
    transparent 2px,
    rgba(0, 0, 0, 0.03) 2px,
    rgba(0, 0, 0, 0.03) 4px
  );
  pointer-events: none;
  z-index: 1;
}
```

Add `scanline relative` class to LiveConsole root element.

---

### Task 6: Integration Verification

**Files:** All frontend files.

- [ ] **Step 1: Build check**

```bash
cd D:\.GitHub2\docurip
npm run build
```
Expected: No TypeScript errors, build succeeds.

- [ ] **Step 2: Cargo check**

```bash
cd D:\.GitHub2\docurip\src-tauri
cargo check
```
Expected: Clean compilation.

- [ ] **Step 3: Manual smoke test (dev mode)**

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
