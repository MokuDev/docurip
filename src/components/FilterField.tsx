interface FilterFieldProps {
  label: string;
  values: string[];
  onChange: (next: string[]) => void;
  disabled?: boolean;
  rows?: number;
  placeholder?: string;
  help?: string;
}

export function FilterField({
  label,
  values,
  onChange,
  disabled,
  rows = 2,
  placeholder,
  help,
}: FilterFieldProps) {
  return (
    <div>
      <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
        {label}
      </label>
      <textarea
        value={values.join('\n')}
        onChange={(e) => onChange(e.target.value.split('\n'))}
        disabled={disabled}
        rows={rows}
        placeholder={placeholder}
        className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all resize-none"
      />
      {help && <p className="text-[11px] text-charcoal mt-1">{help}</p>}
    </div>
  );
}
