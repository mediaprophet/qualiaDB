const PRESETS = {
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
        document.getElementById('latency-badge').innerText = \`\${(end - start).toFixed(1)}ms\`;
    }, 45); // simulated WASM delay
}
