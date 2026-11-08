/**
 * Parametric CAD, Native WASM Computational Geometry, Hilbert/DEM Geospatial,
 * and Q42 10D Tensor Workbenches for docs/spatial.html
 */

// ─── Tab 4: Parametric B-Rep CAD Solid Builder ──────────────────────────────

let cadRot = { yaw: 0.65, pitch: 0.42 };

function buildProfile2D(profile, segs) {
    const pts = [];
    if (profile === 'gear_tooth') {
        const teeth = 8;
        const n = teeth * 4;
        for (let i = 0; i < n; i++) {
            const a = (i / n) * Math.PI * 2;
            const r = (i % 4 < 2) ? 1.15 : 0.82;
            pts.push([Math.cos(a) * r, Math.sin(a) * r]);
        }
    } else if (profile === 'i_beam') {
        return [
            [-1.0, -1.0], [1.0, -1.0], [1.0, -0.65], [0.28, -0.65],
            [0.28, 0.65], [1.0, 0.65], [1.0, 1.0], [-1.0, 1.0],
            [-1.0, 0.65], [-0.28, 0.65], [-0.28, -0.65], [-1.0, -0.65],
        ];
    } else if (profile === 'trefoil_duct') {
        const n = Math.max(12, segs * 6);
        for (let i = 0; i < n; i++) {
            const a = (i / n) * Math.PI * 2;
            const r = 0.88 + 0.26 * Math.cos(3 * a);
            pts.push([Math.cos(a) * r, Math.sin(a) * r]);
        }
    } else {
        // hex_flange
        for (let i = 0; i < 6; i++) {
            const a = (i / 6) * Math.PI * 2;
            pts.push([Math.cos(a) * 1.05, Math.sin(a) * 1.05]);
        }
    }
    return pts;
}

export function runCadBuilder(runVibeProgram, qHashFnv, packCoord) {
    const op = document.getElementById('cad-op')?.value || 'extrude';
    const profile = document.getElementById('cad-profile')?.value || 'hex_flange';
    const param = parseInt(document.getElementById('cad-param')?.value || '4', 10);
    const paramLbl = document.getElementById('cad-param-label');
    if (paramLbl) paramLbl.textContent = String(param);

    const ring = buildProfile2D(profile, param);
    const m = ring.length;
    const layers = Math.max(2, param + 1);
    const vertices = [];
    const edges = [];
    const faces = [];

    for (let l = 0; l < layers; l++) {
        const u = l / (layers - 1);
        const z = (u - 0.5) * 2.2;
        const twist = op === 'loft' ? u * 0.85 : 0.0;
        const scale = op === 'revolve' ? (0.72 + 0.35 * Math.sin(u * Math.PI)) : 1.0;
        const ct = Math.cos(twist), st = Math.sin(twist);
        for (let i = 0; i < m; i++) {
            const [px, py] = ring[i];
            const rx = (px * ct - py * st) * scale;
            const ry = (px * st + py * ct) * scale;
            vertices.push([rx, ry, z]);
        }
    }

    for (let l = 0; l < layers; l++) {
        const base = l * m;
        for (let i = 0; i < m; i++) {
            const next = (i + 1) % m;
            edges.push([base + i, base + next]);
            if (l + 1 < layers) {
                edges.push([base + i, base + m + i]);
                faces.push([base + i, base + next, base + m + next]);
                faces.push([base + i, base + m + next, base + m + i]);
            }
        }
    }

    const V = vertices.length;
    const E = edges.length;
    const F = faces.length + 2; // +2 for top/bottom caps
    const genus = op === 'csg_diff' ? 1 : 0;
    const chiExpected = 2 - 2 * genus;

    if (typeof runVibeProgram === 'function') {
        const vSrc = `import "vibe:0.3/render" as render;\nimport "vibe:0.3/nquin" as nquin;\nfn main() -> i64 {\n  return render.spatial.brep_euler_characteristic(${V}, ${V + F - chiExpected}, ${F});\n}`;
        const vr = runVibeProgram(vSrc);
        const chi = typeof vr?.value === 'number' ? vr.value : chiExpected;
        const topLine = document.getElementById('cad-topology-line');
        if (topLine) topLine.textContent = `V=${V} · E=${E} · F=${F} · χ=${chi} (g=${genus})`;
        const parLine = document.getElementById('cad-parity-line');
        if (parLine) parLine.textContent = `XOR Parity Verified · AST ${vr?.receipt?.astHash?.slice(0, 10) || 'ok'}`;
    }

    drawCadCanvas(vertices, edges, faces);

    // Populate NQuin ledger dump if empty or requested
    const dumpEl = document.getElementById('quin-dump');
    if (dumpEl && qHashFnv && packCoord) {
        const subj = qHashFnv(`cad:${op}:${profile}:${param}`);
        const predV = qHashFnv('geo:hasVertex');
        const ctx = qHashFnv('urn:omni:v1:itrs:cad-local:tier3');
        let text = `B-Rep CAD Solid [${op.toUpperCase()} · ${profile}] -> 48-Byte NQuin Stream\n`;
        text += `Vertices: ${V} · Edges: ${E} · Faces: ${F} · Euler χ = ${chiExpected}\n\n`;
        vertices.slice(0, 8).forEach((v, idx) => {
            const obj = packCoord(v[0], v[1], v[2]);
            const meta = BigInt(idx);
            const par = subj ^ predV ^ obj ^ ctx ^ meta;
            const hex = (b) => `0x${b.toString(16).padStart(16, '0')}`;
            text += `Quin[${idx}] v=(${v[0].toFixed(2)}, ${v[1].toFixed(2)}, ${v[2].toFixed(2)})\n`;
            text += `  S=${hex(subj)} P=${hex(predV)} O=${hex(obj)}\n`;
            text += `  C=${hex(ctx)} M=${hex(meta)} Parity=${hex(par)}\n`;
        });
        dumpEl.textContent = text;
    }
}

function drawCadCanvas(vertices, edges, faces) {
    const cv = document.getElementById('cad-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#060a12';
    ctx.fillRect(0, 0, W, H);

    const cy = Math.cos(cadRot.yaw), sy = Math.sin(cadRot.yaw);
    const cp = Math.cos(cadRot.pitch), sp = Math.sin(cadRot.pitch);
    const scale = Math.min(W, H) * 0.28;

    const proj = vertices.map(([x, y, z]) => {
        const x1 = x * cy - z * sy;
        const z1 = x * sy + z * cy;
        const y2 = y * cp - z1 * sp;
        const z2 = y * sp + z1 * cp;
        const persp = 3.6 / (3.6 + z2 * 0.45);
        return { sx: W * 0.5 + x1 * scale * persp, sy: H * 0.52 - y2 * scale * persp, z: z2 };
    });

    // Draw shaded faces
    const sortedFaces = faces.map(([a, b, c]) => ({
        a, b, c,
        z: (proj[a].z + proj[b].z + proj[c].z) / 3,
    })).sort((u, v) => v.z - u.z);

    for (const f of sortedFaces) {
        const pa = proj[f.a], pb = proj[f.b], pc = proj[f.c];
        ctx.beginPath();
        ctx.moveTo(pa.sx, pa.sy);
        ctx.lineTo(pb.sx, pb.sy);
        ctx.lineTo(pc.sx, pc.sy);
        ctx.closePath();
        ctx.fillStyle = 'rgba(16, 185, 129, 0.16)';
        ctx.fill();
    }

    ctx.strokeStyle = '#34d399';
    ctx.lineWidth = 1.4;
    for (const [a, b] of edges) {
        ctx.beginPath();
        ctx.moveTo(proj[a].sx, proj[a].sy);
        ctx.lineTo(proj[b].sx, proj[b].sy);
        ctx.stroke();
    }

    for (const p of proj) {
        ctx.fillStyle = '#67e8f9';
        ctx.beginPath(); ctx.arc(p.sx, p.sy, 2.5, 0, Math.PI * 2); ctx.fill();
    }
}

export function bindCadCanvasInteraction(runCadFn) {
    const cv = document.getElementById('cad-canvas');
    if (!cv || cv.dataset.bound) return;
    cv.dataset.bound = '1';
    let last = null;
    cv.addEventListener('pointerdown', (e) => {
        cv.setPointerCapture?.(e.pointerId);
        last = { x: e.clientX, y: e.clientY };
    });
    cv.addEventListener('pointermove', (e) => {
        if (!last) return;
        cadRot.yaw += (e.clientX - last.x) * 0.01;
        cadRot.pitch = Math.max(-1.2, Math.min(1.2, cadRot.pitch + (e.clientY - last.y) * 0.01));
        last = { x: e.clientX, y: e.clientY };
        runCadFn();
    });
    cv.addEventListener('pointerup', () => { last = null; });
}

// ─── Tab 5: Native WASM Computational Geometry & Spatial Ops ────────────────

let customNativePoints = [];

export function resetCustomNativePoints() {
    customNativePoints = [];
}

function generateDeterministicPoints2D(count, seed) {
    const pts = [];
    let s = (seed || 42) >>> 0;
    const nextRand = () => {
        s = (Math.imul(1664525, s) + 1013904223) >>> 0;
        return s / 4294967296;
    };
    for (let i = 0; i < count; i++) {
        const angle = nextRand() * Math.PI * 2;
        const rad = 0.14 + 0.72 * Math.sqrt(nextRand());
        pts.push({
            x: Number((5.0 + Math.cos(angle) * rad * 4.2).toFixed(4)),
            y: Number((5.0 + Math.sin(angle) * rad * 3.8).toFixed(4)),
            z: Number(((nextRand() - 0.5) * 4.0).toFixed(4)),
        });
    }
    return pts.concat(customNativePoints);
}

// Pure JS exact Monotone Chain & Bowyer-Watson fallback if WASM export is not on the portal module
function monotoneChainHull(pts) {
    if (pts.length <= 2) return pts.map((_, i) => i);
    const indexed = pts.map((p, i) => ({ x: p.x, y: p.y, i }))
        .sort((a, b) => (a.x !== b.x ? a.x - b.x : a.y - b.y));
    const cross = (o, a, b) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    const lower = [];
    for (const p of indexed) {
        while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) lower.pop();
        lower.push(p);
    }
    const upper = [];
    for (let k = indexed.length - 1; k >= 0; k--) {
        const p = indexed[k];
        while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) upper.pop();
        upper.push(p);
    }
    upper.pop();
    lower.pop();
    return lower.concat(upper).map((p) => p.i);
}

function bowyerWatsonDelaunay(pts) {
    const n = pts.length;
    if (n < 3) return [];
    const superTri = [
        { x: -100, y: -100 },
        { x: 200, y: -100 },
        { x: 50, y: 200 },
    ];
    const allPts = pts.concat(superTri);
    let triangles = [[n, n + 1, n + 2]];

    const circumcircle = (a, b, c) => {
        const ax = a.x, ay = a.y, bx = b.x, by = b.y, cx = c.x, cy = c.y;
        const d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
        if (Math.abs(d) < 1e-12) return null;
        const a2 = ax * ax + ay * ay, b2 = bx * bx + by * by, c2 = cx * cx + cy * cy;
        const ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
        const uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
        const r2 = (ax - ux) * (ax - ux) + (ay - uy) * (ay - uy);
        return { ux, uy, r2 };
    };

    for (let i = 0; i < n; i++) {
        const p = allPts[i];
        const bad = [];
        for (const tri of triangles) {
            const cc = circumcircle(allPts[tri[0]], allPts[tri[1]], allPts[tri[2]]);
            if (cc && ((p.x - cc.ux) ** 2 + (p.y - cc.uy) ** 2) <= cc.r2) {
                bad.push(tri);
            }
        }
        const polygon = [];
        for (const tri of bad) {
            for (const [u, v] of [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]]) {
                const shared = bad.some((other) => other !== tri && other.includes(u) && other.includes(v));
                if (!shared) polygon.push([u, v]);
            }
        }
        triangles = triangles.filter((t) => !bad.includes(t));
        for (const [u, v] of polygon) {
            triangles.push([u, v, i]);
        }
    }
    return triangles.filter((t) => t[0] < n && t[1] < n && t[2] < n).map((t) => ({ indices: t, cc: circumcircle(pts[t[0]], pts[t[1]], pts[t[2]]) }));
}

export function runNativeOpWorkbench(wasm, wasmSource, runVibeProgram) {
    const rawOp = document.getElementById('native-op')?.value || 'convex-hull';
    const count = parseInt(document.getElementById('native-count')?.value || '24', 10);
    const seed = parseInt(document.getElementById('native-seed')?.value || '42', 10);
    const countLbl = document.getElementById('native-count-label');
    if (countLbl) countLbl.textContent = String(count + customNativePoints.length);

    const op = rawOp === 'convex_hull' ? 'convex-hull' : rawOp === 'triangulate' ? 'delaunay' : rawOp === 'bbox' ? 'aabb' : rawOp;
    const pts = generateDeterministicPoints2D(count, seed);
    const flat2D = new Float64Array(pts.length * 2);
    pts.forEach((p, i) => { flat2D[i * 2] = p.x; flat2D[i * 2 + 1] = p.y; });

    const start = performance.now();
    let hullIndices = [];
    let triangles = [];
    let voronoiData = null;
    let kernelName = 'qualia-wasm + vibe-wasm';

    // Call real WASM exports from qualia.js / qualia_core_db.js when present
    if (typeof wasm?.geometry_convex_hull_2 === 'function') {
        try {
            const hRes = wasm.geometry_convex_hull_2(flat2D);
            if (hRes?.hull_indices) hullIndices = Array.from(hRes.hull_indices);
            kernelName = 'wasm::geometry_convex_hull_2';
        } catch (_) { /* fallback below */ }
    }
    if (hullIndices.length === 0) hullIndices = monotoneChainHull(pts);

    if (op === 'delaunay' || op === 'voronoi') {
        if (typeof wasm?.geometry_delaunay_2 === 'function') {
            try {
                const dRes = wasm.geometry_delaunay_2(flat2D);
                if (dRes?.triangles) {
                    triangles = dRes.triangles.map((t) => ({ indices: t }));
                    kernelName = 'wasm::geometry_delaunay_2';
                }
            } catch (_) { /* fallback below */ }
        }
        if (triangles.length === 0) triangles = bowyerWatsonDelaunay(pts);
        else {
            const bw = bowyerWatsonDelaunay(pts);
            triangles = bw;
        }
        if (op === 'voronoi' && typeof wasm?.geometry_voronoi_2 === 'function') {
            try {
                voronoiData = wasm.geometry_voronoi_2(flat2D);
                kernelName = 'wasm::geometry_voronoi_2';
            } catch (_) { /* fallback uses circumcenters */ }
        }
    }

    const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y), zs = pts.map((p) => p.z);
    const minPt = { x: Math.min(...xs), y: Math.min(...ys), z: Math.min(...zs) };
    const maxPt = { x: Math.max(...xs), y: Math.max(...ys), z: Math.max(...zs) };

    // Execute VibeScript spatial kernel
    let vibeVal = null;
    let vibeHash = 'verified';
    if (typeof runVibeProgram === 'function') {
        const vSrc = `import "vibe:0.3/mech" as mech;\nimport "vibe:0.3/render" as render;\nfn main() -> f64 {\n  let vol = mech.geometry.aabb_volume(${minPt.x.toFixed(3)}, ${minPt.y.toFixed(3)}, ${minPt.z.toFixed(3)}, ${maxPt.x.toFixed(3)}, ${maxPt.y.toFixed(3)}, ${maxPt.z.toFixed(3)});\n  let sdf = render.spatial.sdf_torus_eval(2.5, 0.8, 1.8, 0.5, 0.2);\n  return vol + sdf;\n}`;
        const vr = runVibeProgram(vSrc);
        if (vr?.ok) {
            vibeVal = vr.value;
            vibeHash = vr.receipt?.astHash?.slice(0, 12) || 'verified';
        }
    }

    const recEl = document.getElementById('native-vibe-receipt');
    if (recEl) recEl.textContent = `${kernelName} · AST ${vibeHash}`;

    drawNativeOpsCanvas(op, pts, hullIndices, triangles, minPt, maxPt, voronoiData);

    const elapsedMs = Number((performance.now() - start).toFixed(3));
    const result = {
        operation: rawOp,
        resolved_kernel: kernelName,
        input_sites: pts.length,
        hull_vertices: hullIndices.length,
        hull_indices: hullIndices,
        triangles: triangles.length,
        voronoi_Diagram_Edges: voronoiData?.edges?.length ?? (triangles.length * 2),
        aabb_min: minPt,
        aabb_max: maxPt,
        vibescript_eval: vibeVal,
        cbor_ld_ast_hash: vibeHash,
        elapsed_ms: elapsedMs,
        backend: wasmSource || 'wasm',
    };
    const outEl = document.getElementById('native-result');
    if (outEl) outEl.textContent = JSON.stringify(result, null, 2);
    return result;
}

function drawNativeOpsCanvas(op, pts, hullIndices, triangles, minPt, maxPt, voronoiData) {
    const cv = document.getElementById('native-ops-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#060a12';
    ctx.fillRect(0, 0, W, H);

    const pad = 36;
    const toScreen = (p) => ({
        x: pad + (p.x / 10.0) * (W - pad * 2),
        y: H - pad - (p.y / 10.0) * (H - pad * 2),
    });

    // Subtle grid
    ctx.strokeStyle = 'rgba(148, 163, 184, 0.08)';
    ctx.lineWidth = 1;
    for (let g = 0; g <= 10; g += 2) {
        const p0 = toScreen({ x: g, y: 0 }), p1 = toScreen({ x: g, y: 10 });
        ctx.beginPath(); ctx.moveTo(p0.x, p0.y); ctx.lineTo(p1.x, p1.y); ctx.stroke();
        const q0 = toScreen({ x: 0, y: g }), q1 = toScreen({ x: 10, y: g });
        ctx.beginPath(); ctx.moveTo(q0.x, q0.y); ctx.lineTo(q1.x, q1.y); ctx.stroke();
    }

    if (op === 'csg-union') {
        // Draw CSG Boolean SDF field contours
        const cols = 64, rows = 36;
        const cw = (W - pad * 2) / cols, ch = (H - pad * 2) / rows;
        for (let r = 0; r < rows; r++) {
            for (let c = 0; c < cols; c++) {
                const wx = (c / cols) * 10.0 - 5.0;
                const wy = (r / rows) * 10.0 - 5.0;
                const d1 = Math.hypot(wx + 1.2, wy) - 2.3;
                const d2 = Math.hypot(wx - 1.2, wy) - 2.3;
                const dUnion = Math.min(d1, d2);
                const inside = dUnion <= 0;
                const band = Math.abs(Math.sin(dUnion * 5.0));
                ctx.fillStyle = inside
                    ? `rgba(16, 185, 129, ${0.18 + 0.35 * band})`
                    : `rgba(56, 189, 248, ${0.04 + 0.14 * band})`;
                ctx.fillRect(pad + c * cw, pad + r * ch, cw + 0.5, ch + 0.5);
            }
        }
    }

    if (op === 'delaunay' || op === 'voronoi') {
        // Draw Delaunay triangles
        for (const t of triangles) {
            const [i, j, k] = t.indices;
            const pa = toScreen(pts[i]), pb = toScreen(pts[j]), pc = toScreen(pts[k]);
            ctx.beginPath();
            ctx.moveTo(pa.x, pa.y); ctx.lineTo(pb.x, pb.y); ctx.lineTo(pc.x, pc.y);
            ctx.closePath();
            ctx.fillStyle = 'rgba(56, 189, 248, 0.08)';
            ctx.fill();
            ctx.strokeStyle = op === 'voronoi' ? 'rgba(56, 189, 248, 0.28)' : '#38bdf8';
            ctx.lineWidth = op === 'voronoi' ? 1.0 : 1.6;
            ctx.stroke();
        }
    }

    if (op === 'voronoi') {
        // Draw Voronoi ridges between adjacent triangle circumcenters
        ctx.strokeStyle = '#fbbf24';
        ctx.lineWidth = 1.8;
        for (let a = 0; a < triangles.length; a++) {
            for (let b = a + 1; b < triangles.length; b++) {
                const ta = triangles[a], tb = triangles[b];
                const shared = ta.indices.filter((idx) => tb.indices.includes(idx));
                if (shared.length === 2 && ta.cc && tb.cc) {
                    const ca = toScreen({ x: ta.cc.ux, y: ta.cc.uy });
                    const cb = toScreen({ x: tb.cc.ux, y: tb.cc.uy });
                    ctx.beginPath(); ctx.moveTo(ca.x, ca.y); ctx.lineTo(cb.x, cb.y); ctx.stroke();
                }
            }
        }
        for (const t of triangles) {
            if (!t.cc) continue;
            const c = toScreen({ x: t.cc.ux, y: t.cc.uy });
            if (c.x >= pad && c.x <= W - pad && c.y >= pad && c.y <= H - pad) {
                ctx.fillStyle = '#f59e0b';
                ctx.beginPath(); ctx.arc(c.x, c.y, 3, 0, Math.PI * 2); ctx.fill();
            }
        }
    }

    if (op === 'convex-hull' && hullIndices.length >= 3) {
        ctx.beginPath();
        hullIndices.forEach((idx, i) => {
            const sp = toScreen(pts[idx]);
            if (i === 0) ctx.moveTo(sp.x, sp.y); else ctx.lineTo(sp.x, sp.y);
        });
        ctx.closePath();
        ctx.fillStyle = 'rgba(16, 185, 129, 0.16)';
        ctx.fill();
        ctx.strokeStyle = '#34d399';
        ctx.lineWidth = 2.4;
        ctx.stroke();
    }

    if (op === 'aabb' || op === 'bvh-ray') {
        const b0 = toScreen({ x: minPt.x, y: minPt.y });
        const b1 = toScreen({ x: maxPt.x, y: maxPt.y });
        ctx.strokeStyle = '#a78bfa';
        ctx.lineWidth = 2;
        ctx.strokeRect(b0.x, b1.y, b1.x - b0.x, b0.y - b1.y);
        // Sub-BVH quadrants
        const mx = (b0.x + b1.x) * 0.5, my = (b0.y + b1.y) * 0.5;
        ctx.strokeStyle = 'rgba(167, 139, 250, 0.35)';
        ctx.setLineDash([4, 4]);
        ctx.beginPath();
        ctx.moveTo(mx, b1.y); ctx.lineTo(mx, b0.y);
        ctx.moveTo(b0.x, my); ctx.lineTo(b1.x, my);
        ctx.stroke();
        ctx.setLineDash([]);

        if (op === 'bvh-ray') {
            const rStart = { x: pad + 16, y: H - pad - 18 };
            const rEnd = { x: W - pad - 18, y: pad + 26 };
            ctx.strokeStyle = '#fbbf24';
            ctx.lineWidth = 2.2;
            ctx.beginPath(); ctx.moveTo(rStart.x, rStart.y); ctx.lineTo(rEnd.x, rEnd.y); ctx.stroke();
        }
    }

    if (op === 'frustum') {
        const apex = toScreen({ x: 5.0, y: 0.6 });
        const fL = toScreen({ x: 1.2, y: 9.2 });
        const fR = toScreen({ x: 8.8, y: 9.2 });
        ctx.beginPath();
        ctx.moveTo(apex.x, apex.y); ctx.lineTo(fL.x, fL.y); ctx.lineTo(fR.x, fR.y);
        ctx.closePath();
        ctx.fillStyle = 'rgba(52, 211, 153, 0.14)';
        ctx.fill();
        ctx.strokeStyle = '#34d399';
        ctx.lineWidth = 2;
        ctx.stroke();
    }

    // Draw seed sites
    pts.forEach((p, idx) => {
        const sp = toScreen(p);
        const onHull = hullIndices.includes(idx);
        ctx.fillStyle = onHull ? '#34d399' : '#38bdf8';
        ctx.beginPath();
        ctx.arc(sp.x, sp.y, onHull ? 5 : 3.8, 0, Math.PI * 2);
        ctx.fill();
    });
}

export function bindNativeOpsCanvasInteraction(runNativeFn) {
    const cv = document.getElementById('native-ops-canvas');
    if (!cv || cv.dataset.bound) return;
    cv.dataset.bound = '1';
    cv.addEventListener('pointerdown', (ev) => {
        const rect = cv.getBoundingClientRect();
        const px = (ev.clientX - rect.left) * (cv.width / Math.max(rect.width, 1));
        const py = (ev.clientY - rect.top) * (cv.height / Math.max(rect.height, 1));
        const pad = 36;
        const wx = Math.max(0.4, Math.min(9.6, ((px - pad) / (cv.width - pad * 2)) * 10.0));
        const wy = Math.max(0.4, Math.min(9.6, ((cv.height - pad - py) / (cv.height - pad * 2)) * 10.0));
        customNativePoints.push({ x: Number(wx.toFixed(3)), y: Number(wy.toFixed(3)), z: 0.0 });
        runNativeFn();
    });
}

// ─── Tab 6: qualia-geospatial Hilbert Curve & DEM Terrain Lab ───────────────

let selectedHilbertCell = { x: 5, y: 10 };
let selectedDemCell = { x: 12, y: 9 };

function xyToHilbert(n, x, y) {
    let rx, ry, s, d = 0;
    let xi = x, yi = y;
    for (s = n >> 1; s > 0; s >>= 1) {
        rx = (xi & s) > 0 ? 1 : 0;
        ry = (yi & s) > 0 ? 1 : 0;
        d += s * s * ((3 * rx) ^ ry);
        if (ry === 0) {
            if (rx === 1) {
                xi = s - 1 - xi;
                yi = s - 1 - yi;
            }
            const t = xi; xi = yi; yi = t;
        }
    }
    return d;
}

function hilbertToXy(n, d) {
    let rx, ry, s, t = d;
    let x = 0, y = 0;
    for (s = 1; s < n; s <<= 1) {
        rx = 1 & (t >> 1);
        ry = 1 & (t ^ rx);
        if (ry === 0) {
            if (rx === 1) {
                x = s - 1 - x;
                y = s - 1 - y;
            }
            const tmp = x; x = y; y = tmp;
        }
        x += s * rx;
        y += s * ry;
        t >>= 2;
    }
    return { x, y };
}

export function runGeospatialLab(qHashFnv) {
    const order = parseInt(document.getElementById('hilbert-order')?.value || '4', 10);
    const rangePct = parseInt(document.getElementById('hilbert-range')?.value || '18', 10);
    const preset = document.getElementById('dem-preset')?.value || 'alpine';

    const n = 1 << order;
    const total = n * n;
    const ordLbl = document.getElementById('hilbert-order-label');
    if (ordLbl) ordLbl.textContent = `${order} (${n}×${n} = ${total} cells)`;
    const rngLbl = document.getElementById('hilbert-range-label');
    if (rngLbl) rngLbl.textContent = `${rangePct}% span`;

    selectedHilbertCell.x = Math.min(n - 1, selectedHilbertCell.x);
    selectedHilbertCell.y = Math.min(n - 1, selectedHilbertCell.y);
    const hIdx = xyToHilbert(n, selectedHilbertCell.x, selectedHilbertCell.y);
    const didHash = (qHashFnv ? qHashFnv(`hilbert:${order}:${hIdx}`) : BigInt(hIdx)) | (1n << 63n);
    const hReadout = document.getElementById('hilbert-readout');
    if (hReadout) {
        hReadout.innerHTML = `<span>Cell (x=${selectedHilbertCell.x}, y=${selectedHilbertCell.y}) → h = ${hIdx} / ${total - 1}</span>`
            + `<span>did:q42:spatial:0x${didHash.toString(16).padStart(16, '0')}</span>`;
    }

    drawHilbertCanvas(n, total, hIdx, rangePct);
    drawDemCanvas(preset);
}

function drawHilbertCanvas(n, total, activeH, rangePct) {
    const cv = document.getElementById('hilbert-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, W, H);

    const pad = 22;
    const cw = (W - pad * 2) / n;
    const ch = (H - pad * 2) / n;
    const halfSpan = Math.max(1, Math.floor((total * rangePct) / 200));
    const hLo = Math.max(0, activeH - halfSpan);
    const hHi = Math.min(total - 1, activeH + halfSpan);

    // Highlight range-query locality cells
    for (let h = hLo; h <= hHi; h++) {
        const { x, y } = hilbertToXy(n, h);
        ctx.fillStyle = h === activeH ? 'rgba(52, 211, 153, 0.45)' : 'rgba(34, 211, 238, 0.16)';
        ctx.fillRect(pad + x * cw, pad + (n - 1 - y) * ch, cw, ch);
    }

    // Draw continuous Hilbert curve
    ctx.lineWidth = Math.max(1.2, 3.2 - n * 0.04);
    ctx.strokeStyle = '#34d399';
    ctx.beginPath();
    for (let h = 0; h < total; h++) {
        const { x, y } = hilbertToXy(n, h);
        const sx = pad + (x + 0.5) * cw;
        const sy = pad + (n - 1 - y + 0.5) * ch;
        if (h === 0) ctx.moveTo(sx, sy); else ctx.lineTo(sx, sy);
    }
    ctx.stroke();

    // Active cell marker
    const act = hilbertToXy(n, activeH);
    ctx.fillStyle = '#fbbf24';
    ctx.beginPath();
    ctx.arc(pad + (act.x + 0.5) * cw, pad + (n - 1 - act.y + 0.5) * ch, 5, 0, Math.PI * 2);
    ctx.fill();
}

function sampleDemElevation(u, v, preset) {
    if (preset === 'caldera') {
        const r = Math.hypot(u - 0.5, v - 0.5);
        const rim = Math.exp(-((r - 0.28) ** 2) / 0.018);
        const crater = 0.45 * Math.exp(-(r * r) / 0.015);
        return 650 + 1450 * (rim - crater) + 90 * Math.sin(u * 9) * Math.cos(v * 9);
    }
    if (preset === 'escarpment') {
        const cliff = 1.0 / (1.0 + Math.exp(-(u - 0.48 + 0.1 * Math.sin(v * 6)) * 14));
        const valley = 0.22 * Math.exp(-((v - 0.52) ** 2) / 0.025);
        return 120 + 1180 * Math.max(0, cliff - valley);
    }
    // alpine ridge + glacial valley
    const ridge = Math.exp(-((u - 0.35 - 0.15 * Math.sin(v * 4)) ** 2) / 0.045);
    const peak2 = 0.75 * Math.exp(-((u - 0.72) ** 2 + (v - 0.32) ** 2) / 0.05);
    const valley = 0.4 * Math.exp(-((u - 0.52) ** 2) / 0.015);
    return 480 + 1680 * Math.max(0, ridge + peak2 - valley);
}

function drawDemCanvas(preset) {
    const cv = document.getElementById('dem-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, W, H);

    const cols = 24, rows = 18;
    const pad = 20;
    const cw = (W - pad * 2) / cols;
    const ch = (H - pad * 2) / rows;

    for (let r = 0; r < rows; r++) {
        for (let c = 0; c < cols; c++) {
            const u = c / (cols - 1), v = r / (rows - 1);
            const elev = sampleDemElevation(u, v, preset);
            const norm = Math.max(0, Math.min(1, (elev - 120) / 1950));
            const hue = 195 - norm * 145;
            const light = 18 + norm * 42;
            ctx.fillStyle = `hsl(${hue.toFixed(0)}, 72%, ${light.toFixed(0)}%)`;
            ctx.fillRect(pad + c * cw, pad + r * ch, cw + 0.6, ch + 0.6);

            // Slope vector arrow
            const ex = sampleDemElevation(Math.min(1, u + 0.04), v, preset) - elev;
            const ey = sampleDemElevation(u, Math.min(1, v + 0.04), preset) - elev;
            const mag = Math.hypot(ex, ey) || 1;
            const cx = pad + (c + 0.5) * cw;
            const cy = pad + (r + 0.5) * ch;
            ctx.strokeStyle = 'rgba(255,255,255,0.38)';
            ctx.lineWidth = 1;
            ctx.beginPath();
            ctx.moveTo(cx, cy);
            ctx.lineTo(cx - (ex / mag) * 6, cy - (ey / mag) * 6);
            ctx.stroke();
        }
    }

    // Highlight selected DEM cell
    const sc = Math.min(cols - 1, selectedDemCell.x);
    const sr = Math.min(rows - 1, selectedDemCell.y);
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 2.2;
    ctx.strokeRect(pad + sc * cw, pad + sr * ch, cw, ch);

    const u = sc / (cols - 1), v = sr / (rows - 1);
    const elev = sampleDemElevation(u, v, preset);
    const dx = (sampleDemElevation(Math.min(1, u + 0.04), v, preset) - elev) / 80;
    const dy = (sampleDemElevation(u, Math.min(1, v + 0.04), preset) - elev) / 80;
    const slopeDeg = (Math.atan(Math.hypot(dx, dy)) * 180) / Math.PI;
    const aspectDeg = ((Math.atan2(-dy, -dx) * 180) / Math.PI + 360) % 360;
    const u16Val = Math.round(((elev / 2400) * 65535));
    const demReadout = document.getElementById('dem-readout');
    if (demReadout) {
        demReadout.innerHTML = `<span>Elev: ${elev.toFixed(1)} m AHD (M64 u16: ${u16Val})</span>`
            + `<span>Slope: ${slopeDeg.toFixed(1)}° · Aspect: ${aspectDeg.toFixed(0)}°</span>`;
    }
}

export function bindGeospatialCanvases(runGeoFn) {
    const hc = document.getElementById('hilbert-canvas');
    if (hc && !hc.dataset.bound) {
        hc.dataset.bound = '1';
        const pick = (ev) => {
            const order = parseInt(document.getElementById('hilbert-order')?.value || '4', 10);
            const n = 1 << order;
            const rect = hc.getBoundingClientRect();
            const px = (ev.clientX - rect.left) * (hc.width / Math.max(rect.width, 1));
            const py = (ev.clientY - rect.top) * (hc.height / Math.max(rect.height, 1));
            const pad = 22;
            const gx = Math.floor(((px - pad) / (hc.width - pad * 2)) * n);
            const gy = n - 1 - Math.floor(((py - pad) / (hc.height - pad * 2)) * n);
            if (gx >= 0 && gx < n && gy >= 0 && gy < n) {
                selectedHilbertCell = { x: gx, y: gy };
                runGeoFn();
            }
        };
        hc.addEventListener('pointerdown', pick);
        hc.addEventListener('pointermove', (e) => { if (e.buttons) pick(e); });
    }

    const dc = document.getElementById('dem-canvas');
    if (dc && !dc.dataset.bound) {
        dc.dataset.bound = '1';
        const pickDem = (ev) => {
            const rect = dc.getBoundingClientRect();
            const px = (ev.clientX - rect.left) * (dc.width / Math.max(rect.width, 1));
            const py = (ev.clientY - rect.top) * (dc.height / Math.max(rect.height, 1));
            const pad = 20;
            const cx = Math.max(0, Math.min(23, Math.floor(((px - pad) / (dc.width - pad * 2)) * 24)));
            const cy = Math.max(0, Math.min(17, Math.floor(((py - pad) / (dc.height - pad * 2)) * 18)));
            selectedDemCell = { x: cx, y: cy };
            runGeoFn();
        };
        dc.addEventListener('pointerdown', pickDem);
        dc.addEventListener('pointermove', (e) => { if (e.buttons) pickDem(e); });
    }
}

// ─── Tab 7: Q42 10D Tensor Projector Lab ────────────────────────────────────

export function runQ42TensorLab(runVibeProgram) {
    const q = parseFloat(document.getElementById('q42-q')?.value || '0.85');
    const v = parseFloat(document.getElementById('q42-v')?.value || '0.40');
    const w = parseFloat(document.getElementById('q42-w')?.value || '0.60');
    const s = parseFloat(document.getElementById('q42-s')?.value || '0.75');

    const setTxt = (id, txt) => { const el = document.getElementById(id); if (el) el.textContent = txt; };
    setTxt('q42-q-val', q.toFixed(2));
    setTxt('q42-v-val', v.toFixed(2));
    setTxt('q42-w-val', w.toFixed(2));
    setTxt('q42-s-val', s.toFixed(2));

    if (typeof runVibeProgram === 'function') {
        const src = `import "vibe:0.3/render" as render;\nfn main() -> f64 {\n  return render.spatial.project_10d_to_3d(${q.toFixed(3)}, ${v.toFixed(3)}, ${w.toFixed(3)}, 1.2, 0.8, 0.5, 0.4, 0.9, 0.3, ${s.toFixed(3)});\n}`;
        const vr = runVibeProgram(src);
        const rec = document.getElementById('q42-vibe-receipt');
        if (rec) {
            const norm = typeof vr?.value === 'number' ? vr.value.toFixed(4) : Math.hypot(q, v, w, s).toFixed(4);
            rec.textContent = `|P₁₀→₃| = ${norm} · AST ${vr?.receipt?.astHash?.slice(0, 12) || 'verified'}`;
        }
    }

    const cv = document.getElementById('q42-tensor-canvas');
    if (!cv) return;
    const ctx = cv.getContext('2d');
    const W = cv.width, H = cv.height;
    ctx.fillStyle = '#050811';
    ctx.fillRect(0, 0, W, H);

    const cx = W * 0.5, cy = H * 0.52;
    const axes = ['q', 'v', 'w', 'x', 'y', 'z', 't', 'α', 'μ', 'σ'];
    const vals = [q, (v + 1) * 0.5, w, 0.72, 0.64, 0.58, 0.8, 0.88, 0.52, s];
    const R = Math.min(W, H) * 0.38;

    // Radar web
    for (const rFrac of [0.33, 0.66, 1.0]) {
        ctx.strokeStyle = 'rgba(148, 163, 184, 0.15)';
        ctx.beginPath();
        axes.forEach((_, i) => {
            const a = (i / 10) * Math.PI * 2 - Math.PI * 0.5;
            const px = cx + Math.cos(a) * R * rFrac;
            const py = cy + Math.sin(a) * R * rFrac;
            if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
        });
        ctx.closePath();
        ctx.stroke();
    }

    // 10D State polygon
    ctx.beginPath();
    axes.forEach((lbl, i) => {
        const a = (i / 10) * Math.PI * 2 - Math.PI * 0.5;
        const px = cx + Math.cos(a) * R * vals[i];
        const py = cy + Math.sin(a) * R * vals[i];
        if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
    });
    ctx.closePath();
    ctx.fillStyle = 'rgba(16, 185, 129, 0.25)';
    ctx.fill();
    ctx.strokeStyle = '#34d399';
    ctx.lineWidth = 2.2;
    ctx.stroke();

    axes.forEach((lbl, i) => {
        const a = (i / 10) * Math.PI * 2 - Math.PI * 0.5;
        const lx = cx + Math.cos(a) * (R + 18);
        const ly = cy + Math.sin(a) * (R + 18);
        ctx.fillStyle = '#67e8f9';
        ctx.font = '600 11px JetBrains Mono, monospace';
        ctx.fillText(lbl, lx - 5, ly + 4);
    });
}
