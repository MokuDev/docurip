import { useState, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { motion } from 'framer-motion';
import {
  X,
  DownloadSimple,
  FileArrowUp,
  FolderOpen,
  FileText,
} from '@phosphor-icons/react';
import type { CrawlJob, PageResult } from '../types';
import { ResultTree } from '../components/ResultTree';
import { MarkdownPreview } from '../components/MarkdownPreview';
import { ResultSearch } from '../components/ResultSearch';
import { EmptyState } from '../components/EmptyState';

interface ResultBrowserProps {
  job: CrawlJob;
  onClose: () => void;
}

export function ResultBrowser({ job, onClose }: ResultBrowserProps) {
  const [selectedPage, setSelectedPage] = useState<PageResult | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [exporting, setExporting] = useState(false);
  const [exportPath, setExportPath] = useState('');

  const pages = job.results;

  const filteredPages = useMemo(() => {
    if (!searchQuery) return pages;
    const q = searchQuery.toLowerCase();
    return pages.filter(
      (p) =>
        p.title.toLowerCase().includes(q) ||
        p.url.toLowerCase().includes(q) ||
        p.content.toLowerCase().includes(q)
    );
  }, [pages, searchQuery]);

  const handleExport = useCallback(async () => {
    setExporting(true);
    try {
      const path: string = await invoke('export_job_zip', { jobId: job.id });
      setExportPath(path);
    } catch (err) {
      console.error('Export failed', err);
    } finally {
      setExporting(false);
    }
  }, [job.id]);

  const handleOpenFolder = useCallback(async () => {
    try {
      await invoke('open_output_folder', { path: job.config.outputDir });
    } catch (err) {
      console.error('Open folder failed', err);
    }
  }, [job.config.outputDir]);

  return (
    <div className="fixed inset-0 z-50 flex">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />

      {/* Panel */}
      <motion.div
        initial={{ x: '100%' }}
        animate={{ x: 0 }}
        exit={{ x: '100%' }}
        transition={{ type: 'spring', damping: 25, stiffness: 200 }}
        className="relative ml-auto w-full max-w-5xl h-full bg-deepVoid border-l border-abyssal/50 flex flex-col"
      >
        {/* Header */}
        <div className="h-14 flex items-center justify-between px-4 border-b border-abyssal/50 bg-surface/30">
          <div className="flex items-center gap-3 min-w-0">
            <FileArrowUp size={18} className="text-accentGreen" />
            <div className="min-w-0">
              <h2 className="text-ghost font-semibold text-sm truncate">{job.url}</h2>
              <p className="text-charcoal text-xs">{pages.length} pages</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs text-secondary hover:text-ghost hover:bg-surface/60 transition-all"
            >
              <FolderOpen size={14} />
              Open Folder
            </button>
            <button
              onClick={handleExport}
              disabled={exporting}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all disabled:opacity-50"
            >
              <DownloadSimple size={14} />
              {exporting ? 'Exporting...' : 'Export ZIP'}
            </button>
            {exportPath && (
              <span className="text-charcoal text-xs max-w-[200px] truncate" title={exportPath}>
                {exportPath}
              </span>
            )}
            <button
              onClick={onClose}
              className="p-1.5 rounded-md hover:bg-surface/60 text-charcoal hover:text-ghost transition-all"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Toolbar */}
        <div className="px-4 py-2 border-b border-abyssal/50">
          <ResultSearch
            value={searchQuery}
            onChange={setSearchQuery}
            resultCount={filteredPages.length}
          />
        </div>

        {/* Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Tree */}
          <div className="w-80 flex-shrink-0 border-r border-abyssal/50 bg-surface/20">
            {pages.length > 0 ? (
              <ResultTree
                pages={filteredPages}
                selectedUrl={selectedPage?.url || ''}
                onSelect={setSelectedPage}
                filterQuery={searchQuery}
              />
            ) : (
              <EmptyState
                icon={<FileText size={40} />}
                title="No pages"
                description="This crawl produced no results."
              />
            )}
          </div>

          {/* Preview */}
          <div className="flex-1 bg-[#050a0f]">
            {selectedPage ? (
              <MarkdownPreview
                content={selectedPage.content}
                searchQuery={searchQuery}
              />
            ) : (
              <EmptyState
                icon={<FileText size={48} />}
                title="Select a page"
                description="Click a page in the tree to preview its content."
              />
            )}
          </div>
        </div>
      </motion.div>
    </div>
  );
}
