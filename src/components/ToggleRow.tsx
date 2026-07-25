import type { Icon } from '@phosphor-icons/react';

interface ToggleRowProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  description?: string;
  IconOn: Icon;
  IconOff?: Icon;
}

export function ToggleRow({ checked, onChange, label, description, IconOn, IconOff }: ToggleRowProps) {
  const Icon = checked || !IconOff ? IconOn : IconOff;
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className="w-full flex items-center justify-between px-1 py-1 group"
    >
      <div className="flex items-center gap-3">
        <Icon
          size={18}
          weight={checked ? 'fill' : 'regular'}
          className={checked ? 'text-accentGreen' : 'text-charcoal'}
        />
        <div className="text-left">
          <span className="block text-sm text-ghost">{label}</span>
          {description && (
            <span className="block text-xs text-charcoal mt-0.5">{description}</span>
          )}
        </div>
      </div>
      <div
        className={`relative w-10 h-[22px] rounded-full transition-colors ${
          checked ? 'bg-accentGreen' : 'bg-abyssal'
        }`}
      >
        <div
          className={`absolute top-[3px] h-4 w-4 rounded-full bg-white shadow transition-transform ${
            checked ? 'translate-x-[22px]' : 'translate-x-[3px]'
          }`}
        />
      </div>
    </button>
  );
}
