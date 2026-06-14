// ─────────────────────────────────────────────────────────────────────────────
// Qualia Playground — real in-browser engine wiring.
//
// Every panel on this page runs the actual qualia-core-db WASM build compiled
// from Rust. No mocks, no fake hashes: `compile_query_to_json`, `parse_*_wasm`,
// and the solver exports are the same functions the native engine ships.
// ─────────────────────────────────────────────────────────────────────────────

import { initQualiaWasm } from '../js/qualia-wasm-runtime.js';

let MOD = null;

const $  = (id) => document.getElementById(id);
const el = (tag, cls, html) => {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (html != null) n.innerHTML = html;
    return n;
};

/** Time a synchronous WASM call, returning { value, ms }. */
function timed(fn) {
    const t0 = performance.now();
    const value = fn();
    return { value, ms: performance.now() - t0 };
}

const fmtMs = (ms) => (ms < 1 ? `${(ms * 1000).toFixed(0)} µs` : `${ms.toFixed(2)} ms`);

// ── Format presets for the compiler editor ───────────────────────────────────
const PRESETS = {
    sparql: `SELECT ?person ?name WHERE {
  ?person a foaf:Person .
  ?person foaf:name ?name .
  ?person foaf:age ?age .
  FILTER(?age > 18)
}`,
    n3: `@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix : <http://qualia.db/> .

{ ?x a foaf:Person } => { ?x a :Mortal } .
:socrates a foaf:Person .`,
    turtle: `@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix : <http://qualia.db/> .

:alice a foaf:Person ;
       foaf:name "Alice" ;
       foaf:knows :bob .
:bob   a foaf:Person ;
       foaf:name "Bob" .`,
    ntriples: `<http://qualia.db/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .
<http://qualia.db/alice> <http://xmlns.com/foaf/0.1/knows> <http://qualia.db/bob> .
<http://qualia.db/bob> <http://xmlns.com/foaf/0.1/name> "Bob" .`,
};

function detectFormat(text) {
    const t = text.trim();
    if (/^\s*(SELECT|ASK|CONSTRUCT|DESCRIBE)\b/i.test(t)) return 'sparql';
    if (/=>|\{[^}]*\}\s*=>/.test(t)) return 'n3';
    if (/^\s*@prefix/i.test(t) || /;\s*$/m.test(t)) return 'turtle';
    if (/^\s*</.test(t)) return 'ntriples';
    return 'turtle';
}

// ─────────────────────────────────────────────────────────────────────────────
// Compiler tab
// ─────────────────────────────────────────────────────────────────────────────

function syntaxColorOp(op) {
    // Colour-code the opcode mnemonic for readability.
    const m = op.match(/^([A-Za-z_]+)/);
    const head = m ? m[1] : op;
    let cls = 'text-slate-300';
    if (/MATCH|SCAN|SEEK/i.test(head))        cls = 'text-cyan-300';
    else if (/BIND|VAR|PUSH|LOAD/i.test(head)) cls = 'text-emerald-300';
    else if (/FILTER|EVAL|MASK|CMP/i.test(head)) cls = 'text-amber-300';
    else if (/HALT|JMP|RET|EMIT/i.test(head)) cls = 'text-fuchsia-300';
    return `<span class="${cls}">${op}</span>`;
}

function runCompile() {
    if (!MOD?.compile_query_to_json) return;
    const query = $('editor').value.trim();
    if (!query) return;

    const out = $('bytecode-output');
    const { value: raw, ms } = timed(() => MOD.compile_query_to_json(query));
    $('compile-latency').textContent = fmtMs(ms);

    let prog;
    try { prog = JSON.parse(raw); } catch (_) { prog = { error: 'parse error', raw }; }

    if (prog.error) {
        out.innerHTML = `<span class="text-rose-400">● compilation error</span>\n${prog.error}`;
        $('op-count').textContent = '0 ops';
        return;
    }

    const lines = prog.instructions.map((ins, i) =>
        `<span class="text-slate-600">${String(i).padStart(3, '0')}</span>  ${syntaxColorOp(ins.op)}`
    );
    out.innerHTML =
        `<span class="text-cyan-400">; source: ${prog.source} · ${prog.compiled_len} instruction(s)</span>\n` +
        lines.join('\n');
    $('op-count').textContent = `${prog.compiled_len} ops`;
}

function runParse() {
    const payload = $('editor').value.trim();
    if (!payload) return;
    const fmt = detectFormat(payload);
    const fn = (fmt === 'n3') ? MOD?.parse_n3logic_wasm : MOD?.parse_turtle_wasm;
    const grid = $('quin-output');

    if (!fn) { grid.innerHTML = `<div class="text-slate-500 text-center py-10">Parser unavailable in this build.</div>`; return; }

    const { value: triples, ms } = timed(() => fn(payload));
    $('parse-latency').textContent = fmtMs(ms);

    if (!triples || !triples.length) {
        grid.innerHTML = `<div class="text-slate-500 text-center py-10">No triples parsed. Try the Turtle or N3 preset.</div>`;
        $('quin-count').textContent = '0 triples';
        return;
    }

    grid.innerHTML = triples.map((t) => `
        <div class="qrow">
          <div class="qcell"><span class="qlabel">S</span><span class="qval text-cyan-300" title="${escAttr(t.subject)}">${trunc(t.subject)}</span></div>
          <div class="qcell"><span class="qlabel">P</span><span class="qval text-emerald-300" title="${escAttr(t.predicate)}">${trunc(t.predicate)}</span></div>
          <div class="qcell"><span class="qlabel">O</span><span class="qval text-amber-300" title="${escAttr(t.object)}">${trunc(t.object)}</span></div>
        </div>`).join('');
    $('quin-count').textContent = `${triples.length} triple${triples.length === 1 ? '' : 's'}`;
}

const trunc = (s, n = 42) => { s = String(s); return s.length > n ? esc(s.slice(0, n)) + '…' : esc(s); };
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const escAttr = (s) => esc(s).replace(/"/g, '&quot;');

// ─────────────────────────────────────────────────────────────────────────────
// Solvers tab
// ─────────────────────────────────────────────────────────────────────────────

function solverResult(targetId, html, ms) {
    const box = $(targetId);
    box.innerHTML = html;
    box.classList.remove('hidden');
    if (ms != null) {
        const tag = box.querySelector('.solver-ms');
        if (tag) tag.textContent = fmtMs(ms);
    }
}

function demoLipinski() {
    if (!MOD?.evaluate_lipinski_wasm) return;
    const smiles = $('lip-smiles').value.trim();
    if (!smiles) return;
    const t0 = performance.now();
    let r;
    try { r = MOD.evaluate_lipinski_wasm({ smiles }); }
    catch (_) { return solverResult('lip-out', `<span class="text-rose-400">Could not parse SMILES.</span>`); }
    const ms = performance.now() - t0;

    const flt  = (name, pass) => `<span class="chip ${pass ? 'chip-true' : 'chip-false'}">${name} ${pass ? '✓' : '✗'}</span>`;
    const prop = (k, v, u = '') => `<div class="flex justify-between"><span class="text-slate-500">${k}</span><span class="font-mono text-slate-200">${v}${u}</span></div>`;
    solverResult('lip-out',
        `<div class="flex items-center mb-2"><span class="text-slate-400 text-xs uppercase tracking-wide">Drug-likeness filters</span><span class="solver-ms ml-auto"></span></div>
         <div class="flex flex-wrap gap-1.5 mb-2.5">
           ${flt('Lipinski', r.lipinski_passes)}${flt('Veber', r.veber_passes)}${flt('Ghose', r.ghose_passes)}${flt('Egan', r.egan_passes)}
         </div>
         <div class="grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs">
           ${prop('MW', r.mw.toFixed(1), ' g/mol')}
           ${prop('logP', r.logp.toFixed(2))}
           ${prop('TPSA', r.tpsa.toFixed(1), ' Å²')}
           ${prop('HBD / HBA', `${r.hbd} / ${r.hba}`)}
           ${prop('Rotatable', r.rot_bonds)}
           ${prop('Violations', r.lipinski_violations)}
         </div>`, ms);
}

function demoBlackScholes() {
    if (!MOD?.black_scholes_wasm) return;
    const p = {
        spot: +$('bs-spot').value, strike: +$('bs-strike').value,
        rate: +$('bs-rate').value / 100, vol: +$('bs-vol').value / 100,
        time_years: +$('bs-time').value, is_call: $('bs-type').value === 'call',
    };
    const { value: r, ms } = timed(() => MOD.black_scholes_wasm(p));
    const row = (k, v, c = 'text-slate-200') => `<div class="flex justify-between"><span class="text-slate-500">${k}</span><span class="font-mono ${c}">${v}</span></div>`;
    solverResult('bs-out',
        `<div class="flex items-center mb-2"><span class="text-2xl font-mono text-emerald-300">$${r.price.toFixed(4)}</span><span class="solver-ms ml-auto"></span></div>
         <div class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
           ${row('Δ delta', r.delta.toFixed(4), 'text-cyan-300')}
           ${row('Γ gamma', r.gamma.toFixed(4), 'text-cyan-300')}
           ${row('ν vega', r.vega.toFixed(4), 'text-amber-300')}
           ${row('Θ theta', r.theta.toFixed(4), 'text-amber-300')}
           ${row('ρ rho', r.rho.toFixed(4), 'text-fuchsia-300')}
         </div>`, ms);
}

function demoPid() {
    if (!MOD?.compute_pid_step_wasm) return;
    const p = {
        setpoint: +$('pid-set').value, current_value: +$('pid-cur').value,
        prev_error: 0, integral: 0,
        kp: +$('pid-kp').value, ki: +$('pid-ki').value, kd: +$('pid-kd').value, dt: 0.1,
    };
    const { value: r, ms } = timed(() => MOD.compute_pid_step_wasm(p));
    solverResult('pid-out',
        `<div class="flex items-center mb-2"><span class="text-slate-400 text-xs uppercase tracking-wide">Control output</span><span class="solver-ms ml-auto"></span></div>
         <div class="text-2xl font-mono text-cyan-300 mb-1">${r.output.toFixed(4)}</div>
         <div class="text-xs text-slate-500 font-mono">error ${r.new_error.toFixed(3)} · ∫ ${r.new_integral.toFixed(3)}</div>`, ms);
}

function sparkline(canvasId, ys, color) {
    const cv = $(canvasId);
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height, pad = 4;
    ctx.clearRect(0, 0, W, H);
    if (!ys.length) return;
    const min = Math.min(...ys), max = Math.max(...ys), span = (max - min) || 1;
    ctx.beginPath();
    ys.forEach((y, i) => {
        const px = pad + (i / (ys.length - 1)) * (W - 2 * pad);
        const py = H - pad - ((y - min) / span) * (H - 2 * pad);
        i ? ctx.lineTo(px, py) : ctx.moveTo(px, py);
    });
    ctx.strokeStyle = color; ctx.lineWidth = 1.5; ctx.stroke();
    // soft fill
    ctx.lineTo(W - pad, H - pad); ctx.lineTo(pad, H - pad); ctx.closePath();
    ctx.fillStyle = color + '22'; ctx.fill();
}

function demoGbm() {
    if (!MOD?.simulate_gbm_path_wasm) return;
    const p = {
        initial_price: +$('gbm-s0').value, drift: +$('gbm-mu').value / 100,
        volatility: +$('gbm-vol').value / 100, time_horizon: 1.0, steps: 120,
    };
    const { value: r, ms } = timed(() => MOD.simulate_gbm_path_wasm(p));
    const path = r.prices || r.path || r.values || (Array.isArray(r) ? r : []);
    const final = path.length ? path[path.length - 1] : 0;
    solverResult('gbm-out',
        `<div class="flex items-center mb-2"><span class="text-slate-400 text-xs uppercase tracking-wide">Terminal price</span><span class="solver-ms ml-auto"></span></div>
         <div class="text-xl font-mono text-emerald-300 mb-2">$${Number(final).toFixed(2)}</div>
         <canvas id="gbm-spark" width="260" height="48" class="w-full"></canvas>`, ms);
    sparkline('gbm-spark', path.map(Number), '#34d399');
}

function demoOde() {
    if (!MOD?.solve_ode_exponential_decay_wasm) return;
    const p = { k: +$('ode-k').value, y0: +$('ode-y0').value, t0: 0, t_final: 5, dt: 0.05 };
    const { value: r, ms } = timed(() => MOD.solve_ode_exponential_decay_wasm(p));
    const ys = r.y_values || [];
    solverResult('ode-out',
        `<div class="flex items-center mb-2"><span class="text-slate-400 text-xs uppercase tracking-wide">y(5) via RK4</span><span class="solver-ms ml-auto"></span></div>
         <div class="text-xl font-mono text-cyan-300 mb-2">${Number(r.final_y).toFixed(5)}</div>
         <canvas id="ode-spark" width="260" height="48" class="w-full"></canvas>`, ms);
    sparkline('ode-spark', ys.map(Number), '#22d3ee');
}

function demoFramingham() {
    if (!MOD?.compute_framingham_risk_wasm) return;
    const p = {
        age: +$('fr-age').value, sex_male: $('fr-sex').value === 'male',
        total_cholesterol_mmol: +$('fr-tc').value, hdl_cholesterol_mmol: +$('fr-hdl').value,
        systolic_bp: +$('fr-sbp').value, bp_treated: $('fr-bptx').checked,
        current_smoker: $('fr-smoke').checked, diabetic: $('fr-dm').checked,
    };
    const { value: r, ms } = timed(() => MOD.compute_framingham_risk_wasm(p));
    const pct = r.risk_10yr_pct;
    const col = pct < 10 ? 'text-emerald-300' : pct < 20 ? 'text-amber-300' : 'text-rose-400';
    const barCol = pct < 10 ? '#34d399' : pct < 20 ? '#fbbf24' : '#fb7185';
    solverResult('fr-out',
        `<div class="flex items-baseline gap-2 mb-2">
           <span class="text-3xl font-mono ${col}">${pct.toFixed(1)}%</span>
           <span class="text-slate-400 text-sm">10-yr CVD · ${esc(r.category)}</span>
           <span class="solver-ms ml-auto"></span>
         </div>
         <div class="h-2 rounded-full bg-white/5 overflow-hidden"><div style="width:${Math.min(pct, 100)}%;background:${barCol}" class="h-full"></div></div>`, ms);
}

// ─────────────────────────────────────────────────────────────────────────────
// Capabilities tab
// ─────────────────────────────────────────────────────────────────────────────

let ALL_CAPS = [];

function renderCaps(filter = '') {
    const grid = $('caps-grid');
    const f = filter.trim().toLowerCase();
    const list = ALL_CAPS.filter((c) => !f || c.toLowerCase().includes(f));
    $('caps-shown').textContent = `${list.length} / ${ALL_CAPS.length}`;
    grid.innerHTML = list.map((c) =>
        `<div class="cap-chip"><i class="fa-solid fa-microchip text-cyan-400/70"></i><span>${esc(c)}</span></div>`
    ).join('') || `<div class="text-slate-500 col-span-full text-center py-8">No capability matches “${esc(filter)}”.</div>`;
}

// ─────────────────────────────────────────────────────────────────────────────
// Tabs
// ─────────────────────────────────────────────────────────────────────────────

function showTab(name) {
    document.querySelectorAll('[data-tab]').forEach((b) => b.classList.toggle('tab-active', b.dataset.tab === name));
    document.querySelectorAll('[data-panel]').forEach((p) => p.classList.toggle('hidden', p.dataset.panel !== name));
}

// ─────────────────────────────────────────────────────────────────────────────
// Boot
// ─────────────────────────────────────────────────────────────────────────────

async function boot() {
    MOD = await initQualiaWasm({ base: '..' });

    const ok = MOD && typeof MOD.compile_query_to_json === 'function';
    const overlay = $('boot-overlay');

    if (!ok) {
        overlay.querySelector('#boot-text').innerHTML =
            `<span class="text-rose-400">Engine failed to load.</span><br><span class="text-slate-500 text-sm">Check the console — the WASM bundle may be missing.</span>`;
        return;
    }

    // Engine banner
    let info = {};
    try { info = MOD.get_engine_info ? MOD.get_engine_info() : {}; } catch (_) {}
    const version = info.version || (MOD.get_engine_version ? MOD.get_engine_version() : '?');
    try { ALL_CAPS = MOD.list_capabilities_wasm ? MOD.list_capabilities_wasm() : (info.capabilities || []); } catch (_) { ALL_CAPS = info.capabilities || []; }

    $('banner-version').textContent = `v${version}`;
    $('banner-caps').textContent = `${ALL_CAPS.length} capabilities`;
    $('banner-target').textContent = info.target || 'wasm32';

    renderCaps();

    // Wire compiler
    $('btn-compile').addEventListener('click', runCompile);
    $('btn-parse').addEventListener('click', runParse);
    document.querySelectorAll('[data-preset]').forEach((b) =>
        b.addEventListener('click', () => {
            $('editor').value = PRESETS[b.dataset.preset];
            $('fmt-chip').textContent = b.dataset.preset;
            runCompile(); runParse();
        }));
    $('editor').addEventListener('input', () => { $('fmt-chip').textContent = detectFormat($('editor').value); });

    // Wire solvers
    const wire = (id, fn) => { const b = $(id); if (b) b.addEventListener('click', fn); };
    wire('btn-lip', demoLipinski);
    wire('btn-bs', demoBlackScholes);
    wire('btn-pid', demoPid);
    wire('btn-gbm', demoGbm);
    wire('btn-ode', demoOde);
    wire('btn-fr', demoFramingham);
    document.querySelectorAll('[data-smiles]').forEach((b) =>
        b.addEventListener('click', () => { $('lip-smiles').value = b.dataset.smiles; demoLipinski(); }));

    // Capabilities search
    $('caps-search').addEventListener('input', (e) => renderCaps(e.target.value));

    // Tabs
    document.querySelectorAll('[data-tab]').forEach((b) => b.addEventListener('click', () => showTab(b.dataset.tab)));

    // Initial run so the page is alive on load
    $('editor').value = PRESETS.turtle;
    $('fmt-chip').textContent = 'turtle';
    runCompile(); runParse();
    demoBlackScholes(); demoFramingham(); demoLipinski();

    // Reveal
    overlay.style.opacity = '0';
    setTimeout(() => overlay.remove(), 400);
}

boot();
