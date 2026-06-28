import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { motion } from 'framer-motion';
import { FileArrowUp, SpinnerGap, CheckCircle, WarningCircle, File as FileIcon, Image } from '@phosphor-icons/react';
import { useToasts } from '../hooks/useToasts';
import type { ImportResult } from '../types';

const SUPPORTED_EXTENSIONS = ['pdf', 'epub'];

function getExtension(path: string): string {
  return path.split('.').pop()?.toLowerCase() ?? '';
}

export function ImportView() {
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [cleanText, setCleanText] = useState(true);
  const { pushToast } = useToasts();

  const handleImport = async (filePath?: string) => {
    let selectedPath = filePath;

    if (!selectedPath) {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Documents', extensions: ['pdf', 'epub'] },
          { name: 'PDF Files', extensions: ['pdf'] },
          { name: 'EPUB Files', extensions: ['epub'] },
        ],
      });

      if (!selected) return;
      selectedPath = selected as string;
    }

    setImporting(true);
    setResult(null);
    setError(null);

    try {
      const res = await invoke<ImportResult>('import_file', {
        filePath: selectedPath,
        outputDir: null,
        cleanText,
      });
      setResult(res);
      pushToast('success', `Imported "${res.title}" — ${res.pageCount} pages, ${res.imageCount} images`);
    } catch (err) {
      const msg = `${err}`;
      setError(msg);
      pushToast('error', `Import failed: ${msg}`);
    } finally {
      setImporting(false);
    }
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    getCurrentWebview().onDragDropEvent((event) => {
      const { type } = event.payload;

      if (type === 'enter' || type === 'over') {
        setDragOver(true);
      } else if (type === 'leave') {
        setDragOver(false);
      } else if (type === 'drop') {
        setDragOver(false);
        const paths = event.payload.paths;
        const validFile = paths.find((p) => SUPPORTED_EXTENSIONS.includes(getExtension(p)));
        if (validFile) {
          handleImport(validFile);
        } else if (paths.length > 0) {
          pushToast('error', 'Only .pdf and .epub files are supported');
        }
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <div className="max-w-2xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-2xl font-bold text-ghost mb-2">Import Document</h1>
          <p className="text-sm text-charcoal">
            Convert PDF or EPUB files to Markdown with automatic image extraction.
          </p>
        </div>

        {/* Drop zone */}
        <motion.div
          onClick={() => !importing && handleImport()}
          className={`
            relative cursor-pointer rounded-xl border-2 border-dashed p-12
            flex flex-col items-center justify-center text-center
            transition-all duration-200
            ${dragOver
              ? 'border-accentGreen bg-accentGreen/5 scale-[1.01]'
              : 'border-abyssal/50 hover:border-abyssal hover:bg-surface/20'
            }
            ${importing ? 'pointer-events-none opacity-60' : ''}
          `}
          whileHover={{ scale: importing ? 1 : 1.005 }}
          whileTap={{ scale: importing ? 1 : 0.995 }}
        >
          {importing ? (
            <SpinnerGap size={48} className="text-accentGreen animate-spin mb-4" />
          ) : (
            <FileArrowUp size={48} className={`mb-4 ${dragOver ? 'text-accentGreen' : 'text-charcoal'}`} />
          )}
          <p className="text-ghost font-medium text-lg mb-1">
            {importing ? 'Converting...' : dragOver ? 'Drop to import' : 'Drop a file here or click to browse'}
          </p>
          <p className="text-charcoal text-sm">
            Supports <span className="text-secondary">.pdf</span> and <span className="text-secondary">.epub</span> files
          </p>
        </motion.div>

        {/* Clean text toggle */}
        <label className="mt-4 flex items-center gap-3 cursor-pointer select-none">
          <div
            onClick={() => setCleanText(!cleanText)}
            className={`
              relative w-9 h-5 rounded-full transition-colors duration-200
              ${cleanText ? 'bg-accentGreen' : 'bg-abyssal'}
            `}
          >
            <div
              className={`
                absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-ghost transition-transform duration-200
                ${cleanText ? 'translate-x-4' : 'translate-x-0'}
              `}
            />
          </div>
          <div>
            <span className="text-sm text-ghost">Clean text</span>
            <span className="text-xs text-charcoal ml-2">Remove headers, footers, page numbers, footnotes</span>
          </div>
        </label>

        {/* Result */}
        {result && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-6 p-5 rounded-xl border border-accentGreen/30 bg-accentGreen/5"
          >
            <div className="flex items-start gap-3">
              <CheckCircle size={24} weight="fill" className="text-accentGreen mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <h3 className="text-ghost font-semibold text-base mb-3">{result.title}</h3>
                <div className="grid grid-cols-2 gap-3">
                  <div className="flex items-center gap-2 text-sm text-secondary">
                    <FileIcon size={16} className="text-charcoal" />
                    <span>{result.pageCount} {result.pageCount === 1 ? 'page' : 'pages'}</span>
                  </div>
                  <div className="flex items-center gap-2 text-sm text-secondary">
                    <Image size={16} className="text-charcoal" />
                    <span>{result.imageCount} {result.imageCount === 1 ? 'image' : 'images'} extracted</span>
                  </div>
                </div>
                <div className="mt-3 p-2 bg-surface/30 rounded text-xs text-charcoal font-mono truncate">
                  {result.markdownPath}
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    const dir = result.markdownPath.substring(0, result.markdownPath.lastIndexOf('/') + 1) ||
                                result.markdownPath.substring(0, result.markdownPath.lastIndexOf('\\') + 1);
                    invoke('open_output_folder', { path: dir });
                  }}
                  className="mt-3 px-3 py-1.5 text-xs bg-accentGreen/15 text-accentGreen rounded hover:bg-accentGreen/25 transition-colors"
                >
                  Open output folder
                </button>
              </div>
            </div>
          </motion.div>
        )}

        {/* Error */}
        {error && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-6 p-5 rounded-xl border border-crimson/30 bg-crimson/5"
          >
            <div className="flex items-start gap-3">
              <WarningCircle size={24} weight="fill" className="text-crimson mt-0.5 flex-shrink-0" />
              <div>
                <h3 className="text-ghost font-semibold mb-1">Import Failed</h3>
                <p className="text-sm text-secondary">{error}</p>
              </div>
            </div>
          </motion.div>
        )}

        {/* Info */}
        <div className="mt-8 p-4 rounded-lg bg-surface/20 border border-abyssal/30">
          <h4 className="text-secondary text-sm font-medium mb-2">How it works</h4>
          <ul className="space-y-1.5 text-xs text-charcoal">
            <li><span className="text-secondary">PDF:</span> Extracts text page-by-page and embedded images (JPEG, PNG)</li>
            <li><span className="text-secondary">EPUB:</span> Converts chapters from HTML to Markdown, extracts all images</li>
            <li>Images are saved to an <code className="text-secondary">images/</code> subfolder and linked in the Markdown</li>
            <li>Output is saved to your configured output directory</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
