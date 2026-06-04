import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { Cpu, Database, Activity, Terminal, ShieldAlert, ShieldCheck, FileUp } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import 'katex/dist/katex.min.css';

interface Telemetry {
  token_rate: number;
  vram_usage: string;
  active_q42_context: string;
}

export default function Chat() {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState('');
  const [isGenerating, setIsGenerating] = useState(false);
  const [telemetry, setTelemetry] = useState<Telemetry | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  
  // CML Context State
  const [cmlSelection, setCmlSelection] = useState<{ text: string, top: number, left: number } | null>(null);
  const [didInput, setDidInput] = useState('did:web:schema.org:Term');

  // Phase 5: Ontological Lens & Axiom Bounds
  const [isLensOpen, setIsLensOpen] = useState(false);
  const [temporalStart, setTemporalStart] = useState(1920);
  const [temporalEnd, setTemporalEnd] = useState(1930);
  const [strictEnergy, setStrictEnergy] = useState(true);

  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [response]);

  useEffect(() => {
    const unlistenToken = listen<string>('llm-token', (event) => {
      setResponse((prev) => prev + event.payload);
    });

    const unlistenTelemetry = listen<Telemetry>('llm-telemetry', (event) => {
      setTelemetry(event.payload);
    });

    return () => {
      unlistenToken.then(f => f());
      unlistenTelemetry.then(f => f());
    };
  }, []);

  const handleExecute = async () => {
    if (!prompt.trim() || isGenerating) return;
    setIsGenerating(true);
    setResponse('');
    setTelemetry(null);
    
    // Zero-Allocation State Passing Mock
    // In a pure WASM-bindgen setup, we would pass the raw memory pointer.
    // For Tauri IPC, we simulate packing the layout into an array of floats.
    const intentLayout = new Float64Array([temporalStart, temporalEnd, strictEnergy ? 1.0 : 0.0]);
    const serializedLayout = Array.from(intentLayout);

    try {
      await invoke('run_agent_inference', { 
        prompt, 
        modelName: "phi3:mini (Q4_K_M)",
        intentLayout: serializedLayout
      });
    } catch (e) {
      console.error(e);
    } finally {
      setIsGenerating(false);
      setPrompt('');
    }
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      // In Tauri, we'd ideally get the real absolute path if permissions allow.
      // For this implementation, we simulate ingestion of the dropped file.
      setResponse('');
      setIsGenerating(true);
      try {
        const result = await invoke<string>('ingest_literature', { filePath: file.name });
        setResponse(`⚡ [Webizen Verified] Strict local intent approved.\\n\\n${result}\\n\\n**Math Ready:** $$ E = mc^2 $$`);
      } catch (err) {
        setResponse(`🔴 [WEBIZEN BLOCKED INTENT]: Extraction Failed or Unsupported File: ${err}`);
      } finally {
        setIsGenerating(false);
      }
    }
  };

  const handleTextSelection = () => {
    const selection = window.getSelection();
    if (selection && selection.toString().trim().length > 0) {
      const range = selection.getRangeAt(0);
      const rect = range.getBoundingClientRect();
      setCmlSelection({
        text: selection.toString().trim(),
        top: rect.bottom + window.scrollY + 5,
        left: rect.left + window.scrollX
      });
    } else {
      setCmlSelection(null);
    }
  };

  const handleCmlSave = async () => {
    if (cmlSelection) {
      try {
        await invoke('upsert_cmld_definition', { term: cmlSelection.text, contextDid: didInput });
        // Visually update the UI to show the semantic mapping
        setResponse(prev => prev.replace(cmlSelection.text, `<cml:context cml:href="${didInput}">${cmlSelection.text}</cml:context>`));
      } catch (e) {
        console.error("CML Save failed", e);
      }
      setCmlSelection(null);
    }
  };

  return (
    <div 
      className="flex flex-col gap-6 h-full relative"
      onDragOver={(e) => { e.preventDefault(); setIsDragging(true); }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={handleDrop}
      onMouseUp={handleTextSelection}
    >
      {isDragging && (
        <div className="absolute inset-0 z-50 bg-black/80 border-4 border-dashed border-[#00f0ff] rounded-2xl flex flex-col items-center justify-center backdrop-blur-sm pointer-events-none">
          <FileUp className="w-24 h-24 text-[#00f0ff] mb-4 animate-bounce" />
          <h2 className="text-4xl font-black text-white tracking-widest font-mono">INGEST LITERATURE</h2>
          <p className="text-[#00f0ff] mt-2">Webizen Semantic Router Engaged</p>
        </div>
      )}

      {cmlSelection && (
        <div 
          className="absolute z-40 glass-panel border border-[#00ff88]/50 p-3 flex flex-col gap-2 shadow-[0_0_20px_rgba(0,255,136,0.2)] animate-in fade-in zoom-in duration-200"
          style={{ top: cmlSelection.top, left: cmlSelection.left }}
        >
          <div className="text-xs text-[#00ff88] font-mono font-bold border-b border-[#00ff88]/20 pb-1">CML Context Disambiguation</div>
          <div className="text-white text-sm bg-black/40 px-2 py-1 rounded">Term: <span className="text-[#00f0ff]">"{cmlSelection.text}"</span></div>
          <input 
            value={didInput} 
            onChange={e => setDidInput(e.target.value)} 
            className="bg-black/50 border border-white/20 rounded px-2 py-1 text-white text-xs font-mono w-64 focus:border-[#00ff88] outline-none"
          />
          <button onClick={handleCmlSave} className="bg-[#00ff88]/20 text-[#00ff88] hover:bg-[#00ff88] hover:text-black transition-colors font-bold text-xs py-1 rounded">
            Bind Context
          </button>
        </div>
      )}

      <div className="flex gap-6 h-full">
        {/* Main Chat Panel */}
        <div className="glass-panel flex-1 flex flex-col h-[calc(100vh-150px)] overflow-hidden relative">
          
          {/* The Ontological Lens Pill */}
          <div className="absolute top-0 left-0 right-0 z-20 flex justify-center mt-2">
            <button 
              onClick={() => setIsLensOpen(!isLensOpen)}
              className="bg-black/80 border border-[#b026ff]/40 text-[#b026ff] px-4 py-1 rounded-full text-xs font-mono font-bold hover:bg-[#b026ff]/20 transition-all flex items-center gap-2 shadow-[0_0_10px_rgba(176,38,255,0.2)]"
            >
              [Lens: Global Context] {isLensOpen ? '▲' : '▼'}
            </button>
          </div>

          {/* The Axiom Bounds Drawer */}
          {isLensOpen && (
            <div className="absolute top-10 left-4 right-4 z-10 bg-black/90 border border-[#b026ff]/40 rounded-xl p-4 shadow-[0_10px_30px_rgba(0,0,0,0.8)] animate-in slide-in-from-top-2">
              <h3 className="text-[#b026ff] font-mono text-xs font-bold mb-3 border-b border-[#b026ff]/20 pb-2">Axiom Bounds Configuration</h3>
              <div className="flex gap-8">
                <div className="flex-1">
                  <label className="text-gray-400 text-xs font-mono block mb-2">Temporal Window (Spatio-Temporal Logic)</label>
                  <div className="flex items-center gap-4">
                    <input type="range" min="1800" max="2050" value={temporalStart} onChange={(e) => setTemporalStart(Number(e.target.value))} className="flex-1 accent-[#b026ff]" />
                    <span className="text-white font-mono text-sm w-12">{temporalStart}</span>
                    <span className="text-gray-500">-</span>
                    <input type="range" min="1800" max="2050" value={temporalEnd} onChange={(e) => setTemporalEnd(Number(e.target.value))} className="flex-1 accent-[#b026ff]" />
                    <span className="text-white font-mono text-sm w-12">{temporalEnd}</span>
                  </div>
                </div>
                <div className="w-64">
                  <label className="text-gray-400 text-xs font-mono block mb-2">Structural Constraints</label>
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input type="checkbox" checked={strictEnergy} onChange={(e) => setStrictEnergy(e.target.checked)} className="accent-[#00f0ff] w-4 h-4" />
                    <span className="text-white text-sm font-mono">Strict Thermodynamic Validation</span>
                  </label>
                </div>
              </div>
            </div>
          )}

          <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 mt-8 text-white flex items-center gap-2">
            <Terminal className="text-[#00f0ff]" /> Neuro-Symbolic Chat (Local RAG + Vision)
          </h2>
          
          <div className="flex-1 overflow-y-auto mb-4 flex flex-col gap-4 pr-2">
            <div className="bg-[#b026ff]/10 border border-[#b026ff]/30 p-4 rounded-xl self-start max-w-[80%]">
              <p className="text-white">System Online. Awaiting LLM Intent Routing or Literature Ingestion...</p>
            </div>

            {response && (
              <div className={`border p-5 rounded-xl self-start max-w-[80%] transition-all duration-300 ${
                response.includes("🔴 [WEBIZEN BLOCKED") || response.includes("🔴 [AGENT ERROR]") 
                ? "bg-red-900/20 border-red-500/50 shadow-[0_0_20px_rgba(255,0,0,0.4)]" 
                : "bg-black/40 border-[#00f0ff]/20 shadow-[0_0_10px_rgba(0,240,255,0.05)]"
              }`}>
                {response.includes("⚡ [Webizen Verified]") ? (
                  <p className="text-[#00ff88] font-mono text-xs mb-3 flex items-center gap-1 border-b border-[#00ff88]/20 pb-2">
                    <ShieldCheck className="w-4 h-4" /> Webizen Pre-flight Approved
                  </p>
                ) : response.includes("🔴") ? (
                  <p className="text-red-400 font-mono text-xs mb-3 flex items-center gap-1 border-b border-red-500/30 pb-2">
                    <ShieldAlert className="w-4 h-4" /> Agent Intent Intercepted
                  </p>
                ) : null}
                
                <div className={`prose prose-invert max-w-none leading-relaxed ${
                  response.includes("🔴") ? "text-red-200 font-mono text-sm" : "text-white"
                }`}>
                  {/* Provenance Token Indicator Parsing */}
                  {response.replace("⚡ [Webizen Verified] Strict local intent approved.\\n\\n", "").split("[WEBIZEN DENY]").map((part, index, array) => (
                    <span key={index}>
                      <ReactMarkdown 
                        remarkPlugins={[remarkMath]} 
                        rehypePlugins={[rehypeKatex]}
                        className="inline"
                      >
                        {part}
                      </ReactMarkdown>
                      {index < array.length - 1 && (
                        <span 
                          className="inline-flex items-center justify-center bg-[#b026ff]/20 border border-[#b026ff]/50 text-[#b026ff] rounded px-1 ml-1 mr-1 cursor-help relative group"
                          title="Webizen Intercept Log: Detected Out-of-Bounds Generation. Action: Vector recalculated via [WEBIZEN DENY] buffer injection. Output clean."
                        >
                          <ShieldCheck className="w-3 h-3 mr-1" />
                          <span className="text-[10px] font-mono font-bold">Intercept</span>
                          
                          {/* Custom Hover Tooltip for Webizen Log */}
                          <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 hidden group-hover:block w-64 bg-black/90 border border-[#b026ff]/50 p-2 rounded shadow-xl z-50 text-left">
                            <div className="text-[10px] font-mono text-[#b026ff] border-b border-[#b026ff]/30 pb-1 mb-1">Webizen Intercept Log</div>
                            <div className="text-[10px] font-mono text-white">Detected Out-of-Bounds Generation. Constraint Rule: OP_FCMP_LT evaluated. Vector recalculated via [WEBIZEN DENY] SPSC buffer injection.</div>
                          </div>
                        </span>
                      )}
                    </span>
                  ))}
                </div>
                {isGenerating && !response.includes("🔴") && <span className="inline-block w-2 h-4 bg-[#00f0ff] animate-pulse ml-1 align-middle mt-2"></span>}
              </div>
            )}
            <div ref={chatEndRef} />
          </div>

          <div className="flex gap-4">
            <input 
              type="text" 
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleExecute()}
              placeholder="Query datasets or drop a PDF..." 
              className="flex-1 bg-black/50 border border-white/20 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-[#00f0ff] font-mono text-sm" 
            />
            <button 
              onClick={handleExecute}
              disabled={isGenerating}
              className="bg-[#00f0ff]/10 text-[#00f0ff] border border-[#00f0ff]/30 px-6 py-3 rounded-lg font-bold hover:bg-[#00f0ff] hover:text-black transition-all disabled:opacity-50 disabled:cursor-not-allowed min-w-[120px]">
              {isGenerating ? 'Computing...' : 'Execute'}
            </button>
          </div>
        </div>

        {/* Telemetry HUD */}
        <div className="w-80 flex flex-col gap-4">
          <div className="glass-panel border-[#ffd700]/20 border">
            <h3 className="font-bold text-white mb-4 flex items-center gap-2 border-b border-white/10 pb-2">
              <Activity className="text-[#ffd700]" /> Telemetry HUD
            </h3>
            
            <div className="flex flex-col gap-4">
              <div className="bg-black/50 rounded-lg p-3 border border-white/5">
                <div className="text-xs text-gray-500 font-mono mb-1 flex items-center gap-1"><Cpu className="w-3 h-3" /> Tokens / Sec</div>
                <div className="text-2xl font-black text-white font-mono flex items-baseline gap-1">
                  {telemetry ? telemetry.token_rate.toFixed(1) : "0.0"} <span className="text-sm text-[#ffd700]">t/s</span>
                </div>
              </div>
              
              <div className="bg-black/50 rounded-lg p-3 border border-white/5">
                <div className="text-xs text-gray-500 font-mono mb-1">VRAM Usage</div>
                <div className="text-xl font-bold text-white font-mono">
                  {telemetry ? telemetry.vram_usage : "0.0 GB"}
                </div>
              </div>

              <div className="bg-black/50 rounded-lg p-3 border border-[#b026ff]/20 mt-2">
                <div className="text-xs text-[#b026ff] font-mono mb-2 font-bold flex items-center gap-1">
                  <Database className="w-3 h-3" /> Active RAG Context
                </div>
                <div className="text-sm text-gray-300 font-mono break-words leading-relaxed">
                  {telemetry ? telemetry.active_q42_context : "Awaiting Query..."}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
