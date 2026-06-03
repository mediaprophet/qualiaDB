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

function generateBillingReceipt() {
    const io = Math.floor(Math.random() * 50) + 10;
    const sieve = Math.floor(Math.random() * 5);
    const vm = Math.floor(Math.random() * 500) + 100;
    
    const io_cost = io * 10;
    const sieve_cost = sieve * 1;
    const vm_cost = vm * 5;
    
    const total_micro_sats = io_cost + sieve_cost + vm_cost;
    const total_sats = Math.ceil(total_micro_sats / 1000000);

    const receipt = {
        query_id: "tx-" + Math.random().toString(36).substring(2, 9),
        superblock_cost: io,
        sieve_ops_cost: sieve,
        vm_cycles_cost: vm,
        total_sats_owed: total_sats
    };

    return JSON.stringify(receipt, null, 2);
}

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

let isNativeConnected = false;
window.QUALIA_NATIVE_ACTIVE = false;

const DAEMON_BASE = 'http://127.0.0.1:4242';

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
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
        const resp = await fetch(`${DAEMON_BASE}/health`, {
            method: 'GET',
            signal: controller.signal
        });
        isNativeConnected = resp.ok;
        window.QUALIA_NATIVE_ACTIVE = resp.ok;
        return resp.ok;
    } catch (_err) {
        // TypeError: Failed to fetch (daemon offline) — swallow silently
        isNativeConnected = false;
        window.QUALIA_NATIVE_ACTIVE = false;
        return false;
    } finally {
        clearTimeout(timer);
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
    const byteOut  = document.getElementById('bytecode-output');
    const quinOut  = document.getElementById('quin-output');
    const billOut  = document.getElementById('billing-receipt');
    const countBadge = document.getElementById('quin-count');

    byteOut.innerText = '[Sentinel VM] Routing query to native daemon…';

    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 5000);

    try {
        const resp = await fetch(`${DAEMON_BASE}/query`, {
            method: 'POST',
            signal: controller.signal,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                query,
                format: query.trim().startsWith('{') ? 'json-ld' : 'sparql-star'
            })
        });
        clearTimeout(timer);

        const data = await resp.json();
        const elapsedNs = Math.floor((performance.now() - startTime) * 1000);

        if (!resp.ok || data.status === 'error') {
            byteOut.innerText = `[Native Error]\n${data.message || resp.statusText}`;
            billOut.innerText = JSON.stringify({ path: 'native', error: data.code }, null, 2);
            return;
        }

        // Bytecode panel — show compile status + routing metadata
        byteOut.innerText = [
            `[Sentinel VM] status: ${data.status}`,
            `[Sentinel VM] format: ${data.format}`,
            data.message ? `[Sentinel VM] ${data.message}` : null,
            data.routing_tier !== undefined
                ? `[Router]      tier: 0b${data.routing_tier.toString(2).padStart(2,'0')}  mask: 0x${(data.validation_mask||0).toString(16).padStart(4,'0')}`
                : null
        ].filter(Boolean).join('\n');

        // Quin result matrix
        if (data.quin) {
            const q = data.quin;
            const hex = v => BigInt(v).toString(16).padStart(16, '0');
            quinOut.innerHTML = `
                <div class="quin-row">
                    <div><span style="color:#888">S:</span> 0x${hex(q.subject)}</div>
                    <div><span style="color:#888">P:</span> 0x${hex(q.predicate)}</div>
                    <div><span style="color:#888">O:</span> 0x${hex(q.object)}</div>
                    <div><span style="color:#888">C:</span> 0x${hex(q.context)}</div>
                    <div><span style="color:#888">M:</span> 0x${hex(q.metadata)}</div>
                </div>`;
            countBadge.innerText = '1 Quin (Native)';
        }

        // Billing panel
        billOut.innerText = JSON.stringify({
            path: 'native',
            routing_tier: data.routing_tier,
            validation_mask: data.validation_mask,
            format: data.format,
            latency_ns: elapsedNs
        }, null, 2);

        document.getElementById('latency-badge').innerText = `${elapsedNs}ns`;

    } catch (err) {
        clearTimeout(timer);
        // Daemon went away mid-request — flip to fallback
        isNativeConnected = false;
        window.QUALIA_NATIVE_ACTIVE = false;
        updateConnectionBadge();
        runFallbackSimulation(query, startTime);
    }
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
        rangeLog += '\n[WASM] Local compile complete.\n\n';
        rangeLog += generateBillingReceipt();

        document.getElementById('billing-receipt').innerText = rangeLog;

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
