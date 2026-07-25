import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AnnotationPanel } from './AnnotationPanel';

function setup(overrides: Partial<React.ComponentProps<typeof AnnotationPanel>> = {}) {
  const save = vi.fn().mockResolvedValue(undefined);
  const onSaved = vi.fn();
  const props: React.ComponentProps<typeof AnnotationPanel> = {
    jobId: 'job-1',
    pageUrl: 'https://example.com/a',
    value: '',
    onSaved,
    save,
    ...overrides,
  };
  const utils = render(<AnnotationPanel {...props} />);
  return { save, onSaved, ...utils };
}

async function flushMicrotasks() {
  await Promise.resolve();
  await Promise.resolve();
}

describe('AnnotationPanel', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('renders the current value in the textarea', () => {
    setup({ value: 'existing note' });
    expect(screen.getByRole('textbox')).toHaveValue('existing note');
  });

  it('debounces save on keystrokes', async () => {
    vi.useFakeTimers();
    const { save } = setup();
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: 'hi' } });
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(700);
    expect(save).toHaveBeenCalledWith('https://example.com/a', 'hi');
  });

  it('force-flushes on blur if the draft differs from value', async () => {
    const { save } = setup({ value: 'old' });
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: 'new draft' } });
    fireEvent.blur(textarea);
    await flushMicrotasks();
    expect(save).toHaveBeenCalledWith('https://example.com/a', 'new draft');
  });

  it('does not save on blur when the trimmed draft matches value', () => {
    const { save } = setup({ value: 'same' });
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: '  same  ' } });
    fireEvent.blur(textarea);
    expect(save).not.toHaveBeenCalled();
  });

  it('resets the draft when pageUrl changes', () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const onSaved = vi.fn();
    const { rerender } = render(
      <AnnotationPanel
        jobId="job-1"
        pageUrl="https://example.com/a"
        value="note A"
        onSaved={onSaved}
        save={save}
      />,
    );
    expect(screen.getByRole('textbox')).toHaveValue('note A');
    rerender(
      <AnnotationPanel
        jobId="job-1"
        pageUrl="https://example.com/b"
        value="note B"
        onSaved={onSaved}
        save={save}
      />,
    );
    expect(screen.getByRole('textbox')).toHaveValue('note B');
  });
});
