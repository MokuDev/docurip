import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(),
  requestPermission: vi.fn(),
  sendNotification: vi.fn(),
}));

import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import { notifyCrawlComplete, notifyCrawlFailed } from './useNotifications';

const mockGranted = isPermissionGranted as ReturnType<typeof vi.fn>;
const mockRequest = requestPermission as ReturnType<typeof vi.fn>;
const mockSend = sendNotification as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
});

describe('notifyCrawlComplete', () => {
  it('sends notification when permission is already granted', async () => {
    mockGranted.mockResolvedValue(true);

    await notifyCrawlComplete('https://docs.example.com', 42);

    expect(mockSend).toHaveBeenCalledWith({
      title: 'Crawl Complete',
      body: 'Finished crawling https://docs.example.com — 42 pages saved',
    });
    expect(mockRequest).not.toHaveBeenCalled();
  });

  it('requests permission when not already granted', async () => {
    mockGranted.mockResolvedValue(false);
    mockRequest.mockResolvedValue('granted');

    await notifyCrawlComplete('https://example.com', 10);

    expect(mockRequest).toHaveBeenCalledOnce();
    expect(mockSend).toHaveBeenCalledOnce();
  });

  it('does not send when permission is denied', async () => {
    mockGranted.mockResolvedValue(false);
    mockRequest.mockResolvedValue('denied');

    await notifyCrawlComplete('https://example.com', 10);

    expect(mockSend).not.toHaveBeenCalled();
  });
});

describe('notifyCrawlFailed', () => {
  it('sends failure notification with error message', async () => {
    mockGranted.mockResolvedValue(true);

    await notifyCrawlFailed('https://example.com', 'Connection timeout');

    expect(mockSend).toHaveBeenCalledWith({
      title: 'Crawl Failed',
      body: 'Failed to crawl https://example.com: Connection timeout',
    });
  });

  it('sends failure notification without error message', async () => {
    mockGranted.mockResolvedValue(true);

    await notifyCrawlFailed('https://example.com');

    expect(mockSend).toHaveBeenCalledWith({
      title: 'Crawl Failed',
      body: 'Failed to crawl https://example.com',
    });
  });
});
