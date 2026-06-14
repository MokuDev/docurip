import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useCrawlEvents } from '../hooks/useCrawlEvents';
import {
  Play,
  Stop,
  ArrowClockwise,
  Globe,
  FileText,
  Download,
  CheckCircle,
  SpinnerGap,
  Pause,
} from '@phosphor-icons/react';
import type { CrawlConfig, CrawlJob, PageResult } from '../types';

const DEFAULT_CONFIG: CrawlConfig = {
  url: '',
  maxDepth: 2,
  pageLimit: 50,
  downloadAssets: false,
  headlessStrategy: 'never',
  contentSelectors: ['main', 'article', '.content'],
  excludePatterns: [],
  respectRobotsTxt: true,
  outputDir: '',
};

export function NewCrawlView({ prefillUrl }: { prefillUrl?: string }) {
  const [config, setConfig] = useState<CrawlConfig>(DEFAULT_CONFIG);
  const [activeJob, setActiveJob] = useState<CrawlJob | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [isStarting, setIsStarting] = useState(false);
  const [urlError, setUrlError] = useState('');
  const logEndRef = useRef<HTMLDivElement>(null);
  const prevEventCountRef = useRef(0);

  // Auto-scroll logs
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  // Prefill URL from quick start
  useEffect(() => {
    if (prefillUrl) {
      setConfig(prev => {
        if (prev.url) return prev;
        return { ...prev, url: prefillUrl };
      });
    }
  }, [prefillUrl]);

  const { events, error, clearError } = useCrawlEvents();

  // React to global events for the active job
  useEffect(() => {
    if (!activeJob) return;
    const jobEvents = events.filter((e) => e.jobId === activeJob.id);
    if (jobEvents.length === 0) return;

    const prevLen = prevEventCountRef.current;
    if (jobEvents.length <= prevLen) return;

    const newEvents = jobEvents.slice(prevLen);
    prevEventCountRef.current = jobEvents.length;

    let jobChanged = false;

    for (const latest of newEvents) {
      if (latest.type === 'log') {
        setLogs((prev) => [...prev, `${new Date().toLocaleTimeString()} ${latest.message || ''}`]);
      } else if (latest.type === 'progress') {
        prevEventCountRef.current; // keep tracking
      } else if (latest.type === 'pageComplete') {
        jobChanged = true;
      } else if (latest.type === 'jobStatusChanged') {
        jobChanged = true;
      }
    }

    if (jobChanged || newEvents.some((e) => e.type === 'progress' || e.type === 'pageComplete' || e.type === 'jobStatusChanged')) {
      setActiveJob((prev) => {
        if (!prev) return prev;
        const updated = { ...prev };
        const latestProgress = newEvents.filter((e) => e.type === 'progress').pop()?.progress;
        if (latestProgress) {
          updated.progress = latestProgress;
        }
        const latestPage = newEvents.filter((e) => e.type === 'pageComplete').map((e) => e.page).filter(Boolean) as PageResult[];
        if (latestPage.length > 0) {
          updated.results = [...prev.results, ...latestPage];
        }
        const latestStatus = newEvents.filter((e) => e.type === 'jobStatusChanged').pop()?.status;
        if (latestStatus) {
          updated.status = latestStatus;
        }
        return updated;
      });
    }
  }, [events, activeJob?.id]);

  const validateUrl = (url: string): boolean => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const handleStart = async () => {
    clearError();
    if (!validateUrl(config.url)) {
      setUrlError('Please enter a valid URL (e.g., https://example.com)');
      return;
    }
    setUrlError('');
    setIsStarting(true);
    setLogs([]);

    try {
      const jobId: string = await invoke('start_crawl', {
        url: config.url,
        config: {
          maxDepth: config.maxDepth,
          pageLimit: config.pageLimit,
          downloadAssets: config.downloadAssets,
              headlessStrategy: config.headlessStrategy,
              contentSelectors: config.contentSelectors.filter(Boolean),
              excludePatterns: config.excludePatterns.filter(Boolean),
              respectRobotsTxt: config.respectRobotsTxt,
              outputDir: config.outputDir,
            },
      });

      const job: CrawlJob = await invoke('get_job', { jobId });
      setActiveJob(job);
      setLogs((prev) => [...prev, `Started crawl: ${jobId}`]);
    } catch (err) {
      setLogs((prev) => [...prev, `Error starting crawl: ${String(err)}`]);
    } finally {
      setIsStarting(false);
    }
  };

  const handleCancel = async () => {
    if (!activeJob) return;
    try {
      await invoke('stop_crawl', { jobId: activeJob.id });
    } catch (err) {
      console.error('Failed to cancel crawl', err);
    }
  };

  const handlePause = async () => {
    if (!activeJob) return;
    try {
      await invoke('pause_crawl', { jobId: activeJob.id });
      setLogs((prev) => [...prev, `Paused crawl: ${activeJob.id}`]);
    } catch (err) {
      setLogs((prev) => [...prev, `Error pausing crawl: ${String(err)}`]);
    }
  };

  const handleResume = async () => {
    if (!activeJob) return;
    try {
      await invoke('resume_crawl', { jobId: activeJob.id });
      setLogs((prev) => [...prev, `Resumed crawl: ${activeJob.id}`]);
    } catch (err) {
      setLogs((prev) => [...prev, `Error resuming crawl: ${String(err)}`]);
    }
  };

  const handleReset = () => {
    setActiveJob(null);
    setLogs([]);
    setConfig(DEFAULT_CONFIG);
  };

  return (
    <div className="h-full flex">
      {/* Left: Config Panel */}
      <div className="w-[420px] flex-shrink-0 border-r border-abyssal/50 bg-deepVoid/30 flex flex-col">
        <div className="h-14 flex items-center px-5 border-b border-abyssal/50">
          <h1 className="text-ghost font-semibold text-base">New Crawl</h1>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {/* URL */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Target URL
            </label>
            <div className="relative">
              <Globe size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal" />
              <input
                type="url"
                value={config.url}
                onChange={(e) => setConfig({ ...config, url: e.target.value })}
                placeholder="https://docs.example.com"
                disabled={!!activeJob}
                className="w-full bg-surface/50 border border-abyssal rounded-md pl-9 pr-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all"
              />
            </div>
            {urlError && <p className="text-crimson text-xs mt-1">{urlError}</p>}
          </div>

          {/* Output Dir */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Output Directory
            </label>
            <input
              type="text"
              value={config.outputDir}
              onChange={(e) => setConfig({ ...config, outputDir: e.target.value })}
              placeholder="Leave empty for default"
              disabled={!!activeJob}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all"
            />
          </div>

          {/* Limits */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Max Depth
              </label>
              <input
                type="number"
                min={1}
                max={10}
                value={config.maxDepth}
                onChange={(e) => setConfig({ ...config, maxDepth: parseInt(e.target.value) || 1 })}
                disabled={!!activeJob}
                className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
              />
            </div>
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Page Limit
              </label>
              <input
                type="number"
                min={1}
                max={10000}
                value={config.pageLimit}
                onChange={(e) => setConfig({ ...config, pageLimit: parseInt(e.target.value) || 1 })}
                disabled={!!activeJob}
                className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
              />
            </div>
          </div>

          {/* Options */}
          <div className="space-y-3">
            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={config.downloadAssets}
                onChange={(e) => setConfig({ ...config, downloadAssets: e.target.checked })}
                disabled={!!activeJob}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">Download images & stylesheets</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={config.respectRobotsTxt}
                onChange={(e) => setConfig({ ...config, respectRobotsTxt: e.target.checked })}
                disabled={!!activeJob}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">Respect robots.txt</span>
            </label>
          </div>

          {/* Headless */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Headless Strategy
            </label>
            <select
              value={config.headlessStrategy}
              onChange={(e) => setConfig({ ...config, headlessStrategy: e.target.value as any })}
              disabled={!!activeJob}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
            >
              <option value="never">Disabled (raw HTML)</option>
              <option value="auto">JS-rendered pages only</option>
              <option value="always">All pages headless</option>
            </select>
          </div>

          {/* Content Selectors */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Content selectors (one per line)
            </label>
            <textarea
              value={config.contentSelectors.join('\n')}
              onChange={(e) =>
                setConfig({ ...config, contentSelectors: e.target.value.split('\n') })
              }
              disabled={!!activeJob}
              rows={3}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
              placeholder="main&#10;article&#10;.content"
            />
          </div>

          {/* Exclude Patterns */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Exclude patterns (one per line)
            </label>
            <textarea
              value={config.excludePatterns.join('\n')}
              onChange={(e) =>
                setConfig({ ...config, excludePatterns: e.target.value.split('\n') })
              }
              disabled={!!activeJob}
              rows={2}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
              placeholder="/admin/*&#10;*.pdf"
            />
          </div>
        </div>

        {/* Action Bar */}
        <div className="h-16 border-t border-abyssal/50 px-5 flex items-center space-x-3">
          {!activeJob ? (
            <button
              onClick={handleStart}
              disabled={isStarting}
              className="flex-1 bg-accentGreen hover:bg-brightGreen text-deepVoid font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast hover:shadow-[0_0_15px_rgba(22,224,141,0.3)] disabled:opacity-50"
            >
              {isStarting ? (
                <SpinnerGap className="animate-spin" size={18} />
              ) : (
                <Play weight="fill" size={18} />
              )}
              <span>{isStarting ? 'Starting...' : 'Start Crawl'}</span>
            </button>
          ) : (
            <>
              {activeJob.status === 'running' && (
                <button
                  onClick={handlePause}
                  className="flex-1 bg-amber/80 hover:bg-amber text-deepVoid font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast"
                >
                  <Pause weight="fill" size={18} />
                  <span>Pause</span>
                </button>
              )}
              {activeJob.status === 'paused' && (
                <button
                  onClick={handleResume}
                  className="flex-1 bg-accentGreen/80 hover:bg-accentGreen text-deepVoid font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast"
                >
                  <Play weight="fill" size={18} />
                  <span>Resume</span>
                </button>
              )}
              <button
                onClick={handleCancel}
                className="px-4 py-2.5 bg-crimson/80 hover:bg-crimson text-ghost font-semibold rounded-md flex items-center space-x-2 transition-all duration-fast"
              >
                <Stop weight="fill" size={16} />
                <span>Cancel</span>
              </button>
              <button
                onClick={handleReset}
                className="px-4 py-2.5 bg-surface hover:bg-abyssal text-secondary hover:text-ghost border border-abyssal rounded-md flex items-center space-x-2 transition-all duration-fast"
              >
                <ArrowClockwise size={16} />
                <span>New</span>
              </button>
            </>
          )}
        </div>
      </div>

      {/* Right: Live Monitor */}
      <div className="flex-1 flex flex-col bg-[#050a0f]">
        {error && (
          <div className="bg-crimson/10 border-l-4 border-crimson text-crimson px-4 py-3 m-4 mb-0 rounded flex justify-between items-center">
            <span className="text-sm">{error}</span>
            <button onClick={clearError} className="font-bold text-crimson hover:text-ghost ml-2">&times;</button>
          </div>
        )}
        <div className="h-14 flex items-center px-5 border-b border-abyssal/50">
          <h2 className="text-ghost font-semibold text-base flex items-center">
            <FileText weight="fill" size={18} className="text-accentGreen mr-2" />
            Live Monitor
          </h2>
          {activeJob && (
            <div className="ml-auto flex items-center space-x-3">
              <StatusBadge status={activeJob.status} />
              {activeJob.status === 'running' && (
                <SpinnerGap className="animate-spin text-accentGreen" size={16} />
              )}
            </div>
          )}
        </div>

        {/* Progress */}
        {activeJob && (
          <div className="px-5 py-4 border-b border-abyssal/30">
            <div className="flex items-center justify-between text-xs text-charcoal mb-2">
              <span>
                Pages: {activeJob.progress.pagesCrawled} / {activeJob.progress.pageLimit}
              </span>
              <span>
                Depth: {activeJob.progress.depth} / {activeJob.progress.maxDepth}
              </span>
            </div>
            <div className="h-2 bg-surface/50 rounded-full overflow-hidden">
              <div
                className="h-full bg-accentGreen rounded-full transition-all duration-slow"
                style={{
                  width: `${Math.min(
                    (activeJob.progress.pagesCrawled / activeJob.progress.pageLimit) * 100,
                    100
                  )}%`,
                }}
              />
            </div>
            {activeJob.progress.currentUrl && (
              <p className="text-xs text-charcoal mt-2 truncate">
                <span className="text-secondary">Current:</span>{' '}
                {activeJob.progress.currentUrl}
              </p>
            )}
          </div>
        )}

        {/* Stats */}
        {activeJob && (
          <div className="grid grid-cols-3 border-b border-abyssal/30">
            <StatBox
              icon={<FileText weight="fill" size={16} className="text-accentGreen" />}
              label="Pages"
              value={activeJob.results.length}
            />
            <StatBox
              icon={<Download size={16} className="text-cyberBlue" />}
              label="Assets"
              value={activeJob.results.reduce((sum, r) => sum + r.assets.length, 0)}
            />
            <StatBox
              icon={<CheckCircle weight="fill" size={16} className="text-brightGreen" />}
              label="Links"
              value={activeJob.results.reduce((sum, r) => sum + r.links.length, 0)}
            />
          </div>
        )}

        {/* Logs */}
        <div className="flex-1 overflow-hidden flex flex-col">
          <div className="px-5 py-2 border-b border-abyssal/30 text-[11px] font-medium uppercase tracking-wider text-charcoal">
            Logs
          </div>
          <div className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1">
            {logs.length === 0 ? (
              <p className="text-charcoal/40">No logs yet. Start a crawl to see activity.</p>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="text-secondary break-all">
                  <span className="text-charcoal/50">{log.split(' ')[0]}</span>{' '}
                  {log.slice(log.indexOf(' ') + 1)}
                </div>
              ))
            )}
            <div ref={logEndRef} />
          </div>
        </div>
      </div>
    </div>
  );
}

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

const StatBox = ({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
}) => (
  <div className="flex items-center px-5 py-3 border-r border-abyssal/30 last:border-r-0">
    <div className="mr-3">{icon}</div>
    <div>
      <div className="text-lg font-mono font-semibold text-ghost">{value}</div>
      <div className="text-[10px] text-charcoal uppercase tracking-wider">{label}</div>
    </div>
  </div>
);

