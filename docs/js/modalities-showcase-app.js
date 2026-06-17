import wasmInit, * as WasmExports from '../playground/qualia_core_db.js';
import { CATEGORIES, MODALITIES, runModalityDemo, runAllDemos } from './modality-engine.js';
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
    const url = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
    const resp = await fetch(url, { cache: 'no-store' });
    if (!resp.ok) throw new Error(`WASM ${resp.status}`);
    await wasmInit({ module_or_path: resp });
    return WasmExports;
}

function filteredModalities() {
    if (activeCategory === 'all') return MODALITIES;
    return MODALITIES.filter(m => m.category === activeCategory);
}

function renderCategoryPills() {
    const host = $('cat-pills');
    if (!host) return;
    const counts = Object.fromEntries(CATEGORIES.map(c => [c.id, MODALITIES.filter(m => m.category === c.id).length]));
    let html = `<button type="button" data-cat="all" class="cat-pill ${activeCategory === 'all' ? 'cat-active' : ''}">All <span class="opacity-60">${MODALITIES.length}</span></button>`;
    for (const c of CATEGORIES) {
        html += `<button type="button" data-cat="${c.id}" class="cat-pill ${activeCategory === c.id ? 'cat-active' : ''}"><i class="fa-solid ${c.icon} mr-1 opacity-70"></i>${esc(c.label)} <span class="opacity-60">${counts[c.id]}</span></button>`;
    }
    host.innerHTML = html;
    host.querySelectorAll('.cat-pill').forEach(btn => {
        btn.addEventListener('click', () => {
            activeCategory = btn.dataset.cat;
            renderCategoryPills();
            renderGrid();
        });
    });
}

function renderGrid() {
    const grid = $('mod-grid');
    if (!grid) return;
    const list = filteredModalities();
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

function drawIntervals(canvas) {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const W = canvas.width, H = canvas.height;
    ctx.clearRect(0, 0, W, H);
    const draw = (y, s, e, color) => {
        const x1 = 40 + (s / 25) * (W - 80);
        const x2 = 40 + (e / 25) * (W - 80);
        ctx.fillStyle = color + '33';
        ctx.fillRect(x1, y, x2 - x1, 14);
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.strokeRect(x1, y, x2 - x1, 14);
    };
    draw(30, 1, 5, '#fbbf24');
    draw(55, 10, 20, '#22d3ee');
    ctx.fillStyle = '#94a3b8';
    ctx.font = '11px monospace';
    ctx.fillText('τ₁ [1,5] Before [10,20] τ₂', 40, 88);
}

function drawTrace(canvas) {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    const W = canvas.width, H = canvas.height;
    ctx.clearRect(0, 0, W, H);
    const pts = [100, 100, 200];
    const step = (W - 60) / (pts.length - 1);
    ctx.strokeStyle = '#334155';
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(30, H / 2);
    pts.forEach((_, i) => { if (i) ctx.lineTo(30 + i * step, H / 2); });
    ctx.stroke();
    pts.forEach((p, i) => {
        ctx.beginPath();
        ctx.arc(30 + i * step, H / 2, 10, 0, Math.PI * 2);
        ctx.fillStyle = p === 100 ? '#34d399' : '#fbbf24';
        ctx.fill();
        ctx.fillStyle = '#e2e8f0';
        ctx.font = '10px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(String(p), 30 + i * step, H / 2 + 28);
    });
}

function renderStage() {
    const m = MODALITIES.find(x => x.id === activeId);
    if (!m) return;
    $('stage-title').textContent = m.name;
    $('stage-opcode').textContent = m.opcode;
    $('stage-blurb').textContent = m.blurb;
    const cat = CATEGORIES.find(c => c.id === m.category);
    $('stage-cat').innerHTML = cat ? `<i class="fa-solid ${cat.icon} mr-1"></i>${esc(cat.label)}` : '';

    const result = runModalityDemo(m.id, MOD);
    const out = $('stage-output');
    const viz = $('stage-viz');

    if (result.error) {
        out.innerHTML = `<span class="text-rose-400">${esc(result.error)}</span>`;
    } else {
        out.innerHTML = (result.lines || []).map(l => `<div class="font-mono text-sm text-slate-300 py-0.5">${esc(l)}</div>`).join('');
        if (result.pass === true) out.innerHTML += '<div class="mt-2 text-emerald-400 text-xs font-semibold">✓ check passed</div>';
        if (result.pass === false) out.innerHTML += '<div class="mt-2 text-rose-400 text-xs font-semibold">✗ check failed</div>';
    }

    viz.innerHTML = '';
    const c = document.createElement('canvas');
    c.width = 360; c.height = 100;
    c.className = 'w-full max-w-md rounded-xl bg-black/40';
    if (result.visual === 'intervals') { viz.appendChild(c); drawIntervals(c); }
    else if (result.visual === 'trace') { viz.appendChild(c); drawTrace(c); }
    else if (result.visual === 'split') {
        viz.innerHTML = `<div class="grid grid-cols-2 gap-3 text-center text-xs">
            <div class="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/30"><div class="text-2xl font-bold text-emerald-300">2</div>consistent</div>
            <div class="p-4 rounded-xl bg-rose-500/10 border border-rose-500/30"><div class="text-2xl font-bold text-rose-300">1</div>isolated</div></div>`;
    } else if (result.visual === 'verdicts') {
        const chip = (l) => {
            const cls = l.includes('Active') || l.includes('✓') ? 'bg-emerald-500/15 text-emerald-300 border-emerald-500/25'
                : l.includes('Defeated') ? 'bg-amber-500/15 text-amber-300 border-amber-500/25'
                : 'bg-slate-500/15 text-slate-300 border-slate-500/25';
            return `<span class="px-3 py-1.5 rounded-full text-xs font-mono border ${cls}">${esc(l)}</span>`;
        };
        viz.innerHTML = `<div class="flex flex-wrap gap-2">${(result.lines || []).map(chip).join('')}</div>`;
    } else if (result.visual === 'badge' && result.pass != null) {
        viz.innerHTML = `<div class="text-4xl font-bold ${result.pass ? 'text-emerald-400' : 'text-rose-400'}">${result.pass ? 'PASS' : 'FAIL'}</div>`;
    }
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
    $('pipeline').innerHTML = steps.map((s, i) => `
        ${i ? '<i class="fa-solid fa-chevron-right text-slate-600 text-xs"></i>' : ''}
        <div class="pipe-step text-${s.color}-300"><i class="fa-solid ${s.icon}"></i><span>${s.label}</span></div>
    `).join('');
}

function runOrchestra() {
    const results = runAllDemos(MOD);
    const ok = results.filter(r => !r.error && r.pass !== false).length;
    $('orchestra-out').innerHTML = `
        <div class="text-emerald-300 font-semibold mb-2">${ok} / ${MODALITIES.length} modalities executed</div>
        <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-1.5 max-h-48 overflow-y-auto text-[10px] font-mono">
            ${results.map(r => `<div class="${r.error ? 'text-rose-400' : 'text-slate-400'}">${esc(r.name)}: ${r.error ? 'err' : (r.lines?.[0]?.slice(0, 28) || 'ok')}…</div>`).join('')}
        </div>`;
}

async function boot() {
    try {
        MOD = await loadWasm();
        const ver = MOD.get_engine_version?.() ?? '?';
        const badge = $('engine-badge');
        if (badge) {
            badge.textContent = `WASM v${ver} · ${MODALITIES.length} modalities`;
            badge.className = 'text-xs px-3 py-1 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25';
        }
        $('stat-count').textContent = String(MODALITIES.length);
        $('stat-cats').textContent = String(CATEGORIES.length);
        renderPipeline();
        renderCategoryPills();
        renderGrid();
        renderStage();
        $('btn-run-all')?.addEventListener('click', () => {
            $('orchestra-out')?.classList.remove('hidden');
            runOrchestra();
        });
        $('btn-run-stage')?.addEventListener('click', renderStage);
        $('boot-overlay')?.remove();
    } catch (e) {
        console.error(e);
        const o = $('boot-overlay');
        if (o) o.innerHTML = `<span class="text-rose-400">Boot failed: ${esc(e.message)}</span>`;
    }
}

boot();