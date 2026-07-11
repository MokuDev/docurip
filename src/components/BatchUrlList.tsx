import { useMemo } from 'react';
import { Warning } from '@phosphor-icons/react';

interface BatchUrlListProps {
  value: string[];
  onChange: (urls: string[]) => void;
  disabled?: boolean;
}

const MAX_BATCH_URLS = 500;

const isValidHttpUrl = (raw: string): boolean => {
  try {
    const u = new URL(raw);
    return u.protocol === 'http:' || u.protocol === 'https:';
  } catch {
    return false;
  }
};

export function BatchUrlList({ value, onChange, disabled }: BatchUrlListProps) {
  const text = useMemo(() => value.join('\n'), [value]);

  const stats = useMemo(() => {
    const lines = value.map((l) => l.trim()).filter(Boolean);
    const invalid = lines.filter((l) => !isValidHttpUrl(l));
    const unique = new Set(lines);
    const duplicates = lines.length - unique.size;
    return {
      count: lines.length,
      invalid: invalid.length,
      duplicates,
      overLimit: lines.length > MAX_BATCH_URLS,
    };
  }, [value]);

  const handleChange = (raw: string) => {
    // Preserve blank trailing lines while typing but strip leading whitespace
    // per-line so pasted lists come out clean.
    onChange(raw.split('\n').map((l) => l.replace(/^\s+/, '')));
  };

  const handleDedupeAndClean = () => {
    const seen = new Set<string>();
    const cleaned: string[] = [];
    for (const line of value) {
      const t = line.trim();
      if (!t) continue;
      if (!seen.has(t)) {
        seen.add(t);
        cleaned.push(t);
      }
    }
    onChange(cleaned);
  };

  return (
    <div>
      <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
        URLs to crawl (one per line)
      </label>
      <textarea
        value={text}
        onChange={(e) => handleChange(e.target.value)}
        disabled={disabled}
        rows={8}
        placeholder={`https://docs.example.com/v1\nhttps://docs.example.com/v2\nhttps://api.example.com`}
        className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm font-mono placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
      />

      <div className="mt-1.5 flex items-center justify-between text-[11px]">
        <span className="text-charcoal">
          {stats.count} URL{stats.count === 1 ? '' : 's'}
          {stats.invalid > 0 && (
            <span className="text-crimson"> · {stats.invalid} invalid</span>
          )}
          {stats.duplicates > 0 && (
            <span className="text-amber"> · {stats.duplicates} duplicate{stats.duplicates === 1 ? '' : 's'}</span>
          )}
        </span>
        {(stats.duplicates > 0 || stats.invalid > 0) && !disabled && (
          <button
            type="button"
            onClick={handleDedupeAndClean}
            className="text-accentGreen hover:text-brightGreen transition-colors"
          >
            Clean up
          </button>
        )}
      </div>

      {stats.overLimit && (
        <div className="mt-1.5 flex items-start gap-1.5 text-[11px] text-crimson">
          <Warning size={12} className="mt-0.5 flex-shrink-0" />
          <span>Batch is capped at {MAX_BATCH_URLS} URLs; extra lines will be ignored.</span>
        </div>
      )}
    </div>
  );
}

export const BATCH_MAX_URLS = MAX_BATCH_URLS;

export function sanitizeBatchUrls(urls: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const raw of urls) {
    const t = raw.trim();
    if (!t) continue;
    if (!isValidHttpUrl(t)) continue;
    if (seen.has(t)) continue;
    seen.add(t);
    out.push(t);
    if (out.length >= MAX_BATCH_URLS) break;
  }
  return out;
}
