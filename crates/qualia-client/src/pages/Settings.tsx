import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

interface AgentConfig {
  storage_path: string;
  storage_quota_gb: number;
  base_connectivity_cost_ilp: number;
}

export default function Settings() {
  const [config, setConfig] = useState<AgentConfig>({
    storage_path: '',
    storage_quota_gb: 50,
    base_connectivity_cost_ilp: 5000,
  });
  const [status, setStatus] = useState('');

  useEffect(() => {
    invoke<AgentConfig>('get_config').then(setConfig).catch(console.error);
  }, []);

  const handleSave = async () => {
    try {
      await invoke('save_config', { newConfig: config });
      setStatus('Configuration saved.');
      setTimeout(() => setStatus(''), 3000);
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-6 text-white">System Configuration</h2>

        <div className="mb-6">
          <label className="block text-gray-400 text-sm mb-2">Data Storage Path</label>
          <input
            type="text"
            value={config.storage_path}
            onChange={e => setConfig({ ...config, storage_path: e.target.value })}
            className="w-full bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#00f0ff] font-mono text-sm"
          />
          <p className="text-xs text-gray-500 mt-2">Models, ontologies, and vector databases will be stored here.</p>
        </div>

        <div className="mb-6">
          <label className="block text-gray-400 text-sm mb-2">Storage Quota (GB)</label>
          <input
            type="range" min="1" max="500"
            value={config.storage_quota_gb}
            onChange={e => setConfig({ ...config, storage_quota_gb: Number(e.target.value) })}
            className="w-full accent-[#ffd700]"
          />
          <div className="flex justify-between text-xs text-[#ffd700] font-mono mt-2">
            <span>1 GB</span>
            <span>{config.storage_quota_gb} GB Selected</span>
            <span>500 GB</span>
          </div>
        </div>

        {status && (
          <div className={`text-sm font-mono mb-4 ${status.startsWith('Error') ? 'text-red-400' : 'text-[#00ff88]'}`}>
            {status}
          </div>
        )}

        <button
          onClick={handleSave}
          className="bg-[#00ff88]/10 text-[#00ff88] border border-[#00ff88]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#00ff88] hover:text-black transition-all"
        >
          Save Configuration
        </button>
      </div>

      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-6 text-white">12% TAX ROUTER — RECIPIENT SUITE</h2>
        <div className="text-sm text-gray-400 mb-6">Every accepted ILP payment is automatically split: 12% is dispatched as micropayments to the addresses below.</div>

        <div className="bg-black/30 border border-white/5 rounded-lg p-4 font-mono text-sm text-gray-300">
          <div className="flex justify-between border-b border-white/5 pb-2 mb-2">
            <span>Cooperative Infrastructure Fund</span>
            <span className="text-[#ffd700]">40%</span>
          </div>
          <div className="flex justify-between border-b border-white/5 pb-2 mb-2">
            <span>Digital Rights Legal Defence</span>
            <span className="text-[#ffd700]">30%</span>
          </div>
          <div className="flex justify-between border-b border-white/5 pb-2 mb-2">
            <span>Open Source Sustainability Pool</span>
            <span className="text-[#ffd700]">20%</span>
          </div>
          <div className="flex justify-between">
            <span>Disaster Recovery Reserve</span>
            <span className="text-[#ffd700]">10%</span>
          </div>
        </div>
      </div>
    </div>
  );
}
