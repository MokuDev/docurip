import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { ToastProvider, useToasts } from './useToasts';

describe('useToasts', () => {
  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <ToastProvider>{children}</ToastProvider>
  );

  it('should push a toast', () => {
    const { result } = renderHook(() => useToasts(), { wrapper });
    
    act(() => {
      result.current.pushToast('success', 'Test message');
    });
    
    expect(result.current.toasts).toHaveLength(1);
    expect(result.current.toasts[0].message).toBe('Test message');
    expect(result.current.toasts[0].type).toBe('success');
  });

  it('should dismiss a toast manually', () => {
    const { result } = renderHook(() => useToasts(), { wrapper });
    
    act(() => {
      result.current.pushToast('info', 'Manual remove');
    });
    
    expect(result.current.toasts).toHaveLength(1);
    const toastId = result.current.toasts[0].id;
    
    act(() => {
      result.current.dismissToast(toastId);
    });
    
    expect(result.current.toasts).toHaveLength(0);
  });

  it('should auto-dismiss non-error toasts after 6s', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useToasts(), { wrapper });
    
    act(() => {
      result.current.pushToast('info', 'Auto dismiss');
    });
    
    expect(result.current.toasts).toHaveLength(1);
    
    act(() => {
      vi.advanceTimersByTime(6000);
    });
    
    expect(result.current.toasts).toHaveLength(0);
    vi.useRealTimers();
  });

  it('should not auto-dismiss error toasts', async () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useToasts(), { wrapper });
    
    act(() => {
      result.current.pushToast('error', 'Persistent error');
    });
    
    act(() => {
      vi.advanceTimersByTime(6000);
    });
    
    expect(result.current.toasts).toHaveLength(1);
    vi.useRealTimers();
  });
});
