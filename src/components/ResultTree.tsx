import { FileText, CaretRight, CaretDown } from '@phosphor-icons/react';
import { useState } from 'react';
import type { PageResult } from '../types';

interface TreeNode {
  name: string;
  path: string;
  page?: PageResult;
  children: TreeNode[];
}

function buildTree(pages: PageResult[]): TreeNode[] {
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

function TreeNodeView({
  node,
  selectedUrl,
  onSelect,
  depth = 0,
}: {
  node: TreeNode;
  selectedUrl: string;
  onSelect: (page: PageResult) => void;
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const isSelected = node.page?.url === selectedUrl;
  const hasChildren = node.children.length > 0;

  return (
    <div>
      <button
        onClick={() => {
          if (node.page) onSelect(node.page);
          if (hasChildren) setExpanded(!expanded);
        }}
        className={`w-full flex items-center gap-2 px-2 py-1.5 text-sm rounded-md transition-all ${
          isSelected
            ? 'bg-accentGreen/10 text-accentGreen'
            : 'text-secondary hover:text-ghost hover:bg-surface/40'
        }`}
        style={{ paddingLeft: `${8 + depth * 16}px` }}
      >
        {hasChildren ? (
          expanded ? <CaretDown size={14} className="text-charcoal" /> : <CaretRight size={14} className="text-charcoal" />
        ) : (
          <FileText size={14} className="text-charcoal" />
        )}
        <span className="truncate">{node.name}</span>
        {node.page && (
          <span className="ml-auto text-[10px] text-charcoal font-mono">{node.page.status}</span>
        )}
      </button>
      {expanded && node.children.map((child) => (
        <TreeNodeView
          key={child.path}
          node={child}
          selectedUrl={selectedUrl}
          onSelect={onSelect}
          depth={depth + 1}
        />
      ))}
    </div>
  );
}

interface ResultTreeProps {
  pages: PageResult[];
  selectedUrl: string;
  onSelect: (page: PageResult) => void;
  filterQuery?: string;
}

export function ResultTree({ pages, selectedUrl, onSelect, filterQuery }: ResultTreeProps) {
  const filtered = filterQuery
    ? pages.filter(
        (p) =>
          p.title.toLowerCase().includes(filterQuery.toLowerCase()) ||
          p.url.toLowerCase().includes(filterQuery.toLowerCase())
      )
    : pages;

  const tree = buildTree(filtered);

  return (
    <div className="overflow-y-auto h-full">
      {tree.map((node) => (
        <TreeNodeView key={node.path} node={node} selectedUrl={selectedUrl} onSelect={onSelect} />
      ))}
      {filtered.length === 0 && (
        <p className="text-charcoal text-xs px-3 py-4 text-center">No results found</p>
      )}
    </div>
  );
}
