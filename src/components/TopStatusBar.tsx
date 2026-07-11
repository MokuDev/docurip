import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SessionInfo } from '../types';
import { useCrawlEvents } from '../hooks/useCrawlEvents';
import { useTheme, THEME_ORDER, THEME_META } from '../hooks/useTheme';

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  return [h, m, s].map((v) => String(v).padStart(2, '0')).join(':');
}

export function TopStatusBar() {
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [now, setNow] = useState<number>(Date.now());
  const { activeJobIds } = useCrawlEvents();
  const { theme, setTheme } = useTheme();

  useEffect(() => {
    let cancelled = false;
    invoke<SessionInfo>('get_session_info')
      .then((s) => {
        if (!cancelled) setSession(s);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  const shortId = session ? session.id.slice(0, 8) : '--------';
  void now;

  const cycleTheme = () => {
    const next = THEME_ORDER[(THEME_ORDER.indexOf(theme) + 1) % THEME_ORDER.length];
    setTheme(next);
  };
  const ThemeIcon = THEME_META[theme].icon;

  return (
    <div className="h-6 w-full bg-deepVoid border-b border-abyssal/50 flex items-center justify-between px-3 text-[10px] font-mono text-secondary select-none">
      <div className="flex items-center gap-2">
        <span className="text-charcoal font-medium uppercase tracking-widest">Session</span>
        <span className="text-ghost">{shortId}</span>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-charcoal font-medium uppercase tracking-widest">Uptime</span>
        <span className="text-accentGreen tabular-nums">
          {session ? formatUptime(session.uptimeSecs) : '--:--:--'}
        </span>
      </div>
      <div className="flex items-center gap-3">
        <span className="text-charcoal font-medium uppercase tracking-widest">Jobs</span>
        <span className={activeJobIds.size > 0 ? 'text-accentGreen tabular-nums' : 'text-charcoal tabular-nums'}>
          {activeJobIds.size}
        </span>
        <button
          type="button"
          onClick={cycleTheme}
          title={`Theme: ${theme}`}
          className="flex items-center text-charcoal hover:text-accentGreen transition-colors"
        >
          <ThemeIcon size={12} />
        </button>
      </div>
    </div>
  );
}
