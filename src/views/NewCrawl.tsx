import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Play,
  Stop,
  ArrowClockwise,
  Globe,
  FileText,
  CheckCircle,
  SpinnerGap,
  Pause,
  TreeStructure,
  X,
  ListNumbers,
} from '@phosphor-icons/react';
import type {
  AppSettings,
  BatchFailureMode,
  BatchJob,
  CrawlConfig,
  CrawlJob,
  CrawlProfile,
  CrawlTemplate,
} from '../types';
import { CRAWL_PROFILES } from '../types';
import { StatusBadge } from '../components/StatusBadge';
import { TemplateBar } from '../components/TemplateBar';
import { SitemapPickerModal } from '../components/SitemapPickerModal';
import { BatchUrlList, sanitizeBatchUrls } from '../components/BatchUrlList';

const MAX_LOGS = 500;

const DEFAULT_CONFIG: CrawlConfig = {
  url: '',
  maxDepth: 2,
  pageLimit: 1000,
  downloadAssets: false,
  headlessStrategy: 'never',
  contentSelectors: ['main', 'article', '.content'],
  excludePatterns: [],
  includePatterns: [],
  pathPrefix: '',
  respectRobotsTxt: true,
  stayWithinDomain: true,
  ssrfProtection: true,
  outputDir: '',
  profile: null,
};

const applyProfile = (profileId: CrawlProfile, current: CrawlConfig): CrawlConfig => {
  const profile = CRAWL_PROFILES.find((p) => p.id === profileId);
  if (!profile) return current;
  return {
    ...current,
    profile: profileId,
    maxDepth: profile.defaultMaxDepth,
    pageLimit: profile.defaultPageLimit,
    respectRobotsTxt: profile.defaultRespectRobotsTxt,
  };
};

/** Normalizes UI-only config shape (raw textarea lines, unenforced pathPrefix)
 * into the payload the backend's CrawlConfig expects — shared by start_crawl
 * and save_template so both stay in sync as fields are added. */
const toBackendConfig = (config: CrawlConfig) => ({
  maxDepth: config.maxDepth,
  pageLimit: config.pageLimit,
  downloadAssets: config.downloadAssets,
  headlessStrategy: config.headlessStrategy,
  contentSelectors: config.contentSelectors.map((s) => s.trim()).filter(Boolean),
  excludePatterns: config.excludePatterns.map((s) => s.trim()).filter(Boolean),
  includePatterns: config.includePatterns.map((s) => s.trim()).filter(Boolean),
  pathPrefix: config.pathPrefix.trim().replace(/[?#].*$/, '').replace(/^(?!\/)/, '/').replace(/^\/$/, ''),
  respectRobotsTxt: config.respectRobotsTxt,
  stayWithinDomain: config.stayWithinDomain,
  ssrfProtection: config.ssrfProtection,
  outputDir: config.outputDir,
  profile: config.profile,
});

export function NewCrawlView({ prefillUrl, prefillConfig }: { prefillUrl?: string; prefillConfig?: CrawlConfig }) {
  const [config, setConfig] = useState<CrawlConfig>(DEFAULT_CONFIG);
  const [activeJob, setActiveJob] = useState<CrawlJob | null>(null);
  const logsRef = useRef<string[]>([]);
  const [logTick, setLogTick] = useState(0);
  const [isStarting, setIsStarting] = useState(false);
  const [urlError, setUrlError] = useState('');
  const logEndRef = useRef<HTMLDivElement>(null);
  const consecutiveErrors = useRef(0);
  const logs = logsRef.current;
  const [templates, setTemplates] = useState<CrawlTemplate[]>([]);

  // Sitemap discovery + picker
  const [autoDiscover, setAutoDiscover] = useState(true);
  const [discoveredSitemaps, setDiscoveredSitemaps] = useState<string[]>([]);
  const [dismissedDiscoveryFor, setDismissedDiscoveryFor] = useState<string>('');
  const [pickerUrl, setPickerUrl] = useState<string | null>(null);
  const [seedUrls, setSeedUrls] = useState<string[]>([]);
  const discoveryReqRef = useRef(0);

  // Batch mode
  const [mode, setMode] = useState<'single' | 'batch'>('single');
  const [batchUrls, setBatchUrls] = useState<string[]>([]);
  const [batchName, setBatchName] = useState('');
  const [batchOnFailure, setBatchOnFailure] = useState<BatchFailureMode>('continue');
  const [activeBatch, setActiveBatch] = useState<BatchJob | null>(null);

  // Initialize activeJob from sessionStorage on mount
  useEffect(() => {
    if (activeJob) return;
    const storedJobId = sessionStorage.getItem('docurip_active_job');
    if (!storedJobId) return;

    invoke<CrawlJob>('get_job', { jobId: storedJobId })
      .then((job) => {
        if (job.status === 'running' || job.status === 'queued' || job.status === 'paused') {
          setActiveJob(job);
        } else {
          sessionStorage.removeItem('docurip_active_job');
        }
      })
      .catch(() => {
        sessionStorage.removeItem('docurip_active_job');
      });
  }, []);

  const appendLog = (msg: string) => {
    const arr = logsRef.current;
    arr.push(msg);
    if (arr.length > MAX_LOGS) arr.splice(0, arr.length - MAX_LOGS);
    setLogTick((t) => t + 1);
  };

  const clearLogs = () => {
    logsRef.current = [];
    setLogTick((t) => t + 1);
  };

  // Auto-scroll logs
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logTick]);

  // Prefill URL from quick start
  useEffect(() => {
    if (prefillUrl) {
      setConfig((prev) => ({ ...prev, url: prefillUrl }));
    }
  }, [prefillUrl]);

  // Prefill full config from "Crawl Again"
  useEffect(() => {
    if (prefillConfig) {
      setConfig(prefillConfig);
    }
  }, [prefillConfig]);

  const loadTemplates = () => {
    invoke<CrawlTemplate[]>('list_templates')
      .then(setTemplates)
      .catch((err) => console.warn('Failed to load templates:', err));
  };

  useEffect(() => {
    loadTemplates();
    invoke<AppSettings>('get_settings')
      .then((s) => {
        setAutoDiscover(s.sitemapAutoDiscover ?? true);
        setBatchOnFailure(s.batchOnFailure ?? 'continue');
      })
      .catch(() => {});
  }, []);

  // Debounced sitemap auto-discovery on URL change.
  useEffect(() => {
    if (!autoDiscover || activeJob) return;
    const url = config.url.trim();
    if (!validateUrl(url)) {
      setDiscoveredSitemaps([]);
      return;
    }
    if (dismissedDiscoveryFor === url) return;
    const reqId = ++discoveryReqRef.current;
    const timeout = setTimeout(() => {
      invoke<string[]>('discover_sitemap', { url, ssrfProtection: config.ssrfProtection })
        .then((found) => {
          if (reqId === discoveryReqRef.current) setDiscoveredSitemaps(found);
        })
        .catch(() => {
          if (reqId === discoveryReqRef.current) setDiscoveredSitemaps([]);
        });
    }, 700);
    return () => clearTimeout(timeout);
  }, [config.url, config.ssrfProtection, autoDiscover, activeJob, dismissedDiscoveryFor]);

  const handleSitemapConfirm = (urls: string[]) => {
    if (urls.length === 0) {
      setPickerUrl(null);
      return;
    }
    setDiscoveredSitemaps([]);
    setPickerUrl(null);
    if (urls.length === 1) {
      setConfig((prev) => ({ ...prev, url: urls[0] }));
      setSeedUrls([]);
      appendLog(`Loaded 1 URL from sitemap as start URL.`);
      return;
    }
    // Multi-URL selection → batch mode.
    setMode('batch');
    setBatchUrls(urls);
    setSeedUrls([]);
    appendLog(`Loaded ${urls.length} URLs from sitemap into batch queue.`);
  };

  const handleApplyTemplate = (template: CrawlTemplate) => {
    setConfig({ ...template.config, url: template.url });
  };

  const handleSaveTemplate = async (name: string) => {
    if (!validateUrl(config.url)) {
      setUrlError('Please enter a valid URL before saving a template');
      return;
    }
    try {
      await invoke('save_template', { name, url: config.url, config: toBackendConfig(config) });
      loadTemplates();
    } catch (err) {
      appendLog(`Error saving template: ${String(err)}`);
    }
  };

  const handleDeleteTemplate = async (templateId: string) => {
    try {
      await invoke('delete_template', { templateId });
      setTemplates((prev) => prev.filter((t) => t.id !== templateId));
    } catch (err) {
      console.warn('Failed to delete template:', err);
    }
  };

  useEffect(() => {
    if (!activeJob) return;
    consecutiveErrors.current = 0;
    const id = setInterval(async () => {
      try {
        const job: CrawlJob = await invoke('get_job', { jobId: activeJob.id });
        consecutiveErrors.current = 0;
        setActiveJob(job);
        // Clear sessionStorage if job is done
        if (job.status === 'completed' || job.status === 'failed' || job.status === 'cancelled') {
          sessionStorage.removeItem('docurip_active_job');
        }
      } catch (err) {
        consecutiveErrors.current++;
        console.warn('[NewCrawl] get_job polling failed:', err);
        if (consecutiveErrors.current >= 3) {
          clearInterval(id);
          setActiveJob(prev => prev ? { ...prev, status: 'failed' } : prev);
          sessionStorage.removeItem('docurip_active_job');
        }
      }
    }, 2000);
    return () => clearInterval(id);
  }, [activeJob?.id]);

  // Restore active batch from sessionStorage on mount.
  useEffect(() => {
    if (activeBatch) return;
    const storedBatchId = sessionStorage.getItem('docurip_active_batch');
    if (!storedBatchId) return;
    invoke<BatchJob>('get_batch', { batchId: storedBatchId })
      .then((batch) => {
        if (batch.status === 'running' || batch.status === 'queued') {
          setActiveBatch(batch);
          setMode('batch');
        } else {
          sessionStorage.removeItem('docurip_active_batch');
        }
      })
      .catch(() => sessionStorage.removeItem('docurip_active_batch'));
  }, []);

  // Poll active batch, and mirror its current child job into the live
  // monitor so per-page progress remains visible.
  useEffect(() => {
    if (!activeBatch) return;
    const id = setInterval(async () => {
      try {
        const batch: BatchJob = await invoke('get_batch', { batchId: activeBatch.id });
        setActiveBatch(batch);
        // Track the current child job for the Live Monitor pane.
        const currentChildId = batch.childJobIds[batch.currentIndex];
        if (currentChildId && currentChildId !== activeJob?.id) {
          try {
            const job: CrawlJob = await invoke('get_job', { jobId: currentChildId });
            setActiveJob(job);
          } catch { /* child not yet visible */ }
        } else if (activeJob?.id) {
          try {
            const job: CrawlJob = await invoke('get_job', { jobId: activeJob.id });
            setActiveJob(job);
          } catch { /* ignore */ }
        }
        if (batch.status === 'completed' || batch.status === 'failed' || batch.status === 'cancelled') {
          sessionStorage.removeItem('docurip_active_batch');
        }
      } catch (err) {
        console.warn('[NewCrawl] get_batch polling failed:', err);
      }
    }, 1500);
    return () => clearInterval(id);
  }, [activeBatch?.id]);

  const validateUrl = (url: string): boolean => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const handleStart = async () => {
    if (mode === 'batch') {
      await handleStartBatch();
      return;
    }
    if (!validateUrl(config.url)) {
      setUrlError('Please enter a valid URL (e.g., https://example.com)');
      return;
    }
    setUrlError('');
    setIsStarting(true);
    clearLogs();

    try {
      const jobId: string = await invoke('start_crawl', {
        url: config.url,
        config: toBackendConfig(config),
      });

      const job: CrawlJob = await invoke('get_job', { jobId });
      setActiveJob(job);
      sessionStorage.setItem('docurip_active_job', jobId);
      appendLog(`Started crawl: ${jobId}`);
    } catch (err) {
      appendLog(`Error starting crawl: ${String(err)}`);
    } finally {
      setIsStarting(false);
    }
  };

  const handleStartBatch = async () => {
    const urls = sanitizeBatchUrls(batchUrls);
    if (urls.length === 0) {
      setUrlError('Enter at least one valid http(s) URL.');
      return;
    }
    setUrlError('');
    setIsStarting(true);
    clearLogs();

    try {
      const batchId: string = await invoke('start_batch', {
        urls,
        config: toBackendConfig(config),
        name: batchName.trim() || null,
        onFailure: batchOnFailure,
      });
      const batch: BatchJob = await invoke('get_batch', { batchId });
      setActiveBatch(batch);
      sessionStorage.setItem('docurip_active_batch', batchId);
      appendLog(`Started batch: ${urls.length} URLs (${batchOnFailure} on failure).`);
    } catch (err) {
      appendLog(`Error starting batch: ${String(err)}`);
    } finally {
      setIsStarting(false);
    }
  };

  const handleCancel = async () => {
    if (activeBatch) {
      try {
        await invoke('stop_batch', { batchId: activeBatch.id });
        appendLog(`Cancelling batch: ${activeBatch.id}`);
      } catch (err) {
        console.error('Failed to stop batch', err);
      }
      return;
    }
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
      appendLog(`Paused crawl: ${activeJob.id}`);
    } catch (err) {
      appendLog(`Error pausing crawl: ${String(err)}`);
    }
  };

  const handleResume = async () => {
    if (!activeJob) return;
    try {
      await invoke('resume_crawl', { jobId: activeJob.id });
      appendLog(`Resumed crawl: ${activeJob.id}`);
    } catch (err) {
      appendLog(`Error resuming crawl: ${String(err)}`);
    }
  };

  const handleReset = () => {
    setActiveJob(null);
    setActiveBatch(null);
    clearLogs();
    setConfig(DEFAULT_CONFIG);
    setBatchUrls([]);
    setBatchName('');
    setSeedUrls([]);
    setMode('single');
  };

  const isBusy = !!activeJob || !!activeBatch;
  const inactiveJobFor = (s: CrawlJob['status'] | undefined) =>
    s === 'completed' || s === 'failed' || s === 'cancelled' || s === undefined;

  return (
    <div className="h-full flex">
      {pickerUrl && (
        <SitemapPickerModal
          sitemapUrl={pickerUrl}
          ssrfProtection={config.ssrfProtection}
          onClose={() => setPickerUrl(null)}
          onConfirm={handleSitemapConfirm}
        />
      )}
      {/* Left: Config Panel */}
      <div className="w-[420px] flex-shrink-0 border-r border-abyssal/50 bg-deepVoid/30 flex flex-col">
        <div className="h-14 flex items-center px-5 border-b border-abyssal/50">
          <h1 className="text-ghost font-semibold text-base">New Crawl</h1>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {/* Mode toggle */}
          <div className="grid grid-cols-2 gap-1 p-1 bg-surface/50 border border-abyssal rounded-md">
            <button
              type="button"
              disabled={isBusy}
              onClick={() => setMode('single')}
              className={`flex items-center justify-center gap-1.5 py-1.5 px-3 rounded text-xs font-medium transition-all ${
                mode === 'single'
                  ? 'bg-accentGreen/20 text-accentGreen'
                  : 'text-charcoal hover:text-ghost'
              } disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              <Globe size={12} />
              Single URL
            </button>
            <button
              type="button"
              disabled={isBusy}
              onClick={() => setMode('batch')}
              className={`flex items-center justify-center gap-1.5 py-1.5 px-3 rounded text-xs font-medium transition-all ${
                mode === 'batch'
                  ? 'bg-accentGreen/20 text-accentGreen'
                  : 'text-charcoal hover:text-ghost'
              } disabled:opacity-50 disabled:cursor-not-allowed`}
            >
              <ListNumbers size={12} />
              Batch
              {batchUrls.filter((u) => u.trim()).length > 0 && (
                <span className="ml-1 text-[10px] text-charcoal">
                  {batchUrls.filter((u) => u.trim()).length}
                </span>
              )}
            </button>
          </div>

          {mode === 'single' && (
          <>
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
                disabled={isBusy}
                className="w-full bg-surface/50 border border-abyssal rounded-md pl-9 pr-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all"
              />
            </div>
          {urlError && <p className="text-crimson text-xs mt-1">{urlError}</p>}

          {/* Sitemap discovery banner */}
          {!activeJob && discoveredSitemaps.length > 0 && (
            <div className="mt-2 flex items-start gap-2 px-3 py-2 bg-accentGreen/10 border border-accentGreen/30 rounded-md">
              <TreeStructure size={14} className="text-accentGreen mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-xs text-ghost">
                  Sitemap found for this site
                  {discoveredSitemaps.length > 1 ? ` (${discoveredSitemaps.length} sources)` : ''}
                </p>
                <button
                  onClick={() => setPickerUrl(discoveredSitemaps[0])}
                  className="text-xs text-accentGreen hover:text-brightGreen font-medium mt-0.5"
                >
                  Import URLs →
                </button>
              </div>
              <button
                onClick={() => {
                  setDismissedDiscoveryFor(config.url.trim());
                  setDiscoveredSitemaps([]);
                }}
                className="text-charcoal hover:text-ghost transition-colors flex-shrink-0"
                aria-label="Dismiss"
              >
                <X size={14} />
              </button>
            </div>
          )}

          {/* Seed URLs preview — switch into batch mode to consume them. */}
          {seedUrls.length > 0 && (
            <div className="mt-2 flex items-center justify-between gap-2 px-3 py-2 bg-surface/50 border border-abyssal rounded-md">
              <span className="text-xs text-charcoal truncate">
                <span className="text-accentGreen">+{seedUrls.length}</span> URLs queued
              </span>
              <div className="flex items-center gap-2 flex-shrink-0">
                <button
                  onClick={() => {
                    const startUrl = config.url.trim();
                    setBatchUrls(startUrl ? [startUrl, ...seedUrls] : seedUrls);
                    setSeedUrls([]);
                    setMode('batch');
                  }}
                  className="text-xs text-accentGreen hover:text-brightGreen transition-colors"
                >
                  Use as batch
                </button>
                <button
                  onClick={() => setSeedUrls([])}
                  className="text-charcoal hover:text-ghost transition-colors"
                  aria-label="Clear queued URLs"
                >
                  <X size={12} />
                </button>
              </div>
            </div>
          )}

          {/* Manual "Load sitemap" trigger */}
          {!activeJob && validateUrl(config.url) && (
            <button
              type="button"
              onClick={() => {
                const url = config.url.trim();
                try {
                  const u = new URL(url);
                  setPickerUrl(`${u.origin}/sitemap.xml`);
                } catch {
                  /* invalid URL — button is only shown when valid */
                }
              }}
              className="mt-2 flex items-center gap-1.5 text-xs text-charcoal hover:text-accentGreen transition-colors"
            >
              <TreeStructure size={12} />
              Load sitemap manually
            </button>
          )}
        </div>
          </>
          )}

          {mode === 'batch' && (
          <>
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Batch name (optional)
            </label>
            <input
              type="text"
              value={batchName}
              onChange={(e) => setBatchName(e.target.value)}
              disabled={isBusy}
              placeholder="e.g., API docs v1 + v2"
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all"
            />
          </div>

          <BatchUrlList
            value={batchUrls}
            onChange={setBatchUrls}
            disabled={isBusy}
          />

          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              On failure
            </label>
            <select
              value={batchOnFailure}
              onChange={(e) => setBatchOnFailure(e.target.value as BatchFailureMode)}
              disabled={isBusy}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
            >
              <option value="continue">Continue with the next URL</option>
              <option value="stop">Stop the batch</option>
            </select>
          </div>
          {urlError && <p className="text-crimson text-xs mt-1">{urlError}</p>}
          </>
          )}

          {/* Templates */}
          <TemplateBar
            templates={templates}
            disabled={isBusy}
            onApply={handleApplyTemplate}
            onSave={handleSaveTemplate}
            onDelete={handleDeleteTemplate}
          />

          {/* Profile */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Crawl Profile
            </label>
            <select
              value={config.profile || 'documentation'}
              onChange={(e) => setConfig(applyProfile(e.target.value as CrawlProfile, config))}
              disabled={isBusy}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
            >
              {CRAWL_PROFILES.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
            <p className="text-[11px] text-charcoal mt-1">
              {CRAWL_PROFILES.find((p) => p.id === (config.profile || 'documentation'))?.description}
            </p>
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
                disabled={isBusy}
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
                disabled={isBusy}
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
                disabled={isBusy}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">Download images & stylesheets</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={config.respectRobotsTxt}
                onChange={(e) => setConfig({ ...config, respectRobotsTxt: e.target.checked })}
                disabled={isBusy}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">Respect robots.txt</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={config.stayWithinDomain}
                onChange={(e) => setConfig({ ...config, stayWithinDomain: e.target.checked })}
                disabled={isBusy}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">Stay within domain</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer">
              <input
                type="checkbox"
                checked={config.ssrfProtection}
                onChange={(e) => setConfig({ ...config, ssrfProtection: e.target.checked })}
                disabled={isBusy}
                className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20"
              />
              <span className="text-sm text-secondary">SSRF protection</span>
            </label>
          </div>

          {/* Headless */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Headless Strategy
            </label>
            <select
              value={config.headlessStrategy}
              onChange={(e) => setConfig({ ...config, headlessStrategy: e.target.value as CrawlConfig['headlessStrategy'] })}
              disabled={isBusy}
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
              disabled={isBusy}
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
              disabled={isBusy}
              rows={2}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
              placeholder="/admin/*&#10;*.pdf"
            />
          </div>

          {/* Include Patterns */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Include patterns (one per line)
            </label>
            <textarea
              value={config.includePatterns.join('\n')}
              onChange={(e) =>
                setConfig({ ...config, includePatterns: e.target.value.split('\n') })
              }
              disabled={isBusy}
              rows={2}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
              placeholder="/docs/api/.*&#10;/reference/.*"
            />
            <p className="text-[11px] text-charcoal mt-1">
              Only crawl URLs matching at least one pattern. Leave empty to crawl all.
            </p>
          </div>

          {/* Path Prefix */}
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Path prefix filter
            </label>
            <input
              type="text"
              value={config.pathPrefix}
              onChange={(e) => setConfig({ ...config, pathPrefix: e.target.value })}
              disabled={isBusy}
              placeholder="/docs/api/"
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all"
            />
            <p className="text-[11px] text-charcoal mt-1">
              Only crawl URLs whose path starts with this prefix.
            </p>
          </div>
        </div>

        {/* Action Bar */}
        <div className="h-16 border-t border-abyssal/50 px-5 flex items-center space-x-3">
          {!isBusy ? (
            <button
              onClick={handleStart}
              disabled={isStarting}
              className="flex-1 bg-accentGreen hover:bg-brightGreen text-slate-900 font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast hover:shadow-[0_0_15px_rgba(22,224,141,0.3)] disabled:opacity-50"
            >
              {isStarting ? (
                <SpinnerGap className="animate-spin" size={18} />
              ) : (
                <Play weight="fill" size={18} />
              )}
              <span>
                {isStarting
                  ? 'Starting...'
                  : mode === 'batch'
                  ? `Start Batch${batchUrls.filter((u) => u.trim()).length > 0 ? ` (${batchUrls.filter((u) => u.trim()).length})` : ''}`
                  : 'Start Crawl'}
              </span>
            </button>
          ) : activeBatch ? (
            <>
              <button
                onClick={handleCancel}
                disabled={activeBatch.status !== 'queued' && activeBatch.status !== 'running'}
                className="flex-1 px-4 py-2.5 bg-crimson/80 hover:bg-crimson text-ghost font-semibold rounded-md flex items-center justify-center space-x-2 transition-all duration-fast disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <Stop weight="fill" size={16} />
                <span>Cancel Batch</span>
              </button>
              <button
                onClick={handleReset}
                className="px-4 py-2.5 bg-surface hover:bg-abyssal text-secondary hover:text-ghost border border-abyssal rounded-md flex items-center space-x-2 transition-all duration-fast"
              >
                <ArrowClockwise size={16} />
                <span>New</span>
              </button>
            </>
          ) : (
            <>
              {activeJob!.status === 'running' && (
                <button
                  onClick={handlePause}
                  className="flex-1 bg-amber/80 hover:bg-amber text-slate-900 font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast"
                >
                  <Pause weight="fill" size={18} />
                  <span>Pause</span>
                </button>
              )}
              {activeJob!.status === 'paused' && (
                <button
                  onClick={handleResume}
                  className="flex-1 bg-accentGreen/80 hover:bg-accentGreen text-slate-900 font-semibold py-2.5 px-4 rounded-md flex items-center justify-center space-x-2 transition-all duration-fast"
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
      <div className="flex-1 flex flex-col bg-deepVoid">
        <div className="h-14 flex items-center px-5 border-b border-abyssal/50">
          <h2 className="text-ghost font-semibold text-base flex items-center">
            <FileText weight="fill" size={18} className="text-accentGreen mr-2" />
            Live Monitor
          </h2>
          {activeBatch && (
            <div className="ml-auto flex items-center space-x-3">
              <span className="text-xs text-charcoal">Batch</span>
              <StatusBadge
                status={activeBatch.status as unknown as CrawlJob['status']}
              />
              {(activeBatch.status === 'running' || activeBatch.status === 'queued') && (
                <SpinnerGap className="animate-spin text-accentGreen" size={16} />
              )}
            </div>
          )}
          {!activeBatch && activeJob && (
            <div className="ml-auto flex items-center space-x-3">
              <StatusBadge status={activeJob.status} />
              {activeJob.status === 'running' && (
                <SpinnerGap className="animate-spin text-accentGreen" size={16} />
              )}
            </div>
          )}
        </div>

        {/* Batch progress */}
        {activeBatch && (
          <div className="px-5 py-4 border-b border-abyssal/30">
            <div className="flex items-center justify-between text-xs text-charcoal mb-2">
              <span>
                {activeBatch.name && (
                  <span className="text-ghost font-medium mr-2">{activeBatch.name}</span>
                )}
                Batch: {Math.min(activeBatch.currentIndex, activeBatch.urls.length)} / {activeBatch.urls.length} URLs
              </span>
              <span>
                {inactiveJobFor(activeJob?.status) && activeBatch.status === 'running'
                  ? 'Preparing next…'
                  : ''}
              </span>
            </div>
            <div className="h-2 bg-surface/50 rounded-full overflow-hidden">
              <div
                className={`h-full rounded-full transition-all duration-slow ${
                  activeBatch.status === 'failed'
                    ? 'bg-crimson'
                    : activeBatch.status === 'cancelled'
                    ? 'bg-amber'
                    : 'bg-accentGreen'
                }`}
                style={{
                  width: `${Math.min(
                    (activeBatch.currentIndex / Math.max(activeBatch.urls.length, 1)) * 100,
                    100,
                  )}%`,
                }}
              />
            </div>
            {activeBatch.urls[activeBatch.currentIndex] && activeBatch.status === 'running' && (
              <p className="text-xs text-charcoal mt-2 truncate">
                <span className="text-secondary">Now crawling:</span>{' '}
                {activeBatch.urls[activeBatch.currentIndex]}
              </p>
            )}
            {activeBatch.error && (
              <p className="text-xs text-crimson mt-2 truncate">{activeBatch.error}</p>
            )}
          </div>
        )}

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
              icon={<CheckCircle weight="fill" size={16} className="text-brightGreen" />}
              label="Links"
              value={activeJob.results.reduce((sum, r) => sum + r.linksCount, 0)}
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

