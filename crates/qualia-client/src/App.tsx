import { BrowserRouter as Router, Routes, Route, NavLink } from 'react-router-dom';
import { LayoutDashboard, Settings, MessageSquare, Database, Library, BrainCircuit, Package, Wallet as WalletIcon, Cpu, MemoryStick, Users, Fingerprint, FolderOpen } from 'lucide-react';
import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

import Dashboard from './pages/Dashboard';
import LLMHub from './pages/LLMHub';
import OntologyHub from './pages/OntologyHub';
import Chat from './pages/Chat';
import AppSettings from './pages/Settings';
import AppStore from './pages/AppStore';
import Wallet from './pages/Wallet';
import SpatialPhysics from './pages/SpatialPhysics';
import AssetLibrary from './pages/AssetLibrary';
import AddressBook from './pages/AddressBook';
import CredentialManager from './pages/CredentialManager';

function Sidebar() {
  const navItems = [
    { path: '/', label: 'Dashboard', icon: LayoutDashboard },
    { path: '/credentials', label: 'Identifiers & Credentials', icon: Fingerprint },
    { path: '/addressbook', label: 'Social Directory', icon: Users },
    { path: '/chat', label: 'Neuro-Chat', icon: MessageSquare },
    { path: '/llms', label: 'LLM Hub', icon: BrainCircuit },
    { path: '/ontologies', label: 'Ontologies', icon: Database },
    { path: '/assets', label: 'Asset Library', icon: Database },
    { path: '/physics', label: 'Physics Engine', icon: Package },
    { path: '/apps', label: 'App Manager', icon: Package },
    { path: '/wallet', label: 'Wallets & Routes', icon: WalletIcon },
    { path: '/settings', label: 'Settings', icon: Settings },
  ];

  return (
    <div className="w-72 bg-black/50 backdrop-blur-xl border-r border-white/10 flex flex-col h-full z-20">
      <div className="p-8 text-center border-b border-white/10">
        <h1 className="text-2xl font-black bg-gradient-to-r from-[#00f0ff] to-[#b026ff] bg-clip-text text-transparent tracking-widest uppercase">
          Qualia-DB
        </h1>
        <div className="text-xs text-gray-400 mt-2 tracking-widest">Edge-Native Webizen</div>
      </div>
      <nav className="flex-1 py-6 flex flex-col gap-2">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `flex items-center gap-4 px-8 py-3 text-sm font-semibold transition-all duration-300 border-l-4 ${
                isActive
                  ? 'text-[#00f0ff] bg-[#00f0ff]/5 border-[#00f0ff] shadow-[inset_10px_0_20px_-10px_rgba(0,240,255,0.2)]'
                  : 'text-gray-500 border-transparent hover:text-white hover:bg-white/5'
              }`
            }
          >
            <item.icon className="w-5 h-5" />
            {item.label}
          </NavLink>
        ))}
      </nav>
      <div className="p-6 text-xs text-gray-600 border-t border-white/10 flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-[#00ff88] animate-pulse"></div>
        Daemon Connected
      </div>
    </div>
  );
}

interface AgentConfig {
  storage_path: string;
  storage_quota_gb: number;
  base_connectivity_cost_ilp: number;
}

function SetupWizard({ onComplete }: { onComplete: () => void }) {
  const [path, setPath] = useState('');
  const [quota, setQuota] = useState(50);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    invoke<AgentConfig>('get_config').then(c => setPath(c.storage_path)).catch(console.error);
  }, []);

  const handleInit = async () => {
    setSaving(true);
    setError('');
    try {
      await invoke('save_config', {
        newConfig: { storage_path: path, storage_quota_gb: quota, base_connectivity_cost_ilp: 5000 },
      });
      onComplete();
    } catch (e: any) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-md">
      <div className="w-full max-w-lg glass-panel border-[#00f0ff]/30 shadow-[0_0_80px_rgba(0,240,255,0.1)]">
        <h1 className="text-2xl font-black bg-gradient-to-r from-[#00f0ff] to-[#b026ff] bg-clip-text text-transparent tracking-widest uppercase mb-1">
          Qualia-DB
        </h1>
        <p className="text-gray-400 text-sm mb-8">First-time setup — configure where your agent stores its data.</p>

        <div className="mb-6">
          <label className="block text-gray-400 text-xs font-bold uppercase tracking-widest mb-2 flex items-center gap-2">
            <FolderOpen className="w-3 h-3" /> Data Storage Path
          </label>
          <input
            type="text"
            value={path}
            onChange={e => setPath(e.target.value)}
            className="w-full bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#00f0ff] font-mono text-sm"
          />
          <p className="text-xs text-gray-600 mt-1.5">Models, ontologies, and vector databases will be stored here. The directory will be created if it doesn't exist.</p>
        </div>

        <div className="mb-8">
          <label className="block text-gray-400 text-xs font-bold uppercase tracking-widest mb-2">Storage Quota</label>
          <input
            type="range" min="1" max="500"
            value={quota}
            onChange={e => setQuota(Number(e.target.value))}
            className="w-full accent-[#ffd700]"
          />
          <div className="flex justify-between text-xs text-[#ffd700] font-mono mt-1.5">
            <span>1 GB</span><span>{quota} GB</span><span>500 GB</span>
          </div>
        </div>

        {error && <p className="text-red-400 text-xs font-mono mb-4">{error}</p>}

        <button
          onClick={handleInit}
          disabled={saving || !path}
          className="w-full py-3 rounded-lg bg-[#00f0ff]/10 border border-[#00f0ff]/40 text-[#00f0ff] font-bold hover:bg-[#00f0ff]/20 transition-all disabled:opacity-50"
        >
          {saving ? 'Initializing…' : 'Initialize Qualia'}
        </button>

        <p className="text-xs text-gray-600 text-center mt-4">
          After setup, go to <span className="text-[#b026ff]">Identifiers &amp; Credentials</span> to generate your DID root.
        </p>
      </div>
    </div>
  );
}

export default function App() {
  const [hwStats, setHwStats] = useState({ cpu: '0.0%', ram: '0.00 GB' });
  const [showSetup, setShowSetup] = useState(false);

  useEffect(() => {
    const unlisten = listen('hardware-telemetry', (event: any) => {
      setHwStats(event.payload);
    });
    invoke<boolean>('is_first_run')
      .then(first => { if (first) setShowSetup(true); })
      .catch(console.error);
    return () => {
      unlisten.then(f => f());
    };
  }, []);

  return (
    <Router>
      {showSetup && <SetupWizard onComplete={() => setShowSetup(false)} />}
      <div className="flex h-screen w-screen">
        <div className="grid-bg"></div>
        <Sidebar />
        <main className="flex-1 overflow-y-auto p-8 relative z-10 flex flex-col gap-6">
          <div className="bg-[#0a0a0f] border border-white/10 rounded-lg p-4 flex gap-8 text-sm font-mono text-gray-400 shadow-lg shrink-0 items-center">
            <div>Engine: <strong className="text-[#00f0ff]">WASM SANDBOX</strong></div>
            <div className="flex items-center gap-2"><Cpu className="w-4 h-4 text-[#ff9900]" /> Host CPU: <strong className="text-white">{hwStats.cpu}</strong></div>
            <div className="flex items-center gap-2"><MemoryStick className="w-4 h-4 text-[#00ff88]" /> Host RAM: <strong className="text-white">{hwStats.ram}</strong></div>
            <div className="ml-auto">DAG: <strong className="text-[#b026ff]">mediaprophet/qualiaDB</strong></div>
          </div>
          
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/credentials" element={<CredentialManager />} />
            <Route path="/addressbook" element={<AddressBook />} />
            <Route path="/chat" element={<Chat />} />
            <Route path="/llms" element={<LLMHub />} />
            <Route path="/ontologies" element={<OntologyHub />} />
            <Route path="/assets" element={<AssetLibrary />} />
            <Route path="/physics" element={<SpatialPhysics />} />
            <Route path="/apps" element={<AppStore />} />
            <Route path="/wallet" element={<Wallet />} />
            <Route path="/settings" element={<AppSettings />} />
            <Route path="*" element={<div className="glass-panel text-center text-gray-400">Page Under Construction</div>} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}
