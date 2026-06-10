import { useState, useEffect } from 'react';
import { invoke } from '../lib/tauri-compat';
import LogViewer from '../components/LogViewer';
import { Activity, Cpu, Lock, Unlock } from 'lucide-react';

interface AgentConfig {
  storage_path: string;
  storage_quota_gb: number;
  base_connectivity_cost_ilp: number;
}

interface TaxRecipient {
  label: string;
  ilp_address: string;
  share_percent: number;
  use_nym: boolean;
}

interface TaxRecipientSuite {
  jurisdiction_did: string;
  recipients: TaxRecipient[];
}

interface QpuSettings {
  feature_unlocked: boolean;
  ibm_token_configured: boolean;
  dwave_token_configured: boolean;
  max_shots_per_task: number;
  fallback_to_classical: boolean;
  enable_qubo_routing: boolean;
  enable_dft_ground_state: boolean;
  enable_defeasible_resolution: boolean;
  ibm_quota_minutes_remaining: number;
  dwave_quota_minutes_remaining: number;
}

const FALLBACK_RECIPIENTS: TaxRecipient[] = [
  { label: 'Cooperative Infrastructure Fund',  ilp_address: '$ilp.qualia.coop/infrastructure',   share_percent: 40, use_nym: false },
  { label: 'Digital Rights Legal Defence',      ilp_address: '$ilp.qualia.coop/legal-defence',    share_percent: 30, use_nym: false },
  { label: 'Open Source Sustainability Pool',   ilp_address: '$ilp.qualia.coop/oss-sustainability', share_percent: 20, use_nym: false },
  { label: 'Disaster Recovery Reserve',         ilp_address: '$ilp.qualia.coop/disaster-reserve', share_percent: 10, use_nym: true  },
];

export default function Settings() {
  const [config, setConfig] = useState<AgentConfig>({
    storage_path: '',
    storage_quota_gb: 50,
    base_connectivity_cost_ilp: 5000,
  });
  const [status, setStatus] = useState('');
  const [taxRecipients, setTaxRecipients] = useState<TaxRecipient[]>(FALLBACK_RECIPIENTS);
  const [showLogs, setShowLogs] = useState(false);
  const [qpuSettings, setQpuSettings] = useState<QpuSettings | null>(null);
  const [showQpuSettings, setShowQpuSettings] = useState(false);
  const [qpuTokenInput, setQpuTokenInput] = useState({ ibm: '', dwave: '' });

  useEffect(() => {
    invoke<AgentConfig>('get_config').then(setConfig).catch(console.error);
    invoke<TaxRecipientSuite>('get_tax_suite')
      .then(suite => { if (suite?.recipients?.length) setTaxRecipients(suite.recipients); })
      .catch(console.error);
    invoke<QpuSettings>('get_qpu_settings').then(setQpuSettings).catch(console.error);
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

  const handleQpuSave = async () => {
    try {
      await invoke('save_qpu_settings', {
        max_shots_per_task: qpuSettings?.max_shots_per_task || 1000,
        fallback_to_classical: qpuSettings?.fallback_to_classical ?? true,
        enable_qubo_routing: qpuSettings?.enable_qubo_routing ?? true,
        enable_dft_ground_state: qpuSettings?.enable_dft_ground_state ?? true,
        enable_defeasible_resolution: qpuSettings?.enable_defeasible_resolution ?? false,
        ibm_token: qpuTokenInput.ibm || undefined,
        dwave_token: qpuTokenInput.dwave || undefined,
      });
      setStatus('QPU settings saved.');
      setTimeout(() => setStatus(''), 3000);
      setQpuTokenInput({ ibm: '', dwave: '' });
      invoke<QpuSettings>('get_qpu_settings').then(setQpuSettings).catch(console.error);
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
  };

  const handleToggleQpu = async () => {
    try {
      if (qpuSettings?.feature_unlocked) {
        await invoke('disable_qpu_feature');
      } else {
        await invoke('enable_qpu_feature');
      }
      invoke<QpuSettings>('get_qpu_settings').then(setQpuSettings).catch(console.error);
      setStatus('QPU feature toggled.');
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
        
        <div className="mt-4 pt-4 border-t border-white/10 flex gap-4">
          <button
            onClick={() => setShowLogs(!showLogs)}
            className="flex-1 bg-[#00f0ff]/10 text-[#00f0ff] border border-[#00f0ff]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#00f0ff]/20 transition-all flex items-center justify-center gap-2"
          >
            <Activity className="w-4 h-4" />
            {showLogs ? 'Hide System Logs' : 'View System Logs'}
          </button>
          <button
            onClick={() => setShowQpuSettings(!showQpuSettings)}
            className="flex-1 bg-[#b026ff]/10 text-[#b026ff] border border-[#b026ff]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#b026ff]/20 transition-all flex items-center justify-center gap-2"
          >
            <Cpu className="w-4 h-4" />
            QPU Configuration
          </button>
        </div>
      </div>

      {showLogs && (
        <div className="glass-panel">
          <LogViewer embedded={true} />
        </div>
      )}

      {showQpuSettings && (
        <div className="glass-panel">
          <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-6 text-white flex items-center gap-2">
            <Cpu className="w-5 h-5 text-[#b026ff]" />
            Quantum Processing Unit (QPU) Configuration
          </h2>

          <div className="mb-6 p-4 bg-[#b026ff]/5 border border-[#b026ff]/20 rounded-lg">
            <div className="flex items-center justify-between mb-4">
              <div className="flex items-center gap-2">
                {qpuSettings?.feature_unlocked ? (
                  <Unlock className="w-5 h-5 text-[#00ff88]" />
                ) : (
                  <Lock className="w-5 h-5 text-gray-500" />
                )}
                <span className="text-white font-semibold">
                  QPU Feature: {qpuSettings?.feature_unlocked ? 'Unlocked' : 'Locked'}
                </span>
              </div>
              <button
                onClick={handleToggleQpu}
                className={`px-4 py-2 rounded-lg font-bold transition-all ${
                  qpuSettings?.feature_unlocked
                    ? 'bg-red-500/20 text-red-400 border border-red-500/30 hover:bg-red-500/30'
                    : 'bg-[#00ff88]/10 text-[#00ff88] border border-[#00ff88]/30 hover:bg-[#00ff88]/20'
                }`}
              >
                {qpuSettings?.feature_unlocked ? 'Lock' : 'Unlock'}
              </button>
            </div>
            <p className="text-xs text-gray-400">
              Unlock to enable quantum offload for NP-hard optimization problems. Requires API tokens for IBM Quantum or D-Wave.
            </p>
          </div>

          {qpuSettings?.feature_unlocked && (
            <div className="space-y-6">
              <div className="grid grid-cols-2 gap-4">
                <div className="bg-black/30 border border-white/5 rounded-lg p-4">
                  <h3 className="text-sm font-bold text-[#00f0ff] mb-2">IBM Quantum</h3>
                  <div className="text-xs text-gray-400 mb-2">
                    Status: {qpuSettings.ibm_token_configured ? '✓ Configured' : 'Not configured'}
                  </div>
                  <div className="text-xs text-gray-400 mb-2">
                    Quota: {qpuSettings.ibm_quota_minutes_remaining.toFixed(1)} / {10.0} min remaining
                  </div>
                  <input
                    type="password"
                    placeholder="IBM Quantum API Token"
                    value={qpuTokenInput.ibm}
                    onChange={e => setQpuTokenInput({ ...qpuTokenInput, ibm: e.target.value })}
                    className="w-full bg-black/50 border border-white/20 rounded px-3 py-2 text-white text-xs font-mono"
                  />
                </div>

                <div className="bg-black/30 border border-white/5 rounded-lg p-4">
                  <h3 className="text-sm font-bold text-[#ffd700] mb-2">D-Wave</h3>
                  <div className="text-xs text-gray-400 mb-2">
                    Status: {qpuSettings.dwave_token_configured ? '✓ Configured' : 'Not configured'}
                  </div>
                  <div className="text-xs text-gray-400 mb-2">
                    Quota: {qpuSettings.dwave_quota_minutes_remaining.toFixed(1)} / {1.0} min remaining
                  </div>
                  <input
                    type="password"
                    placeholder="D-Wave API Token"
                    value={qpuTokenInput.dwave}
                    onChange={e => setQpuTokenInput({ ...qpuTokenInput, dwave: e.target.value })}
                    className="w-full bg-black/50 border border-white/20 rounded px-3 py-2 text-white text-xs font-mono"
                  />
                </div>
              </div>

              <div>
                <label className="block text-gray-400 text-sm mb-2">Max Shots Per Task (1-1000)</label>
                <input
                  type="number" min="1" max="1000"
                  value={qpuSettings?.max_shots_per_task}
                  onChange={e => setQpuSettings({ ...qpuSettings!, max_shots_per_task: Number(e.target.value) })}
                  className="w-full bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#b026ff] font-mono text-sm"
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={qpuSettings?.fallback_to_classical}
                    onChange={e => setQpuSettings({ ...qpuSettings!, fallback_to_classical: e.target.checked })}
                    className="accent-[#b026ff]"
                  />
                  Fallback to Classical
                </label>
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={qpuSettings?.enable_qubo_routing}
                    onChange={e => setQpuSettings({ ...qpuSettings!, enable_qubo_routing: e.target.checked })}
                    className="accent-[#b026ff]"
                  />
                  Enable QUBO Routing
                </label>
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={qpuSettings?.enable_dft_ground_state}
                    onChange={e => setQpuSettings({ ...qpuSettings!, enable_dft_ground_state: e.target.checked })}
                    className="accent-[#b026ff]"
                  />
                  Enable DFT Ground State
                </label>
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={qpuSettings?.enable_defeasible_resolution}
                    onChange={e => setQpuSettings({ ...qpuSettings!, enable_defeasible_resolution: e.target.checked })}
                    className="accent-[#b026ff]"
                  />
                  Defeasible Resolution
                </label>
              </div>

              <button
                onClick={handleQpuSave}
                className="bg-[#b026ff]/10 text-[#b026ff] border border-[#b026ff]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#b026ff]/20 transition-all"
              >
                Save QPU Settings
              </button>
            </div>
          )}
        </div>
      )}

      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-6 text-white">12% TAX ROUTER — RECIPIENT SUITE</h2>
        <div className="text-sm text-gray-400 mb-6">Every accepted ILP payment is automatically split: 12% is dispatched as micropayments to the addresses below.</div>

        <div className="bg-black/30 border border-white/5 rounded-lg p-4 font-mono text-sm text-gray-300">
          {taxRecipients.map((r, i) => (
            <div
              key={i}
              className={`flex justify-between ${i < taxRecipients.length - 1 ? 'border-b border-white/5 pb-2 mb-2' : ''}`}
            >
              <span className="flex items-center gap-2">
                {r.label}
                {r.use_nym && (
                  <span className="text-[9px] bg-[#00f0ff]/10 text-[#00f0ff] border border-[#00f0ff]/20 px-1 rounded">NYM</span>
                )}
              </span>
              <span className="text-[#ffd700]">{r.share_percent}%</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}