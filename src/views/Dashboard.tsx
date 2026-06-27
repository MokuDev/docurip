import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useCrawlEvents } from '../hooks/useCrawlEvents';
import {
  Download,
  FileText,
  Warning,
  ArrowRight,
  HardDrive,
  Lightning,
  Archive,
} from '@phosphor-icons/react';
import { StatusIcon, StatusBadge } from '../components/StatusBadge';
import { AnimatedCounter } from '../components/AnimatedCounter';
import type { CrawlJob, DashboardStats, RecentExport } from '../types';

export function DashboardView({ onQuickStart }: { onQuickStart: (url: string) => void }) {
  const [recentJobs, setRecentJobs] = useState<CrawlJob[]>([]);
  const [quickUrl, setQuickUrl] = useState('');
  const [stats, setStats] = useState<DashboardStats>({
    pagesSaved: 0,
    totalSizeBytes: 0,
    crawlVelocity: 0,
    failRate: 0,
  });
  const [recentExports, setRecentExports] = useState<RecentExport[]>([]);
  const { activeJobIds } = useCrawlEvents();
  const activeJobsRef = useRef(activeJobIds);

  useEffect(() => {
    activeJobsRef.current = activeJobIds;
  }, [activeJobIds]);

  useEffect(() => {
    loadRecentJobs();
    loadStats();
    loadRecentExports();
    let tick = 0;
    const interval = setInterval(() => {
      tick += 1;
      loadRecentJobs();
      loadRecentExports();
      // Stats: live (every 3s) while crawls are active, else throttled to ~12s
      const hasActiveJobs = activeJobsRef.current.size > 0;
      if (hasActiveJobs || tick % 4 === 0) {
        loadStats();
      }
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  const loadRecentJobs = async () => {
    try {
      const jobs: CrawlJob[] = await invoke('list_jobs');
      setRecentJobs((jobs || []).slice(-5).reverse());
    } catch (err) {
      console.warn('[Dashboard] Failed to load recent jobs:', err);
    }
  };

  const loadStats = async () => {
    try {
      const s: DashboardStats = await invoke('get_dashboard_stats');
      setStats({
        pagesSaved: s?.pagesSaved ?? 0,
        totalSizeBytes: s?.totalSizeBytes ?? 0,
        crawlVelocity: s?.crawlVelocity ?? 0,
        failRate: s?.failRate ?? 0,
      });
    } catch (err) {
      console.warn('[Dashboard] get_dashboard_stats failed:', err);
      setStats({
        pagesSaved: 0,
        totalSizeBytes: 0,
        crawlVelocity: 0,
        failRate: 0,
      });
    }
  };

  const loadRecentExports = async () => {
    try {
      const list: RecentExport[] = await invoke('list_exports', { limit: 5 });
      setRecentExports(list || []);
    } catch (err) {
      console.warn('[Dashboard] Failed to load recent exports:', err);
    }
  };

  return (
    <div className="h-full overflow-y-auto p-8">
      {/* Welcome */}
      <div className="mb-8">
        <h1 className="text-2xl font-display font-bold text-ghost mb-2">
          Dashboard
        </h1>
        <p className="text-secondary text-sm">
          High-performance documentation extraction. Ready to rip the web.
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-4 gap-4 mb-8">
        <StatCard
          icon={<FileText size={20} className="text-cyberBlue" />}
          label="Pages Saved"
          value={<AnimatedCounter value={stats.pagesSaved} />}
        />
        <StatCard
          icon={<HardDrive size={20} className="text-accentGreen" />}
          label="Total Size"
          value={<AnimatedCounter value={stats.totalSizeBytes} formatValue={formatBytes} />}
        />
        <StatCard
          icon={<Lightning size={20} className="text-amber" />}
          label="Crawl Velocity"
          value={<AnimatedCounter value={stats.crawlVelocity} formatValue={(v) => `${v.toFixed(1)} pages/min`} />}
        />
        <StatCard
          icon={<Warning size={20} className="text-crimson" />}
          label="Fail Rate"
          value={<AnimatedCounter value={stats.failRate} formatValue={(v) => `${v.toFixed(1)}%`} />}
        />
      </div>

      {/* Quick Start */}
      <div className="bg-surface/30 border border-abyssal/50 rounded-lg p-6 mb-8">
        <h2 className="text-ghost font-semibold mb-4 flex items-center">
          <ArrowRight size={18} className="text-accentGreen mr-2" />
          Quick Start
        </h2>
        <div className="flex items-center space-x-4">
          <input
            type="url"
            placeholder="Enter a URL to crawl..."
            value={quickUrl}
            onChange={(e) => setQuickUrl(e.target.value)}
            className="flex-1 bg-surface/50 border border-abyssal rounded-md px-4 py-3 text-ghost placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 transition-all"
          />
          <button
            onClick={() => onQuickStart(quickUrl)}
            disabled={!quickUrl}
            className="bg-accentGreen hover:bg-brightGreen text-deepVoid px-6 py-3 rounded-md font-semibold transition-all duration-fast disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Start Crawl
          </button>
        </div>
        <p className="text-charcoal text-xs mt-2">
          Go to "New Crawl" for advanced options
        </p>
      </div>

      {/* Recent Activity */}
      <div>
        <h2 className="text-ghost font-semibold mb-4 text-sm uppercase tracking-wider">
          Recent Activity
        </h2>
        {recentJobs.length === 0 ? (
          <div className="bg-surface/20 border border-abyssal/30 rounded-lg p-8 text-center">
            <Download size={32} className="text-charcoal/20 mx-auto mb-3" />
            <p className="text-charcoal text-sm">No crawls yet</p>
            <p className="text-charcoal/50 text-xs mt-1">
              Your recent crawl history will appear here
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {recentJobs.map((job) => (
              <div
                key={job.id}
                className="flex items-center justify-between bg-surface/20 border border-abyssal/30 rounded-md px-4 py-3"
              >
                <div className="flex items-center space-x-3 min-w-0">
                  <StatusIcon status={job.status} />
                  <div className="min-w-0">
                    <p className="text-sm text-ghost truncate">{job.url}</p>
                    <p className="text-[10px] text-charcoal">
                      {job.results?.length || 0} pages
                      {job.startTime && ` · ${new Date(job.startTime).toLocaleDateString()}`}
                    </p>
                  </div>
                </div>
                <StatusBadge status={job.status} />
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Recent Exports */}
      <div className="mt-8">
        <h2 className="text-ghost font-semibold mb-4 text-sm uppercase tracking-wider">
          Recent Exports
        </h2>
        {recentExports.length === 0 ? (
          <div className="bg-surface/20 border border-abyssal/30 rounded-lg p-8 text-center">
            <Download size={32} className="text-charcoal/20 mx-auto mb-3" />
            <p className="text-charcoal text-sm">No exports yet</p>
            <p className="text-charcoal/50 text-xs mt-1">
              Export a completed job from History to see it here.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            {recentExports.map((exp) => (
              <div
                key={exp.jobId}
                className="flex items-center justify-between bg-surface/20 border border-abyssal/30 rounded-md px-4 py-3"
                title={exp.path}
              >
                <div className="flex items-center space-x-3 min-w-0">
                  <Archive size={16} className="text-accentGreen" />
                  <div className="min-w-0">
                    <p className="text-sm text-ghost truncate font-mono">
                      {exp.jobId}.zip
                    </p>
                    <p className="text-[10px] text-charcoal">
                      {formatBytes(exp.sizeBytes)}
                      {exp.createdAt && ` · ${new Date(exp.createdAt).toLocaleString()}`}
                    </p>
                  </div>
                </div>
                <span className="text-[10px] font-semibold uppercase tracking-wider px-2 py-1 rounded bg-accentGreen/10 text-accentGreen">
                  zip
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

const StatCard = ({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: React.ReactNode;
}) => (
  <div className="bg-surface/30 border border-abyssal/50 rounded-lg p-4">
    <div className="flex items-center justify-between mb-2">
      <span className="text-[11px] text-charcoal uppercase tracking-wider">{label}</span>
      {icon}
    </div>
    <div className="text-2xl font-mono font-bold text-ghost">{value}</div>
  </div>
);

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
  const v = bytes / Math.pow(k, i);
  return `${v.toFixed(v >= 100 || i === 0 ? 0 : 1)} ${sizes[i]}`;
}
