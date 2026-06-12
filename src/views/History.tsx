import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  FileText,
  Clock,
  CheckCircle,
  Warning,
  FolderOpen,
  Trash,
  MagnifyingGlass,
  Funnel,
} from '@phosphor-icons/react';
import type { CrawlJob } from '../types';

export function HistoryView() {
  const [jobs, setJobs] = useState<CrawlJob[]>([]);
  const [filter, setFilter] = useState('all');
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(true);

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
    // TODO: implement delete command
    setJobs((prev) => prev.filter((j) => j.id !== jobId));
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
            <Clock size={48} className="mb-4 opacity-20" />
            <p>No crawls found</p>
            <p className="text-xs mt-1 opacity-50">
              {search || filter !== 'all' ? 'Try adjusting your filters' : 'Start a new crawl to see history'}
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {filteredJobs.map((job) => (
              <div
                key={job.id}
                className="bg-surface/30 border border-abyssal/50 rounded-lg p-4 hover:border-abyssal hover:bg-surface/50 transition-all duration-fast group"
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
