import {
  CheckCircle,
  Play,
  Warning,
  Globe,
  Clock,
  FileText,
  Prohibit,
} from '@phosphor-icons/react';

const badgeStyles: Record<string, string> = {
  queued: 'bg-amber/10 text-amber',
  running: 'bg-accentGreen/10 text-accentGreen',
  paused: 'bg-cyberBlue/10 text-cyberBlue',
  completed: 'bg-brightGreen/10 text-brightGreen',
  failed: 'bg-crimson/10 text-crimson',
  cancelled: 'bg-charcoal/20 text-charcoal',
};

export function StatusBadge({ status }: { status: string }) {
  return (
    <span
      className={`text-[10px] font-semibold uppercase tracking-wider px-2 py-1 rounded ${badgeStyles[status] || 'text-charcoal'}`}
    >
      {status}
    </span>
  );
}

export function StatusIcon({ status, size = 16 }: { status: string; size?: number }) {
  switch (status) {
    case 'completed':
      return <CheckCircle weight="fill" size={size} className="text-brightGreen" />;
    case 'running':
      return <Play weight="fill" size={size} className="text-accentGreen" />;
    case 'failed':
      return <Warning weight="fill" size={size} className="text-crimson" />;
    case 'queued':
      return <Clock size={size} className="text-amber" />;
    case 'paused':
      return <FileText size={size} className="text-cyberBlue" />;
    case 'cancelled':
      return <Prohibit size={size} className="text-charcoal" />;
    default:
      return <Globe size={size} className="text-charcoal" />;
  }
}
