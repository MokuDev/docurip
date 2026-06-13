import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { CrawlJob } from '../types';
import { useSystemStats } from '../hooks/useSystemStats';
import { useCrawlEvents } from '../hooks/useCrawlEvents';

export function SystemStatusBar() {
  const stats = useSystemStats(2000);
  const { activeJobIds } = useCrawlEvents();
  const [activeOutputPath, setActiveOutputPath] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const resolve = async () => {
      if (activeJobIds.size === 0) {
        if (!cancelled) setActiveOutputPath(null);
        return;
      }
      try {
        const jobs = await invoke<CrawlJob[]>('list_jobs');
        if (cancelled) return;
        const active = jobs.find(
          (j) => activeJobIds.has(j.id) && (j.status === 'running' || j.status === 'paused' || j.status === 'queued')
        );
        setActiveOutputPath(active?.config?.outputDir ?? null);
      } catch {
        if (!cancelled) setActiveOutputPath(null);
      }
    };
    resolve();
    const id = setInterval(resolve, 3000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [activeJobIds]);

  const cpu = stats ? `${stats.cpuPercent.toFixed(0)}%` : '—';
  const ram = stats ? `${stats.memUsedMb} / ${stats.memTotalMb} MB` : '— / — MB';
  const path = activeOutputPath ?? '(idle)';

  return (
    <div className="h-5 w-full bg-deepVoid border-t border-abyssal/50 flex items-center justify-between px-3 text-[10px] font-mono text-secondary select-none">
      <div className="flex items-center gap-2">
        <span className="text-charcoal uppercase tracking-widest">CPU</span>
        <span className="text-ghost tabular-nums">{cpu}</span>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-charcoal uppercase tracking-widest">RAM</span>
        <span className="text-ghost tabular-nums">{ram}</span>
      </div>
      <div className="flex items-center gap-2 min-w-0">
        <span className="text-charcoal uppercase tracking-widest flex-shrink-0">Output</span>
        <span className="text-ghost truncate max-w-[60ch]" title={path}>
          {path}
        </span>
      </div>
    </div>
  );
}
