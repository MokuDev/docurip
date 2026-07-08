import { createContext, useContext, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, CrawlEvent, CrawlJob } from '../types';
import { notifyCrawlComplete, notifyCrawlFailed } from './useNotifications';

interface CrawlEventsState {
  events: CrawlEvent[];
  activeJobIds: Set<string>;
}

const CrawlEventsContext = createContext<CrawlEventsState>({
  events: [],
  activeJobIds: new Set(),
});

export function CrawlEventsProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = useState<CrawlEventsState>({ events: [], activeJobIds: new Set() });

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<CrawlEvent>('crawl-event', (raw) => {
      const event = raw.payload;
      setState((prev) => {
        const nextEvents = [...prev.events, event].slice(-500);
        const nextActive = new Set(prev.activeJobIds);
        if (event.type === 'jobStatusChanged') {
          if (event.status === 'running' || event.status === 'queued') {
            nextActive.add(event.jobId);
          } else if (event.status === 'completed' || event.status === 'failed' || event.status === 'cancelled') {
            nextActive.delete(event.jobId);
            invoke<AppSettings>('get_settings').then((settings) => {
              if (!settings.notificationsEnabled) return;
              invoke<CrawlJob>('get_job', { jobId: event.jobId }).then((job) => {
                if (event.status === 'completed') {
                  notifyCrawlComplete(job.url, job.results.length);
                } else if (event.status === 'failed') {
                  notifyCrawlFailed(job.url, job.error);
                }
              }).catch(() => {});
            }).catch(() => {});
          }
        }
        return { ...prev, events: nextEvents, activeJobIds: nextActive };
      });
    })
      .then((fn) => { unlisten = fn; })
      .catch((err) => { console.warn('Tauri event listener not available:', err); });

    return () => unlisten?.();
  }, []);

  return <CrawlEventsContext.Provider value={state}>{children}</CrawlEventsContext.Provider>;
}

export function useCrawlEvents() {
  return useContext(CrawlEventsContext);
}
