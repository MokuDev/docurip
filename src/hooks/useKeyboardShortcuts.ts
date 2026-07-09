import { useEffect, useRef } from 'react';

export interface ShortcutAction {
  id: string;
  label: string;
  category: string;
  defaultKeys: string;
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  { id: 'new-crawl', label: 'New / Active Crawl', category: 'Navigation', defaultKeys: 'mod+n' },
  { id: 'search', label: 'Focus Search', category: 'Navigation', defaultKeys: 'mod+f' },
  { id: 'dashboard', label: 'Dashboard', category: 'Navigation', defaultKeys: 'mod+d' },
  { id: 'history', label: 'History', category: 'Navigation', defaultKeys: 'mod+h' },
  { id: 'settings', label: 'Settings', category: 'Navigation', defaultKeys: 'mod+,' },
  { id: 'import', label: 'Import', category: 'Navigation', defaultKeys: 'mod+i' },
];

const MODIFIER_KEYS = new Set(['control', 'meta', 'shift', 'alt']);

export function resolveBinding(actionId: string, overrides: Record<string, string>): string {
  const override = overrides[actionId];
  if (override !== undefined) return override;
  return SHORTCUT_ACTIONS.find((a) => a.id === actionId)?.defaultKeys ?? '';
}

export function normalizeCombo(e: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>): string {
  const key = e.key.toLowerCase();
  if (MODIFIER_KEYS.has(key)) return '';
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('mod');
  if (e.shiftKey) parts.push('shift');
  if (e.altKey) parts.push('alt');
  parts.push(key === ' ' ? 'space' : key);
  return parts.join('+');
}

export function formatCombo(combo: string): string {
  if (!combo) return '—';
  const isMac = typeof navigator !== 'undefined' && /mac/i.test(navigator.userAgent);
  return combo
    .split('+')
    .map((part) => {
      if (part === 'mod') return isMac ? '⌘' : 'Ctrl';
      if (part === 'shift') return isMac ? '⇧' : 'Shift';
      if (part === 'alt') return isMac ? '⌥' : 'Alt';
      if (part === 'escape') return 'Esc';
      if (part === 'space') return 'Space';
      return part.length === 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join(isMac ? '' : '+');
}

interface UseKeyboardShortcutsOptions {
  handlers: Record<string, () => void>;
  onEscape: () => void;
  overrides?: Record<string, string>;
}

export function useKeyboardShortcuts({ handlers, onEscape, overrides = {} }: UseKeyboardShortcutsOptions): void {
  const ref = useRef({ handlers, onEscape, overrides });
  ref.current = { handlers, onEscape, overrides };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        ref.current.onEscape();
        return;
      }

      const target = e.target as HTMLElement;
      const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT';
      if (isInput) return;

      const combo = normalizeCombo(e);
      if (!combo) return;

      const { handlers, overrides } = ref.current;
      for (const action of SHORTCUT_ACTIONS) {
        if (resolveBinding(action.id, overrides) === combo) {
          e.preventDefault();
          handlers[action.id]?.();
          return;
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, []);
}
