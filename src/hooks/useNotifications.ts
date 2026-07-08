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
  if (!(await ensurePermission())) return;
  sendNotification({
    title: 'Crawl Complete',
    body: `Finished crawling ${jobUrl} — ${pageCount} pages saved`,
  });
}

export async function notifyCrawlFailed(jobUrl: string, error?: string): Promise<void> {
  if (!(await ensurePermission())) return;
  sendNotification({
    title: 'Crawl Failed',
    body: `Failed to crawl ${jobUrl}${error ? `: ${error}` : ''}`,
  });
}
