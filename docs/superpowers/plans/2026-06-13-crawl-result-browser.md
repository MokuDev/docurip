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

- [ ] **Step 1: Add `zip` dependency to Cargo.toml**

```toml
[dependencies]
zip = { version = "2.1", default-features = false, features = ["deflate"] }
```

- [ ] **Step 2: Add `search_job_results` command**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub url: String,
    pub title: String,
    pub preview: String,      // first 200 chars of content around match
    pub relevance: u32,       // simple score: matches in title = 10, content = 1
}

#[tauri::command]
pub async fn search_job_results(
    job_id: String,
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<SearchMatch>, String> {
    let job = get_job(job_id, state).await?;
    let q = query.to_lowercase();
    let mut matches = Vec::new();

    for page in &job.results {
        let title_lower = page.title.to_lowercase();
        let content_lower = page.content.to_lowercase();
        let url_lower = page.url.to_lowercase();

        let title_score = title_lower.matches(&q).count() as u32;
        let content_score = content_lower.matches(&q).count() as u32;
        let url_score = url_lower.matches(&q).count() as u32;

        let relevance = title_score * 10 + content_score + url_score * 5;

        if relevance > 0 {
            let preview = extract_preview(&page.content, &q);
            matches.push(SearchMatch {
                url: page.url.clone(),
                title: page.title.clone(),
                preview,
                relevance,
            });
        }
    }

    matches.sort_by(|a, b| b.relevance.cmp(&a.relevance));
    Ok(matches)
}

fn extract_preview(content: &str, query: &str) -> String {
    let lower = content.to_lowercase();
    if let Some(pos) = lower.find(&query.to_lowercase()) {
        let start = pos.saturating_sub(80);
        let end = (pos + query.len() + 120).min(content.len());
        let mut preview = content[start..end].to_string();
        if start > 0 { preview.insert_str(0, "…"); }
        if end < content.len() { preview.push('…'); }
        preview
    } else {
        content.chars().take(200).collect::<String>() + "…"
    }
}
```

**WICHTIG:** `get_job` ist bereits definiert in `commands.rs`. Wenn es nicht public ist, kopiere die Job-Ladeslogik oder mache `get_job` public. Die bestehende `get_job` lädt aus active_jobs und persisted_jobs.

- [ ] **Step 3: Add `export_job_zip` command**

```rust
use std::fs::File;
use std::io::Write;
use zip::write::FileOptions;

#[tauri::command]
pub async fn export_job_zip(
    job_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let job = get_job(job_id.clone(), state).await?;
    let output_dir = std::path::PathBuf::from(&job.config.output_dir);

    if !output_dir.exists() {
        return Err("Output directory does not exist".into());
    }

    let zip_path = output_dir.parent()
        .unwrap_or(&output_dir)
        .join(format!("{}-export.zip", job_id));

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<File>,
        base: &std::path::Path,
        current: &std::path::Path,
        options: FileOptions<()>,
    ) -> Result<(), String> {
        for entry in std::fs::read_dir(current).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let relative = path.strip_prefix(base).map_err(|e| e.to_string())?;

            if path.is_file() {
                let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
                zip.start_file_from_path(relative, options.clone())
                    .map_err(|e| e.to_string())?;
                std::io::copy(&mut file, zip).map_err(|e| e.to_string())?;
            } else if path.is_dir() {
                zip.add_directory_from_path(relative, options.clone())
                    .map_err(|e| e.to_string())?;
                add_dir_to_zip(zip, base, &path, options.clone())?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip, &output_dir, &output_dir, options)
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(zip_path.to_string_lossy().to_string())
}
```

**WICHTIG:** Tauri v2 hat eingeschränkte Filesystem-API. Da das Crawlen bereits Dateien in `output_dir` schreibt (unbeschränkter Rust-Code), ist das ZIP-Packen im Rust-Backend ebenfalls unbeschränkt. Der zurückgegebene Pfad kann dann mit `open_output_folder` geöffnet werden (existiert bereits).

- [ ] **Step 4: Register new commands in lib.rs**

In `src-tauri/src/lib.rs` (oder wo `generate_handler!` aufgerufen wird):
```rust
.generate_handler![
    start_crawl,
    stop_crawl,
    get_job,
    list_jobs,
    delete_job,
    get_settings,
    update_settings,
    open_output_folder,
    search_job_results,
    export_job_zip,
]
```

- [ ] **Step 5: `cargo check`**

Run: `cargo check`
Expected: Sauber

---

## Task 2: Frontend — Result Browser Components

**Files:**
- Create: `src/components/ResultTree.tsx`
- Create: `src/components/MarkdownPreview.tsx`
- Create: `src/components/ResultSearch.tsx`
- Create: `src/views/ResultBrowser.tsx`

- [ ] **Step 1: Create `ResultSearch` component**

```tsx
import { useState } from 'react';
import { MagnifyingGlass } from '@phosphor-icons/react';

interface ResultSearchProps {
  value: string;
  onChange: (query: string) => void;
  resultCount: number;
}

export function ResultSearch({ value, onChange, resultCount }: ResultSearchProps) {
  return (
    <div className="relative">
      <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal" size={16} />
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search results..."
        className="w-full bg-deepVoid border border-abyssal/50 text-ghost placeholder-charcoal rounded-md pl-9 pr-4 py-2 text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
      />
      {value && (
        <span className="absolute right-3 top-1/2 -translate-y-1/2 text-charcoal text-xs">
          {resultCount} found
        </span>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create `ResultTree` component**

```tsx
import { FileText, CaretRight, CaretDown } from '@phosphor-icons/react';
import { useState } from 'react';
import type { PageResult } from '../types';

interface TreeNode {
  name: string;
  path: string;
  page?: PageResult;
  children: TreeNode[];
}

function buildTree(pages: PageResult[]): TreeNode[] {
  const root: TreeNode[] = [];

  for (const page of pages) {
    try {
      const url = new URL(page.url);
      const host = url.hostname;
      const pathSegments = url.pathname.split('/').filter(Boolean);

      let hostNode = root.find((n) => n.name === host);
      if (!hostNode) {
        hostNode = { name: host, path: host, children: [] };
        root.push(hostNode);
      }

      let current = hostNode;
      for (let i = 0; i < pathSegments.length; i++) {
        const seg = pathSegments[i];
        const isLast = i === pathSegments.length - 1;
        const fullPath = current.path + '/' + seg;

        let child = current.children.find((c) => c.name === seg);
        if (!child) {
          child = { name: seg, path: fullPath, children: [] };
          if (isLast) {
            child.page = page;
          }
          current.children.push(child);
        } else if (isLast) {
          child.page = page;
        }
        current = child;
      }
    } catch {
      // invalid URL — add as flat entry
      root.push({ name: page.url, path: page.url, page, children: [] });
    }
  }

  return root;
}

function TreeNodeView({
  node,
  selectedUrl,
  onSelect,
  depth = 0,
}: {
  node: TreeNode;
  selectedUrl: string;
  onSelect: (page: PageResult) => void;
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const isSelected = node.page?.url === selectedUrl;
  const hasChildren = node.children.length > 0;

  return (
    <div>
      <button
        onClick={() => {
          if (node.page) onSelect(node.page);
          if (hasChildren) setExpanded(!expanded);
        }}
        className={`w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-md transition-all ${
          isSelected
            ? 'bg-accentGreen/10 text-accentGreen'
            : 'text-secondary hover:text-ghost hover:bg-surface/40'
        }`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
      >
        {hasChildren ? (
          expanded ? <CaretDown size={14} className="text-charcoal" /> : <CaretRight size={14} className="text-charcoal" />
        ) : (
          <FileText size={14} className="text-charcoal" />
        )}
        <span className="truncate">{node.name}</span>
        {node.page && (
          <span className="ml-auto text-[10px] text-charcoal font-mono">{node.page.status}</span>
        )}
      </button>
      {expanded && node.children.map((child) => (
        <TreeNodeView
          key={child.path}
          node={child}
          selectedUrl={selectedUrl}
          onSelect={onSelect}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}

interface ResultTreeProps {
  pages: PageResult[];
  selectedUrl: string;
  onSelect: (page: PageResult) => void;
  filterQuery?: string;
}

export function ResultTree({ pages, selectedUrl, onSelect, filterQuery }: ResultTreeProps) {
  const filtered = filterQuery
    ? pages.filter(
        (p) =>
          p.title.toLowerCase().includes(filterQuery.toLowerCase()) ||
          p.url.toLowerCase().includes(filterQuery.toLowerCase())
      )
    : pages;

  const tree = buildTree(filtered);

  return (
    <div className="overflow-y-auto h-full">
      {tree.map((node) => (
        <TreeNodeView key={node.path} node={node} selectedUrl={selectedUrl} onSelect={onSelect} />
      ))}
      {filtered.length === 0 && (
        <p className="text-charcoal text-xs px-3 py-4 text-center">No results found</p>
      )}
    </div>
  );
}
```

**WICHTIG:** Der Tree baut aus URLs eine hierarchische Struktur. `page.content` wird NICHT hier gerendert — nur im Preview Panel.

- [ ] **Step 3: Create `MarkdownPreview` component**

```tsx
import { useMemo } from 'react';

interface MarkdownPreviewProps {
  content: string;
  searchQuery?: string;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function highlightQuery(text: string, query?: string): string {
  if (!query || query.length < 2) return escapeHtml(text);
  const escaped = escapeHtml(text);
  const q = escapeHtml(query);
  const regex = new RegExp(`(${q})`, 'gi');
  return escaped.replace(regex, '<mark class="bg-accentGreen/30 text-accentGreen rounded px-0.5">$1</mark>');
}

export function MarkdownPreview({ content, searchQuery }: MarkdownPreviewProps) {
  const html = useMemo(() => {
    // Simple Markdown-to-HTML converter for preview
    // Supports: headers, code blocks, inline code, bold, italic, links, lists, blockquotes
    let md = content;

    // Escape HTML
    md = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

    // Code blocks ```...```
    md = md.replace(/```(\w+)?\n([\s\S]*?)```/g, (_, lang, code) => {
      return `<pre class="bg-deepVoid border border-abyssal/50 rounded-md p-3 overflow-x-auto my-3"><code class="text-sm font-mono text-ghost">${code.trim()}</code></pre>`;
    });

    // Inline code
    md = md.replace(/`([^`]+)`/g, '<code class="bg-deepVoid border border-abyssal/50 rounded px-1 py-0.5 text-xs font-mono text-accentGreen">$1</code>');

    // Headers
    md = md.replace(/^#### (.*$)/gim, '<h4 class="text-ghost font-semibold text-sm mt-4 mb-2">$1</h4>');
    md = md.replace(/^### (.*$)/gim, '<h3 class="text-ghost font-semibold text-base mt-5 mb-2">$1</h3>');
    md = md.replace(/^## (.*$)/gim, '<h2 class="text-ghost font-semibold text-lg mt-6 mb-3 border-b border-abyssal/50 pb-1">$1</h2>');
    md = md.replace(/^# (.*$)/gim, '<h1 class="text-ghost font-bold text-xl mt-8 mb-4 border-b border-abyssal/50 pb-2">$1</h1>');

    // Bold + Italic
    md = md.replace(/\*\*\*(.*?)\*\*\*/g, '<strong><em>$1</em></strong>');
    md = md.replace(/\*\*(.*?)\*\*/g, '<strong class="text-ghost">$1</strong>');
    md = md.replace(/\*(.*?)\*/g, '<em>$1</em>');
    md = md.replace(/__(.*?)__/g, '<strong class="text-ghost">$1</strong>');
    md = md.replace(/_(.*?)_/g, '<em>$1</em>');

    // Links
    md = md.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" class="text-accentGreen hover:underline" target="_blank" rel="noopener">$1</a>');

    // Blockquotes
    md = md.replace(/^&gt; (.*$)/gim, '<blockquote class="border-l-2 border-accentGreen/50 pl-3 my-3 text-charcoal italic">$1</blockquote>');

    // Lists
    md = md.replace(/^\s*[-*] (.*$)/gim, '<li class="ml-4 text-secondary">$1</li>');
    md = md.replace(/(<li.*<\/li>\n?)+/g, '<ul class="my-2 space-y-0.5">$&</ul>');
    md = md.replace(/^\s*\d+\. (.*$)/gim, '<li class="ml-4 text-secondary">$1</li>');
    md = md.replace(/(<li.*<\/li>\n?)+/g, (match) => {
      if (match.includes('<ul')) return match;
      return '<ol class="my-2 space-y-0.5 list-decimal">' + match + '</ol>';
    });

    // Horizontal rules
    md = md.replace(/^---$/gim, '<hr class="border-abyssal/50 my-4" />');

    // Paragraphs (wrap remaining lines)
    const lines = md.split('\n');
    let inPre = false;
    const processed = lines.map((line) => {
      if (line.startsWith('<pre')) inPre = true;
      if (line.startsWith('</pre')) { inPre = false; return line; }
      if (inPre) return line;
      if (line.trim() === '') return '<div class="h-2"></div>';
      if (line.startsWith('<')) return line;
      return `<p class="text-secondary leading-relaxed my-1">${line}</p>`;
    });

    return processed.join('\n');
  }, [content]);

  const highlightedHtml = useMemo(() => {
    if (!searchQuery || searchQuery.length < 2) return html;
    const q = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${q})`, 'gi');
    return html.replace(regex, '<mark class="bg-accentGreen/30 text-accentGreen rounded px-0.5">$1</mark>');
  }, [html, searchQuery]);

  return (
    <div
      className="h-full overflow-y-auto px-6 py-4 prose prose-invert prose-sm max-w-none"
      dangerouslySetInnerHTML={{ __html: highlightedHtml }}
    />
  );
}
```

**WICHTIG:** Der `MarkdownPreview` nutzt keinen externen Markdown-Renderer (kein `marked`, `remark`, etc.) um Bundle-Größe zu sparen. Der simple Regex-Parser ist für Dokumentations-Crawl-Ergebnisse ausreichend. Wenn der User einen vollen Parser will, kann er später `react-markdown` hinzufügen.

- [ ] **Step 4: Create `ResultBrowser` view**

```tsx
import { useState, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  X,
  DownloadSimple,
  FileArrowUp,
  FolderOpen,
} from '@phosphor-icons/react';
import type { CrawlJob, PageResult } from '../types';
import { ResultTree } from '../components/ResultTree';
import { MarkdownPreview } from '../components/MarkdownPreview';
import { ResultSearch } from '../components/ResultSearch';
import { EmptyState } from '../components/EmptyState';

interface ResultBrowserProps {
  job: CrawlJob;
  onClose: () => void;
}

export function ResultBrowser({ job, onClose }: ResultBrowserProps) {
  const [selectedPage, setSelectedPage] = useState<PageResult | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [exporting, setExporting] = useState(false);
  const [exportPath, setExportPath] = useState('');

  const pages = job.results;

  const filteredPages = useMemo(() => {
    if (!searchQuery) return pages;
    const q = searchQuery.toLowerCase();
    return pages.filter(
      (p) =>
        p.title.toLowerCase().includes(q) ||
        p.url.toLowerCase().includes(q) ||
        p.content.toLowerCase().includes(q)
    );
  }, [pages, searchQuery]);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const path: string = await invoke('export_job_zip', { jobId: job.id });
      setExportPath(path);
    } catch (err) {
      console.error('Export failed', err);
    } finally {
      setExporting(false);
    }
  }, [job.id]);

  const handleOpenFolder = useCallback(async () => {
    try {
      await invoke('open_output_folder', { path: job.config.outputDir });
    } catch (err) {
      console.error('Open folder failed', err);
    }
  }, [job.config.outputDir]);

  return (
    <div className="fixed inset-0 z-50 flex">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />

      {/* Panel */}
      <motion.div
        initial={{ x: '100%' }}
        animate={{ x: 0 }}
        exit={{ x: '100%' }}
        transition={{ type: 'spring', damping: 25, stiffness: 200 }}
        className="relative ml-auto w-full max-w-5xl h-full bg-deepVoid border-l border-abyssal/50 flex flex-col"
      >
        {/* Header */}
        <div className="h-14 flex items-center justify-between px-4 border-b border-abyssal/50 bg-surface/30">
          <div className="flex items-center gap-3 min-w-0">
            <FileArrowUp size={18} className="text-accentGreen" />
            <div className="min-w-0">
              <h2 className="text-ghost font-semibold text-sm truncate">{job.url}</h2>
              <p className="text-charcoal text-xs">{pages.length} pages</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs text-secondary hover:text-ghost hover:bg-surface/60 transition-all"
            >
              <FolderOpen size={14} />
              Open Folder
            </button>
            <button
              onClick={handleExport}
              disabled={exporting}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all disabled:opacity-50"
            >
              <DownloadSimple size={14} />
              {exporting ? 'Exporting...' : 'Export ZIP'}
            </button>
            {exportPath && (
              <span className="text-charcoal text-xs max-w-[200px] truncate" title={exportPath}>
                {exportPath}
              </span>
            )}
            <button
              onClick={onClose}
              className="p-1.5 rounded-md hover:bg-surface/60 text-charcoal hover:text-ghost transition-all"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Toolbar */}
        <div className="px-4 py-2 border-b border-abyssal/50">
          <ResultSearch
            value={searchQuery}
            onChange={setSearchQuery}
            resultCount={filteredPages.length}
          />
        </div>

        {/* Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Tree */}
          <div className="w-80 flex-shrink-0 border-r border-abyssal/50 bg-surface/20">
            {pages.length > 0 ? (
              <ResultTree
                pages={filteredPages}
                selectedUrl={selectedPage?.url || ''}
                onSelect={setSelectedPage}
                filterQuery={searchQuery}
              />
            ) : (
              <EmptyState
                icon={<FileText size={40} />}
                title="No pages"
                description="This crawl produced no results."
              />
            )}
          </div>

          {/* Preview */}
          <div className="flex-1 bg-[#050a0f]">
            {selectedPage ? (
              <MarkdownPreview
                content={selectedPage.content}
                searchQuery={searchQuery}
              />
            ) : (
              <EmptyState
                icon={<FileText size={48} />}
                title="Select a page"
                description="Click a page in the tree to preview its content."
              />
            )}
          </div>
        </div>
      </motion.div>
    </div>
  );
}
```

**WICHTIG:** `motion.div` erfordert `framer-motion` Import. Wenn der Import fehlt, ergänze:
```tsx
import { motion } from 'framer-motion';
```

- [ ] **Step 5: Add `EmptyState` import check**

`EmptyState` wurde in Task 2 (Phase 2b) als `src/components/EmptyState.tsx` erstellt. Wenn diese Datei noch nicht existiert (weil Phase 2b Task 3 anders implementiert wurde), erstelle sie:

```tsx
import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center h-full">
      <div className="text-charcoal mb-4">{icon}</div>
      <h3 className="text-ghost font-semibold mb-2">{title}</h3>
      <p className="text-secondary text-sm max-w-sm mb-4">{description}</p>
      {action}
    </div>
  );
}
```

- [ ] **Step 6: `npm run build`**

Run: `npm run build`
Expected: Sauber

---

## Task 3: Frontend — History Integration

**Files:**
- Modify: `src/views/History.tsx`

- [ ] **Step 1: Add "Browse Results" button to completed jobs**

In der Job-Card oder Job-Row in `History.tsx`, wenn `job.status === 'completed'`:
```tsx
import { FileArrowUp } from '@phosphor-icons/react';
import { ResultBrowser } from './ResultBrowser';

// In History component state:
const [selectedJob, setSelectedJob] = useState<CrawlJob | null>(null);

// In Job row:
{job.status === 'completed' && job.results.length > 0 && (
  <button
    onClick={() => setSelectedJob(job)}
    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all"
  >
    <FileArrowUp size={14} />
    Browse Results
  </button>
)}

// Render overlay:
{selectedJob && (
  <ResultBrowser job={selectedJob} onClose={() => setSelectedJob(null)} />
)}
```

**WICHTIG:** Stelle sicher, dass `History.tsx` die `CrawlJob` import hat und `CrawlJob` das `results` Feld enthält (ja, aus `src/types/index.ts`).

- [ ] **Step 2: Handle edge case — crawling in progress**

Wenn `job.status === 'running'`, zeige stattdessen:
```tsx
<span className="text-charcoal text-xs">Crawl in progress...</span>
```

- [ ] **Step 3: `npm run build`**

Run: `npm run build`
Expected: Sauber

---

## Task 4: Add Type Definitions

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: Add `SearchMatch` interface**

```typescript
export interface SearchMatch {
  url: string;
  title: string;
  preview: string;
  relevance: number;
}
```

- [ ] **Step 2: Verify `PageResult` has `content` field**

Prüfe, ob `PageResult` in `src/types/index.ts` das `content` Feld hat:
```typescript
export interface PageResult {
  url: string;
  title: string;
  content: string;
  links: string[];
  assets: string[];
  status: number;
}
```

Das ist bereits der Fall (aus Phase 1). Wenn nicht, hinzufügen.

---

## Integration & Verification

- [ ] **Step 1: `cargo check`**

Expected: Sauber

- [ ] **Step 2: `cargo test`**

Expected: Alle bestehenden Tests passing

- [ ] **Step 3: `npm run build`**

Expected: Sauber

- [ ] **Step 4: Manuelle Tests**

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
