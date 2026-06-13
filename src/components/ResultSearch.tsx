import { MagnifyingGlass } from '@phosphor-icons/react';

interface ResultSearchProps {
  value: string;
  onChange: (query: string) => void;
  resultCount: number;
}

export function ResultSearch({ value, onChange, resultCount }: ResultSearchProps) {
  return (
    <div className="relative">
      <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 text-charcoal" size={16} />
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Search results..."
        className="w-full bg-deepVoid border border-abyssal/50 text-ghost placeholder-charcoal rounded-md pl-9 pr-4 py-2 text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
      />
      {value && (
        <span className="absolute right-3 top-1/2 -translate-y-1/2 text-charcoal text-xs">
          {resultCount} found
        </span>
      )}
    </div>
  );
}
