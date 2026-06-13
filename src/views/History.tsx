import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import {
  FileText,
  Clock,
  CheckCircle,
  Warning,
  FolderOpen,
  Trash,
  MagnifyingGlass,
  Funnel,
  X,
  Eye,
  Download,
  FileArrowUp,
} from '@phosphor-icons/react';
import { ResultBrowser } from './ResultBrowser';
import type { CrawlJob } from '../types';

export function HistoryView() {
  const [jobs, setJobs] = useState<CrawlJob[]>([]);
  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);
  const [selectedJob, setSelectedJob] = useState<CrawlJob | null>(null);
  const [browserJob, setBrowserJob] = useState<CrawlJob | null>(null);

  useEffect(() => {
    loadJobs();
  }, []);

  const loadJobs = async () => {
    setLoading(true);
    try {
      const data: CrawlJob[] = await invoke('list_jobs');
      setJobs(data || []);
    } catch (err) {
      console.error('Failed to load jobs', err);
    } finally {
      setLoading(false);
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

  const handleExport = async (jobId: string) => {
    try {
      const path: string = await invoke('export_job', { jobId });
      console.log('Exported to', path);
    } catch (err) {
      console.error('Export failed', err);
    }
  };

  const handleOpenFolder = async (outputDir: string) => {
    try {
      await invoke('open_output_folder', { path: outputDir });
    } catch {
      // fallback: silently fail
    }
  };

  const filteredJobs = jobs
    .filter((j) => {
      if (filter === 'all') return true;
      return j.status === filter;
    })
    .filter((j) => {
      if (!search) return true;
      const term = search.toLowerCase();
      return (
        j.url.toLowerCase().includes(term) ||
        j.id.toLowerCase().includes(term)
      );
    });

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
        ) : filteredJobs.length === 0 ? (
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
            {filteredJobs.map((job) => (
              <div
                key={job.id}
                className="bg-surface/30 border border-abyssal/50 rounded-lg p-4 hover:border-abyssal hover:bg-surface/80 hover:scale-[1.01] transition-all duration-fast group"
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
                        onClick={() => setBrowserJob(job)}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all"
                      >
                        <FileArrowUp size={14} />
                        Browse Results
                      </button>
                    )}
                    <button
                      onClick={() => setSelectedJob(job)}
                      className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                      title="View Details"
                    >
                      <Eye size={16} />
                    </button>
                    {job.status === 'completed' && (
                      <button
                        onClick={() => handleExport(job.id)}
                        className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                        title="Export"
                      >
                        <Download size={16} />
                      </button>
                    )}
                    <button
                      onClick={() => handleOpenFolder(job.config.outputDir)}
                      className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                      title="Open output folder"
                    >
                      <FolderOpen size={16} />
                    </button>
                    <button
                      onClick={() => handleDelete(job.id)}
                      className="p-1.5 text-charcoal hover:text-crimson hover:bg-crimson/10 rounded transition-colors"
                      title="Delete"
                    >
                      <Trash size={16} />
                    </button>
                  </div>
                </div>

                {/* Progress bar for running jobs */}
                {job.status === 'running' && (
                  <div className="mt-3">
                    <div className="h-1.5 bg-surface/50 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-accentGreen rounded-full transition-all duration-slow"
                        style={{
                          width: `${Math.min(
                            ((job.progress?.pagesCrawled || 0) / (job.progress?.pageLimit || 1)) * 100,
                            100
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
            ))}
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
                            <span>{page.links.length} links</span>
                            <span>{page.assets.length} assets</span>
                          </div>
                          {page.links.length > 0 && (
                            <details className="mt-2">
                              <summary className="text-xs text-secondary cursor-pointer hover:text-ghost transition-colors">
                                Links ({page.links.length})
                              </summary>
                              <ul className="mt-1 space-y-0.5 max-h-32 overflow-y-auto">
                                {page.links.map((link, i) => (
                                  <li key={i} className="text-[11px] text-charcoal truncate font-mono">{link}</li>
                                ))}
                              </ul>
                            </details>
                          )}
                          {page.assets.length > 0 && (
                            <details className="mt-2">
                              <summary className="text-xs text-secondary cursor-pointer hover:text-ghost transition-colors">
                                Assets ({page.assets.length})
                              </summary>
                              <ul className="mt-1 space-y-0.5 max-h-32 overflow-y-auto">
                                {page.assets.map((asset, i) => (
                                  <li key={i} className="text-[11px] text-charcoal truncate font-mono">{asset}</li>
                                ))}
                              </ul>
                            </details>
                          )}
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
    </div>
  );
}

const StatusIcon = ({ status }: { status: string }) => {
  switch (status) {
    case 'completed':
      return <CheckCircle weight="fill" size={16} className="text-brightGreen" />;
    case 'running':
      return <FileText weight="fill" size={16} className="text-accentGreen" />;
    case 'failed':
      return <Warning weight="fill" size={16} className="text-crimson" />;
    default:
      return <Clock size={16} className="text-charcoal" />;
  }
};

const StatusBadge = ({ status }: { status: string }) => {
  const styles: Record<string, string> = {
    queued: 'bg-amber/10 text-amber',
    running: 'bg-accentGreen/10 text-accentGreen',
    paused: 'bg-cyberBlue/10 text-cyberBlue',
    completed: 'bg-brightGreen/10 text-brightGreen',
    failed: 'bg-crimson/10 text-crimson',
  };

  return (
    <span
      className={`text-[11px] font-semibold uppercase tracking-wider px-2 py-1 rounded ${styles[status] || 'text-charcoal'}`}
    >
      {status}
    </span>
  );
};
