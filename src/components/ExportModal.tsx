import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Download, FolderOpen, SpinnerGap } from '@phosphor-icons/react';
import { useToasts } from '../hooks/useToasts';
import { EXPORT_OPTIONS } from '../types';
import type { ExportFormat } from '../types';

interface ExportModalProps {
  jobId: string;
  onClose: () => void;
}

export function ExportModal({ jobId, onClose }: ExportModalProps) {
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>('md_files');
  const [destination, setDestination] = useState('');
  const [headlessSupported, setHeadlessSupported] = useState(false);
  const [exporting, setExporting] = useState(false);
  const { pushToast } = useToasts();

  useEffect(() => {
    invoke<boolean>('check_headless_support').then(setHeadlessSupported).catch(() => {});
  }, []);

  const handlePickDestination = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select Export Destination',
      });
      if (selected) {
        setDestination(selected);
      }
    } catch (err) {
      console.error('Failed to open directory picker', err);
    }
  };

  const handleExport = async () => {
    if (!destination) return;
    setExporting(true);
    try {
      await invoke('export_job_v2', {
        jobId,
        format: selectedFormat,
        destination,
      });
      pushToast('success', `Export completed: ${selectedFormat}`);
      onClose();
    } catch (err) {
      pushToast('error', `Export failed: ${err}`);
    } finally {
      setExporting(false);
    }
  };

  const isDisabled = (requiresHeadless: boolean) => requiresHeadless && !headlessSupported;

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
        className="fixed inset-0 m-auto w-[440px] h-fit bg-deepVoid border border-abyssal/50 rounded-xl z-50 shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-abyssal/50">
          <h2 className="text-ghost font-semibold text-base">Export Job</h2>
          <button
            onClick={onClose}
            className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 space-y-4">
          {/* Format selection */}
          <div className="space-y-2">
            <label className="text-xs text-charcoal uppercase tracking-wider">Format</label>
            <div className="grid grid-cols-2 gap-2">
              {EXPORT_OPTIONS.map((opt) => {
                const disabled = isDisabled(opt.requiresHeadless);
                return (
                  <button
                    key={opt.format}
                    onClick={() => !disabled && setSelectedFormat(opt.format)}
                    disabled={disabled}
                    className={`p-3 rounded-lg border text-left transition-all ${
                      selectedFormat === opt.format
                        ? 'border-accentGreen/60 bg-accentGreen/10'
                        : disabled
                        ? 'border-abyssal/30 bg-surface/20 opacity-40 cursor-not-allowed'
                        : 'border-abyssal/50 bg-surface/30 hover:border-abyssal hover:bg-surface/50'
                    }`}
                  >
                    <p className={`text-sm font-medium ${selectedFormat === opt.format ? 'text-accentGreen' : 'text-ghost'}`}>
                      {opt.label}
                    </p>
                    <p className="text-[10px] text-charcoal mt-0.5">{opt.description}</p>
                    {disabled && (
                      <p className="text-[10px] text-crimson mt-1">Requires headless Chrome</p>
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Destination picker */}
          <div className="space-y-2">
            <label className="text-xs text-charcoal uppercase tracking-wider">Destination</label>
            <div className="flex items-center space-x-2">
              <div className="flex-1 bg-surface/30 border border-abyssal/50 rounded-md px-3 py-2 text-sm text-ghost truncate min-h-[36px]">
                {destination || 'No folder selected'}
              </div>
              <button
                onClick={handlePickDestination}
                className="p-2 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                title="Pick folder"
              >
                <FolderOpen size={18} />
              </button>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end space-x-3 px-5 py-4 border-t border-abyssal/50">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-charcoal hover:text-ghost transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleExport}
            disabled={!destination || exporting}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-accentGreen/20 text-accentGreen border border-accentGreen/30 rounded-md hover:bg-accentGreen/30 transition-all disabled:opacity-40 disabled:cursor-not-allowed"
          >
            {exporting ? (
              <>
                <SpinnerGap size={14} className="animate-spin" />
                Exporting...
              </>
            ) : (
              <>
                <Download size={14} />
                Export
              </>
            )}
          </button>
        </div>
      </motion.div>
    </AnimatePresence>,
    document.body
  );
}
