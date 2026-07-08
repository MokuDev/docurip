import { useState, useEffect } from 'react';
import {
  Browser,
  ClockCounterClockwise,
  Gear,
  GlobeHemisphereWest,
  FileArrowUp
} from '@phosphor-icons/react';
import { open } from '@tauri-apps/plugin-shell';
import { DashboardView } from './views/Dashboard';
import { NewCrawlView } from './views/NewCrawl';
import { HistoryView } from './views/History';
import { SettingsView } from './views/Settings';
import { ImportView } from './views/ImportView';
import { LiveConsole } from './components/LiveConsole';
import { TopStatusBar } from './components/TopStatusBar';
import { SystemStatusBar } from './components/SystemStatusBar';
import { ToastContainer } from './components/ToastContainer';
import { useCrawlEvents } from './hooks/useCrawlEvents';
import { useUpdater } from './hooks/useUpdater';

function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'crawls' | 'history' | 'settings' | 'active-crawl' | 'import'>('dashboard');
  const [pendingUrl, setPendingUrl] = useState('');
  const [liveConsoleOpen, setLiveConsoleOpen] = useState(false);
  const { activeJobIds } = useCrawlEvents();
  const { updateAvailable, downloading, error: updateError, installUpdate, dismiss } = useUpdater();
  const activeJobsCount = activeJobIds.size;

  useEffect(() => {
    if (activeJobsCount > 0 && !liveConsoleOpen) {
      setLiveConsoleOpen(true);
    }
  }, [activeJobsCount, liveConsoleOpen]);

  return (
    <div className="h-screen bg-deepVoid flex flex-col text-smooth font-sans">
      <TopStatusBar />

      {updateAvailable && (
        <div className="bg-accentGreen/10 border-b border-accentGreen/20 px-4 py-2 flex items-center justify-between text-sm">
          <div className="flex flex-col">
            <span className="text-ghost">
              Update available: <strong className="text-accentGreen">v{updateAvailable.version}</strong>
            </span>
            {updateError && (
              <span className="text-red-400 text-xs mt-0.5">
                Update failed: {updateError}
              </span>
            )}
          </div>
          <div className="flex items-center space-x-2">
            <button
              onClick={installUpdate}
              disabled={downloading}
              className="px-3 py-1 bg-accentGreen hover:bg-brightGreen text-slate-900 font-semibold rounded text-xs transition-all disabled:opacity-50"
            >
              {downloading ? 'Downloading...' : updateError ? 'Retry' : 'Install & Restart'}
            </button>
            <button
              onClick={dismiss}
              className="px-2 py-1 text-charcoal hover:text-ghost text-xs transition-colors"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}

      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <aside className="w-64 flex-shrink-0 bg-deepVoid border-r border-abyssal/50 flex flex-col">
          {/* Logo */}
            <div className="h-16 flex items-center justify-center px-6 border-b border-abyssal/50">
            <img src="/docurip_logo_g.png" alt="Docurip" className="h-[38px] object-contain" />
          </div>

          {/* Nav */}
          <nav className="flex-1 py-4 px-3 space-y-1.5">
            <NavItem
              icon={<Browser weight="fill" size={18} />}
              label="Dashboard"
              active={activeTab === 'dashboard'}
              onClick={() => setActiveTab('dashboard')}
              badge={activeJobsCount > 0 ? activeJobsCount.toString() : undefined}
            />
            {activeJobsCount > 0 ? (
              <NavItem
                icon={<GlobeHemisphereWest weight="fill" size={18} />}
                label="Active Crawl"
                active={activeTab === 'active-crawl'}
                onClick={() => setActiveTab('active-crawl')}
                badge="RUNNING"
              />
            ) : (
              <NavItem
                icon={<GlobeHemisphereWest weight="fill" size={18} />}
                label="New Crawl"
                active={activeTab === 'crawls'}
                onClick={() => setActiveTab('crawls')}
              />
            )}
            <NavItem
              icon={<FileArrowUp weight="fill" size={18} />}
              label="Import"
              active={activeTab === 'import'}
              onClick={() => setActiveTab('import')}
            />
            <NavItem
              icon={<ClockCounterClockwise weight="fill" size={18} />}
              label="History"
              active={activeTab === 'history'}
              onClick={() => setActiveTab('history')}
            />
            <NavItem
              icon={<Gear weight="fill" size={18} />}
              label="Settings"
              active={activeTab === 'settings'}
              onClick={() => setActiveTab('settings')}
            />
          </nav>

          {/* Footer */}
          <div className="h-auto flex flex-col items-center justify-center py-3 border-t border-abyssal/50 space-y-1">
            <span className="text-charcoal text-[10px] tracking-widest uppercase">
              v0.6.0
            </span>
            <span className="text-charcoal text-[10px]">
              made with love by{' '}
              <button
                onClick={() => open('https://moku.cx')}
                className="text-accentGreen/70 hover:text-accentGreen transition-colors underline-offset-2 hover:underline"
              >
                moku
              </button>
            </span>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 flex flex-col overflow-hidden relative">
          {activeTab === 'dashboard' && <DashboardView onQuickStart={(url) => { setPendingUrl(url); setActiveTab(activeJobsCount > 0 ? 'active-crawl' : 'crawls'); }} />}
          {activeTab === 'active-crawl' && <NewCrawlView prefillUrl={pendingUrl} />}
          {activeTab === 'crawls' && <NewCrawlView prefillUrl={pendingUrl} />}
          {activeTab === 'import' && <ImportView />}
          {activeTab === 'history' && <HistoryView />}
          {activeTab === 'settings' && <SettingsView />}
        </main>
      </div>

      <SystemStatusBar />

      {/* Live Console Drawer */}
      {liveConsoleOpen && (
        <LiveConsole />
      )}

      <ToastContainer />
    </div>
  );
}

const NavItem = ({
  icon,
  label,
  active,
  onClick,
  badge
}: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  onClick: () => void;
  badge?: string;
}) => (
  <button
    onClick={onClick}
    className={`
      w-full flex items-center px-3.5 py-2.5 rounded-md transition-all duration-fast group relative
      ${active
        ? 'bg-accentGreen/10 text-accentGreen border-l-2 border-accentGreen'
        : 'text-secondary hover:text-ghost hover:bg-surface/60 border-l-2 border-transparent'
      }
    `}
  >
    <span className={`mr-3 ${active ? 'text-accentGreen' : 'text-charcoal group-hover:text-secondary'}`}>
      {icon}
    </span>
    <span className="font-medium text-[13px]">{label}</span>
    {badge && (
      <span className="ml-auto text-[10px] bg-accentGreen/15 text-accentGreen px-1.5 py-0.5 rounded font-mono">
        {badge}
      </span>
    )}
  </button>
);

export default App;