import { createContext, useContext, useCallback, useEffect, useState, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, ThemePreference } from '../types';

const STORAGE_KEY = 'docurip-theme';

function resolveTheme(pref: ThemePreference): 'dark' | 'light' {
  if (pref === 'system') {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  }
  return pref;
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
  const [theme, setThemeState] = useState<ThemePreference>(() => {
    const cached = localStorage.getItem(STORAGE_KEY) as ThemePreference | null;
    return cached ?? 'system';
  });
  const [resolvedTheme, setResolvedTheme] = useState<'dark' | 'light'>(() => resolveTheme(theme));

  useEffect(() => {
    invoke<AppSettings>('get_settings')
      .then((settings) => {
        if (settings?.theme && settings.theme !== theme) {
          setThemeState(settings.theme);
        }
      })
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const resolved = resolveTheme(theme);
    setResolvedTheme(resolved);
    applyTheme(resolved);
    localStorage.setItem(STORAGE_KEY, theme);

    if (theme === 'system') {
      const mql = window.matchMedia('(prefers-color-scheme: light)');
      const onChange = () => {
        const next = resolveTheme('system');
        setResolvedTheme(next);
        applyTheme(next);
      };
      mql.addEventListener('change', onChange);
      return () => mql.removeEventListener('change', onChange);
    }
  }, [theme]);

  const setTheme = useCallback((next: ThemePreference) => {
    setThemeState(next);
    invoke<AppSettings>('get_settings')
      .then((settings) => invoke('update_settings', { settings: { ...settings, theme: next } }))
      .catch(() => {});
  }, []);

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
