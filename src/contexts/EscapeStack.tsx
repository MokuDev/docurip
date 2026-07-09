import { createContext, useContext, useRef, useCallback } from 'react';

interface EscapeStackContextValue {
  push: (handler: () => void) => string;
  remove: (id: string) => void;
  fireTop: () => boolean;
}

const EscapeStackContext = createContext<EscapeStackContextValue>({
  push: () => '',
  remove: () => {},
  fireTop: () => false,
});

let nextId = 0;

export function EscapeStackProvider({ children }: { children: React.ReactNode }) {
  const stack = useRef<Map<string, () => void>>(new Map());

  const push = useCallback((handler: () => void): string => {
    const id = `esc-${++nextId}`;
    stack.current.set(id, handler);
    return id;
  }, []);

  const remove = useCallback((id: string) => {
    stack.current.delete(id);
  }, []);

  const fireTop = useCallback((): boolean => {
    const entries = Array.from(stack.current.entries());
    if (entries.length === 0) return false;
    const [, handler] = entries[entries.length - 1];
    handler();
    return true;
  }, []);

  return (
    <EscapeStackContext.Provider value={{ push, remove, fireTop }}>
      {children}
    </EscapeStackContext.Provider>
  );
}

export function useEscapeStack() {
  return useContext(EscapeStackContext);
}
