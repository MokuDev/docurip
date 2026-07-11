import { createContext, useContext, useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, CrawlEvent, CrawlJob } from '../types';
import { notifyCrawlComplete, notifyCrawlFailed } from './useNotifications';
import { useToasts } from './useToasts';

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
  const terminalJobsHandled = useRef(new Set<string>());
  const { pushToast } = useToasts();

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
            if (!terminalJobsHandled.current.has(event.jobId)) {
              terminalJobsHandled.current.add(event.jobId);
              invoke<AppSettings>('get_settings').then((settings) => {
                const wantsNotification = settings.notificationsEnabled;
                const wantsAutoExport = event.status === 'completed' && !!settings.autoExportFormat;
                if (!wantsNotification && !wantsAutoExport) return;
                invoke<CrawlJob>('get_job', { jobId: event.jobId }).then((job) => {
                  if (wantsNotification) {
                    if (event.status === 'completed') {
                      notifyCrawlComplete(job.url, job.results.length);
                    } else if (event.status === 'failed') {
                      notifyCrawlFailed(job.url, job.error);
                    }
                  }
                  if (wantsAutoExport) {
                    const format = settings.autoExportFormat;
                    invoke('export_job_v2', { jobId: event.jobId, format, destination: null })
                      .then(() => pushToast('success', `Auto-exported ${job.url} as ${format}`))
                      .catch((err) => pushToast('error', `Auto-export failed: ${err}`));
                  }
                }).catch((err) => { console.warn('Failed to fetch job for terminal event handling:', err); });
              }).catch((err) => { console.warn('Failed to fetch settings for terminal event handling:', err); });
            }
          }
        }
        return { ...prev, events: nextEvents, activeJobIds: nextActive };
      });
    })
      .then((fn) => { unlisten = fn; })
      .catch((err) => { console.warn('Tauri event listener not available:', err); });

    return () => unlisten?.();
  }, [pushToast]);

  return <CrawlEventsContext.Provider value={state}>{children}</CrawlEventsContext.Provider>;
}

export function useCrawlEvents() {
  return useContext(CrawlEventsContext);
}
