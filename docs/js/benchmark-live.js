/**
 * Live, WASM-backed implementations for the Benchmark Hub's interactive tabs,
 * plus zero-dependency canvas viewers. Replaces the previous hardcoded /
 * Math.random() simulations: every number here is measured from a real call
 * into qualia_core_db_bg.wasm.
 *
 *  - Spatial Math      → geometric_algebra_operation()  + interactive 3D object viewer
 *  - Graph Operations  → parse_turtle_wasm() + compile_query_to_json() + interactive graph
 */

const local = (uri) => String(uri).replace(/[<>]/g, '').split(/[/#]/).filter(Boolean).pop() || uri;

// ───────────────────────────── 3D object viewer ─────────────────────────────
// Procedural unit meshes: vertices [[x,y,z]…] + edges [[i,j]…].
function buildMeshes() {
    const phi = (1 + Math.sqrt(5)) / 2;
    const norm = (v) => { const l = Math.hypot(...v) || 1; return v.map((c) => c / l); };
    const edgesByDist = (verts, tol) => {
        const e = [];
        let min = Infinity;
        for (let i = 0; i < verts.length; i++)
            for (let j = i + 1; j < verts.length; j++) {
                const d = Math.hypot(verts[i][0] - verts[j][0], verts[i][1] - verts[j][1], verts[i][2] - verts[j][2]);
                if (d < min) min = d;
            }
        for (let i = 0; i < verts.length; i++)
            for (let j = i + 1; j < verts.length; j++) {
                const d = Math.hypot(verts[i][0] - verts[j][0], verts[i][1] - verts[j][1], verts[i][2] - verts[j][2]);
                if (d <= min * (1 + tol)) e.push([i, j]);
            }
        return e;
    };

    const tetra = [[1, 1, 1], [1, -1, -1], [-1, 1, -1], [-1, -1, 1]].map(norm);
    const cube = [[-1, -1, -1], [1, -1, -1], [1, 1, -1], [-1, 1, -1], [-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1]].map((v) => v.map((c) => c / Math.sqrt(3)));
    const octa = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
    const ico = [
        [-1, phi, 0], [1, phi, 0], [-1, -phi, 0], [1, -phi, 0],
        [0, -1, phi], [0, 1, phi], [0, -1, -phi], [0, 1, -phi],
        [phi, 0, -1], [phi, 0, 1], [-phi, 0, -1], [-phi, 0, 1],
    ].map(norm);

    // UV sphere
    const sVerts = []; const sEdges = []; const rings = 9; const seg = 14;
    for (let i = 0; i <= rings; i++) {
        const lat = Math.PI * (i / rings - 0.5);
        for (let j = 0; j < seg; j++) {
            const lon = 2 * Math.PI * (j / seg);
            sVerts.push([Math.cos(lat) * Math.cos(lon), Math.sin(lat), Math.cos(lat) * Math.sin(lon)]);
        }
    }
    for (let i = 0; i <= rings; i++)
        for (let j = 0; j < seg; j++) {
            const a = i * seg + j;
            if (j < seg - 1) sEdges.push([a, a + 1]); else sEdges.push([a, i * seg]);
            if (i < rings) sEdges.push([a, a + seg]);
        }

    // Torus
    const tVerts = []; const tEdges = []; const R = 0.7; const r = 0.3; const tu = 16; const tv = 9;
    for (let i = 0; i < tu; i++) {
        const u = 2 * Math.PI * (i / tu);
        for (let j = 0; j < tv; j++) {
            const v = 2 * Math.PI * (j / tv);
            tVerts.push([(R + r * Math.cos(v)) * Math.cos(u), r * Math.sin(v), (R + r * Math.cos(v)) * Math.sin(u)]);
        }
    }
    for (let i = 0; i < tu; i++)
        for (let j = 0; j < tv; j++) {
            const a = i * tv + j;
            tEdges.push([a, i * tv + ((j + 1) % tv)]);
            tEdges.push([a, ((i + 1) % tu) * tv + j]);
        }

    return {
        tetrahedron: { verts: tetra, edges: edgesByDist(tetra, 0.1) },
        cube: { verts: cube, edges: edgesByDist(cube, 0.05) },
        octahedron: { verts: octa, edges: edgesByDist(octa, 0.1) },
        icosahedron: { verts: ico, edges: edgesByDist(ico, 0.05) },
        sphere: { verts: sVerts, edges: sEdges },
        torus: { verts: tVerts, edges: tEdges },
    };
}

export class Object3DViewer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.meshes = buildMeshes();
        this.mesh = this.meshes.icosahedron;
        this.yaw = 0.6; this.pitch = -0.4; this.zoom = 1;
        this.spin = true; this.dragging = false;
        this._bind();
        this._loop = this._loop.bind(this);
        requestAnimationFrame(this._loop);
    }
    setMesh(name) { if (this.meshes[name]) this.mesh = this.meshes[name]; }
    setSpin(on) { this.spin = on; }
    _bind() {
        const c = this.canvas;
        let lx = 0, ly = 0;
        c.style.touchAction = 'none'; c.style.cursor = 'grab';
        c.addEventListener('pointerdown', (e) => { this.dragging = true; this.spin = false; lx = e.clientX; ly = e.clientY; c.setPointerCapture(e.pointerId); c.style.cursor = 'grabbing'; });
        c.addEventListener('pointermove', (e) => {
            if (!this.dragging) return;
            this.yaw += (e.clientX - lx) * 0.01; this.pitch += (e.clientY - ly) * 0.01;
            this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch));
            lx = e.clientX; ly = e.clientY;
        });
        const end = () => { this.dragging = false; c.style.cursor = 'grab'; };
        c.addEventListener('pointerup', end);
        c.addEventListener('pointercancel', end);
        c.addEventListener('wheel', (e) => { e.preventDefault(); this.zoom = Math.max(0.4, Math.min(3, this.zoom * (e.deltaY < 0 ? 1.1 : 0.9))); }, { passive: false });
    }
    _resize() {
        const c = this.canvas; const dpr = window.devicePixelRatio || 1;
        const w = c.clientWidth || 360; const h = c.clientHeight || 300;
        if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) { c.width = Math.round(w * dpr); c.height = Math.round(h * dpr); }
        return { w, h, dpr };
    }
    _loop() {
        const { w, h, dpr } = this._resize();
        const ctx = this.ctx;
        if (this.spin && !this.dragging) this.yaw += 0.006;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, h);
        const cx = w / 2, cy = h / 2, scale = Math.min(w, h) * 0.36 * this.zoom;
        const cy_ = Math.cos(this.yaw), sy = Math.sin(this.yaw), cp = Math.cos(this.pitch), sp = Math.sin(this.pitch);
        const proj = this.mesh.verts.map(([x, y, z]) => {
            let X = x * cy_ - z * sy, Z = x * sy + z * cy_;
            let Y = y * cp - Z * sp; Z = y * sp + Z * cp;
            const persp = 3 / (3 + Z);
            return [cx + X * scale * persp, cy + Y * scale * persp, Z];
        });
        ctx.lineWidth = 1.2;
        for (const [i, j] of this.mesh.edges) {
            const a = proj[i], b = proj[j];
            const depth = (a[2] + b[2]) / 2;
            const t = Math.max(0, Math.min(1, (depth + 1.4) / 2.8));
            ctx.strokeStyle = `rgba(52,211,153,${0.25 + t * 0.6})`;
            ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
        }
        for (const p of proj) {
            const t = Math.max(0, Math.min(1, (p[2] + 1.4) / 2.8));
            ctx.fillStyle = `rgba(96,165,250,${0.4 + t * 0.6})`;
            ctx.beginPath(); ctx.arc(p[0], p[1], 1.5 + t * 1.8, 0, 7); ctx.fill();
        }
        requestAnimationFrame(this._loop);
    }
}

// ───────────────────────────── graph viewer ─────────────────────────────
export class GraphViewer {
    constructor(canvas) {
        this.canvas = canvas; this.ctx = canvas.getContext('2d');
        this.nodes = []; this.edges = [];
        this.ox = 0; this.oy = 0; this.scale = 1;
        this.drag = null; this.pan = false;
        this._bind();
        this._loop = this._loop.bind(this);
        requestAnimationFrame(this._loop);
    }
    setData(triples) {
        const idx = new Map();
        const node = (uri) => {
            if (!idx.has(uri)) { idx.set(uri, this.nodes.length); this.nodes.push({ id: uri, label: local(uri), x: (Math.random() - 0.5) * 360, y: (Math.random() - 0.5) * 360, vx: 0, vy: 0 }); }
            return idx.get(uri);
        };
        this.nodes = []; this.edges = [];
        for (const t of triples) {
            const s = node(t.subject), o = node(t.object);
            this.edges.push({ s, o, label: local(t.predicate) });
        }
    }
    _toScreen(p, w, h) { return [w / 2 + (p.x + this.ox) * this.scale, h / 2 + (p.y + this.oy) * this.scale]; }
    _bind() {
        const c = this.canvas; let lx = 0, ly = 0;
        c.style.touchAction = 'none'; c.style.cursor = 'grab';
        c.addEventListener('pointerdown', (e) => {
            const r = c.getBoundingClientRect(); const w = c.clientWidth, h = c.clientHeight;
            const mx = e.clientX - r.left, my = e.clientY - r.top;
            this.drag = null;
            for (const n of this.nodes) { const [sx, sy] = this._toScreen(n, w, h); if (Math.hypot(mx - sx, my - sy) < 14) { this.drag = n; break; } }
            this.pan = !this.drag; lx = e.clientX; ly = e.clientY; c.setPointerCapture(e.pointerId); c.style.cursor = 'grabbing';
        });
        c.addEventListener('pointermove', (e) => {
            const dx = e.clientX - lx, dy = e.clientY - ly; lx = e.clientX; ly = e.clientY;
            if (this.drag) { this.drag.x += dx / this.scale; this.drag.y += dy / this.scale; this.drag.vx = 0; this.drag.vy = 0; }
            else if (this.pan) { this.ox += dx / this.scale; this.oy += dy / this.scale; }
        });
        const end = () => { this.drag = null; this.pan = false; c.style.cursor = 'grab'; };
        c.addEventListener('pointerup', end); c.addEventListener('pointercancel', end);
        c.addEventListener('wheel', (e) => { e.preventDefault(); this.scale = Math.max(0.3, Math.min(3, this.scale * (e.deltaY < 0 ? 1.1 : 0.9))); }, { passive: false });
    }
    _physics() {
        const N = this.nodes;
        for (let i = 0; i < N.length; i++) {
            const a = N[i];
            for (let j = i + 1; j < N.length; j++) {
                const b = N[j]; let dx = a.x - b.x, dy = a.y - b.y; let d2 = dx * dx + dy * dy || 1; const f = 6000 / d2;
                const d = Math.sqrt(d2); const ux = dx / d, uy = dy / d;
                a.vx += ux * f; a.vy += uy * f; b.vx -= ux * f; b.vy -= uy * f;
            }
            a.vx -= a.x * 0.004; a.vy -= a.y * 0.004; // gentle centering
        }
        for (const e of this.edges) {
            const a = N[e.s], b = N[e.o]; const dx = b.x - a.x, dy = b.y - a.y; const d = Math.hypot(dx, dy) || 1;
            const f = (d - 120) * 0.02; const ux = dx / d, uy = dy / d;
            a.vx += ux * f; a.vy += uy * f; b.vx -= ux * f; b.vy -= uy * f;
        }
        for (const n of N) { if (n === this.drag) continue; n.vx *= 0.85; n.vy *= 0.85; n.x += n.vx; n.y += n.vy; }
    }
    _loop() {
        const c = this.canvas; const dpr = window.devicePixelRatio || 1;
        const w = c.clientWidth || 360, h = c.clientHeight || 300;
        if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) { c.width = Math.round(w * dpr); c.height = Math.round(h * dpr); }
        const ctx = this.ctx; ctx.setTransform(dpr, 0, 0, dpr, 0, 0); ctx.clearRect(0, 0, w, h);
        if (this.nodes.length) this._physics();
        ctx.lineWidth = 1; ctx.strokeStyle = 'rgba(148,163,184,0.4)'; ctx.font = '10px monospace';
        for (const e of this.edges) {
            const [ax, ay] = this._toScreen(this.nodes[e.s], w, h); const [bx, by] = this._toScreen(this.nodes[e.o], w, h);
            ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
            ctx.fillStyle = 'rgba(148,163,184,0.65)'; ctx.fillText(e.label, (ax + bx) / 2 + 3, (ay + by) / 2 - 2);
        }
        for (const n of this.nodes) {
            const [x, y] = this._toScreen(n, w, h);
            ctx.fillStyle = 'rgba(52,211,153,0.9)'; ctx.beginPath(); ctx.arc(x, y, 6, 0, 7); ctx.fill();
            ctx.fillStyle = '#e2e8f0'; ctx.font = '11px sans-serif'; ctx.fillText(n.label, x + 9, y + 4);
        }
        if (!this.nodes.length) { ctx.fillStyle = 'rgba(148,163,184,0.5)'; ctx.fillText('Run a graph operation to render the graph…', 16, h / 2); }
        requestAnimationFrame(this._loop);
    }
}

// ───────────────────────────── benchmark runners ─────────────────────────────
const SPATIAL_OPS = ['geo', 'inner', 'outer', 'reverse'];
function randomMultivector() { return Array.from({ length: 8 }, () => Math.round((Math.random() * 4 - 2) * 100) / 100); }

/** Real geometric-algebra benchmark via geometric_algebra_operation(). */
export function runSpatialLive({ wasm, charts, output, count }) {
    if (!wasm?.geometric_algebra_operation) { if (output) output.textContent = 'WASM geometric_algebra_operation unavailable.'; return null; }
    const iters = Math.max(100, Math.min(count || 10000, 500000));
    const perfByOp = {}; const sample = {};
    for (const op of SPATIAL_OPS) {
        const a = randomMultivector(), b = randomMultivector();
        const payload = JSON.stringify({ a, b, op });
        // warm
        sample[op] = JSON.parse(wasm.geometric_algebra_operation(payload));
        const t0 = performance.now();
        for (let i = 0; i < iters; i++) wasm.geometric_algebra_operation(payload);
        const dt = performance.now() - t0;
        perfByOp[op] = { opsPerSec: Math.round(iters / (dt / 1000)), msPer: dt / iters };
    }
    if (charts?.spatialPerformance) {
        charts.spatialPerformance.data.labels = SPATIAL_OPS.map((o) => o.toUpperCase());
        charts.spatialPerformance.data.datasets[0].label = 'Ops/sec (measured)';
        charts.spatialPerformance.data.datasets[0].data = SPATIAL_OPS.map((o) => perfByOp[o].opsPerSec);
        charts.spatialPerformance.update();
    }
    if (charts?.spatialScaling) {
        const scales = [1000, 5000, iters];
        const lat = scales.map((s) => { const p = JSON.stringify({ a: randomMultivector(), b: randomMultivector(), op: 'geo' }); const t0 = performance.now(); for (let i = 0; i < s; i++) wasm.geometric_algebra_operation(p); return (performance.now() - t0) / s; });
        charts.spatialScaling.data.labels = scales.map((s) => `${s.toLocaleString()}×`);
        charts.spatialScaling.data.datasets[0].label = 'ms / op (measured)';
        charts.spatialScaling.data.datasets[0].data = lat;
        charts.spatialScaling.update();
    }
    const geo = sample.geo;
    if (output) {
        output.textContent =
            `✓ Geometric algebra benchmark (REAL — geometric_algebra_operation)\n` +
            `Iterations per op: ${iters.toLocaleString()}\n` +
            SPATIAL_OPS.map((o) => `  ${o.padEnd(6)} ${perfByOp[o].opsPerSec.toLocaleString().padStart(12)} ops/sec  (${perfByOp[o].msPer.toFixed(5)} ms/op)`).join('\n') +
            `\n\nSample G(3,0,0) product (geo):\n  a∘b = [${geo.result.map((n) => Number(n).toFixed(2)).join(', ')}]\n  grades = [${geo.grades.map((n) => Number(n).toFixed(3)).join(', ')}]  compute_ops=${geo.compute_ops}`;
    }
    return { perfByOp, sample };
}

const SAMPLE_TURTLE = `@prefix f: <http://qualia.dev/foaf/> .
f:Alice f:knows f:Bob . f:Alice f:knows f:Carol . f:Bob f:knows f:Dave .
f:Carol f:knows f:Dave . f:Dave f:knows f:Erin . f:Erin f:knows f:Alice .
f:Alice f:topic f:Graphs . f:Bob f:topic f:Graphs . f:Carol f:topic f:Spatial .
f:Dave f:topic f:Tensors . f:Erin f:topic f:Spatial . f:Graphs f:partOf f:Qualia .
f:Spatial f:partOf f:Qualia . f:Tensors f:partOf f:Qualia .`;

const GRAPH_QUERIES = {
    point: '?s f:topic f:Graphs',
    twohop: '?s f:knows ?m . ?m f:knows ?o',
    filter: '?s f:partOf f:Qualia',
    ingest: '?s ?p ?o',
};

/** Real graph benchmark via parse_turtle_wasm() + compile_query_to_json(). */
export function runGraphLive({ wasm, charts, output, viewer, operation }) {
    if (!wasm?.parse_turtle_wasm || !wasm?.compile_query_to_json) { if (output) output.textContent = 'WASM graph functions unavailable.'; return null; }
    // Real ingestion: parse Turtle → quins (measured).
    const t0 = performance.now();
    let triples = wasm.parse_turtle_wasm(SAMPLE_TURTLE);
    const ingestMs = performance.now() - t0;
    if (typeof triples === 'string') triples = JSON.parse(triples);
    if (viewer) viewer.setData(triples);

    // Real query compilation timing for each pattern (measured, many iters).
    const compileMs = {}; let lastPlan = null;
    for (const [name, q] of Object.entries(GRAPH_QUERIES)) {
        const iters = 5000; const t = performance.now();
        for (let i = 0; i < iters; i++) lastPlan = wasm.compile_query_to_json(q);
        compileMs[name] = (performance.now() - t) / iters;
    }
    if (charts?.graphLatency) {
        charts.graphLatency.data.labels = ['Point', 'Two-hop', 'Filter', 'Ingest'];
        charts.graphLatency.data.datasets[0].label = 'Compile ms/op (measured)';
        charts.graphLatency.data.datasets[0].data = [compileMs.point, compileMs.twohop, compileMs.filter, compileMs.ingest];
        charts.graphLatency.update();
    }
    if (charts?.graphThroughput) {
        const op = operation && compileMs[operation] != null ? operation : 'point';
        const ops = Math.round(1000 / compileMs[op]);
        charts.graphThroughput.data.labels = ['compile/s'];
        charts.graphThroughput.data.datasets[0].label = `${op} compiles/sec (measured)`;
        charts.graphThroughput.data.datasets[0].data = [ops];
        charts.graphThroughput.update();
    }
    const plan = (() => { try { return JSON.parse(lastPlan); } catch { return null; } })();
    if (output) {
        output.textContent =
            `✓ Graph benchmark (REAL — parse_turtle_wasm + compile_query_to_json)\n` +
            `Ingested ${triples.length} triples in ${ingestMs.toFixed(3)} ms (${Math.round(triples.length / (ingestMs / 1000)).toLocaleString()} triples/sec)\n` +
            `Query compile (ms/op, ${(5000).toLocaleString()} iters):\n` +
            Object.entries(compileMs).map(([k, v]) => `  ${k.padEnd(8)} ${v.toFixed(5)} ms`).join('\n') +
            (plan ? `\n\nCompiled plan (${GRAPH_QUERIES[operation] || GRAPH_QUERIES.point}):\n  source=${plan.source} len=${plan.compiled_len}` : '') +
            `\n\nGraph rendered below — drag nodes, scroll to zoom.`;
    }
    return { triples, ingestMs, compileMs };
}
