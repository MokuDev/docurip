import { AnimatePresence, motion } from 'framer-motion';
import { X, WarningCircle, Info, CheckCircle } from '@phosphor-icons/react';
import { useToasts, type Toast } from '../hooks/useToasts';

const TYPE_STYLES: Record<Toast['type'], { border: string; bg: string; text: string; Icon: typeof WarningCircle }> = {
  error: {
    border: 'border-crimson',
    bg: 'bg-crimson/10',
    text: 'text-crimson',
    Icon: WarningCircle,
  },
  info: {
    border: 'border-cyberBlue',
    bg: 'bg-cyberBlue/10',
    text: 'text-cyberBlue',
    Icon: Info,
  },
  success: {
    border: 'border-accentGreen',
    bg: 'bg-accentGreen/10',
    text: 'text-accentGreen',
    Icon: CheckCircle,
  },
};

export function ToastContainer() {
  const { toasts, dismissToast } = useToasts();
  const visible = toasts.slice(-3);

  return (
    <div className="fixed bottom-12 left-4 z-50 flex flex-col gap-2 w-[360px] max-w-[calc(100vw-2rem)] pointer-events-none">
      <AnimatePresence initial={false}>
        {visible.map((toast) => {
          const style = TYPE_STYLES[toast.type];
          const Icon = style.Icon;
          return (
            <motion.div
              key={toast.id}
              layout
              initial={{ opacity: 0, x: -24, scale: 0.95 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: -24, scale: 0.95 }}
              transition={{ duration: 0.2 }}
              className={`pointer-events-auto bg-surface/95 backdrop-blur-sm border-l-4 ${style.border} ${style.bg} rounded shadow-lg flex items-start px-3 py-2.5 gap-2.5`}
            >
              <Icon weight="fill" size={18} className={`${style.text} flex-shrink-0 mt-0.5`} />
              <p className="flex-1 text-xs font-mono text-ghost break-words leading-snug">
                {toast.message}
              </p>
              <button
                onClick={() => dismissToast(toast.id)}
                className={`${style.text} hover:text-ghost transition-colors flex-shrink-0 -mr-1 -mt-0.5 p-0.5`}
                aria-label="Dismiss"
              >
                <X size={14} weight="bold" />
              </button>
            </motion.div>
          );
        })}
      </AnimatePresence>
    </div>
  );
}
