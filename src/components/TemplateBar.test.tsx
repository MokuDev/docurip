import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TemplateBar } from './TemplateBar';
import type { CrawlTemplate } from '../types';

const templateConfig: CrawlTemplate['config'] = {
  maxDepth: 2, pageLimit: 100, downloadAssets: false, headlessStrategy: 'never',
  contentSelectors: [], excludePatterns: [], includePatterns: [], pathPrefix: '',
  respectRobotsTxt: true, stayWithinDomain: true, ssrfProtection: true, outputDir: '', profile: null,
};

function makeTemplate(overrides: Partial<CrawlTemplate> = {}): CrawlTemplate {
  return {
    id: 't-1', name: 'My Template', url: 'https://example.com', config: templateConfig,
    createdAt: '2026-01-01T00:00:00Z', ...overrides,
  };
}

describe('TemplateBar', () => {
  it('renders each template as a named chip', () => {
    render(
      <TemplateBar templates={[makeTemplate()]} onApply={vi.fn()} onSave={vi.fn()} onDelete={vi.fn()} />
    );
    expect(screen.getByText('My Template')).toBeInTheDocument();
  });

  it('shows an empty-state message when there are no templates', () => {
    render(<TemplateBar templates={[]} onApply={vi.fn()} onSave={vi.fn()} onDelete={vi.fn()} />);
    expect(screen.getByText('No templates saved yet.')).toBeInTheDocument();
  });

  it('calls onApply when a template chip is clicked', () => {
    const onApply = vi.fn();
    const template = makeTemplate();
    render(<TemplateBar templates={[template]} onApply={onApply} onSave={vi.fn()} onDelete={vi.fn()} />);

    fireEvent.click(screen.getByText('My Template'));
    expect(onApply).toHaveBeenCalledWith(template);
  });

  it('calls onDelete when a chip delete button is clicked', () => {
    const onDelete = vi.fn();
    render(
      <TemplateBar templates={[makeTemplate()]} onApply={vi.fn()} onSave={vi.fn()} onDelete={onDelete} />
    );

    fireEvent.click(screen.getByTitle('Delete template'));
    expect(onDelete).toHaveBeenCalledWith('t-1');
  });

  it('opens the naming input and calls onSave with the trimmed name', () => {
    const onSave = vi.fn();
    render(<TemplateBar templates={[]} onApply={vi.fn()} onSave={onSave} onDelete={vi.fn()} />);

    fireEvent.click(screen.getByText('Save current'));
    const input = screen.getByPlaceholderText('Template name');
    fireEvent.change(input, { target: { value: '  Docs Site  ' } });
    fireEvent.click(screen.getByText('Save'));

    expect(onSave).toHaveBeenCalledWith('Docs Site');
  });

  it('does not call onSave for a blank name', () => {
    const onSave = vi.fn();
    render(<TemplateBar templates={[]} onApply={vi.fn()} onSave={onSave} onDelete={vi.fn()} />);

    fireEvent.click(screen.getByText('Save current'));
    const input = screen.getByPlaceholderText('Template name');
    fireEvent.change(input, { target: { value: '   ' } });
    fireEvent.click(screen.getByText('Save'));

    expect(onSave).not.toHaveBeenCalled();
  });

  it('cancels naming on Escape', () => {
    render(<TemplateBar templates={[]} onApply={vi.fn()} onSave={vi.fn()} onDelete={vi.fn()} />);

    fireEvent.click(screen.getByText('Save current'));
    const input = screen.getByPlaceholderText('Template name');
    fireEvent.keyDown(input, { key: 'Escape' });

    expect(screen.queryByPlaceholderText('Template name')).not.toBeInTheDocument();
    expect(screen.getByText('Save current')).toBeInTheDocument();
  });

  it('disables apply and delete controls when disabled', () => {
    render(
      <TemplateBar templates={[makeTemplate()]} disabled onApply={vi.fn()} onSave={vi.fn()} onDelete={vi.fn()} />
    );

    expect(screen.getByText('My Template')).toBeDisabled();
    expect(screen.getByTitle('Delete template')).toBeDisabled();
    expect(screen.getByText('Save current')).toBeDisabled();
  });
});
