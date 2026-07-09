import { describe, it, expect, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useKeyboardShortcuts, normalizeCombo, formatCombo, resolveBinding } from './useKeyboardShortcuts';

function fireKey(key: string, opts: Partial<KeyboardEventInit> = {}) {
  document.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...opts }));
}

function makeHandlers() {
  return {
    onEscape: vi.fn(),
    handlers: {
      'new-crawl': vi.fn(),
      search: vi.fn(),
      dashboard: vi.fn(),
    },
  };
}

describe('useKeyboardShortcuts', () => {
  it('fires new-crawl on Ctrl+N', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    fireKey('n', { ctrlKey: true });
    expect(handlers['new-crawl']).toHaveBeenCalledOnce();
  });

  it('fires search on Ctrl+F', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    fireKey('f', { ctrlKey: true });
    expect(handlers.search).toHaveBeenCalledOnce();
  });

  it('fires onEscape on Escape', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    fireKey('Escape');
    expect(onEscape).toHaveBeenCalledOnce();
  });

  it('does not fire new-crawl without modifier', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    fireKey('n');
    expect(handlers['new-crawl']).not.toHaveBeenCalled();
  });

  it('fires new-crawl with metaKey (Cmd on Mac)', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    fireKey('n', { metaKey: true });
    expect(handlers['new-crawl']).toHaveBeenCalledOnce();
  });

  it('suppresses Ctrl+N when target is an input', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true }));
    expect(handlers['new-crawl']).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });

  it('still fires Escape when target is an input', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(onEscape).toHaveBeenCalledOnce();
    document.body.removeChild(input);
  });

  it('cleans up listener on unmount', () => {
    const { handlers, onEscape } = makeHandlers();
    const { unmount } = renderHook(() => useKeyboardShortcuts({ handlers, onEscape }));

    unmount();
    fireKey('Escape');
    expect(onEscape).not.toHaveBeenCalled();
  });

  it('respects a custom override binding', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape, overrides: { dashboard: 'mod+shift+d' } }));

    fireKey('d', { ctrlKey: true, shiftKey: true });
    expect(handlers.dashboard).toHaveBeenCalledOnce();
  });

  it('does not fire the default binding once overridden', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape, overrides: { dashboard: 'mod+shift+d' } }));

    fireKey('d', { ctrlKey: true });
    expect(handlers.dashboard).not.toHaveBeenCalled();
  });

  it('an unbound action ("") never fires', () => {
    const { handlers, onEscape } = makeHandlers();
    renderHook(() => useKeyboardShortcuts({ handlers, onEscape, overrides: { search: '' } }));

    fireKey('f', { ctrlKey: true });
    expect(handlers.search).not.toHaveBeenCalled();
  });
});

describe('normalizeCombo', () => {
  it('normalizes ctrl and meta to "mod"', () => {
    expect(normalizeCombo({ key: 'n', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false })).toBe('mod+n');
    expect(normalizeCombo({ key: 'n', ctrlKey: false, metaKey: true, shiftKey: false, altKey: false })).toBe('mod+n');
  });

  it('returns empty string for a bare modifier key press', () => {
    expect(normalizeCombo({ key: 'Control', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false })).toBe('');
    expect(normalizeCombo({ key: 'Shift', ctrlKey: false, metaKey: false, shiftKey: true, altKey: false })).toBe('');
  });

  it('combines multiple modifiers in order mod, shift, alt', () => {
    expect(normalizeCombo({ key: 'd', ctrlKey: true, metaKey: false, shiftKey: true, altKey: true })).toBe('mod+shift+alt+d');
  });

  it('lowercases the key', () => {
    expect(normalizeCombo({ key: 'N', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false })).toBe('mod+n');
  });
});

describe('resolveBinding', () => {
  it('returns the default binding when no override exists', () => {
    expect(resolveBinding('dashboard', {})).toBe('mod+d');
  });

  it('returns the override when present', () => {
    expect(resolveBinding('dashboard', { dashboard: 'mod+shift+d' })).toBe('mod+shift+d');
  });

  it('returns an empty string when explicitly unbound', () => {
    expect(resolveBinding('dashboard', { dashboard: '' })).toBe('');
  });
});

describe('formatCombo', () => {
  it('formats an empty combo as an em dash', () => {
    expect(formatCombo('')).toBe('—');
  });

  it('formats a single letter key uppercase', () => {
    expect(formatCombo('mod+n')).toMatch(/N$/);
  });
});
