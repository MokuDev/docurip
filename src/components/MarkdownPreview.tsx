import { useMemo } from 'react';

interface MarkdownPreviewProps {
  content: string;
  searchQuery?: string;
}

export function MarkdownPreview({ content, searchQuery }: MarkdownPreviewProps) {
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

    return processed.join('\n');
  }, [content]);

  const highlightedHtml = useMemo(() => {
    if (!searchQuery || searchQuery.length < 2) return html;
    const q = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${q})`, 'gi');
    return html.replace(regex, '<mark class="bg-accentGreen/30 text-accentGreen rounded px-0.5">$1</mark>');
  }, [html, searchQuery]);

  return (
    <div
      className="h-full overflow-y-auto px-6 py-4 prose prose-invert prose-sm max-w-none"
      dangerouslySetInnerHTML={{ __html: highlightedHtml }}
    />
  );
}
