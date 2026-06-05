const chatArea = document.getElementById('chatArea');
const chatInput = document.getElementById('chatInput');
const btnSend = document.getElementById('btnSend');
const btnMic = document.getElementById('btnMic');
const modelSelect = document.getElementById('modelSelect');
const agentAvatar = document.getElementById('agentAvatar');
const activeModelName = document.getElementById('activeModelName');
const telTps = document.getElementById('telTps');
const telVram = document.getElementById('telVram');
const activeQ42 = document.getElementById('activeQ42');

let isRecording = false;
let recognition = null;
let currentAgentMessage = null;
let synth = window.speechSynthesis;
let speechTimeout = null;

// Speech Recognition Init
if ('webkitSpeechRecognition' in window) {
    recognition = new webkitSpeechRecognition();
    recognition.continuous = false;
    recognition.interimResults = false;
    
    recognition.onresult = (event) => {
        const text = event.results[0][0].transcript;
        chatInput.value = text;
        sendMessage();
    };
    
    recognition.onend = () => {
        isRecording = false;
        btnMic.classList.remove('recording');
    };
}

btnMic.addEventListener('click', () => {
    if (!recognition) return alert('Speech recognition not supported in this browser.');
    if (isRecording) {
        recognition.stop();
    } else {
        recognition.start();
        isRecording = true;
        btnMic.classList.add('recording');
    }
});

btnSend.addEventListener('click', sendMessage);
chatInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') sendMessage();
});

async function sendMessage() {
    const text = chatInput.value.trim();
    if (!text) return;
    
    // User bubble
    const div = document.createElement('div');
    div.className = 'message msg-user';
    div.innerHTML = `<strong>You:</strong> ${text}`;
    chatArea.appendChild(div);
    chatInput.value = '';
    
    // Agent bubble wrapper
    currentAgentMessage = document.createElement('div');
    currentAgentMessage.className = 'message msg-agent';
    currentAgentMessage.innerHTML = `<strong>Agent:</strong> <span class="content"></span>`;
    chatArea.appendChild(currentAgentMessage);
    chatArea.scrollTop = chatArea.scrollHeight;
    
    const isTauri = window.__TAURI__ != null;
    if (isTauri) {
        try {
            await window.__TAURI__.tauri.invoke('run_agent_inference', { 
                prompt: text, 
                modelName: modelSelect.value 
            });
        } catch (e) {
            currentAgentMessage.querySelector('.content').innerText = "Error reaching Ollama backend.";
        }
    } else {
        // Mock fallback for browser mode
        let i = 0;
        const resp = ["Based ", "on ", "your ", ".q42 ", "clinical ", "history, ", "you ", "have ", "full ", "agency."];
        const interval = setInterval(() => {
            if (i >= resp.length) {
                clearInterval(interval);
                triggerSpeech(currentAgentMessage.querySelector('.content').innerText);
                return;
            }
            appendToken(resp[i]);
            updateTelemetry({ token_rate: 42.5, vram_usage: "3.8 GB", active_q42_context: "elena_pathology.q42\nun_rights.q42" });
            i++;
        }, 150);
    }
}

function appendToken(token) {
    if (currentAgentMessage) {
        currentAgentMessage.querySelector('.content').innerText += token;
        chatArea.scrollTop = chatArea.scrollHeight;
        
        // Debounce speech synthesis until stream stops
        clearTimeout(speechTimeout);
        speechTimeout = setTimeout(() => {
            triggerSpeech(currentAgentMessage.querySelector('.content').innerText);
        }, 800);
    }
}

function updateTelemetry(tel) {
    telTps.innerText = `${tel.token_rate} t/s`;
    telVram.innerText = tel.vram_usage;
    activeQ42.innerText = tel.active_q42_context;
}

function triggerSpeech(text) {
    if (!synth) return;
    synth.cancel(); // Stop current speech
    const utterance = new SpeechSynthesisUtterance(text);
    synth.speak(utterance);
}

// Tauri Event Listeners
if (window.__TAURI__) {
    const { listen } = window.__TAURI__.event;
    const { invoke } = window.__TAURI__.tauri;
    
    invoke('discover_models').then(models => {
        modelSelect.innerHTML = '';
        models.forEach(m => {
            const opt = document.createElement('option');
            opt.value = m.name;
            opt.dataset.avatar = m.avatar_type;
            opt.innerText = `${m.name} ${m.is_active ? '(Active)' : ''}`;
            modelSelect.appendChild(opt);
        });
        updateAvatar();
    });

    listen('llm-token', event => {
        appendToken(event.payload);
    });

    listen('llm-telemetry', event => {
        updateTelemetry(event.payload);
    });
}

modelSelect.addEventListener('change', updateAvatar);

function updateAvatar() {
    const opt = modelSelect.options[modelSelect.selectedIndex];
    if (!opt) return;
    const type = opt.dataset.avatar;
    activeModelName.innerText = opt.value;
    
    agentAvatar.className = 'agent-avatar'; // Reset animation
    if (type === 'grok') agentAvatar.classList.add('grok');
    else if (type === 'llama') agentAvatar.classList.add('llama');
    else agentAvatar.classList.add('phi');
}
