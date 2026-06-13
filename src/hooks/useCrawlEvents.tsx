import { createContext, useContext, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
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
    const setup = async () => {
      unlisten = await listen<CrawlEvent>('crawl-event', (event) => {
        const ev = event.payload;
        setState((prev) => {
          const nextEvents = [...prev.events, ev].slice(-500);
          const nextActive = new Set(prev.activeJobIds);
          if (ev.type === 'jobStatusChanged') {
            if (ev.status === 'running' || ev.status === 'queued') {
              nextActive.add(ev.jobId);
            } else {
              nextActive.delete(ev.jobId);
            }
          } else {
            // Any other event implies the job is active
            nextActive.add(ev.jobId);
          }
          return { events: nextEvents, activeJobIds: nextActive };
        });
      });
    };
    setup();
    return () => unlisten?.();
  }, []);

  return <CrawlEventsContext.Provider value={state}>{children}</CrawlEventsContext.Provider>;
}

export function useCrawlEvents() {
  return useContext(CrawlEventsContext);
}
