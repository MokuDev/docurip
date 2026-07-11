import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import {
  FileText,
  Clock,
  FolderOpen,
  Trash,
  MagnifyingGlass,
  Funnel,
  X,
  Eye,
  Download,
  FileArrowUp,
  ArrowClockwise,
  ListNumbers,
  CaretRight,
  CaretDown,
} from '@phosphor-icons/react';
import { ResultBrowser } from './ResultBrowser';
import { ExportModal } from '../components/ExportModal';
import { StatusIcon, StatusBadge } from '../components/StatusBadge';
import type { BatchJob, CrawlJob } from '../types';

export function HistoryView({ onCrawlAgain }: { onCrawlAgain: (job: CrawlJob) => void }) {
  const [jobs, setJobs] = useState<CrawlJob[]>([]);
  const [batches, setBatches] = useState<BatchJob[]>([]);
  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [selectedJob, setSelectedJob] = useState<CrawlJob | null>(null);
  const [browserJob, setBrowserJob] = useState<CrawlJob | null>(null);
  const [exportJobId, setExportJobId] = useState<string | null>(null);
  const [expandedBatches, setExpandedBatches] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadJobs(true);
    const interval = setInterval(() => loadJobs(false), 3000);
    return () => clearInterval(interval);
  }, []);

  const loadJobs = async (showSpinner = false) => {
    if (showSpinner) setLoading(true);
    try {
      const [jobsData, batchesData] = await Promise.all([
        invoke<CrawlJob[]>('list_jobs'),
        invoke<BatchJob[]>('list_batches').catch(() => []),
      ]);
      setJobs(jobsData || []);
      setBatches(batchesData || []);
    } catch (err) {
      console.error('Failed to load history', err);
    } finally {
      if (showSpinner) setLoading(false);
    }
  };

  const handleDelete = async (jobId: string) => {
    try {
      await invoke('delete_job', { jobId });
      await loadJobs();
    } catch (err) {
      console.error('Failed to delete job', err);
    }
  };

  const handleDeleteBatch = async (batchId: string) => {
    try {
      await invoke('delete_batch', { batchId });
      // Also drop the child jobs so the UI is coherent.
      const children = jobs.filter((j) => j.batchId === batchId).map((j) => j.id);
      await Promise.all(children.map((id) => invoke('delete_job', { jobId: id }).catch(() => {})));
      await loadJobs();
    } catch (err) {
      console.error('Failed to delete batch', err);
    }
  };

  const toggleBatch = (batchId: string) => {
    setExpandedBatches((prev) => {
      const next = new Set(prev);
      if (next.has(batchId)) next.delete(batchId);
      else next.add(batchId);
      return next;
    });
  };

  const handleExport = (job: CrawlJob) => {
    setExportJobId(job.id);
  };

  const handleOpenFolder = async (outputDir: string) => {
    try {
      const mainDir = outputDir ? `${outputDir}/main` : outputDir;
      await invoke('open_output_folder', { path: mainDir });
    } catch {
      // fallback: silently fail
    }
  };

  const matchesSearch = (haystacks: string[]) => {
    if (!search) return true;
    const term = search.toLowerCase();
    return haystacks.some((h) => h.toLowerCase().includes(term));
  };

  const filteredJobs = useMemo(() => {
    return jobs
      .filter((j) => {
        if (filter === 'batch') return !!j.batchId;
        if (filter === 'all') return true;
        return j.status === filter;
      })
      .filter((j) => matchesSearch([j.url, j.id, j.batchId ?? '']));
  }, [jobs, filter, search]);

  const groups = useMemo(() => {
    // Build a sorted list of entries that mixes standalone jobs and
    // batches. Each batch entry carries the child jobs that survived
    // the current filter/search.
    type BatchEntry = { kind: 'batch'; batch: BatchJob; children: CrawlJob[]; sortKey: string };
    type JobEntry = { kind: 'job'; job: CrawlJob; sortKey: string };
    type Entry = BatchEntry | JobEntry;

    const childrenByBatch = new Map<string, CrawlJob[]>();
    for (const j of filteredJobs) {
      if (!j.batchId) continue;
      const list = childrenByBatch.get(j.batchId) ?? [];
      list.push(j);
      childrenByBatch.set(j.batchId, list);
    }

    const entries: Entry[] = [];

    for (const batch of batches) {
      if (filter === 'batch') {
        // fall through — batches always match this filter
      } else if (filter !== 'all' && batch.status !== filter) {
        // batch doesn't match, but its children still might
        const children = childrenByBatch.get(batch.id) ?? [];
        if (children.length === 0) continue;
      }
      if (!matchesSearch([batch.id, batch.name ?? '', ...batch.urls])) {
        // batch itself doesn't match search — include only if a filtered
        // child slipped through the search filter above.
        if (!childrenByBatch.has(batch.id)) continue;
      }
      const children = childrenByBatch.get(batch.id) ?? [];
      entries.push({
        kind: 'batch',
        batch,
        children,
        sortKey: batch.startTime ?? batch.createdAt,
      });
    }

    for (const job of filteredJobs) {
      if (job.batchId) continue; // shown under its batch
      entries.push({
        kind: 'job',
        job,
        sortKey: job.startTime ?? job.endTime ?? '',
      });
    }

    entries.sort((a, b) => (b.sortKey ?? '').localeCompare(a.sortKey ?? ''));
    return entries;
  }, [batches, filteredJobs, filter, search]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="h-14 flex items-center justify-between px-5 border-b border-abyssal/50">
        <h1 className="text-ghost font-semibold text-base">History</h1>
        <div className="flex items-center space-x-3">
          {/* Search */}
          <div className="relative">
            <MagnifyingGlass
              size={14}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal"
            />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search crawls..."
              className="bg-surface/50 border border-abyssal rounded-md pl-8 pr-3 py-1.5 text-sm text-ghost placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 w-56"
            />
          </div>

          {/* Filter */}
          <div className="relative">
            <Funnel
              size={14}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal"
            />
            <select
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="bg-surface/50 border border-abyssal rounded-md pl-8 pr-3 py-1.5 text-sm text-ghost focus:outline-none focus:border-accentGreen/50 appearance-none cursor-pointer"
            >
              <option value="all">All</option>
              <option value="running">Running</option>
              <option value="completed">Completed</option>
              <option value="failed">Failed</option>
              <option value="batch">Batches only</option>
            </select>
          </div>
        </div>
      </div>

      {/* Job List */}
      <div className="flex-1 overflow-y-auto p-5">
        {loading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-accentGreen" />
          </div>
        ) : groups.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-charcoal">
            {!search && filter === 'all' ? (
              <>
                <FileText size={48} className="mb-4 opacity-20" />
                <p className="text-ghost font-medium">No crawls yet</p>
                <p className="text-xs mt-1 opacity-50">Start a new crawl to see history here</p>
              </>
            ) : (
              <>
                <Clock size={48} className="mb-4 opacity-20" />
                <p className="text-ghost font-medium">No crawls found</p>
                <p className="text-xs mt-1 opacity-50">Try adjusting your filters</p>
              </>
            )}
          </div>
        ) : (
          <div className="space-y-3">
            {groups.map((entry) =>
              entry.kind === 'batch' ? (
                <BatchCard
                  key={entry.batch.id}
                  batch={entry.batch}
                  childJobs={entry.children}
                  expanded={expandedBatches.has(entry.batch.id)}
                  onToggle={() => toggleBatch(entry.batch.id)}
                  onDelete={() => handleDeleteBatch(entry.batch.id)}
                  onOpenChild={(job) => setSelectedJob(job)}
                  onBrowseChild={(job) => setBrowserJob(job)}
                  onExportChild={handleExport}
                  onDeleteChild={handleDelete}
                  onCrawlAgainChild={onCrawlAgain}
                  onOpenFolderChild={handleOpenFolder}
                />
              ) : (
                <JobCard
                  key={entry.job.id}
                  job={entry.job}
                  onOpen={setSelectedJob}
                  onBrowse={setBrowserJob}
                  onExport={handleExport}
                  onDelete={handleDelete}
                  onCrawlAgain={onCrawlAgain}
                  onOpenFolder={handleOpenFolder}
                />
              ),
            )}
          </div>
        )}
      </div>

      {/* Detail Panel */}
      <AnimatePresence>
        {selectedJob && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 bg-black/40 z-40"
              onClick={() => setSelectedJob(null)}
            />
            <motion.div
              initial={{ x: 300, opacity: 0 }}
              animate={{ x: 0, opacity: 1 }}
              exit={{ x: 300, opacity: 0 }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className="fixed right-0 top-0 h-full w-[480px] bg-deepVoid border-l border-abyssal/50 z-50 flex flex-col shadow-2xl"
            >
              {/* Header */}
              <div className="h-14 flex items-center justify-between px-5 border-b border-abyssal/50">
                <h2 className="text-ghost font-semibold text-base">Job Details</h2>
                <button
                  onClick={() => setSelectedJob(null)}
                  className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                >
                  <X size={18} />
                </button>
              </div>

              {/* Content */}
              <div className="flex-1 overflow-y-auto p-5 space-y-6">
                {/* Job Info */}
                <div className="space-y-3">
                  <div>
                    <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">ID</label>
                    <p className="text-sm text-ghost font-mono">{selectedJob.id}</p>
                  </div>
                  <div>
                    <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">URL</label>
                    <p className="text-sm text-ghost break-all">{selectedJob.url}</p>
                  </div>
                  <div className="flex items-center space-x-4">
                    <div>
                      <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">Status</label>
                      <div className="mt-1"><StatusBadge status={selectedJob.status} /></div>
                    </div>
                    {selectedJob.startTime && (
                      <div>
                        <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">Started</label>
                        <p className="text-sm text-ghost">{new Date(selectedJob.startTime).toLocaleString()}</p>
                      </div>
                    )}
                    {selectedJob.endTime && (
                      <div>
                        <label className="text-[11px] font-medium uppercase tracking-wider text-charcoal">Ended</label>
                        <p className="text-sm text-ghost">{new Date(selectedJob.endTime).toLocaleString()}</p>
                      </div>
                    )}
                  </div>
                </div>

                {/* Results */}
                <div>
                  <h3 className="text-ghost font-semibold text-sm mb-3">
                    Pages ({selectedJob.results?.length || 0})
                  </h3>
                  {(!selectedJob.results || selectedJob.results.length === 0) ? (
                    <p className="text-charcoal text-sm">No pages crawled yet.</p>
                  ) : (
                    <div className="space-y-3">
                      {selectedJob.results.map((page, idx) => (
                        <div key={idx} className="bg-surface/30 border border-abyssal/50 rounded-lg p-3">
                          <div className="flex items-center justify-between mb-1">
                            <p className="text-sm text-ghost font-medium truncate pr-2" title={page.url}>
                              {page.title || page.url}
                            </p>
                            <span className="text-[10px] text-charcoal bg-abyssal/50 px-1.5 py-0.5 rounded">
                              {page.status}
                            </span>
                          </div>
                          <p className="text-xs text-charcoal break-all mb-2">{page.url}</p>
                          <div className="flex items-center space-x-4 text-xs text-charcoal">
                            <span>{page.linksCount} links</span>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </motion.div>
          </>
        )}
      </AnimatePresence>

      {/* Result Browser Overlay */}
      <AnimatePresence>
        {browserJob && (
          <ResultBrowser
            job={browserJob}
            onClose={() => setBrowserJob(null)}
          />
        )}
      </AnimatePresence>

      {/* Export Modal */}
      {exportJobId && (
        <ExportModal
          jobId={exportJobId}
          onClose={() => setExportJobId(null)}
        />
      )}
    </div>
  );
}

interface JobCardProps {
  job: CrawlJob;
  onOpen: (job: CrawlJob) => void;
  onBrowse: (job: CrawlJob) => void;
  onExport: (job: CrawlJob) => void;
  onDelete: (jobId: string) => void;
  onCrawlAgain: (job: CrawlJob) => void;
  onOpenFolder: (outputDir: string) => void;
  /** When true, the card is rendered inside a batch group with a subtle
   * indentation instead of a top-level border. */
  nested?: boolean;
}

function JobCard({
  job,
  onOpen,
  onBrowse,
  onExport,
  onDelete,
  onCrawlAgain,
  onOpenFolder,
  nested,
}: JobCardProps) {
  return (
    <div
      className={
        nested
          ? 'bg-surface/20 border border-abyssal/30 rounded-lg p-3 hover:border-abyssal hover:bg-surface/50 transition-all duration-fast group'
          : 'bg-surface/30 border border-abyssal/50 rounded-lg p-4 hover:border-abyssal hover:bg-surface/80 hover:scale-[1.01] transition-all duration-fast group'
      }
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <div className="flex items-center space-x-2 mb-1">
            <StatusIcon status={job.status} />
            <h3 className="text-ghost font-medium text-sm truncate">{job.url}</h3>
          </div>
          <div className="flex items-center space-x-4 text-xs text-charcoal">
            <span>ID: {job.id.slice(0, 8)}...</span>
            <span>Pages: {job.results?.length || 0}</span>
            {job.startTime && (
              <span>
                {new Date(job.startTime).toLocaleDateString()}{' '}
                {new Date(job.startTime).toLocaleTimeString()}
              </span>
            )}
          </div>
        </div>

        <div className="flex items-center space-x-2 opacity-0 group-hover:opacity-100 transition-opacity duration-fast">
          {job.status === 'completed' && job.results.length > 0 && (
            <button
              onClick={() => onBrowse(job)}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all"
            >
              <FileArrowUp size={14} />
              Browse Results
            </button>
          )}
          {(job.status === 'completed' || job.status === 'failed' || job.status === 'cancelled') && (
            <button
              onClick={() => onCrawlAgain(job)}
              className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
              title="Crawl again with the same settings"
            >
              <ArrowClockwise size={16} />
            </button>
          )}
          <button
            onClick={() => onOpen(job)}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
            title="View Details"
          >
            <Eye size={16} />
          </button>
          {job.status === 'completed' && (
            <button
              onClick={() => onExport(job)}
              className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
              title="Export"
            >
              <Download size={16} />
            </button>
          )}
          <button
            onClick={() => onOpenFolder(job.config.outputDir)}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
            title="Open output folder"
          >
            <FolderOpen size={16} />
          </button>
          <button
            onClick={() => onDelete(job.id)}
            className="p-1.5 text-charcoal hover:text-crimson hover:bg-crimson/10 rounded transition-colors"
            title="Delete"
          >
            <Trash size={16} />
          </button>
        </div>
      </div>

      {job.status === 'running' && (
        <div className="mt-3">
          <div className="h-1.5 bg-surface/50 rounded-full overflow-hidden">
            <div
              className="h-full bg-accentGreen rounded-full transition-all duration-slow"
              style={{
                width: `${Math.min(
                  ((job.progress?.pagesCrawled || 0) / (job.progress?.pageLimit || 1)) * 100,
                  100,
                )}%`,
              }}
            />
          </div>
          <p className="text-[10px] text-charcoal mt-1 truncate">
            {job.progress?.currentUrl || 'Initializing...'}
          </p>
        </div>
      )}
    </div>
  );
}

interface BatchCardProps {
  batch: BatchJob;
  childJobs: CrawlJob[];
  expanded: boolean;
  onToggle: () => void;
  onDelete: () => void;
  onOpenChild: (job: CrawlJob) => void;
  onBrowseChild: (job: CrawlJob) => void;
  onExportChild: (job: CrawlJob) => void;
  onDeleteChild: (jobId: string) => void;
  onCrawlAgainChild: (job: CrawlJob) => void;
  onOpenFolderChild: (outputDir: string) => void;
}

function BatchCard({
  batch,
  childJobs,
  expanded,
  onToggle,
  onDelete,
  onOpenChild,
  onBrowseChild,
  onExportChild,
  onDeleteChild,
  onCrawlAgainChild,
  onOpenFolderChild,
}: BatchCardProps) {
  const total = batch.urls.length;
  const done = Math.min(batch.currentIndex, total);
  const progress = total === 0 ? 0 : (done / total) * 100;
  const displayName = batch.name || `Batch of ${total} URLs`;
  return (
    <div className="bg-surface/30 border border-abyssal/50 rounded-lg overflow-hidden hover:border-abyssal transition-all duration-fast group">
      <div className="p-4 flex items-start justify-between">
        <button
          type="button"
          onClick={onToggle}
          className="flex items-start gap-3 flex-1 min-w-0 text-left"
        >
          <span className="mt-0.5 text-charcoal">
            {expanded ? <CaretDown size={14} /> : <CaretRight size={14} />}
          </span>
          <ListNumbers size={16} className="mt-0.5 text-accentGreen flex-shrink-0" />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-ghost font-medium text-sm truncate">{displayName}</h3>
              <StatusBadge status={batch.status} />
            </div>
            <div className="flex items-center gap-4 text-xs text-charcoal">
              <span>
                {done}/{total} URLs · on-failure: {batch.onFailure}
              </span>
              {batch.createdAt && (
                <span>
                  {new Date(batch.createdAt).toLocaleDateString()}{' '}
                  {new Date(batch.createdAt).toLocaleTimeString()}
                </span>
              )}
            </div>
          </div>
        </button>

        <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity duration-fast">
          <button
            onClick={onDelete}
            className="p-1.5 text-charcoal hover:text-crimson hover:bg-crimson/10 rounded transition-colors"
            title="Delete batch and its child jobs"
          >
            <Trash size={16} />
          </button>
        </div>
      </div>

      <div className="px-4 pb-3">
        <div className="h-1.5 bg-surface/50 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-slow ${
              batch.status === 'failed'
                ? 'bg-crimson'
                : batch.status === 'cancelled'
                ? 'bg-amber'
                : 'bg-accentGreen'
            }`}
            style={{ width: `${Math.min(progress, 100)}%` }}
          />
        </div>
        {batch.error && (
          <p className="text-[11px] text-crimson mt-1 truncate">{batch.error}</p>
        )}
      </div>

      {expanded && childJobs.length > 0 && (
        <div className="border-t border-abyssal/30 bg-deepVoid/40 p-3 space-y-2">
          {childJobs.map((child) => (
            <JobCard
              key={child.id}
              job={child}
              nested
              onOpen={onOpenChild}
              onBrowse={onBrowseChild}
              onExport={onExportChild}
              onDelete={onDeleteChild}
              onCrawlAgain={onCrawlAgainChild}
              onOpenFolder={onOpenFolderChild}
            />
          ))}
        </div>
      )}
      {expanded && childJobs.length === 0 && (
        <div className="border-t border-abyssal/30 bg-deepVoid/40 px-4 py-3 text-xs text-charcoal">
          No child jobs match the current filter.
        </div>
      )}
    </div>
  );
}
