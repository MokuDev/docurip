import { useState, useEffect, useRef } from 'react';

interface AnimatedCounterProps {
  value: number;
  formatValue?: (v: number) => string;
  duration?: number;
}

export function AnimatedCounter({
  value,
  formatValue,
  duration = 600,
}: AnimatedCounterProps) {
  const [display, setDisplay] = useState(value);
  const rafRef = useRef<number>(0);
  const fromRef = useRef(value);
  const toRef = useRef(value);
  const startRef = useRef(0);

  useEffect(() => {
    cancelAnimationFrame(rafRef.current);
    fromRef.current = display;
    toRef.current = value;
    startRef.current = performance.now();

    if (fromRef.current === toRef.current) return;

    const tick = (now: number) => {
      const elapsed = now - startRef.current;
      const progress = Math.min(elapsed / duration, 1);
      // ease-out cubic
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = fromRef.current + (toRef.current - fromRef.current) * eased;
      setDisplay(current);
      if (progress < 1) {
        rafRef.current = requestAnimationFrame(tick);
      }
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [value, duration]);

  const formatted = formatValue ? formatValue(display) : Math.round(display).toString();
  return <>{formatted}</>;
}
