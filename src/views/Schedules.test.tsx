import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SchedulesView } from './Schedules';
import type { AppSettings, Schedule } from '../types';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const settings = {
  defaultMaxDepth: 2, defaultPageLimit: 100, defaultDownloadAssets: false,
  defaultHeadlessStrategy: 'never', defaultRespectRobotsTxt: true,
  defaultStayWithinDomain: true, defaultSsrfProtection: true,
} as unknown as AppSettings;

const schedule: Schedule = {
  id: 's1', name: 'Nightly docs', url: 'https://docs.example.com',
  config: {
    maxDepth: 2, pageLimit: 100, downloadAssets: false, headlessStrategy: 'never',
    contentSelectors: [], excludePatterns: [], includePatterns: [], pathPrefix: '',
    respectRobotsTxt: true, stayWithinDomain: true, ssrfProtection: true, outputDir: '', profile: null,
  },
  cadence: 'weekly', hour: 6, minute: 30, weekday: 1, dayOfMonth: null,
  enabled: true, createdAt: '2026-07-25T00:00:00Z',
  lastRun: null, nextRun: '2026-07-27T06:30:00+00:00', lastJobId: null,
};

describe('SchedulesView', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'list_schedules') return Promise.resolve([schedule]);
      if (cmd === 'get_settings') return Promise.resolve(settings);
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
