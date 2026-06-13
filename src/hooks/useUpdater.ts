import { useEffect, useState } from 'react';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

interface UpdateInfo {
  version: string;
  body: string;
}

export function useUpdater() {
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function checkForUpdate() {
      try {
        const update = await check();
        if (update && !cancelled) {
          setUpdateAvailable({
            version: update.version,
            body: update.body ?? '',
          });
        }
      } catch (err) {
        if (!cancelled) {
          console.warn('Update check failed:', err);
          setError(String(err));
        }
      }
    }

    checkForUpdate();
    return () => { cancelled = true; };
  }, []);

  const installUpdate = async () => {
    setDownloading(true);
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        await relaunch();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setDownloading(false);
    }
  };

  return { updateAvailable, downloading, error, installUpdate, dismiss: () => setUpdateAvailable(null) };
}
