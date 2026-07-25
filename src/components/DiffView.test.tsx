import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { DiffView } from './DiffView';
import type { CrawlJob, DiffResult, DiffLine } from '../types';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const baseConfig: CrawlJob['config'] = {
  maxDepth: 2, pageLimit: 100, downloadAssets: false, headlessStrategy: 'never',
  contentSelectors: [], excludePatterns: [], includePatterns: [], pathPrefix: '',
  respectRobotsTxt: true, stayWithinDomain: true, ssrfProtection: true, outputDir: '/out', profile: null,
};

function makeJob(id: string, startTime: string): CrawlJob {
  return {
    id, url: 'https://example.com', status: 'completed', config: baseConfig,
    progress: { pagesCrawled: 1, pageLimit: 100, currentUrl: '', depth: 0, maxDepth: 2, startTime },
    results: [], startTime,
  };
}

const diffResult: DiffResult = {
  entries: [
    { url: 'https://example.com/new', title: 'New Page', kind: 'added' },
    { url: 'https://example.com/b', title: 'Changed', kind: 'modified' },
    { url: 'https://example.com/gone', title: 'Gone', kind: 'removed' },
    { url: 'https://example.com/a', title: 'Same', kind: 'unchanged' },
  ],
  added: 1, removed: 1, modified: 1, unchanged: 1, unknown: 0,
};

const lineDiff: DiffLine[] = [
  { tag: 'equal', content: 'kept line' },
  { tag: 'delete', content: 'old line' },
  { tag: 'insert', content: 'new line' },
];

describe('DiffView', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'diff_jobs') return Promise.resolve(diffResult);
      if (cmd === 'diff_page') return Promise.resolve(lineDiff);
      return Promise.resolve(null);
    });
  });

  const newJob = makeJob('new', '2026-07-25T10:00:00Z');
  const oldJob = makeJob('old', '2026-07-24T10:00:00Z');

  it('auto-selects the earlier crawl and renders bucket counts', async () => {
    render(<DiffView job={newJob} allJobs={[newJob, oldJob]} onClose={vi.fn()} />);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('diff_jobs', { oldJobId: 'old', newJobId: 'new' }),
    );
    expect(await screen.findByText('Added (1)')).toBeInTheDocument();
    expect(screen.getByText('Modified (1)')).toBeInTheDocument();
    expect(screen.getByText('Removed (1)')).toBeInTheDocument();
  });

  it('loads a line diff when a modified page is clicked', async () => {
    render(<DiffView job={newJob} allJobs={[newJob, oldJob]} onClose={vi.fn()} />);
    const changed = await screen.findByText('Changed');
    fireEvent.click(changed);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith('diff_page', {
        oldJobId: 'old', newJobId: 'new', url: 'https://example.com/b',
      }),
    );
    expect(await screen.findByText('new line')).toBeInTheDocument();
    expect(screen.getByText('old line')).toBeInTheDocument();
  });

  it('tells the user when there is no baseline to compare against', () => {
    render(<DiffView job={newJob} allJobs={[newJob]} onClose={vi.fn()} />);
    expect(
      screen.getByText(/No earlier completed crawl of this URL/i),
    ).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
