import { useState } from 'react';
import { Users, UserPlus, Fingerprint, Copy, ShieldCheck, Mail } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';

export default function AddressBook() {
  const [frontDoorDid, setFrontDoorDid] = useState('');
  const [copied, setCopied] = useState(false);

  const handleGenerateFrontDoor = async () => {
    try {
      const did: string = await invoke('generate_front_door_invite');
      const emailTemplate = `Let's connect via Qualia Webizen.\n\nHere is my Front Door DID:\n${did}\n\nPaste this into your Addressbook to establish a semantic P2P link.`;
      setFrontDoorDid(emailTemplate);
      setCopied(false);
    } catch (e) {
      console.error(e);
    }
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(frontDoorDid);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const connections = [
    { name: "Satoshi (Alpha Node)", did: "did:qualia:0x49B2...", status: "Connected", ping: "24ms" },
    { name: "Mediaprophet Foundation", did: "did:qualia:0x88A1...", status: "Connected", ping: "14ms" },
    { name: "Wellfair Oracle", did: "did:qualia:0x99C4...", status: "Syncing", ping: "89ms" },
  ];

  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="grid grid-cols-2 gap-6 flex-1">
        
        {/* Left Col: Front Door Identifier */}
        <div className="glass-panel flex flex-col">
          <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white flex items-center gap-2">
            <UserPlus className="text-[#00f0ff]" /> Generate Front Door Invite
          </h2>
          
          <p className="text-sm text-gray-400 mb-6 font-mono">
            Generate an ephemeral connection DID to safely route social graphs and P2P WebTorrent seeds without exposing your Master Root Identifier.
          </p>

          <button 
            onClick={handleGenerateFrontDoor}
            className="w-full py-4 rounded-lg border border-[#00f0ff] text-white hover:bg-[#00f0ff]/10 hover:shadow-[0_0_15px_rgba(0,240,255,0.2)] transition-all font-bold tracking-widest uppercase flex items-center justify-center gap-2 mb-6 bg-black/40"
          >
            <Fingerprint className="w-5 h-5 text-[#00f0ff]" /> Generate Connect String
          </button>

          {frontDoorDid && (
            <div className="bg-black/60 border border-white/10 rounded-lg p-4 relative group flex-1 flex flex-col">
              <div className="flex justify-between items-center mb-2">
                <span className="text-xs text-gray-400 font-mono flex items-center gap-2 uppercase tracking-widest">
                  <Mail className="w-3 h-3" /> Email Template Ready
                </span>
                <button 
                  onClick={handleCopy}
                  className="text-xs bg-white/10 hover:bg-white/20 text-white px-3 py-1 rounded flex items-center gap-1 transition-all"
                >
                  <Copy className="w-3 h-3" /> {copied ? 'Copied!' : 'Copy'}
                </button>
              </div>
              <textarea 
                readOnly 
                value={frontDoorDid} 
                className="w-full flex-1 bg-transparent border-none outline-none text-[#00ff88] font-mono text-sm resize-none"
              />
            </div>
          )}
        </div>

        {/* Right Col: Social Graph */}
        <div className="glass-panel flex flex-col">
          <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white flex items-center gap-2">
            <Users className="text-[#b026ff]" /> Active Social Graph (FOAF)
          </h2>

          <div className="flex items-center gap-2 mb-4 bg-black/40 border border-white/5 rounded px-3 py-2">
            <UserPlus className="w-4 h-4 text-gray-500" />
            <input 
              type="text" 
              placeholder="Paste Front Door DID here to connect..." 
              className="bg-transparent border-none outline-none text-xs text-white w-full font-mono" 
            />
            <button className="bg-[#b026ff]/20 text-[#b026ff] px-3 py-1 rounded text-[10px] uppercase font-bold tracking-widest hover:bg-[#b026ff]/40">Connect</button>
          </div>

          <div className="flex-1 overflow-y-auto pr-2">
            <div className="flex flex-col gap-3">
              {connections.map((conn, i) => (
                <div key={i} className="bg-black/40 border border-white/5 rounded p-3 flex items-center justify-between hover:bg-white/5 transition-colors">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-full bg-gradient-to-br from-black to-white/10 border border-white/20 flex items-center justify-center">
                      <ShieldCheck className={`w-5 h-5 ${conn.status === 'Connected' ? 'text-[#00ff88]' : 'text-[#ff9900]'}`} />
                    </div>
                    <div>
                      <h3 className="font-bold text-sm text-white">{conn.name}</h3>
                      <div className="text-[10px] text-gray-500 font-mono mt-1">{conn.did}</div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className={`text-[10px] font-bold uppercase tracking-widest mb-1 ${conn.status === 'Connected' ? 'text-[#00ff88]' : 'text-[#ff9900] animate-pulse'}`}>
                      {conn.status}
                    </div>
                    <div className="text-xs font-mono text-gray-600">Ping: {conn.ping}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>

        </div>
      </div>
    </div>
  );
}
