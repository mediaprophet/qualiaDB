import './style.css'
import { QualiaClient } from './qualia-sdk'
import initWasm, { run_semantic_simulation } from 'qualia-core-db'
import Chart from 'chart.js/auto'

const client = new QualiaClient();

// DOM Elements
const elTerminal = document.getElementById('ui-terminal') as HTMLDivElement;
const elPeerId = document.getElementById('ui-peerid') as HTMLSpanElement;
const btnSimulate = document.getElementById('btn-simulate') as HTMLButtonElement;
const themeSelect = document.getElementById('theme-select') as HTMLSelectElement;
let simChart: Chart | null = null;

// Theme Switcher Logic
themeSelect.addEventListener('change', (e) => {
    const target = e.target as HTMLSelectElement;
    document.documentElement.setAttribute('data-theme', target.value);
});

// Set default theme
document.documentElement.setAttribute('data-theme', themeSelect.value);

// Helper to log to the UI Terminal
function logTerminal(msg: string, isSystem = false) {
    const entry = document.createElement('div');
    entry.className = `q-terminal-entry ${isSystem ? 'system' : ''}`;
    entry.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
    elTerminal.appendChild(entry);
    elTerminal.scrollTop = elTerminal.scrollHeight;
}

// 1. Connect the WebSocket Bridge
client.connectBridge((event) => {
    // We expect the daemon to push events like "P2P_HANDSHAKE", "SYNC_ACK", "VM_CYCLE"
    if (event.type === 'P2P_HANDSHAKE') {
        logTerminal(`Handshake established with Peer: ${event.peer_id}`);
    } else if (event.type === 'SYNC_ACK') {
        logTerminal(`Replicated ${event.blocks} blocks to ${event.peer_id}`);
    } else if (event.type === 'DAEMON_INIT') {
        elPeerId.textContent = event.local_peer_id;
        logTerminal(`Daemon Online. Edge Identity: ${event.local_peer_id}`, true);
    } else {
        logTerminal(JSON.stringify(event));
    }
});

// =========================================================================
// MOCK ONTOLOGY: Simulation Parameter Shape (JSON-LD / SHACL)
// In production, this is pulled from `client.queryGraph('qualia:SimulationParameterShape')`
// =========================================================================
const mockSimulationShape = {
    "@context": { "qualia": "http://qualia.io/ns#", "xsd": "http://www.w3.org/2001/XMLSchema#" },
    "@id": "qualia:SimulationParameterShape",
    "properties": [
        { "path": "qualia:initialPrice", "label": "Initial Price (USD)", "datatype": "xsd:decimal", "default": 100.0, "step": 1.0 },
        { "path": "qualia:drift", "label": "Drift (mu)", "datatype": "xsd:decimal", "default": 0.05, "step": 0.01 },
        { "path": "qualia:volatility", "label": "Volatility (sigma)", "datatype": "xsd:decimal", "default": 0.2, "step": 0.01 },
        { "path": "qualia:timeHorizon", "label": "Time Horizon (Yrs)", "datatype": "xsd:integer", "default": 1, "step": 1 },
        { "path": "qualia:simulationSteps", "label": "Simulation Steps", "datatype": "xsd:integer", "default": 1000, "step": 100 }
    ]
};

// =========================================================================
// SEMANTIC RENDERER (Vanilla TS)
// Iterates over a given JSON-LD SHACL shape to build UI components natively
// =========================================================================
function renderSemanticForm(shape: any, containerId: string) {
    const container = document.getElementById(containerId);
    if (!container) return;
    
    container.innerHTML = ''; // Clear previous

    shape.properties.forEach((prop: any) => {
        const group = document.createElement('div');
        group.className = 'q-form-group';
        group.style.marginBottom = '1rem';

        const label = document.createElement('label');
        label.textContent = prop.label || prop.path.split(':')[1];

        const input = document.createElement('input');
        input.className = 'q-input';
        input.id = `input-${prop.path.split(':')[1]}`;
        
        if (prop.datatype === 'xsd:decimal' || prop.datatype === 'xsd:integer') {
            input.type = 'number';
            if (prop.step) input.step = prop.step.toString();
        } else {
            input.type = 'text';
        }
        
        if (prop.default !== undefined) {
            input.value = prop.default.toString();
        }

        group.appendChild(label);
        group.appendChild(input);
        container.appendChild(group);
    });
}

// Render the UI on load!
renderSemanticForm(mockSimulationShape, 'semantic-form-container');

function updateChart(initial: number, mean: number, var95: number) {
    const ctx = document.getElementById('sim-chart') as HTMLCanvasElement;
    if (!ctx) return;

    if (simChart) {
        simChart.destroy();
    }

    const cssVar = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#FFF';
    
    simChart = new Chart(ctx, {
        type: 'bar',
        data: {
            labels: ['Initial Price', 'Expected Mean', '95% VaR (Worst Case)'],
            datasets: [{
                label: 'Asset Value (USD)',
                data: [initial, mean, var95],
                backgroundColor: [
                    cssVar('--q-text-secondary'),
                    cssVar('--q-accent-secondary'),
                    cssVar('--q-accent-warning')
                ],
                borderWidth: 1
            }]
        },
        options: {
            responsive: true,
            scales: {
                y: {
                    beginAtZero: false,
                    grid: { color: cssVar('--q-border-line').split(' ')[2] || 'rgba(255,255,255,0.1)' }
                },
                x: {
                    grid: { color: 'transparent' }
                }
            },
            plugins: {
                legend: { display: false }
            }
        }
    });
}

// 2. Wire the Economics Simulator
btnSimulate.addEventListener('click', async () => {
    // Dynamically pull values based on the generated semantic IDs
    const getVal = (id: string) => parseFloat((document.getElementById(`input-${id}`) as HTMLInputElement).value);
    
    const price = getVal('initialPrice');
    const drift = getVal('drift');
    const vol = getVal('volatility');
    const time = getVal('timeHorizon');
    const steps = getVal('simulationSteps');

    btnSimulate.textContent = "Executing WebAssembly Core...";
    btnSimulate.disabled = true;

    logTerminal(`Dispatching Monte Carlo Task to local WASM Engine...`, true);

    try {
        const result = run_semantic_simulation({
            initial_price: price,
            drift: drift,
            volatility: vol,
            time_horizon: time,
            simulation_steps: steps
        });

        if (result) {
            document.getElementById('simulation-results')!.style.display = 'block';
            document.getElementById('res-mean')!.textContent = `$${result.mean.toFixed(2)}`;
            document.getElementById('res-var')!.textContent = `$${result.value_at_risk.toFixed(2)}`;
            logTerminal(`Task Complete (Offline WASM). Mean: $${result.mean.toFixed(2)}, VaR: $${result.value_at_risk.toFixed(2)}`);
            updateChart(price, result.mean, result.value_at_risk);
        }
    } catch (e: any) {
        logTerminal(`Simulation Task Failed: ${e.toString()}`, true);
    }

    btnSimulate.textContent = "Execute Edge Simulation";
    btnSimulate.disabled = false;
});

// Initial boot message
logTerminal("Initializing Qualia Client SDK...", true);

// Initialize WASM Core
initWasm().then(() => {
    logTerminal("Webizen WASM Core Initialized.", true);
}).catch((e: any) => {
    logTerminal(`Failed to initialize WASM: ${e}`, true);
});
