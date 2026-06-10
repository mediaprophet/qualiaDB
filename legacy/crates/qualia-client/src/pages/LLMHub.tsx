import { useState, useEffect, useCallback } from 'react';
import { Download, Cpu, HardDrive, X, CheckCircle, AlertCircle, Zap, Play, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

const MODELS_MANIFEST_URL =
  'https://raw.githubusercontent.com/mediaprophet/qualiaDB/refs/heads/main/manifests/models.json';

interface HardwareStatus { ram_total_gb: number; ram_used_gb: number; vram_estimated_gb: number; }
interface InstalledModel { name: string; is_active: boolean; avatar_type: string; }

interface ModelEntry {
  id: string;
  name: string;
  tag: string;
  params: string;
  format: string;
  size: string;
  total_bytes: number;
  vram: string;
  filename: string;
  url: string;
}

type DLStatus = 'idle' | 'downloading' | 'complete' | 'cancelled' | 'error';
interface DLState { status: DLStatus; progress: number; speedKbps: number; downloadedBytes: number; totalBytes: number; error?: string; }

const TAG_COLORS: Record<string, string> = {
  Reasoning:   'text-[#b026ff] bg-[#b026ff]/10 border-[#b026ff]/30',
  General:     'text-[#00f0ff] bg-[#00f0ff]/10 border-[#00f0ff]/30',
  Multilingual:'text-[#00ff88] bg-[#00ff88]/10 border-[#00ff88]/30',
  Google:      'text-[#ffd700] bg-[#ffd700]/10 border-[#ffd700]/30',
  Extended:    'text-orange-400 bg-orange-400/10 border-orange-400/30',
  Large:       'text-red-400 bg-red-400/10 border-red-400/30',
  Code:        'text-blue-400 bg-blue-400/10 border-blue-400/30',
  Tiny:        'text-gray-400 bg-white/5 border-white/10',
};

// Bundled fallback — used when the remote manifest can't be fetched
const FALLBACK_MODELS: ModelEntry[] = [
  { id: 'llama32-1b-q4',       name: 'Llama 3.2 1B Instruct',       tag: 'Tiny',        params: '1B',   format: 'Q4_K_M', size: '0.7 GB', total_bytes: 770695168,    vram: '2 GB',  filename: 'Llama-3.2-1B-Instruct-Q4_K_M.gguf',                     url: 'https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf' },
  { id: 'gemma3-4b-q4',        name: 'Gemma 3 4B IT',               tag: 'Google',      params: '4B',   format: 'Q4_K_M', size: '2.5 GB', total_bytes: 2684354560,   vram: '4 GB',  filename: 'gemma-3-4b-it-Q4_K_M.gguf',                             url: 'https://huggingface.co/bartowski/gemma-3-4b-it-GGUF/resolve/main/gemma-3-4b-it-Q4_K_M.gguf' },
  { id: 'phi4-mini-q4',        name: 'Phi-4 Mini Instruct',         tag: 'Reasoning',   params: '3.8B', format: 'Q4_K_M', size: '2.3 GB', total_bytes: 2469606195,   vram: '3 GB',  filename: 'Phi-4-mini-instruct-Q4_K_M.gguf',                        url: 'https://huggingface.co/bartowski/Phi-4-mini-instruct-GGUF/resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf' },
  { id: 'llama32-3b-q4',       name: 'Llama 3.2 3B Instruct',       tag: 'General',     params: '3B',   format: 'Q4_K_M', size: '2.0 GB', total_bytes: 2013265920,   vram: '3 GB',  filename: 'Llama-3.2-3B-Instruct-Q4_K_M.gguf',                     url: 'https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF/resolve/main/Llama-3.2-3B-Instruct-Q4_K_M.gguf' },
  { id: 'llama31-8b-q4',       name: 'Llama 3.1 8B Instruct',       tag: 'General',     params: '8B',   format: 'Q4_K_M', size: '4.9 GB', total_bytes: 5268299776,   vram: '8 GB',  filename: 'Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf',                url: 'https://huggingface.co/bartowski/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q4_K_M.gguf' },
  { id: 'deepseek-r1-8b',      name: 'DeepSeek R1 Distill 8B',      tag: 'Reasoning',   params: '8B',   format: 'Q4_K_M', size: '4.9 GB', total_bytes: 5250512896,   vram: '8 GB',  filename: 'DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf',              url: 'https://huggingface.co/bartowski/DeepSeek-R1-Distill-Llama-8B-GGUF/resolve/main/DeepSeek-R1-Distill-Llama-8B-Q4_K_M.gguf' },
  { id: 'qwen25-7b-q4',        name: 'Qwen 2.5 7B Instruct',        tag: 'Multilingual', params: '7B',  format: 'Q4_K_M', size: '4.4 GB', total_bytes: 4726123520,   vram: '8 GB',  filename: 'Qwen2.5-7B-Instruct-Q4_K_M.gguf',                       url: 'https://huggingface.co/bartowski/Qwen2.5-7B-Instruct-GGUF/resolve/main/Qwen2.5-7B-Instruct-Q4_K_M.gguf' },
  { id: 'gemma3-12b-q4',       name: 'Gemma 3 12B IT',              tag: 'Google',      params: '12B',  format: 'Q4_K_M', size: '7.2 GB', total_bytes: 7730941952,   vram: '12 GB', filename: 'gemma-3-12b-it-Q4_K_M.gguf',                            url: 'https://huggingface.co/bartowski/gemma-3-12b-it-GGUF/resolve/main/gemma-3-12b-it-Q4_K_M.gguf' },
  { id: 'qwen25-14b-q4',       name: 'Qwen 2.5 14B Instruct',       tag: 'Multilingual', params: '14B', format: 'Q4_K_M', size: '8.5 GB', total_bytes: 9127026688,   vram: '14 GB', filename: 'Qwen2.5-14B-Instruct-Q4_K_M.gguf',                      url: 'https://huggingface.co/bartowski/Qwen2.5-14B-Instruct-GGUF/resolve/main/Qwen2.5-14B-Instruct-Q4_K_M.gguf' },
  { id: 'mistral-small-24b-q4',name: 'Mistral Small 3.1 24B',       tag: 'Extended',    params: '24B',  format: 'Q4_K_M', size: '14.3 GB',total_bytes: 15361286144,  vram: '18 GB', filename: 'Mistral-Small-3.1-24B-Instruct-2503-Q4_K_M.gguf',       url: 'https://huggingface.co/bartowski/Mistral-Small-3.1-24B-Instruct-2503-GGUF/resolve/main/Mistral-Small-3.1-24B-Instruct-2503-Q4_K_M.gguf' },
  { id: 'llama33-70b-q4',      name: 'Llama 3.3 70B Instruct',      tag: 'Large',       params: '70B',  format: 'Q4_K_M', size: '42 GB',  total_bytes: 45097156608,  vram: '48 GB', filename: 'Llama-3.3-70B-Instruct-Q4_K_M.gguf',                   url: 'https://huggingface.co/bartowski/Llama-3.3-70B-Instruct-GGUF/resolve/main/Llama-3.3-70B-Instruct-Q4_K_M.gguf' },
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
  const [models, setModels] = useState<ModelEntry[]>(FALLBACK_MODELS);
  const [manifestSource, setManifestSource] = useState<'remote' | 'fallback'>('fallback');
  const [activeModel, setActiveModel] = useState<string | null>(null);
  const [loadingModel, setLoadingModel] = useState<string | null>(null);

  const refreshInstalled = useCallback(() => {
    invoke<InstalledModel[]>('discover_models')
      .then(ms => setInstalled(new Set(ms.map(m => m.name))))
      .catch(console.error);
  }, []);

  useEffect(() => {
    // Hardware status
    invoke<HardwareStatus>('get_hardware_status').then(setHw).catch(console.error);

    // Installed models
    refreshInstalled();

    // Restore any in-progress downloads that survived page navigation
    invoke<any[]>('get_active_downloads').then(active => {
      if (!active.length) return;
      const restored: Record<string, DLState> = {};
      for (const p of active) {
        restored[p.id] = {
          status: p.status as DLStatus,
          progress: p.progress,
          downloadedBytes: p.downloaded_bytes,
          totalBytes: p.total_bytes,
          speedKbps: p.speed_kbps,
        };
      }
      setDownloads(prev => ({ ...restored, ...prev }));
    }).catch(console.error);

    // Active model
    invoke<string | null>('get_active_model').then(setActiveModel).catch(console.error);

    // Fetch remote manifest, fall back to bundled list on error
    invoke<string>('fetch_remote_manifest', { url: MODELS_MANIFEST_URL })
      .then(json => {
        const data = JSON.parse(json);
        if (Array.isArray(data.models) && data.models.length > 0) {
          setModels(data.models);
          setManifestSource('remote');
        }
      })
      .catch(() => { /* stay with FALLBACK_MODELS */ });
  }, [refreshInstalled]);

  // Listen for download progress
  useEffect(() => {
    const unlisten = listen<any>('download-progress', ({ payload }) => {
      const { id, progress, downloaded_bytes, total_bytes, speed_kbps, status } = payload;
      setDownloads(prev => ({
        ...prev,
        [id]: { status, progress, downloadedBytes: downloaded_bytes, totalBytes: total_bytes, speedKbps: speed_kbps },
      }));
      if (status === 'complete') refreshInstalled();
    });
    return () => { unlisten.then(f => f()); };
  }, [refreshInstalled]);

  // Listen for active model changes from other pages
  useEffect(() => {
    const unlisten = listen<string>('active-model-changed', (e) => setActiveModel(e.payload));
    return () => { unlisten.then(f => f()); };
  }, []);

  const handleDownload = useCallback(async (m: ModelEntry) => {
    setDownloads(prev => ({ ...prev, [m.id]: { status: 'downloading', progress: 0, downloadedBytes: 0, totalBytes: m.total_bytes, speedKbps: 0 } }));
    try {
      await invoke('download_model', { url: m.url, filename: m.filename, modelId: m.id });
    } catch (e: any) {
      if (e !== 'Cancelled') {
        setDownloads(prev => ({ ...prev, [m.id]: { ...prev[m.id], status: 'error', error: String(e) } }));
      }
    }
  }, []);

  const handleCancel = useCallback(async (id: string) => {
    await invoke('cancel_download', { id }).catch(console.error);
  }, []);

  const handleSetActive = useCallback(async (filename: string) => {
    setLoadingModel(filename);
    try {
      await invoke('set_active_model', { modelName: filename });
      setActiveModel(filename);
    } catch (e) {
      console.error('set_active_model failed:', e);
    } finally {
      setLoadingModel(null);
    }
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
            <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Active Model</div>
            <div className="text-sm font-bold text-white truncate max-w-[180px]" title={activeModel ?? undefined}>
              {activeModel ? activeModel.replace(/-Q4_K_M\.gguf$/, '') : <span className="text-gray-500 italic">None loaded</span>}
            </div>
          </div>
        </div>
      </div>

      <div className="glass-panel">
        <div className="flex items-center justify-between border-b border-white/10 pb-2 mb-4">
          <h2 className="text-xl font-bold text-white">Model Hub — GGUF (HuggingFace)</h2>
          <div className="flex items-center gap-2">
            {manifestSource === 'remote' ? (
              <span className="text-[9px] text-[#00ff88] font-mono bg-[#00ff88]/10 border border-[#00ff88]/20 px-2 py-0.5 rounded">Remote manifest</span>
            ) : (
              <span className="text-[9px] text-gray-500 font-mono bg-white/5 border border-white/10 px-2 py-0.5 rounded flex items-center gap-1">
                <RefreshCw className="w-2.5 h-2.5" /> Bundled fallback
              </span>
            )}
          </div>
        </div>
        <p className="text-gray-500 text-sm mb-5">
          Open-weight models downloaded to your local Models directory. Set one as active to use it in Neuro-Chat.
          <br />
          <span className="text-[11px] text-gray-600">Model list is fetched from the remote manifest — update
          <code className="text-gray-500 mx-1">manifests/models.json</code> on GitHub to add models without recompiling.</span>
        </p>
        <div className="grid gap-3">
          {models.map(m => {
            const dl = downloads[m.id];
            const isInstalled = installed.has(m.filename);
            const isActive = activeModel === m.filename;
            const isDownloading = dl?.status === 'downloading';
            const isLoading = loadingModel === m.filename;
            return (
              <div key={m.id} className={`bg-black/40 border rounded-xl p-4 hover:bg-white/[0.03] transition-colors ${isActive ? 'border-[#00ff88]/30 shadow-[0_0_12px_rgba(0,255,136,0.07)]' : 'border-white/5'}`}>
                <div className="flex items-center justify-between gap-4">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <h3 className="font-bold text-white">{m.name}</h3>
                      <span className={`text-[10px] font-bold px-2 py-0.5 rounded border ${TAG_COLORS[m.tag] ?? 'text-gray-400 bg-white/5 border-white/10'}`}>{m.tag}</span>
                      {isActive && (
                        <span className="text-[9px] font-bold px-2 py-0.5 rounded border bg-[#00ff88]/10 text-[#00ff88] border-[#00ff88]/30 flex items-center gap-1">
                          <CheckCircle className="w-2.5 h-2.5" /> Active
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-gray-500 font-mono mt-1 flex gap-3 flex-wrap">
                      <span>{m.params} params</span>
                      <span>{m.format}</span>
                      <span>{m.size}</span>
                      <span className="text-[#00f0ff]">≥ {m.vram} VRAM</span>
                    </div>
                  </div>
                  <div className="shrink-0 flex gap-2 items-center">
                    {isInstalled && !isActive && (
                      <button
                        onClick={() => handleSetActive(m.filename)}
                        disabled={isLoading}
                        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[#00ff88]/10 text-[#00ff88] border border-[#00ff88]/20 text-xs font-bold hover:bg-[#00ff88]/20 transition-colors disabled:opacity-50"
                      >
                        <Play className="w-3 h-3" /> {isLoading ? 'Loading…' : 'Load'}
                      </button>
                    )}
                    {isInstalled && isActive ? (
                      <span className="flex items-center gap-1.5 text-[#00ff88] text-sm font-bold">
                        <CheckCircle className="w-4 h-4" /> Active
                      </span>
                    ) : isInstalled && !isActive ? (
                      <span className="flex items-center gap-1.5 text-gray-500 text-xs font-bold">
                        <CheckCircle className="w-3.5 h-3.5" /> Installed
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
