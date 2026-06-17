import { hashToken, parseBigDecimal, toHex16, hasMsb } from './hash.js';
import { VFSProvider, QUIN_SIZE, formatOpfsCacheLabel } from './vfs.js';
import wasmInit, { execute_ntriples_query } from './qualia_core_db.js';

const DEFAULT_DATASET_ID = 'wordnet';

// ---------------------------------------------------------------------------
// Cost widget state
// ---------------------------------------------------------------------------

const COST_RATES = [0.001, 0.01, 0.1, 1, 10]; // μsat / 1 000 VM ops
let costRateIdx  = 3; // default: 1 μsat / 1k ops
const BTC_USD    = 70_000;

function updateCostRate(sliderVal) {
  costRateIdx = Number(sliderVal);
  document.getElementById('rate-label').textContent =
    `${COST_RATES[costRateIdx]} μsat / 1 000 VM ops`;
}
window.updateCostRate = updateCostRate;

function renderCostPanel(cycles) {
  const price   = COST_RATES[costRateIdx];
  const muSat   = cycles * price / 1_000;
  const sat     = muSat / 1_000_000;
  const usd     = sat * BTC_USD / 1e8;
  const rows    = document.querySelectorAll('#cost-table tr');
  const vals = [
    cycles.toLocaleString(),
    muSat.toFixed(4) + ' μsat',
    sat.toFixed(9)   + ' sat',
    '$' + usd.toExponential(2),
  ];
  rows.forEach((r, i) => {
    const cells = r.querySelectorAll('td');
    if (cells[1]) cells[1].textContent = vals[i] ?? '—';
  });
}

// ---------------------------------------------------------------------------
// WASM bootstrap
// ---------------------------------------------------------------------------

let wasmReady = false;
let execQuery  = null;

async function loadWasm() {
  try {
    const wasmUrl = new URL('qualia_core_db_bg.wasm', import.meta.url);
    const resp = await fetch(wasmUrl, { cache: 'no-store' });
    if (!resp.ok) throw new Error(`WASM fetch HTTP ${resp.status}`);
    await wasmInit({ module_or_path: resp });
    if (typeof execute_ntriples_query === 'function') {
      execQuery = execute_ntriples_query;
      wasmReady = true;
      setBadge('wasm-badge', 'WASM Ready', 'green');
    } else {
      setBadge('wasm-badge', 'WASM (legacy — rebuild needed)', 'amber');
    }
  } catch (e) {
    setBadge('wasm-badge', 'WASM unavailable — JS fallback active', 'amber');
    console.warn('[WordNet] WASM load failed:', e);
  }
}

// ---------------------------------------------------------------------------
// VFS + manifest bootstrap
// ---------------------------------------------------------------------------

let provider = null;
let activeVfs = null;
let dbBytes   = null;

async function loadManifest() {
  provider = await VFSProvider.fromManifest('./vfs-manifest.json');
  const sel = document.getElementById('dataset-select');
  const count = provider.available.length;

  for (const ds of provider.available) {
    const opt = document.createElement('option');
    opt.value       = ds.id;
    opt.textContent = `${ds.icon ?? ''} ${ds.label}`;
    sel.appendChild(opt);
  }

  if (count === 0) {
    setBadge('vfs-badge', 'Manifest empty — check vfs-manifest.json', 'amber');
    return;
  }

  const preferred = provider.available.find(d => d.id === DEFAULT_DATASET_ID) ?? provider.available[0];
  sel.value = preferred.id;
  updateDatasetDescription();
  document.getElementById('mount-btn').disabled = false;
  setBadge('vfs-badge', `Manifest ready — ${count} dataset(s)`, 'green');

  await window.mountDataset();
}

function updateDatasetDescription() {
  const id  = document.getElementById('dataset-select').value;
  const ds  = provider?.available.find(d => d.id === id);
  document.getElementById('dataset-description').textContent = ds?.description ?? '';
}

let _opfsWatchTimer = null;

function watchOpfsCache(vfs) {
  if (_opfsWatchTimer) clearInterval(_opfsWatchTimer);
  if (!vfs?.opfsCache?.prefetching) return;
  _opfsWatchTimer = setInterval(() => {
    const c = vfs.opfsCache;
    if (!c.prefetching) {
      clearInterval(_opfsWatchTimer);
      _opfsWatchTimer = null;
    }
    const bom = vfs.blockOffsetMap;
    const blockCount = bom ? bom.count : 0;
    const sizeMb = bom ? (bom.totalBytes / 1024 / 1024).toFixed(1) : '?';
    setBadge('vfs-badge',
      `VFS Ready — ${blockCount.toLocaleString()} blocks · ${sizeMb} MB · demand-paging` +
      formatOpfsCacheLabel(c),
      c.complete ? 'green' : 'amber');
  }, 1500);
}

window.mountDataset = async function mountDataset() {
  const id = document.getElementById('dataset-select').value;
  if (!id || !provider) return;
  const ds = provider.available.find(d => d.id === id);

  setBadge('mount-badge', `Mounting ${ds?.label ?? id}…`, 'amber');
  setBadge('vfs-badge',   'VFS loading…', 'amber');
  setBadge('lex-badge',   'Lexicon loading…', 'amber');

  try {
    activeVfs = await provider.mount(id, { loadLex: true });

    if (ds.compressed) {
      // Compressed LZ4 block-stream: fetch + decompress once (no Range support).
      setBadge('vfs-badge', 'Decompressing…', 'amber');
      try {
        dbBytes = await activeVfs.readAllDecompressed();
        const tripleCount = Math.floor(dbBytes.length / QUIN_SIZE);
        setBadge('vfs-badge', `VFS Ready — ${(dbBytes.length/1024/1024).toFixed(1)} MB decompressed`, 'green');
        updateStatsRow(4, 'Triples in Dataset', tripleCount.toLocaleString());
      } catch (e) {
        setBadge('vfs-badge', `Decompress failed: ${e.message}`, 'amber');
      }
    } else {
      // Q42 v3 unified volume — preamble Range boot loads lex+bidx+block_dir;
      // readBlock() demand-pages individual LZ4 SuperBlocks.
      const bom      = activeVfs.blockOffsetMap;
      const blockCount = bom ? bom.count : 0;
      if (blockCount > 0) {
        const sizeMb       = (bom.totalBytes / 1024 / 1024).toFixed(1);
        const approxTriples = blockCount * 850;
        const cacheNote = formatOpfsCacheLabel(activeVfs.opfsCache);
        setBadge('vfs-badge',
          `VFS Ready — ${blockCount.toLocaleString()} blocks · ${sizeMb} MB · demand-paging${cacheNote}`,
          'green');
        watchOpfsCache(activeVfs);
        updateStatsRow(4, 'Triples in Dataset', `~${approxTriples.toLocaleString()}`);
      } else {
        setBadge('vfs-badge',
          'VFS header probe failed — check that ' + ds.url + ' is reachable', 'amber');
      }
    }

    if (activeVfs.lexLoaded) {
      setBadge('lex-badge', `Lexicon — ${activeVfs._lexMap.size.toLocaleString()} entries`, 'green');
    } else {
      setBadge('lex-badge', 'No lexicon (hashes only)', 'amber');
    }

    hcSet('hc-bidx',
      activeVfs.bidxLoaded
        ? `${activeVfs.blockCount.toLocaleString()} ranges · O(log N)`
        : activeVfs.embeddedPreamble ? 'embedded in preamble' : 'BIDX not loaded',
      activeVfs.bidxLoaded ? 'green' : 'amber');

    setBadge('mount-badge', `Mounted: ${ds?.label ?? id}`, 'green');

    // Populate sample query presets from manifest
    if (ds?.sampleQueries?.length) {
      const presetBar = document.querySelector('.presets');
      presetBar.innerHTML = '<span style="font-size:.78rem;color:var(--text-muted);margin-right:.25rem;align-self:center">Presets:</span>';
      for (const sq of ds.sampleQueries) {
        const b = document.createElement('button');
        b.className   = 'preset-btn';
        b.textContent = sq.label;
        b.onclick     = () => {
          document.getElementById('query-input').value = sq.pattern;
          runQuery();
        };
        presetBar.appendChild(b);
      }
    }

    document.getElementById('search-btn').disabled = false;
    hcSet('hc-vfs', hcVfsLabel(), activeVfs || dbBytes ? 'green' : 'amber');
  } catch (e) {
    setBadge('mount-badge', 'Mount failed: ' + e.message, 'amber');
    console.error('[WordNet] Mount failed:', e);
  }
};

// ---------------------------------------------------------------------------
// Query mode tabs
// ---------------------------------------------------------------------------

let queryMode = 'nt';

window.setQueryMode = function setQueryMode(mode) {
  queryMode = mode;
  document.getElementById('mode-nt').style.display     = mode === 'nt'     ? '' : 'none';
  document.getElementById('mode-sparql').style.display = mode === 'sparql' ? '' : 'none';

  const tabNt     = document.getElementById('tab-nt');
  const tabSparql = document.getElementById('tab-sparql');
  tabNt.style.borderBottomColor     = mode === 'nt'     ? 'var(--primary)' : 'transparent';
  tabNt.style.color                  = mode === 'nt'     ? 'var(--primary)' : 'var(--text-muted)';
  tabSparql.style.borderBottomColor = mode === 'sparql' ? 'var(--primary)' : 'transparent';
  tabSparql.style.color              = mode === 'sparql' ? 'var(--primary)' : 'var(--text-muted)';
};

// ---------------------------------------------------------------------------
// SPARQL BGP parser (single-triple WHERE clause only)
// ---------------------------------------------------------------------------

function parseSparqlBgp(sparql) {
  const m = sparql.match(/WHERE\s*\{([^}]+)\}/i);
  if (!m) return null;
  // Strip trailing dot and collapse whitespace
  return m[1].trim().replace(/\s*\.\s*$/, '');
}

window.runSparql = async function runSparql() {
  const raw = document.getElementById('sparql-input').value.trim();
  const pattern = parseSparqlBgp(raw);
  if (!pattern) {
    showError('Could not parse a single BGP triple from the WHERE clause.');
    return;
  }
  const start = performance.now();
  await executeQuery(pattern, start);
};

// ---------------------------------------------------------------------------
// Query execution
// ---------------------------------------------------------------------------

window.runQuery = async function runQuery() {
  const raw = document.getElementById('query-input').value.trim();
  if (!raw) return;
  const start = performance.now();
  const btn = document.getElementById('search-btn');
  btn.disabled = true; btn.textContent = 'Querying…';
  try { await executeQuery(raw, start); }
  finally { btn.disabled = false; btn.textContent = 'Search'; }
};

async function executeQuery(raw, startTime) {
  const pattern = /^[<?]/.test(raw) ? raw : `?s ?p "${raw}"`;

  let result;

  if (dbBytes && dbBytes.length > 0) {
    // Pre-loaded buffer (compressed dataset or user-uploaded OPFS data).
    // The flat-quin layout has no SuperBlock headers — pass directly to WASM/JS.
    if (wasmReady && execQuery) {
      result = JSON.parse(execQuery(pattern, dbBytes, 200));
    } else {
      result = jsFallbackQuery(pattern, dbBytes, 200);
    }
  } else if (activeVfs) {
    // Demand-paging path: scan SuperBlocks one-by-one via HTTP Range / OPFS.
    if (activeVfs.blockCount === 0) {
      showError('Dataset not reachable. Check the file URL or run scripts/fetch_wordnet.sh.');
      return;
    }
    activeVfs.resetTelemetry();
    result = await streamingQuery(pattern, activeVfs, 200);
  } else {
    showError('No dataset mounted. Select a dataset above and click Mount.');
    return;
  }

  const elapsed = Math.round((performance.now() - startTime) * 1000);
  document.getElementById('latency-badge').textContent = `${elapsed} µs`;

  if (result.error) { showError(result.error); return; }

  const cycles = Number(result.vm_cycles ?? 0);
  const totalQuins = dbBytes
    ? dbBytes.length / QUIN_SIZE
    : (activeVfs?.blockCount ?? 1) * 850;
  renderResults(result, totalQuins);
  renderCostPanel(cycles);
  updateHealthConsole(cycles, result.matches.length, elapsed, result._costHeader ?? null, result._bidxUsed ?? null);
}

/**
 * Block-streaming query for demand-paged datasets (uncompressed SuperBlock format).
 *
 * Resolution order:
 *   1. BIDX lookup  — if the index is loaded and a bound hash exists, binary-
 *                     search the block ranges and fetch only 1-3 candidate blocks.
 *   2. Full scan    — if BIDX is absent or the query has no bound term, iterate
 *                     all blocks in 32-concurrent batches (OPFS-first, then Range).
 *
 * Each SuperBlock has a 160-byte header followed by up to 850 × 48-byte Quins;
 * zero-filled padding quins at the tail of the last block are skipped.
 *
 * @returns {object} matches, vm_cycles, direct_jump_ops, lexicon_lookup_ops, _bidxUsed
 */
async function streamingQuery(pattern, vfs, maxResults) {
  const CONCURRENCY  = 32;
  const HEADER_BYTES = 160;

  const tokens = pattern.trim().split(/\s+/).filter(t => t !== '.');
  if (tokens.length < 3) {
    return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0, _bidxUsed: false };
  }
  const [sT, pT, oT] = tokens;
  const sH = sT.startsWith('?') ? null : hashToken(sT);
  const pH = pT.startsWith('?') ? null : hashToken(pT);
  const oH = oT.startsWith('?') ? null : hashToken(oT);

  // ── BIDX-guided lookup (O(log N)) ──────────────────────────────────────
  // The BIDX is sorted by object hash.  Prefer oH for the lookup; fall back
  // to sH (future: secondary subject index) or full scan.
  let candidateBlocks = null;
  if (oH !== null) candidateBlocks = vfs.lookupBlocks(oH);
  // sH / pH lookups would need a separate subject/predicate-sorted BIDX;
  // leave as full-scan for now.

  const bidxUsed  = candidateBlocks !== null;
  const blockList = candidateBlocks ?? Array.from({ length: vfs.blockCount }, (_, i) => i);

  const matches = [];
  let cycles = 0, dj = 0, lx = 0;

  // ── Scan selected blocks in parallel batches ───────────────────────────
  for (let base = 0; base < blockList.length && matches.length < maxResults; base += CONCURRENCY) {
    const slice   = blockList.slice(base, base + CONCURRENCY);
    const fetches = slice.map(bi => vfs.readBlock(bi).catch(() => null));
    const blocks  = await Promise.all(fetches);

    for (const blockBytes of blocks) {
      if (!blockBytes || matches.length >= maxResults) break;

      const view      = new DataView(blockBytes.buffer, blockBytes.byteOffset);
      const quinSlots = Math.floor((blockBytes.length - HEADER_BYTES) / QUIN_SIZE);

      for (let qi = 0; qi < quinSlots && matches.length < maxResults; qi++) {
        const b = HEADER_BYTES + qi * QUIN_SIZE;
        const s = getU64(view, b),      p = getU64(view, b + 8),
              o = getU64(view, b + 16), c = getU64(view, b + 24),
              m = getU64(view, b + 32);

        if (s === 0n && p === 0n && o === 0n) continue; // zero-padding

        let ok = true;
        if (sH !== null) { cycles++; hasMsb(sH) ? dj++ : lx++; if (s !== sH) ok = false; }
        if (ok && pH !== null) { cycles++; hasMsb(pH) ? dj++ : lx++; if (p !== pH) ok = false; }
        if (ok && oH !== null) { cycles++; hasMsb(oH) ? dj++ : lx++; if (o !== oH) ok = false; }
        if (ok) matches.push({
          s: String(s), p: String(p), o: String(o), c: String(c), m: String(m),
        });
      }
    }
  }
  return { matches, vm_cycles: cycles, direct_jump_ops: dj, lexicon_lookup_ops: lx, _bidxUsed: bidxUsed };
}

async function getDbBytes() {
  if (dbBytes) return dbBytes;
  if (!activeVfs) return null;
  try { return await activeVfs.readBlock(0); } catch (_) { return null; }
}

// ---------------------------------------------------------------------------
// JS fallback scan
// ---------------------------------------------------------------------------

function jsFallbackQuery(pattern, bytes, maxResults) {
  const tokens = pattern.trim().split(/\s+/).filter(t => t !== '.');
  if (tokens.length < 3) return { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };
  const [sT, pT, oT] = tokens;
  const sH = sT.startsWith('?') ? null : hashToken(sT);
  const pH = pT.startsWith('?') ? null : hashToken(pT);
  const oH = oT.startsWith('?') ? null : hashToken(oT);

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const quins = Math.floor(bytes.length / QUIN_SIZE);
  const matches = [];
  let cycles = 0, dj = 0, lx = 0;

  for (let i = 0; i < quins && matches.length < maxResults; i++) {
    const b = i * QUIN_SIZE;
    const s = getU64(view, b),     p = getU64(view, b+8),
          o = getU64(view, b+16),  c = getU64(view, b+24),
          m = getU64(view, b+32);
    let ok = true;
    if (sH !== null) { cycles++; hasMsb(sH) ? dj++ : lx++; if (s !== sH) ok = false; }
    if (ok && pH !== null) { cycles++; hasMsb(pH) ? dj++ : lx++; if (p !== pH) ok = false; }
    if (ok && oH !== null) { cycles++; hasMsb(oH) ? dj++ : lx++; if (o !== oH) ok = false; }
    if (ok) matches.push({ s: String(s), p: String(p), o: String(o), c: String(c), m: String(m) });
  }
  return { matches, vm_cycles: cycles, direct_jump_ops: dj, lexicon_lookup_ops: lx };
}

function getU64(view, off) {
  return BigInt(view.getUint32(off, true)) | (BigInt(view.getUint32(off+4, true)) << 32n);
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

function renderResults(result, totalQuins) {
  const vfs = activeVfs;
  document.getElementById('match-count').textContent = `${result.matches.length} triple(s)`;

  const list = document.getElementById('triple-list');
  if (result.matches.length === 0) {
    list.innerHTML = '<div class="no-results">No matching triples found.</div>';
  } else {
    list.innerHTML = result.matches.map(q => {
      const sh = parseBigDecimal(q.s), ph = parseBigDecimal(q.p), oh = parseBigDecimal(q.o);
      const sl = vfs ? esc(vfs.lookup(sh)) : toHex16(sh);
      const pl = vfs ? esc(vfs.lookup(ph)) : toHex16(ph);
      const ol = vfs ? esc(vfs.lookup(oh)) : toHex16(oh);
      return `<div class="triple-row">
        <span class="triple-label">S</span><span class="triple-s">${sl}</span>&nbsp;
        <span class="triple-label">P</span><span class="triple-p">${pl}</span>&nbsp;
        <span class="triple-label">O</span><span class="triple-o">${ol}</span>
      </div>`;
    }).join('');
  }

  updateStatsRow(0, 'VM Cycles',                result.vm_cycles.toLocaleString());
  updateStatsRow(1, 'Direct Jump Ops (did:q42)', result.direct_jump_ops.toLocaleString());
  updateStatsRow(2, 'Lexicon Lookup Ops',         result.lexicon_lookup_ops.toLocaleString());
  updateStatsRow(3, 'Blocks Scanned',            Math.ceil(totalQuins / 850).toLocaleString());

  if (result.matches.length > 0) {
    const q = result.matches[0];
    document.getElementById('raw-quin').textContent =
      `S: 0x${toHex16(parseBigDecimal(q.s))}\nP: 0x${toHex16(parseBigDecimal(q.p))}\n` +
      `O: 0x${toHex16(parseBigDecimal(q.o))}\nC: 0x${toHex16(parseBigDecimal(q.c))}\n` +
      `M: 0x${toHex16(parseBigDecimal(q.m))}`;
  }
}

function showError(msg) {
  document.getElementById('triple-list').innerHTML =
    `<div class="no-results" style="color:var(--error)">${esc(msg)}</div>`;
  document.getElementById('match-count').textContent = 'error';
}

function updateStatsRow(idx, label, value) {
  const rows = document.querySelectorAll('#stats-table tr');
  if (rows[idx]) rows[idx].innerHTML = `<td>${label}</td><td>${value}</td>`;
}

function esc(s) {
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

// ---------------------------------------------------------------------------
// Preset word search
// ---------------------------------------------------------------------------

window.preset = function preset(word) {
  document.getElementById('query-input').value = `?s ?p "${word}"`;
  runQuery();
};

// ---------------------------------------------------------------------------
// Upload / OPFS ingest
// ---------------------------------------------------------------------------

let ingestWorker = null;

window.startIngest = function startIngest() {
  const file = document.getElementById('nt-file').files?.[0];
  if (!file) { alert('Please select an .nt file first.'); return; }
  const status = document.getElementById('ingest-status');
  status.textContent = 'Starting…';
  if (ingestWorker) ingestWorker.terminate();
  ingestWorker = new Worker('./ingest_worker.js');
  ingestWorker.onmessage = ({ data }) => {
    if (data.type === 'progress') {
      status.textContent = `${data.triples.toLocaleString()} triples, ${data.blocks} blocks…`;
    } else if (data.type === 'done') {
      status.textContent = `Done — ${data.triples.toLocaleString()} triples, ${data.lexEntries.toLocaleString()} lexicon entries.`;
      setTimeout(() => location.reload(), 1500);
    } else if (data.type === 'error') {
      status.textContent = 'Error: ' + data.message;
    }
  };
  ingestWorker.postMessage({ type: 'ingest', file });
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function setBadge(id, text, cls) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = text;
  el.className   = `badge ${cls}`;
}

document.getElementById('dataset-select')
  .addEventListener('change', updateDatasetDescription);

// ---------------------------------------------------------------------------
// System Health Console
// ---------------------------------------------------------------------------

function hcVfsLabel() {
  if (dbBytes)    return `In-memory (${(dbBytes.length / 1024).toFixed(0)} KB)`;
  if (activeVfs)  return activeVfs._opfsRoot ? 'OPFS + HTTP Range' : 'Remote HTTP Range';
  return 'none';
}

window.toggleHealthConsole = function toggleHealthConsole() {
  const el  = document.getElementById('health-console');
  const btn = el.querySelector('.hc-toggle');
  el.classList.toggle('collapsed');
  btn.textContent = el.classList.contains('collapsed') ? '▲' : '▼';
};

function hcSet(id, text, cls = '') {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = text;
  el.className   = 'hc-val' + (cls ? ' ' + cls : '');
}

function updateHealthConsole(vmCycles, matchCount, latencyUs, costHeader, bidxUsed) {
  const costText = costHeader
    ? costHeader
    : `${matchCount}+${vmCycles.toLocaleString()}`;
  hcSet('hc-cost',   costText,                           'green');
  hcSet('hc-cycles', vmCycles.toLocaleString() + ' ops', vmCycles > 0 ? 'green' : 'dim');
  hcSet('hc-lat',    latencyUs + ' µs',                  'green');
  hcSet('hc-vfs',    hcVfsLabel(), activeVfs || dbBytes ? 'green' : 'amber');

  if (activeVfs) {
    // BIDX row: persist the "loaded" state; update with per-query lookup info
    if (activeVfs.bidxLoaded) {
      hcSet('hc-bidx',
        bidxUsed == null
          ? `${activeVfs.blockCount.toLocaleString()} ranges · O(log N)`
          : bidxUsed
            ? 'BIDX HIT · O(log N)'
            : 'full scan (no object bound)',
        'green');
    }

    const t = activeVfs.telemetry;
    hcSet('hc-net',
      t.netRequests > 0 ? `${t.netRequests} × 40KB range` : '0',
      t.netRequests > 0 ? 'amber' : 'dim');
    hcSet('hc-opfs',
      t.opfsHits > 0 ? `${t.opfsHits} blocks` : '0',
      t.opfsHits > 0 ? 'green' : 'dim');
    hcSet('hc-fault',
      t.lastFaultMs > 0 ? `${t.lastFaultMs.toFixed(0)} ms last` : '—',
      t.lastFaultMs > 0 ? 'green' : 'dim');
  }
}

async function pollDaemonHealth() {
  const el = document.getElementById('hc-daemon');
  try {
    const r = await fetch('http://127.0.0.1:4242/health', {
      signal: AbortSignal.timeout(800),
    });
    if (r.ok) {
      const data = await r.json().catch(() => ({}));
      const ver  = data.version ? ` v${data.version}` : '';
      if (el) {
        el.innerHTML = `<span class="hc-pulse"></span>ACTIVE${ver}`;
        el.className = 'hc-val green';
      }
    } else {
      hcSet('hc-daemon', 'HTTP ' + r.status, 'amber');
    }
  } catch {
    if (el) {
      el.innerHTML = '<span class="hc-pulse amber"></span>WASM-only';
      el.className = 'hc-val dim';
    }
  }
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

try {
  await Promise.all([loadWasm(), loadManifest()]);
} catch (e) {
  console.error('[WordNet] Boot failed:', e);
  setBadge('mount-badge', 'Boot failed: ' + e.message, 'amber');
  showError('Boot failed: ' + e.message);
}
pollDaemonHealth();
setInterval(pollDaemonHealth, 15_000);
