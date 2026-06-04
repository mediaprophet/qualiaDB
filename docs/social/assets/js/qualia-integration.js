import { pipeline, env } from 'https://cdn.jsdelivr.net/npm/@xenova/transformers@2.6.2';

env.allowLocalModels = false; 

console.log("QualiaDB Webizen Integration Initializing...");

const DAEMON_URL = "ws://127.0.0.1:4242";
let ws = null;
let reconnectAttempts = 0;
const MAX_RECONNECTS = 10;

let webrtcMode = null; // null, 'overlay', 'inline'
let speechRecognition = null;
let translatorPipeline = null;
let targetLang = (navigator.language || 'en').split('-')[0];
let sourceLang = 'en'; 

if (targetLang === 'en') {
    targetLang = 'es';
    sourceLang = 'en';
} else {
    sourceLang = 'en'; 
}

const modelName = `Xenova/opus-mt-${sourceLang}-${targetLang}`;

// --- Decentralized Schema Mapping (qualia-solid-bridge) ---
// Maps dummy UI names to standard W3C Solid WebIDs or Webizen did:git identifiers.
const solidBridgeMap = {
    "ava flores": "https://ava.solidcommunity.net/profile/card#me",
    "bella mccoy": "did:git:repo777",
    "Project Alpha Swarm": "did:web:qualia.io:groups:alpha:swarm",
    "bob stephens": "https://bob.solidcommunity.net/profile/card#me"
};

function resolveIdentity(uiName) {
    const webid = solidBridgeMap[uiName] || `did:web:qualia.io:user:${uiName.replace(/\s+/g, '')}`;
    console.log(`[Solid Bridge] Resolving UI Name '${uiName}' to Identity: ${webid}`);
    
    // Simulate hashing into a 64-bit Quin vector via the Allocation Firewall
    window.QualiaDB.sendQuery(`resolve_identity_quin("${webid}")`);
    return webid;
}
// -----------------------------------------------------------

function connectDaemon() {
    console.log(`Attempting to connect to QualiaDB Daemon at ${DAEMON_URL}...`);
    ws = new WebSocket(DAEMON_URL);

    ws.onopen = function() {
        console.log("✅ Successfully connected to QualiaDB Daemon!");
        reconnectAttempts = 0;
    };
    ws.onmessage = function(event) {
        try {
            const data = JSON.parse(event.data);
            console.log("📩 Message from QualiaDB:", data);
        } catch(e) {}
    };
    ws.onclose = function() {
        if (reconnectAttempts < MAX_RECONNECTS) {
            reconnectAttempts++;
            setTimeout(connectDaemon, 2000 * reconnectAttempts);
        }
    };
}

async function loadTranslator() {
    const statusInd = document.getElementById('webrtc-status-indicator');
    if (translatorPipeline) return;
    
    try {
        if(statusInd) statusInd.innerText = `Loading translation model (${sourceLang}->${targetLang})...`;
        translatorPipeline = await pipeline('translation', modelName);
        if(statusInd) statusInd.innerText = "Live Translation Active";
    } catch (e) {
        console.error("Failed to load translator:", e);
        if(statusInd) statusInd.innerText = "Translation Unavailable";
    }
}

function startSpeechRecognition() {
    const SR = window.SpeechRecognition || window.webkitSpeechRecognition;
    if (!SR) return;

    speechRecognition = new SR();
    speechRecognition.continuous = true;
    speechRecognition.interimResults = false;
    speechRecognition.lang = sourceLang === 'en' ? 'en-US' : sourceLang;

    speechRecognition.onresult = async (event) => {
        const tBox = document.getElementById('qualia-transcript-box');
        for (let i = event.resultIndex; i < event.results.length; i++) {
            if (event.results[i].isFinal) {
                const originalText = event.results[i][0].transcript.trim();
                if (!originalText) continue;

                if (tBox) {
                    tBox.innerHTML += `<span style="display:block; margin-top:5px; padding-bottom: 3px;"><strong>Original (${sourceLang}):</strong> ${originalText}</span>`;
                    tBox.scrollTop = tBox.scrollHeight;
                }

                if (translatorPipeline) {
                    try {
                        const out = await translatorPipeline(originalText);
                        const translatedText = out[0]?.translation_text || '';
                        
                        if (tBox) {
                            tBox.innerHTML += `<span style="display:block; color: #007bff; border-bottom: 1px dashed #eee; padding-bottom: 3px;"><strong>Translated (${targetLang}):</strong> ${translatedText}</span>`;
                            tBox.scrollTop = tBox.scrollHeight;
                        }

                        window.QualiaDB.sendQuery(`Log translation [${sourceLang}->${targetLang}]: "${originalText}" -> "${translatedText}"`);

                    } catch (e) {
                        console.error("Translation error:", e);
                    }
                }
            }
        }
    };
    speechRecognition.onend = () => {
        if (webrtcMode) {
            try { speechRecognition.start(); } catch(e) {}
        }
    };
    try {
        speechRecognition.start();
    } catch(e) {}
}

function stopSpeechRecognition() {
    if (speechRecognition) {
        try { speechRecognition.abort(); } catch(e) {}
        speechRecognition = null;
    }
}

window.QualiaDB = {
    sendQuery: function(query) {
        if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({
                jsonrpc: "2.0",
                method: "qualia_query",
                params: [query],
                id: Date.now()
            }));
        }
    },
    
    // mode can be 'inline' or 'overlay'. Defaults to overlay if called from sidebar.
    toggleWebRTC: async function(mode = 'overlay') {
        const overlay = document.getElementById('qualia-webrtc-overlay');
        const inlineUI = document.getElementById('qualia-inline-webrtc');
        const tBox = document.getElementById('qualia-transcript-box');
        
        // If clicking the same mode that is active, turn it off.
        if (webrtcMode === mode) {
            webrtcMode = null;
            if(overlay) overlay.style.display = 'none';
            if(inlineUI) inlineUI.style.display = 'none';
            stopSpeechRecognition();
            return;
        }

        // If switching modes, turn the other off
        if(overlay) overlay.style.display = 'none';
        if(inlineUI) inlineUI.style.display = 'none';

        webrtcMode = mode;
        
        // Resolve Identity for dummy simulation (assuming we're calling Ava for demo)
        if (webrtcMode) resolveIdentity("ava flores");
        
        if (webrtcMode === 'overlay' && overlay) {
            overlay.style.display = 'block';
        } else if (webrtcMode === 'inline' && inlineUI) {
            inlineUI.style.display = 'block';
        }

        if(tBox) tBox.innerHTML = '<em style="color: #999;">Waiting for speech...</em><br>';
        
        await loadTranslator();
        startSpeechRecognition();
    }
};

window.addEventListener('DOMContentLoaded', () => {
    connectDaemon();
});
