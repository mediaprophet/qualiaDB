import { Key, Lock, Plus, Network, Download, ShieldCheck } from 'lucide-react';
import { useState, useEffect } from 'react';
import { invoke } from '../lib/tauri-compat';

interface ImportedAccount {
  network: string;
  label: string;
  address: string;
}

export default function CredentialManager() {
  const [seedPhrase, setSeedPhrase] = useState('');
  const [derivedWallets, setDerivedWallets] = useState<any>(null);
  const [identityLoaded, setIdentityLoaded] = useState(false);

  // Multi-Seed Account State
  const [showImportModal, setShowImportModal] = useState(false);
  const [importNetwork, setImportNetwork] = useState('eCash (XEC)');
  const [importSeed, setImportSeed] = useState('');
  const [importLabel, setImportLabel] = useState('');
  const [importedAccounts, setImportedAccounts] = useState<ImportedAccount[]>([]);

  useEffect(() => {
    invoke<any>('load_identity')
      .then(identity => {
        if (identity) {
          setDerivedWallets(identity);
          setIdentityLoaded(true);
        }
      })
      .catch(console.error);
    invoke<ImportedAccount[]>('load_imported_accounts')
      .then(accounts => { if (Array.isArray(accounts)) setImportedAccounts(accounts); })
      .catch(console.error);
  }, []);

  const handleGenerateSeed = async () => {
    try {
      const seed: string = await invoke('generate_bip39_seed');
      setSeedPhrase(seed);
      const wallets: any = await invoke('derive_wallets_from_seed', { seed });
      setDerivedWallets(wallets);
      await invoke('save_identity', { wallets });
      setIdentityLoaded(false); // seed is visible this session
    } catch (e) {
      console.error(e);
    }
  };

  const handleImportAccount = async () => {
    if (!importSeed || !importLabel) return;
    try {
      const address: string = await invoke('import_external_seed', {
        network: importNetwork,
        seed: importSeed,
        label: importLabel,
      });
      const updated = [...importedAccounts, { network: importNetwork, label: importLabel, address }];
      setImportedAccounts(updated);
      await invoke('save_imported_accounts', { accounts: updated }).catch(console.error);
      setShowImportModal(false);
      setImportSeed('');
      setImportLabel('');
    } catch (e) {
      console.error(e);
      alert("Invalid seed phrase provided.");
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full relative">
      <div className="flex justify-between items-center mb-2">
        <h1 className="text-2xl font-bold text-white flex items-center gap-2">
          <Key className="text-[#b026ff]" /> Identifiers & Credentials
        </h1>
        <button 
          onClick={() => setShowImportModal(true)}
          className="flex items-center gap-2 bg-[#00f0ff]/10 hover:bg-[#00f0ff]/20 text-[#00f0ff] border border-[#00f0ff]/30 px-4 py-2 rounded-lg font-bold transition-all"
        >
          <Plus className="w-4 h-4" /> Add External Account
        </button>
      </div>

      <div className="glass-panel border-[#b026ff]/30">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white">
          Master DID Root (BIP39 Credentials)
        </h2>
        
        <p className="text-sm text-gray-400 mb-6 font-mono">
          Your 12-word seed phrase is the cryptographic root of your identity. It deterministically derives your default decentralized identifiers (DIDs) across the Webizen topology.
        </p>
        
        {identityLoaded && !seedPhrase ? (
          <div className="bg-black/50 border border-[#00ff88]/30 rounded-lg p-4 flex items-center gap-3">
            <ShieldCheck className="w-5 h-5 text-[#00ff88] shrink-0" />
            <div>
              <div className="text-[#00ff88] font-bold text-sm">Identity Active</div>
              <div className="text-gray-500 text-xs mt-0.5">Seed phrase was shown at creation — back it up offline. Derived addresses are restored below.</div>
            </div>
            <button
              onClick={handleGenerateSeed}
              className="ml-auto text-xs text-gray-500 hover:text-white border border-white/10 px-3 py-1.5 rounded transition-colors"
            >
              Regenerate
            </button>
          </div>
        ) : !seedPhrase ? (
          <div className="flex items-center gap-4">
            <button
              onClick={handleGenerateSeed}
              className="px-6 py-3 rounded-lg border border-[#00f0ff] text-white hover:shadow-[0_0_15px_rgba(0,240,255,0.4)] transition-all font-semibold bg-white/5"
            >
              Generate 12-Word Seed (BIP39)
            </button>
          </div>
        ) : (
          <div className="bg-black/50 border border-white/10 rounded-lg p-4">
            <div className="flex justify-between items-center mb-2">
              <span className="text-gray-400 text-sm font-mono uppercase tracking-widest flex items-center gap-2">
                <Lock className="w-3 h-3 text-[#00ff88]" /> Write this down — not stored on disk
              </span>
            </div>
            <div className="font-mono text-[#00f0ff] text-lg bg-black p-3 rounded border border-white/5 tracking-wider select-all">
              {seedPhrase}
            </div>
            <p className="text-xs text-yellow-500/70 mt-2">Your derived addresses have been saved. The seed phrase above will not be shown again after you leave this page.</p>
          </div>
        )}
      </div>

      {derivedWallets && (
        <div className="grid grid-cols-2 gap-4">
          <div className="glass-panel bg-gradient-to-br from-black to-[#00f0ff]/5 border-[#00f0ff]/20 border p-4">
            <h3 className="text-sm font-bold text-white mb-2 flex justify-between">Qualia Root <span>[Default]</span></h3>
            <div className="font-mono text-xs text-[#00f0ff] bg-black/40 p-2 rounded truncate">{derivedWallets.qualia_root}</div>
          </div>
          
          <div className="glass-panel bg-gradient-to-br from-black to-[#b026ff]/5 border-[#b026ff]/20 border p-4">
            <h3 className="text-sm font-bold text-white mb-2 flex justify-between">Nym Mixnet (Nyx) <span>[Default]</span></h3>
            <div className="font-mono text-xs text-[#b026ff] bg-black/40 p-2 rounded truncate">{derivedWallets.nym_mixnet}</div>
          </div>

          <div className="glass-panel bg-gradient-to-br from-black to-[#00ff88]/5 border-[#00ff88]/20 border p-4">
            <h3 className="text-sm font-bold text-white mb-2 flex justify-between">eCash (XEC) <span>[Default]</span></h3>
            <div className="font-mono text-xs text-[#00ff88] bg-black/40 p-2 rounded truncate">{derivedWallets.ecash_xec}</div>
          </div>

          <div className="glass-panel bg-gradient-to-br from-black to-gray-500/5 border-gray-500/20 border p-4">
            <h3 className="text-sm font-bold text-white mb-2 flex justify-between">Ethereum (EVM) <span>[Default]</span></h3>
            <div className="font-mono text-xs text-gray-400 bg-black/40 p-2 rounded truncate">{derivedWallets.ethereum}</div>
          </div>
        </div>
      )}

      {importedAccounts.length > 0 && (
        <div className="mt-4">
          <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white">Imported External Accounts</h2>
          <div className="grid grid-cols-2 gap-4">
            {importedAccounts.map((acc, i) => (
              <div key={i} className="glass-panel bg-gradient-to-br from-black to-blue-500/5 border-blue-500/20 border p-4">
                <h3 className="text-sm font-bold text-white mb-2 flex items-center justify-between">
                  <span className="flex items-center gap-2"><Network className="w-4 h-4 text-blue-400"/> {acc.network}</span>
                  <span className="text-xs bg-blue-500/20 text-blue-300 px-2 py-0.5 rounded uppercase font-bold">{acc.label}</span>
                </h3>
                <div className="font-mono text-xs text-blue-300 bg-black/40 p-2 rounded truncate select-all">{acc.address}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Import Account Modal */}
      {showImportModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-lg border-[#00f0ff]/30 shadow-[0_0_50px_rgba(0,240,255,0.1)]">
            <h2 className="text-xl font-bold mb-6 text-white flex items-center gap-2">
              <Download className="text-[#00f0ff] w-5 h-5" /> Import External Seed Phrase
            </h2>
            
            <div className="flex flex-col gap-4 mb-6">
              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Network</label>
                <select 
                  className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none"
                  value={importNetwork}
                  onChange={e => setImportNetwork(e.target.value)}
                >
                  <option value="eCash (XEC)">eCash (XEC)</option>
                  <option value="Bitcoin (BTC)">Bitcoin (BTC)</option>
                  <option value="Nym (NYM) - Nyx Chain">Nym (NYM) - Nyx Chain</option>
                  <option value="Monero (XMR)">Monero (XMR)</option>
                  <option value="Ethereum (EVM)">Ethereum (EVM)</option>
                </select>
              </div>
              
              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Account Label</label>
                <input 
                  type="text" 
                  value={importLabel}
                  onChange={e => setImportLabel(e.target.value)}
                  className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none" 
                  placeholder="e.g. Trading Wallet, Cold Storage..." 
                />
              </div>

              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">12/24 Word Seed Phrase</label>
                <textarea 
                  rows={3}
                  value={importSeed}
                  onChange={e => setImportSeed(e.target.value)}
                  className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-[#00ff88] outline-none select-all" 
                  placeholder="abandon ability able about..." 
                />
                <div className="text-[10px] text-gray-500 mt-1 uppercase tracking-wider text-right">Secured in OS Keychain upon import</div>
              </div>
            </div>

            <div className="flex justify-end gap-3">
              <button onClick={() => setShowImportModal(false)} className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">Cancel</button>
              <button 
                onClick={handleImportAccount} 
                className="px-6 py-2 bg-[#00f0ff]/20 text-[#00f0ff] hover:bg-[#00f0ff]/40 border border-[#00f0ff]/30 rounded font-bold transition-all disabled:opacity-50"
                disabled={!importSeed || !importLabel}
              >
                Import Account
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
