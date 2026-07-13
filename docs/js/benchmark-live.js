/**
 * Live, data-backed implementations for the Benchmark Hub's interactive tabs.
 *
 * Graph Operations:
 * - real dataset selection via docs/benchmark-datasets/*.json
 * - real execute_ntriples_query() timings over flat NQuin bytes
 * - clickable graph neighborhood explorer
 *
 * Spatial Mathematics:
 * - geometric algebra via WASM
 * - GeoSPARQL topology via WASM
 * - spatial encoding / indexing via WASM
 * - interval algebra fallback clearly labeled until a dedicated WASM export exists
 */

import {
    decodeFlatDb,
    fetchManifest,
    loadDataset,
    qHash,
    queriesForManifest,
} from '../benchmark-dataset-loader.js';

const local = (uri) => {
    const clean = String(uri).replace(/[<>]/g, '');
    const parts = clean.split(/[/#]/).filter(Boolean);
    const tail = parts.at(-1) || clean || uri;
    const parent = parts.at(-2);
    return /^\d+$/.test(tail) && parent ? `${parent}/${tail}` : tail;
};

function median(values) {
    if (!values.length) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const mid = Math.floor(sorted.length / 2);
    return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function percentile(values, p) {
    if (!values.length) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
}

function stats(samples) {
    const mean = samples.reduce((sum, value) => sum + value, 0) / Math.max(samples.length, 1);
    return {
        mean,
        p50: median(samples),
        p95: percentile(samples, 0.95),
        min: Math.min(...samples),
        max: Math.max(...samples),
    };
}

function fmtMs(value) {
    if (!Number.isFinite(value)) return 'n/a';
    if (value < 0.01) return `${value.toFixed(4)} ms`;
    if (value < 1) return `${value.toFixed(3)} ms`;
    return `${value.toFixed(2)} ms`;
}

function fmtOps(value) {
    if (!Number.isFinite(value)) return 'n/a';
    return Math.round(value).toLocaleString();
}

function shortHash(hashish) {
    const hex = BigInt(hashish).toString(16).padStart(16, '0');
    return `0x${hex.slice(0, 8)}â€¦${hex.slice(-4)}`;
}

function fullHash(hashish) {
    return `0x${BigInt(hashish).toString(16).padStart(16, '0')}`;
}

function isHexLabel(value) {
    return typeof value === 'string' && /^0x[0-9a-f]+$/i.test(value);
}

function shouldUpgradeNodeText(current, next, id) {
    if (!next) return false;
    if (!current) return true;
    return current === id || current === local(id) || isHexLabel(current);
}

function hashFromValue(value) {
    try { return BigInt(value); } catch { return 0n; }
}

function decodeMatchLabel(raw, labelMap) {
    const exact = hashFromValue(raw);
    const masked = exact & 0x0fffffffffffffffn;
    return labelMap?.get(exact) ?? labelMap?.get(masked) ?? shortHash(exact);
}

function decodeMatchEntry(raw, labelMap, kind = 'subject') {
    if (raw == null || raw === '') {
        return null;
    }
    const exact = hashFromValue(raw);
    const masked = exact & 0x0fffffffffffffffn;
    const key = kind === 'object' ? masked : exact;
    const description = labelMap?.get(exact) ?? labelMap?.get(masked) ?? fullHash(key);
    return {
        id: fullHash(key),
        label: isHexLabel(description) ? shortHash(key) : local(description),
        description,
        hash: key,
    };
}

function normalizeRenderableTriple(triple) {
    const subjectHash = triple.subjectHash != null ? BigInt(triple.subjectHash) : null;
    const predicateHash = triple.predicateHash != null ? BigInt(triple.predicateHash) : null;
    const objectHash = triple.objectHash != null ? BigInt(triple.objectHash) : null;
    const objectKey = objectHash != null ? (objectHash & 0x0fffffffffffffffn) : null;

    const subjectDescription = triple.subject ?? (subjectHash != null ? fullHash(subjectHash) : '');
    const predicateDescription = triple.predicate ?? (predicateHash != null ? fullHash(predicateHash) : '');
    const objectDescription = triple.object ?? (objectKey != null ? fullHash(objectKey) : '');

    return {
        ...triple,
        subjectId: triple.subjectId ?? (subjectHash != null ? fullHash(subjectHash) : String(subjectDescription)),
        predicateId: triple.predicateId ?? (predicateHash != null ? fullHash(predicateHash) : String(predicateDescription)),
        objectId: triple.objectId ?? (objectKey != null ? fullHash(objectKey) : String(objectDescription)),
        subjectLabel: triple.subjectLabel ?? (isHexLabel(subjectDescription) ? shortHash(subjectHash ?? subjectDescription) : local(subjectDescription)),
        predicateLabel: triple.predicateLabel ?? (isHexLabel(predicateDescription) ? shortHash(predicateHash ?? predicateDescription) : local(predicateDescription)),
        objectLabel: triple.objectLabel ?? (isHexLabel(objectDescription) ? shortHash(objectKey ?? objectDescription) : local(objectDescription)),
        subjectDescription,
        predicateDescription,
        objectDescription,
    };
}

function uniquePush(bucket, value, limit = 24) {
    if (bucket.length >= limit || bucket.includes(value)) return;
    bucket.push(value);
}

function buildNeighborhood(allTriples, focusId, maxNodes = 42, maxEdges = 96) {
    const nodes = new Map();
    const edges = [];
    const seenEdges = new Set();
    const frontier = [focusId];
    const visited = new Set();
    const incidentCounts = new Map();
    for (const triple of allTriples) {
        incidentCounts.set(triple.subjectId, (incidentCounts.get(triple.subjectId) || 0) + 1);
        incidentCounts.set(triple.objectId, (incidentCounts.get(triple.objectId) || 0) + 1);
    }

    const isHub = (id) => id !== focusId && (incidentCounts.get(id) || 0) > 24;

    const ensureNode = (id, label, description) => {
        const existing = nodes.get(id);
        if (existing) {
            if (shouldUpgradeNodeText(existing.label, label, id)) existing.label = label;
            if (shouldUpgradeNodeText(existing.description, description, id)) existing.description = description;
            existing.hub = existing.hub || isHub(id);
            return existing;
        }
        if (!nodes.has(id)) {
            nodes.set(id, {
                id,
                label: label || local(id),
                description: description || id,
                isFocus: id === focusId,
                hub: isHub(id),
            });
        }
        return nodes.get(id);
    };

    ensureNode(focusId);

    while (frontier.length && nodes.size < maxNodes && edges.length < maxEdges) {
        const current = frontier.shift();
        if (!current || visited.has(current)) continue;
        visited.add(current);
        if (isHub(current)) continue;

        for (const triple of allTriples) {
            if (edges.length >= maxEdges || nodes.size >= maxNodes) break;
            const touches = triple.subjectId === current || triple.objectId === current;
            if (!touches) continue;
            const edgeKey = `${triple.subjectId}|${triple.predicateId || triple.predicateLabel}|${triple.objectId}`;
            if (seenEdges.has(edgeKey)) continue;
            ensureNode(triple.subjectId, triple.subjectLabel, triple.subjectDescription);
            ensureNode(triple.objectId, triple.objectLabel, triple.objectDescription);
            seenEdges.add(edgeKey);
            edges.push({
                source: triple.subjectId,
                target: triple.objectId,
                label: triple.predicateLabel,
                raw: triple,
            });
            if (!visited.has(triple.subjectId)) frontier.push(triple.subjectId);
            if (!visited.has(triple.objectId)) frontier.push(triple.objectId);
        }
    }

    return {
        nodes: [...nodes.values()],
        edges,
        focusId,
    };
}

function measuredGraphTriples(opResults, operation) {
    const opOrder = operation === 'all'
        ? ['point']
        : [operation];
    const triples = [];
    const seen = new Set();

    for (const op of opOrder) {
        for (const triple of opResults[op]?.lastSummary?.decodedMatches || []) {
            const normalized = normalizeRenderableTriple(triple);
            const key = `${normalized.subjectId}|${normalized.predicateId || normalized.predicateLabel}|${normalized.objectId}`;
            if (seen.has(key)) continue;
            seen.add(key);
            triples.push(normalized);
        }
    }

    return triples;
}

function buildGraphModel(triples) {
    const byNode = new Map();
    const nodes = new Set();

    const ensure = (id, label, description) => {
        if (!byNode.has(id)) {
            byNode.set(id, { id, label: label || local(id), description: description || id, outbound: [], inbound: [] });
        }
        return byNode.get(id);
    };

    for (const triple of triples) {
        nodes.add(triple.subjectId);
        nodes.add(triple.objectId);
        ensure(triple.subjectId, triple.subjectLabel, triple.subjectDescription).outbound.push(triple);
        ensure(triple.objectId, triple.objectLabel, triple.objectDescription).inbound.push(triple);
    }

    return {
        nodes,
        byNode,
    };
}

function renderInspector(target, model, nodeId) {
    if (!target) return;
    if (!model || !nodeId || !model.byNode.has(nodeId)) {
        target.innerHTML = `<div class="text-sm text-white/45">Click a node in the graph to open its relationship explorer.</div>`;
        return;
    }

    const node = model.byNode.get(nodeId);
    const outbound = node.outbound.slice(0, 20);
    const inbound = node.inbound.slice(0, 20);

    const listHtml = (triples, dir) => {
        if (!triples.length) return '<div class="text-white/45 text-sm">None</div>';
        return `<div class="space-y-2">${triples.map((triple) => `
            <div class="rounded-xl border border-white/8 bg-black/20 px-3 py-2 text-xs font-mono text-white/75" title="${dir === 'out' ? triple.objectDescription : triple.subjectDescription}">
                <span class="text-emerald-300" title="${triple.predicateDescription}">${triple.predicateLabel}</span>
                <span class="text-white/35 mx-2">${dir === 'out' ? 'â†’' : 'â†'}</span>
                <span>${dir === 'out' ? triple.objectLabel : triple.subjectLabel}</span>
            </div>`).join('')}</div>`;
    };

    target.innerHTML = `
        <div class="flex items-center justify-between gap-4 flex-wrap mb-4">
            <div>
                <div class="text-xs uppercase tracking-[2px] text-slate-500 mb-1">Node Explorer</div>
                <div class="text-lg font-semibold text-emerald-300">${node.label}</div>
                <div class="text-xs text-white/45 font-mono">${node.description}</div>
            </div>
            <div class="flex gap-4 text-xs text-white/60">
                <div><span class="text-emerald-400 font-semibold">${node.outbound.length}</span> outbound</div>
                <div><span class="text-cyan-400 font-semibold">${node.inbound.length}</span> inbound</div>
            </div>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
                <div class="text-sm font-semibold text-emerald-400 mb-2">Outgoing relationships</div>
                ${listHtml(outbound, 'out')}
            </div>
            <div>
                <div class="text-sm font-semibold text-cyan-400 mb-2">Incoming relationships</div>
                ${listHtml(inbound, 'in')}
            </div>
        </div>
        ${(node.outbound.length > outbound.length || node.inbound.length > inbound.length)
            ? `<div class="text-[11px] text-white/35 mt-3">Showing the first 20 edges in each direction.</div>`
            : ''}`;
}

// ------------------------------ 3D object viewer ------------------------------

function buildMeshes() {
    const phi = (1 + Math.sqrt(5)) / 2;
    const norm = (v) => { const l = Math.hypot(...v) || 1; return v.map((c) => c / l); };
    const edgesByDist = (verts, tol) => {
        const e = [];
        let min = Infinity;
        for (let i = 0; i < verts.length; i++) {
            for (let j = i + 1; j < verts.length; j++) {
                const d = Math.hypot(verts[i][0] - verts[j][0], verts[i][1] - verts[j][1], verts[i][2] - verts[j][2]);
                if (d < min) min = d;
            }
        }
        for (let i = 0; i < verts.length; i++) {
            for (let j = i + 1; j < verts.length; j++) {
                const d = Math.hypot(verts[i][0] - verts[j][0], verts[i][1] - verts[j][1], verts[i][2] - verts[j][2]);
                if (d <= min * (1 + tol)) e.push([i, j]);
            }
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

    const sVerts = [];
    const sEdges = [];
    const rings = 9;
    const seg = 14;
    for (let i = 0; i <= rings; i++) {
        const lat = Math.PI * (i / rings - 0.5);
        for (let j = 0; j < seg; j++) {
            const lon = 2 * Math.PI * (j / seg);
            sVerts.push([Math.cos(lat) * Math.cos(lon), Math.sin(lat), Math.cos(lat) * Math.sin(lon)]);
        }
    }
    for (let i = 0; i <= rings; i++) {
        for (let j = 0; j < seg; j++) {
            const a = i * seg + j;
            sEdges.push([a, j < seg - 1 ? a + 1 : i * seg]);
            if (i < rings) sEdges.push([a, a + seg]);
        }
    }

    const tVerts = [];
    const tEdges = [];
    const R = 0.7;
    const r = 0.3;
    const tu = 16;
    const tv = 9;
    for (let i = 0; i < tu; i++) {
        const u = 2 * Math.PI * (i / tu);
        for (let j = 0; j < tv; j++) {
            const v = 2 * Math.PI * (j / tv);
            tVerts.push([(R + r * Math.cos(v)) * Math.cos(u), r * Math.sin(v), (R + r * Math.cos(v)) * Math.sin(u)]);
        }
    }
    for (let i = 0; i < tu; i++) {
        for (let j = 0; j < tv; j++) {
            const a = i * tv + j;
            tEdges.push([a, i * tv + ((j + 1) % tv)]);
            tEdges.push([a, ((i + 1) % tu) * tv + j]);
        }
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
        this.yaw = 0.6;
        this.pitch = -0.4;
        this.zoom = 1;
        this.spin = true;
        this.dragging = false;
        this._bind();
        this._loop = this._loop.bind(this);
        requestAnimationFrame(this._loop);
    }
    setMesh(name) { if (this.meshes[name]) this.mesh = this.meshes[name]; }
    setSpin(on) { this.spin = on; }
    _bind() {
        const c = this.canvas;
        let lx = 0;
        let ly = 0;
        c.style.touchAction = 'none';
        c.style.cursor = 'grab';
        c.addEventListener('pointerdown', (e) => {
            this.dragging = true;
            this.spin = false;
            lx = e.clientX;
            ly = e.clientY;
            c.setPointerCapture(e.pointerId);
            c.style.cursor = 'grabbing';
        });
        c.addEventListener('pointermove', (e) => {
            if (!this.dragging) return;
            this.yaw += (e.clientX - lx) * 0.01;
            this.pitch += (e.clientY - ly) * 0.01;
            this.pitch = Math.max(-1.5, Math.min(1.5, this.pitch));
            lx = e.clientX;
            ly = e.clientY;
        });
        const end = () => {
            this.dragging = false;
            c.style.cursor = 'grab';
        };
        c.addEventListener('pointerup', end);
        c.addEventListener('pointercancel', end);
        c.addEventListener('wheel', (e) => {
            e.preventDefault();
            this.zoom = Math.max(0.4, Math.min(3, this.zoom * (e.deltaY < 0 ? 1.1 : 0.9)));
        }, { passive: false });
    }
    _resize() {
        const c = this.canvas;
        const dpr = window.devicePixelRatio || 1;
        const w = c.clientWidth || 360;
        const h = c.clientHeight || 300;
        if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
            c.width = Math.round(w * dpr);
            c.height = Math.round(h * dpr);
        }
        return { w, h, dpr };
    }
    _loop() {
        const { w, h, dpr } = this._resize();
        const ctx = this.ctx;
        if (this.spin && !this.dragging) this.yaw += 0.006;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, h);
        const cx = w / 2;
        const cy = h / 2;
        const scale = Math.min(w, h) * 0.36 * this.zoom;
        const cy_ = Math.cos(this.yaw);
        const sy = Math.sin(this.yaw);
        const cp = Math.cos(this.pitch);
        const sp = Math.sin(this.pitch);
        const proj = this.mesh.verts.map(([x, y, z]) => {
            let X = x * cy_ - z * sy;
            let Z = x * sy + z * cy_;
            let Y = y * cp - Z * sp;
            Z = y * sp + Z * cp;
            const persp = 3 / (3 + Z);
            return [cx + X * scale * persp, cy + Y * scale * persp, Z];
        });
        ctx.lineWidth = 1.2;
        for (const [i, j] of this.mesh.edges) {
            const a = proj[i];
            const b = proj[j];
            const depth = (a[2] + b[2]) / 2;
            const t = Math.max(0, Math.min(1, (depth + 1.4) / 2.8));
            ctx.strokeStyle = `rgba(52,211,153,${0.25 + t * 0.6})`;
            ctx.beginPath();
            ctx.moveTo(a[0], a[1]);
            ctx.lineTo(b[0], b[1]);
            ctx.stroke();
        }
        for (const p of proj) {
            const t = Math.max(0, Math.min(1, (p[2] + 1.4) / 2.8));
            ctx.fillStyle = `rgba(96,165,250,${0.4 + t * 0.6})`;
            ctx.beginPath();
            ctx.arc(p[0], p[1], 1.5 + t * 1.8, 0, 7);
            ctx.fill();
        }
        requestAnimationFrame(this._loop);
    }
}

// ------------------------------- Graph viewer --------------------------------

export class GraphViewer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.nodes = [];
        this.edges = [];
        this.nodeMap = new Map();
        this.ox = 0;
        this.oy = 0;
        this.scale = 1;
        this.drag = null;
        this.pan = false;
        this.selectedId = null;
        this.onNodeSelect = null;
        this.pointerMoved = false;
        this.physicsEnabled = false;
        this._bind();
        this._loop = this._loop.bind(this);
        requestAnimationFrame(this._loop);
    }
    setNodeSelectHandler(handler) {
        this.onNodeSelect = handler;
    }
    setData(graph) {
        const source = Array.isArray(graph) ? {
            nodes: [...new Map(graph.flatMap((triple) => [
                [triple.subjectId, { id: triple.subjectId, label: triple.subjectLabel, description: triple.subjectDescription }],
                [triple.objectId, { id: triple.objectId, label: triple.objectLabel, description: triple.objectDescription }],
            ])).values()],
            edges: graph.map((triple) => ({ source: triple.subjectId, target: triple.objectId, label: triple.predicateLabel, raw: triple })),
        } : graph;
        this.nodes = [];
        this.edges = source?.edges ? [...source.edges] : [];
        this.nodeMap = new Map();
        const focusId = source?.focusId || source?.nodes?.find((node) => node?.isFocus)?.id || source?.nodes?.[0]?.id || null;

        const addNode = (baseNode) => {
            if (!baseNode?.id) return;
            const existing = this.nodeMap.get(baseNode.id);
            if (existing) {
                if (shouldUpgradeNodeText(existing.label, baseNode.label, baseNode.id)) existing.label = baseNode.label;
                if (shouldUpgradeNodeText(existing.description, baseNode.description, baseNode.id)) existing.description = baseNode.description;
                return;
            }
            const index = this.nodes.length;
            const total = Math.max((source?.nodes?.length || 0), 1);
            const isFocus = baseNode.id === focusId || baseNode.isFocus === true;
            const orbitIndex = Math.max(index - 1, 0);
            const perRing = 8;
            const ringIndex = Math.floor(orbitIndex / perRing);
            const slot = orbitIndex % perRing;
            const remaining = Math.max(total - 1 - ringIndex * perRing, 1);
            const slotsInRing = Math.min(perRing, remaining);
            const angle = (slot / Math.max(slotsInRing, 1)) * Math.PI * 2 - Math.PI / 2;
            const ring = 170 + ringIndex * 60;
            const node = {
                id: baseNode.id,
                label: baseNode.label ?? local(baseNode.id),
                description: baseNode.description ?? baseNode.id,
                x: isFocus ? 0 : Math.cos(angle) * ring,
                y: isFocus ? 0 : Math.sin(angle) * ring,
                vx: 0,
                vy: 0,
                isFocus,
                hub: baseNode.hub === true,
            };
            this.nodes.push(node);
            this.nodeMap.set(node.id, node);
        };

        for (const baseNode of source?.nodes ?? []) {
            addNode(baseNode);
        }

        for (const edge of this.edges) {
            if (!this.nodeMap.has(edge.source)) {
                addNode({
                    id: edge.source,
                    label: edge.raw?.subjectLabel ?? local(edge.source),
                    description: edge.raw?.subjectDescription ?? edge.source,
                });
            }
            if (!this.nodeMap.has(edge.target)) {
                addNode({
                    id: edge.target,
                    label: edge.raw?.objectLabel ?? local(edge.target),
                    description: edge.raw?.objectDescription ?? edge.target,
                });
            }
        }

        this.ox = 0;
        this.oy = 0;
        this.scale = 1;
        this.selectedId = focusId || this.nodes[0]?.id || null;
        if (this.selectedId && this.onNodeSelect) {
            this.onNodeSelect(this.selectedId);
        }
    }
    _toScreen(p, w, h) { return [w / 2 + (p.x + this.ox) * this.scale, h / 2 + (p.y + this.oy) * this.scale]; }
    _nodeSize(node) {
        const label = String(node?.label || '');
        return {
            w: Math.max(138, Math.min(190, 88 + label.length * 5.2)),
            h: 46,
        };
    }
    _nodeKind(node) {
        if (node?.isFocus) return 'FOCUS';
        if (node?.hub) return 'HUB';
        if (/^https?:\/\//i.test(String(node?.description || ''))) return 'IRI';
        return 'VALUE';
    }
    _nodeColor(node) {
        if (node?.isFocus) return '#f59e0b';
        if (node?.hub) return '#60a5fa';
        if (/^https?:\/\//i.test(String(node?.description || ''))) return '#34d399';
        return '#38bdf8';
    }
    _truncate(text, chars) {
        const value = String(text || '');
        return value.length > chars ? `${value.slice(0, Math.max(chars - 1, 1))}...` : value;
    }
    _roundRect(ctx, x, y, w, h, r) {
        ctx.beginPath();
        ctx.moveTo(x + r, y);
        ctx.arcTo(x + w, y, x + w, y + h, r);
        ctx.arcTo(x + w, y + h, x, y + h, r);
        ctx.arcTo(x, y + h, x, y, r);
        ctx.arcTo(x, y, x + w, y, r);
        ctx.closePath();
    }
    _rectEdgePoint(from, to, size) {
        const dx = to[0] - from[0];
        const dy = to[1] - from[1];
        if (!dx && !dy) return from;
        const sx = size.w / 2 / Math.max(Math.abs(dx), 0.0001);
        const sy = size.h / 2 / Math.max(Math.abs(dy), 0.0001);
        const t = Math.min(sx, sy);
        return [from[0] + dx * t, from[1] + dy * t];
    }
    _arrowHead(ctx, x, y, ux, uy, color) {
        const a = 9;
        const px = -uy;
        const py = ux;
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x - ux * a + px * a * 0.45, y - uy * a + py * a * 0.45);
        ctx.lineTo(x - ux * a - px * a * 0.45, y - uy * a - py * a * 0.45);
        ctx.closePath();
        ctx.fill();
    }
    _findNodeAt(mx, my, w, h) {
        for (const node of this.nodes) {
            const [sx, sy] = this._toScreen(node, w, h);
            const size = this._nodeSize(node);
            if (mx >= sx - size.w / 2 && mx <= sx + size.w / 2 && my >= sy - size.h / 2 && my <= sy + size.h / 2) {
                return node;
            }
        }
        return null;
    }
    _bind() {
        const c = this.canvas;
        let lx = 0;
        let ly = 0;
        c.style.touchAction = 'none';
        c.style.cursor = 'grab';
        c.addEventListener('pointerdown', (e) => {
            const r = c.getBoundingClientRect();
            const w = c.clientWidth;
            const h = c.clientHeight;
            const mx = e.clientX - r.left;
            const my = e.clientY - r.top;
            this.drag = this._findNodeAt(mx, my, w, h);
            this.pan = !this.drag;
            this.pointerMoved = false;
            lx = e.clientX;
            ly = e.clientY;
            c.setPointerCapture(e.pointerId);
            c.style.cursor = 'grabbing';
        });
        c.addEventListener('pointermove', (e) => {
            const r = c.getBoundingClientRect();
            const w = c.clientWidth;
            const h = c.clientHeight;
            const hoverNode = this._findNodeAt(e.clientX - r.left, e.clientY - r.top, w, h);
            c.title = hoverNode ? hoverNode.description : '';
            const dx = e.clientX - lx;
            const dy = e.clientY - ly;
            if (Math.abs(dx) + Math.abs(dy) > 2) this.pointerMoved = true;
            lx = e.clientX;
            ly = e.clientY;
            if (this.drag) {
                this.drag.x += dx / this.scale;
                this.drag.y += dy / this.scale;
                this.drag.vx = 0;
                this.drag.vy = 0;
            } else if (this.pan) {
                this.ox += dx / this.scale;
                this.oy += dy / this.scale;
            }
        });
        c.addEventListener('pointerup', (e) => {
            const r = c.getBoundingClientRect();
            const w = c.clientWidth;
            const h = c.clientHeight;
            const mx = e.clientX - r.left;
            const my = e.clientY - r.top;
            const clicked = this._findNodeAt(mx, my, w, h);
            if (!this.pointerMoved && clicked) {
                this.selectedId = clicked.id;
                this.onNodeSelect?.(clicked.id);
            }
            this.drag = null;
            this.pan = false;
            c.style.cursor = 'grab';
        });
        c.addEventListener('pointercancel', () => {
            this.drag = null;
            this.pan = false;
            c.style.cursor = 'grab';
            c.title = '';
        });
        c.addEventListener('wheel', (e) => {
            e.preventDefault();
            this.scale = Math.max(0.3, Math.min(3, this.scale * (e.deltaY < 0 ? 1.1 : 0.9)));
        }, { passive: false });
    }
    _physics() {
        const N = this.nodes;
        for (let i = 0; i < N.length; i++) {
            const a = N[i];
            for (let j = i + 1; j < N.length; j++) {
                const b = N[j];
                const dx = a.x - b.x;
                const dy = a.y - b.y;
                const d2 = dx * dx + dy * dy || 1;
                const f = 6000 / d2;
                const d = Math.sqrt(d2);
                const ux = dx / d;
                const uy = dy / d;
                a.vx += ux * f;
                a.vy += uy * f;
                b.vx -= ux * f;
                b.vy -= uy * f;
            }
            a.vx -= a.x * 0.004;
            a.vy -= a.y * 0.004;
            if (a.isFocus && a !== this.drag) {
                a.vx -= a.x * 0.12;
                a.vy -= a.y * 0.12;
            }
        }
        for (const edge of this.edges) {
            const a = this.nodeMap.get(edge.source);
            const b = this.nodeMap.get(edge.target);
            if (!a || !b) continue;
            const dx = b.x - a.x;
            const dy = b.y - a.y;
            const d = Math.hypot(dx, dy) || 1;
            const f = (d - 120) * 0.02;
            const ux = dx / d;
            const uy = dy / d;
            a.vx += ux * f;
            a.vy += uy * f;
            b.vx -= ux * f;
            b.vy -= uy * f;
        }
        for (const node of N) {
            if (node === this.drag) continue;
            node.vx *= 0.85;
            node.vy *= 0.85;
            if (node.isFocus) {
                node.x = 0;
                node.y = 0;
                node.vx = 0;
                node.vy = 0;
                continue;
            }
            node.x = Math.max(-900, Math.min(900, node.x + node.vx));
            node.y = Math.max(-700, Math.min(700, node.y + node.vy));
        }
    }
    _loop() {
        const c = this.canvas;
        const dpr = window.devicePixelRatio || 1;
        const w = c.clientWidth || 360;
        const h = c.clientHeight || 300;
        if (c.width !== Math.round(w * dpr) || c.height !== Math.round(h * dpr)) {
            c.width = Math.round(w * dpr);
            c.height = Math.round(h * dpr);
        }
        const ctx = this.ctx;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, h);
        ctx.fillStyle = 'rgba(255,255,255,0.045)';
        for (let gx = 12; gx < w; gx += 24) {
            for (let gy = 12; gy < h; gy += 24) {
                ctx.fillRect(gx, gy, 1.4, 1.4);
            }
        }
        if (this.nodes.length && this.physicsEnabled) this._physics();
        ctx.lineWidth = 1.5;
        ctx.strokeStyle = 'rgba(148,163,184,0.48)';
        ctx.font = '10px JetBrains Mono, monospace';
        ctx.textAlign = 'center';
        for (const edge of this.edges) {
            const a = this.nodeMap.get(edge.source);
            const b = this.nodeMap.get(edge.target);
            if (!a || !b) continue;
            const ac = this._toScreen(a, w, h);
            const bc = this._toScreen(b, w, h);
            const [ax, ay] = this._rectEdgePoint(ac, bc, this._nodeSize(a));
            const [bx, by] = this._rectEdgePoint(bc, ac, this._nodeSize(b));
            const dx = bx - ax;
            const dy = by - ay;
            const d = Math.hypot(dx, dy) || 1;
            const ux = dx / d;
            const uy = dy / d;
            const lineColor = edge.source === this.selectedId || edge.target === this.selectedId
                ? 'rgba(245,158,11,0.82)'
                : 'rgba(148,163,184,0.52)';
            ctx.strokeStyle = lineColor;
            ctx.lineWidth = edge.source === this.selectedId || edge.target === this.selectedId ? 2.2 : 1.5;
            ctx.beginPath();
            ctx.moveTo(ax, ay);
            ctx.lineTo(bx, by);
            ctx.stroke();
            this._arrowHead(ctx, bx, by, ux, uy, lineColor);
            const label = this._truncate(edge.label, 28);
            const mx = (ax + bx) / 2;
            const my = (ay + by) / 2;
            const tw = Math.min(ctx.measureText(label).width + 12, 180);
            this._roundRect(ctx, mx - tw / 2, my - 13, tw, 18, 7);
            ctx.fillStyle = 'rgba(11,15,23,0.82)';
            ctx.fill();
            ctx.strokeStyle = 'rgba(148,163,184,0.2)';
            ctx.stroke();
            ctx.fillStyle = '#cbd5e1';
            ctx.fillText(label, mx, my);
        }
        ctx.textAlign = 'left';
        for (const node of this.nodes) {
            const [cx, cy] = this._toScreen(node, w, h);
            const size = this._nodeSize(node);
            const x = cx - size.w / 2;
            const y = cy - size.h / 2;
            const selected = node.id === this.selectedId;
            const fill = this._nodeColor(node);
            this._roundRect(ctx, x, y, size.w, size.h, 10);
            ctx.fillStyle = fill;
            ctx.fill();
            ctx.strokeStyle = selected ? '#fde68a' : 'rgba(255,255,255,0.18)';
            ctx.lineWidth = selected ? 2 : 1;
            ctx.stroke();
            ctx.fillStyle = 'rgba(11,15,23,0.72)';
            ctx.font = '700 9px Inter, sans-serif';
            ctx.fillText(this._nodeKind(node), x + 10, y + 15);
            ctx.fillStyle = '#0b0f17';
            ctx.font = selected ? '700 12px Inter, sans-serif' : '600 12px Inter, sans-serif';
            ctx.fillText(this._truncate(node.label, Math.max(10, Math.floor((size.w - 18) / 7))), x + 10, y + 34);
        }
        if (!this.nodes.length) {
            ctx.fillStyle = 'rgba(148,163,184,0.5)';
            ctx.fillText('Run a graph operation to render the graphâ€¦', 16, h / 2);
        }
        requestAnimationFrame(this._loop);
    }
}

// --------------------------- Graph benchmark logic ---------------------------

const GRAPH_DATASETS = {
    'synthetic-10k': {
        manifestId: 'synthetic-10k',
        storageFormat: 'synthetic',
        label: 'Synthetic 10K triples',
    },
    'schemaorg-30': {
        manifestId: 'schemaorg-30-current-https',
        storageFormat: 'nt',
        fallbackStorageFormats: ['q42'],
        label: 'Schema.org 30.0 current HTTPS',
    },
    'wikidata-sample': {
        unavailable: 'Wikidata sample is not shipped in this repo yet, so this selector is disabled instead of pretending to run it.',
    },
};

const GRAPH_OPS = ['point', 'twohop', 'filter', 'ingest'];
const graphManifestCache = new Map();
const graphDatasetCache = new Map();
let ontologyGraphDatasetPromise = null;

function normalizeDatasetKey(value) {
    return String(value || '').trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

function extractSubjectFromPattern(pattern) {
    const match = String(pattern || '').trim().match(/^<([^>]+)>\s+\?p\s+\?o\b/);
    return match?.[1] || null;
}

async function getOntologyGraphDatasets() {
    if (!ontologyGraphDatasetPromise) {
        ontologyGraphDatasetPromise = (async () => {
            try {
                const res = await fetch('./playground/vfs-manifest.json');
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                const manifest = await res.json();
                const datasets = [];
                for (const ds of manifest.datasets || []) {
                    if (!ds?.url || !String(ds.url).endsWith('.q42')) continue;
                    const key = `ontology:${normalizeDatasetKey(ds.id || ds.label)}`;
                    const pointSubject = extractSubjectFromPattern(ds.sampleQueries?.[0]?.pattern)
                        || extractSubjectFromPattern(ds.sampleQueries?.[1]?.pattern)
                        || null;
                    datasets.push({
                        key,
                        id: ds.id,
                        label: ds.label || ds.id,
                        group: ds.group || ds.profile || ds.source || 'Ontologies',
                        config: {
                            manifestId: null,
                            storageFormat: 'q42',
                            label: ds.label || ds.id,
                            manifest: {
                                id: ds.id,
                                label: ds.label || ds.id,
                                paths: { q42: ds.url },
                                queries: {
                                    point_subject: pointSubject || undefined,
                                    twohop_start: pointSubject || undefined,
                                    filter_predicate: 'http://www.w3.org/1999/02/22-rdf-syntax-ns#type',
                                },
                                dataset_info: {
                                    label: ds.label || ds.id,
                                    source_format: 'q42',
                                    source: ds.source || 'manifest',
                                    namespace: ds.namespace || '',
                                    homepage: ds.homepage || '',
                                },
                                _manifestUrl: res.url,
                            },
                        },
                    });
                }
                return datasets;
            } catch (error) {
                console.warn('[benchmark-live] Ontology manifest unavailable:', error);
                return [];
            }
        })();
    }
    return ontologyGraphDatasetPromise;
}

async function getManifest(manifestId) {
    if (!graphManifestCache.has(manifestId)) {
        graphManifestCache.set(manifestId, await fetchManifest(manifestId));
    }
    return graphManifestCache.get(manifestId);
}

async function getDatasetProfile(selection) {
    const config = GRAPH_DATASETS[selection];
    if (!config) throw new Error(`Unknown graph dataset: ${selection}`);
    if (config.unavailable) return { unavailable: config.unavailable };
    const manifest = config.manifest ?? await getManifest(config.manifestId);
    const candidateFormats = [config.storageFormat, ...(config.fallbackStorageFormats || [])];
    for (const format of candidateFormats) {
        const cacheKey = `${selection}:${format}`;
        if (graphDatasetCache.has(cacheKey)) {
            return graphDatasetCache.get(cacheKey);
        }
        try {
            const dataset = await loadDataset(manifest, format);
            const profile = {
                config,
                manifest,
                dataset,
                storageFormat: format,
                queries: queriesForManifest(manifest, selection),
            };
            graphDatasetCache.set(cacheKey, profile);
            return profile;
        } catch (error) {
            if (format === candidateFormats[candidateFormats.length - 1]) throw error;
        }
    }
}

function _graphDatasetMetaTextLegacy(selection, profile) {
    if (!profile) return '';
    if (profile.unavailable) return profile.unavailable;
    const info = profile.manifest.dataset_info || {};
    return `${profile.config.label} Â· ${profile.dataset.label} Â· ${profile.dataset.quinCount.toLocaleString()} quins Â· source ${info.source_format || profile.dataset.format}`;
}

function graphDatasetMetaText(selection, profile) {
    if (!profile) return '';
    if (profile.unavailable) return profile.unavailable;
    const info = profile.manifest.dataset_info || {};
    const formatNote = profile.storageFormat !== profile.config.storageFormat
        ? ` Â· fallback ${profile.storageFormat.toUpperCase()}`
        : '';
    return `${profile.config.label} Â· ${profile.dataset.label} Â· ${profile.dataset.quinCount.toLocaleString()} quins Â· source ${info.source_format || profile.dataset.format}${formatNote}`;
}

export async function initializeGraphBenchmarkUi() {
    const datasetSelect = document.getElementById('graph-dataset');
    const meta = document.getElementById('graph-dataset-meta');
    if (!datasetSelect) return;
    const ontologyDatasets = await getOntologyGraphDatasets();
    for (const ds of ontologyDatasets) {
        if (!GRAPH_DATASETS[ds.key]) {
            GRAPH_DATASETS[ds.key] = ds.config;
        }
    }
    const dynamicOptgroups = [...datasetSelect.querySelectorAll('optgroup[data-dynamic="ontology"]')];
    dynamicOptgroups.forEach((group) => group.remove());
    if (ontologyDatasets.length) {
        const groups = new Map();
        for (const ds of ontologyDatasets) {
            const groupName = ds.group || 'Ontologies';
            if (!groups.has(groupName)) groups.set(groupName, []);
            groups.get(groupName).push(ds);
        }
        for (const [groupName, entries] of groups) {
            const optgroup = document.createElement('optgroup');
            optgroup.label = groupName;
            optgroup.dataset.dynamic = 'ontology';
            for (const entry of entries) {
                if (datasetSelect.querySelector(`option[value="${entry.key}"]`)) continue;
                const option = document.createElement('option');
                option.value = entry.key;
                option.textContent = entry.label;
                optgroup.appendChild(option);
            }
            if (optgroup.children.length) datasetSelect.appendChild(optgroup);
        }
    }
    for (const option of [...datasetSelect.options]) {
        const config = GRAPH_DATASETS[option.value];
        if (config?.unavailable) {
            option.disabled = true;
            option.textContent = `${option.textContent} (not bundled)`;
        }
    }
    const selection = datasetSelect.value;
    const profile = await getDatasetProfile(selection);
    if (meta) meta.textContent = graphDatasetMetaText(selection, profile);
}

async function measureAsyncSeries(runs, fn) {
    const latencies = [];
    const throughputs = [];
    let lastSummary = null;
    for (let i = 0; i < runs; i++) {
        const t0 = performance.now();
        lastSummary = await fn(i);
        const dt = performance.now() - t0;
        latencies.push(dt);
        throughputs.push(dt > 0 ? 1000 / dt : 0);
    }
    return {
        latencies,
        throughputs,
        summary: stats(latencies),
        lastSummary,
    };
}

function decodeResultMatches(matches, labelMap) {
    return matches.map((match) => {
        const subject = decodeMatchEntry(match.s, labelMap, 'subject');
        const predicate = decodeMatchEntry(match.p, labelMap, 'predicate');
        const object = decodeMatchEntry(match.o, labelMap, 'object');
        return {
            subject: subject?.description ?? null,
            predicate: predicate?.description ?? null,
            object: object?.description ?? null,
            subjectLabel: subject?.label ?? null,
            predicateLabel: predicate?.label ?? null,
            objectLabel: object?.label ?? null,
            subjectId: subject?.id ?? null,
            predicateId: predicate?.id ?? null,
            objectId: object?.id ?? null,
            subjectHash: match.s == null || match.s === '' ? null : hashFromValue(match.s),
            predicateHash: match.p == null || match.p === '' ? null : hashFromValue(match.p),
            objectHash: match.o == null || match.o === '' ? null : hashFromValue(match.o),
        };
    });
}

function runSingleQuery(wasm, query, db, maxResults = 128) {
    const raw = wasm.execute_ntriples_query(query, db, maxResults);
    const parsed = JSON.parse(raw);
    if (parsed.error) throw new Error(parsed.error);
    return parsed;
}

function graphOperationLabel(op) {
    return op === 'twohop' ? 'Two-hop' : op === 'ingest' ? 'Ingest' : op[0].toUpperCase() + op.slice(1);
}

function buildRenderableTriples(dataset, preferredFocusId = null) {
    if (dataset.triples?.length) return dataset.triples.map(normalizeRenderableTriple);

    const initialLimit = Math.min(dataset.quinCount, 2400);
    const initialTriples = decodeFlatDb(dataset.db, dataset.labelMap, initialLimit).map(normalizeRenderableTriple);
    if (!preferredFocusId || initialLimit >= dataset.quinCount) return initialTriples;

    const hasPreferredFocus = initialTriples.some((triple) =>
        triple.subjectId === preferredFocusId || triple.objectId === preferredFocusId
    );
    if (hasPreferredFocus) return initialTriples;

    return decodeFlatDb(dataset.db, dataset.labelMap, dataset.quinCount).map(normalizeRenderableTriple);
}

function chooseFocusNode(opResults, fallbackTriple) {
    for (const key of ['point', 'twohop', 'filter']) {
        const result = opResults[key];
        const first = result?.lastSummary?.decodedMatches?.[0];
        if (first?.subjectId) return first.subjectId;
        if (first?.objectId) return first.objectId;
    }
    return fallbackTriple?.subjectId || null;
}

export async function runGraphLive({ wasm, charts, output, viewer, operation, datasetId, inspectorTarget }) {
    if (typeof wasm?.execute_ntriples_query !== 'function') {
        if (output) output.textContent = 'WASM execute_ntriples_query unavailable.';
        return null;
    }

    const selection = datasetId || document.getElementById('graph-dataset')?.value || 'synthetic-10k';
    const profile = await getDatasetProfile(selection);
    if (profile.unavailable) {
        if (output) output.textContent = profile.unavailable;
        return null;
    }

    const meta = document.getElementById('graph-dataset-meta');
    if (meta) meta.textContent = graphDatasetMetaText(selection, profile);

    const { manifest, dataset, queries, config } = profile;
    const chosenOps = operation === 'all' ? GRAPH_OPS : [operation];
    const opResults = {};

    const pointRunner = () => {
        const res = runSingleQuery(wasm, queries.point, dataset.db, 96);
        return {
            raw: res,
            decodedMatches: decodeResultMatches(res.matches, dataset.labelMap),
        };
    };

    const filterRunner = () => {
        const res = runSingleQuery(wasm, queries.filter, dataset.db, Math.min(dataset.quinCount, 20000));
        return {
            raw: res,
            decodedMatches: decodeResultMatches(res.matches, dataset.labelMap),
        };
    };

    const twohopRunner = () => {
        const firstHop = runSingleQuery(wasm, queries.twohop1, dataset.db, 16);
        let secondHop = { matches: [], vm_cycles: 0, direct_jump_ops: 0, lexicon_lookup_ops: 0 };
        if (queries.twohop2) {
            secondHop = runSingleQuery(wasm, queries.twohop2, dataset.db, 32);
        } else if (firstHop.matches[0]) {
            const nextNode = decodeMatchLabel(firstHop.matches[0].o, dataset.labelMap);
            const nextQuery = `<${nextNode}> ?p ?o .`;
            secondHop = runSingleQuery(wasm, nextQuery, dataset.db, 32);
        }
        return {
            raw: {
                matches: [...firstHop.matches, ...secondHop.matches],
                vm_cycles: firstHop.vm_cycles + secondHop.vm_cycles,
                direct_jump_ops: firstHop.direct_jump_ops + secondHop.direct_jump_ops,
                lexicon_lookup_ops: firstHop.lexicon_lookup_ops + secondHop.lexicon_lookup_ops,
            },
            decodedMatches: [
                ...decodeResultMatches(firstHop.matches, dataset.labelMap),
                ...decodeResultMatches(secondHop.matches, dataset.labelMap),
            ],
        };
    };

    const ingestRunner = async () => {
        const fresh = await loadDataset(manifest, profile.storageFormat || config.storageFormat);
        return {
            loadMs: fresh.loadMs,
            quinCount: fresh.quinCount,
        };
    };

    for (const op of chosenOps) {
        if (op === 'point') {
            opResults.point = await measureAsyncSeries(16, pointRunner);
        } else if (op === 'twohop') {
            opResults.twohop = await measureAsyncSeries(12, twohopRunner);
        } else if (op === 'filter') {
            opResults.filter = await measureAsyncSeries(12, filterRunner);
        } else if (op === 'ingest') {
            opResults.ingest = await measureAsyncSeries(6, ingestRunner);
        }
    }

    const requestedFocusNode = chooseFocusNode(opResults)
        || (queries.pointSubject ? fullHash(qHash(queries.pointSubject)) : null);
    const measuredTriples = measuredGraphTriples(opResults, operation);
    const renderTriples = measuredTriples.length
        ? measuredTriples
        : buildRenderableTriples(dataset, requestedFocusNode);
    const focusNode = requestedFocusNode || renderTriples[0]?.subjectId || null;
    const graphModel = buildGraphModel(renderTriples);
    const neighborhood = buildNeighborhood(renderTriples, focusNode || renderTriples[0]?.subjectId, 44, 100);

    if (viewer) {
        viewer.setData(neighborhood);
        viewer.setNodeSelectHandler((nodeId) => renderInspector(inspectorTarget, graphModel, nodeId));
    }
    renderInspector(inspectorTarget, graphModel, focusNode || neighborhood.nodes[0]?.id);

    if (charts?.graphLatency) {
        charts.graphLatency.data.labels = GRAPH_OPS.map((op) => graphOperationLabel(op));
        charts.graphLatency.data.datasets = [{
            label: 'p50 latency (measured ms)',
            data: GRAPH_OPS.map((op) => opResults[op]?.summary?.p50 ?? 0),
            backgroundColor: 'rgba(52, 211, 153, 0.5)',
            borderColor: 'rgba(52, 211, 153, 1)',
            borderWidth: 1,
        }];
        charts.graphLatency.update();
    }

    if (charts?.graphThroughput) {
        const activeOps = operation === 'all' ? GRAPH_OPS : [operation];
        charts.graphThroughput.data.labels = Array.from({ length: Math.max(...activeOps.map((op) => opResults[op]?.throughputs?.length || 0)) }, (_, i) => `run ${i + 1}`);
        charts.graphThroughput.data.datasets = activeOps
            .filter((op) => opResults[op]?.throughputs?.length)
            .map((op, idx) => {
                const palette = [
                    ['rgba(59, 130, 246, 1)', 'rgba(59, 130, 246, 0.12)'],
                    ['rgba(16, 185, 129, 1)', 'rgba(16, 185, 129, 0.12)'],
                    ['rgba(250, 204, 21, 1)', 'rgba(250, 204, 21, 0.12)'],
                    ['rgba(244, 114, 182, 1)', 'rgba(244, 114, 182, 0.12)'],
                ][idx % 4];
                return {
                    label: `${graphOperationLabel(op)} ops/sec`,
                    data: opResults[op].throughputs,
                    borderColor: palette[0],
                    backgroundColor: palette[1],
                    fill: false,
                    tension: 0.3,
                };
            });
        charts.graphThroughput.update();
    }

    const summaryLines = chosenOps.map((op) => {
        const result = opResults[op];
        if (!result) return null;
        if (op === 'ingest') {
            return `  ingest   p50=${fmtMs(result.summary.p50)}  p95=${fmtMs(result.summary.p95)}  ${dataset.quinCount.toLocaleString()} quins`;
        }
        const matches = result.lastSummary?.decodedMatches?.length ?? result.lastSummary?.raw?.matches?.length ?? 0;
        return `  ${op.padEnd(8)} p50=${fmtMs(result.summary.p50)}  p95=${fmtMs(result.summary.p95)}  matches=${matches}`;
    }).filter(Boolean);

    const focusInfo = focusNode ? graphModel.byNode.get(focusNode) : null;
    const focusSummary = focusInfo
        ? `Focused neighborhood: ${focusInfo.label} (${focusInfo.description})`
        : focusNode ? `Focused neighborhood: ${focusNode}` : 'No focus node available';
    const sourceNote = dataset.labelMap?.size
        ? 'Explorer labels come from the parsed dataset terms, with full IRIs available on hover.'
        : 'Explorer is showing hashed node IDs because this storage format does not currently expose labels.';
    const viewNote = operation === 'all'
        ? 'Graph panel is scoped to the measured point-lookup neighborhood; two-hop, filter, and ingest stay measured without taking over the default view.'
        : 'Graph panel renders the measured matches from the selected graph operation.';

    if (output) {
        output.textContent =
            `âœ“ Graph benchmark (REAL â€” execute_ntriples_query over ${dataset.label})\n` +
            `Dataset: ${config.label}\n` +
            `Load path: ${dataset.format} Â· ${dataset.quinCount.toLocaleString()} quins Â· initial load ${fmtMs(dataset.loadMs)}\n` +
            `Operations measured:\n${summaryLines.join('\n')}\n\n` +
            `${focusSummary}\n` +
            `${sourceNote}\n\n` +
            `${viewNote}\n\n` +
            `Latency chart = measured p50 per operation.\n` +
            `Throughput chart = real per-run ops/sec over time.`;
    }

    return {
        dataset,
        opResults,
        graphModel,
        renderTriples,
        focusNode,
    };
}

// -------------------------- Spatial benchmark logic --------------------------

const SPATIAL_OPS = ['ga', 'topology', 'indexing', 'interval'];

function spatialFocusOp(operation, perfByOp) {
    if (operation && operation !== 'all' && perfByOp?.[operation]) return operation;
    if (perfByOp?.ga) return 'ga';
    return Object.keys(perfByOp || {})[0] || 'ga';
}

function shapeForSpatialFocus(operation, dimension) {
    if (dimension === '4d') {
        if (operation === 'indexing') return 'cube';
        if (operation === 'topology') return 'sphere';
        return 'torus';
    }
    if (operation === 'topology') return 'sphere';
    if (operation === 'indexing') return 'cube';
    if (operation === 'interval') return dimension === '2d' ? 'tetrahedron' : 'octahedron';
    if (operation === 'ga') return dimension === '2d' ? 'tetrahedron' : 'icosahedron';
    return dimension === '2d' ? 'cube' : 'icosahedron';
}

function spatialViewerLabel(shape) {
    return {
        tetrahedron: 'Tetrahedron',
        cube: 'Cube',
        octahedron: 'Octahedron',
        icosahedron: 'Icosahedron',
        sphere: 'Sphere',
        torus: 'Torus',
    }[shape] || shape;
}

function spatialOpLabel(op) {
    return {
        ga: 'Geometric Algebra',
        topology: 'Topology',
        indexing: 'Indexing',
        interval: 'Interval',
    }[op] || String(op).toUpperCase();
}

function spatialRepeats(requestedCount) {
    return {
        ga: Math.max(64, Math.min(2048, Math.round(requestedCount / 40))),
        topology: Math.max(32, Math.min(512, Math.round(requestedCount / 80))),
        indexing: Math.max(8, Math.min(64, Math.round(requestedCount / 400))),
        interval: Math.max(16, Math.min(256, Math.round(requestedCount / 120))),
    };
}

function measureSpatialOperation(wasm, op, dims, requestedCount, repeats) {
    const latencies = [];
    const throughputs = [];
    let backend = 'wasm';
    let note = '';
    let sample = null;

    if (op === 'ga') {
        if (typeof wasm?.geometric_algebra_operation !== 'function') throw new Error('WASM geometric_algebra_operation unavailable.');
        for (let i = 0; i < 12; i++) {
            const payload = JSON.stringify({ a: randomMultivector(dims), b: randomMultivector(dims), op: 'geo' });
            const t0 = performance.now();
            for (let j = 0; j < repeats.ga; j++) {
                sample = parseMaybeJson(wasm.geometric_algebra_operation(payload));
            }
            const dt = performance.now() - t0;
            latencies.push(dt / repeats.ga);
            throughputs.push(dt > 0 ? (repeats.ga * 1000) / dt : 0);
        }
        note = `sample grades=${sample.grades.length} compute_ops=${sample.compute_ops} Ã‚Â· ${repeats.ga}x batched`;
    } else if (op === 'topology') {
        if (typeof wasm?.geosparql_operation_wasm !== 'function') throw new Error('WASM geosparql_operation_wasm unavailable.');
        for (let i = 0; i < 12; i++) {
            const payload = sampleTopologyPayload(i + (dims === '4d' ? 6 : dims === '3d' ? 3 : 0));
            const t0 = performance.now();
            for (let j = 0; j < repeats.topology; j++) {
                sample = parseMaybeJson(wasm.geosparql_operation_wasm(payload));
            }
            const dt = performance.now() - t0;
            latencies.push(dt / repeats.topology);
            throughputs.push(dt > 0 ? (repeats.topology * 1000) / dt : 0);
        }
        note = `GeoSPARQL ${sample.predicate}=${JSON.stringify(sample.result)} Ã‚Â· ${repeats.topology}x batched`;
    } else if (op === 'indexing') {
        if (typeof wasm?.spatial_encode_wasm !== 'function') throw new Error('WASM spatial_encode_wasm unavailable.');
        const payload = sampleEncodingPayload(dims, requestedCount);
        for (let i = 0; i < 10; i++) {
            const t0 = performance.now();
            for (let j = 0; j < repeats.indexing; j++) {
                sample = parseMaybeJson(wasm.spatial_encode_wasm(payload));
            }
            const dt = performance.now() - t0;
            latencies.push(dt / repeats.indexing);
            throughputs.push(dt > 0 ? (repeats.indexing * 1000) / dt : 0);
        }
        note = `${sample.quin_count.toLocaleString()} quins Ã‚Â· ${sample.memory_kb} KB Ã‚Â· ${repeats.indexing}x batched`;
    } else if (op === 'interval') {
        backend = 'js-fallback';
        for (let i = 0; i < 12; i++) {
            const mult = dims === '4d' ? 4 : dims === '3d' ? 2 : 1;
            const t0 = performance.now();
            for (let j = 0; j < repeats.interval; j++) {
                sample = intervalBenchmark(Math.min(requestedCount / 50, 5000), mult + i + j);
            }
            const dt = performance.now() - t0;
            latencies.push(dt / repeats.interval);
            throughputs.push(dt > 0 ? (repeats.interval * 1000) / dt : 0);
        }
        note = `Allen interval algebra benchmark (honest JS fallback until dedicated WASM export exists) Ã‚Â· ${repeats.interval}x batched`;
    }

    const summary = stats(latencies);
    return {
        backend,
        note,
        sample,
        latencies,
        throughputs,
        summary,
        opsPerSec: percentile(throughputs, 0.5),
    };
}

function randomMultivector(dimension) {
    const width = dimension === '4d' ? 12 : dimension === '3d' ? 8 : 6;
    return Array.from({ length: width }, (_, i) => Math.round((Math.sin(i + Math.random() * 3) * 2) * 100) / 100);
}

function relationResult(a, b) {
    if (a.end < b.start) return 'Before';
    if (a.start > b.end) return 'After';
    if (a.end === b.start) return 'Meets';
    if (a.start === b.end) return 'MetBy';
    if (a.start === b.start && a.end === b.end) return 'Equal';
    if (a.start < b.start && a.end < b.end) return 'Overlaps';
    if (a.start > b.start && a.end > b.end) return 'OverlappedBy';
    return 'During';
}

function intervalBenchmark(count, multiplier) {
    const intervals = Array.from({ length: count }, (_, i) => ({
        start: i * multiplier,
        end: i * multiplier + multiplier + (i % 5),
    }));
    let hits = 0;
    const t0 = performance.now();
    for (let i = 0; i < intervals.length - 1; i++) {
        if (relationResult(intervals[i], intervals[i + 1]) !== 'After') hits++;
    }
    const dt = performance.now() - t0;
    return { hits, dt };
}

function sampleTopologyPayload(complexity) {
    const size = 10 + complexity;
    return JSON.stringify({
        geoA: `POLYGON((0 0, ${size} 0, ${size} ${size}, 0 ${size}, 0 0))`,
        geoB: `POINT(${(size / 2).toFixed(1)} ${(size / 2).toFixed(1)})`,
        op: 'contains',
        crs: '4326',
    });
}

function sampleEncodingPayload(dimension, count) {
    const detail = Math.max(1, Math.min(5, Math.round(Math.log10(Math.max(count, 100)))));
    const type = dimension === '4d' ? 'torus' : dimension === '3d' ? 'sphere' : 'cube';
    return JSON.stringify({ type, detail });
}

function parseMaybeJson(value) {
    return typeof value === 'string' ? JSON.parse(value) : value;
}

export function runSpatialLive({ wasm, charts, output, count, operation, dimension }) {
    const selected = operation === 'all' ? SPATIAL_OPS : [operation];
    const dims = dimension || '3d';
    const requestedCount = Math.max(100, Math.min(Number(count) || 10000, 200000));
    const perfByOp = {};
    const repeats = spatialRepeats(requestedCount);

    for (const op of selected) {
        perfByOp[op] = measureSpatialOperation(wasm, op, dims, requestedCount, repeats);
    }

    if (charts?.spatialPerformance) {
        charts.spatialPerformance.data.labels = selected.map((op) => spatialOpLabel(op));
        charts.spatialPerformance.data.datasets = [{
            label: 'Median ops/sec (measured)',
            data: selected.map((op) => perfByOp[op]?.opsPerSec ?? 0),
            backgroundColor: 'rgba(52, 211, 153, 0.5)',
            borderColor: 'rgba(52, 211, 153, 1)',
            borderWidth: 1,
        }];
        charts.spatialPerformance.update();
    }

    const focusOp = spatialFocusOp(operation, perfByOp);
    const scalingDims = ['2d', '3d', '4d'];
    const scalingProfile = {};
    for (const scalingDim of scalingDims) {
        scalingProfile[scalingDim] = measureSpatialOperation(
            wasm,
            focusOp,
            scalingDim,
            requestedCount,
            spatialRepeats(requestedCount),
        );
    }

    if (charts?.spatialScaling) {
        charts.spatialScaling.data.labels = scalingDims.map((value) => value.toUpperCase());
        charts.spatialScaling.data.datasets = [{
            label: `${spatialOpLabel(focusOp)} p50 latency (ms)`,
            data: scalingDims.map((value) => scalingProfile[value]?.summary?.p50 ?? 0),
            borderColor: 'rgba(59, 130, 246, 1)',
            backgroundColor: 'rgba(59, 130, 246, 0.12)',
            fill: true,
            tension: 0.3,
        }];
        charts.spatialScaling.update();
    }

    if (output) {
        output.textContent =
            `Spatial benchmark (${dims.toUpperCase()} | ${requestedCount.toLocaleString()} objects requested)\n` +
            selected.map((op) => {
                const result = perfByOp[op];
                return `  ${op.padEnd(9)} ${fmtOps(result.opsPerSec).padStart(10)} ops/sec  p50=${fmtMs(result.summary.p50)}  backend=${result.backend}`;
            }).join('\n') +
            `\n\nNotes:\n` +
            selected.map((op) => `  ${op.padEnd(9)} ${perfByOp[op].note}`).join('\n') +
            `\n\nDimension scaling for ${spatialOpLabel(focusOp)}:\n` +
            scalingDims.map((value) =>
                `  ${value.toUpperCase().padEnd(3)} p50=${fmtMs(scalingProfile[value].summary.p50)}  ops/sec=${fmtOps(scalingProfile[value].opsPerSec)}`
            ).join('\n') +
            `\n\nSpatial controls now drive the measured pathway. Interval remains an explicitly labeled fallback until a native WASM export lands.`;
    }

    const viewerShape = shapeForSpatialFocus(focusOp, dims);
    return {
        perfByOp,
        focusOp,
        scalingProfile,
        viewer: {
            shape: viewerShape,
            label: spatialViewerLabel(viewerShape),
            dimension: dims,
            operation: focusOp,
        },
    };
}

const SPARQL_COMPLEXITY_ORDER = ['simple', 'medium', 'complex'];

function defaultSparqlPattern() {
    return '?s <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?o .';
}

function isUnboundedTriplePattern(pattern) {
    const parts = String(pattern || '').replace(/\s*\.\s*$/, '').trim().split(/\s+/);
    return parts.length >= 3 && parts.slice(0, 3).every((part) => part.startsWith('?'));
}

function canonicalExecutionPattern(pattern) {
    const parts = String(pattern || '').replace(/\s*\.\s*$/, '').trim().split(/\s+/);
    if (parts.length < 3) return defaultSparqlPattern();
    const vars = ['?s', '?p', '?o'];
    const canonical = parts.slice(0, 3).map((part, index) => part.startsWith('?') ? vars[index] : part);
    return `${canonical.join(' ')} .`;
}

function extractPrimaryTriplePattern(query) {
    const whereMatch = String(query || '').match(/WHERE\s*\{([\s\S]*?)\}/i);
    const body = whereMatch?.[1] || String(query || '');
    const candidates = body
        .split('\n')
        .map((line) => line.trim())
        .filter((line) => line && !line.startsWith('#') && !/^FILTER\b/i.test(line));
    for (const line of candidates) {
        const cleaned = line.replace(/\s*\.\s*$/, '').trim();
        if (cleaned.split(/\s+/).length >= 3) {
            const pattern = `${cleaned} .`;
            return isUnboundedTriplePattern(pattern) ? defaultSparqlPattern() : canonicalExecutionPattern(pattern);
        }
    }
    return defaultSparqlPattern();
}

function sparqlQueryTypeLabel(type) {
    return ({
        select: 'SELECT',
        construct: 'CONSTRUCT',
        ask: 'ASK',
        describe: 'DESCRIBE',
    })[String(type || '').toLowerCase()] || 'SELECT';
}

function sparqlVariantForComplexity(type, complexity, pattern) {
    if (complexity === 'complex') {
        return `WHERE {
  ${pattern}
  ?subject ?predicate ?object .
  ?subject ?predicate2 ?o2 .
  ?subject ?predicate3 ?o3 .
  ?o2 ?predicate4 ?o3 .
}
LIMIT 96`;
    }
    if (complexity === 'medium') {
        return `WHERE {
  ${pattern}
  ?subject ?predicate ?object .
  ?subject ?predicate2 ?o2 .
}
LIMIT 64`;
    }
    return `WHERE {
  ${pattern}
}
LIMIT 48`;
}

function measureSparqlVariant(wasm, dataset, queryText, executionPattern, runs, maxResults) {
    const latencies = [];
    const throughputs = [];
    const compileLatencies = [];
    const bytecodeLengths = [];
    let lastExecution = null;

    for (let i = 0; i < runs; i++) {
        const compileStart = performance.now();
        let compiled = null;
        if (typeof wasm?.compile_query_to_json === 'function') {
            compiled = wasm.compile_query_to_json(queryText);
        }
        const compileMs = performance.now() - compileStart;
        const executionStart = performance.now();
        lastExecution = runSingleQuery(wasm, executionPattern, dataset.db, maxResults);
        const executionMs = performance.now() - executionStart;
        const totalMs = compileMs + executionMs;

        compileLatencies.push(compileMs);
        latencies.push(totalMs);
        throughputs.push(totalMs > 0 ? ((lastExecution.matches?.length || 0) * 1000) / totalMs : 0);
        bytecodeLengths.push(typeof compiled === 'string' ? compiled.length : 0);
    }

    const decodedMatches = decodeResultMatches(lastExecution?.matches || [], dataset.labelMap)
        .map(normalizeRenderableTriple);
    return {
        latencies,
        throughputs,
        compileLatencies,
        bytecodeLengths,
        summary: stats(latencies),
        compileSummary: stats(compileLatencies),
        bytecodeLength: Math.round(median(bytecodeLengths)),
        resultCount: decodedMatches.length,
        lastSummary: {
            raw: lastExecution,
            decodedMatches,
        },
    };
}

export async function runSparqlLive({
    wasm,
    charts,
    output,
    viewer,
    inspectorTarget,
    query,
    queryType,
    complexity,
    datasetId,
}) {
    if (typeof wasm?.execute_ntriples_query !== 'function') {
        if (output) output.textContent = 'WASM execute_ntriples_query unavailable.';
        return null;
    }

    const selection = datasetId || document.getElementById('graph-dataset')?.value || 'schemaorg-30';
    const profile = await getDatasetProfile(selection);
    if (profile.unavailable) {
        if (output) output.textContent = profile.unavailable;
        return null;
    }

    const pattern = extractPrimaryTriplePattern(query);
    const resultCapacity = Math.min(profile.dataset.quinCount || 0, 20000) || 4096;
    const variantResults = {};
    for (const level of SPARQL_COMPLEXITY_ORDER) {
        const variantQuery = sparqlVariantForComplexity(queryType, level, pattern);
        variantResults[level] = measureSparqlVariant(
            wasm,
            profile.dataset,
            variantQuery,
            pattern,
            level === complexity ? 10 : 4,
            resultCapacity,
        );
    }

    const active = variantResults[complexity] || variantResults.simple;
    const graphTriples = active.lastSummary.decodedMatches;
    const fallbackFocus = graphTriples[0]?.subjectId || graphTriples[0]?.objectId || null;
    const graphModel = buildGraphModel(graphTriples);
    const graphData = fallbackFocus
        ? buildNeighborhood(graphTriples, fallbackFocus, 28, 56)
        : { nodes: [], edges: [] };

    if (viewer) {
        viewer.setData(graphData);
        viewer.setNodeSelectHandler((nodeId) => renderInspector(inspectorTarget, graphModel, nodeId));
    }
    renderInspector(inspectorTarget, graphModel, fallbackFocus || graphData.nodes[0]?.id || null);

    if (charts?.sparqlLatency) {
        charts.sparqlLatency.data.labels = active.latencies.map((_, index) => `run ${index + 1}`);
        charts.sparqlLatency.data.datasets = [{
            label: `${sparqlQueryTypeLabel(queryType)} total time (ms)`,
            data: active.latencies,
            backgroundColor: 'rgba(52, 211, 153, 0.5)',
            borderColor: 'rgba(52, 211, 153, 1)',
            borderWidth: 1,
        }];
        charts.sparqlLatency.update();
    }

    if (charts?.sparqlComplexity) {
        charts.sparqlComplexity.data.labels = SPARQL_COMPLEXITY_ORDER.map((value) => value[0].toUpperCase() + value.slice(1));
        charts.sparqlComplexity.data.datasets = [{
            label: 'Compile + execute p50 (ms)',
            data: SPARQL_COMPLEXITY_ORDER.map((value) => variantResults[value]?.summary?.p50 ?? 0),
            borderColor: 'rgba(59, 130, 246, 1)',
            backgroundColor: 'rgba(59, 130, 246, 0.1)',
            fill: true,
            tension: 0.4,
        }];
        charts.sparqlComplexity.update();
    }

    if (output) {
        output.textContent =
            `SPARQL benchmark (${sparqlQueryTypeLabel(queryType)} | ${complexity})\n` +
            `Dataset: ${profile.config.label} | ${profile.dataset.quinCount.toLocaleString()} quins | loaded via ${profile.storageFormat}\n` +
            `Execution pathway: compile_query_to_json + execute_ntriples_query(first triple pattern)\n` +
            `Primary pattern: ${pattern}\n\n` +
            `Selected run profile:\n` +
            `  total p50=${fmtMs(active.summary.p50)}  compile p50=${fmtMs(active.compileSummary.p50)}  results=${active.resultCount}  bytecode=${active.bytecodeLength} chars\n` +
            `  throughput p50=${fmtOps(percentile(active.throughputs, 0.5))} rows/sec\n\n` +
            `Complexity sweep:\n` +
            SPARQL_COMPLEXITY_ORDER.map((value) =>
                `  ${value.padEnd(7)} p50=${fmtMs(variantResults[value].summary.p50)}  compile=${fmtMs(variantResults[value].compileSummary.p50)}  rows=${variantResults[value].resultCount}`
            ).join('\n') +
            `\n\nGraph panel renders the measured matches from the selected run.`;
    }

    return {
        dataset: profile.dataset,
        profile,
        pattern,
        variantResults,
        active,
        graphTriples,
        graphModel,
    };
}
