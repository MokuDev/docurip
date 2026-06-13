import type { ReactNode } from 'react';

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center h-full">
      <div className="text-charcoal mb-4">{icon}</div>
      <h3 className="text-ghost font-semibold mb-2">{title}</h3>
      <p className="text-secondary text-sm max-w-sm mb-4">{description}</p>
      {action}
    </div>
  );
}
