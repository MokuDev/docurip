import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../types';

export const SHORTCUTS_UPDATED_EVENT = 'docurip:shortcuts-updated';

export function useShortcutOverrides(): Record<string, string> {
  const [overrides, setOverrides] = useState<Record<string, string>>({});

  const load = useCallback(() => {
    invoke<AppSettings>('get_settings')
      .then((s) => setOverrides(s.shortcutOverrides ?? {}))
      .catch((err) => console.warn('Failed to load shortcut overrides:', err));
  }, []);

  useEffect(() => {
    load();
    window.addEventListener(SHORTCUTS_UPDATED_EVENT, load);
    return () => window.removeEventListener(SHORTCUTS_UPDATED_EVENT, load);
  }, [load]);

  return overrides;
}
