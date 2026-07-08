import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';

async function ensurePermission(): Promise<boolean> {
  let permitted = await isPermissionGranted();
  if (!permitted) {
    const result = await requestPermission();
    permitted = result === 'granted';
  }
  return permitted;
}

export async function notifyCrawlComplete(jobUrl: string, pageCount: number): Promise<void> {
  try {
    if (!(await ensurePermission())) return;
    sendNotification({
      title: 'Crawl Complete',
      body: `Finished crawling ${jobUrl} — ${pageCount} pages saved`,
    });
  } catch (err) {
    console.warn('Failed to send completion notification:', err);
  }
}

export async function notifyCrawlFailed(jobUrl: string, error?: string): Promise<void> {
  try {
    if (!(await ensurePermission())) return;
    sendNotification({
      title: 'Crawl Failed',
      body: `Failed to crawl ${jobUrl}${error ? `: ${error}` : ''}`,
    });
  } catch (err) {
    console.warn('Failed to send failure notification:', err);
  }
}
