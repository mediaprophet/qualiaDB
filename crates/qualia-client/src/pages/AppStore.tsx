import { Package, ShieldCheck, Play, Key, Copy, Check } from 'lucide-react';
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface AppEntry {
  name: string;
  id: string;
  status: string;
  vc: string;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 1800); };
  return (
    <button onClick={copy} className="text-gray-600 hover:text-[#00f0ff] transition-colors ml-2" title="Copy">
      {copied ? <Check className="w-3 h-3 text-[#00ff88]" /> : <Copy className="w-3 h-3" />}
    </button>
  );
}

export default function AppStore() {
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [vcInput, setVcInput] = useState('');
  const [generatedVc, setGeneratedVc] = useState('');
  const [vcError, setVcError] = useState('');
  const [launching, setLaunching] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>('list_installed_apps')
      .then(names => setApps(names.map(n => ({ name: n, id: n, status: 'Installed', vc: 'Valid' }))))
      .catch(console.error);
  }, []);

  const handleLaunch = async (appName: string) => {
    setLaunching(appName);
    try {
      await invoke('launch_installed_app', { appName });
    } catch (e) {
      console.error('Launch failed:', e);
    } finally {
      setLaunching(null);
    }
  };

  const handleSignVc = async () => {
    if (!vcInput.trim()) return;
    setVcError('');
    setGeneratedVc('');
    try {
      const vc = await invoke<string>('generate_app_credential', { appName: vcInput.trim() });
      setGeneratedVc(vc);
    } catch (e: any) {
      setVcError(String(e));
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white">
          <Package className="text-[#00f0ff]" /> Local App Manager
        </h2>
        <p className="text-gray-400 mb-6">Install and manage third-party edge-native web apps. Apps are sandboxed and verified via VCs.</p>

        {apps.length === 0 ? (
          <div className="text-center text-gray-600 text-sm font-mono py-8">
            No apps installed — place app directories in your <span className="text-gray-400">Apps/</span> data folder.
          </div>
        ) : (
          <div className="grid gap-4">
            {apps.map(app => (
              <div key={app.id} className="bg-black/40 border border-white/5 rounded-xl p-4 flex items-center justify-between">
                <div>
                  <h3 className="font-bold text-lg text-white">{app.name}</h3>
                  <div className="text-xs text-gray-500 font-mono mt-1 flex gap-3">
                    <span className="flex items-center gap-1 text-[#00ff88]"><ShieldCheck className="w-3 h-3"/> VC: {app.vc}</span>
                    <span>ID: {app.id}</span>
                  </div>
                </div>
                <button
                  onClick={() => handleLaunch(app.id)}
                  disabled={launching === app.id}
                  className="bg-white/10 text-white hover:bg-[#00f0ff] hover:text-black transition-all px-4 py-2 rounded-lg font-semibold flex items-center gap-2 text-sm disabled:opacity-50"
                >
                  <Play className="w-4 h-4" /> {launching === app.id ? 'Launching…' : 'Launch'}
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="glass-panel border-[#ffd700]/30 border">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white">
          <Key className="text-[#ffd700]" /> Developer Credentials
        </h2>
        <p className="text-sm text-gray-400 mb-4">Generate Verifiable Credentials (VCs) to self-sign your own local applications before loading them into the daemon.</p>

        <div className="flex gap-4 mb-3">
          <input
            type="text"
            value={vcInput}
            onChange={e => { setVcInput(e.target.value); setGeneratedVc(''); setVcError(''); }}
            onKeyDown={e => e.key === 'Enter' && handleSignVc()}
            placeholder="App ID (e.g. com.my.app)"
            className="flex-1 bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#ffd700] font-mono text-sm"
          />
          <button
            onClick={handleSignVc}
            disabled={!vcInput.trim()}
            className="bg-[#ffd700]/10 text-[#ffd700] border border-[#ffd700]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#ffd700] hover:text-black transition-all disabled:opacity-40"
          >
            Sign &amp; Generate VC
          </button>
        </div>

        {vcError && (
          <div className="bg-red-500/10 border border-red-500/30 rounded px-3 py-2 text-xs text-red-400 font-mono">
            {vcError}
          </div>
        )}

        {generatedVc && (
          <div className="bg-[#00ff88]/5 border border-[#00ff88]/20 rounded-lg px-4 py-3 font-mono text-xs text-[#00ff88] flex items-center gap-2 break-all">
            <span className="flex-1">{generatedVc}</span>
            <CopyButton text={generatedVc} />
          </div>
        )}
      </div>
    </div>
  );
}
