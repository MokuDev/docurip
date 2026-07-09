import { useEffect, useRef } from 'react';

interface ShortcutHandlers {
  onNewCrawl: () => void;
  onSearch: () => void;
  onEscape: () => void;
}

export function useKeyboardShortcuts(handlers: ShortcutHandlers): void {
  const ref = useRef(handlers);
  ref.current = handlers;

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      const target = e.target as HTMLElement;
      const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT';

      if (e.key === 'Escape') {
        ref.current.onEscape();
        return;
      }

      if (isInput) return;

      const key = e.key.toLowerCase();
      if (mod && key === 'n') {
        e.preventDefault();
        ref.current.onNewCrawl();
      } else if (mod && key === 'f') {
        e.preventDefault();
        ref.current.onSearch();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
