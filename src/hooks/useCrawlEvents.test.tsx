import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { CrawlEventsProvider, useCrawlEvents } from './useCrawlEvents';
import type { CrawlEvent } from '../types';

// Mock the Tauri event system
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: (payload: any) => void) => {
    // Store handler for later use in tests
    (window as any).__tauriEventHandlers = (window as any).__tauriEventHandlers || {};
    (window as any).__tauriEventHandlers[event] = handler;
    return Promise.resolve(() => {});
  }),
}));

describe('useCrawlEvents', () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <CrawlEventsProvider>{children}</CrawlEventsProvider>
  );

  beforeEach(() => {
    vi.clearAllMocks();
    (window as any).__tauriEventHandlers = {};
  });

  it('should start with empty events and no active jobs', () => {
    const { result } = renderHook(() => useCrawlEvents(), { wrapper });
    
    expect(result.current.events).toEqual([]);
    expect(result.current.activeJobIds).toEqual(new Set());
  });

  it('should add event when crawl-event is emitted', async () => {
    const { result } = renderHook(() => useCrawlEvents(), { wrapper });
    
    // Wait for the hook to initialize
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    // Simulate Tauri event emission via the stored handler
    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    if (handler) {
      handler({
        payload: {
          type: 'log',
          jobId: 'test-job-1',
          message: 'Test log message',
          level: 'INFO',
        } as CrawlEvent,
      });
    }

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.events.length).toBeGreaterThan(0);
  });

  it('should track active jobs based on jobStatusChanged events', async () => {
    const { result } = renderHook(() => useCrawlEvents(), { wrapper });
    
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    
    // Simulate job starting
    if (handler) {
      handler({
        payload: {
          type: 'jobStatusChanged',
          jobId: 'job-1',
          status: 'running',
        } as CrawlEvent,
      });
    }

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.activeJobIds.has('job-1')).toBe(true);

    // Simulate job completing
    if (handler) {
      handler({
        payload: {
          type: 'jobStatusChanged',
          jobId: 'job-1',
          status: 'completed',
        } as CrawlEvent,
      });
    }

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.activeJobIds.has('job-1')).toBe(false);
  });

  it('should cap events at 500', async () => {
    const { result } = renderHook(() => useCrawlEvents(), { wrapper });
    
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    
    // Emit 501 events
    if (handler) {
      for (let i = 0; i < 501; i++) {
        handler({
          payload: {
            type: 'log',
            jobId: `job-${i}`,
            message: `Log ${i}`,
            level: 'INFO',
          } as CrawlEvent,
        });
      }
    }

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.events.length).toBeLessThanOrEqual(500);
  });
});