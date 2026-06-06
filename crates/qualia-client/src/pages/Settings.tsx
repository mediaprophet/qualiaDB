import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

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

  useEffect(() => {
    invoke<AgentConfig>('get_config').then(setConfig).catch(console.error);
    invoke<TaxRecipientSuite>('get_tax_suite')
      .then(suite => { if (suite?.recipients?.length) setTaxRecipients(suite.recipients); })
      .catch(console.error);
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
