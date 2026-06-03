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

function compileToBytecode() {
    const start = performance.now();
    
    // Simulate compilation
    setTimeout(() => {
        document.getElementById('bytecode-output').innerText = generateBytecode();
        document.getElementById('quin-output').innerHTML = generateQuins();
        document.getElementById('quin-count').innerText = "2 Quins";
        
        // Generate billing receipt
        document.getElementById('billing-receipt').innerText = generateBillingReceipt();
        
        const end = performance.now();
        const latencyNs = Math.floor((end - start) * 1000);
        document.getElementById('latency-badge').innerText = `${latencyNs}ns`;
    }, 45); // simulated WASM delay
}

window.QUALIA_NATIVE_ACTIVE = false;

function probeNativeDaemon(timeoutMs = 500) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);

    return fetch('http://127.0.0.1:4242/health', {
        method: 'GET',
        signal: controller.signal
    }).then(res => {
        window.QUALIA_NATIVE_ACTIVE = res.ok;
        return res.ok;
    }).catch(() => {
        window.QUALIA_NATIVE_ACTIVE = false;
        return false;
    }).finally(() => clearTimeout(timeout));
}

function markNativeButtonActive() {
    const installBtn = document.getElementById('install-btn-header');
    if(installBtn) {
        installBtn.style.background = 'rgba(74, 222, 128, 0.1)';
        installBtn.style.color = 'var(--success)';
        installBtn.style.border = '1px solid var(--success)';
        installBtn.innerText = 'Daemon Active (Native)';
        installBtn.onclick = null;
        installBtn.removeAttribute('href');
    }
}

async function retryNativeLaunch() {
    for (let attempt = 0; attempt < 5; attempt++) {
        await new Promise(resolve => setTimeout(resolve, 2000));
        if (await probeNativeDaemon()) {
            markNativeButtonActive();
            return;
        }
    }
}

// Check if Native Daemon is installed and running
probeNativeDaemon().then(active => {
    if(active) {
        markNativeButtonActive();
    } else {
        const installBtn = document.getElementById('install-btn-header');
        if(installBtn) {
            installBtn.innerText = 'Launch Local Engine';
            installBtn.onclick = () => {
                window.location.href = 'qualia://start';
                retryNativeLaunch();
            };
        }
    }
});
