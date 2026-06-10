import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

function timestamp(): string {
  const now = new Date();
  return now.toTimeString().slice(0, 8);
}

export default function Dashboard() {
  const [logs, setLogs] = useState<{ ts: string; text: string; color: string }[]>([
    { ts: timestamp(), text: 'System Initialized. Awaiting ILP routes.', color: 'text-[#4ade80]' },
  ]);
  const [running, setRunning] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  const appendLog = (text: string, color = 'text-gray-300') => {
    setLogs(prev => [...prev, { ts: timestamp(), text, color }]);
  };

  // Live hardware telemetry → append to terminal every ~10 s to avoid flooding
  useEffect(() => {
    let count = 0;
    const unlisten = listen<any>('hardware-telemetry', (event) => {
      count++;
      if (count % 5 !== 0) return;
      const { cpu, ram } = event.payload;
      appendLog(`[HW] CPU: ${cpu}  RAM: ${ram}`, 'text-[#00f0ff]');
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  const runCommand = async (cmd: string, label: string) => {
    if (running) return;
    setRunning(cmd);
    appendLog(`> ${label}`, 'text-[#ffd700]');
    try {
      const result = await invoke<string>('run_engine_command', { cmd });
      result.split('\n').forEach(line => appendLog(line, 'text-[#4ade80]'));
    } catch (e) {
      appendLog(`Error: ${e}`, 'text-red-400');
    } finally {
      setRunning(null);
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white">Edge-Native Benchmarks</h2>
        <div className="flex gap-4">
          <button
            onClick={() => runCommand('ingest_bench', 'Ingest 100,000 Quins')}
            disabled={!!running}
            className="px-6 py-3 rounded-lg border border-[#ffd700] text-white hover:text-white hover:shadow-[0_0_15px_rgba(255,215,0,0.4)] transition-all font-semibold relative overflow-hidden group bg-white/5 disabled:opacity-50"
          >
            <span className="absolute left-0 top-0 w-1 h-full bg-[#ffd700] group-hover:w-full group-hover:opacity-20 transition-all z-0"></span>
            <span className="relative z-10">
              {running === 'ingest_bench' ? 'Running…' : '[Execute] Ingest 100,000 Quins'}
            </span>
          </button>
          <button
            onClick={() => runCommand('zk_screen', '[Zero-Knowledge] Toxicity Screening')}
            disabled={!!running}
            className="px-6 py-3 rounded-lg border border-[#00f0ff] text-white hover:shadow-[0_0_15px_rgba(0,240,255,0.4)] transition-all font-semibold relative overflow-hidden group bg-white/5 disabled:opacity-50"
          >
            <span className="absolute left-0 top-0 w-1 h-full bg-[#00f0ff] group-hover:w-full group-hover:opacity-20 transition-all z-0"></span>
            <span className="relative z-10">
              {running === 'zk_screen' ? 'Running…' : '[Zero-Knowledge] Toxicity Screening'}
            </span>
          </button>
        </div>
      </div>
      <div className="glass-panel h-64 p-0 overflow-hidden flex flex-col border border-[#00f0ff]/20">
        <div className="bg-[#050508] flex-1 p-6 font-mono text-sm text-gray-300 overflow-y-auto shadow-[inset_0_0_20px_rgba(0,0,0,0.8)]">
          {logs.map((log, i) => (
            <div key={i} className={`${log.color} flex gap-4`}>
              <span className="text-gray-500 shrink-0">[{log.ts}]</span>
              <span>{log.text}</span>
            </div>
          ))}
          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  );
}
