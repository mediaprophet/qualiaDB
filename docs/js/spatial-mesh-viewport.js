/**
 * 3D Indexed B-Rep Mesh Generator + WebGL2 (#version 300 es) / Canvas2D Viewport
 * for docs/spatial.html (#render-canvas & #plane2d-canvas).
 */

function normalizeVec3([x, y, z]) {
    const l = Math.hypot(x, y, z) || 1;
    return [x / l, y / l, z / l];
}

function computeVertexNormals(vertices, faces) {
    const norms = vertices.map(() => [0, 0, 0]);
    for (const [a, b, c] of faces) {
        const va = vertices[a], vb = vertices[b], vc = vertices[c];
        const ux = vb[0] - va[0], uy = vb[1] - va[1], uz = vb[2] - va[2];
        const vx = vc[0] - va[0], vy = vc[1] - va[1], vz = vc[2] - va[2];
        const nx = uy * vz - uz * vy;
        const ny = uz * vx - ux * vz;
        const nz = ux * vy - uy * vx;
        for (const idx of [a, b, c]) {
            norms[idx][0] += nx;
            norms[idx][1] += ny;
            norms[idx][2] += nz;
        }
    }
    return norms.map((n, i) => {
        const l = Math.hypot(n[0], n[1], n[2]);
        return l > 1e-6 ? [n[0] / l, n[1] / l, n[2] / l] : normalizeVec3(vertices[i]);
    });
}

function extractUniqueEdges(faces) {
    const set = new Set();
    const edges = [];
    for (const [a, b, c] of faces) {
        for (const [u, v] of [[a, b], [b, c], [c, a]]) {
            const lo = Math.min(u, v), hi = Math.max(u, v);
            const key = `${lo}:${hi}`;
            if (!set.has(key)) {
                set.add(key);
                edges.push([lo, hi]);
            }
        }
    }
    return edges;
}

export function buildIndexedMesh(type = 'icosahedron', scale = 1.4, detail = 2) {
    const d = Math.max(0, Math.min(5, Number(detail) || 0));
    let vertices = [];
    let faces = [];
    let genus = 0;

    if (type === 'cube') {
        const seg = Math.max(1, d + 1);
        const facesDirs = [
            { n: [0, 0, 1], u: [1, 0, 0], v: [0, 1, 0] },
            { n: [0, 0, -1], u: [-1, 0, 0], v: [0, 1, 0] },
            { n: [0, 1, 0], u: [1, 0, 0], v: [0, 0, -1] },
            { n: [0, -1, 0], u: [1, 0, 0], v: [0, 0, 1] },
            { n: [1, 0, 0], u: [0, 0, -1], v: [0, 1, 0] },
            { n: [-1, 0, 0], u: [0, 0, 1], v: [0, 1, 0] },
        ];
        const s = scale * 0.72;
        for (const fd of facesDirs) {
            const base = vertices.length;
            for (let iy = 0; iy <= seg; iy++) {
                for (let ix = 0; ix <= seg; ix++) {
                    const a = (ix / seg) * 2 - 1;
                    const b = (iy / seg) * 2 - 1;
                    vertices.push([
                        (fd.n[0] + a * fd.u[0] + b * fd.v[0]) * s,
                        (fd.n[1] + a * fd.u[1] + b * fd.v[1]) * s,
                        (fd.n[2] + a * fd.u[2] + b * fd.v[2]) * s,
                    ]);
                }
            }
            for (let iy = 0; iy < seg; iy++) {
                for (let ix = 0; ix < seg; ix++) {
                    const i0 = base + iy * (seg + 1) + ix;
                    const i1 = i0 + 1;
                    const i2 = i0 + (seg + 1);
                    const i3 = i2 + 1;
                    faces.push([i0, i1, i3], [i0, i3, i2]);
                }
            }
        }
    } else if (type === 'sphere') {
        const lat = 8 + d * 4;
        const lon = 12 + d * 6;
        for (let i = 0; i <= lat; i++) {
            const theta = (i * Math.PI) / lat;
            const st = Math.sin(theta), ct = Math.cos(theta);
            for (let j = 0; j <= lon; j++) {
                const phi = (j * 2 * Math.PI) / lon;
                vertices.push([scale * st * Math.cos(phi), scale * ct, scale * st * Math.sin(phi)]);
            }
        }
        for (let i = 0; i < lat; i++) {
            for (let j = 0; j < lon; j++) {
                const a = i * (lon + 1) + j;
                const b = a + lon + 1;
                faces.push([a, b, a + 1], [b, b + 1, a + 1]);
            }
        }
    } else if (type === 'torus') {
        genus = 1;
        const majSeg = 16 + d * 6;
        const minSeg = 10 + d * 4;
        const R = scale * 0.78;
        const r = scale * 0.30;
        for (let i = 0; i < majSeg; i++) {
            const u = (i / majSeg) * Math.PI * 2;
            const cu = Math.cos(u), su = Math.sin(u);
            for (let j = 0; j < minSeg; j++) {
                const v = (j / minSeg) * Math.PI * 2;
                const cv = Math.cos(v), sv = Math.sin(v);
                vertices.push([(R + r * cv) * cu, r * sv, (R + r * cv) * su]);
            }
        }
        for (let i = 0; i < majSeg; i++) {
            const ni = (i + 1) % majSeg;
            for (let j = 0; j < minSeg; j++) {
                const nj = (j + 1) % minSeg;
                const a = i * minSeg + j;
                const b = ni * minSeg + j;
                const c = ni * minSeg + nj;
                const e = i * minSeg + nj;
                faces.push([a, b, c], [a, c, e]);
            }
        }
    } else if (type === 'knot') {
        genus = 1;
        const p = 2, q = 3;
        const tubSeg = 36 + d * 14;
        const radSeg = 8 + d * 2;
        const tubeR = scale * 0.16;
        const centerPts = [];
        for (let i = 0; i < tubSeg; i++) {
            const t = (i / tubSeg) * Math.PI * 2;
            const r = scale * 0.52 * (1.4 + 0.55 * Math.cos(q * t));
            centerPts.push([
                r * Math.cos(p * t),
                scale * 0.42 * Math.sin(q * t),
                r * Math.sin(p * t),
            ]);
        }
        for (let i = 0; i < tubSeg; i++) {
            const prev = centerPts[(i - 1 + tubSeg) % tubSeg];
            const curr = centerPts[i];
            const next = centerPts[(i + 1) % tubSeg];
            const T = normalizeVec3([next[0] - prev[0], next[1] - prev[1], next[2] - prev[2]]);
            const up = Math.abs(T[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
            const N = normalizeVec3([
                T[1] * up[2] - T[2] * up[1],
                T[2] * up[0] - T[0] * up[2],
                T[0] * up[1] - T[1] * up[0],
            ]);
            const B = normalizeVec3([
                T[1] * N[2] - T[2] * N[1],
                T[2] * N[0] - T[0] * N[2],
                T[0] * N[1] - T[1] * N[0],
            ]);
            for (let j = 0; j < radSeg; j++) {
                const a = (j / radSeg) * Math.PI * 2;
                const ca = Math.cos(a), sa = Math.sin(a);
                vertices.push([
                    curr[0] + tubeR * (ca * N[0] + sa * B[0]),
                    curr[1] + tubeR * (ca * N[1] + sa * B[1]),
                    curr[2] + tubeR * (ca * N[2] + sa * B[2]),
                ]);
            }
        }
        for (let i = 0; i < tubSeg; i++) {
            const ni = (i + 1) % tubSeg;
            for (let j = 0; j < radSeg; j++) {
                const nj = (j + 1) % radSeg;
                const a = i * radSeg + j;
                const b = ni * radSeg + j;
                const c = ni * radSeg + nj;
                const e = i * radSeg + nj;
                faces.push([a, b, c], [a, c, e]);
            }
        }
    } else {
        // Geodesic subdivided icosahedron
        const t = (1 + Math.sqrt(5)) / 2;
        const baseV = [
            [-1, t, 0], [1, t, 0], [-1, -t, 0], [1, -t, 0],
            [0, -1, t], [0, 1, t], [0, -1, -t], [0, 1, -t],
            [t, 0, -1], [t, 0, 1], [-t, 0, -1], [-t, 0, 1],
        ].map((v) => {
            const n = normalizeVec3(v);
            return [n[0] * scale, n[1] * scale, n[2] * scale];
        });
        let baseF = [
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
            [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
            [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];
        vertices = baseV;
        faces = baseF;
        const subdivs = Math.min(3, d);
        for (let s = 0; s < subdivs; s++) {
            const midCache = new Map();
            const getMid = (i1, i2) => {
                const key = i1 < i2 ? `${i1}:${i2}` : `${i2}:${i1}`;
                if (midCache.has(key)) return midCache.get(key);
                const v1 = vertices[i1], v2 = vertices[i2];
                const m = normalizeVec3([(v1[0] + v2[0]) * 0.5, (v1[1] + v2[1]) * 0.5, (v1[2] + v2[2]) * 0.5]);
                const idx = vertices.length;
                vertices.push([m[0] * scale, m[1] * scale, m[2] * scale]);
                midCache.set(key, idx);
                return idx;
            };
            const nextFaces = [];
            for (const [a, b, c] of faces) {
                const ab = getMid(a, b);
                const bc = getMid(b, c);
                const ca = getMid(c, a);
                nextFaces.push([a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]);
            }
            faces = nextFaces;
        }
    }

    const normals = computeVertexNormals(vertices, faces);
    const edges = extractUniqueEdges(faces);

    // Serialize Wavefront OBJ bytes for WebGPU QualiaPortal.upload_mesh_asset(buf, 'obj')
    const objLines = ['# QualiaDB Procedural B-Rep Asset'];
    for (const [x, y, z] of vertices) objLines.push(`v ${x.toFixed(5)} ${y.toFixed(5)} ${z.toFixed(5)}`);
    for (const [nx, ny, nz] of normals) objLines.push(`vn ${nx.toFixed(5)} ${ny.toFixed(5)} ${nz.toFixed(5)}`);
    for (const [a, b, c] of faces) objLines.push(`f ${a + 1}//${a + 1} ${b + 1}//${b + 1} ${c + 1}//${c + 1}`);
    const objBytes = new TextEncoder().encode(objLines.join('\n'));

    return { type, scale, detail: d, vertices, normals, faces, edges, genus, objBytes };
}

// ─── WebGL2 (#version 300 es) + Canvas2D 3D Mesh Viewport ───────────────────

const VS_300_ES = `#version 300 es
precision highp float;
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_norm;
uniform mat4 u_mvp;
uniform mat3 u_normMat;
uniform float u_pointSize;
out vec3 v_norm;
out vec3 v_pos;
void main() {
    v_norm = normalize(u_normMat * a_norm);
    v_pos = a_pos;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
    gl_PointSize = u_pointSize;
}`;

const FS_300_ES = `#version 300 es
precision highp float;
in vec3 v_norm;
in vec3 v_pos;
uniform int u_pass; // 0 = solid Phong, 1 = wireframe, 2 = points
uniform float u_alpha;
out vec4 outColor;
void main() {
    if (u_pass == 1) {
        outColor = vec4(0.20, 0.88, 0.72, 0.88);
        return;
    }
    if (u_pass == 2) {
        vec2 c = gl_PointCoord - vec2(0.5);
        if (dot(c, c) > 0.25) discard;
        outColor = vec4(0.40, 0.94, 0.99, 0.95);
        return;
    }
    vec3 N = normalize(v_norm);
    vec3 L1 = normalize(vec3(0.65, 0.82, 0.58));
    vec3 L2 = normalize(vec3(-0.55, -0.35, 0.65));
    vec3 V = normalize(vec3(0.0, 0.15, 1.0));
    vec3 H = normalize(L1 + V);
    float diff1 = max(dot(N, L1), 0.0);
    float diff2 = max(dot(N, L2), 0.0) * 0.35;
    float spec = pow(max(dot(N, H), 0.0), 28.0) * 0.45;
    float rim = pow(1.0 - max(dot(N, V), 0.0), 2.4) * 0.38;
    vec3 baseCol = mix(vec3(0.06, 0.42, 0.48), vec3(0.16, 0.78, 0.56), 0.5 + 0.5 * N.y);
    vec3 col = baseCol * (0.24 + diff1 * 0.72 + diff2) + vec3(0.55, 0.96, 0.88) * (spec + rim);
    outColor = vec4(col, u_alpha);
}`;

function mat4Perspective(fovRad, aspect, near, far) {
    const f = 1.0 / Math.tan(fovRad * 0.5);
    const nf = 1.0 / (near - far);
    return new Float32Array([
        f / aspect, 0, 0, 0,
        0, f, 0, 0,
        0, 0, (far + near) * nf, -1,
        0, 0, 2 * far * near * nf, 0,
    ]);
}

function mat4Mul(a, b) {
    const out = new Float32Array(16);
    for (let c = 0; c < 4; c++) {
        for (let r = 0; r < 4; r++) {
            out[c * 4 + r] =
                a[0 * 4 + r] * b[c * 4 + 0] +
                a[1 * 4 + r] * b[c * 4 + 1] +
                a[2 * 4 + r] * b[c * 4 + 2] +
                a[3 * 4 + r] * b[c * 4 + 3];
        }
    }
    return out;
}

export class MeshViewport3D {
    constructor(canvas) {
        this.canvas = canvas;
        this.mesh = null;
        this.mode = 'hybrid';
        this.yaw = 0.55;
        this.pitch = 0.32;
        this.distance = 3.8;
        this.autoRotate = true;
        this.gl = null;
        this.ctx2d = null;
        this.backendTier = 'Canvas2D';
        this._raf = 0;
        this._pointers = new Map();
        this._pinchDist = 0;
        this._initContext();
        this._bindEvents();
    }

    _initContext() {
        if (!this.canvas) return;
        try {
            const gl = this.canvas.getContext('webgl2', {
                alpha: true,
                antialias: true,
                depth: true,
                premultipliedAlpha: false,
            });
            if (gl) {
                const vs = gl.createShader(gl.VERTEX_SHADER);
                gl.shaderSource(vs, VS_300_ES);
                gl.compileShader(vs);
                const fs = gl.createShader(gl.FRAGMENT_SHADER);
                gl.shaderSource(fs, FS_300_ES);
                gl.compileShader(fs);
                const prog = gl.createProgram();
                gl.attachShader(prog, vs);
                gl.attachShader(prog, fs);
                gl.linkProgram(prog);
                if (gl.getProgramParameter(prog, gl.LINK_STATUS)) {
                    this.gl = gl;
                    this.prog = prog;
                    this.uMvp = gl.getUniformLocation(prog, 'u_mvp');
                    this.uNormMat = gl.getUniformLocation(prog, 'u_normMat');
                    this.uPass = gl.getUniformLocation(prog, 'u_pass');
                    this.uAlpha = gl.getUniformLocation(prog, 'u_alpha');
                    this.uPointSize = gl.getUniformLocation(prog, 'u_pointSize');
                    this.vao = gl.createVertexArray();
                    this.vbo = gl.createBuffer();
                    this.iboTris = gl.createBuffer();
                    this.iboLines = gl.createBuffer();
                    this.backendTier = 'WebGL2 (#version 300 es)';
                    return;
                }
            }
        } catch (_) { /* fallback to 2D canvas */ }
        this.ctx2d = this.canvas.getContext('2d');
        this.backendTier = 'Canvas2D Software';
    }

    _bindEvents() {
        const cv = this.canvas;
        if (!cv) return;
        cv.addEventListener('pointerdown', (e) => {
            cv.setPointerCapture?.(e.pointerId);
            this._pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
            if (this._pointers.size === 2) {
                const [p1, p2] = Array.from(this._pointers.values());
                this._pinchDist = Math.hypot(p2.x - p1.x, p2.y - p1.y);
            }
        });
        cv.addEventListener('pointermove', (e) => {
            if (!this._pointers.has(e.pointerId)) return;
            const prev = this._pointers.get(e.pointerId);
            this._pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
            if (this._pointers.size === 1) {
                this.yaw += (e.clientX - prev.x) * 0.0085;
                this.pitch = Math.max(-1.45, Math.min(1.45, this.pitch + (e.clientY - prev.y) * 0.0085));
            } else if (this._pointers.size === 2) {
                const [p1, p2] = Array.from(this._pointers.values());
                const dist = Math.hypot(p2.x - p1.x, p2.y - p1.y);
                if (this._pinchDist > 10) {
                    this.distance = Math.max(1.8, Math.min(8.5, this.distance * (this._pinchDist / Math.max(dist, 10))));
                }
                this._pinchDist = dist;
            }
        });
        const up = (e) => { this._pointers.delete(e.pointerId); };
        cv.addEventListener('pointerup', up);
        cv.addEventListener('pointercancel', up);
        cv.addEventListener('wheel', (e) => {
            e.preventDefault();
            this.distance = Math.max(1.8, Math.min(8.5, this.distance + e.deltaY * 0.003));
        }, { passive: false });
    }

    setMesh(mesh) {
        this.mesh = mesh;
        if (!this.gl || !mesh) return;
        const gl = this.gl;
        const vCount = mesh.vertices.length;
        const interleaved = new Float32Array(vCount * 6);
        for (let i = 0; i < vCount; i++) {
            const v = mesh.vertices[i], n = mesh.normals[i];
            interleaved[i * 6 + 0] = v[0];
            interleaved[i * 6 + 1] = v[1];
            interleaved[i * 6 + 2] = v[2];
            interleaved[i * 6 + 3] = n[0];
            interleaved[i * 6 + 4] = n[1];
            interleaved[i * 6 + 5] = n[2];
        }
        const triIndices = new Uint32Array(mesh.faces.length * 3);
        mesh.faces.forEach(([a, b, c], i) => {
            triIndices[i * 3 + 0] = a;
            triIndices[i * 3 + 1] = b;
            triIndices[i * 3 + 2] = c;
        });
        const lineIndices = new Uint32Array(mesh.edges.length * 2);
        mesh.edges.forEach(([a, b], i) => {
            lineIndices[i * 2 + 0] = a;
            lineIndices[i * 2 + 1] = b;
        });

        gl.bindVertexArray(this.vao);
        gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
        gl.bufferData(gl.ARRAY_BUFFER, interleaved, gl.STATIC_DRAW);
        gl.enableVertexAttribArray(0);
        gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
        gl.enableVertexAttribArray(1);
        gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);

        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iboTris);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, triIndices, gl.STATIC_DRAW);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iboLines);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, lineIndices, gl.STATIC_DRAW);
        gl.bindVertexArray(null);

        this.triIndexCount = triIndices.length;
        this.lineIndexCount = lineIndices.length;
        this.vertCount = vCount;
    }

    resetCamera() {
        this.yaw = 0.55;
        this.pitch = 0.32;
        this.distance = 3.8;
    }

    start() {
        if (this._raf) return;
        const loop = () => {
            if (this.autoRotate && this._pointers.size === 0) {
                this.yaw += 0.0055;
            }
            this.renderFrame();
            this._raf = requestAnimationFrame(loop);
        };
        this._raf = requestAnimationFrame(loop);
    }

    renderFrame() {
        if (!this.canvas || !this.mesh) return;
        const dpr = Math.min(window.devicePixelRatio || 1, 2);
        const w = Math.max(1, Math.floor((this.canvas.clientWidth || 640) * dpr));
        const h = Math.max(1, Math.floor((this.canvas.clientHeight || 420) * dpr));
        if (this.canvas.width !== w || this.canvas.height !== h) {
            this.canvas.width = w;
            this.canvas.height = h;
        }

        const cy = Math.cos(this.yaw), sy = Math.sin(this.yaw);
        const cp = Math.cos(this.pitch), sp = Math.sin(this.pitch);

        // Update companion 2D shadow canvas (#plane2d-canvas)
        const shadowCv = document.getElementById('plane2d-canvas');
        if (shadowCv) {
            const sctx = shadowCv.getContext('2d');
            const sw = shadowCv.width, sh = shadowCv.height;
            sctx.clearRect(0, 0, sw, sh);
            const sc = sw * 0.28;
            sctx.strokeStyle = 'rgba(52, 211, 153, 0.32)';
            sctx.lineWidth = 0.8;
            const step = Math.max(1, Math.floor(this.mesh.edges.length / 180));
            for (let i = 0; i < this.mesh.edges.length; i += step) {
                const [a, b] = this.mesh.edges[i];
                const va = this.mesh.vertices[a], vb = this.mesh.vertices[b];
                const ax = va[0] * cy - va[2] * sy;
                const az = va[0] * sy + va[2] * cy;
                const ay = va[1] * cp - az * sp;
                const bx = vb[0] * cy - vb[2] * sy;
                const bz = vb[0] * sy + vb[2] * cy;
                const by = vb[1] * cp - bz * sp;
                sctx.beginPath();
                sctx.moveTo(sw * 0.5 + ax * sc, sh * 0.5 - ay * sc);
                sctx.lineTo(sw * 0.5 + bx * sc, sh * 0.5 - by * sc);
                sctx.stroke();
            }
        }

        if (this.gl) {
            const gl = this.gl;
            gl.viewport(0, 0, w, h);
            gl.clearColor(0, 0, 0, 0);
            gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
            gl.enable(gl.DEPTH_TEST);
            gl.enable(gl.BLEND);
            gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

            const proj = mat4Perspective(0.78, w / h, 0.1, 50.0);
            // Model-View matrix: RotateY(yaw) then RotateX(pitch) then TranslateZ(-distance)
            const mv = new Float32Array([
                cy, sp * sy, cp * sy, 0,
                0, cp, -sp, 0,
                -sy, sp * cy, cp * cy, 0,
                0, 0, -this.distance, 1,
            ]);
            const mvp = mat4Mul(proj, mv);
            const normMat = new Float32Array([
                cy, sp * sy, cp * sy,
                0, cp, -sp,
                -sy, sp * cy, cp * cy,
            ]);

            gl.useProgram(this.prog);
            gl.uniformMatrix4fv(this.uMvp, false, mvp);
            gl.uniformMatrix3fv(this.uNormMat, false, normMat);
            gl.bindVertexArray(this.vao);

            if (this.mode === 'solid' || this.mode === 'hybrid') {
                gl.uniform1i(this.uPass, 0);
                gl.uniform1f(this.uAlpha, this.mode === 'hybrid' ? 0.86 : 0.98);
                gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iboTris);
                gl.drawElements(gl.TRIANGLES, this.triIndexCount, gl.UNSIGNED_INT, 0);
            }
            if (this.mode === 'wireframe' || this.mode === 'hybrid') {
                gl.uniform1i(this.uPass, 1);
                gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.iboLines);
                gl.drawElements(gl.LINES, this.lineIndexCount, gl.UNSIGNED_INT, 0);
            }
            if (this.mode === 'points' || this.mode === 'hybrid') {
                gl.uniform1i(this.uPass, 2);
                gl.uniform1f(this.uPointSize, (this.mode === 'points' ? 5.5 : 3.5) * dpr);
                gl.drawArrays(gl.POINTS, 0, this.vertCount);
            }
            gl.bindVertexArray(null);
            return;
        }

        // Canvas2D fallback
        const ctx = this.ctx2d;
        if (!ctx) return;
        ctx.clearRect(0, 0, w, h);
        const scale = Math.min(w, h) * (1.15 / this.distance);
        const projPts = this.mesh.vertices.map(([x, y, z]) => {
            const x1 = x * cy - z * sy;
            const z1 = x * sy + z * cy;
            const y2 = y * cp - z1 * sp;
            const z2 = y * sp + z1 * cp;
            const p = this.distance / Math.max(0.5, this.distance + z2 * 0.55);
            return { sx: w * 0.5 + x1 * scale * p, sy: h * 0.5 - y2 * scale * p, z: z2 };
        });

        if (this.mode === 'solid' || this.mode === 'hybrid') {
            const tris = this.mesh.faces.map(([a, b, c]) => ({
                a, b, c, z: (projPts[a].z + projPts[b].z + projPts[c].z) / 3,
            })).sort((u, v) => v.z - u.z);
            for (const f of tris) {
                const pa = projPts[f.a], pb = projPts[f.b], pc = projPts[f.c];
                ctx.beginPath();
                ctx.moveTo(pa.sx, pa.sy); ctx.lineTo(pb.sx, pb.sy); ctx.lineTo(pc.sx, pc.sy);
                ctx.closePath();
                ctx.fillStyle = 'rgba(16, 185, 129, 0.25)';
                ctx.fill();
            }
        }
        if (this.mode === 'wireframe' || this.mode === 'hybrid') {
            ctx.strokeStyle = 'rgba(52, 211, 153, 0.78)';
            ctx.lineWidth = 1.2 * dpr;
            for (const [a, b] of this.mesh.edges) {
                ctx.beginPath();
                ctx.moveTo(projPts[a].sx, projPts[a].sy);
                ctx.lineTo(projPts[b].sx, projPts[b].sy);
                ctx.stroke();
            }
        }
        if (this.mode === 'points' || this.mode === 'hybrid') {
            ctx.fillStyle = '#67e8f9';
            for (const p of projPts) {
                ctx.beginPath(); ctx.arc(p.sx, p.sy, 2.5 * dpr, 0, Math.PI * 2); ctx.fill();
            }
        }
    }
}
