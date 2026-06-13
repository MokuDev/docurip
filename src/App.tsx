import { useState, useEffect } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import {
  Browser,
  ClockCounterClockwise,
  Gear,
  GlobeHemisphereWest
} from '@phosphor-icons/react';
import { DashboardView } from './views/Dashboard';
import { NewCrawlView } from './views/NewCrawl';
import { HistoryView } from './views/History';
import { SettingsView } from './views/Settings';
import { LiveConsole } from './components/LiveConsole';
import { TopStatusBar } from './components/TopStatusBar';
import { SystemStatusBar } from './components/SystemStatusBar';
import { ToastContainer } from './components/ToastContainer';
import { useCrawlEvents } from './hooks/useCrawlEvents';

function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'crawls' | 'history' | 'settings'>('dashboard');
  const [pendingUrl, setPendingUrl] = useState('');
  const [liveConsoleOpen, setLiveConsoleOpen] = useState(false);
  const { activeJobIds } = useCrawlEvents();
  const activeJobsCount = activeJobIds.size;

  useEffect(() => {
    if (activeJobsCount > 0 && !liveConsoleOpen) {
      setLiveConsoleOpen(true);
    }
  }, [activeJobsCount, liveConsoleOpen]);

  return (
    <div className="h-screen bg-[#050a0f] flex flex-col text-smooth font-sans">
      <TopStatusBar />

      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <aside className="w-64 flex-shrink-0 bg-deepVoid border-r border-abyssal/50 flex flex-col">
          {/* Logo */}
          <div className="h-16 flex items-center px-6 border-b border-abyssal/50">
            <img src="/docurip_logo_g.png" alt="Docurip" className="h-8 object-contain" />
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
          <AnimatePresence mode="wait">
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.2 }}
              className="h-full"
            >
              {activeTab === 'dashboard' && <DashboardView onQuickStart={(url) => { setPendingUrl(url); setActiveTab('crawls'); }} />}
              {activeTab === 'crawls' && <NewCrawlView prefillUrl={pendingUrl} />}
              {activeTab === 'history' && <HistoryView />}
              {activeTab === 'settings' && <SettingsView />}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      <SystemStatusBar />

      {/* Live Console Drawer */}
      {liveConsoleOpen && (
        <LiveConsole onClose={() => setLiveConsoleOpen(false)} />
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