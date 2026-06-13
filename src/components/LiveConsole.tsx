import { useState, useEffect, useRef } from 'react';
import { useCrawlEvents } from '../hooks/useCrawlEvents';
import {
  X,
  Minus,
  Terminal,
  Circle,
  Trash,
} from '@phosphor-icons/react';

interface LogEntry {
  id: number;
  timestamp: string;
  level: 'info' | 'success' | 'warning' | 'error';
  message: string;
  jobId?: string;
}

export function LiveConsole({ onClose }: { onClose: () => void }) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [minimized, setMinimized] = useState(false);
  const logEndRef = useRef<HTMLDivElement>(null);
  const logIdCounter = useRef(0);
  const { events } = useCrawlEvents();

  useEffect(() => {
    if (events.length === 0) return;
    const latest = events[events.length - 1];
    const level: LogEntry['level'] =
      latest.type === 'error'
        ? 'error'
        : latest.type === 'pageComplete'
          ? 'success'
          : 'info';

    const message =
      latest.type === 'progress'
        ? `Crawling ${latest.progress?.currentUrl || '...'} (depth ${latest.progress?.depth ?? 0}/${latest.progress?.maxDepth ?? 0})`
        : latest.type === 'pageComplete'
          ? `Completed: ${latest.page?.url} (${latest.page?.title || 'no title'})`
          : latest.type === 'error'
            ? `Error: ${latest.message || 'Unknown error'}`
            : latest.type === 'log'
              ? latest.message || ''
              : latest.type === 'jobStatusChanged'
                ? `Job status: ${latest.status || ''}`
                : 'Unknown event';

    const entry: LogEntry = {
      id: logIdCounter.current++,
      timestamp: new Date().toLocaleTimeString(),
      level,
      message,
      jobId: latest.jobId,
    };

    setLogs((prev) => [...prev, entry].slice(-500));
  }, [events]);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  const clearLogs = () => setLogs([]);

  if (minimized) {
    return (
      <div className="fixed bottom-4 right-4 z-50">
        <button
          onClick={() => setMinimized(false)}
          className="bg-deepVoid border border-abyssal/50 text-secondary hover:text-ghost px-4 py-2 rounded-md shadow-lg flex items-center space-x-2 transition-all duration-fast"
        >
          <Circle weight="fill" size={8} className="text-accentGreen animate-pulse" />
          <Terminal size={16} />
          <span className="text-xs font-medium">Live Console</span>
          {logs.length > 0 && (
            <span className="text-[10px] bg-accentGreen/15 text-accentGreen px-1.5 py-0.5 rounded">
              {logs.length}
            </span>
          )}
        </button>
      </div>
    );
  }

  return (
    <div className="fixed bottom-0 right-0 w-[600px] h-[400px] z-50 flex flex-col bg-deepVoid border border-abyssal/50 shadow-2xl rounded-tl-lg overflow-hidden">
      {/* Header */}
      <div className="h-10 flex items-center justify-between px-4 bg-surface/50 border-b border-abyssal/50">
        <div className="flex items-center space-x-2">
          <Terminal size={14} className="text-accentGreen" />
          <span className="text-xs font-semibold text-ghost uppercase tracking-wider">Live Console</span>
          {logs.length > 0 && (
            <span className="text-[10px] bg-accentGreen/10 text-accentGreen px-1.5 py-0.5 rounded">
              {logs.length} events
            </span>
          )}
        </div>
        <div className="flex items-center space-x-2">
          <button
            onClick={clearLogs}
            className="p-1 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
            title="Clear"
          >
            <Trash size={12} />
          </button>
          <button
            onClick={() => setMinimized(true)}
            className="p-1 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
          >
            <Minus size={12} />
          </button>
          <button
            onClick={onClose}
            className="p-1 text-charcoal hover:text-crimson hover:bg-crimson/10 rounded transition-colors"
          >
            <X size={12} />
          </button>
        </div>
      </div>

      {/* Log Content */}
      <div className="flex-1 overflow-y-auto p-3 font-mono text-[11px] space-y-1 bg-[#050a0f]">
        {logs.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-charcoal/30">
            <Terminal size={32} className="mb-2 opacity-30" />
            <p>Waiting for crawl events...</p>
          </div>
        ) : (
          logs.map((log) => (
            <div
              key={log.id}
              className={`flex items-start space-x-2 ${levelColor(log.level)}`}
            >
              <span className="text-charcoal/40 flex-shrink-0">[{log.timestamp}]</span>
              <span className="flex-shrink-0 font-semibold uppercase">{log.level}</span>
              {log.jobId && (
                <span className="text-charcoal/30 flex-shrink-0">[{log.jobId.slice(0, 8)}]</span>
              )}
              <span className="break-all">{log.message}</span>
            </div>
          ))
        )}
        <div ref={logEndRef} />
      </div>
    </div>
  );
}

const levelColor = (level: LogEntry['level']) => {
  switch (level) {
    case 'error':
      return 'text-crimson';
    case 'warning':
      return 'text-amber';
    case 'success':
      return 'text-brightGreen';
    default:
      return 'text-secondary';
  }
};
