import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  CalendarCheck,
  Plus,
  Trash,
  PencilSimple,
  Clock,
  X,
  Power,
} from '@phosphor-icons/react';
import { ToggleRow } from '../components/ToggleRow';
import type { AppSettings, Cadence, Schedule, TemplateConfig } from '../types';

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

const INPUT_CLS =
  'w-full bg-surface/50 border border-abyssal rounded-md px-3 py-1.5 text-sm text-ghost placeholder-charcoal/40 focus:outline-none focus:border-accentGreen/50';

/** Build a crawl config from the user's default settings — schedules use
 * the same defaults a plain crawl would, minus the URL. */
function configFromSettings(s: AppSettings): TemplateConfig {
  return {
    maxDepth: s.defaultMaxDepth,
    pageLimit: s.defaultPageLimit,
    downloadAssets: s.defaultDownloadAssets,
    headlessStrategy: (s.defaultHeadlessStrategy as TemplateConfig['headlessStrategy']) || 'never',
    contentSelectors: [],
    excludePatterns: [],
    includePatterns: [],
    pathPrefix: '',
    respectRobotsTxt: s.defaultRespectRobotsTxt,
    stayWithinDomain: s.defaultStayWithinDomain,
    ssrfProtection: s.defaultSsrfProtection,
    outputDir: '',
    profile: null,
  };
}

function pad(n: number) {
  return String(n).padStart(2, '0');
}

function fmt(iso?: string | null): string {
  if (!iso) return '—';
  const d = new Date(iso);
  return isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

function cadenceSummary(s: Schedule): string {
  const time = `${pad(s.hour)}:${pad(s.minute)} UTC`;
  if (s.cadence === 'daily') return `Daily at ${time}`;
  if (s.cadence === 'weekly') return `Weekly on ${WEEKDAYS[s.weekday ?? 1]} at ${time}`;
  return `Monthly on day ${s.dayOfMonth ?? 1} at ${time}`;
}

interface DraftState {
  id: string;
  name: string;
  url: string;
  cadence: Cadence;
  time: string; // "HH:MM" UTC
  weekday: number;
  dayOfMonth: number;
  enabled: boolean;
}

const emptyDraft: DraftState = {
  id: '',
  name: '',
  url: '',
  cadence: 'daily',
  time: '09:00',
  weekday: 1,
  dayOfMonth: 1,
  enabled: true,
};

export function SchedulesView() {
  const [schedules, setSchedules] = useState<Schedule[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<DraftState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const [list, s] = await Promise.all([
        invoke<Schedule[]>('list_schedules'),
        invoke<AppSettings>('get_settings'),
      ]);
      setSchedules(list || []);
      setSettings(s);
    } catch (e) {
      console.error('Failed to load schedules', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const startNew = () => {
    setError(null);
    setEditing({ ...emptyDraft });
  };

  const startEdit = (s: Schedule) => {
    setError(null);
    setEditing({
      id: s.id,
      name: s.name,
      url: s.url,
      cadence: s.cadence,
      time: `${pad(s.hour)}:${pad(s.minute)}`,
      weekday: s.weekday ?? 1,
      dayOfMonth: s.dayOfMonth ?? 1,
      enabled: s.enabled,
    });
  };

  const save = async () => {
    if (!editing || !settings) return;
    if (!editing.name.trim()) {
      setError('Name is required.');
      return;
    }
    try {
      new URL(editing.url);
    } catch {
      setError('Enter a valid URL (including http:// or https://).');
      return;
    }
    const [hourStr, minStr] = editing.time.split(':');
    // Preserve the existing config when editing; use defaults for new ones.
    const existing = schedules.find((s) => s.id === editing.id);
    const schedule: Schedule = {
      id: editing.id,
      name: editing.name.trim(),
      url: editing.url.trim(),
      config: existing?.config ?? configFromSettings(settings),
      cadence: editing.cadence,
      hour: Number(hourStr) || 0,
      minute: Number(minStr) || 0,
      weekday: editing.cadence === 'weekly' ? editing.weekday : null,
      dayOfMonth: editing.cadence === 'monthly' ? editing.dayOfMonth : null,
      enabled: editing.enabled,
      createdAt: existing?.createdAt ?? '',
      lastRun: existing?.lastRun ?? null,
      nextRun: existing?.nextRun ?? '',
      lastJobId: existing?.lastJobId ?? null,
    };
    setSaving(true);
    setError(null);
    try {
      await invoke<Schedule>('save_schedule', { schedule });
      setEditing(null);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async (id: string) => {
    try {
      await invoke('delete_schedule', { scheduleId: id });
      await load();
    } catch (e) {
      console.error('Failed to delete schedule', e);
    }
  };

  const toggle = async (s: Schedule) => {
    try {
      await invoke('toggle_schedule', { scheduleId: s.id, enabled: !s.enabled });
      await load();
    } catch (e) {
      console.error('Failed to toggle schedule', e);
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="h-14 flex items-center justify-between px-5 border-b border-abyssal/50">
        <h1 className="text-ghost font-semibold text-base">Scheduled Crawls</h1>
        <button
          onClick={startNew}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm bg-accentGreen/10 text-accentGreen hover:bg-accentGreen/20 transition-all"
        >
          <Plus size={14} />
          New Schedule
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-5">
        {loading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-accentGreen" />
          </div>
        ) : schedules.length === 0 && !editing ? (
          <div className="flex flex-col items-center justify-center h-full text-charcoal">
            <CalendarCheck size={48} className="mb-4 opacity-20" />
            <p className="text-ghost font-medium">No scheduled crawls</p>
            <p className="text-xs mt-1 opacity-50">
              Create one to crawl a site automatically on a recurring cadence.
            </p>
          </div>
        ) : (
          <div className="space-y-3 max-w-3xl">
            {schedules.map((s) => (
              <div
                key={s.id}
                className="bg-surface/30 border border-abyssal/50 rounded-lg p-4 group"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <h3 className="text-ghost font-medium text-sm truncate">{s.name}</h3>
                      <span
                        className={`text-[10px] px-1.5 py-0.5 rounded uppercase tracking-wider ${
                          s.enabled
                            ? 'bg-accentGreen/15 text-accentGreen'
                            : 'bg-abyssal/50 text-charcoal'
                        }`}
                      >
                        {s.enabled ? 'Active' : 'Paused'}
                      </span>
                    </div>
                    <p className="text-xs text-charcoal break-all mb-2">{s.url}</p>
                    <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-charcoal">
                      <span className="flex items-center gap-1">
                        <Clock size={12} /> {cadenceSummary(s)}
                      </span>
                      <span>Next: {s.enabled ? fmt(s.nextRun) : '—'}</span>
                      <span>Last: {fmt(s.lastRun)}</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                      onClick={() => toggle(s)}
                      className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors text-xs px-2"
                      title={s.enabled ? 'Pause schedule' : 'Enable schedule'}
                    >
                      {s.enabled ? 'Pause' : 'Enable'}
                    </button>
                    <button
                      onClick={() => startEdit(s)}
                      className="p-1.5 text-charcoal hover:text-ghost hover:bg-abyssal rounded transition-colors"
                      title="Edit"
                    >
                      <PencilSimple size={16} />
                    </button>
                    <button
                      onClick={() => remove(s.id)}
                      className="p-1.5 text-charcoal hover:text-crimson hover:bg-crimson/10 rounded transition-colors"
                      title="Delete"
                    >
                      <Trash size={16} />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {editing && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/50" onClick={() => setEditing(null)} />
          <div className="relative bg-deepVoid border border-abyssal rounded-xl w-[440px] max-w-[92vw] p-5 shadow-2xl">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-ghost font-semibold text-sm">
                {editing.id ? 'Edit Schedule' : 'New Schedule'}
              </h2>
              <button
                onClick={() => setEditing(null)}
                className="p-1 text-charcoal hover:text-ghost rounded"
              >
                <X size={16} />
              </button>
            </div>

            <div className="space-y-3">
              <Field label="Name">
                <input
                  value={editing.name}
                  onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                  placeholder="Nightly docs sync"
                  className={INPUT_CLS}
                />
              </Field>
              <Field label="URL">
                <input
                  value={editing.url}
                  onChange={(e) => setEditing({ ...editing, url: e.target.value })}
                  placeholder="https://docs.example.com"
                  className={INPUT_CLS}
                />
              </Field>

              <div className="grid grid-cols-2 gap-3">
                <Field label="Cadence">
                  <select
                    value={editing.cadence}
                    onChange={(e) =>
                      setEditing({ ...editing, cadence: e.target.value as Cadence })
                    }
                    className={`${INPUT_CLS} cursor-pointer`}
                  >
                    <option value="daily">Daily</option>
                    <option value="weekly">Weekly</option>
                    <option value="monthly">Monthly</option>
                  </select>
                </Field>
                <Field label="Time (UTC)">
                  <input
                    type="time"
                    value={editing.time}
                    onChange={(e) => setEditing({ ...editing, time: e.target.value })}
                    className={`${INPUT_CLS} cursor-pointer`}
                  />
                </Field>
              </div>

              {editing.cadence === 'weekly' && (
                <Field label="Weekday">
                  <select
                    value={editing.weekday}
                    onChange={(e) => setEditing({ ...editing, weekday: Number(e.target.value) })}
                    className={`${INPUT_CLS} cursor-pointer`}
                  >
                    {WEEKDAYS.map((d, i) => (
                      <option key={i} value={i}>
                        {d}
                      </option>
                    ))}
                  </select>
                </Field>
              )}

              {editing.cadence === 'monthly' && (
                <Field label="Day of month (1–28)">
                  <input
                    type="number"
                    min={1}
                    max={28}
                    value={editing.dayOfMonth}
                    onChange={(e) =>
                      setEditing({
                        ...editing,
                        dayOfMonth: Math.min(28, Math.max(1, Number(e.target.value) || 1)),
                      })
                    }
                    className={INPUT_CLS}
                  />
                </Field>
              )}

              <ToggleRow
                label="Enabled"
                description="When off, the schedule is kept but won't fire."
                checked={editing.enabled}
                onChange={(v) => setEditing({ ...editing, enabled: v })}
                IconOn={Power}
              />

              {error && <p className="text-xs text-crimson">{error}</p>}

              <div className="flex justify-end gap-2 pt-1">
                <button
                  onClick={() => setEditing(null)}
                  className="px-3 py-1.5 rounded-md text-sm text-charcoal hover:text-ghost hover:bg-abyssal transition-colors"
                >
                  Cancel
                </button>
                <button
                  onClick={save}
                  disabled={saving}
                  className="px-3 py-1.5 rounded-md text-sm bg-accentGreen text-slate-900 font-semibold hover:bg-brightGreen transition-all disabled:opacity-50"
                >
                  {saving ? 'Saving…' : 'Save'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="text-[11px] font-medium uppercase tracking-wider text-charcoal">{label}</span>
      <div className="mt-1">{children}</div>
    </label>
  );
}
