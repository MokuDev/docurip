import { useMemo, useEffect, useRef, useState } from 'react';
import DOMPurify from 'dompurify';
import { CaretUp, CaretDown } from '@phosphor-icons/react';

interface MarkdownPreviewProps {
  content: string;
  searchQuery?: string;
}

const ALLOWED_TAGS = ['p', 'h1', 'h2', 'h3', 'h4', 'strong', 'em', 'code', 'pre', 'a', 'ul', 'ol', 'li', 'blockquote', 'hr', 'div', 'mark'];
const ALLOWED_ATTR = ['class', 'href', 'target', 'rel', 'data-match-index'];
const ALLOWED_URI_REGEXP = /^https?:/i;

const HIGHLIGHT_CLASS = 'bg-accentGreen/30 text-accentGreen rounded px-0.5';
const HIGHLIGHT_ACTIVE_CLASS = 'bg-accentGreen text-slate-900 rounded px-0.5';

export function MarkdownPreview({ content, searchQuery }: MarkdownPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [matchCount, setMatchCount] = useState(0);
  const [activeMatch, setActiveMatch] = useState(0);

  const html = useMemo(() => {
    let md = content;

    // Escape HTML
    md = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

    // Code blocks ```...```
    md = md.replace(/```(\w+)?\n([\s\S]*?)```/g, (_, _lang, code) => {
      return `<pre class="bg-deepVoid border border-abyssal/50 rounded-md p-3 overflow-x-auto my-3"><code class="text-sm font-mono text-ghost">${code.trim()}</code></pre>`;
    });

    // Inline code
    md = md.replace(/`([^`]+)`/g, '<code class="bg-deepVoid border border-abyssal/50 rounded px-1 py-0.5 text-xs font-mono text-accentGreen">$1</code>');

    // Headers
    md = md.replace(/^#### (.*$)/gim, '<h4 class="text-ghost font-semibold text-sm mt-4 mb-2">$1</h4>');
    md = md.replace(/^### (.*$)/gim, '<h3 class="text-ghost font-semibold text-base mt-5 mb-2">$1</h3>');
    md = md.replace(/^## (.*$)/gim, '<h2 class="text-ghost font-semibold text-lg mt-6 mb-3 border-b border-abyssal/50 pb-1">$1</h2>');
    md = md.replace(/^# (.*$)/gim, '<h1 class="text-ghost font-bold text-xl mt-8 mb-4 border-b border-abyssal/50 pb-2">$1</h1>');

    // Bold + Italic
    md = md.replace(/\*\*\*(.*?)\*\*\*/g, '<strong><em>$1</em></strong>');
    md = md.replace(/\*\*(.*?)\*\*/g, '<strong class="text-ghost">$1</strong>');
    md = md.replace(/\*(.*?)\*/g, '<em>$1</em>');
    md = md.replace(/__(.*?)__/g, '<strong class="text-ghost">$1</strong>');
    md = md.replace(/_(.*?)_/g, '<em>$1</em>');

    // Links
    md = md.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" class="text-accentGreen hover:underline" target="_blank" rel="noopener">$1</a>');

    // Blockquotes
    md = md.replace(/^&gt; (.*$)/gim, '<blockquote class="border-l-2 border-accentGreen/50 pl-3 my-3 text-charcoal italic">$1</blockquote>');

    // Lists
    md = md.replace(/^\s*[-*] (.*$)/gim, '<li class="ml-4 text-secondary">$1</li>');
    md = md.replace(/(<li.*<\/li>\n?)+/g, '<ul class="my-2 space-y-0.5">$&</ul>');
    md = md.replace(/^\s*\d+\. (.*$)/gim, '<li class="ml-4 text-secondary">$1</li>');
    md = md.replace(/(<li.*<\/li>\n?)+/g, (match) => {
      if (match.includes('<ul')) return match;
      return '<ol class="my-2 space-y-0.5 list-decimal">' + match + '</ol>';
    });

    // Horizontal rules
    md = md.replace(/^---$/gim, '<hr class="border-abyssal/50 my-4" />');

    // Paragraphs (wrap remaining lines)
    const lines = md.split('\n');
    let inPre = false;
    const processed = lines.map((line) => {
      if (line.startsWith('<pre')) inPre = true;
      if (line.startsWith('</pre')) { inPre = false; return line; }
      if (inPre) return line;
      if (line.trim() === '') return '<div class="h-2"></div>';
      if (line.startsWith('<')) return line;
      return `<p class="text-secondary leading-relaxed my-1">${line}</p>`;
    });

    const raw = processed.join('\n');
    return DOMPurify.sanitize(raw, {
      ALLOWED_TAGS,
      ALLOWED_ATTR,
      ALLOWED_URI_REGEXP,
    });
  }, [content]);

  // Render HTML, then walk text nodes and wrap matches in <mark>. Doing
  // the highlight after the DOM is built (instead of a regex replace on
  // the HTML string) prevents matches inside tag attributes and class
  // names — a search for "text" no longer breaks `class="text-ghost"`.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    container.innerHTML = html;
    if (!searchQuery || searchQuery.length < 2) {
      setMatchCount(0);
      setActiveMatch(0);
      return;
    }
    const count = highlightMatches(container, searchQuery);
    setMatchCount(count);
    setActiveMatch(count > 0 ? 0 : -1);
  }, [html, searchQuery]);

  // Update the active-match styling and scroll into view whenever the
  // user navigates prev/next. Kept separate from the re-render effect
  // above so nav doesn't rebuild the whole subtree.
  useEffect(() => {
    const container = containerRef.current;
    if (!container || matchCount === 0) return;
    const marks = container.querySelectorAll<HTMLElement>('mark[data-match-index]');
    marks.forEach((el, idx) => {
      el.className = idx === activeMatch ? HIGHLIGHT_ACTIVE_CLASS : HIGHLIGHT_CLASS;
    });
    const active = marks[activeMatch];
    if (active) {
      active.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }, [activeMatch, matchCount]);

  const gotoPrev = () => {
    if (matchCount === 0) return;
    setActiveMatch((i) => (i - 1 + matchCount) % matchCount);
  };
  const gotoNext = () => {
    if (matchCount === 0) return;
    setActiveMatch((i) => (i + 1) % matchCount);
  };

  return (
    <div className="relative h-full">
      {searchQuery && searchQuery.length >= 2 && (
        <div className="absolute top-2 right-4 z-10 flex items-center gap-1 bg-surface/95 border border-abyssal/60 rounded-md px-2 py-1 shadow-lg backdrop-blur-sm">
          <span className="text-charcoal text-[11px] font-mono tabular-nums min-w-[3rem] text-center">
            {matchCount === 0 ? '0 / 0' : `${activeMatch + 1} / ${matchCount}`}
          </span>
          <button
            type="button"
            onClick={gotoPrev}
            disabled={matchCount === 0}
            aria-label="Previous match"
            className="p-1 rounded hover:bg-abyssal/60 text-secondary hover:text-ghost disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
          >
            <CaretUp size={12} weight="bold" />
          </button>
          <button
            type="button"
            onClick={gotoNext}
            disabled={matchCount === 0}
            aria-label="Next match"
            className="p-1 rounded hover:bg-abyssal/60 text-secondary hover:text-ghost disabled:opacity-30 disabled:hover:bg-transparent transition-colors"
          >
            <CaretDown size={12} weight="bold" />
          </button>
        </div>
      )}
      <div
        ref={containerRef}
        className="h-full overflow-y-auto px-6 py-4 prose prose-invert prose-sm max-w-none"
      />
    </div>
  );
}

/**
 * Wraps every case-insensitive occurrence of `query` inside `root`'s
 * text nodes with a `<mark data-match-index="…">`. Skips text inside
 * <script>/<style> (impossible with our sanitized allowlist but cheap
 * to be defensive) and inside existing <mark> nodes (idempotent).
 * Returns the number of matches wrapped.
 */
export function highlightMatches(root: HTMLElement, query: string): number {
  const needle = query.toLowerCase();
  if (!needle) return 0;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (!parent) return NodeFilter.FILTER_REJECT;
      const tag = parent.tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'MARK') return NodeFilter.FILTER_REJECT;
      return node.nodeValue && node.nodeValue.toLowerCase().includes(needle)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT;
    },
  });
  const targets: Text[] = [];
  let current = walker.nextNode();
  while (current) {
    targets.push(current as Text);
    current = walker.nextNode();
  }
  let matchIndex = 0;
  for (const textNode of targets) {
    const value = textNode.nodeValue ?? '';
    const lower = value.toLowerCase();
    const frag = document.createDocumentFragment();
    let cursor = 0;
    let hit = lower.indexOf(needle, cursor);
    while (hit !== -1) {
      if (hit > cursor) {
        frag.appendChild(document.createTextNode(value.slice(cursor, hit)));
      }
      const mark = document.createElement('mark');
      mark.className = HIGHLIGHT_CLASS;
      mark.setAttribute('data-match-index', String(matchIndex));
      mark.textContent = value.slice(hit, hit + needle.length);
      frag.appendChild(mark);
      matchIndex += 1;
      cursor = hit + needle.length;
      hit = lower.indexOf(needle, cursor);
    }
    if (cursor < value.length) {
      frag.appendChild(document.createTextNode(value.slice(cursor)));
    }
    textNode.parentNode?.replaceChild(frag, textNode);
  }
  return matchIndex;
}
