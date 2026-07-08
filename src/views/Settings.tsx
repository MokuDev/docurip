import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  FolderOpen,
  FloppyDisk,
  ArrowClockwise,
  CheckCircle,
  Warning,
} from '@phosphor-icons/react';
import type { AppSettings } from '../types';
import { useTheme, THEME_ORDER, THEME_META } from '../hooks/useTheme';

const DEFAULT_SETTINGS: AppSettings = {
  outputDir: '',
  concurrency: 3,
  requestDelay: 1000,
  timeout: 30000,
  userAgent: 'Docurip/0.1.0 (Documentation Crawler)',
  defaultMaxDepth: 2,
  defaultPageLimit: 50,
  defaultDownloadAssets: false,
  defaultHeadlessStrategy: 'auto',
  defaultRespectRobotsTxt: true,
  defaultStayWithinDomain: true,
  defaultSsrfProtection: true,
  windowWidth: 1280,
  windowHeight: 900,
  theme: 'system',
};

const WINDOW_PRESETS = [
  { w: 1280, h: 900, label: 'Compact' },
  { w: 1600, h: 1000, label: 'Standard' },
  { w: 1920, h: 1080, label: 'Full HD' },
  { w: 2560, h: 1440, label: 'QHD' },
  { w: 3840, h: 2160, label: 'UHD / 4K' },
];

export function SettingsView() {
  const { theme, setTheme } = useTheme();
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [notice, setNotice] = useState('');

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    setLoading(true);
    try {
      const data: AppSettings = await invoke('get_settings');
      if (data) setSettings(data);
    } catch (err) {
      console.error('Failed to load settings', err);
    } finally {
      setLoading(false);
    }
  };

  const validate = (s: AppSettings) => {
    const e: Record<string, string> = {};
    if (Number.isNaN(s.defaultMaxDepth) || s.defaultMaxDepth < 1) e.defaultMaxDepth = 'Must be at least 1';
    if (Number.isNaN(s.defaultPageLimit) || s.defaultPageLimit < 1) e.defaultPageLimit = 'Must be at least 1';
    if (Number.isNaN(s.concurrency) || s.concurrency < 1) e.concurrency = 'Must be at least 1';
    if (Number.isNaN(s.requestDelay) || s.requestDelay < 0) e.requestDelay = 'Must be 0 or greater';
    if (Number.isNaN(s.timeout) || s.timeout < 1000) e.timeout = 'Must be at least 1000 ms';
    return e;
  };

  const handleSave = async () => {
    setSaved(false);
    setError('');
    const validationErrors = validate(settings);
    if (Object.keys(validationErrors).length > 0) {
      setErrors(validationErrors);
      return;
    }
    try {
      // Theme is persisted independently via useTheme() (see setTheme), so
      // splice in its live value here instead of this component's own
      // possibly-stale copy to avoid reverting a theme change made elsewhere
      // (e.g. the TopStatusBar quick toggle) while this page was open.
      await invoke('update_settings', { settings: { ...settings, theme } });
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleReset = () => {
    setSettings(DEFAULT_SETTINGS);
    setTheme(DEFAULT_SETTINGS.theme);
    setSaved(false);
    setError('');
    setErrors({});
    setNotice('');
    invoke('set_window_size', { width: DEFAULT_SETTINGS.windowWidth, height: DEFAULT_SETTINGS.windowHeight }).catch(() => {});
  };

  const handleWindowSizeChange = async (value: string) => {
    const [w, h] = value.split('x').map(Number);
    setSettings({ ...settings, windowWidth: w, windowHeight: h });
    try {
      const result: { clamped: boolean; appliedWidth: number; appliedHeight: number } =
        await invoke('set_window_size', { width: w, height: h });
      if (result.clamped) {
        setNotice(`Window size clamped to ${result.appliedWidth}\u00d7${result.appliedHeight} to fit your display.`);
        setTimeout(() => setNotice(''), 5000);
      } else {
        setNotice('');
      }
    } catch (err) {
      console.error('Failed to resize window', err);
    }
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-accentGreen" />
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-8 max-w-3xl">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-display font-bold text-ghost">Settings</h1>
          <p className="text-secondary text-sm mt-1">Configure your crawling preferences</p>
        </div>
        <div className="flex items-center space-x-3">
          <button
            onClick={handleReset}
            className="flex items-center space-x-2 px-4 py-2 bg-surface hover:bg-abyssal text-secondary hover:text-ghost border border-abyssal rounded-md transition-all duration-fast"
          >
            <ArrowClockwise size={16} />
            <span>Reset</span>
          </button>
          <button
            onClick={handleSave}
            className="flex items-center space-x-2 px-4 py-2 bg-accentGreen hover:bg-brightGreen text-slate-900 font-semibold rounded-md transition-all duration-fast hover:shadow-[0_0_15px_rgba(22,224,141,0.3)]"
          >
            <FloppyDisk weight="fill" size={16} />
            <span>Save</span>
          </button>
        </div>
      </div>

      {saved && (
        <div className="mb-6 flex items-center text-brightGreen text-sm bg-brightGreen/10 border border-brightGreen/20 rounded-md px-4 py-3">
          <CheckCircle weight="fill" size={16} className="mr-2" />
          Settings saved successfully
        </div>
      )}

      {error && (
        <div className="mb-6 flex items-center text-crimson text-sm bg-crimson/10 border border-crimson/20 rounded-md px-4 py-3">
          <Warning weight="fill" size={16} className="mr-2" />
          {error}
        </div>
      )}

      {notice && (
        <div className="mb-6 flex items-center text-yellow-400 text-sm bg-yellow-400/10 border border-yellow-400/20 rounded-md px-4 py-3">
          <Warning weight="fill" size={16} className="mr-2" />
          {notice}
        </div>
      )}

      <div className="space-y-6">
        {/* Appearance Settings */}
        <Section title="Appearance">
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Theme
            </label>
            <div className="flex gap-2">
              {THEME_ORDER.map((value) => {
                const { label, icon: Icon } = THEME_META[value];
                return (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setTheme(value)}
                    className={`flex-1 flex items-center justify-center gap-2 px-3 py-2.5 rounded-md border text-sm transition-all ${
                      theme === value
                        ? 'bg-accentGreen/10 border-accentGreen/50 text-accentGreen'
                        : 'bg-surface/50 border-abyssal text-secondary hover:text-ghost hover:border-accentGreen/30'
                    }`}
                  >
                    <Icon size={16} weight={theme === value ? 'fill' : 'regular'} />
                    <span>{label}</span>
                  </button>
                );
              })}
            </div>
            <p className="text-charcoal text-xs mt-1.5">
              Applied immediately. "System" follows your OS light/dark setting.
            </p>
          </div>
        </Section>

        {/* Output Settings */}
        <Section title="Output">
          <div className="space-y-4">
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Default Output Directory
              </label>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      const selected = await open({ directory: true, multiple: false, title: 'Select Default Output Directory' });
                      if (selected) setSettings({ ...settings, outputDir: selected });
                    } catch (err) {
                      console.error('Failed to open directory picker', err);
                    }
                  }}
                  className="flex items-center gap-2 bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm hover:border-accentGreen/50 focus:outline-none focus:border-accentGreen/50 focus:ring-1 focus:ring-accentGreen/20 transition-all flex-1"
                >
                  <FolderOpen className="w-4 h-4 text-charcoal" />
                  <span>{settings.outputDir || 'Default (~/.docurip)'}</span>
                </button>
                {settings.outputDir && (
                  <button
                    type="button"
                    onClick={() => setSettings({ ...settings, outputDir: '' })}
                    className="text-charcoal hover:text-ghost text-xs transition-colors"
                  >
                    Reset
                  </button>
                )}
              </div>
            </div>
          </div>
        </Section>

        {/* Window Settings */}
        <Section title="Window">
          <div>
            <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
              Window Size
            </label>
            <select
              value={`${settings.windowWidth}x${settings.windowHeight}`}
              onChange={(e) => handleWindowSizeChange(e.target.value)}
              className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all appearance-none"
            >
              {WINDOW_PRESETS.map((p) => (
                <option key={`${p.w}x${p.h}`} value={`${p.w}x${p.h}`}>
                  {p.w} × {p.h} — {p.label}
                </option>
              ))}
            </select>
            <p className="text-charcoal text-xs mt-1.5">
              Applied immediately. Resizes and centers the window on your current display.
            </p>
          </div>
        </Section>

        {/* Crawling Settings */}
        <Section title="Crawling">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Default Max Depth
              </label>
              <input
                type="number"
                min={1}
                max={10}
                value={settings.defaultMaxDepth}
                onChange={(e) => {
                  const val = e.target.value === '' ? NaN : Number(e.target.value);
                  setSettings({ ...settings, defaultMaxDepth: val });
                  setErrors((prev) => ({ ...prev, defaultMaxDepth: '' }));
                }}
                aria-invalid={!!errors.defaultMaxDepth}
                aria-describedby={errors.defaultMaxDepth ? 'defaultMaxDepth-error' : undefined}
                className={`w-full bg-surface/50 border rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all ${errors.defaultMaxDepth ? 'border-crimson' : 'border-abyssal'}`}
              />
              {errors.defaultMaxDepth && <p id="defaultMaxDepth-error" className="text-crimson text-xs mt-1">{errors.defaultMaxDepth}</p>}
            </div>
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Default Page Limit
              </label>
              <input
                type="number"
                min={1}
                max={10000}
                value={settings.defaultPageLimit}
                onChange={(e) => {
                  const val = e.target.value === '' ? NaN : Number(e.target.value);
                  setSettings({ ...settings, defaultPageLimit: val });
                  setErrors((prev) => ({ ...prev, defaultPageLimit: '' }));
                }}
                aria-invalid={!!errors.defaultPageLimit}
                aria-describedby={errors.defaultPageLimit ? 'defaultPageLimit-error' : undefined}
                className={`w-full bg-surface/50 border rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all ${errors.defaultPageLimit ? 'border-crimson' : 'border-abyssal'}`}
              />
              {errors.defaultPageLimit && <p id="defaultPageLimit-error" className="text-crimson text-xs mt-1">{errors.defaultPageLimit}</p>}
            </div>
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Concurrency
              </label>
              <input
                type="number"
                min={1}
                max={20}
                value={settings.concurrency}
                onChange={(e) => {
                  const val = e.target.value === '' ? NaN : Number(e.target.value);
                  setSettings({ ...settings, concurrency: val });
                  setErrors((prev) => ({ ...prev, concurrency: '' }));
                }}
                aria-invalid={!!errors.concurrency}
                aria-describedby={errors.concurrency ? 'concurrency-error' : undefined}
                className={`w-full bg-surface/50 border rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all ${errors.concurrency ? 'border-crimson' : 'border-abyssal'}`}
              />
              {errors.concurrency && <p id="concurrency-error" className="text-crimson text-xs mt-1">{errors.concurrency}</p>}
            </div>
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Request Delay (ms)
              </label>
              <input
                type="number"
                min={0}
                max={30000}
                value={settings.requestDelay}
                onChange={(e) => {
                  const val = e.target.value === '' ? NaN : Number(e.target.value);
                  setSettings({ ...settings, requestDelay: val });
                  setErrors((prev) => ({ ...prev, requestDelay: '' }));
                }}
                aria-invalid={!!errors.requestDelay}
                aria-describedby={errors.requestDelay ? 'requestDelay-error' : undefined}
                className={`w-full bg-surface/50 border rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all ${errors.requestDelay ? 'border-crimson' : 'border-abyssal'}`}
              />
              {errors.requestDelay && <p id="requestDelay-error" className="text-crimson text-xs mt-1">{errors.requestDelay}</p>}
            </div>
          </div>
        </Section>

        {/* Network Settings */}
        <Section title="Network">
          <div className="space-y-4">
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                Timeout (ms)
              </label>
              <input
                type="number"
                min={1000}
                max={120000}
                value={settings.timeout}
                onChange={(e) => {
                  const val = e.target.value === '' ? NaN : Number(e.target.value);
                  setSettings({ ...settings, timeout: val });
                  setErrors((prev) => ({ ...prev, timeout: '' }));
                }}
                aria-invalid={!!errors.timeout}
                aria-describedby={errors.timeout ? 'timeout-error' : undefined}
                className={`w-full bg-surface/50 border rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all ${errors.timeout ? 'border-crimson' : 'border-abyssal'}`}
              />
              {errors.timeout && <p id="timeout-error" className="text-crimson text-xs mt-1">{errors.timeout}</p>}
            </div>
            <div>
              <label className="block text-[11px] font-medium uppercase tracking-wider text-charcoal mb-1.5">
                User Agent
              </label>
              <input
                type="text"
                value={settings.userAgent}
                onChange={(e) =>
                  setSettings({ ...settings, userAgent: e.target.value })
                }
                className="w-full bg-surface/50 border border-abyssal rounded-md px-3 py-2.5 text-ghost text-sm focus:outline-none focus:border-accentGreen/50 transition-all"
              />
            </div>
          </div>
        </Section>
      </div>
    </div>
  );
}

const Section = ({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) => (
  <div className="bg-surface/30 border border-abyssal/50 rounded-lg p-5">
    <h3 className="text-sm font-semibold text-ghost mb-4">{title}</h3>
    {children}
  </div>
);
