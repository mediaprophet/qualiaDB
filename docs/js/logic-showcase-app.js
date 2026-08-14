import wasmInit, * as WasmExports from '../playground/qualia_core_db.js';
import { fetchWasmBinary } from './wasm-fetch.js';
import {
    N3_PRESETS, N3_RULE_ARROWS, SHACL_WASM_CONSTRAINTS, SHACL_QUALIA_EXTENSIONS,
    detectN3Rules, parseN3Triples, validateShaclConstraint, runForwardChain,
    FORWARD_CHAIN_PRESETS, esc,
} from './logic-demos.js';

let MOD = null;

async function loadEngine() {
    const wasmUrl = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
    const resp = await fetchWasmBinary(wasmUrl);
    await wasmInit({ module_or_path: resp });
    return WasmExports;
}

function $(id) { return document.getElementById(id); }

const CHIP_STYLE = {
    emerald: 'bg-emerald-500/15 text-emerald-300 border-emerald-500/25',
    amber: 'bg-amber-500/15 text-amber-300 border-amber-500/25',
    rose: 'bg-rose-500/15 text-rose-300 border-rose-500/25',
    cyan: 'bg-cyan-500/15 text-cyan-300 border-cyan-500/25',
};
const GROUP_COLOR = {
    rose: 'text-rose-300', purple: 'text-purple-300', amber: 'text-amber-300',
    cyan: 'text-cyan-300', emerald: 'text-emerald-300',
};

function renderN3Chips() {
    const host = $('n3-chips');
    if (!host) return;
    host.innerHTML = N3_RULE_ARROWS.map((r) =>
        `<span class="arrow-chip border ${CHIP_STYLE[r.color] || 'bg-slate-500/15 text-slate-300'}" title="${esc(r.desc)}">${esc(r.arrow)} ${esc(r.name)}</span>`
    ).join('');
}

function renderShaclSelect() {
    const sel = $('showcase-shacl-type');
    if (!sel) return;
    sel.innerHTML = SHACL_WASM_CONSTRAINTS.map((c) =>
        `<option value="${esc(c.id)}">${esc(c.label)}</option>`
    ).join('');
}

function renderExtensionsGrid() {
    const grid = $('extensions-grid');
    if (!grid) return;
    grid.innerHTML = SHACL_QUALIA_EXTENSIONS.map((g) => `
        <div class="glass-strong rounded-3xl p-5">
            <h3 class="font-semibold ${GROUP_COLOR[g.color] || 'text-slate-300'} mb-3"><i class="fa-solid ${g.icon} mr-2"></i>${esc(g.group)}</h3>
            <ul class="space-y-2 text-sm">
                ${g.items.map((it) => `
                    <li class="border-b border-white/5 pb-2">
                        <span class="font-mono text-white/90">${esc(it.name)}</span>
                        <p class="text-white/50 text-xs mt-0.5">${esc(it.desc)}</p>
                    </li>`).join('')}
            </ul>
        </div>`).join('');
}

function analyzeN3() {
    const text = $('showcase-n3')?.value?.trim();
    const out = $('showcase-n3-out');
    if (!text || !out) return;

    const rules = detectN3Rules(text);
    const triples = parseN3Triples(MOD, text);

    let html = '';
    if (rules.length) {
        html += '; Detected rules\n';
        rules.forEach((r, i) => {
            html += `${String(i + 1).padStart(2, '0')}  [${r.type}] ${r.arrow}\n`;
            html += `    premise:    ${r.premise}\n`;
            html += `    conclusion: ${r.conclusion}\n`;
            if (r.desc) html += `    ; ${r.desc}\n`;
        });
    } else {
        html += '; No rule arrows found\n';
    }
    html += `\n; Static triples (${triples.length})\n`;
    triples.forEach((t) => { html += `${t.subject} ${t.predicate} ${t.object}\n`; });

    out.textContent = html;
}

function demoForwardChain() {
    const out = $('showcase-n3-out');
    const preset = FORWARD_CHAIN_PRESETS.penguin;
    try {
        const r = runForwardChain(MOD, preset);
        out.textContent =
            `; forward_chain_wasm — ${preset.label}\n` +
            `facts: ${preset.facts.join(', ')}\n` +
            `inferred: ${(r.inferred || []).join(', ') || '(none)'}\n` +
            `; "flies" defeated when penguin present`;
    } catch (e) {
        out.textContent = `Error: ${e.message || e}`;
    }
}

function demoShacl() {
    const box = $('showcase-shacl-out');
    if (!box) return;
    const constraint_type = $('showcase-shacl-type').value;
    const value = +$('showcase-shacl-bound').value;
    const target_value = +$('showcase-shacl-target').value;
    try {
        const r = validateShaclConstraint(MOD, constraint_type, value, target_value);
        box.classList.remove('hidden');
        box.innerHTML = r.passes
            ? `<span class="text-emerald-400 font-semibold">✓ PASS</span> — ${esc(r.constraint_type)} (bound ${r.value}, target ${r.target_value})`
            : `<span class="text-rose-400 font-semibold">✗ VIOLATION</span> — ${esc(r.constraint_type)} (bound ${r.value}, target ${r.target_value})`;
    } catch (e) {
        box.classList.remove('hidden');
        box.innerHTML = `<span class="text-rose-400">${esc(e.message || e)}</span>`;
    }
}

async function boot() {
    try {
        MOD = await loadEngine();
        const ver = MOD.get_engine_version?.() ?? '?';
        const badge = $('engine-badge');
        if (badge) {
            badge.textContent = `WASM v${ver}`;
            badge.className = 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25';
        }

        renderN3Chips();
        renderShaclSelect();
        renderExtensionsGrid();

        if ($('showcase-n3')) {
            $('showcase-n3').value = N3_PRESETS.deontic;
            $('btn-showcase-n3')?.addEventListener('click', analyzeN3);
            $('btn-showcase-chain')?.addEventListener('click', demoForwardChain);
            analyzeN3();
        }
        $('btn-showcase-shacl')?.addEventListener('click', demoShacl);

        const overlay = $('boot-overlay');
        if (overlay) overlay.remove();
    } catch (e) {
        console.error(e);
        const overlay = $('boot-overlay');
        if (overlay) overlay.innerHTML = `<span class="text-rose-400">Engine failed: ${esc(e.message || e)}</span>`;
    }
}

boot();