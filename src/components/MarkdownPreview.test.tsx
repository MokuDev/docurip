import { describe, it, expect } from 'vitest';
import { highlightMatches } from './MarkdownPreview';

function makeRoot(html: string): HTMLElement {
  const root = document.createElement('div');
  root.innerHTML = html;
  return root;
}

describe('highlightMatches', () => {
  it('wraps text matches in <mark> and returns the count', () => {
    const root = makeRoot('<p>Hello world, hello again</p>');
    const count = highlightMatches(root, 'hello');
    expect(count).toBe(2);
    const marks = root.querySelectorAll('mark');
    expect(marks).toHaveLength(2);
    expect(marks[0].textContent).toBe('Hello');
    expect(marks[1].textContent).toBe('hello');
    expect(marks[0].getAttribute('data-match-index')).toBe('0');
    expect(marks[1].getAttribute('data-match-index')).toBe('1');
  });

  it('does not match inside tag attributes / class names', () => {
    const root = makeRoot('<p class="text-ghost">nothing here</p>');
    const count = highlightMatches(root, 'text');
    expect(count).toBe(0);
    expect(root.querySelectorAll('mark')).toHaveLength(0);
  });

  it('is case-insensitive and preserves original casing in the mark', () => {
    const root = makeRoot('<p>API and api and Api</p>');
    const count = highlightMatches(root, 'api');
    expect(count).toBe(3);
    const texts = Array.from(root.querySelectorAll('mark')).map((m) => m.textContent);
    expect(texts).toEqual(['API', 'api', 'Api']);
  });

  it('returns 0 for an empty query', () => {
    const root = makeRoot('<p>anything</p>');
    expect(highlightMatches(root, '')).toBe(0);
  });

  it('skips existing <mark> nodes so a re-run is idempotent', () => {
    const root = makeRoot('<p>foo <mark>foo</mark> foo</p>');
    const count = highlightMatches(root, 'foo');
    expect(count).toBe(2);
  });

  it('handles matches spread across multiple text nodes', () => {
    const root = makeRoot('<p>foo <strong>foo</strong> foo</p>');
    const count = highlightMatches(root, 'foo');
    expect(count).toBe(3);
  });
});
