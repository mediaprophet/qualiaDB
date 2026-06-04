export default function Dashboard() {
  return (
    <div className="flex flex-col gap-6">
      <div className="glass-panel">
        <h2 className="text-xl font-bold border-b border-white/10 pb-2 mb-4 text-white">Edge-Native Benchmarks</h2>
        <div className="flex gap-4">
          <button className="px-6 py-3 rounded-lg border border-[#ffd700] text-white hover:text-white hover:shadow-[0_0_15px_rgba(255,215,0,0.4)] transition-all font-semibold relative overflow-hidden group bg-white/5">
            <span className="absolute left-0 top-0 w-1 h-full bg-[#ffd700] group-hover:w-full group-hover:opacity-20 transition-all z-0"></span>
            <span className="relative z-10">[Execute] Ingest 100,000 Quins</span>
          </button>
          <button className="px-6 py-3 rounded-lg border border-[#00f0ff] text-white hover:shadow-[0_0_15px_rgba(0,240,255,0.4)] transition-all font-semibold relative overflow-hidden group bg-white/5">
            <span className="absolute left-0 top-0 w-1 h-full bg-[#00f0ff] group-hover:w-full group-hover:opacity-20 transition-all z-0"></span>
            <span className="relative z-10">[Zero-Knowledge] Toxicity Screening</span>
          </button>
        </div>
      </div>
      <div className="glass-panel h-64 p-0 overflow-hidden flex flex-col border border-[#00f0ff]/20">
        <div className="bg-[#050508] flex-1 p-6 font-mono text-sm text-gray-300 overflow-y-auto shadow-[inset_0_0_20px_rgba(0,0,0,0.8)]">
          <div className="text-[#4ade80] flex gap-4"><span className="text-gray-500">[12:00:00]</span> System Initialized. Awaiting ILP routes.</div>
        </div>
      </div>
    </div>
  );
}
