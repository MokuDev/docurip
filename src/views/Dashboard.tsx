import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Play,
  Download,
  Globe,
  FileText,
  CheckCircle,
  Warning,
  ArrowRight,
} from '@phosphor-icons/react';
import type { CrawlJob } from '../types';

export function DashboardView() {
  const [recentJobs, setRecentJobs] = useState<CrawlJob[]>([]);
  const [stats, setStats] = useState({
    totalCrawls: 0,
    totalPages: 0,
    activeCrawls: 0,
    successRate: 0,
  });

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 3000);
    return () => clearInterval(interval);
  }, []);

  const loadStats = async () => {
    try {
      const jobs: CrawlJob[] = await invoke('list_jobs');
      const j = jobs || [];
      const completed = j.filter((x) => x.status === 'completed');
      const pages = j.reduce((sum, x) => sum + (x.results?.length || 0), 0);

      setRecentJobs(j.slice(-5).reverse());
      setStats({
        totalCrawls: j.length,
        totalPages: pages,
        activeCrawls: j.filter((x) => x.status === 'running').length,
        successRate:
          completed.length > 0
            ? Math.round((completed.length / j.length) * 100)
            : 0,
      });
    } catch {
      // ignore
    }
  };

  return (
    <div className="h-full overflow-y-auto p-8">
      {/* Welcome */}
      <div className="mb-8">
        <h1 className="text-2xl font-display font-bold text-ghost mb-2">
          Docurip Dashboard
        </h1>
        <p className="text-secondary text-sm">
          High-performance documentation extraction. Ready to rip the web.
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-4 gap-4 mb-8">
        <StatCard
          icon={<Globe size={20} className="text-accentGreen" />}
          label="Total Crawls"
          value={stats.totalCrawls}
        />
        <StatCard
          icon={<FileText size={20} className="text-cyberBlue" />}
          label="Pages Extracted"
          value={stats.totalPages}
        />
        <StatCard
          icon={<Play size={20} className="text-amber" />}
          label="Active"
          value={stats.activeCrawls}
        />
        <StatCard
          icon={<CheckCircle size={20} className="text-brightGreen" />}
          label="Success Rate"
          value={`${stats.successRate}%`}
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
            disabled
            className="flex-1 bg-surface/50 border border-abyssal rounded-md px-4 py-3 text-ghost placeholder-charcoal/40 cursor-not-allowed"
          />
          <button
            disabled
            className="bg-accentGreen/20 text-accentGreen/50 px-6 py-3 rounded-md font-semibold cursor-not-allowed"
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
  value: string | number;
}) => (
  <div className="bg-surface/30 border border-abyssal/50 rounded-lg p-4">
    <div className="flex items-center justify-between mb-2">
      <span className="text-[11px] text-charcoal uppercase tracking-wider">{label}</span>
      {icon}
    </div>
    <div className="text-2xl font-mono font-bold text-ghost">{value}</div>
  </div>
);

const StatusIcon = ({ status }: { status: string }) => {
  switch (status) {
    case 'completed':
      return <CheckCircle weight="fill" size={16} className="text-brightGreen" />;
    case 'running':
      return <Play weight="fill" size={16} className="text-accentGreen" />;
    case 'failed':
      return <Warning weight="fill" size={16} className="text-crimson" />;
    default:
      return <Globe size={16} className="text-charcoal" />;
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
      className={`text-[10px] font-semibold uppercase tracking-wider px-2 py-1 rounded ${styles[status] || 'text-charcoal'}`}
    >
      {status}
    </span>
  );
};
