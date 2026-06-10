import { useState, useEffect, useCallback } from 'react';
import { Zap, Shield, Send, ArrowDownLeft, Layers, Activity, Hexagon, Cpu, Server, Sun, Database, Copy, Check, X, TrendingUp, TrendingDown, ChevronRight, Plus, Trash2 } from 'lucide-react';
import { invoke, listen } from '../lib/tauri-compat';


interface CoinBalance {
  coin: string; ticker: string; address: string;
  balance: number; balance_display: string;
  fiat_usd: number; price_usd: number; change_24h: number;
  network: string; status: string;
}
interface TxRecord {
  txid: string; ticker: string; direction: string;
  amount: string; label: string; timestamp: string;
  status: string; confirmations: number; fee: string; counterparty: string;
}
interface TokenEntry {
  id: string; chain: string; token_type: string; contract: string;
  symbol: string; name: string; balance: string; decimals: number; fiat_usd: number;
}

const CHAIN_TYPES: Record<string, string[]> = {
  eCash: ['ALP', 'SLP'],
  Ethereum: ['ERC-20'],
  Nyx: ['CW-20'],
};
const CHAIN_DECIMALS: Record<string, number> = { eCash: 8, Ethereum: 18, Nyx: 6 };
const TOKEN_TYPE_COLORS: Record<string, string> = {
  ALP:     'bg-[#00ff88]/20 text-[#00ff88] border-[#00ff88]/30',
  SLP:     'bg-gray-600/30 text-gray-400 border-gray-600/50',
  'ERC-20': 'bg-[#b026ff]/20 text-[#b026ff] border-[#b026ff]/30',
  'CW-20':  'bg-[#00f0ff]/20 text-[#00f0ff] border-[#00f0ff]/30',
};
const CHAIN_COLORS: Record<string, string> = {
  eCash:    'text-[#00ff88]',
  Ethereum: 'text-[#b026ff]',
  Nyx:      'text-[#00f0ff]',
};

const COIN_COLORS: Record<string, { border: string; text: string; bg: string; icon: React.ReactNode }> = {
  XEC: { border: 'border-[#00ff88]/30', text: 'text-[#00ff88]', bg: 'from-black to-[#00ff88]/5', icon: <Hexagon className="w-4 h-4 text-[#00ff88]" /> },
  BTC: { border: 'border-[#f2a900]/30', text: 'text-[#f2a900]', bg: 'from-black to-[#f2a900]/5', icon: <Zap className="w-4 h-4 text-[#f2a900]" /> },
  XMR: { border: 'border-orange-500/30', text: 'text-orange-400', bg: 'from-black to-orange-500/5', icon: <Shield className="w-4 h-4 text-orange-400" /> },
  NYM: { border: 'border-[#00f0ff]/30', text: 'text-[#00f0ff]', bg: 'from-black to-[#00f0ff]/5', icon: <Shield className="w-4 h-4 text-[#00f0ff]" /> },
  ETH: { border: 'border-gray-500/30', text: 'text-gray-300', bg: 'from-black to-gray-500/5', icon: <Hexagon className="w-4 h-4 text-gray-400" /> },
};

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 1800); };
  return (
    <button onClick={copy} className="text-gray-600 hover:text-[#00f0ff] transition-colors" title="Copy">
      {copied ? <Check className="w-3 h-3 text-[#00ff88]" /> : <Copy className="w-3 h-3" />}
    </button>
  );
}

function fmtUsd(n: number) {
  return n >= 1000 ? `$${(n / 1000).toFixed(2)}k` : `$${n.toFixed(2)}`;
}

export default function Wallet() {
  const [coins, setCoins] = useState<CoinBalance[]>([]);
  const [tokens, setTokens] = useState<TokenEntry[]>([]);
  const [tokenChain, setTokenChain] = useState<string>('All');
  const [txs, setTxs] = useState<TxRecord[]>([]);
  const [txFilter, setTxFilter] = useState('ALL');
  const [loading, setLoading] = useState(true);

  // Send modal
  const [showSend, setShowSend] = useState(false);
  const [selectedAsset, setSelectedAsset] = useState<any>(null);
  const [sendAddr, setSendAddr] = useState('');
  const [sendAmount, setSendAmount] = useState('');
  const [sendStatus, setSendStatus] = useState<string | null>(null);

  // Receive modal
  const [showReceive, setShowReceive] = useState(false);
  const [receiveAsset, setReceiveAsset] = useState<CoinBalance | null>(null);

  // Import token modal
  const [showImport, setShowImport] = useState(false);
  const [importChain, setImportChain] = useState('Ethereum');
  const [importType, setImportType] = useState('ERC-20');
  const [importContract, setImportContract] = useState('');
  const [importSymbol, setImportSymbol] = useState('');
  const [importName, setImportName] = useState('');
  const [importDecimals, setImportDecimals] = useState(18);
  const [importError, setImportError] = useState('');
  const [importBusy, setImportBusy] = useState(false);

  // Hardware orchestration
  const [solarWatts, setSolarWatts] = useState(0);
  const [nymActive, setNymActive] = useState(false);
  const [starkActive, setStarkActive] = useState(false);
  const [nymTel, setNymTel] = useState({ packets_routed: 0, packets_dropped: 0, buffer_memory_mb: 0, is_congested: false });
  const [starkTel, setStarkTel] = useState({ status: 'Dormant', cpu_utilization: 0, ram_usage_mb: 0, fragments_paged: 0 });

  useEffect(() => {
    const u1 = listen('nym-telemetry',   (e: any) => setNymTel(e.payload));
    const u2 = listen('stark-telemetry', (e: any) => setStarkTel(e.payload));
    return () => { u1.then(f => f()); u2.then(f => f()); };
  }, []);

  const refreshTokens = useCallback(() => {
    invoke<TokenEntry[]>('get_tokens').then(setTokens).catch(console.error);
  }, []);

  // Load balances, tokens, and transactions
  useEffect(() => {
    Promise.all([
      invoke<CoinBalance[]>('get_coin_balances'),
      invoke<TokenEntry[]>('get_tokens'),
      invoke<TxRecord[]>('get_transaction_history', { ticker: 'ALL' }),
    ]).then(([c, t, tx]) => {
      setCoins(c);
      setTokens(t);
      setTxs(tx);
    }).catch(console.error).finally(() => setLoading(false));
  }, []);

  // Filter transactions when tab changes
  useEffect(() => {
    invoke<TxRecord[]>('get_transaction_history', { ticker: txFilter })
      .then(setTxs).catch(console.error);
  }, [txFilter]);

  const totalUsd = coins.reduce((s, c) => s + c.fiat_usd, 0);
  const avgChange = coins.length ? coins.reduce((s, c) => s + c.change_24h, 0) / coins.length : 0;

  const handleToggleNym = async () => {
    const newState = await invoke<boolean>('toggle_nym_relay').catch(() => nymActive);
    setNymActive(newState);
  };
  const handleToggleStark = async () => {
    const newState = await invoke<boolean>('toggle_stark_prover').catch(() => starkActive);
    setStarkActive(newState);
  };
  const handleSolar = (e: React.ChangeEvent<HTMLInputElement>) => {
    const w = parseInt(e.target.value); setSolarWatts(w);
    invoke('update_solar_input', { watts: w });
  };

  const openSend = (asset: any) => { setSelectedAsset(asset); setSendAddr(''); setSendAmount(''); setShowSend(true); };
  const openReceive = (coin: CoinBalance) => { setReceiveAsset(coin); setShowReceive(true); };

  const handleRemoveToken = useCallback(async (id: string) => {
    await invoke('remove_token', { id }).catch(console.error);
    refreshTokens();
  }, [refreshTokens]);

  const handleImportToken = useCallback(async () => {
    if (!importContract.trim() || !importSymbol.trim() || !importName.trim()) return;
    setImportBusy(true); setImportError('');
    try {
      await invoke('add_token', {
        chain: importChain, tokenType: importType, contract: importContract.trim(),
        symbol: importSymbol.trim().toUpperCase(), name: importName.trim(), decimals: importDecimals,
      });
      setShowImport(false);
      setImportContract(''); setImportSymbol(''); setImportName('');
      refreshTokens();
    } catch (e: any) {
      setImportError(String(e));
    } finally {
      setImportBusy(false);
    }
  }, [importChain, importType, importContract, importSymbol, importName, importDecimals, refreshTokens]);

  const executeSend = () => {
    setSendStatus(`[Mock] Broadcast ${sendAmount} ${selectedAsset?.ticker} → ${sendAddr}`);
    setTimeout(() => {
      setSendStatus(null);
      setShowSend(false);
    }, 3000);
  };

  const tickers = ['ALL', ...coins.map(c => c.ticker)];

  return (
    <div className="flex flex-col gap-4 h-full">

      {/* Portfolio summary */}
      <div className="glass-panel py-3 flex items-center gap-8">
        <div>
          <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">Total Portfolio</div>
          <div className="text-2xl font-black text-white font-mono">{loading ? '—' : `$${totalUsd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`}</div>
        </div>
        <div>
          <div className="text-[10px] text-gray-500 font-mono uppercase tracking-widest">24h Avg Change</div>
          <div className={`text-lg font-bold flex items-center gap-1 ${avgChange >= 0 ? 'text-[#00ff88]' : 'text-red-400'}`}>
            {avgChange >= 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
            {avgChange >= 0 ? '+' : ''}{avgChange.toFixed(2)}%
          </div>
        </div>
        <div className="ml-auto flex gap-2">
          {coins.map(c => (
            <div key={c.ticker} className={`text-[10px] font-mono px-2 py-1 rounded border ${COIN_COLORS[c.ticker]?.border ?? 'border-white/10'} ${COIN_COLORS[c.ticker]?.text ?? 'text-gray-400'} bg-black/40`}>
              {c.ticker} {c.change_24h >= 0 ? '+' : ''}{c.change_24h}%
            </div>
          ))}
        </div>
      </div>

      <div className="flex gap-4 flex-1 min-h-0">

        {/* LEFT: coins + eTokens + hardware */}
        <div className="flex flex-col gap-4 flex-[3] overflow-y-auto pr-1">

          <h2 className="text-lg font-bold text-white flex items-center gap-2 shrink-0">
            <Layers className="text-[#00f0ff] w-5 h-5" /> Native Blockchains
          </h2>

          <div className="grid grid-cols-2 gap-3">
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="glass-panel h-28 animate-pulse bg-white/5" />
              ))
            ) : coins.map(coin => {
              const c = COIN_COLORS[coin.ticker] ?? { border: 'border-white/10', text: 'text-gray-300', bg: 'from-black to-gray-500/5', icon: <Hexagon className="w-4 h-4" /> };
              return (
                <div key={coin.ticker} className={`glass-panel bg-gradient-to-br ${c.bg} ${c.border} border p-4`}>
                  <div className="flex justify-between items-start mb-1">
                    <h3 className="text-sm font-bold text-white flex items-center gap-1.5">{c.icon} {coin.coin}</h3>
                    <span className={`text-[10px] font-bold ${coin.change_24h >= 0 ? 'text-[#00ff88]' : 'text-red-400'}`}>
                      {coin.change_24h >= 0 ? '+' : ''}{coin.change_24h}%
                    </span>
                  </div>
                  <div className="text-[10px] text-gray-500 font-mono mb-2 truncate" title={coin.network}>{coin.network}</div>
                  <div className="text-xl font-black text-white font-mono tracking-tight">{coin.balance_display}</div>
                  <div className={`text-xs font-mono mb-3 ${c.text}`}>{fmtUsd(coin.fiat_usd)} <span className="text-gray-600">@ ${coin.price_usd < 0.01 ? coin.price_usd.toFixed(6) : coin.price_usd.toLocaleString()}</span></div>
                  <div className="flex gap-2">
                    <button onClick={() => openSend({ ticker: coin.ticker, network: coin.network, balance: coin.balance_display })} className={`flex-1 bg-white/5 hover:bg-white/10 ${c.text} border ${c.border} font-bold py-1 rounded text-xs transition-colors flex items-center justify-center gap-1`}>
                      <Send className="w-3 h-3" /> Send
                    </button>
                    <button onClick={() => openReceive(coin)} className="flex-1 bg-white/5 hover:bg-white/10 text-gray-300 border border-white/10 font-bold py-1 rounded text-xs transition-colors flex items-center justify-center gap-1">
                      <ArrowDownLeft className="w-3 h-3" /> Receive
                    </button>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Multi-chain Tokens */}
          <div className="flex items-center justify-between mt-2 shrink-0">
            <h2 className="text-lg font-bold text-white flex items-center gap-2">
              <Hexagon className="text-[#00ff88] w-5 h-5" /> Token Holdings
            </h2>
            <button
              onClick={() => { setShowImport(true); setImportError(''); }}
              className="flex items-center gap-1.5 text-xs font-bold px-3 py-1.5 rounded-lg bg-white/5 border border-white/10 text-gray-400 hover:text-white hover:border-white/30 transition-all"
            >
              <Plus className="w-3.5 h-3.5" /> Import Token
            </button>
          </div>

          {/* Chain tabs */}
          <div className="flex gap-1.5 shrink-0">
            {['All', 'eCash', 'Ethereum', 'Nyx'].map(ch => (
              <button
                key={ch}
                onClick={() => setTokenChain(ch)}
                className={`px-3 py-1 rounded text-xs font-bold border transition-colors ${
                  tokenChain === ch
                    ? ch === 'All' ? 'bg-white/10 text-white border-white/20'
                      : ch === 'eCash' ? 'bg-[#00ff88]/15 text-[#00ff88] border-[#00ff88]/30'
                      : ch === 'Ethereum' ? 'bg-[#b026ff]/15 text-[#b026ff] border-[#b026ff]/30'
                      : 'bg-[#00f0ff]/15 text-[#00f0ff] border-[#00f0ff]/30'
                    : 'bg-black/30 text-gray-500 border-white/5 hover:text-gray-300'
                }`}
              >
                {ch}
                <span className="ml-1.5 opacity-60 font-mono text-[9px]">
                  {ch === 'All' ? tokens.length : tokens.filter(t => t.chain === ch).length}
                </span>
              </button>
            ))}
          </div>

          <div className="flex flex-col gap-2">
            {loading ? (
              Array.from({ length: 3 }).map((_, i) => <div key={i} className="h-14 bg-white/5 rounded animate-pulse" />)
            ) : tokens
              .filter(t => tokenChain === 'All' || t.chain === tokenChain)
              .map(token => (
                <div key={token.id} className="bg-black/40 border border-white/5 rounded-lg p-3 hover:bg-white/[0.03] transition-colors">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-bold text-white">{token.name}</span>
                        <span className={`text-[9px] px-1.5 py-0.5 rounded uppercase font-bold border ${TOKEN_TYPE_COLORS[token.token_type] ?? 'bg-white/5 text-gray-400 border-white/10'}`}>
                          {token.token_type}
                        </span>
                        <span className={`text-[9px] font-mono ${CHAIN_COLORS[token.chain] ?? 'text-gray-500'}`}>{token.chain}</span>
                      </div>
                      <div className="text-[10px] text-gray-600 font-mono truncate mt-0.5" title={token.contract}>
                        {token.contract.length > 20 ? `${token.contract.slice(0, 10)}…${token.contract.slice(-8)}` : token.contract}
                        <CopyButton text={token.contract} />
                      </div>
                    </div>
                    <div className="text-right shrink-0">
                      <div className="font-black text-base text-white font-mono">
                        {token.balance} <span className="text-xs text-gray-500">{token.symbol}</span>
                      </div>
                      {token.fiat_usd > 0 && (
                        <div className="text-[10px] text-gray-500 font-mono">{fmtUsd(token.fiat_usd)}</div>
                      )}
                    </div>
                  </div>
                  <div className="flex gap-2 mt-2 pt-2 border-t border-white/5">
                    <button
                      onClick={() => openSend({ ticker: token.symbol, network: token.chain, balance: token.balance })}
                      className="flex-1 bg-white/5 hover:bg-white/10 text-gray-300 border border-white/10 font-bold py-1 rounded text-[11px] transition-colors flex items-center justify-center gap-1"
                    >
                      <Send className="w-3 h-3" /> Send
                    </button>
                    <button
                      onClick={() => handleRemoveToken(token.id)}
                      className="bg-white/5 hover:bg-red-500/10 text-gray-600 hover:text-red-400 border border-white/5 hover:border-red-500/20 p-1 rounded transition-colors"
                      title="Remove from wallet"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))
            }
            {!loading && tokens.filter(t => tokenChain === 'All' || t.chain === tokenChain).length === 0 && (
              <div className="text-center text-gray-600 text-xs font-mono py-6">
                No {tokenChain === 'All' ? '' : tokenChain + ' '}tokens — click <strong>Import Token</strong> to add one.
              </div>
            )}
          </div>

          {/* Hardware Orchestration */}
          <div className="glass-panel border-[#ff9900]/30 border mt-2 shrink-0">
            <h2 className="text-base font-bold border-b border-white/10 pb-2 mb-4 flex items-center justify-between text-white">
              <span className="flex items-center gap-2"><Cpu className="text-[#ff9900] w-4 h-4" /> Hardware Orchestration</span>
              <span className="text-[10px] bg-[#ff9900]/20 text-[#ff9900] px-2 py-1 rounded-full font-bold">Idle Monetisation</span>
            </h2>
            <div className="grid grid-cols-2 gap-4">
              {/* Nym */}
              <div className="bg-black/50 border border-white/10 rounded-lg p-3">
                <div className="flex justify-between items-center mb-3">
                  <h3 className="font-bold text-xs flex items-center gap-1.5 text-white"><Server className="w-3.5 h-3.5 text-[#00f0ff]" /> Nym Relay</h3>
                  <button onClick={handleToggleNym} className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase transition-colors ${nymActive ? 'bg-red-500/20 text-red-400 border border-red-500/30' : 'bg-[#00f0ff]/20 text-[#00f0ff] border border-[#00f0ff]/30'}`}>
                    {nymActive ? 'Halt' : 'Start'}
                  </button>
                </div>
                <div className="grid grid-cols-2 gap-1.5 text-[10px] font-mono">
                  <div className="bg-white/5 p-1.5 rounded"><div className="text-gray-500 mb-0.5">Routed/Dropped</div><div className="text-[#00f0ff] font-bold">{nymTel.packets_routed}/{nymTel.packets_dropped}</div></div>
                  <div className={`p-1.5 rounded border ${nymTel.is_congested ? 'bg-red-500/10 border-red-500/30' : 'bg-white/5 border-transparent'}`}>
                    <div className="text-gray-500 mb-0.5">RAM Buffer</div>
                    <div className={nymTel.is_congested ? 'text-red-400 font-bold' : 'text-white font-bold'}>{nymTel.buffer_memory_mb.toFixed(1)} MB</div>
                  </div>
                </div>
              </div>
              {/* ZK-STARK */}
              <div className="bg-black/50 border border-white/10 rounded-lg p-3">
                <div className="flex justify-between items-center mb-3">
                  <h3 className="font-bold text-xs flex items-center gap-1.5 text-white"><Activity className="w-3.5 h-3.5 text-[#ff9900]" /> ZK-STARK</h3>
                  <button onClick={handleToggleStark} className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase transition-colors ${starkActive ? 'bg-red-500/20 text-red-400 border border-red-500/30' : 'bg-[#ff9900]/20 text-[#ff9900] border border-[#ff9900]/30'}`}>
                    {starkActive ? 'Halt' : 'Start'}
                  </button>
                </div>
                <div className="grid grid-cols-2 gap-1.5 text-[10px] font-mono">
                  <div className="bg-white/5 p-1.5 rounded"><div className="text-gray-500 mb-0.5 flex items-center gap-1"><Sun className="w-2.5 h-2.5 text-[#ffd700]" /> Solar</div><div className={`font-bold ${solarWatts >= 400 ? 'text-[#00ff88]' : 'text-[#ff9900]'}`}>{solarWatts}W</div></div>
                  <div className="bg-white/5 p-1.5 rounded"><div className="text-gray-500 mb-0.5">CPU / RAM</div><div className="font-bold text-white">{starkTel.cpu_utilization.toFixed(0)}% / {starkTel.ram_usage_mb.toFixed(0)}MB</div></div>
                </div>
                <div className="mt-2">
                  <input type="range" min="0" max="1000" value={solarWatts} onChange={handleSolar} className="w-full accent-[#ff9900]" />
                  <div className="text-[9px] text-gray-600 font-mono text-center">{solarWatts}W simulated solar input</div>
                </div>
              </div>
            </div>
          </div>

        </div>

        {/* RIGHT: Transaction History */}
        <div className="flex-[2] glass-panel flex flex-col min-h-0">
          <h2 className="text-lg font-bold border-b border-white/10 pb-2 mb-3 text-white flex items-center gap-2 shrink-0">
            <Activity className="text-[#b026ff] w-5 h-5" /> Transaction History
          </h2>

          {/* Coin filter tabs */}
          <div className="flex gap-1 flex-wrap mb-3 shrink-0">
            {tickers.map(t => (
              <button
                key={t}
                onClick={() => setTxFilter(t)}
                className={`px-2.5 py-1 rounded text-[10px] font-bold uppercase transition-colors border ${txFilter === t ? 'bg-[#b026ff]/20 text-[#b026ff] border-[#b026ff]/40' : 'bg-white/5 text-gray-500 border-white/10 hover:text-white'}`}
              >
                {t}
              </button>
            ))}
          </div>

          {/* Transaction list */}
          <div className="flex flex-col gap-2 overflow-y-auto flex-1">
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => <div key={i} className="h-14 bg-white/5 rounded animate-pulse" />)
            ) : txs.length === 0 ? (
              <div className="flex-1 flex items-center justify-center text-gray-600 text-sm font-mono">No transactions</div>
            ) : txs.map((tx, i) => (
              <div key={i} className="bg-black/40 border border-white/5 rounded-lg p-3 hover:bg-white/[0.03] transition-colors">
                <div className="flex items-start justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <div className={`w-6 h-6 rounded-full flex items-center justify-center shrink-0 ${tx.direction === 'in' ? 'bg-[#00ff88]/10 text-[#00ff88]' : 'bg-red-500/10 text-red-400'}`}>
                      {tx.direction === 'in' ? <ArrowDownLeft className="w-3.5 h-3.5" /> : <Send className="w-3.5 h-3.5" />}
                    </div>
                    <div className="min-w-0">
                      <div className="text-sm font-bold text-white truncate">{tx.label}</div>
                      <div className="text-[10px] text-gray-600 font-mono truncate">{tx.counterparty}</div>
                    </div>
                  </div>
                  <div className="text-right shrink-0">
                    <div className={`text-sm font-bold font-mono ${tx.direction === 'in' ? 'text-[#00ff88]' : 'text-red-400'}`}>
                      {tx.direction === 'in' ? '+' : '−'}{tx.amount} {tx.ticker}
                    </div>
                    <div className="text-[10px] text-gray-600 font-mono">{tx.timestamp}</div>
                  </div>
                </div>
                <div className="flex items-center justify-between mt-2 pt-2 border-t border-white/5">
                  <div className="flex items-center gap-1 text-[9px] font-mono text-gray-600 truncate">
                    <ChevronRight className="w-2.5 h-2.5" />
                    <span className="truncate">{tx.txid}</span>
                    <CopyButton text={tx.txid} />
                  </div>
                  <div className="flex items-center gap-2 shrink-0 ml-2">
                    {tx.fee && <span className="text-[9px] text-gray-600 font-mono">fee {tx.fee}</span>}
                    <span className={`text-[9px] px-1.5 py-0.5 rounded font-bold ${tx.status === 'confirmed' ? 'bg-[#00ff88]/10 text-[#00ff88]' : 'bg-yellow-500/10 text-yellow-400'}`}>
                      {tx.status === 'confirmed' ? `✓ ${tx.confirmations}` : 'pending'}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Send Modal */}
      {showSend && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-md border-[#00f0ff]/30">
            <div className="flex justify-between items-center mb-5">
              <h2 className="text-xl font-bold text-white flex items-center gap-2"><Send className="text-[#00f0ff] w-5 h-5" /> Send {selectedAsset?.ticker}</h2>
              <button onClick={() => setShowSend(false)} className="text-gray-500 hover:text-white"><X className="w-5 h-5" /></button>
            </div>
            <div className="flex flex-col gap-4 mb-2">
              <div className="bg-black/30 border border-white/5 rounded px-3 py-2 text-xs font-mono text-gray-400">
                Balance: <span className="text-white">{selectedAsset?.balance_display ?? selectedAsset?.balance}</span>
              </div>
              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Recipient Address or DID</label>
                <input value={sendAddr} onChange={e => setSendAddr(e.target.value)} type="text" className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#00f0ff]" placeholder={`Enter ${selectedAsset?.network} address…`} />
              </div>
              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Amount</label>
                <div className="flex items-center gap-2">
                  <input value={sendAmount} onChange={e => setSendAmount(e.target.value)} type="number" className="flex-1 bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#00f0ff]" placeholder="0.00" />
                  <span className="font-bold text-gray-500 w-14 text-right">{selectedAsset?.ticker}</span>
                </div>
              </div>
            </div>
            <p className="text-[10px] text-gray-600 font-mono mb-4">Transaction signing is not yet integrated — this will broadcast a mock transaction for UI testing.</p>
            {sendStatus && (
              <div className="bg-[#00ff88]/10 border border-[#00ff88]/30 rounded px-3 py-2 text-xs text-[#00ff88] font-mono mb-4">
                {sendStatus}
              </div>
            )}
            <div className="flex justify-end gap-3">
              <button onClick={() => { setShowSend(false); setSendStatus(null); }} className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">Cancel</button>
              <button onClick={executeSend} disabled={!sendAddr || !sendAmount || !!sendStatus} className="px-6 py-2 bg-[#00f0ff]/20 text-[#00f0ff] hover:bg-[#00f0ff]/40 border border-[#00f0ff]/30 rounded font-bold transition-all flex items-center gap-2 disabled:opacity-40">
                <Send className="w-4 h-4" /> Broadcast Tx
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Receive Modal */}
      {showReceive && receiveAsset && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-md border-[#00ff88]/30">
            <div className="flex justify-between items-center mb-5">
              <h2 className="text-xl font-bold text-white flex items-center gap-2"><ArrowDownLeft className="text-[#00ff88] w-5 h-5" /> Receive {receiveAsset.ticker}</h2>
              <button onClick={() => setShowReceive(false)} className="text-gray-500 hover:text-white"><X className="w-5 h-5" /></button>
            </div>
            <div className="mb-3">
              <div className="text-xs text-gray-500 font-bold uppercase tracking-widest mb-2">Your {receiveAsset.network} Address</div>
              <div className="bg-black/60 border border-white/10 rounded-lg p-3 font-mono text-xs text-[#00ff88] break-all flex items-start gap-2">
                <span className="flex-1">{receiveAsset.address}</span>
                <CopyButton text={receiveAsset.address} />
              </div>
            </div>
            {receiveAsset.address.includes('generate identity') && (
              <div className="bg-yellow-500/10 border border-yellow-500/30 rounded px-3 py-2 text-xs text-yellow-400 mb-4">
                Go to <strong>Identifiers &amp; Credentials</strong> to generate your seed and derive wallet addresses first.
              </div>
            )}
            <p className="text-[10px] text-gray-600 font-mono">Send only {receiveAsset.ticker} to this address. Sending other assets may result in permanent loss.</p>
            <div className="flex justify-end mt-4">
              <button onClick={() => setShowReceive(false)} className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">Close</button>
            </div>
          </div>
        </div>
      )}

      {/* Import Token Modal */}
      {showImport && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm">
          <div className="glass-panel w-full max-w-md border-[#b026ff]/30">
            <div className="flex justify-between items-center mb-5">
              <h2 className="text-xl font-bold text-white flex items-center gap-2">
                <Plus className="text-[#b026ff] w-5 h-5" /> Import Token
              </h2>
              <button onClick={() => setShowImport(false)} className="text-gray-500 hover:text-white"><X className="w-5 h-5" /></button>
            </div>

            <div className="flex flex-col gap-4 mb-4">
              {/* Chain + Type row */}
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Chain</label>
                  <select
                    value={importChain}
                    onChange={e => {
                      const ch = e.target.value;
                      setImportChain(ch);
                      setImportType(CHAIN_TYPES[ch]?.[0] ?? 'ERC-20');
                      setImportDecimals(CHAIN_DECIMALS[ch] ?? 18);
                    }}
                    className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 text-sm text-white outline-none focus:border-[#b026ff] font-mono"
                  >
                    <option>eCash</option>
                    <option>Ethereum</option>
                    <option>Nyx</option>
                  </select>
                </div>
                <div>
                  <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Type</label>
                  <select
                    value={importType}
                    onChange={e => setImportType(e.target.value)}
                    className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 text-sm text-white outline-none focus:border-[#b026ff] font-mono"
                  >
                    {(CHAIN_TYPES[importChain] ?? ['ERC-20']).map(t => (
                      <option key={t}>{t}</option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">
                  {importChain === 'eCash' ? 'Token ID' : 'Contract Address'}
                </label>
                <input
                  value={importContract} onChange={e => setImportContract(e.target.value)}
                  type="text"
                  className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#b026ff]"
                  placeholder={importChain === 'Ethereum' ? '0x…' : importChain === 'Nyx' ? 'nyx1…' : 'alp:0x… or slp:0x…'}
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Symbol</label>
                  <input
                    value={importSymbol} onChange={e => setImportSymbol(e.target.value)}
                    type="text" maxLength={10}
                    className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#b026ff] uppercase"
                    placeholder="e.g. USDT"
                  />
                </div>
                <div>
                  <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Decimals</label>
                  <input
                    value={importDecimals} onChange={e => setImportDecimals(Number(e.target.value))}
                    type="number" min={0} max={18}
                    className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#b026ff]"
                  />
                </div>
              </div>

              <div>
                <label className="text-xs text-gray-400 font-bold uppercase tracking-widest mb-1 block">Token Name</label>
                <input
                  value={importName} onChange={e => setImportName(e.target.value)}
                  type="text"
                  className="w-full bg-black/50 border border-white/10 rounded px-3 py-2 font-mono text-sm text-white outline-none focus:border-[#b026ff]"
                  placeholder="e.g. Tether USD"
                />
              </div>
            </div>

            {importError && (
              <div className="bg-red-500/10 border border-red-500/30 rounded px-3 py-2 text-xs text-red-400 font-mono mb-4">
                {importError}
              </div>
            )}

            <div className="flex justify-end gap-3">
              <button onClick={() => setShowImport(false)} className="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">Cancel</button>
              <button
                onClick={handleImportToken}
                disabled={importBusy || !importContract.trim() || !importSymbol.trim() || !importName.trim()}
                className="px-6 py-2 bg-[#b026ff]/20 text-[#b026ff] hover:bg-[#b026ff]/40 border border-[#b026ff]/30 rounded font-bold transition-all flex items-center gap-2 disabled:opacity-40"
              >
                <Plus className="w-4 h-4" /> {importBusy ? 'Adding…' : 'Add to Wallet'}
              </button>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
