import { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import { X, GitDiff, ArrowLeft } from '@phosphor-icons/react';
import type { CrawlJob, DiffResult, DiffLine, ChangeKind, PageDiff } from '../types';

interface DiffViewProps {
  /** The newer crawl — the "after" side of the comparison. */
  job: CrawlJob;
  /** All jobs, used to offer earlier crawls of the same URL as a baseline. */
  allJobs: CrawlJob[];
  onClose: () => void;
}

const KIND_STYLES: Record<ChangeKind, { label: string; dot: string; text: string }> = {
  added: { label: 'Added', dot: 'bg-accentGreen', text: 'text-accentGreen' },
  removed: { label: 'Removed', dot: 'bg-crimson', text: 'text-crimson' },
  modified: { label: 'Modified', dot: 'bg-amber', text: 'text-amber' },
  unchanged: { label: 'Unchanged', dot: 'bg-charcoal', text: 'text-charcoal' },
  unknown: { label: 'Unknown', dot: 'bg-secondary', text: 'text-secondary' },
};

/** Kinds for which a line-level diff is meaningful (content on at least one side). */
const DIFFABLE: ChangeKind[] = ['added', 'removed', 'modified'];

export function DiffView({ job, allJobs, onClose }: DiffViewProps) {
  const [baselineId, setBaselineId] = useState<string | null>(null);
  const [diff, setDiff] = useState<DiffResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<ChangeKind | 'all'>('all');
  const [selectedPage, setSelectedPage] = useState<PageDiff | null>(null);
  const [lineDiff, setLineDiff] = useState<DiffLine[] | null>(null);
  const [lineLoading, setLineLoading] = useState(false);

  // Earlier completed crawls of the same URL make sensible baselines.
  const candidates = useMemo(
    () =>
      allJobs
        .filter(
          (j) => j.id !== job.id && j.status === 'completed' && j.url === job.url,
        )
        .sort((a, b) => (b.startTime ?? '').localeCompare(a.startTime ?? '')),
    [allJobs, job],
  );

  // Default to the most recent earlier crawl.
  useEffect(() => {
    if (!baselineId && candidates.length > 0) {
      setBaselineId(candidates[0].id);
    }
  }, [candidates, baselineId]);

  useEffect(() => {
    if (!baselineId) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    setSelectedPage(null);
    setLineDiff(null);
    invoke<DiffResult>('diff_jobs', { oldJobId: baselineId, newJobId: job.id })
      .then((res) => {
        if (!cancelled) setDiff(res);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [baselineId, job.id]);

  const openLineDiff = useCallback(
    (page: PageDiff) => {
      if (!baselineId || !DIFFABLE.includes(page.kind)) return;
      setSelectedPage(page);
      setLineLoading(true);
      setLineDiff(null);
      invoke<DiffLine[]>('diff_page', {
        oldJobId: baselineId,
        newJobId: job.id,
        url: page.url,
      })
        .then(setLineDiff)
        .catch((e) => setLineDiff([{ tag: 'equal', content: `Could not load diff: ${e}` }]))
        .finally(() => setLineLoading(false));
    },
    [baselineId, job.id],
  );

  const visibleEntries = useMemo(() => {
    if (!diff) return [];
    if (kindFilter === 'all') return diff.entries;
    return diff.entries.filter((e) => e.kind === kindFilter);
  }, [diff, kindFilter]);

  const counts: { kind: ChangeKind; n: number }[] = diff
    ? [
        { kind: 'added', n: diff.added },
        { kind: 'removed', n: diff.removed },
        { kind: 'modified', n: diff.modified },
        { kind: 'unchanged', n: diff.unchanged },
        { kind: 'unknown', n: diff.unknown },
      ]
    : [];

  return (
    <>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 bg-black/60 z-40"
        onClick={onClose}
      />
      <motion.div
        initial={{ opacity: 0, scale: 0.98 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.98 }}
        className="fixed inset-6 bg-deepVoid border border-abyssal/50 rounded-xl z-50 flex flex-col shadow-2xl overflow-hidden"
      >
        {/* Header */}
        <div className="h-14 flex items-center justify-between px-5 border-b border-abyssal/50 flex-shrink-0">
          <div className="flex items-center gap-2 min-w-0">
            <GitDiff size={18} className="text-accentGreen flex-shrink-0" />
            <h2 className="text-ghost font-semibold text-base truncate">Compare Crawls</h2>
            <span className="text-xs text-charcoal truncate">— {job.url}</span>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Baseline picker */}
        <div className="px-5 py-3 border-b border-abyssal/50 flex items-center gap-3 flex-wrap flex-shrink-0">
          <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">
            Compare against
          </label>
          {candidates.length === 0 ? (
            <span className="text-sm text-charcoal">
              No earlier completed crawl of this URL to compare with.
            </span>
          ) : (
            <select
              value={baselineId ?? ''}
              onChange={(e) => setBaselineId(e.target.value)}
              className="bg-surface/50 border border-abyssal rounded-md px-3 py-1.5 text-sm text-ghost focus:outline-none focus:border-accentGreen/50 cursor-pointer max-w-md"
            >
              {candidates.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.startTime ? new Date(c.startTime).toLocaleString() : c.id.slice(0, 8)} ·{' '}
                  {c.results?.length ?? 0} pages
                </option>
              ))}
            </select>
          )}
        </div>

        {/* Summary counts */}
        {diff && (
          <div className="px-5 py-3 border-b border-abyssal/50 flex items-center gap-2 flex-wrap flex-shrink-0">
            <button
              onClick={() => setKindFilter('all')}
              className={`px-2.5 py-1 rounded-md text-xs transition-colors ${
                kindFilter === 'all'
                  ? 'bg-accentGreen/20 text-accentGreen'
                  : 'bg-surface/40 text-charcoal hover:text-ghost'
              }`}
            >
              All ({diff.entries.length})
            </button>
            {counts.map(({ kind, n }) => (
              <button
                key={kind}
                onClick={() => setKindFilter(kind)}
                disabled={n === 0}
                className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs transition-colors disabled:opacity-40 ${
                  kindFilter === kind ? 'bg-surface text-ghost' : 'bg-surface/40 text-charcoal hover:text-ghost'
                }`}
              >
                <span className={`w-2 h-2 rounded-full ${KIND_STYLES[kind].dot}`} />
                {KIND_STYLES[kind].label} ({n})
              </button>
            ))}
          </div>
        )}

        {/* Body */}
        <div className="flex-1 overflow-hidden flex">
          {/* Left: page list */}
          <div className="w-1/2 border-r border-abyssal/50 overflow-y-auto">
            {loading ? (
              <div className="flex items-center justify-center h-full">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-accentGreen" />
              </div>
            ) : error ? (
              <div className="p-5 text-sm text-crimson">{error}</div>
            ) : !diff ? (
              <div className="p-5 text-sm text-charcoal">Select a baseline crawl to compare.</div>
            ) : visibleEntries.length === 0 ? (
              <div className="p-5 text-sm text-charcoal">No pages in this category.</div>
            ) : (
              <ul className="divide-y divide-abyssal/30">
                {visibleEntries.map((entry) => {
                  const diffable = DIFFABLE.includes(entry.kind);
                  const active = selectedPage?.url === entry.url;
                  return (
                    <li key={`${entry.kind}-${entry.url}`}>
                      <button
                        onClick={() => openLineDiff(entry)}
                        disabled={!diffable}
                        className={`w-full text-left px-4 py-2.5 flex items-center gap-3 transition-colors ${
                          active ? 'bg-surface/60' : 'hover:bg-surface/30'
                        } ${diffable ? 'cursor-pointer' : 'cursor-default'}`}
                      >
                        <span className={`w-2 h-2 rounded-full flex-shrink-0 ${KIND_STYLES[entry.kind].dot}`} />
                        <span className="flex-1 min-w-0">
                          <span className="block text-sm text-ghost truncate">
                            {entry.title || entry.url}
                          </span>
                          <span className="block text-xs text-charcoal truncate">{entry.url}</span>
                        </span>
                        <span className={`text-[10px] uppercase tracking-wider ${KIND_STYLES[entry.kind].text}`}>
                          {KIND_STYLES[entry.kind].label}
                        </span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          {/* Right: line diff */}
          <div className="w-1/2 overflow-y-auto bg-deepVoid/60">
            {!selectedPage ? (
              <div className="flex flex-col items-center justify-center h-full text-charcoal gap-2">
                <ArrowLeft size={24} className="opacity-30" />
                <p className="text-sm">Select a changed page to see its line diff.</p>
              </div>
            ) : lineLoading ? (
              <div className="flex items-center justify-center h-full">
                <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-accentGreen" />
              </div>
            ) : (
              <div>
                <div className="sticky top-0 bg-deepVoid/95 backdrop-blur px-4 py-2 border-b border-abyssal/50">
                  <p className="text-xs text-ghost truncate">{selectedPage.title || selectedPage.url}</p>
                </div>
                <pre className="text-xs font-mono leading-relaxed p-0 m-0">
                  {(lineDiff ?? []).map((line, i) => (
                    <div
                      key={i}
                      className={`px-4 py-0.5 whitespace-pre-wrap break-words ${
                        line.tag === 'insert'
                          ? 'bg-accentGreen/10 text-accentGreen'
                          : line.tag === 'delete'
                          ? 'bg-crimson/10 text-crimson'
                          : 'text-smooth'
                      }`}
                    >
                      <span className="select-none opacity-50 mr-2">
                        {line.tag === 'insert' ? '+' : line.tag === 'delete' ? '-' : ' '}
                      </span>
                      {line.content || ' '}
                    </div>
                  ))}
                </pre>
              </div>
            )}
          </div>
        </div>
      </motion.div>
    </>
  );
}
