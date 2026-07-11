import { useEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Download, MagnifyingGlass, SpinnerGap, Warning } from '@phosphor-icons/react';
import { useEscapeStack } from '../contexts/EscapeStack';
import type { SitemapEntry, SitemapResult } from '../types';

const WARN_URL_THRESHOLD = 1000;

interface SitemapPickerModalProps {
  /** Sitemap URL to fetch. If empty, modal shows a URL input instead. */
  sitemapUrl: string;
  /** True if backend SSRF protection is active for the current crawl config. */
  ssrfProtection: boolean;
  onClose: () => void;
  /** Called with the selected URLs when the user confirms. */
  onConfirm: (urls: string[]) => void;
}

export function SitemapPickerModal({
  sitemapUrl,
  ssrfProtection,
  onClose,
  onConfirm,
}: SitemapPickerModalProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [result, setResult] = useState<SitemapResult | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState('');
  const [prefixFilter, setPrefixFilter] = useState('');
  const escapeStack = useEscapeStack();

  useEffect(() => {
    const id = escapeStack.push(onClose);
    return () => escapeStack.remove(id);
  }, [onClose]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError('');
    invoke<SitemapResult>('fetch_sitemap', {
      url: sitemapUrl,
      ssrfProtection,
    })
      .then((r) => {
        if (cancelled) return;
        setResult(r);
        setSelected(new Set(r.entries.map((e) => e.url)));
      })
      .catch((err) => {
        if (cancelled) return;
        setError(String(err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [sitemapUrl, ssrfProtection]);

  const filtered: SitemapEntry[] = useMemo(() => {
    if (!result) return [];
    const q = query.trim().toLowerCase();
    const prefix = prefixFilter.trim();
    return result.entries.filter((e) => {
      if (q && !e.url.toLowerCase().includes(q)) return false;
      if (prefix) {
        try {
          const path = new URL(e.url).pathname;
          if (!path.startsWith(prefix)) return false;
        } catch {
          return false;
        }
      }
      return true;
    });
  }, [result, query, prefixFilter]);

  const toggle = (url: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(url)) next.delete(url);
      else next.add(url);
      return next;
    });
  };

  const selectAllVisible = () => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const e of filtered) next.add(e.url);
      return next;
    });
  };

  const deselectAllVisible = () => {
    setSelected((prev) => {
      const next = new Set(prev);
      for (const e of filtered) next.delete(e.url);
      return next;
    });
  };

  const handleConfirm = () => {
    onConfirm(Array.from(selected));
  };

  const totalCount = result?.entries.length ?? 0;
  const showLargeWarning = totalCount >= WARN_URL_THRESHOLD;

  return createPortal(
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 bg-black/40 z-40"
        onClick={onClose}
      />
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.95 }}
        transition={{ type: 'spring', damping: 25, stiffness: 300 }}
        className="fixed inset-0 m-auto w-[720px] max-h-[85vh] h-fit bg-deepVoid border border-abyssal/50 rounded-xl z-50 shadow-2xl flex flex-col"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-abyssal/50">
          <div>
            <h2 className="text-ghost font-semibold text-base">Import from Sitemap</h2>
            <p className="text-[11px] text-charcoal mt-0.5 truncate max-w-[560px]">{sitemapUrl}</p>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
            aria-label="Close"
          >
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-hidden flex flex-col">
          {loading && (
            <div className="flex-1 flex items-center justify-center text-charcoal text-sm gap-3">
              <SpinnerGap size={20} className="animate-spin" />
              Fetching sitemap…
            </div>
          )}

          {!loading && error && (
            <div className="flex-1 flex flex-col items-center justify-center p-8 text-center gap-2">
              <Warning size={28} className="text-crimson" />
              <p className="text-sm text-ghost">Could not load sitemap</p>
              <p className="text-xs text-charcoal max-w-[480px]">{error}</p>
            </div>
          )}

          {!loading && !error && result && (
            <>
              {/* Warnings */}
              <div className="px-5 pt-4 space-y-2">
                {showLargeWarning && (
                  <div className="flex items-start gap-2 text-xs text-amber bg-amber/10 border border-amber/30 rounded-md px-3 py-2">
                    <Warning size={14} className="mt-0.5 flex-shrink-0" />
                    <span>
                      Large sitemap: {totalCount.toLocaleString()} URLs. Select carefully to
                      keep the crawl focused.
                    </span>
                  </div>
                )}
                {result.truncated && (
                  <div className="flex items-start gap-2 text-xs text-crimson bg-crimson/10 border border-crimson/30 rounded-md px-3 py-2">
                    <Warning size={14} className="mt-0.5 flex-shrink-0" />
                    <span>
                      Result truncated at 10,000 URLs. The site's sitemap contains more entries
                      than are shown.
                    </span>
                  </div>
                )}
              </div>

              {/* Filter bar */}
              <div className="px-5 pt-3 pb-2 grid grid-cols-2 gap-3">
                <div className="relative">
                  <MagnifyingGlass
                    size={14}
                    className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal"
                  />
                  <input
                    type="text"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder="Filter URLs…"
                    className="w-full bg-surface/50 border border-abyssal rounded-md pl-9 pr-3 py-2 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 transition-all"
                  />
                </div>
                <input
                  type="text"
                  value={prefixFilter}
                  onChange={(e) => setPrefixFilter(e.target.value)}
                  placeholder="Only paths starting with… (e.g. /docs/)"
                  className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 transition-all"
                />
              </div>

              {/* Controls */}
              <div className="px-5 pb-2 flex items-center justify-between text-xs text-charcoal">
                <span>
                  {filtered.length.toLocaleString()} of {totalCount.toLocaleString()} shown ·{' '}
                  <span className="text-accentGreen">{selected.size.toLocaleString()}</span> selected
                </span>
                <div className="flex items-center gap-3">
                  <button
                    onClick={selectAllVisible}
                    className="text-accentGreen hover:text-brightGreen transition-colors"
                  >
                    Select visible
                  </button>
                  <button
                    onClick={deselectAllVisible}
                    className="text-charcoal hover:text-ghost transition-colors"
                  >
                    Deselect visible
                  </button>
                </div>
              </div>

              {/* URL list */}
              <div className="flex-1 overflow-y-auto border-t border-abyssal/30">
                {filtered.length === 0 ? (
                  <p className="p-8 text-center text-sm text-charcoal">
                    No URLs match the current filter.
                  </p>
                ) : (
                  <ul className="divide-y divide-abyssal/30">
                    {filtered.slice(0, 500).map((entry) => {
                      const checked = selected.has(entry.url);
                      return (
                        <li key={entry.url}>
                          <label className="flex items-center gap-3 px-5 py-2 hover:bg-surface/30 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={checked}
                              onChange={() => toggle(entry.url)}
                              className="w-4 h-4 rounded border-abyssal bg-surface text-accentGreen focus:ring-accentGreen/20 flex-shrink-0"
                            />
                            <div className="flex-1 min-w-0">
                              <p className="text-sm text-ghost truncate font-mono">{entry.url}</p>
                              {entry.lastmod && (
                                <p className="text-[10px] text-charcoal">
                                  Last modified: {entry.lastmod}
                                </p>
                              )}
                            </div>
                          </label>
                        </li>
                      );
                    })}
                    {filtered.length > 500 && (
                      <li className="px-5 py-3 text-center text-xs text-charcoal bg-surface/20">
                        Showing first 500 of {filtered.length.toLocaleString()} matches. Refine the
                        filter to see more.
                      </li>
                    )}
                  </ul>
                )}
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-5 py-4 border-t border-abyssal/50">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-charcoal hover:text-ghost transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleConfirm}
            disabled={loading || !!error || selected.size === 0}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-accentGreen/20 text-accentGreen border border-accentGreen/30 rounded-md hover:bg-accentGreen/30 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Download size={14} />
            Import {selected.size > 0 ? `${selected.size.toLocaleString()} URL${selected.size === 1 ? '' : 's'}` : ''}
          </button>
        </div>
      </motion.div>
    </AnimatePresence>,
    document.body,
  );
}
