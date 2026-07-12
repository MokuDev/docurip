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

interface Entry {
  id: string;
  handler: () => void;
}

export function EscapeStackProvider({ children }: { children: React.ReactNode }) {
  const stack = useRef<Entry[]>([]);

  const push = useCallback((handler: () => void): string => {
    const id = `esc-${++nextId}`;
    stack.current.push({ id, handler });
    return id;
  }, []);

  const remove = useCallback((id: string) => {
    const arr = stack.current;
    for (let i = arr.length - 1; i >= 0; i--) {
      if (arr[i].id === id) {
        arr.splice(i, 1);
        return;
      }
    }
  }, []);

  const fireTop = useCallback((): boolean => {
    const arr = stack.current;
    if (arr.length === 0) return false;
    arr[arr.length - 1].handler();
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
