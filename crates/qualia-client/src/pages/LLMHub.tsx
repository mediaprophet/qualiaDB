import { useState, useEffect, useCallback } from 'react';
import { Download, Cpu, HardDrive, X, CheckCircle, AlertCircle, Zap } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

interface HardwareStatus { ram_total_gb: number; ram_used_gb: number; vram_estimated_gb: number; }
interface InstalledModel { name: string; is_active: boolean; avatar_type: string; }
type DLStatus = 'idle' | 'downloading' | 'complete' | 'cancelled' | 'error';
interface DLState { status: DLStatus; progress: number; speedKbps: number; downloadedBytes: number; totalBytes: number; error?: string; }

const TAG_COLORS: Record<string, string> = {
  Reasoning: 'text-[#b026ff] bg-[#b026ff]/10 border-[#b026ff]/30',
  General: 'text-[#00f0ff] bg-[#00f0ff]/10 border-[#00f0ff]/30',
  Multilingual: 'text-[#00ff88] bg-[#00ff88]/10 border-[#00ff88]/30',
  Google: 'text-[#ffd700] bg-[#ffd700]/10 border-[#ffd700]/30',
  Extended: 'text-orange-400 bg-orange-400/10 border-orange-400/30',
  Large: 'text-red-400 bg-red-400/10 border-red-400/30',
  Code: 'text-blue-400 bg-blue-400/10 border-blue-400/30',
};

const MODELS = [
  { id: 'gemma2-2b-q4',    name: 'Gemma 2 2B IT',             tag: 'Google',     params: '2B',   format: 'Q4_K_M', size: '1.6 GB', totalBytes: 1_717_986_918, vram: '3 GB',  filename: 'gemma-2-2b-it-Q4_K_M.gguf',                         url: 'https://huggingface.co/bartowski/gemma-2-2b-it-GGUF/resolve/main/gemma-2-2b-it-Q4_K_M.gguf' },
  { id: 'phi35-mini-q4',   name: 'Phi-3.5 Mini Instruct',     tag: 'Reasoning',  params: '3.8B', format: 'Q4_K_M', size: '2.2 GB', totalBytes: 2_362_232_012, vram: '3 GB',  filename: 'Phi-3.5-mini-instruct-Q4_K_M.gguf',                  url: 'https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf' },
  { id: 'llama32-3b-q4',   name: 'Llama 3.2 3B Instruct',     tag: 'General',    params: '3B',   format: 'Q4_K_M', size: '2.0 GB', totalBytes: 2_013_265_920, vram: '3 GB',  filename: 'Llama-3.2-3B-Instruct-Q4_K_M.gguf',                  url: 'https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf' },
  { id: 'llama31-8b-q4',   name: 'Llama 3.1 8B Instruct',     tag: 'General',    params: '8B',   format: 'Q4_K_M', size: '4.9 GB', totalBytes: 5_268_299_776, vram: '8 GB',  filename: 'Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf',             url: 'https://huggingface.co/bartowski/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf' },
  { id: 'mistral-7b-q4',   name: 'Mistral 7B v0.3 Instruct',  tag: 'General',    params: '7B',   format: 'Q4_K_M', size: '4.1 GB', totalBytes: 4_404_019_200, vram: '8 GB',  filename: 'Mistral-7B-Instruct-v0.3-Q4_K_M.gguf',               url: 'https://huggingface.co/bartowski/Mistral-7B-Instruct-v0.3-GGUF/resolve/main/Mistral-7B-Instruct-v0.3-Q4_K_M.gguf' },
  { id: 'deepseek-r1-8b',  name: 'DeepSeek R1 Distill 8B',    tag: 'Reasoning',  params: '8B',   format: 'Q4_K_M', size: '4.9 GB', totalBytes: 5_250_512_896, vram: '8 GB',  filename: 'DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf',           url: 'https://huggingface.co/bartowski/DeepSeek-R1-Distill-Llama-8B-GGUF/resolve/main/DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf' },
  { id: 'qwen25-7b-q4',    name: 'Qwen 2.5 7B Instruct',      tag: 'Multilingual',params: '7B',  format: 'Q4_K_M', size: '4.4 GB', totalBytes: 4_726_123_520, vram: '8 GB',  filename: 'Qwen2.5-7B-Instruct-Q4_K_M.gguf',                    url: 'https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf' },
  { id: 'gemma2-9b-q4',    name: 'Gemma 2 9B IT',             tag: 'Google',     params: '9B',   format: 'Q4_K_M', size: '5.5 GB', totalBytes: 5_905_580_032, vram: '10 GB', filename: 'gemma-2-9b-it-Q4_K_M.gguf',                          url: 'https://huggingface.co/bartowski/gemma-2-9b-it-GGUF/resolve/main/gemma-2-9b-it-Q4_K_M.gguf' },
  { id: 'codellama-7b-q4', name: 'CodeLlama 7B Instruct',     tag: 'Code',       params: '7B',   format: 'Q4_K_M', size: '3.8 GB', totalBytes: 4_081_340_416, vram: '8 GB',  filename: 'CodeLlama-7b-Instruct-Q4_K_M.gguf',                  url: 'https://huggingface.co/bartowski/CodeLlama-7b-Instruct-hf-GGUF/resolve/main/CodeLlama-7b-Instruct-hf-Q4_K_M.gguf' },
  { id: 'mistral-nemo-q4', name: 'Mistral Nemo 12B',          tag: 'Extended',   params: '12B',  format: 'Q4_K_M', size: '7.1 GB', totalBytes: 7_626_023_936, vram: '12 GB', filename: 'Mistral-Nemo-Instruct-2407-Q4_K_M.gguf',              url: 'https://huggingface.co/bartowski/Mistral-Nemo-Instruct-2407-GGUF/resolve/main/Mistral-Nemo-Instruct-2407-Q4_K_M.gguf' },
  { id: 'qwen25-14b-q4',   name: 'Qwen 2.5 14B Instruct',     tag: 'Multilingual',params: '14B', format: 'Q4_K_M', size: '8.5 GB', totalBytes: 9_127_026_688, vram: '14 GB', filename: 'Qwen2.5-14B-Instruct-Q4_K_M.gguf',                   url: 'https://huggingface.co/bartowski/Qwen2.5-14B-Instruct-GGUF/resolve/main/Qwen2.5-14B-Instruct-Q4_K_M.gguf' },
  { id: 'llama31-70b-q4',  name: 'Llama 3.1 70B Instruct',    tag: 'Large',      params: '70B',  format: 'Q4_K_M', size: '42 GB',  totalBytes: 45_097_156_608, vram: '48 GB', filename: 'Meta-Llama-3.1-70B-Instruct-Q4_K_M.gguf',            url: 'https://huggingface.co/bartowski/Meta-Llama-3.1-70B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-70B-Instruct-Q4_K_M.gguf' },
];

function fmtBytes(b: number) {
  if (b === 0) return '0 B';
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
function fmtSpeed(kbps: number) {
  if (kbps < 1024) return `${kbps.toFixed(0)} KB/s`;
  return `${(kbps / 1024).toFixed(1)} MB/s`;
}

function ProgressBar({ dl, onCancel }: { dl: DLState; onCancel: () => void }) {
  const pct = Math.min(dl.progress, 100);
  const isActive = dl.status === 'downloading';
  return (
    <div className="mt-3">
      <div className="flex items-center justify-between text-[10px] font-mono mb-1">
        <span className={dl.status === 'error' ? 'text-red-400' : dl.status === 'cancelled' ? 'text-gray-500' : 'text-[#00f0ff]'}>
          {dl.status === 'error' ? `Error: ${dl.error}` : dl.status === 'cancelled' ? 'Cancelled' : dl.status === 'complete' ? 'Installed' : `${pct.toFixed(1)}%  ${fmtBytes(dl.downloadedBytes)} / ${dl.totalBytes > 0 ? fmtBytes(dl.totalBytes) : '?'}  •  ${fmtSpeed(dl.speedKbps)}`}
        </span>
        {isActive && (
          <button onClick={onCancel} className="text-gray-500 hover:text-red-400 transition-colors ml-2" title="Cancel">
            <X className="w-3 h-3" />
          </button>
        )}
      </div>
      <div className="w-full bg-white/5 rounded-full h-1.5 overflow-hidden">
        <div
          className={`h-full rounded-full transition-all duration-300 ${dl.status === 'complete' ? 'bg-[#00ff88]' : dl.status === 'error' || dl.status === 'cancelled' ? 'bg-red-500/50' : 'bg-gradient-to-r from-[#00f0ff] to-[#b026ff]'}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

export default function LLMHub() {
  const [hw, setHw] = useState<HardwareStatus>({ ram_total_gb: 0, ram_used_gb: 0, vram_estimated_gb: 0 });
  const [installed, setInstalled] = useState<Set<string>>(new Set());
  const [downloads, setDownloads] = useState<Record<string, DLState>>({});

  // Load hardware + installed models
  useEffect(() => {
    invoke<HardwareStatus>('get_hardware_status').then(setHw).catch(console.error);
    invoke<InstalledModel[]>('discover_models').then(models => {
      setInstalled(new Set(models.map(m => m.name)));
    }).catch(console.error);
  }, []);

  // Listen for download progress events
  useEffect(() => {
    const unlisten = listen<any>('download-progress', ({ payload }) => {
      const { id, progress, downloaded_bytes, total_bytes, speed_kbps, status } = payload;
      setDownloads(prev => ({
        ...prev,
        [id]: { status, progress, downloadedBytes: downloaded_bytes, totalBytes: total_bytes, speedKbps: speed_kbps },
      }));
      if (status === 'complete') {
        // refresh installed list
        invoke<InstalledModel[]>('discover_models').then(m => setInstalled(new Set(m.map(x => x.name)))).catch(console.error);
      }
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  const handleDownload = useCallback(async (model: typeof MODELS[0]) => {
    setDownloads(prev => ({ ...prev, [model.id]: { status: 'downloading', progress: 0, downloadedBytes: 0, totalBytes: model.totalBytes, speedKbps: 0 } }));
    try {
      await invoke('download_model', { url: model.url, filename: model.filename, modelId: model.id });
    } catch (e: any) {
      if (e !== 'Cancelled') {
        setDownloads(prev => ({ ...prev, [model.id]: { ...prev[model.id], status: 'error', error: String(e) } }));
      }
    }
  }, []);

  const handleCancel = useCallback(async (id: string) => {
    await invoke('cancel_download', { id }).catch(console.error);
  }, []);

  return (
    <div className="flex flex-col gap-6">
      {/* Hardware status */}
      <div className="flex gap-4">
        <div className="glass-panel flex-1 flex items-center gap-3 py-3">
          <HardDrive className="text-[#00ff88] w-5 h-5 shrink-0" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">System RAM</div>
            <div className="text-sm font-bold text-white">{hw.ram_total_gb.toFixed(0)} GB total · <span className="text-gray-400">{hw.ram_used_gb.toFixed(1)} GB used</span></div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-3 py-3">
          <Cpu className="text-[#ffd700] w-5 h-5 shrink-0" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Est. VRAM</div>
            <div className="text-sm font-bold text-white">{hw.vram_estimated_gb.toFixed(0)} GB</div>
          </div>
        </div>
        <div className="glass-panel flex-1 flex items-center gap-3 py-3">
          <Zap className="text-[#b026ff] w-5 h-5 shrink-0" />
          <div>
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Installed Models</div>
            <div className="text-sm font-bold text-white">{installed.size}</div>
          </div>
        </div>
      </div>

      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white">Model Hub — GGUF (HuggingFace)</h2>
        <p className="text-gray-500 text-sm mb-5">Open-weight models downloaded directly to your local Models directory and run via llama.cpp / Ollama.</p>
        <div className="grid gap-3">
          {MODELS.map(m => {
            const dl = downloads[m.id];
            const isInstalled = installed.has(m.filename);
            const isDownloading = dl?.status === 'downloading';
            return (
              <div key={m.id} className="bg-black/40 border border-white/5 rounded-xl p-4 hover:bg-white/[0.03] transition-colors">
                <div className="flex items-center justify-between gap-4">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h3 className="font-bold text-white">{m.name}</h3>
                      <span className={`text-[10px] font-bold px-2 py-0.5 rounded border ${TAG_COLORS[m.tag] ?? 'text-gray-400 bg-white/5 border-white/10'}`}>{m.tag}</span>
                    </div>
                    <div className="text-xs text-gray-500 font-mono mt-1 flex gap-3 flex-wrap">
                      <span>{m.params} params</span>
                      <span>{m.format}</span>
                      <span>{m.size}</span>
                      <span className="text-[#00f0ff]">≥ {m.vram} VRAM</span>
                    </div>
                  </div>
                  <div className="shrink-0">
                    {isInstalled ? (
                      <span className="flex items-center gap-1.5 text-[#00ff88] text-sm font-bold">
                        <CheckCircle className="w-4 h-4" /> Installed
                      </span>
                    ) : isDownloading ? (
                      <button onClick={() => handleCancel(m.id)} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-red-500/10 text-red-400 border border-red-500/20 text-xs font-bold hover:bg-red-500/20 transition-colors">
                        <X className="w-3.5 h-3.5" /> Stop
                      </button>
                    ) : dl?.status === 'error' ? (
                      <button onClick={() => handleDownload(m)} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-orange-500/10 text-orange-400 border border-orange-500/20 text-xs font-bold hover:bg-orange-500/20 transition-colors">
                        <AlertCircle className="w-3.5 h-3.5" /> Retry
                      </button>
                    ) : (
                      <button onClick={() => handleDownload(m)} className="flex items-center gap-2 px-4 py-2 rounded-lg bg-white/5 text-white border border-white/10 text-sm font-semibold hover:bg-[#00f0ff]/10 hover:border-[#00f0ff]/40 hover:text-[#00f0ff] transition-all">
                        <Download className="w-4 h-4" /> Download
                      </button>
                    )}
                  </div>
                </div>
                {dl && dl.status !== 'idle' && !isInstalled && (
                  <ProgressBar dl={dl} onCancel={() => handleCancel(m.id)} />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
