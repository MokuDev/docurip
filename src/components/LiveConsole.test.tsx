import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, act, waitFor } from '@testing-library/react';
import { LiveConsole } from './LiveConsole';
import { CrawlEventsProvider } from '../hooks/useCrawlEvents';
import { ToastProvider } from '../hooks/useToasts';
import type { CrawlEvent } from '../types';

// Capture the crawl-event handler so the test can push events.
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: (payload: any) => void) => {
    (window as any).__handlers = (window as any).__handlers || {};
    (window as any).__handlers[event] = handler;
    return Promise.resolve(() => {});
  }),
}));

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

function emit(ev: CrawlEvent) {
  const handler = (window as any).__handlers?.['crawl-event'];
  handler?.({ payload: ev });
}

describe('LiveConsole', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') return Promise.resolve({ liveConsoleMaxEvents: 1000 });
      return Promise.resolve(null);
    });
    (window as any).__handlers = {};
    // jsdom doesn't implement scrollIntoView; the console calls it on new logs.
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
  });

  function renderConsole() {
    return render(
      <ToastProvider>
        <CrawlEventsProvider>
          <LiveConsole />
        </CrawlEventsProvider>
      </ToastProvider>,
    );
  }

  it('keeps showing new events past the old 500-event freeze point', async () => {
    renderConsole();
    // Let the async listen() attach.
    await waitFor(() => expect((window as any).__handlers['crawl-event']).toBeDefined());

    // Emit well past 500 — the count where the index-based console used to
    // stop updating.
    await act(async () => {
      for (let i = 0; i < 620; i++) {
        emit({ type: 'log', jobId: 'j', message: `Log ${i}` } as CrawlEvent);
      }
    });

    // The most recent event must be visible — proof the console didn't
    // freeze once the shared buffer began sliding.
    expect(await screen.findByText('Log 619')).toBeInTheDocument();
  });
});
