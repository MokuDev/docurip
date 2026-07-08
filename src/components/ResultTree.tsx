import { FileText, CaretRight, CaretDown } from '@phosphor-icons/react';
import { useState, useMemo, useRef, useEffect } from 'react';
import { List } from 'react-window';
import type { PageMeta } from '../types';

interface TreeNode {
  name: string;
  path: string;
  page?: PageMeta;
  children: TreeNode[];
}

function buildTree(pages: PageMeta[]): TreeNode[] {
  const root: TreeNode[] = [];

  for (const page of pages) {
    try {
      const url = new URL(page.url);
      const host = url.hostname;
      const pathSegments = url.pathname.split('/').filter(Boolean);

      let hostNode = root.find((n) => n.name === host);
      if (!hostNode) {
        hostNode = { name: host, path: host, children: [] };
        root.push(hostNode);
      }

      let current = hostNode;
      for (let i = 0; i < pathSegments.length; i++) {
        const seg = pathSegments[i];
        const isLast = i === pathSegments.length - 1;
        const fullPath = current.path + '/' + seg;

        let child = current.children.find((c) => c.name === seg);
        if (!child) {
          child = { name: seg, path: fullPath, children: [] };
          if (isLast) {
            child.page = page;
          }
          current.children.push(child);
        } else if (isLast) {
          child.page = page;
        }
        current = child;
      }
    } catch {
      root.push({ name: page.url, path: page.url, page, children: [] });
    }
  }

  return root;
}

interface FlatNode {
  node: TreeNode;
  depth: number;
}

interface ResultTreeProps {
  pages: PageMeta[];
  selectedUrl: string;
  onSelect: (page: PageMeta) => void;
  filterQuery?: string;
}

const ROW_HEIGHT = 32;

export function ResultTree({ pages, selectedUrl, onSelect, filterQuery }: ResultTreeProps) {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [focusedIndex, setFocusedIndex] = useState(-1);
  const listRef = useRef<any>(null);

  const filtered = useMemo(() => {
    if (!filterQuery) return pages;
    return pages.filter(
      (p) =>
        p.title.toLowerCase().includes(filterQuery.toLowerCase()) ||
        p.url.toLowerCase().includes(filterQuery.toLowerCase())
    );
  }, [pages, filterQuery]);

  const tree = useMemo(() => buildTree(filtered), [filtered]);

  const toggleExpanded = (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const visibleNodes = useMemo(() => {
    const result: FlatNode[] = [];
    function walk(nodes: TreeNode[], depth: number) {
      for (const node of nodes) {
        result.push({ node, depth });
        const isExpanded = expandedPaths.size === 0 || expandedPaths.has(node.path);
        if (isExpanded && node.children.length > 0) {
          walk(node.children, depth + 1);
        }
      }
    }
    walk(tree, 0);
    return result;
  }, [tree, expandedPaths]);

  useEffect(() => {
    if (focusedIndex >= 0 && listRef.current) {
      listRef.current.scrollToItem(focusedIndex);
    }
  }, [focusedIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setFocusedIndex((i) => Math.min(i + 1, visibleNodes.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setFocusedIndex((i) => Math.max(i - 1, 0));
        break;
      case 'ArrowRight': {
        const node = visibleNodes[focusedIndex]?.node;
        if (node?.children.length > 0) {
          setExpandedPaths((prev) => new Set([...prev, node.path]));
        }
        break;
      }
      case 'ArrowLeft': {
        const node = visibleNodes[focusedIndex]?.node;
        if (node) {
          setExpandedPaths((prev) => {
            const next = new Set(prev);
            next.delete(node.path);
            return next;
          });
        }
        break;
      }
      case 'Enter': {
        const node = visibleNodes[focusedIndex]?.node;
        if (node?.page) onSelect(node.page);
        break;
      }
    }
  };

  if (filtered.length === 0) {
    return <p className="text-charcoal text-xs px-3 py-4 text-center">No results found</p>;
  }

  return (
    <div tabIndex={0} onKeyDown={handleKeyDown} className="outline-none h-full">
      <List
        ref={listRef}
        rowCount={visibleNodes.length}
        rowHeight={ROW_HEIGHT}
        rowProps={{}}
        rowComponent={({ index, style }) => {
          const { node, depth } = visibleNodes[index];
          const isSelected = node.page?.url === selectedUrl;
          const isFocused = index === focusedIndex;
          const hasChildren = node.children.length > 0;
          const isExpanded = expandedPaths.size === 0 || expandedPaths.has(node.path);

          return (
            <div style={style}>
              <button
                onClick={() => {
                  setFocusedIndex(index);
                  if (node.page) onSelect(node.page);
                  if (hasChildren) toggleExpanded(node.path);
                }}
                className={`w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-md transition-all ${
                  isSelected
                    ? 'bg-accentGreen/10 text-accentGreen'
                    : isFocused
                      ? 'ring-1 ring-accentGreen/40 text-ghost bg-surface/30'
                      : 'text-secondary hover:text-ghost hover:bg-surface/40'
                }`}
                style={{ paddingLeft: `${8 + depth * 16}px` }}
              >
                {hasChildren ? (
                  isExpanded ? <CaretDown size={14} className="text-charcoal" /> : <CaretRight size={14} className="text-charcoal" />
                ) : (
                  <FileText size={14} className="text-charcoal" />
                )}
                <span className="truncate">{node.name}</span>
                {node.page && (
                  <span className="ml-auto text-[10px] text-charcoal font-mono">{node.page.status}</span>
                )}
              </button>
            </div>
          );
        }}
      />
    </div>
  );
}
