import { useState, useEffect } from 'react';
import { Image as ImageIcon, Search, Filter, Hash, Network, Upload, ShieldCheck, Share2, Activity, BrainCircuit } from 'lucide-react';
import { invoke, listen } from '../lib/tauri-compat';


export default function AssetLibrary() {
  const [pipelineState, setPipelineState] = useState<'idle' | 'analyzing'>('idle');
  const [swarmStats, setSwarmStats] = useState({ seeders: 0, leechers: 0, speed: '0 KB/s' });
  const [typologyLens, setTypologyLens] = useState<'Generic' | 'Meme' | 'Heraldry'>('Generic');
  
  const [assets, setAssets] = useState([
    { id: '0x8F3BC122', type: 'Heraldry', facet: 'Lion Rampant | Or on Gules', origin: '14th Century', region: 'xywh=120,40,200,200', magnet: 'magnet:?xt=urn:btih:3f8a...', alpTokenId: 'alp:0x1A2...', isGhost: false },
    { id: '0x9E4CD233', type: 'Meme', facet: 'Distracted Boyfriend | Irony Tensor: 0.8', origin: '2015', region: 'xywh=0,0,1024,768', magnet: 'magnet:?xt=urn:btih:4a9b...', alpTokenId: null, isGhost: false },
    { id: '0xA15DE344', type: 'Hieroglyph', facet: 'Eye of Horus | Wedjat', origin: 'Ptolemaic', region: 'xywh=450,300,50,50', magnet: 'magnet:?xt=urn:btih:5b0c...', alpTokenId: 'alp:0x9B4...', isGhost: false }
  ]);

  // Handle true asynchronous LLaVA ingestion event from Rust
  useEffect(() => {
    const unlisten = listen('ingestion-complete', (event: any) => {
      const payload = event.payload;
      setAssets(prev => prev.map(a => 
        a.id === 'ghost-node' 
          ? { 
              id: payload.lexicon_id, 
              type: payload.type, 
              facet: payload.facet, 
              origin: payload.origin, 
              region: payload.region, 
              magnet: payload.magnet_uri,
              alpTokenId: null, 
              isGhost: false 
            } 
          : a
      ));
    });
    return () => {
      unlisten.then(f => f());
    };
  }, []);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const stats: any = await invoke('fetch_torrent_telemetry');
        setSwarmStats(stats);
      } catch (e) {}
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleDragDrop = async () => {
    if (pipelineState !== 'idle') return;
    
    // 1. Optimistic UI: Inject the Ghost Card immediately to free the JS Thread
    setAssets(prev => [{
      id: 'ghost-node',
      type: typologyLens === 'Generic' ? 'Analyzing...' : `Extracting ${typologyLens} Facets...`,
      facet: 'Pending Vision Extraction',
      origin: 'Unknown',
      region: 'Pending...',
      magnet: 'Initializing local seed...',
      alpTokenId: null,
      isGhost: true
    }, ...prev]);

    setPipelineState('analyzing');
    
    // 2. Fire the asynchronous native Tauri command and forget
    try {
      await invoke('ingest_image_async', { filePath: 'mock/path/to/media.mp4', typology: typologyLens });
      setPipelineState('idle');
    } catch (e) {
      console.error(e);
      setPipelineState('idle');
    }
  };

  const handleMintALP = async (id: string) => {
    try {
      const tokenId: string = await invoke('mint_semantic_token', { assetId: id });
      setAssets(prev => prev.map(a => a.id === id ? { ...a, alpTokenId: tokenId } : a));
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="flex flex-col gap-6 h-full">
      {/* Top Telemetry Bar */}
      <div className="flex gap-4">
        <div className="glass-panel flex-1 flex items-center gap-4 py-3">
          <Network className="text-[#00f0ff]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Active Seeders</div>
            <div className="text-xl font-bold text-white font-mono">{swarmStats.seeders} Nodes</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-4 py-3 border-[#00ff88]/30">
          <Share2 className="text-[#00ff88]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Swarm Tx Speed</div>
            <div className="text-xl font-bold text-white font-mono">{swarmStats.speed}</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-4 py-3 border-[#b026ff]/30">
          <Hash className="text-[#b026ff]" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Vectorized Assets</div>
            <div className="text-xl font-bold text-white font-mono">{assets.length} Indexed</div>
          </div>
        </div>
      </div>

      <div className="flex gap-6 flex-1 min-h-0">
        
        {/* Left Viewport: Ingestion, Filter Matrix, Swarm Graph */}
        <div className="flex-[1] flex flex-col gap-6 overflow-y-auto pr-2">
          {/* Dropzone with Typology Router */}
          <div className="glass-panel flex flex-col gap-4">
            
            <div className="flex items-center justify-between border-b border-white/10 pb-2">
               <h2 className="text-sm font-bold flex items-center gap-2 text-white font-mono">
                <BrainCircuit className="text-[#ff9900] w-4 h-4" /> Semantic Typology Lens
              </h2>
              <select 
                value={typologyLens} 
                onChange={(e) => setTypologyLens(e.target.value as any)}
                className="bg-black/50 border border-[#ff9900]/30 text-[#ff9900] text-xs font-bold rounded px-2 py-1 outline-none"
              >
                <option value="Generic">Generic Vector</option>
                <option value="Meme">Meme Engine</option>
                <option value="Heraldry">Heraldry Engine</option>
              </select>
            </div>

            <div 
              onClick={handleDragDrop}
              className={`border-2 border-dashed rounded-lg flex flex-col items-center justify-center p-6 cursor-pointer transition-all ${pipelineState !== 'idle' ? 'border-[#ff9900]/50 bg-[#ff9900]/5' : 'border-[#ff9900]/20 hover:border-[#ff9900]/50 hover:bg-[#ff9900]/5'}`}
            >
            {pipelineState === 'idle' ? (
              <>
                <Upload className="w-8 h-8 text-gray-400 mb-3" />
                <h3 className="text-sm font-bold text-white mb-1">Ingest Multimodal Asset</h3>
                <p className="text-[10px] text-gray-500 text-center">Drop Image/Video for Native Rust Analysis.</p>
              </>
            ) : (
              <div className="flex flex-col items-center">
                <Activity className="w-8 h-8 text-[#ff9900] animate-pulse mb-3" />
                <h3 className="text-[10px] font-bold text-[#ff9900] uppercase tracking-widest text-center">
                  Native Swarm: LLaVA Extraction in Progress...
                </h3>
              </div>
            )}
          </div>
        </div>

          {/* 2D Swarm Topology Graph */}
          <div className="glass-panel flex-1 flex flex-col relative overflow-hidden min-h-[200px]">
             <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white font-mono z-10">
              <Network className="text-[#00f0ff] w-4 h-4" /> 2D Swarm Topology
            </h2>
            <div className="absolute inset-0 flex items-center justify-center mt-8">
              {/* Lightweight SVG Graph */}
              <svg className="w-full h-full opacity-60" viewBox="0 0 200 200">
                <circle cx="100" cy="100" r="40" fill="none" stroke="#00f0ff" strokeWidth="1" strokeDasharray="4 4" className="animate-[spin_10s_linear_infinite]" />
                <circle cx="100" cy="100" r="80" fill="none" stroke="#b026ff" strokeWidth="1" strokeDasharray="2 6" className="animate-[spin_20s_linear_infinite_reverse]" />
                <circle cx="100" cy="100" r="6" fill="#00ff88" className="animate-pulse" />
                <line x1="100" y1="100" x2="135" y2="80" stroke="#00f0ff" strokeWidth="1" />
                <circle cx="135" cy="80" r="3" fill="#00f0ff" />
                <line x1="100" y1="100" x2="60" y2="120" stroke="#b026ff" strokeWidth="1" />
                <circle cx="60" cy="120" r="3" fill="#b026ff" />
                <line x1="100" y1="100" x2="110" y2="170" stroke="#00f0ff" strokeWidth="1" />
                <circle cx="110" cy="170" r="3" fill="#00f0ff" />
              </svg>
            </div>
          </div>
          
          {/* Faceted Search Matrix */}
          <div className="glass-panel flex flex-col">
            <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-4 flex items-center gap-2 text-white font-mono">
              <Filter className="text-[#b026ff] w-4 h-4" /> Faceted Query (SPARQL-MM)
            </h2>
            <div className="flex flex-col gap-3">
              <div className="flex items-center gap-2 bg-black/40 border border-white/5 rounded px-3 py-2">
                <Search className="w-4 h-4 text-gray-500" />
                <input type="text" placeholder="Region, Timecode, Semantics" className="bg-transparent border-none outline-none text-xs text-white w-full font-mono" />
              </div>
            </div>
          </div>
        </div>

        {/* Right Viewport: Semantic Asset Grid */}
        <div className="glass-panel flex-[2] flex flex-col relative overflow-hidden">
          <h2 className="text-sm font-bold border-b border-white/10 pb-2 mb-4 flex items-center justify-between text-white font-mono">
            <div className="flex items-center gap-2">
              <ImageIcon className="text-[#00ff88] w-4 h-4" /> Multimodal Library
            </div>
          </h2>
          
          <div className="flex-1 overflow-y-auto pr-2">
            <div className="flex flex-col gap-3">
              {assets.map((asset, i) => (
                <div key={i} className={`bg-black/60 border rounded p-3 flex items-center justify-between transition-colors group ${asset.isGhost ? 'border-[#ff9900] shadow-[0_0_10px_rgba(255,153,0,0.2)] animate-pulse' : 'border-white/5 hover:bg-white/5'}`}>
                  <div className="flex items-center gap-4">
                    <div className={`w-12 h-12 bg-white/5 rounded flex items-center justify-center border ${asset.isGhost ? 'border-[#ff9900]' : 'border-white/10 group-hover:border-[#00f0ff]/50'}`}>
                      {asset.isGhost ? <Activity className="w-5 h-5 text-[#ff9900]" /> : <ImageIcon className="w-5 h-5 text-gray-500 group-hover:text-[#00f0ff]" />}
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className={`font-bold text-sm font-mono ${asset.isGhost ? 'text-[#ff9900]' : 'text-white'}`}>
                          {asset.id}
                        </h3>
                        {!asset.isGhost && <ShieldCheck className="w-3 h-3 text-[#00ff88]" />}
                      </div>
                      <div className="text-[10px] text-gray-500 font-mono mt-1 flex flex-wrap gap-2">
                        <span className="text-[#b026ff]">{asset.type}</span> • 
                        <span className="text-[#00f0ff] truncate max-w-[150px]">Facet: {asset.facet}</span> • 
                        <span className="text-[#ff9900]">Region: {asset.region}</span> • 
                        <span>Era: {asset.origin}</span>
                      </div>
                    </div>
                  </div>
                  <div className="text-right">
                    <div className="text-[10px] text-gray-600 font-mono truncate w-32 mb-1" title={asset.magnet}>
                      {asset.magnet}
                    </div>
                    {!asset.isGhost && (
                      <div className="flex gap-2">
                        {!asset.alpTokenId ? (
                          <button 
                            onClick={() => handleMintALP(asset.id)}
                            className="text-[10px] bg-[#ffd700]/10 text-[#ffd700] px-2 py-1 rounded font-bold uppercase tracking-widest hover:bg-[#ffd700]/20 transition-colors"
                          >
                            Mint ALP
                          </button>
                        ) : (
                          <div className="text-[10px] bg-[#00ff88]/10 text-[#00ff88] px-2 py-1 rounded font-bold font-mono border border-[#00ff88]/30">
                            {asset.alpTokenId}
                          </div>
                        )}
                        <button className="text-[10px] bg-[#00f0ff]/10 text-[#00f0ff] px-2 py-1 rounded font-bold uppercase tracking-widest hover:bg-[#00f0ff]/20 transition-colors">
                          P2P Fetch
                        </button>
                      </div>
                    )}
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
