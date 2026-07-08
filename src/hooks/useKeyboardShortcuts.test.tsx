import { describe, it, expect, vi } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useKeyboardShortcuts } from './useKeyboardShortcuts';

function fireKey(key: string, opts: Partial<KeyboardEventInit> = {}) {
  document.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...opts }));
}

describe('useKeyboardShortcuts', () => {
  it('fires onNewCrawl on Ctrl+N', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    fireKey('n', { ctrlKey: true });
    expect(handlers.onNewCrawl).toHaveBeenCalledOnce();
  });

  it('fires onSearch on Ctrl+F', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    fireKey('f', { ctrlKey: true });
    expect(handlers.onSearch).toHaveBeenCalledOnce();
  });

  it('fires onEscape on Escape', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    fireKey('Escape');
    expect(handlers.onEscape).toHaveBeenCalledOnce();
  });

  it('does not fire onNewCrawl without modifier', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    fireKey('n');
    expect(handlers.onNewCrawl).not.toHaveBeenCalled();
  });

  it('fires onNewCrawl with metaKey (Cmd on Mac)', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    fireKey('n', { metaKey: true });
    expect(handlers.onNewCrawl).toHaveBeenCalledOnce();
  });

  it('suppresses Ctrl+N when target is an input', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', ctrlKey: true, bubbles: true }));
    expect(handlers.onNewCrawl).not.toHaveBeenCalled();
    document.body.removeChild(input);
  });

  it('still fires Escape when target is an input', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    renderHook(() => useKeyboardShortcuts(handlers));

    const input = document.createElement('input');
    document.body.appendChild(input);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(handlers.onEscape).toHaveBeenCalledOnce();
    document.body.removeChild(input);
  });

  it('cleans up listener on unmount', () => {
    const handlers = { onNewCrawl: vi.fn(), onSearch: vi.fn(), onEscape: vi.fn() };
    const { unmount } = renderHook(() => useKeyboardShortcuts(handlers));

    unmount();
    fireKey('Escape');
    expect(handlers.onEscape).not.toHaveBeenCalled();
  });
});
