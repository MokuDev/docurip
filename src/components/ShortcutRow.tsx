import { ArrowClockwise } from '@phosphor-icons/react';
import { formatCombo, normalizeCombo, type ShortcutAction } from '../hooks/useKeyboardShortcuts';

interface ShortcutRowProps {
  action: ShortcutAction;
  combo: string;
  isCustom: boolean;
  isEditing: boolean;
  conflictError?: string;
  onStartEdit: () => void;
  onCapture: (combo: string) => void;
  onCancelEdit: () => void;
  onReset: () => void;
}

export function ShortcutRow({
  action,
  combo,
  isCustom,
  isEditing,
  conflictError,
  onStartEdit,
  onCapture,
  onCancelEdit,
  onReset,
}: ShortcutRowProps) {
  return (
    <div className="flex items-center justify-between py-2">
      <div className="min-w-0">
        <span className="block text-sm text-ghost">{action.label}</span>
        {isEditing && (
          <span className="block text-xs text-accentGreen mt-0.5">
            {conflictError || 'Press a key combination… (Esc to cancel)'}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2 flex-shrink-0">
        <button
          type="button"
          onClick={(e) => {
            onStartEdit();
            e.currentTarget.focus();
          }}
          onKeyDown={(e) => {
            if (!isEditing) return;
            e.preventDefault();
            e.stopPropagation();
            if (e.key === 'Escape') {
              onCancelEdit();
              e.currentTarget.blur();
              return;
            }
            const captured = normalizeCombo(e);
            if (!captured) return;
            onCapture(captured);
          }}
          onBlur={() => {
            if (isEditing) onCancelEdit();
          }}
          className={`min-w-[92px] text-center px-3 py-1.5 rounded-md border text-xs font-mono transition-all ${
            isEditing
              ? 'border-accentGreen text-accentGreen bg-accentGreen/10'
              : 'border-abyssal text-ghost bg-surface/50 hover:border-accentGreen/50'
          }`}
        >
          {isEditing ? 'Press keys…' : formatCombo(combo)}
        </button>
        {isCustom && !isEditing && (
          <button
            type="button"
            onClick={onReset}
            title="Reset to default"
            className="text-charcoal hover:text-ghost transition-colors p-1"
          >
            <ArrowClockwise size={14} />
          </button>
        )}
      </div>
    </div>
  );
}
