import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ResultTree } from './ResultTree';
import type { PageMeta } from '../types';

// react-window virtualizes based on measured container size, which jsdom
// (no ResizeObserver/layout) can't provide. Replace it with a version that
// renders every row unconditionally so these tests exercise the component's
// own expand/collapse and focus logic rather than fighting virtualization.
vi.mock('react-window', () => ({
  useListRef: () => ({ current: null }),
  List: ({ rowCount, rowComponent: RowComponent, rowProps }: any) => (
    <>
      {Array.from({ length: rowCount }, (_, index) => (
        <RowComponent key={index} index={index} style={{}} {...rowProps} />
      ))}
    </>
  ),
}));

// Tree node names come from URL path segments, not page titles.
const pages: PageMeta[] = [
  { url: 'https://docs.example.com/guide/intro', title: 'Intro', status: 200, linksCount: 3 },
  { url: 'https://docs.example.com/guide/setup', title: 'Setup', status: 200, linksCount: 5 },
  { url: 'https://docs.example.com/api/reference', title: 'Reference', status: 200, linksCount: 2 },
];

describe('ResultTree', () => {
  it('renders the fully expanded tree by default', () => {
    render(<ResultTree pages={pages} selectedUrl="" onSelect={vi.fn()} />);
    // host + guide + intro + setup + api + reference = 6 rows
    expect(screen.getByText('docs.example.com')).toBeInTheDocument();
    expect(screen.getByText('guide')).toBeInTheDocument();
    expect(screen.getByText('intro')).toBeInTheDocument();
    expect(screen.getByText('setup')).toBeInTheDocument();
    expect(screen.getByText('api')).toBeInTheDocument();
    expect(screen.getByText('reference')).toBeInTheDocument();
  });

  it('collapsing a nested folder only hides its own descendants, not sibling branches', () => {
    render(<ResultTree pages={pages} selectedUrl="" onSelect={vi.fn()} />);

    fireEvent.click(screen.getByText('guide'));

    // guide's children disappear...
    expect(screen.queryByText('intro')).not.toBeInTheDocument();
    expect(screen.queryByText('setup')).not.toBeInTheDocument();
    // ...but everything else stays visible (this was the bug: the whole
    // tree used to collapse down to just the root on any non-root click).
    expect(screen.getByText('docs.example.com')).toBeInTheDocument();
    expect(screen.getByText('guide')).toBeInTheDocument();
    expect(screen.getByText('api')).toBeInTheDocument();
    expect(screen.getByText('reference')).toBeInTheDocument();
  });

  it('re-expanding a collapsed folder restores its children', () => {
    render(<ResultTree pages={pages} selectedUrl="" onSelect={vi.fn()} />);

    fireEvent.click(screen.getByText('guide'));
    expect(screen.queryByText('intro')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('guide'));
    expect(screen.getByText('intro')).toBeInTheDocument();
    expect(screen.getByText('setup')).toBeInTheDocument();
  });

  it('does not throw when collapsing a folder while a descendant row is focused', () => {
    const onSelect = vi.fn();
    render(<ResultTree pages={pages} selectedUrl="" onSelect={onSelect} />);

    // Focus a deep row via keyboard navigation, then collapse an ancestor
    // via mouse click — this is the exact sequence that used to throw an
    // uncaught RangeError from react-window's scrollToRow and blank the
    // whole app (no ErrorBoundary anywhere catches it).
    const tree = screen.getByText('docs.example.com').closest('[tabindex]') as HTMLElement;
    for (let i = 0; i < 5; i++) {
      fireEvent.keyDown(tree, { key: 'ArrowDown' });
    }

    expect(() => {
      fireEvent.click(screen.getByText('guide'));
    }).not.toThrow();
  });
});
