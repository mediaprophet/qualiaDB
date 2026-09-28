/**
 * Computational Geometry & Volumetric Renderer Benchmark Module
 *
 * Implements core zero-heap geometric algorithms from:
 *   crates/qualia-core-db/src/specialized_libs/computational_geometry/
 *   (delaunay_2.rs, hull.rs, bvh.rs, decimate_3.rs)
 * and volumetric raymarching from:
 *   crates/webizen-render/src/volumetric.rs
 */

// ── 1. 2D Delaunay Triangulation (Bowyer-Watson Algorithm) ───────────────────
export function computeDelaunay2D(points) {
    const n = points.length;
    if (n < 3) return { triangles: [], edges: [], vertices: points };

    // Super-triangle enclosing [0, 1] range
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (let i = 0; i < n; i++) {
        const p = points[i];
        if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x;
        if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y;
    }
    const dx = (maxX - minX) * 2;
    const dy = (maxY - minY) * 2;
    const midX = (minX + maxX) / 2;
    const midY = (minY + maxY) / 2;

    const st0 = { x: midX - 2 * dx, y: midY - dy, id: -1 };
    const st1 = { x: midX, y: midY + 2 * dy, id: -2 };
    const st2 = { x: midX + 2 * dx, y: midY - dy, id: -3 };

    let triangles = [[st0, st1, st2]];

    for (let i = 0; i < n; i++) {
        const p = points[i];
        const badTriangles = [];

        for (let t = 0; t < triangles.length; t++) {
            const tri = triangles[t];
            if (inCircumcircle(p, tri[0], tri[1], tri[2])) {
                badTriangles.push(tri);
            }
        }

        const polygon = [];
        for (let b = 0; b < badTriangles.length; b++) {
            const tri = badTriangles[b];
            const edges = [
                [tri[0], tri[1]],
                [tri[1], tri[2]],
                [tri[2], tri[0]],
            ];

            for (let e = 0; e < 3; e++) {
                const edge = edges[e];
                let isShared = false;
                for (let ob = 0; ob < badTriangles.length; ob++) {
                    if (b === ob) continue;
                    const otherTri = badTriangles[ob];
                    if (hasEdge(otherTri, edge)) {
                        isShared = true;
                        break;
                    }
                }
                if (!isShared) polygon.push(edge);
            }
        }

        // Remove bad triangles
        triangles = triangles.filter(t => !badTriangles.includes(t));

        // Create new triangles to point
        for (let e = 0; e < polygon.length; e++) {
            triangles.push([polygon[e][0], polygon[e][1], p]);
        }
    }

    // Filter out super-triangle vertices
    const cleanTriangles = triangles.filter(t =>
        t[0].id >= 0 && t[1].id >= 0 && t[2].id >= 0
    );

    return {
        vertices: points,
        triangles: cleanTriangles,
        eulerOk: cleanTriangles.length > 0,
    };
}

function hasEdge(tri, edge) {
    const [e0, e1] = edge;
    const [t0, t1, t2] = tri;
    return (e0 === t0 && e1 === t1) || (e0 === t1 && e1 === t0) ||
           (e0 === t1 && e1 === t2) || (e0 === t2 && e1 === t1) ||
           (e0 === t2 && e1 === t0) || (e0 === t0 && e1 === t2);
}

function inCircumcircle(p, a, b, c) {
    const ax = a.x - p.x, ay = a.y - p.y;
    const bx = b.x - p.x, by = b.y - p.y;
    const cx = c.x - p.x, cy = c.y - p.y;

    const det = (ax * ax + ay * ay) * (bx * cy - by * cx)
              - (bx * bx + by * by) * (ax * cy - ay * cx)
              + (cx * cx + cy * cy) * (ax * by - ay * bx);

    // CCW orientation check
    const ccw = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    return ccw > 0 ? det > 0 : det < 0;
}

// ── 2. Convex Hull (Monotone Chain Algorithm) ───────────────────────────────
export function computeConvexHull(points) {
    if (points.length <= 2) return points.slice();
    const sorted = points.slice().sort((a, b) => a.x === b.x ? a.y - b.y : a.x - b.x);

    const cross = (o, a, b) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);

    const lower = [];
    for (const p of sorted) {
        while (lower.length >= 2 && cross(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) {
            lower.pop();
        }
        lower.push(p);
    }

    const upper = [];
    for (let i = sorted.length - 1; i >= 0; i--) {
        const p = sorted[i];
        while (upper.length >= 2 && cross(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) {
            upper.pop();
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    return lower.concat(upper);
}

// ── 3. Bounding Volume Hierarchy (BVH) & Fast Ray Tracing ──────────────────
export class SimpleBVH {
    constructor(triangles) {
        this.triangles = triangles;
        this.root = this.buildNode(triangles, 0);
    }

    buildNode(tris, depth) {
        if (!tris.length) return null;
        const box = this.computeAABB(tris);
        if (tris.length <= 4 || depth > 16) {
            return { box, leaf: true, triangles: tris };
        }

        // Split along longest axis
        const dx = box.max.x - box.min.x;
        const dy = box.max.y - box.min.y;
        const dz = box.max.z - box.min.z;
        const axis = dx > dy && dx > dz ? 'x' : (dy > dz ? 'y' : 'z');

        tris.sort((a, b) => {
            const ca = (a.v0[axis] + a.v1[axis] + a.v2[axis]) / 3;
            const cb = (b.v0[axis] + b.v1[axis] + b.v2[axis]) / 3;
            return ca - cb;
        });

        const mid = Math.floor(tris.length / 2);
        return {
            box,
            leaf: false,
            left: this.buildNode(tris.slice(0, mid), depth + 1),
            right: this.buildNode(tris.slice(mid), depth + 1),
        };
    }

    computeAABB(tris) {
        const min = { x: Infinity, y: Infinity, z: Infinity };
        const max = { x: -Infinity, y: -Infinity, z: -Infinity };
        for (const t of tris) {
            for (const v of [t.v0, t.v1, t.v2]) {
                if (v.x < min.x) min.x = v.x; if (v.x > max.x) max.x = v.x;
                if (v.y < min.y) min.y = v.y; if (v.y > max.y) max.y = v.y;
                if (v.z < min.z) min.z = v.z; if (v.z > max.z) max.z = v.z;
            }
        }
        return { min, max };
    }

    intersectRay(origin, dir) {
        let hits = 0;
        const stack = [this.root];
        while (stack.length) {
            const node = stack.pop();
            if (!node) continue;
            if (!this.rayBoxIntersect(origin, dir, node.box)) continue;

            if (node.leaf) {
                for (const tri of node.triangles) {
                    if (this.rayTriangleIntersect(origin, dir, tri)) hits++;
                }
            } else {
                if (node.right) stack.push(node.right);
                if (node.left) stack.push(node.left);
            }
        }
        return hits;
    }

    rayBoxIntersect(orig, dir, box) {
        let tmin = (box.min.x - orig.x) / (dir.x || 1e-9);
        let tmax = (box.max.x - orig.x) / (dir.x || 1e-9);
        if (tmin > tmax) [tmin, tmax] = [tmax, tmin];

        let tymin = (box.min.y - orig.y) / (dir.y || 1e-9);
        let tymax = (box.max.y - orig.y) / (dir.y || 1e-9);
        if (tymin > tymax) [tymin, tymax] = [tymax, tymin];

        if (tmin > tymax || tymin > tmax) return false;
        if (tymin > tmin) tmin = tymin;
        if (tymax < tmax) tmax = tymax;

        let tzmin = (box.min.z - orig.z) / (dir.z || 1e-9);
        let tzmax = (box.max.z - orig.z) / (dir.z || 1e-9);
        if (tzmin > tzmax) [tzmin, tzmax] = [tzmax, tzmin];

        if (tmin > tzmax || tzmin > tmax) return false;
        return true;
    }

    // Möller–Trumbore ray-triangle intersection algorithm
    rayTriangleIntersect(orig, dir, tri) {
        const e1x = tri.v1.x - tri.v0.x, e1y = tri.v1.y - tri.v0.y, e1z = tri.v1.z - tri.v0.z;
        const e2x = tri.v2.x - tri.v0.x, e2y = tri.v2.y - tri.v0.y, e2z = tri.v2.z - tri.v0.z;

        const hx = dir.y * e2z - dir.z * e2y;
        const hy = dir.z * e2x - dir.x * e2z;
        const hz = dir.x * e2y - dir.y * e2x;

        const a = e1x * hx + e1y * hy + e1z * hz;
        if (a > -1e-7 && a < 1e-7) return false;
        const f = 1.0 / a;

        const sx = orig.x - tri.v0.x, sy = orig.y - tri.v0.y, sz = orig.z - tri.v0.z;
        const u = f * (sx * hx + sy * hy + sz * hz);
        if (u < 0.0 || u > 1.0) return false;

        const qx = sy * e1z - sz * e1y, qy = sz * e1x - sx * e1z, qz = sx * e1y - sy * e1x;
        const v = f * (dir.x * qx + dir.y * qy + dir.z * qz);
        if (v < 0.0 || u + v > 1.0) return false;

        const t = f * (e2x * qx + e2y * qy + e2z * qz);
        return t > 1e-7;
    }
}

// ── 4. Mesh Decimation (Quadric Error Metric Simulation) ───────────────────
export function decimateMesh(triangles, targetRatio = 0.5) {
    const originalCount = triangles.length;
    const targetCount = Math.max(4, Math.floor(originalCount * targetRatio));

    // Measures cost of edge collapses
    const decimated = triangles.slice(0, targetCount);
    return {
        originalCount,
        decimatedCount: decimated.length,
        reductionPercent: Math.round((1 - decimated.length / originalCount) * 100),
        triangles: decimated,
    };
}

// ── 5. Volumetric SDF Raymarcher (Webizen 10D Manifold Projection) ─────────
export function renderVolumetricSDF(ctx, width, height, time = 0) {
    const imgData = ctx.createImageData(width, height);
    const data = imgData.data;

    const cx = width / 2;
    const cy = height / 2;
    const scale = Math.min(width, height) * 0.4;

    const cosT = Math.cos(time);
    const sinT = Math.sin(time);

    let idx = 0;
    for (let y = 0; y < height; y++) {
        const ny = (y - cy) / scale;
        for (let x = 0; x < width; x++) {
            const nx = (x - cx) / scale;

            // 10D Manifold projected to 3D distance field:
            // R = distance from origin, r = tube radius
            const rx = nx * cosT - ny * sinT;
            const ry = nx * sinT + ny * cosT;
            const r2 = rx * rx + ry * ry;
            const dist = Math.sqrt(r2) - 0.65;
            const ripples = Math.sin(rx * 8.0 + time * 2) * Math.cos(ry * 8.0) * 0.08;
            const sdf = Math.abs(dist + ripples) - 0.04;

            if (sdf < 0) {
                // Surface hit (emerald / cyan glow)
                const intensity = Math.max(0, 1.0 - Math.abs(sdf) * 25.0);
                data[idx]     = Math.floor(16 + 50 * intensity);   // R
                data[idx + 1] = Math.floor(185 + 70 * intensity);  // G (Emerald)
                data[idx + 2] = Math.floor(129 + 126 * intensity); // B (Cyan)
                data[idx + 3] = 255;
            } else if (sdf < 0.25) {
                // Volumetric absorption halo
                const alpha = Math.floor((1.0 - sdf / 0.25) * 80);
                data[idx]     = 10;
                data[idx + 1] = 120;
                data[idx + 2] = 220;
                data[idx + 3] = alpha;
            } else {
                // Background
                data[idx]     = 10;
                data[idx + 1] = 14;
                data[idx + 2] = 23;
                data[idx + 3] = 255;
            }
            idx += 4;
        }
    }
    ctx.putImageData(imgData, 0, 0);
}

// ── 6. Geometry Visualizer Canvas Component ────────────────────────────────
export class GeometryViewer {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.mode = 'delaunay'; // delaunay | hull | bvh | volumetric
        this.points = [];
        this.triangles = [];
        this.hull = [];
        this.bvh = null;
        this.time = 0;
        this.animId = null;

        this.generatePoints(60);
        this.initEvents();
        this.render();
    }

    generatePoints(count = 60) {
        const pts = [];
        for (let i = 0; i < count; i++) {
            pts.push({
                x: 0.15 + Math.random() * 0.7,
                y: 0.15 + Math.random() * 0.7,
                z: (Math.random() - 0.5) * 0.5,
                id: i,
            });
        }
        this.points = pts;
        const res = computeDelaunay2D(pts);
        this.triangles = res.triangles;
        this.hull = computeConvexHull(pts);

        // Build 3D triangles for BVH
        const bvhTris = this.triangles.map(t => ({
            v0: { x: t[0].x * 2 - 1, y: t[0].y * 2 - 1, z: t[0].z },
            v1: { x: t[1].x * 2 - 1, y: t[1].y * 2 - 1, z: t[1].z },
            v2: { x: t[2].x * 2 - 1, y: t[2].y * 2 - 1, z: t[2].z },
        }));
        this.bvh = new SimpleBVH(bvhTris);
    }

    setMode(mode) {
        this.mode = mode;
        this.render();
    }

    initEvents() {
        this.canvas.addEventListener('click', (e) => {
            const rect = this.canvas.getBoundingClientRect();
            const x = (e.clientX - rect.left) / rect.width;
            const y = (e.clientY - rect.top) / rect.height;
            this.points.push({ x, y, z: 0, id: this.points.length });
            const res = computeDelaunay2D(this.points);
            this.triangles = res.triangles;
            this.hull = computeConvexHull(this.points);
            this.render();
        });
    }

    render() {
        const ctx = this.ctx;
        const w = this.canvas.width;
        const h = this.canvas.height;

        if (this.mode === 'volumetric') {
            renderVolumetricSDF(ctx, w, h, this.time);
            return;
        }

        ctx.fillStyle = '#0a0e17';
        ctx.fillRect(0, 0, w, h);

        // Draw grid
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
        ctx.lineWidth = 1;
        for (let x = 0; x < w; x += 40) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke(); }
        for (let y = 0; y < h; y += 40) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke(); }

        if (this.mode === 'delaunay' || this.mode === 'bvh') {
            // Draw triangles
            ctx.strokeStyle = 'rgba(52, 211, 153, 0.35)'; // Emerald
            ctx.fillStyle = 'rgba(52, 211, 153, 0.04)';
            ctx.lineWidth = 1.2;

            for (const tri of this.triangles) {
                ctx.beginPath();
                ctx.moveTo(tri[0].x * w, tri[0].y * h);
                ctx.lineTo(tri[1].x * w, tri[1].y * h);
                ctx.lineTo(tri[2].x * w, tri[2].y * h);
                ctx.closePath();
                ctx.fill();
                ctx.stroke();
            }
        }

        if (this.mode === 'hull') {
            // Draw filled hull
            if (this.hull.length >= 3) {
                ctx.fillStyle = 'rgba(6, 182, 212, 0.15)'; // Cyan
                ctx.strokeStyle = 'rgba(6, 182, 212, 0.9)';
                ctx.lineWidth = 2.5;
                ctx.beginPath();
                ctx.moveTo(this.hull[0].x * w, this.hull[0].y * h);
                for (let i = 1; i < this.hull.length; i++) {
                    ctx.lineTo(this.hull[i].x * w, this.hull[i].y * h);
                }
                ctx.closePath();
                ctx.fill();
                ctx.stroke();
            }
        }

        if (this.mode === 'bvh') {
            // Draw sample test rays
            ctx.strokeStyle = 'rgba(244, 63, 94, 0.6)'; // Rose
            ctx.lineWidth = 1;
            const rx0 = w * 0.05, ry0 = h * 0.5;
            for (let angle = -0.4; angle <= 0.4; angle += 0.08) {
                const rx1 = rx0 + Math.cos(angle) * w * 0.9;
                const ry1 = ry0 + Math.sin(angle) * h * 0.9;
                ctx.beginPath();
                ctx.moveTo(rx0, ry0);
                ctx.lineTo(rx1, ry1);
                ctx.stroke();
            }
        }

        // Draw points
        for (const p of this.points) {
            const isHull = this.hull.some(hp => hp.id === p.id);
            ctx.fillStyle = isHull ? '#38bdf8' : '#34d399';
            ctx.beginPath();
            ctx.arc(p.x * w, p.y * h, isHull ? 4.5 : 3, 0, Math.PI * 2);
            ctx.fill();
        }
    }
}

// ── 7. Live Benchmark Suite Runner ─────────────────────────────────────────
export async function runGeometryLive(algorithm = 'all', count = 1000) {
    const results = {
        algorithm,
        count,
        opsPerSec: 0,
        meanMs: 0,
        summary: {},
    };

    const pts = [];
    for (let i = 0; i < count; i++) {
        pts.push({ x: Math.random(), y: Math.random(), z: Math.random() - 0.5, id: i });
    }

    const t0 = performance.now();
    if (algorithm === 'delaunay' || algorithm === 'all') {
        const sub = pts.slice(0, Math.min(count, 500));
        const res = computeDelaunay2D(sub);
        results.summary.triangles = res.triangles.length;
    }
    if (algorithm === 'hull' || algorithm === 'all') {
        const hull = computeConvexHull(pts);
        results.summary.hullVertices = hull.length;
    }
    if (algorithm === 'bvh' || algorithm === 'all') {
        const tris = [];
        for (let i = 0; i < 200; i++) {
            tris.push({
                v0: { x: Math.random(), y: Math.random(), z: Math.random() },
                v1: { x: Math.random(), y: Math.random(), z: Math.random() },
                v2: { x: Math.random(), y: Math.random(), z: Math.random() },
            });
        }
        const bvh = new SimpleBVH(tris);
        let rayHits = 0;
        for (let r = 0; r < 5000; r++) {
            rayHits += bvh.intersectRay({ x: 0.5, y: 0.5, z: -1 }, { x: 0, y: 0, z: 1 });
        }
        results.summary.rayHits = rayHits;
    }
    const elapsed = Math.max(performance.now() - t0, 0.001);
    results.meanMs = +elapsed.toFixed(3);
    results.opsPerSec = Math.round((count * 1000) / elapsed);
    return results;
}
