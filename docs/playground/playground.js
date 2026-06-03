const PRESETS = {
    ambient: `{
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@id": "did:wellfare:user123/telemetry/hr_0942",
  "@type": "HeartRateObservation",
  "subject": "did:wellfare:user123",
  "bpm": 72,
  "timestamp": "1717228382000"
}`,
    bilateral: `{
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@graph": [
    {
      "@id": "qualia:rule_1",
      "type": "n3:Implication",
      "antecedent": { "?x": "a", "object": "Person" },
      "consequent": { "?x": "a", "object": "Mortal" },
      "routing_tier": "Bilateral Micro-Commons (0b10)"
    }
  ]
}`,
    permissive: `{
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@graph": [
    {
      "@id": "qualia:dataset_44",
      "type": "SensorData",
      "value": 42.5,
      "routing_tier": "Permissive Commons (0b01)"
    }
  ]
}`,
    spatiotemporal: `{
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@id": "did:wellfare:user123/logs/crisis_01",
  "@type": "AmbiguousIntakeEvent",
  "rawText": "Fled with nothing but my thongs and wallet.",
  "location": "H3:891ea6d82b7ffff",
  "qualia:linguisticState": "MASK_LINGUISTIC_AMBIGUITY"
}`,
    hubmap: `{
  "@context": "https://hubmapconsortium.github.io/hubmap-ontology/ccf-context.jsonld",
  "@graph": [
    {
      "@id": "http://purl.org/ccf/latest/ccf.owl#VHF_Left_Kidney",
      "@type": "ccf:SpatialEntity",
      "ccf:x_dimension": 63.5,
      "ccf:y_dimension": 113.8,
      "ccf:z_dimension": 55.2,
      "ccf:dimension_unit": "millimeter",
      "ccf:placement": {
        "@type": "ccf:SpatialPlacement",
        "ccf:x_translation": -51.0,
        "ccf:y_translation": 125.0,
        "ccf:z_translation": -23.0,
        "ccf:translation_unit": "millimeter",
        "ccf:x_rotation": 15.0,
        "ccf:y_rotation": 0.0,
        "ccf:z_rotation": 0.0,
        "ccf:placement_for": "http://purl.org/ccf/latest/ccf.owl#VHF_Left_Kidney_Volume"
      },
      "routing_tier": "Spatiotemporal Ambiguous (0b11)"
    }
  ]
}`,
    llm_vector: `{
  "@context": "https://mediaprophet.github.io/qualiaDB/qualia-context.jsonld",
  "@graph": [
    {
      "@id": "llm:llama3:layer_12:attention_head_4:vector_8992",
      "@type": "LatentSpaceVector",
      "qualia:hasInterpretabilityNote": "This vector strongly activates on concepts of 'water' and 'fluid dynamics'.",
      "qualia:annotatedBy": "did:researcher:ai_safety_team",
      "qualia:containerSession": "container:session_abc123:prompt_hash_xyz",
      "routing_tier": "Bilateral Micro-Commons (0b10)"
    }
  ]
}`
};

function loadPreset(key) {
    document.getElementById('jsonld-input').value = PRESETS[key];
}

function hashString(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash).toString(16).padStart(16, '0');
}

function generateBytecode() {
    return `[SentinelCompiler] Compiling N3Logic AST...
-> OP_MATCH_SUBJECT(0x${hashString("?x")})
-> OP_MATCH_PREDICATE(0x${hashString("a")})
-> OP_MATCH_OBJECT(0x${hashString("Person")})
-> OP_BIND_VAR(0)
-> OP_EVAL_METADATA_MASK(0b10)
-> OP_HALT_IF_FALSE
[Core 1] Bytecode generated successfully.`;
}

function generateQuins() {
    const s = hashString("qualia:rule_1");
    const p = hashString("type");
    const o = hashString("n3:Implication");
    const c = "0000000000000000";
    const m = "8000000000000002"; // 0b10 routing

    return `
        <div class="quin-row">
            <div><span style="color: #888;">S:</span> 0x${s}</div>
            <div><span style="color: #888;">P:</span> 0x${p}</div>
            <div><span style="color: #888;">O:</span> 0x${o}</div>
            <div><span style="color: #888;">C:</span> 0x${c}</div>
            <div><span style="color: #888;">M:</span> 0x${m}</div>
        </div>
        <div class="quin-row">
            <div><span style="color: #888;">S:</span> 0x${hashString("?x")}</div>
            <div><span style="color: #888;">P:</span> 0x${hashString("a")}</div>
            <div><span style="color: #888;">O:</span> 0x${hashString("Person")}</div>
            <div><span style="color: #888;">C:</span> 0x${c}</div>
            <div><span style="color: #888;">M:</span> 0x${m}</div>
        </div>
    `;
}

// ---------------------------------------------------------------------------
// Permissive Commons Compute-Cost widget
// ---------------------------------------------------------------------------
//
// Unit economics (adjustable via the slider in the UI):
//   UNIT_PRICE_ΜSAT_PER_1K = μsat cost per 1 000 VM cycles
//   1 sat = 1 000 000 μsat  (1 μsat = 1 × 10⁻⁶ sat)
//   BTC/USD rate used for display only — does not affect the sat cost.

const COST_STATE = {
    unitPriceMuSatPer1k: 1,   // default: 1 μsat per 1 000 VM cycles
    btcUsd: 70_000,           // reference rate for fiat display
};

/**
 * Render the compute-cost billing widget into `billOut`.
 *
 * @param {HTMLElement} billOut
 * @param {{ path: string, vmCycles?: number, matchCount?: number,
 *           latencyNs?: number, error?: string, [k: string]: any }} data
 */
function renderCostWidget(billOut, data) {
    const cycles   = Number(data.vmCycles ?? data.vm_compute_cost ?? 0);
    const matches  = Number(data.matchCount ?? data.match_count ?? 0);
    const latNs    = Number(data.latencyNs ?? 0);
    const path     = data.path ?? 'unknown';
    const price    = COST_STATE.unitPriceMuSatPer1k;

    // μsat cost = cycles × (price / 1000)
    const costMuSat = cycles * price / 1_000;
    const costSat   = costMuSat / 1_000_000;
    const costUsd   = costSat * COST_STATE.btcUsd / 100_000_000; // sat → BTC → USD

    const qid = 'tx-' + Math.random().toString(36).slice(2, 9);

    const pad = (s, w) => String(s).padStart(w);
    const fmt = n => n.toLocaleString();

    // Price-rate slider element (created once, stored in state)
    const sliderHtml = `
<label style="font-size:0.72rem;color:#888;display:flex;align-items:center;gap:8px;margin-top:8px">
  Unit price
  <input type="range" id="cost-slider" min="0" max="4" step="1"
         value="${['0.001','0.01','0.1','1','10'].indexOf(String(price)) !== -1
                   ? ['0.001','0.01','0.1','1','10'].indexOf(String(price))
                   : 3}"
         style="width:80px;accent-color:#ffd700"
         oninput="COST_STATE.unitPriceMuSatPer1k=[0.001,0.01,0.1,1,10][+this.value];
                  document.getElementById('slider-label').textContent=
                  [0.001,0.01,0.1,1,10][+this.value]+'μsat/1k ops'" />
  <span id="slider-label">${price}μsat/1k ops</span>
</label>`;

    billOut.innerHTML = `<pre style="margin:0;font-family:'Fira Code',monospace;font-size:12px;line-height:1.8;color:#e0e0e0">
<span style="color:#ffd700">─── Permissive Commons Billing ───────────────</span>
  Query ID   ${qid}
  Path       ${path}
  Matches    ${fmt(matches)} result(s)
  Latency    ${latNs > 0 ? fmt(latNs) + ' ns' : '—'}
<span style="color:#ffd700">─── VM Execution Cost ────────────────────────</span>
  VM Cycles  ${fmt(cycles)}
  Rate       ${price} μsat / 1 000 ops
<span style="color:#ffd700">─── Cost Breakdown ───────────────────────────</span>
  μsat       ${costMuSat.toFixed(3)}
  sat        ${costSat.toFixed(9)}
  BTC        ${(costSat / 1e8).toFixed(12)}
  USD ≈      $${costUsd.toExponential(2)}  (@ $${fmt(COST_STATE.btcUsd)}/BTC)
<span style="color:#4ade80">─── Status: INVOICE READY ────────────────────</span></pre>
${sliderHtml}`;
}

/**
 * Render the fallback/WASM cost widget with a synthetic cycle estimate
 * derived from the number of blocks scanned and pattern complexity.
 */
function generateBillingReceipt() {
    const io    = Math.floor(Math.random() * 50) + 10;
    const sieve = Math.floor(Math.random() * 5);
    const vm    = Math.floor(Math.random() * 500) + 100;

    // Synthetic cycle count: 1 cycle per Quin evaluated
    const syntheticCycles = io * 850 + vm;

    const billOut = document.getElementById('billing-receipt');
    if (billOut) {
        renderCostWidget(billOut, {
            path: 'wasm-fallback',
            vmCycles: syntheticCycles,
            matchCount: sieve,
            latencyNs: 0,
        });
    }

    // Also return legacy string for callers that still use the return value.
    return JSON.stringify({
        query_id: 'tx-' + Math.random().toString(36).slice(2, 9),
        synthetic_cycles: syntheticCycles,
        path: 'wasm-fallback',
    }, null, 2);
}

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

let isNativeConnected = false;
window.QUALIA_NATIVE_ACTIVE = false;

const DAEMON_BASE = 'http://127.0.0.1:4242';

// ---------------------------------------------------------------------------
// Token storage  (pairing logic — written by qualia-cli daemon token pair)
// ---------------------------------------------------------------------------

const TOKEN_KEY = 'qualia_x_token';

/** Read the stored X-Qualia-Token, or return an empty string. */
function getStoredToken() {
    try { return localStorage.getItem(TOKEN_KEY) || ''; }
    catch (_) { return ''; }
}

/**
 * Build an AbortSignal that fires after `ms` milliseconds.
 * Uses `AbortSignal.timeout()` (Chrome 103+, FF 100+, Safari 16+) where
 * available; falls back to the AbortController pattern for older browsers.
 * @param {number} ms
 * @returns {AbortSignal}
 */
function makeAbortSignal(ms) {
    if (typeof AbortSignal.timeout === 'function') return AbortSignal.timeout(ms);
    const ctrl = new AbortController();
    setTimeout(() => ctrl.abort(), ms);
    return ctrl.signal;
}

/**
 * Heuristically detect the query format and return the appropriate
 * `Accept` header value and JSON `format` key for the daemon.
 * @param {string} query
 * @returns {{ format: string, accept: string }}
 */
function detectQueryFormat(query) {
    const t = query.trim();
    // N-Triples pattern: starts with a < IRI or a ?variable
    if (/^[<?]/.test(t)) {
        return { format: 'n-triples', accept: 'application/n-triples' };
    }
    return { format: 'json-ld', accept: 'application/ld+json' };
}

// ---------------------------------------------------------------------------
// Badge / wake-button toggle
// ---------------------------------------------------------------------------

function updateConnectionBadge() {
    const badge  = document.getElementById('conn-badge');
    const wakeBtn = document.getElementById('wake-btn');
    if (!badge) return;

    if (isNativeConnected) {
        badge.className = 'badge green';
        badge.textContent = 'Native Hardware Connected (Zero-Allocation Core)';
        if (wakeBtn) wakeBtn.style.display = 'none';

        // Also update the install button if present
        const installBtn = document.getElementById('install-btn-header');
        if (installBtn) {
            installBtn.style.background = 'rgba(74, 222, 128, 0.1)';
            installBtn.style.color = 'var(--success)';
            installBtn.style.border = '1px solid var(--success)';
            installBtn.textContent = 'Daemon Active';
        }
    } else {
        badge.className = 'badge amber';
        badge.textContent = 'WASM Fallback Mode (Network Streaming)';
        if (wakeBtn) wakeBtn.style.display = '';
    }
}

// ---------------------------------------------------------------------------
// Daemon probe (single shot, returns bool, swallows TypeError silently)
// ---------------------------------------------------------------------------

async function probeNativeDaemon(timeoutMs = 500) {
    // GET /health is a credential-free probe — no X-Qualia-Token sent here.
    // Sending custom headers on the probe would trigger a CORS preflight for
    // every poll tick, adding latency without any security benefit.
    try {
        const resp = await fetch(`${DAEMON_BASE}/health`, {
            method: 'GET',
            signal: makeAbortSignal(timeoutMs)
        });
        isNativeConnected = resp.ok;
        window.QUALIA_NATIVE_ACTIVE = resp.ok;
        return resp.ok;
    } catch (_err) {
        // TypeError / AbortError — daemon offline or timed out; swallow silently.
        isNativeConnected = false;
        window.QUALIA_NATIVE_ACTIVE = false;
        return false;
    } finally {
        updateConnectionBadge();
    }
}

// ---------------------------------------------------------------------------
// Background polling — every 3 s
// ---------------------------------------------------------------------------

function pollNativeDaemon() {
    setInterval(async () => {
        const prev = isNativeConnected;
        await probeNativeDaemon(500);
        // updateConnectionBadge already called inside probeNativeDaemon;
        // only log state transitions to avoid console noise
        if (prev !== isNativeConnected) {
            console.log(
                isNativeConnected
                    ? '[Qualia] Native daemon connected.'
                    : '[Qualia] Native daemon offline — falling back to WASM.'
            );
        }
    }, 3000);
}

// ---------------------------------------------------------------------------
// Wake-button handler (custom protocol + retry)
// ---------------------------------------------------------------------------

function handleWakeClick(evt) {
    evt.preventDefault();
    window.location.href = 'qualia://start';
    const badge = document.getElementById('conn-badge');
    if (badge) {
        badge.className = 'badge amber';
        badge.textContent = 'Launching Local Engine…';
    }
    // Retry probe up to 5× with 2 s gap
    (async () => {
        for (let i = 0; i < 5; i++) {
            await new Promise(r => setTimeout(r, 2000));
            if (await probeNativeDaemon(500)) return;
        }
    })();
}

// ---------------------------------------------------------------------------
// Native query execution
// ---------------------------------------------------------------------------

async function executeNativeQuery(query, startTime) {
    const byteOut    = document.getElementById('bytecode-output');
    const quinOut    = document.getElementById('quin-output');
    const billOut    = document.getElementById('billing-receipt');
    const countBadge = document.getElementById('quin-count');

    byteOut.innerText = '[Sentinel VM] Routing query to native daemon…';

    const { format, accept } = detectQueryFormat(query);

    // Build headers.  X-Qualia-Token is in the daemon's CORS allow_headers list,
    // so sending it triggers a preflight that the daemon already handles.
    // Only include the token when one is stored — an absent header is cleaner
    // than an empty one from a CORS perspective.
    const headers = {
        'Content-Type': 'application/json',
        'Accept': accept
    };
    const storedToken = getStoredToken();
    if (storedToken) headers['X-Qualia-Token'] = storedToken;

    try {
        const resp = await fetch(`${DAEMON_BASE}/query`, {
            method: 'POST',
            signal: makeAbortSignal(5000),
            headers,
            body: JSON.stringify({ query, format })
        });

        // Read the compute-cost telemetry header before consuming the body.
        const computeCost = resp.headers.get('X-Qualia-Compute-Cost') || '—';
        const elapsedNs   = Math.floor((performance.now() - startTime) * 1000);
        const contentType = resp.headers.get('content-type') || '';

        // ── Error response (always JSON) ────────────────────────────────────
        if (!resp.ok) {
            let errBody;
            try { errBody = await resp.json(); } catch (_) { errBody = {}; }
            byteOut.innerText = `[Native Error ${resp.status}]\n${errBody.message || resp.statusText}`;
            billOut.innerText = JSON.stringify(
                { path: 'native', error: errBody.code, status: resp.status },
                null, 2
            );
            return;
        }

        // ── N-Triples text response ─────────────────────────────────────────
        if (contentType.includes('application/n-triples')) {
            const text  = await resp.text();
            const lines = text.trim().split('\n').filter(Boolean);

            byteOut.innerText = [
                `[Sentinel VM] N-Triples result — ${lines.length} triple(s)`,
                `[Sentinel VM] compute_cost: ${computeCost}`,
                '',
                ...lines.slice(0, 20) // show first 20 lines in bytecode panel
            ].join('\n');

            quinOut.innerHTML = lines.map(line =>
                `<div class="quin-row" style="grid-template-columns:1fr;">
                    <div style="font-family:monospace;font-size:11px;word-break:break-all;">${escHtml(line)}</div>
                 </div>`
            ).join('');
            countBadge.innerText = `${lines.length} Triple(s) (Native)`;

            renderCostWidget(billOut, {
                path: 'native/n-triples',
                vmCycles: Number(computeCost) || 0,
                matchCount: lines.length,
                latencyNs: elapsedNs,
            });

        // ── JSON-LD / default response ──────────────────────────────────────
        } else {
            const data = await resp.json();
            const graph = data['@graph'] || [];
            const matchCount = data.match_count ?? graph.length;

            byteOut.innerText = [
                `[Sentinel VM] JSON-LD result — ${matchCount} Quin(s)`,
                `[Sentinel VM] compute_cost: ${computeCost}`,
                `[Sentinel VM] @context: ${JSON.stringify(data['@context'] ?? {})}`
            ].join('\n');

            if (graph.length > 0) {
                const hex = v => {
                    try { return BigInt(v).toString(16).padStart(16, '0'); }
                    catch (_) { return String(v).padStart(16, '0'); }
                };
                quinOut.innerHTML = graph.map(q => `
                    <div class="quin-row">
                        <div><span style="color:#888">S:</span> 0x${hex(q.subject)}</div>
                        <div><span style="color:#888">P:</span> 0x${hex(q.predicate)}</div>
                        <div><span style="color:#888">O:</span> 0x${hex(q.object)}</div>
                        <div><span style="color:#888">C:</span> 0x${hex(q.context)}</div>
                        <div><span style="color:#888">M:</span> 0x${hex(q.metadata)}</div>
                    </div>`).join('');
                countBadge.innerText = `${matchCount} Quin(s) (Native)`;
            } else {
                quinOut.innerHTML =
                    '<div style="color:#666;text-align:center;margin-top:2rem">No matching Quins.</div>';
                countBadge.innerText = '0 Quins (Native)';
            }

            renderCostWidget(billOut, {
                path: 'native/json-ld',
                vmCycles: Number(computeCost) || 0,
                matchCount,
                latencyNs: elapsedNs,
            });
        }

        document.getElementById('latency-badge').innerText = `${elapsedNs}ns`;

    } catch (err) {
        // AbortError or network failure mid-request — flip badge and fall back.
        isNativeConnected = false;
        window.QUALIA_NATIVE_ACTIVE = false;
        updateConnectionBadge();
        console.warn('[Qualia] Native bridge offline; executing via WASM-Fallback.', err);
        runFallbackSimulation(query, startTime);
    }
}

/** Minimal HTML escaping for safe insertion into innerHTML. */
function escHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ---------------------------------------------------------------------------
// WASM / static fallback — logs HTTP Range requests to telemetry panel
// ---------------------------------------------------------------------------

function runFallbackSimulation(query, startTime) {
    setTimeout(() => {
        document.getElementById('bytecode-output').innerText = generateBytecode();
        document.getElementById('quin-output').innerHTML = generateQuins();
        document.getElementById('quin-count').innerText = '2 Quins (WASM)';

        // Phase 1 byte-range inspector: show the Range requests that would be
        // issued to stream the graph without a full download
        const blockSize = 40960;
        const numBlocks = 3;
        let rangeLog = '[Byte-Range Inspector] WASM fallback — streaming via HTTP ranges:\n';
        rangeLog += 'GET /Qualia_Ledger.q42\n';
        for (let i = 0; i < numBlocks; i++) {
            const lo = i * blockSize;
            const hi = lo + blockSize - 1;
            rangeLog += `  Range: bytes=${lo}-${hi}  (Block sector ${i}, ${blockSize} B)\n`;
        }
        rangeLog += '\n[WASM] Local compile complete.\n';
        document.getElementById('bytecode-output').innerText += '\n' + rangeLog;

        // Cost widget renders itself into #billing-receipt
        generateBillingReceipt();

        const elapsedNs = Math.floor((performance.now() - startTime) * 1000);
        document.getElementById('latency-badge').innerText = `${elapsedNs}ns`;
    }, 45);
}

// ---------------------------------------------------------------------------
// Main execute button handler
// ---------------------------------------------------------------------------

async function compileToBytecode() {
    const query = (document.getElementById('jsonld-input').value || '').trim();
    if (!query) return;

    const start = performance.now();

    if (isNativeConnected) {
        await executeNativeQuery(query, start);
    } else {
        // Daemon is offline — log once to the console so telemetry can pick it up.
        console.warn('[Qualia] Native bridge offline; executing via WASM-Fallback.');
        runFallbackSimulation(query, start);
    }
}

// ---------------------------------------------------------------------------
// Boot: probe once immediately, then start 3 s polling loop
// ---------------------------------------------------------------------------

probeNativeDaemon(500).then(() => {
    updateConnectionBadge();
});

pollNativeDaemon();
