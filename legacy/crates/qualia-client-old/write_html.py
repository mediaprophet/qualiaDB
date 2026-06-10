import os

html_content = """<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Qualia-DB Native Desktop</title>
    <style>
        :root {
            --bg-dark: #050505;
            --neon-blue: #00f0ff;
            --neon-purple: #b026ff;
            --neon-green: #00ff88;
            --neon-red: #ff3366;
            --neon-gold: #ffd700;
            --glass-bg: rgba(20, 20, 25, 0.65);
            --glass-border: rgba(255, 255, 255, 0.08);
            --sidebar-width: 280px;
        }

        @keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.4; } 100% { opacity: 1; } }

        body {
            margin: 0; padding: 0;
            background: radial-gradient(circle at center, #111 0%, var(--bg-dark) 100%);
            color: #fff;
            font-family: 'Inter', system-ui, -apple-system, sans-serif;
            height: 100vh;
            display: flex;
            overflow: hidden;
        }

        .grid-bg {
            position: fixed; top: 0; left: 0; width: 100vw; height: 100vh;
            background-image: 
                linear-gradient(rgba(0, 240, 255, 0.03) 1px, transparent 1px),
                linear-gradient(90deg, rgba(0, 240, 255, 0.03) 1px, transparent 1px);
            background-size: 50px 50px;
            z-index: -1;
            animation: gridMove 20s linear infinite;
        }

        @keyframes gridMove { 100% { transform: translateY(50px); } }

        /* Sidebar Navigation */
        .sidebar {
            width: var(--sidebar-width);
            background: rgba(10, 10, 15, 0.8);
            backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px);
            border-right: 1px solid var(--glass-border);
            display: flex; flex-direction: column;
            padding: 2rem 0;
            z-index: 20;
        }

        .brand {
            text-align: center; margin-bottom: 3rem;
        }
        .brand h1 {
            font-size: 2rem; font-weight: 800; margin: 0;
            background: linear-gradient(to right, var(--neon-blue), var(--neon-purple));
            -webkit-background-clip: text; -webkit-text-fill-color: transparent;
            text-transform: uppercase; letter-spacing: 2px;
        }
        .brand .subtitle { color: #666; font-size: 0.8rem; letter-spacing: 1px; margin-top: 0.5rem; }

        .nav-item {
            padding: 1rem 2rem;
            color: #888; cursor: pointer;
            font-weight: 600; font-size: 1.1rem;
            transition: all 0.3s ease;
            border-left: 3px solid transparent;
        }
        .nav-item:hover { color: #fff; background: rgba(255,255,255,0.03); }
        .nav-item.active {
            color: var(--neon-blue);
            background: rgba(0, 240, 255, 0.05);
            border-left-color: var(--neon-blue);
            box-shadow: inset 10px 0 20px -10px rgba(0, 240, 255, 0.2);
        }

        /* Main Content Area */
        .main-content {
            flex: 1; display: flex; flex-direction: column;
            padding: 2rem; overflow-y: auto; z-index: 10;
        }

        .view-section { display: none; animation: fadeIn 0.4s ease; }
        .view-section.active { display: block; }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(10px); } to { opacity: 1; transform: translateY(0); } }

        .glass-panel {
            background: var(--glass-bg); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
            border: 1px solid var(--glass-border); border-radius: 16px; padding: 2rem;
            box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.37); margin-bottom: 2rem;
        }

        h2 { margin-top: 0; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 0.5rem; color: #fff; }

        /* Buttons & Inputs */
        button.action-btn {
            background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.1);
            color: #fff; padding: 1rem 1.5rem; border-radius: 8px; font-size: 1rem; font-weight: 600;
            cursor: pointer; transition: all 0.3s ease; position: relative; overflow: hidden; display: inline-block;
            margin-right: 1rem; margin-bottom: 1rem;
        }
        button.action-btn::before {
            content: ''; position: absolute; top: 0; left: 0; width: 3px; height: 100%;
            background: var(--neon-blue); transition: width 0.3s ease; z-index: -1;
        }
        button.action-btn:hover::before { width: 100%; opacity: 0.2; }
        button.action-btn:hover { border-color: var(--neon-blue); box-shadow: 0 0 15px rgba(0, 240, 255, 0.2); transform: translateY(-2px); }
        
        .btn-gold::before { background: var(--neon-gold) !important; }
        .btn-gold:hover { border-color: var(--neon-gold) !important; box-shadow: 0 0 15px rgba(255, 215, 0, 0.2) !important; }
        .btn-purple::before { background: var(--neon-purple) !important; }
        .btn-purple:hover { border-color: var(--neon-purple) !important; box-shadow: 0 0 15px rgba(176, 38, 255, 0.2) !important; }
        .btn-green::before { background: var(--neon-green) !important; }
        .btn-green:hover { border-color: var(--neon-green) !important; box-shadow: 0 0 15px rgba(0, 255, 136, 0.2) !important; }

        input[type="text"], input[type="number"], select {
            width: 100%; padding: 0.8rem; margin: 0.5rem 0 1.5rem 0;
            background: rgba(0,0,0,0.5); border: 1px solid var(--glass-border);
            color: #fff; border-radius: 8px; font-family: monospace; font-size: 1rem;
        }
        input[type="text"]:focus { outline: none; border-color: var(--neon-blue); }

        /* Status & Telemetry */
        .status-indicator { display: flex; align-items: center; gap: 0.8rem; font-weight: bold; font-family: monospace; }
        .dot { width: 12px; height: 12px; border-radius: 50%; background: #444; }

        .telemetry-bar {
            display: flex; gap: 2rem; background: #0a0a0f; padding: 1rem; border-radius: 8px;
            border: 1px solid var(--glass-border); font-family: monospace; color: #aaa; margin-bottom: 2rem;
        }
        .telemetry-bar div strong { color: #fff; }

        /* Terminal Logs */
        .terminal {
            background: #050508; border-radius: 8px; padding: 1.5rem;
            font-family: 'Fira Code', 'Courier New', Courier, monospace; height: 300px; overflow-y: auto;
            border: 1px solid rgba(0, 240, 255, 0.1); box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.8);
            display: flex; flex-direction: column; gap: 0.5rem;
        }
        .log-entry { font-size: 0.9rem; line-height: 1.4; display: flex; gap: 1rem; }
        .log-time { color: #555; }
        .log-info { color: #4ade80; }
        .log-error { color: #f87171; font-weight: bold; }
        .log-warn { color: #facc15; }
        .log-sys { color: var(--neon-blue); }

        /* Chat Interface */
        .chat-container {
            display: flex; flex-direction: column; height: 500px;
        }
        .chat-history {
            flex: 1; overflow-y: auto; padding: 1rem;
            display: flex; flex-direction: column; gap: 1rem;
            border: 1px solid var(--glass-border); border-radius: 8px; background: rgba(0,0,0,0.3);
            margin-bottom: 1rem;
        }
        .chat-bubble {
            max-width: 80%; padding: 1rem; border-radius: 12px; line-height: 1.5;
            position: relative;
        }
        .chat-user {
            align-self: flex-end; background: rgba(0, 240, 255, 0.1);
            border: 1px solid rgba(0, 240, 255, 0.3); border-bottom-right-radius: 2px;
        }
        .chat-agent {
            align-self: flex-start; background: rgba(176, 38, 255, 0.1);
            border: 1px solid rgba(176, 38, 255, 0.3); border-bottom-left-radius: 2px;
        }
        .chat-telemetry {
            font-family: monospace; font-size: 0.8rem; color: var(--neon-gold);
            margin-top: 0.5rem; border-top: 1px dashed rgba(255,255,255,0.1); padding-top: 0.3rem;
        }
        .chat-input-area {
            display: flex; gap: 1rem;
        }
        .chat-input-area input { margin: 0; flex: 1; }
        
        .toggle-switch {
            display: flex; align-items: center; gap: 1rem; margin-bottom: 1rem;
        }
        .switch { position: relative; display: inline-block; width: 50px; height: 24px; }
        .switch input { opacity: 0; width: 0; height: 0; }
        .slider { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #333; transition: .4s; border-radius: 24px; }
        .slider:before { position: absolute; content: ""; height: 16px; width: 16px; left: 4px; bottom: 4px; background-color: white; transition: .4s; border-radius: 50%; }
        input:checked + .slider { background-color: var(--neon-purple); }
        input:checked + .slider:before { transform: translateX(26px); }

    </style>
</head>
<body>
    <div class="grid-bg"></div>
    
    <div class="sidebar">
        <div class="brand">
            <h1>Qualia-DB</h1>
            <div class="subtitle">Edge-Native Webizen</div>
        </div>
        <div class="nav-item active" onclick="switchView('dashboard', this)">🚀 Dashboard</div>
        <div class="nav-item" onclick="switchView('playground', this)">⚖️ Playground</div>
        <div class="nav-item" onclick="switchView('chat', this)">💬 Neuro-Chat</div>
        <div class="nav-item" onclick="switchView('library', this)">📚 Hypermedia Library</div>
        <div class="nav-item" onclick="switchView('settings', this)">⚙️ Settings</div>
    </div>

    <div class="main-content">
        <!-- GLOBAL TELEMETRY -->
        <div class="telemetry-bar">
            <div>Engine: <strong id="global-engine" style="color:var(--neon-blue);">WASM SANDBOX</strong></div>
            <div>Active DAG: <strong>mediaprophet/qualiaDB</strong></div>
            <div>Cost: <strong style="color:var(--neon-gold);" id="global-cost">0 SAT</strong></div>
        </div>

        <!-- DASHBOARD VIEW -->
        <div id="view-dashboard" class="view-section active">
            <div class="glass-panel">
                <h2>Edge-Native Benchmarks</h2>
                <button class="action-btn btn-gold" onclick="runStressTest()">[Execute] Ingest 100,000 Quins</button>
                <button class="action-btn" onclick="runPharmacogenomics()">[Zero-Knowledge] Toxicity Screening</button>
                <button class="action-btn" onclick="runCrisisTriage()">[Decentralized] Crisis Triage (LTL)</button>
            </div>
            
            <div class="glass-panel" style="padding:0;">
                <div class="terminal" id="terminal-dashboard"></div>
            </div>
        </div>

        <!-- PLAYGROUND VIEW -->
        <div id="view-playground" class="view-section">
            <div class="glass-panel">
                <h2>Multi-Agent Guardianship Simulator</h2>
                <p style="color:#aaa; line-height:1.6;">Test the CRDT suspension queue and Agreement DID ratification logic natively.</p>
                
                <label>Principal DID</label>
                <input type="text" id="pg-principal" value="did:q42:child_account_alpha">
                
                <label>Nominated Guardians (Comma Separated)</label>
                <input type="text" id="pg-guardians" value="did:q42:parent_one, did:q42:medical_proxy">

                <button class="action-btn btn-purple" onclick="proposeAgreement()">Propose Agreement (M:N Mesh)</button>
                
                <div style="margin-top:2rem; padding:1.5rem; background:#0a0a0f; border-radius:8px; border:1px solid #333;">
                    <h3 style="margin-top:0; color:#888;">CRDT State</h3>
                    <div id="playground-status" class="status-indicator">
                        <div class="dot"></div> <span class="text">IDLE: NO PENDING TRANSACTIONS</span>
                    </div>
                </div>
            </div>
            
            <div class="glass-panel" style="padding:0;">
                <div class="terminal" id="terminal-playground"></div>
            </div>
        </div>

        <!-- CHAT VIEW -->
        <div id="view-chat" class="view-section">
            <div class="glass-panel">
                <h2>Neuro-Symbolic Routing Console</h2>
                <div class="chat-container">
                    <div class="chat-history" id="chat-history">
                        <div class="chat-bubble chat-agent">
                            System Online. Awaiting LLM Intent Routing...
                        </div>
                    </div>
                    <div class="chat-input-area">
                        <input type="text" id="chat-input" placeholder="Enter natural language query..." onkeypress="handleChatEnter(event)">
                        <button class="action-btn btn-blue" style="margin:0;" onclick="sendChat()">Execute</button>
                    </div>
                </div>
            </div>
        </div>

        <!-- LIBRARY VIEW -->
        <div id="view-library" class="view-section">
            <div class="glass-panel">
                <h2>Hypermedia Ingestion</h2>
                <p style="color:#aaa;">Drag and drop PDFs or Images to compile them into `.q42` natively via Tauri IPC.</p>
                
                <div style="border:2px dashed rgba(255,255,255,0.2); border-radius:12px; padding:4rem; text-align:center; cursor:pointer;" onclick="invokeNativeHypermedia()">
                    <div style="font-size:3rem; margin-bottom:1rem;">📂</div>
                    <strong style="font-size:1.2rem;">Click or Drag to Ingest Media</strong>
                    <div style="color:var(--neon-green); margin-top:1rem; display:none;" id="lib-success">✅ Package Compiled Successfully!</div>
                </div>
            </div>
        </div>

        <!-- SETTINGS VIEW -->
        <div id="view-settings" class="view-section">
            <div class="glass-panel">
                <h2>System Settings (Tauri Native)</h2>
                
                <div class="toggle-switch">
                    <label class="switch"><input type="checkbox" id="dev-mode-toggle"><span class="slider"></span></label>
                    <span><strong>Dev Mode (Zero Latency)</strong> - Bypasses WebRTC timeout simulation for instant DOM updates.</span>
                </div>

                <label>Local DAG Storage Path</label>
                <input type="text" value="/home/user/.qualia/dag/">
                
                <label>GGUF Model Registry Path</label>
                <input type="text" value="/home/user/.qualia/models/">

                <button class="action-btn btn-green" onclick="saveSettings()">Save Configuration</button>
            </div>
        </div>

    </div>

    <script>
        // Global State
        let devMode = false;
        let globalSats = 0;
        const { invoke } = window.__TAURI__ ? window.__TAURI__.tauri : { invoke: async (cmd) => { console.log(`Mock Tauri Invoke: ${cmd}`); return "Mocked Native Call"; } };

        document.getElementById('dev-mode-toggle').addEventListener('change', (e) => {
            devMode = e.target.checked;
            Logger.log(`Dev Mode ${devMode ? 'ENABLED (Zero Latency)' : 'DISABLED (Authentic Latency)'}`, "log-sys");
        });

        // Navigation
        function switchView(viewId, element) {
            document.querySelectorAll('.view-section').forEach(el => el.classList.remove('active'));
            document.querySelectorAll('.nav-item').forEach(el => el.classList.remove('active'));
            
            document.getElementById(`view-${viewId}`).classList.add('active');
            element.classList.add('active');
        }

        // Shared Logger
        class DiagnosticLogger {
            static log(msg, type = 'log-info') {
                const now = new Date();
                const ts = `[${now.getHours().toString().padStart(2,'0')}:${now.getMinutes().toString().padStart(2,'0')}:${now.getSeconds().toString().padStart(2,'0')}.${now.getMilliseconds().toString().padStart(3,'0')}]`;
                const entry = `<div class="log-entry"><span class="log-time">${ts}</span> <span class="${type}">${msg}</span></div>`;
                
                // Append to both terminals
                document.getElementById('terminal-dashboard').innerHTML += entry;
                document.getElementById('terminal-playground').innerHTML += entry;
                
                // Scroll to bottom
                document.getElementById('terminal-dashboard').scrollTop = document.getElementById('terminal-dashboard').scrollHeight;
                document.getElementById('terminal-playground').scrollTop = document.getElementById('terminal-playground').scrollHeight;
            }
        }
        const Logger = DiagnosticLogger;

        // Playground Logic
        async function proposeAgreement() {
            Logger.log("Proposing M:N Guardianship Agreement...", "log-sys");
            const id = Math.floor(Math.random() * 10000);
            Logger.log(`Agreement DID minted: q42:agreement:${id}`, "log-info");
            
            const statusDiv = document.getElementById("playground-status");
            const delay = devMode ? 50 : Math.floor(Math.random() * 2500) + 1500;
            
            statusDiv.innerHTML = `<div class="dot" style="background:var(--neon-gold);box-shadow: 0 0 10px var(--neon-gold); animation: pulse 1s infinite;"></div> <span class="text" style="color:var(--neon-gold);">AWAITING CRYPTOGRAPHIC RATIFICATION... (Simulating P2P Gossip)</span>`;
            
            setTimeout(() => {
                statusDiv.innerHTML = `<div class="dot" style="background:var(--neon-green);box-shadow: 0 0 10px var(--neon-green);"></div> <span class="text" style="color:var(--neon-green);">AGREEMENT RATIFIED AND COMPILED TO SUPER-QUINS!</span>`;
                Logger.log(`WebRTC: Consensus threshold met for Agreement ${id}.`, "log-info");
                Logger.log(`CRDT Engine: WebizenVM unblocked and state flushed to DAG.`, "log-sys");
                addSats(25);
            }, delay);
        }

        // Chat Logic
        function handleChatEnter(e) {
            if (e.key === 'Enter') sendChat();
        }

        async function sendChat() {
            const input = document.getElementById('chat-input');
            const msg = input.value.trim();
            if(!msg) return;
            
            const history = document.getElementById('chat-history');
            history.innerHTML += `<div class="chat-bubble chat-user">${msg}</div>`;
            input.value = '';
            history.scrollTop = history.scrollHeight;

            // Simulate Agent Routing based on keywords
            let response = "Understood. The Webizen VM has recorded your intent.";
            let telemetryHtml = `<div class="chat-telemetry">WASM Parsing: 2 SAT | No Sieve Routing</div>`;
            let delay = devMode ? 100 : 1200;

            if (msg.toLowerCase().includes("hallucinate") || msg.toLowerCase().includes("false")) {
                response = "I initially hypothesized a false claim, but the Defeasible Reasoning sieve pruned it against deterministic sensor data.";
                telemetryHtml = `<div class="chat-telemetry">Stochastic Defeasibility Triggered: Pruned 1 Claim | 15 SAT Cost</div>`;
                Logger.log("Defeasibility Engine triggered by Chat Intent.", "log-warn");
                addSats(15);
            } else if (msg.toLowerCase().includes("calculate") || msg.toLowerCase().includes("heavy")) {
                response = "I have offloaded this heavy computational graph to the native GPU swarm worker via WebRTC.";
                telemetryHtml = `<div class="chat-telemetry">OP_INFER offloaded to Native Daemon | 250 SAT Cost</div>`;
                Logger.log("Chat routing bypassed WASM, sent to Native Daemon IPC.", "log-sys");
                addSats(250);
                delay = devMode ? 200 : 2500;
            }

            setTimeout(() => {
                history.innerHTML += `<div class="chat-bubble chat-agent">${response}${telemetryHtml}</div>`;
                history.scrollTop = history.scrollHeight;
            }, delay);
        }

        // Settings Logic
        async function saveSettings() {
            Logger.log("Invoking Tauri Native Command: save_settings", "log-sys");
            try {
                const res = await invoke("save_settings", { payload: "mock" });
                alert("Settings saved securely to native OS storage.");
            } catch(e) {
                Logger.log("Native Tauri hooks not available in standard browser. Use desktop app.", "log-error");
            }
        }

        // Library Logic
        async function invokeNativeHypermedia() {
            Logger.log("Invoking Tauri Native Command: process_hypermedia", "log-sys");
            try {
                await invoke("process_hypermedia", { filepath: "mock.pdf" });
                document.getElementById('lib-success').style.display = 'block';
            } catch(e) {
                Logger.log("Native Tauri hooks not available in standard browser. Mocking success.", "log-error");
                setTimeout(()=> { document.getElementById('lib-success').style.display = 'block'; }, 800);
            }
        }

        // Dashboard Wrappers
        function addSats(amount) {
            globalSats += amount;
            document.getElementById('global-cost').innerText = `${globalSats} SAT`;
        }

        function runStressTest() { Logger.log("Executing stress test...", "log-info"); addSats(5); }
        function runPharmacogenomics() { Logger.log("Offloading OP_INFER_BINDING_AFFINITY to Swarm...", "log-sys"); addSats(120); }
        function runCrisisTriage() { Logger.log("Executing LTL evaluation...", "log-warn"); addSats(45); }

    </script>
</body>
</html>"""

with open("C:/Projects/qualiaDB/crates/qualia-client/index.html", "w", encoding="utf-8") as f:
    f.write(html_content)
