import init, { compile_query_to_json } from './qualia_core_db.js';

const inputEl = document.getElementById('query-input');
const outputEl = document.getElementById('query-output');

// JSON-LD Presets
// Playground updated for SHACL→Sentinel, Defeasible/Omnimodal, Lazy SuperBlock telemetry (see native `qualia-cli bench --suite full`).
const presets = {
    ambient: `{
  "@context": "https://wellfair.org/contexts/health.jsonld",
  "@id": "did:wellfare:user123/telemetry/hr_0942",
  "@type": "HeartRateObservation",
  "subject": "did:wellfare:user123",
  "bpm": 72,
  "timestamp": "1717228382000"
}`,
    
    bilateral: `{
  "@context": "https://wellfair.org/contexts/safeguard.jsonld",
  "@id": "did:wellfare:survivor88/ping/01",
  "@type": "ProtectedLocationAssertion",
  "assertedBy": "did:wellfare:survivor88",
  "sharedWith": "did:wellfare:advocate44",
  "location": "H3:8a2a1072b59ffff",
  "qualia:logicGate": "MASK_BILATERAL_IDENTITY_LOCKED"
}`,
    
    permissive: `{
  "@context": "https://wellfair.org/contexts/research.jsonld",
  "@id": "did:wellfare:dataset/sleep_nsw_2026",
  "@type": "ClinicalCohort",
  "queryAgent": "did:corporate:pharma_corp_99",
  "qualia:obligationStatus": "MASK_COMMERCIAL_BILLABLE_GATE"
}`,
    
    spatiotemporal: `{
  "@context": "https://wellfair.org/contexts/intake.jsonld",
  "@id": "did:wellfare:user123/logs/crisis_01",
  "@type": "AmbiguousIntakeEvent",
  "rawText": "Fled with nothing but my thongs and wallet.",
  "location": "H3:891ea6d82b7ffff",
  "qualia:linguisticState": "MASK_LINGUISTIC_AMBIGUITY"
}`
};

window.loadPreset = function(key) {
    if(presets[key]) {
        inputEl.value = presets[key];
        triggerCompilation();
    }
};

function syntaxHighlight(json) {
    if (typeof json != 'string') {
        json = JSON.stringify(json, undefined, 2);
    }
    json = json.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return json.replace(/("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g, function (match) {
        let cls = 'number';
        if (/^"/.test(match)) {
            if (/:$/.test(match)) {
                cls = 'key';
            } else {
                cls = 'string';
            }
        } else if (/true|false/.test(match)) {
            cls = 'boolean';
        } else if (/null/.test(match)) {
            cls = 'null';
        }
        return '<span class="' + cls + '">' + match + '</span>';
    });
}

function updateVisualizer(tier) {
    // Reset all classes
    document.querySelectorAll('.core-box').forEach(el => {
        el.classList.remove('active', 'paused');
    });

    if (tier === 0) { // 0b00 Ambient
        document.getElementById('core-3').classList.add('active');
        document.getElementById('core-2').classList.add('active');
    } else if (tier === 2) { // 0b10 Bilateral
        document.getElementById('gpu-sieve').classList.add('active');
        document.getElementById('core-1').classList.add('active');
    } else if (tier === 1) { // 0b01 Permissive
        document.getElementById('gpu-sieve').classList.add('active');
        document.getElementById('core-3').classList.add('paused');
    } else if (tier === 3) { // 0b11 Spatiotemporal
        document.getElementById('edge-npu').classList.add('active');
        document.getElementById('core-1').classList.add('active');
    }
}

let isNativeMode = false;
window.QUALIA_NATIVE_ACTIVE = false;

const QUALIA_DAEMON_BASE = 'http://127.0.0.1:4242';

async function fetchWithTimeout(url, options = {}, timeoutMs = 500) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), timeoutMs);
    try {
        return await fetch(url, { ...options, signal: controller.signal });
    } finally {
        clearTimeout(timeout);
    }
}

function nativeHeaders() {
    const headers = { 'Content-Type': 'application/json' };
    const token = localStorage.getItem('qualia_dev_token');
    if (token) {
        headers['X-Qualia-Token'] = token;
    }
    return headers;
}

async function probeNativeDaemon(timeoutMs = 500) {
    try {
        const resp = await fetchWithTimeout(`${QUALIA_DAEMON_BASE}/health`, { method: 'GET' }, timeoutMs);
        window.QUALIA_NATIVE_ACTIVE = resp.ok;
        isNativeMode = resp.ok;
        return resp.ok;
    } catch (_err) {
        window.QUALIA_NATIVE_ACTIVE = false;
        isNativeMode = false;
        return false;
    }
}

async function retryNativeLaunch() {
    for (let attempt = 0; attempt < 5; attempt++) {
        await new Promise(resolve => setTimeout(resolve, 2000));
        if (await probeNativeDaemon(500)) {
            document.querySelector('.status-indicator').innerHTML = 'Native Hardware Connected';
            triggerCompilation();
            return true;
        }
    }
    document.querySelector('.status-indicator').innerHTML = 'WASM Fallback Active';
    return false;
}

async function triggerCompilation() {
    const query = inputEl.value;
    if(!query.trim()) {
        outputEl.innerHTML = "Awaiting compilation...";
        return;
    }

    if (isNativeMode) {
        try {
            const resp = await fetch(`${QUALIA_DAEMON_BASE}/query`, {
                method: 'POST',
                headers: nativeHeaders(),
                body: JSON.stringify({
                    query,
                    format: query.trim().startsWith('{') ? 'json-ld' : 'sparql-star'
                })
            });
            const data = await resp.json();
            
            if (resp.status === 501) {
                outputEl.innerHTML = `<span class="error">Native compiler limit: ${data.message}<br>Use the browser fallback controls for structural compilation.</span>`;
                return;
            } else if (!resp.ok || data.status === 'error') {
                outputEl.innerHTML = `<span class="error">Native Error: ${data.message || resp.statusText}</span>`;
                return;
            } else {
                outputEl.innerHTML = syntaxHighlight(data);
                if(data.routing_tier !== undefined) {
                    updateVisualizer(data.routing_tier);
                }
                document.querySelector('.status-indicator').innerHTML = 'Native Hardware Connected';
                window.QUALIA_NATIVE_ACTIVE = true;
                return;
            }
        } catch (e) {
            console.warn("Native link failed, falling back to WASM", e);
            isNativeMode = false;
            window.QUALIA_NATIVE_ACTIVE = false;
        }
    }

    // WASM Fallback
    try {
        const resultString = compile_query_to_json(query);
        const jsonObj = JSON.parse(resultString);
        
        if(jsonObj.error) {
            outputEl.innerHTML = `<span class="error">${jsonObj.error}</span>`;
        } else {
            outputEl.innerHTML = syntaxHighlight(jsonObj);
            if(jsonObj.routing_tier !== undefined) {
                updateVisualizer(jsonObj.routing_tier);
            }
            document.querySelector('.status-indicator').innerHTML = 'WASM Fallback Active';
        }
    } catch (e) {
        outputEl.innerHTML = `<span class="error">Compilation error: ${e}</span>`;
    }
}

// Initialize Loopback or WASM
async function bootstrap() {
    if (await probeNativeDaemon(500)) {
        console.log("Qualia Native Loopback initialized.");
        document.querySelector('.status-indicator').innerHTML = 'Native Hardware Connected';
    } else {
        console.warn("Native Loopback not found. Falling back to WASM engine.");
        try {
            await init();
            console.log("Qualia WASM Engine initialized.");
            document.querySelector('.status-indicator').innerHTML = 'WASM Loaded';
        } catch(e) {
            outputEl.innerHTML = `<span class="error">Failed to load WASM module: ${e}</span>`;
            return;
        }
    }

    const launchLink = document.querySelector('a[href="qualia://start"]');
    if (launchLink) {
        launchLink.addEventListener('click', () => {
            document.querySelector('.status-indicator').innerHTML = 'Launching Local Engine...';
            retryNativeLaunch();
        });
    }
    
    inputEl.addEventListener('input', triggerCompilation);
    loadPreset('ambient');
}

bootstrap();
