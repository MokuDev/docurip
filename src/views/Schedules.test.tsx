import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SchedulesView } from './Schedules';
import type { AppSettings, CrawlTemplate, Schedule } from '../types';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const settings = {
  defaultMaxDepth: 2, defaultPageLimit: 100, defaultDownloadAssets: false,
  defaultHeadlessStrategy: 'never', defaultRespectRobotsTxt: true,
  defaultStayWithinDomain: true, defaultSsrfProtection: true,
} as unknown as AppSettings;

const config = {
  maxDepth: 2, pageLimit: 100, downloadAssets: false, headlessStrategy: 'never' as const,
  contentSelectors: [], excludePatterns: [], includePatterns: [], pathPrefix: '',
  respectRobotsTxt: true, stayWithinDomain: true, ssrfProtection: true, outputDir: '', profile: null,
};

const schedule: Schedule = {
  id: 's1', name: 'Nightly docs', urls: ['https://docs.example.com'],
  config,
  cadence: 'weekly', hour: 6, minute: 30, weekday: 1, dayOfMonth: null,
  enabled: true, createdAt: '2026-07-25T00:00:00Z',
  lastRun: null, nextRun: '2026-07-27T06:30:00+00:00', lastJobId: null,
};

const template: CrawlTemplate = {
  id: 't1', name: 'Deep API crawl', url: 'https://api.example.com',
  config: { ...config, maxDepth: 9 },
  createdAt: '2026-07-20T00:00:00Z',
};

describe('SchedulesView', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_schedules') return Promise.resolve([schedule]);
      if (cmd === 'get_settings') return Promise.resolve(settings);
      if (cmd === 'list_templates') return Promise.resolve([template]);
      return Promise.resolve(null);
    });
  });

  it('renders a schedule with its cadence summary', async () => {
    render(<SchedulesView />);
    expect(await screen.findByText('Nightly docs')).toBeInTheDocument();
    expect(screen.getByText(/Weekly on Mon at 06:30 UTC/)).toBeInTheDocument();
  });

  it('toggles a schedule via toggle_schedule', async () => {
    render(<SchedulesView />);
    const pause = await screen.findByTitle('Pause schedule');
    fireEvent.click(pause);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('toggle_schedule', {
        scheduleId: 's1', enabled: false,
      }),
    );
  });

  it('seeds a new schedule from a template', async () => {
    render(<SchedulesView />);
    await screen.findByText('Nightly docs');
    fireEvent.click(screen.getByText('New Schedule'));
    await screen.findByText('New Schedule', { selector: 'h2' });

    fireEvent.change(screen.getByDisplayValue('Default settings'), {
      target: { value: 't1' },
    });

    // Template fills the blank name and URL, and supplies the config.
    expect(screen.getByPlaceholderText('Nightly docs sync')).toHaveValue('Deep API crawl');
    fireEvent.click(screen.getByText('Save'));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('save_schedule', {
        schedule: expect.objectContaining({
          urls: ['https://api.example.com'],
          templateId: 't1',
          config: expect.objectContaining({ maxDepth: 9 }),
        }),
      }),
    );
  });

  it('saves a multi-URL schedule with an on-failure override', async () => {
    render(<SchedulesView />);
    await screen.findByText('Nightly docs');
    fireEvent.click(screen.getByText('New Schedule'));
    await screen.findByText('New Schedule', { selector: 'h2' });

    fireEvent.change(screen.getByPlaceholderText('Nightly docs sync'), {
      target: { value: 'Docs sweep' },
    });
    fireEvent.change(screen.getByPlaceholderText(/docs\.example\.com\/v1/), {
      target: { value: 'https://a.example.com\nhttps://b.example.com' },
    });

    // Two URLs turn the schedule into a batch, which exposes the override.
    fireEvent.change(await screen.findByDisplayValue(/Use default/), {
      target: { value: 'stop' },
    });
    fireEvent.click(screen.getByText('Save'));

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('save_schedule', {
        schedule: expect.objectContaining({
          urls: ['https://a.example.com', 'https://b.example.com'],
          onFailure: 'stop',
        }),
      }),
    );
  });

  it('rejects an invalid URL', async () => {
    render(<SchedulesView />);
    await screen.findByText('Nightly docs');
    fireEvent.click(screen.getByText('New Schedule'));
    await screen.findByText('New Schedule', { selector: 'h2' });

    fireEvent.change(screen.getByPlaceholderText('Nightly docs sync'), {
      target: { value: 'Broken' },
    });
    fireEvent.change(screen.getByPlaceholderText(/docs\.example\.com\/v1/), {
      target: { value: 'not-a-url' },
    });
    fireEvent.click(screen.getByText('Save'));

    expect(
      await screen.findByText('Enter valid URLs (including http:// or https://).'),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith('save_schedule', expect.anything());
  });

  it('opens the editor and validates a missing name', async () => {
    render(<SchedulesView />);
    await screen.findByText('Nightly docs');
    fireEvent.click(screen.getByText('New Schedule'));
    expect(await screen.findByText('New Schedule', { selector: 'h2' })).toBeInTheDocument();
    // Clear default name and try to save
    fireEvent.click(screen.getByText('Save'));
    expect(await screen.findByText('Name is required.')).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith('save_schedule', expect.anything());
  });
});
