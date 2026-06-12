import { useState, useEffect } from 'react';
import {
  Browser,
  ClockCounterClockwise,
  Gear,
  GlobeHemisphereWest,
  XLogo
} from '@phosphor-icons/react';
import { DashboardView } from './views/Dashboard';
import { NewCrawlView } from './views/NewCrawl';
import { HistoryView } from './views/History';
import { SettingsView } from './views/Settings';
import { LiveConsole } from './components/LiveConsole';

interface TrackedJob {
  jobId: string;
  url: string;
  status: string;
}

function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'crawls' | 'history' | 'settings'>('dashboard');
  const [liveConsoleOpen, setLiveConsoleOpen] = useState(false);
  const [activeJobs, setActiveJobs] = useState<TrackedJob[]>([]);

  // Monitor active jobs
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const jobs: TrackedJob[] = await invoke('list_jobs');
        const running = (jobs || []).filter((j: any) =>
          j.status === 'running' || j.status === 'queued'
        );
        setActiveJobs(running);
        if (running.length > 0 && !liveConsoleOpen) {
          setLiveConsoleOpen(true);
        }
      } catch {
        // silently ignore if backend not ready
      }
    }, 2000);
    return () => clearInterval(interval);
  }, [liveConsoleOpen]);

  return (
    <div className="h-screen bg-[#050a0f] flex text-smooth font-sans">
      {/* Sidebar */}
      <aside className="w-64 flex-shrink-0 bg-deepVoid border-r border-abyssal/50 flex flex-col">
        {/* Logo */}
        <div className="h-16 flex items-center px-6 border-b border-abyssal/50">
          <XLogo weight="fill" size={24} className="text-accentGreen mr-2" />
          <span className="font-display text-lg font-bold text-ghost tracking-tight">
            Docurip
          </span>
        </div>

        {/* Nav */}
        <nav className="flex-1 py-4 px-3 space-y-1.5">
          <NavItem
            icon={<Browser weight="fill" size={18} />}
            label="Dashboard"
            active={activeTab === 'dashboard'}
            onClick={() => setActiveTab('dashboard')}
            badge={activeJobs.length > 0 ? `${activeJobs.length}` : undefined}
          />
          <NavItem
            icon={<GlobeHemisphereWest weight="fill" size={18} />}
            label="New Crawl"
            active={activeTab === 'crawls'}
            onClick={() => setActiveTab('crawls')}
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
        <div className="h-10 flex items-center justify-center border-t border-abyssal/50">
          <span className="text-charcoal text-[10px] tracking-widest uppercase">
            v0.1.0-alpha
          </span>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 flex flex-col overflow-hidden relative">
        {activeTab === 'dashboard' && <DashboardView />}
        {activeTab === 'crawls' && <NewCrawlView />}
        {activeTab === 'history' && <HistoryView />}
        {activeTab === 'settings' && <SettingsView />}
      </main>

      {/* Live Console Drawer */}
      {liveConsoleOpen && (
        <LiveConsole onClose={() => setLiveConsoleOpen(false)} />
      )}
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