import { createContext, useContext, useCallback, useEffect, useState, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sun, Moon, Desktop } from '@phosphor-icons/react';
import type { AppSettings, ThemePreference } from '../types';
import { useToasts } from './useToasts';

const STORAGE_KEY = 'docurip-theme';
const LIGHT_QUERY = '(prefers-color-scheme: light)';

export const THEME_ORDER: ThemePreference[] = ['dark', 'light', 'system'];
export const THEME_META: Record<ThemePreference, { label: string; icon: typeof Sun }> = {
  dark: { label: 'Dark', icon: Moon },
  light: { label: 'Light', icon: Sun },
  system: { label: 'System', icon: Desktop },
};

function systemPrefersLight() {
  return window.matchMedia(LIGHT_QUERY).matches;
}

function applyTheme(resolved: 'dark' | 'light') {
  document.documentElement.classList.remove('dark', 'light');
  document.documentElement.classList.add(resolved);
}

interface ThemeContextValue {
  theme: ThemePreference;
  resolvedTheme: 'dark' | 'light';
  setTheme: (theme: ThemePreference) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const { pushToast } = useToasts();
  const [theme, setThemeState] = useState<ThemePreference>(() => {
    const cached = localStorage.getItem(STORAGE_KEY) as ThemePreference | null;
    return cached ?? 'system';
  });
  const [systemIsLight, setSystemIsLight] = useState(systemPrefersLight);
  const resolvedTheme: 'dark' | 'light' = theme === 'system' ? (systemIsLight ? 'light' : 'dark') : theme;

  // Reconcile with the persisted preference once the backend is reachable.
  // Uses the functional setState form so it compares against the *current*
  // theme at resolution time rather than whatever was captured when this
  // effect was created.
  useEffect(() => {
    invoke<AppSettings>('get_settings')
      .then((settings) => {
        if (!settings?.theme) return;
        setThemeState((current) => (settings.theme !== current ? settings.theme : current));
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    const mql = window.matchMedia(LIGHT_QUERY);
    const onChange = () => setSystemIsLight(mql.matches);
    mql.addEventListener('change', onChange);
    return () => mql.removeEventListener('change', onChange);
  }, []);

  useEffect(() => {
    applyTheme(resolvedTheme);
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme, resolvedTheme]);

  const setTheme = useCallback((next: ThemePreference) => {
    setThemeState(next);
    // Persists only the theme field (see src-tauri set_theme) so this never
    // races with the Settings page's full-AppSettings save/reset flow.
    invoke('set_theme', { theme: next }).catch(() => {
      pushToast('error', 'Failed to save theme preference');
    });
  }, [pushToast]);

  return (
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
