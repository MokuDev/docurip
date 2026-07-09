import { useState } from 'react';
import { Plus, X } from '@phosphor-icons/react';
import type { CrawlTemplate } from '../types';

interface TemplateBarProps {
  templates: CrawlTemplate[];
  disabled?: boolean;
  onApply: (template: CrawlTemplate) => void;
  onSave: (name: string) => void;
  onDelete: (id: string) => void;
}

export function TemplateBar({ templates, disabled, onApply, onSave, onDelete }: TemplateBarProps) {
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState('');

  const handleSave = () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    onSave(trimmed);
    setName('');
    setNaming(false);
  };

  return (
    <div>
      <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
        Templates
      </label>
      <div className="flex flex-wrap items-center gap-1.5">
        {templates.map((t) => (
          <div
            key={t.id}
            className="group flex items-center gap-1 pl-2.5 pr-1 py-1 rounded-full border border-abyssal bg-surface/50 hover:border-accentGreen/50 transition-all"
          >
            <button
              type="button"
              onClick={() => onApply(t)}
              disabled={disabled}
              className="text-xs text-secondary hover:text-ghost transition-colors disabled:cursor-not-allowed"
              title={t.url}
            >
              {t.name}
            </button>
            <button
              type="button"
              onClick={() => onDelete(t.id)}
              disabled={disabled}
              className="p-0.5 text-charcoal hover:text-crimson transition-colors rounded-full disabled:cursor-not-allowed"
              title="Delete template"
            >
              <X size={11} />
            </button>
          </div>
        ))}

        {naming ? (
          <div className="flex items-center gap-1">
            <input
              autoFocus
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleSave();
                if (e.key === 'Escape') {
                  setNaming(false);
                  setName('');
                }
              }}
              onBlur={() => {
                if (!name.trim()) setNaming(false);
              }}
              placeholder="Template name"
              className="bg-surface/50 border border-accentGreen/50 rounded-full px-2.5 py-1 text-xs text-ghost placeholder-charcoal/40 focus:outline-none w-32"
            />
            <button
              type="button"
              onClick={handleSave}
              className="text-xs text-accentGreen hover:text-brightGreen px-1"
            >
              Save
            </button>
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setNaming(true)}
            disabled={disabled}
            className="flex items-center gap-1 px-2.5 py-1 rounded-full border border-dashed border-abyssal text-charcoal hover:text-ghost hover:border-accentGreen/50 transition-all text-xs disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Plus size={11} />
            Save current
          </button>
        )}
      </div>
      {templates.length === 0 && !naming && (
        <p className="text-[11px] text-charcoal mt-1.5">No templates saved yet.</p>
      )}
    </div>
  );
}
