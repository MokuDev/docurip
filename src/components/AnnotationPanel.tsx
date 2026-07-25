import { useEffect, useRef, useState } from 'react';
import { NotePencil } from '@phosphor-icons/react';

interface AnnotationPanelProps {
  /** Job id used when persisting via `set_annotation`. */
  jobId: string;
  /** Page URL the note belongs to. Changing this loads the note for the new page. */
  pageUrl: string;
  /** Current stored value for `pageUrl` (from job.annotations). */
  value: string;
  /** Called after a successful debounced save with the trimmed text. */
  onSaved: (trimmed: string) => void;
  /** Delegates the actual persistence call so tests can stub without Tauri. */
  save: (url: string, text: string) => Promise<void>;
}

type SaveState = 'idle' | 'dirty' | 'saving' | 'saved' | 'error';

const DEBOUNCE_MS = 700;

/**
 * Text area with debounced autosave for per-page user notes. Owns its
 * own draft so keystrokes stay local; only pushes to the backend once
 * the user stops typing.
 */
export function AnnotationPanel({ jobId, pageUrl, value, onSaved, save }: AnnotationPanelProps) {
  const [draft, setDraft] = useState(value);
  const [state, setState] = useState<SaveState>('idle');
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const inflightUrl = useRef<string | null>(null);

  // Reset the draft when the page changes so a stale value from a
  // previous selection can't overwrite the newly loaded one.
  useEffect(() => {
    setDraft(value);
    setState('idle');
    if (timerRef.current) clearTimeout(timerRef.current);
  }, [pageUrl, value]);

  const flush = async (text: string, url: string) => {
    setState('saving');
    inflightUrl.current = url;
    try {
      await save(url, text);
      if (inflightUrl.current === url) {
        setState('saved');
        onSaved(text.trim());
      }
    } catch {
      if (inflightUrl.current === url) {
        setState('error');
      }
    }
  };

  const scheduleSave = (next: string) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setState('dirty');
    const url = pageUrl;
    timerRef.current = setTimeout(() => {
      flush(next, url);
    }, DEBOUNCE_MS);
  };

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const next = e.target.value;
    setDraft(next);
    scheduleSave(next);
  };

  // Force-flush on blur so a note is durable if the user selects
  // another page or closes the browser before the debounce fires.
  const handleBlur = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    if (draft.trim() !== value.trim()) {
      flush(draft, pageUrl);
    }
  };

  const statusLabel =
    state === 'saving'
      ? 'Saving…'
      : state === 'saved'
        ? 'Saved'
        : state === 'error'
          ? 'Save failed'
          : state === 'dirty'
            ? 'Unsaved'
            : '';

  return (
    <div className="border-t border-abyssal/50 bg-surface/20 flex flex-col" data-job-id={jobId}>
      <div className="px-4 pt-2 pb-1 flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-charcoal text-[11px] font-medium uppercase tracking-wider">
          <NotePencil size={12} weight="regular" />
          <span>Notes</span>
        </div>
        <span
          className={`text-[11px] tabular-nums ${
            state === 'error'
              ? 'text-crimson'
              : state === 'saved'
                ? 'text-accentGreen'
                : 'text-charcoal'
          }`}
          aria-live="polite"
        >
          {statusLabel}
        </span>
      </div>
      <textarea
        value={draft}
        onChange={handleChange}
        onBlur={handleBlur}
        placeholder="Notes for this page…"
        rows={4}
        className="mx-4 mb-3 bg-deepVoid border border-abyssal/60 rounded-md px-3 py-2 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
      />
    </div>
  );
}
