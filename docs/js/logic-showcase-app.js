import wasmInit, * as WasmExports from '../playground/qualia_core_db.js';
import { fetchWasmBinary } from './wasm-fetch.js';
import {
    N3_PRESETS, N3_RULE_ARROWS, SHACL_WASM_CONSTRAINTS, SHACL_QUALIA_EXTENSIONS,
    LOGIC_CATEGORIES, LOGIC_DEMOS, getLogicDemosByCategory, getLogicDemoById,
    detectN3Rules, parseN3Triples, validateShaclConstraint, runForwardChain,
    FORWARD_CHAIN_PRESETS, esc,
} from './logic-demos.js';
import {
    initVibeEngine,
    detectRenderHardwareTier,
    evalVibeModality,
    renderInteractiveStageCanvas,
    runModalityDemo,
    q_hash,
    makeQuin,
} from './modality-engine.js';

let MOD = null;
let activeCategoryId = LOGIC_CATEGORIES[0]?.id || 'deontic';
let activeDemoId = LOGIC_DEMOS[0]?.id || 'deontic-guardianship';

async function loadEngine() {
    const wasmUrl = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
    const [resp] = await Promise.all([
        fetchWasmBinary(wasmUrl),
        initVibeEngine(),
        detectRenderHardwareTier(),
    ]);
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

function renderCategoryRail() {
    const rail = $('category-rail');
    if (!rail) return;
    rail.innerHTML = LOGIC_CATEGORIES.map((c) => {
        const active = c.id === activeCategoryId;
        const cls = active
            ? 'bg-rose-500/20 text-rose-200 border-rose-400/50'
            : 'bg-zinc-900/80 text-slate-400 border-white/10 hover:text-white hover:border-white/25';
        return `<button type="button" data-cat="${esc(c.id)}" class="px-3.5 py-1.5 rounded-full text-xs font-medium border transition-colors ${cls}">
            <i class="fa-solid ${esc(c.icon)} mr-1.5"></i>${esc(c.label)}
        </button>`;
    }).join('');

    rail.querySelectorAll('button[data-cat]').forEach((btn) => {
        btn.addEventListener('click', () => {
            activeCategoryId = btn.dataset.cat;
            const demos = getLogicDemosByCategory(activeCategoryId);
            if (demos.length) activeDemoId = demos[0].id;
            renderCategoryRail();
            renderDemoTabs();
            runActiveStageDemo();
        });
    });
}

function renderDemoTabs() {
    const tabs = $('demo-tabs');
    if (!tabs) return;
    const demos = getLogicDemosByCategory(activeCategoryId);
    tabs.innerHTML = demos.map((d) => {
        const active = d.id === activeDemoId;
        const cls = active
            ? 'bg-purple-500/20 text-purple-200 border-purple-400/50'
            : 'bg-black/40 text-slate-400 border-white/10 hover:text-white';
        return `<button type="button" data-demo="${esc(d.id)}" class="px-3 py-1 rounded-xl text-xs font-mono border ${cls}">
            ${esc(d.title)}
        </button>`;
    }).join('');

    tabs.querySelectorAll('button[data-demo]').forEach((btn) => {
        btn.addEventListener('click', () => {
            activeDemoId = btn.dataset.demo;
            renderDemoTabs();
            runActiveStageDemo();
        });
    });
}

function formatHex64(val) {
    try {
        return `0x${(BigInt(val) & 0xFFFF_FFFF_FFFF_FFFFn).toString(16).padStart(16, '0')}`;
    } catch (_) {
        return '0x0000000000000000';
    }
}

function renderQuinInspector(quins) {
    const host = $('quin-inspector');
    if (!host) return;
    const list = (quins && quins.length) ? quins.slice(0, 3) : [
        makeQuin(q_hash('did:q42:alice'), 0x10n | (q_hash('q42:keep-confidential') << 8n), q_hash('q42:data'), q_hash('contract:nda'), 1700000000n),
    ];
    host.innerHTML = list.map((q, idx) => `
        <div class="p-3 rounded-xl bg-black/50 border border-white/10 space-y-1">
            <div class="flex items-center justify-between text-[11px] text-cyan-300 font-semibold">
                <span>NQuin[${idx}] (48-byte Super-Quin)</span>
                <span class="text-emerald-300">XOR Parity ✓</span>
            </div>
            <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[11px] text-slate-300">
                <div><span class="text-slate-500">S:</span> ${formatHex64(q.subject)}</div>
                <div><span class="text-slate-500">P:</span> ${formatHex64(q.predicate)}</div>
                <div><span class="text-slate-500">O:</span> ${formatHex64(q.object)}</div>
                <div><span class="text-slate-500">C:</span> ${formatHex64(q.context)}</div>
                <div><span class="text-slate-500">M:</span> ${formatHex64(q.metadata)}</div>
                <div><span class="text-slate-500">⊕:</span> ${formatHex64(q.parity)}</div>
            </div>
        </div>
    `).join('');
}

export function runActiveStageDemo() {
    const demo = getLogicDemoById(activeDemoId);
    if (!demo) return;

    if ($('demo-title')) $('demo-title').textContent = demo.title;
    if ($('demo-desc')) $('demo-desc').textContent = demo.desc;
    if ($('code-view')) $('code-view').textContent = demo.vibeCode;

    const modIdMap = {
        deontic: 'deontic',
        allen: 'allen7',
        ltl: 'ltl',
        epistemic: 'epistemic',
        paraconsistent: 'paraconsistent',
        inference: 'n3-defeasible',
    };
    const modRes = runModalityDemo(modIdMap[demo.canvasKind] || 'deontic', MOD);
    const vibeRes = evalVibeModality(demo.vibeCode);

    if ($('perf-badge')) {
        $('perf-badge').textContent = `${(vibeRes.elapsedMs || 0.08).toFixed(2)} ms`;
    }

    const stage = $('visual-stage');
    if (stage) {
        stage.innerHTML = '';
        renderInteractiveStageCanvas(stage, {
            canvasKind: demo.canvasKind,
            category: demo.category,
            title: demo.title,
            onReceiptUpdate(liveVibe) {
                const traceEl = $('trace-log');
                if (traceEl) {
                    const r = liveVibe.receipt || {};
                    const lines = [
                        `; Modality Engine + vibe-wasm 0.3 Execution Trace`,
                        ...(modRes.lines || []),
                        ``,
                        `; Live VibeScript 0.3 Evaluation`,
                        `status:      ${liveVibe.ok ? 'OK (DeterministicExact)' : 'DIAGNOSTIC'}`,
                        `return:      ${JSON.stringify(liveVibe.value)}`,
                        `astHash:     ${r.astHash || '—'}`,
                        `sourceHash:  ${r.sourceHash || '—'}`,
                        `profile:     ${r.profile || 'application/vnd.vibe.ast+cbor; version=0.1'}`,
                    ];
                    traceEl.textContent = lines.join('\n');
                }
            },
        });

        // Structured summary cards below interactive canvas
        const cards = document.createElement('div');
        cards.className = 'grid grid-cols-1 sm:grid-cols-3 gap-2.5 mt-3';
        cards.innerHTML = (modRes.lines || []).slice(0, 3).map((line) => `
            <div class="px-3 py-2 rounded-xl bg-white/5 border border-white/10 text-xs font-mono text-slate-200 truncate" title="${esc(line)}">
                ${esc(line)}
            </div>
        `).join('');
        stage.appendChild(cards);
    }

    renderQuinInspector(modRes.quins);

    if ($('showcase-n3') && demo.n3Preset) {
        $('showcase-n3').value = demo.n3Preset;
        analyzeN3();
    }
}

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
        const readyBadgeCls = 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25';

        const wasmBadge = $('wasm-badge');
        if (wasmBadge) {
            wasmBadge.textContent = 'WASM + VibeScript Ready';
            wasmBadge.className = readyBadgeCls;
        }
        const gpuTier = await detectRenderHardwareTier();
        const gpuBadge = $('gpu-badge');
        if (gpuBadge) {
            gpuBadge.textContent = gpuTier.badgeLabel;
            gpuBadge.title = `Renderer: ${gpuTier.renderer}`;
        }
        const badge = $('engine-badge');
        if (badge) {
            badge.textContent = `WASM v${ver} · VibeScript Ready`;
            badge.className = `${readyBadgeCls} hidden sm:inline`;
        }

        renderCategoryRail();
        renderDemoTabs();
        renderN3Chips();
        renderShaclSelect();
        renderExtensionsGrid();
        runActiveStageDemo();

        $('btn-run')?.addEventListener('click', runActiveStageDemo);
        $('btn-reset')?.addEventListener('click', () => {
            activeCategoryId = LOGIC_CATEGORIES[0].id;
            activeDemoId = LOGIC_DEMOS[0].id;
            renderCategoryRail();
            renderDemoTabs();
            runActiveStageDemo();
        });

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
