import { createContext, useContext, useEffect, useState } from 'react';
import type { CrawlEvent } from '../types';

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
    try {
      const unlistenFn = (window as any).__TAURI__?.event?.listen('crawl-event', (event: any) => {
        setState((prev) => {
          const nextEvents = [...prev.events, event].slice(-500);
          const nextActive = new Set(prev.activeJobIds);
          if (event.type === 'jobStatusChanged') {
            if (event.status === 'running' || event.status === 'queued') {
              nextActive.add(event.jobId);
            } else {
              nextActive.delete(event.jobId);
            }
          } else {
            nextActive.add(event.jobId);
          }
          return { ...prev, events: nextEvents, activeJobIds: nextActive };
        });
      });
      unlisten = typeof unlistenFn === 'function' ? unlistenFn : undefined;
    } catch (err) {
      console.warn('Tauri event listener not available (running in browser?):', err);
    }
    return () => unlisten?.();
  }, []);

  return <CrawlEventsContext.Provider value={state}>{children}</CrawlEventsContext.Provider>;
}

export function useCrawlEvents() {
  return useContext(CrawlEventsContext);
}
