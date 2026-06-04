import { Package, ShieldCheck, Play, Key } from 'lucide-react';
import { useState } from 'react';

export default function AppStore() {
  const [apps] = useState([
    { name: "Federated Node Monitor", id: "fed-monitor", status: "Installed", vc: "Valid" },
    { name: "Tax Oracle Dashboard", id: "tax-oracle", status: "Installed", vc: "Valid" },
  ]);

  return (
    <div className="flex flex-col gap-6">
      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white">
          <Package className="text-[#00f0ff]" /> Local App Manager
        </h2>
        <p className="text-gray-400 mb-6">Install and manage third-party edge-native web apps. Apps are sandboxed and verified via VCs.</p>

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
              <button className="bg-white/10 text-white hover:bg-[#00f0ff] hover:text-black transition-all px-4 py-2 rounded-lg font-semibold flex items-center gap-2 text-sm">
                <Play className="w-4 h-4" /> Launch
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="glass-panel border-[#ffd700]/30 border">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white">
          <Key className="text-[#ffd700]" /> Developer Credentials
        </h2>
        <p className="text-sm text-gray-400 mb-4">Generate Verifiable Credentials (VCs) to self-sign your own local applications before loading them into the daemon.</p>
        
        <div className="flex gap-4">
          <input type="text" placeholder="App ID (e.g. com.my.app)" className="flex-1 bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#ffd700] font-mono text-sm" />
          <button className="bg-[#ffd700]/10 text-[#ffd700] border border-[#ffd700]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#ffd700] hover:text-black transition-all">Sign & Generate VC</button>
        </div>
      </div>
    </div>
  );
}
