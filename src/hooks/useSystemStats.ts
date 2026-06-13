import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SystemStats } from '../types';

export function useSystemStats(intervalMs = 2000) {
  const [stats, setStats] = useState<SystemStats | null>(null);

  useEffect(() => {
    let cancelled = false;
    const fetchStats = async () => {
      try {
        const s = await invoke<SystemStats>('get_system_stats');
        if (!cancelled) setStats(s);
      } catch {
        if (!cancelled) setStats(null);
      }
    };
    fetchStats();
    const id = setInterval(fetchStats, intervalMs);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [intervalMs]);

  return stats;
}
