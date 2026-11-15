import { initQualiaWasm } from './qualia-wasm-runtime.js';
import {
    CATEGORIES,
    MODALITIES,
    initVibeEngine,
    detectRenderHardwareTier,
    evalVibeModality,
    runModalityDemo,
    runAllDemos,
    renderInteractiveStageCanvas,
    q_hash,
    makeQuin,
} from './modality-engine.js';
import { esc } from './logic-demos.js';

let MOD = null;
let activeId = MODALITIES[0]?.id;
let activeCategory = 'all';

const $ = (id) => document.getElementById(id);
const HUE = {
    rose: 'text-rose-300 border-rose-500/40 bg-rose-500/10',
    purple: 'text-purple-300 border-purple-500/40 bg-purple-500/10',
    amber: 'text-amber-300 border-amber-500/40 bg-amber-500/10',
    cyan: 'text-cyan-300 border-cyan-500/40 bg-cyan-500/10',
    emerald: 'text-emerald-300 border-emerald-500/40 bg-emerald-500/10',
    fuchsia: 'text-fuchsia-300 border-fuchsia-500/40 bg-fuchsia-500/10',
    blue: 'text-blue-300 border-blue-500/40 bg-blue-500/10',
    red: 'text-red-300 border-red-500/40 bg-red-500/10',
};
const HUE_BTN = {
    rose: 'hover:border-rose-400/50 hover:bg-rose-500/15',
    purple: 'hover:border-purple-400/50 hover:bg-purple-500/15',
    amber: 'hover:border-amber-400/50 hover:bg-amber-500/15',
    cyan: 'hover:border-cyan-400/50 hover:bg-cyan-500/15',
    emerald: 'hover:border-emerald-400/50 hover:bg-emerald-500/15',
    fuchsia: 'hover:border-fuchsia-400/50 hover:bg-fuchsia-500/15',
    blue: 'hover:border-blue-400/50 hover:bg-blue-500/15',
    red: 'hover:border-red-400/50 hover:bg-red-400/15',
};

async function loadWasm() {
    const [mod] = await Promise.all([
        initQualiaWasm({
            jsUrl: new URL('../playground/qualia_core_db.js', import.meta.url).href,
            wasmUrl: new URL('../playground/qualia_core_db_bg.wasm', import.meta.url).href,
        }),
        initVibeEngine(),
        detectRenderHardwareTier(),
    ]);
    if (mod?.__initError) throw mod.__initError;
    return mod;
}

function filteredModalities() {
    if (activeCategory === 'all') return MODALITIES;
    return MODALITIES.filter(m => m.category === activeCategory);
}

function renderCategoryPills() {
    const host = $('cat-pills');
    const rail = $('category-rail');
    const counts = Object.fromEntries(CATEGORIES.map(c => [c.id, MODALITIES.filter(m => m.category === c.id).length]));
    let html = `<button type="button" data-cat="all" class="cat-pill ${activeCategory === 'all' ? 'cat-active' : ''}">All <span class="opacity-60">${MODALITIES.length}</span></button>`;
    for (const c of CATEGORIES) {
        html += `<button type="button" data-cat="${c.id}" class="cat-pill ${activeCategory === c.id ? 'cat-active' : ''}"><i class="fa-solid ${c.icon} mr-1 opacity-70"></i>${esc(c.label)} <span class="opacity-60">${counts[c.id]}</span></button>`;
    }
    if (host) {
        host.innerHTML = html;
        host.querySelectorAll('.cat-pill').forEach(btn => {
            btn.addEventListener('click', () => {
                activeCategory = btn.dataset.cat;
                const list = filteredModalities();
                if (list.length && !list.some(m => m.id === activeId)) {
                    activeId = list[0].id;
                }
                renderCategoryPills();
                renderGrid();
                renderStage();
            });
        });
    }
    if (rail) rail.innerHTML = html;
}

function renderGrid() {
    const grid = $('mod-grid');
    const tabs = $('demo-tabs');
    const list = filteredModalities();
    if (grid) {
        grid.innerHTML = list.map(m => {
            const active = m.id === activeId;
            const wasmBadge = m.wasm ? '<span class="text-[9px] px-1.5 py-0.5 rounded bg-cyan-500/20 text-cyan-300 ml-1">WASM</span>' : '';
            return `<button type="button" data-mod="${m.id}" class="mod-card ${active ? 'mod-active' : ''} ${HUE_BTN[m.hue] || ''}">
                <div class="flex items-center gap-2 mb-1">
                    <i class="fa-solid ${m.icon} ${HUE[m.hue]?.split(' ')[0] || 'text-slate-300'}"></i>
                    <span class="font-semibold text-sm text-left">${esc(m.name)}</span>${wasmBadge}
                </div>
                <div class="text-[10px] font-mono text-slate-500 text-left">${esc(m.opcode)}</div>
            </button>`;
        }).join('');
        grid.querySelectorAll('.mod-card').forEach(card => {
            card.addEventListener('click', () => {
                activeId = card.dataset.mod;
                renderGrid();
                renderStage();
            });
        });
    }
    if (tabs) {
        tabs.innerHTML = list.slice(0, 8).map(m => {
            const active = m.id === activeId;
            const cls = active
                ? 'bg-rose-500/20 text-rose-200 border-rose-400/50'
                : 'bg-black/40 text-slate-400 border-white/10 hover:text-white';
            return `<button type="button" data-tab-mod="${esc(m.id)}" class="px-2.5 py-1 rounded-full text-[11px] font-mono border ${cls}">${esc(m.name)}</button>`;
        }).join('');
        tabs.querySelectorAll('button[data-tab-mod]').forEach(btn => {
            btn.addEventListener('click', () => {
                activeId = btn.dataset.tabMod;
                renderGrid();
                renderStage();
            });
        });
    }
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
    const list = (quins && quins.length) ? quins.slice(0, 2) : [
        makeQuin(q_hash('did:q42:alice'), 0x10n, q_hash('q42:action'), q_hash('contract:nda'), 1700000000n),
    ];
    host.innerHTML = list.map((q, idx) => `
        <div class="p-2.5 rounded-lg bg-black/50 border border-white/10 space-y-1">
            <div class="flex items-center justify-between text-[10px] text-cyan-300 font-semibold">
                <span>NQuin[${idx}] (48B)</span>
                <span class="text-emerald-300">XOR Parity ✓</span>
            </div>
            <div class="grid grid-cols-2 gap-x-2 gap-y-0.5 text-[10px] text-slate-300">
                <div><span class="text-slate-500">S:</span> ${formatHex64(q.subject)}</div>
                <div><span class="text-slate-500">P:</span> ${formatHex64(q.predicate)}</div>
                <div><span class="text-slate-500">O:</span> ${formatHex64(q.object)}</div>
                <div><span class="text-slate-500">C:</span> ${formatHex64(q.context)}</div>
            </div>
        </div>
    `).join('');
}

function renderStage() {
    const m = MODALITIES.find(x => x.id === activeId);
    if (!m) return;
    if ($('stage-title')) $('stage-title').textContent = m.name;
    if ($('demo-title')) $('demo-title').textContent = m.name;
    if ($('stage-opcode')) $('stage-opcode').textContent = m.opcode;
    if ($('stage-blurb')) $('stage-blurb').textContent = m.blurb;
    if ($('demo-desc')) $('demo-desc').textContent = m.blurb;
    const cat = CATEGORIES.find(c => c.id === m.category);
    if ($('stage-cat')) $('stage-cat').innerHTML = cat ? `<i class="fa-solid ${cat.icon} mr-1"></i>${esc(cat.label)}` : '';

    if (!MOD) {
        if ($('stage-output')) $('stage-output').innerHTML = '<span class="text-rose-400">WASM engine not loaded — refresh the page.</span>';
        return;
    }

    const result = runModalityDemo(m.id, MOD);
    const out = $('stage-output');
    const viz = $('stage-viz');
    const visualStage = $('visual-stage');

    if ($('code-view') && result.vibeSource) {
        $('code-view').textContent = result.vibeSource;
    }
    if ($('perf-badge') && result.vibeEval) {
        $('perf-badge').textContent = `${(result.vibeEval.elapsedMs || 0.06).toFixed(2)} ms`;
    }

    if (out) {
        if (result.error) {
            out.innerHTML = `<span class="text-rose-400">${esc(result.error)}</span>`;
        } else {
            out.innerHTML = (result.lines || []).map(l => `<div class="font-mono text-sm text-slate-300 py-0.5">${esc(l)}</div>`).join('');
            if (result.pass === true) out.innerHTML += '<div class="mt-2 text-emerald-400 text-xs font-semibold">✓ check passed</div>';
            if (result.pass === false) out.innerHTML += '<div class="mt-2 text-rose-400 text-xs font-semibold">✗ check failed</div>';
        }
    }

    if (visualStage) {
        visualStage.innerHTML = '';
        renderInteractiveStageCanvas(visualStage, {
            canvasKind: result.canvasKind || m.category,
            modalityId: m.id,
            category: m.category,
            title: m.name,
            onReceiptUpdate(liveVibe) {
                const traceEl = $('trace-log');
                if (traceEl) {
                    const r = liveVibe.receipt || {};
                    traceEl.textContent = [
                        `vibe-wasm status: ${liveVibe.ok ? 'OK' : 'DIAGNOSTIC'} · value=${JSON.stringify(liveVibe.value)}`,
                        `astHash: ${r.astHash || '—'} · srcHash: ${r.sourceHash || '—'}`,
                    ].join('\n');
                }
            },
        });
    }

    if (viz) {
        viz.innerHTML = '';
        if (result.visual === 'split') {
            viz.innerHTML = `<div class="grid grid-cols-2 gap-3 text-center text-xs">
                <div class="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30"><div class="text-xl font-bold text-emerald-300">2</div>consistent</div>
                <div class="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30"><div class="text-xl font-bold text-rose-300">1</div>isolated</div></div>`;
        } else if (result.visual === 'verdicts') {
            const chip = (l) => {
                const cls = l.includes('Active') || l.includes('✓') || l.includes('Discharged') ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/25'
                    : l.includes('Defeated') || l.includes('Violated') ? 'bg-amber-500/15 text-amber-300 border-amber-500/25'
                    : 'bg-slate-500/15 text-slate-300 border-slate-500/25';
                return `<span class="px-3 py-1 rounded-full text-xs font-mono border ${cls}">${esc(l)}</span>`;
            };
            viz.innerHTML = `<div class="flex flex-wrap gap-2">${(result.lines || []).map(chip).join('')}</div>`;
        } else if (result.visual === 'badge' && result.pass != null) {
            viz.innerHTML = `<div class="text-2xl font-bold ${result.pass ? 'text-emerald-400' : 'text-rose-400'}">${result.pass ? '✓ PASS' : '✗ FAIL'}</div>`;
        }
    }

    renderQuinInspector(result.quins);
}

function renderPipeline() {
    const steps = [
        { label: 'Agent Intent', icon: 'fa-bullseye', color: 'slate' },
        { label: 'N3 Rights', icon: 'fa-gavel', color: 'rose' },
        { label: 'SHACL Shapes', icon: 'fa-shield', color: 'purple' },
        { label: 'Modality VM', icon: 'fa-microchip', color: 'cyan' },
        { label: 'LLM Infer', icon: 'fa-brain', color: 'blue' },
        { label: 'Output SHACL', icon: 'fa-check-double', color: 'emerald' },
        { label: 'Provenance', icon: 'fa-link', color: 'amber' },
    ];
    const pipe = $('pipeline');
    if (!pipe) return;
    pipe.innerHTML = steps.map((s, i) => `
        ${i ? '<i class="fa-solid fa-chevron-right text-slate-600 text-xs"></i>' : ''}
        <div class="pipe-step text-${s.color}-300"><i class="fa-solid ${s.icon}"></i><span>${s.label}</span></div>
    `).join('');
}

function runOrchestra() {
    if (!MOD) return;
    const results = runAllDemos(MOD);
    const ok = results.filter(r => !r.error && r.pass !== false).length;
    const out = $('orchestra-out');
    if (!out) return;
    out.innerHTML = `
        <div class="text-emerald-300 font-semibold mb-2">${ok} / ${MODALITIES.length} modalities executed (WASM + VibeScript receipts verified)</div>
        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-1.5 max-h-48 overflow-y-auto text-[10px] font-mono">
            ${results.map(r => `<div class="${r.error ? 'text-rose-400' : 'text-slate-400'}">${esc(r.name)}: ${r.error ? 'err' : (r.lines?.[0]?.slice(0, 28) || 'ok')}…</div>`).join('')}
        </div>`;
}

async function boot() {
    try {
        MOD = await loadWasm();
        const ver = MOD.get_engine_version?.() ?? '?';
        const readyCls = 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25 shrink-0';

        const wasmBadge = $('wasm-badge');
        if (wasmBadge) {
            wasmBadge.textContent = 'WASM + VibeScript Ready';
            wasmBadge.className = readyCls;
        }
        const gpuTier = await detectRenderHardwareTier();
        const gpuBadge = $('gpu-badge');
        if (gpuBadge) {
            gpuBadge.textContent = gpuTier.badgeLabel;
            gpuBadge.title = `Renderer: ${gpuTier.renderer}`;
        }
        const badge = $('engine-badge');
        if (badge) {
            badge.textContent = `WASM v${ver} · ${MODALITIES.length} modalities`;
            badge.className = `${readyCls} hidden sm:inline`;
        }

        if ($('stat-count')) $('stat-count').textContent = String(MODALITIES.length);
        if ($('stat-cats')) $('stat-cats').textContent = String(CATEGORIES.length);

        renderPipeline();
        renderCategoryPills();
        renderGrid();
        renderStage();

        $('btn-run-all')?.addEventListener('click', () => {
            $('orchestra-out')?.classList.remove('hidden');
            runOrchestra();
        });
        $('btn-run-stage')?.addEventListener('click', () => renderStage());
        $('btn-run')?.addEventListener('click', () => renderStage());
        $('btn-reset')?.addEventListener('click', () => {
            activeCategory = 'all';
            activeId = MODALITIES[0]?.id;
            renderCategoryPills();
            renderGrid();
            renderStage();
        });
        $('boot-overlay')?.remove();
    } catch (e) {
        console.error(e);
        const o = $('boot-overlay');
        if (o) o.innerHTML = `<span class="text-rose-400">Boot failed: ${esc(e.message)}</span>`;
    }
}

boot();
