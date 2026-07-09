import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { CrawlEventsProvider, useCrawlEvents } from './useCrawlEvents';
import { ToastProvider } from './useToasts';
import type { AppSettings, CrawlEvent, CrawlJob } from '../types';

// Mock the Tauri event system
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((event: string, handler: (payload: any) => void) => {
    // Store handler for later use in tests
    (window as any).__tauriEventHandlers = (window as any).__tauriEventHandlers || {};
    (window as any).__tauriEventHandlers[event] = handler;
    return Promise.resolve(() => {});
  }),
}));

// get_settings/get_job/export_job_v2 are called on terminal job events;
// each test configures the mock's per-command behavior as needed.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

const baseSettings: AppSettings = {
  outputDir: '', concurrency: 1, requestDelay: 0, timeout: 1000, userAgent: '',
  defaultMaxDepth: 1, defaultPageLimit: 1, defaultDownloadAssets: false,
  defaultHeadlessStrategy: 'never', defaultRespectRobotsTxt: true, defaultStayWithinDomain: true,
  defaultSsrfProtection: true, windowWidth: 1280, windowHeight: 900,
  notificationsEnabled: false, theme: 'dark', shortcutOverrides: {}, autoExportFormat: null,
};

const baseJob: CrawlJob = {
  id: 'job-1', url: 'https://example.com', status: 'completed',
  config: {
    maxDepth: 1, pageLimit: 1, downloadAssets: false, headlessStrategy: 'never',
    contentSelectors: [], excludePatterns: [], includePatterns: [], pathPrefix: '',
    respectRobotsTxt: true, stayWithinDomain: true, ssrfProtection: true, outputDir: '', profile: null,
  },
  progress: { pagesCrawled: 1, pageLimit: 1, currentUrl: '', depth: 0, maxDepth: 1, startTime: '' },
  results: [],
};

describe('useCrawlEvents', () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ToastProvider>
      <CrawlEventsProvider>{children}</CrawlEventsProvider>
    </ToastProvider>
  );

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation(() => Promise.reject(new Error('not available in test')));
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

  it('should only track jobs via jobStatusChanged, not other events', async () => {
    const { result } = renderHook(() => useCrawlEvents(), { wrapper });
    
    await new Promise((resolve) => setTimeout(resolve, 50));
    
    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    
    // Emit progress and pageComplete events — should NOT add to activeJobIds
    if (handler) {
      handler({
        payload: {
          type: 'progress',
          jobId: 'job-progress',
          progress: {
            pagesCrawled: 10,
            pageLimit: 100,
            currentUrl: '',
            depth: 1,
            maxDepth: 2,
            startTime: '',
          },
        } as unknown as CrawlEvent,
      });
      handler({
        payload: {
          type: 'pageComplete',
          jobId: 'job-progress',
          page: { url: '/test', title: 'Test', status: 200, linksCount: 5 },
        } as CrawlEvent,
      });
      handler({
        payload: {
          type: 'error',
          jobId: 'job-error',
          message: 'Something broke',
          kind: 'network',
        } as CrawlEvent,
      });
    }

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(result.current.activeJobIds.size).toBe(0);
  });

  it('triggers export_job_v2 when a job completes and autoExportFormat is set', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') return Promise.resolve({ ...baseSettings, autoExportFormat: 'merged_md' });
      if (cmd === 'get_job') return Promise.resolve(baseJob);
      if (cmd === 'export_job_v2') return Promise.resolve('/output/formats');
      return Promise.reject(new Error(`unexpected command: ${cmd}`));
    });

    renderHook(() => useCrawlEvents(), { wrapper });
    await new Promise((resolve) => setTimeout(resolve, 20));

    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    handler({ payload: { type: 'jobStatusChanged', jobId: 'job-1', status: 'completed' } as CrawlEvent });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('export_job_v2', {
        jobId: 'job-1',
        format: 'merged_md',
        destination: null,
      });
    });
  });

  it('does not trigger export_job_v2 when autoExportFormat is unset', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') return Promise.resolve(baseSettings);
      if (cmd === 'get_job') return Promise.resolve(baseJob);
      return Promise.reject(new Error(`unexpected command: ${cmd}`));
    });

    renderHook(() => useCrawlEvents(), { wrapper });
    await new Promise((resolve) => setTimeout(resolve, 20));

    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    handler({ payload: { type: 'jobStatusChanged', jobId: 'job-1', status: 'completed' } as CrawlEvent });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mockInvoke).not.toHaveBeenCalledWith('export_job_v2', expect.anything());
  });

  it('does not trigger export_job_v2 for a failed job even if autoExportFormat is set', async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_settings') return Promise.resolve({ ...baseSettings, notificationsEnabled: true, autoExportFormat: 'merged_md' });
      if (cmd === 'get_job') return Promise.resolve({ ...baseJob, status: 'failed' });
      return Promise.reject(new Error(`unexpected command: ${cmd}`));
    });

    renderHook(() => useCrawlEvents(), { wrapper });
    await new Promise((resolve) => setTimeout(resolve, 20));

    const handler = (window as any).__tauriEventHandlers?.['crawl-event'];
    handler({ payload: { type: 'jobStatusChanged', jobId: 'job-1', status: 'failed' } as CrawlEvent });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(mockInvoke).not.toHaveBeenCalledWith('export_job_v2', expect.anything());
  });
});