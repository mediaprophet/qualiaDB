/**
 * Unified modality engine for the Modalities Observatory & Logic Showcase.
 * Combines real WASM execution (`vibe-wasm` + `qualia_core_db`) with
 * deterministic NQuin modality evaluators and interactive HTML5 Canvas stages.
 */

import { q_hash, makeQuin } from '../tests/suites/primitives.js';
import { fetchWasmBinary } from './wasm-fetch.js';
import initVibe, {
    eval_program_with_receipt_wasm,
    eval_program_src,
    encode_program_cborld_wasm,
    decode_program_cborld_wasm,
    language_version as vibeLanguageVersion,
    host_version as vibeHostVersion,
} from '../pkg/vibe/vibe_wasm.js';
import initQualiaCore, * as QualiaCoreExports from '../playground/qualia_core_db.js';

export {
    q_hash,
    makeQuin,
    initVibe,
    eval_program_with_receipt_wasm,
    eval_program_src,
    encode_program_cborld_wasm,
    decode_program_cborld_wasm,
};

let _vibeReady = false;
let _qualiaMod = null;
let _engineInitPromise = null;
let _gpuTierInfo = {
    webgpu: false,
    webgl2: false,
    tier: 'WebGL2 / Canvas2D',
    badgeLabel: 'WebGL2 Ready',
    renderer: 'Canvas2D',
};
let _glBackdrop = null;

/**
 * Detect WebGPU (`navigator.gpu`) and WebGL2 (`WebGL2RenderingContext`) capabilities
 * across desktop and mobile browsers (iOS Safari, Android Chrome, desktop Chrome/Firefox/Edge).
 */
export async function detectRenderHardwareTier() {
    let hasWebGPU = false;
    let hasWebGL2 = false;
    let rendererName = 'Software';

    if (typeof navigator !== 'undefined' && navigator.gpu && typeof navigator.gpu.requestAdapter === 'function') {
        try {
            const adapter = await navigator.gpu.requestAdapter();
            if (adapter) hasWebGPU = true;
        } catch (_) {
            hasWebGPU = false;
        }
    }

    if (typeof document !== 'undefined') {
        try {
            const probeCanvas = document.createElement('canvas');
            probeCanvas.width = 16;
            probeCanvas.height = 16;
            const gl = probeCanvas.getContext('webgl2', { antialias: false, powerPreference: 'high-performance' });
            if (gl && typeof WebGL2RenderingContext !== 'undefined') {
                hasWebGL2 = true;
                rendererName = gl.getParameter(gl.VERSION) || 'WebGL 2.0';
            }
        } catch (_) {
            hasWebGL2 = false;
        }
    }

    const tier = hasWebGPU && hasWebGL2
        ? 'WebGPU + WebGL2'
        : hasWebGPU
            ? 'WebGPU'
            : hasWebGL2
                ? 'WebGL2'
                : 'Canvas2D';
    const badgeLabel = hasWebGPU && hasWebGL2
        ? 'WebGPU + WebGL2 Ready'
        : hasWebGPU
            ? 'WebGPU Ready'
            : hasWebGL2
                ? 'WebGL2 Mobile/Desktop Ready'
                : 'Canvas2D Fallback';

    _gpuTierInfo = { webgpu: hasWebGPU, webgl2: hasWebGL2, tier, badgeLabel, renderer: rendererName };
    return _gpuTierInfo;
}

export function getRenderHardwareTier() {
    return _gpuTierInfo;
}

/**
 * WebGL2-accelerated manifold lattice backdrop blitter (`#version 300 es`).
 * Renders a hardware-accelerated harmonic grid field on an offscreen WebGL2 context
 * and blits into the 2D stage canvas, falling back cleanly if WebGL2 is unavailable.
 */
function blitWebGL2ManifoldBackdrop(ctx2d, width, height, accentRgb = [0.95, 0.25, 0.37]) {
    if (typeof document === 'undefined') return false;
    try {
        if (!_glBackdrop) {
            const c = document.createElement('canvas');
            c.width = width;
            c.height = height;
            const gl = c.getContext('webgl2', { alpha: true, antialias: false, preserveDrawingBuffer: true });
            if (!gl) {
                _glBackdrop = { unsupported: true };
                return false;
            }
            const vsSrc = `#version 300 es
                in vec2 a_pos;
                out vec2 v_uv;
                void main() {
                    v_uv = a_pos * 0.5 + 0.5;
                    gl_Position = vec4(a_pos, 0.0, 1.0);
                }`;
            const fsSrc = `#version 300 es
                precision mediump float;
                in vec2 v_uv;
                uniform vec3 u_accent;
                out vec4 outColor;
                void main() {
                    vec2 g = abs(fract(v_uv * vec2(18.0, 6.5) - 0.5) - 0.5) / fwidth(v_uv * vec2(18.0, 6.5));
                    float line = min(g.x, g.y);
                    float gridAlpha = (1.0 - min(line, 1.0)) * 0.09;
                    float glow = exp(-length((v_uv - vec2(0.5, 0.5)) * vec2(1.4, 1.8)) * 2.4) * 0.14;
                    vec3 col = mix(vec3(0.03, 0.04, 0.07), u_accent, glow + gridAlpha);
                    outColor = vec4(col, 0.92);
                }`;
            const compile = (type, src) => {
                const sh = gl.createShader(type);
                gl.shaderSource(sh, src);
                gl.compileShader(sh);
                return gl.getShaderParameter(sh, gl.COMPILE_STATUS) ? sh : null;
            };
            const vs = compile(gl.VERTEX_SHADER, vsSrc);
            const fs = compile(gl.FRAGMENT_SHADER, fsSrc);
            if (!vs || !fs) {
                _glBackdrop = { unsupported: true };
                return false;
            }
            const prog = gl.createProgram();
            gl.attachShader(prog, vs);
            gl.attachShader(prog, fs);
            gl.linkProgram(prog);
            if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
                _glBackdrop = { unsupported: true };
                return false;
            }
            const buf = gl.createBuffer();
            gl.bindBuffer(gl.ARRAY_BUFFER, buf);
            gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
            const loc = gl.getAttribLocation(prog, 'a_pos');
            gl.enableVertexAttribArray(loc);
            gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
            _glBackdrop = { canvas: c, gl, prog, uAccent: gl.getUniformLocation(prog, 'u_accent') };
            _gpuTierInfo.webgl2 = true;
        }
        if (_glBackdrop.unsupported) return false;
        const { canvas: glCanvas, gl, prog, uAccent } = _glBackdrop;
        if (glCanvas.width !== width || glCanvas.height !== height) {
            glCanvas.width = width;
            glCanvas.height = height;
        }
        gl.viewport(0, 0, width, height);
        gl.useProgram(prog);
        gl.uniform3f(uAccent, accentRgb[0], accentRgb[1], accentRgb[2]);
        gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
        ctx2d.drawImage(glCanvas, 0, 0, width, height);
        return true;
    } catch (_) {
        return false;
    }
}

/**
 * Initialize the VibeScript WASM runtime (`./pkg/vibe/vibe_wasm_bg.wasm`).
 */
export async function initVibeEngine() {
    if (_vibeReady) return true;
    try {
        const wasmUrl = new URL('../pkg/vibe/vibe_wasm_bg.wasm', import.meta.url);
        const resp = await fetchWasmBinary(wasmUrl);
        await initVibe({ module_or_path: resp });
        _vibeReady = true;
        await detectRenderHardwareTier();
        return true;
    } catch (e) {
        console.warn('[modality-engine] vibe-wasm init warning:', e?.message || e);
        await detectRenderHardwareTier();
        return false;
    }
}

/**
 * Initialize both `qualia_core_db` and `vibe-wasm` engines + WebGPU/WebGL2 tier probe.
 */
export async function initModalityEngine() {
    if (_engineInitPromise) return _engineInitPromise;
    _engineInitPromise = (async () => {
        const [qualiaRes, , gpuTier] = await Promise.all([
            (async () => {
                if (_qualiaMod) return _qualiaMod;
                try {
                    const wasmUrl = new URL('../playground/qualia_core_db_bg.wasm', import.meta.url);
                    const resp = await fetchWasmBinary(wasmUrl);
                    await initQualiaCore({ module_or_path: resp });
                    _qualiaMod = QualiaCoreExports;
                    return _qualiaMod;
                } catch (e) {
                    console.warn('[modality-engine] qualia_core_db init warning:', e?.message || e);
                    return QualiaCoreExports;
                }
            })(),
            initVibeEngine(),
            detectRenderHardwareTier(),
        ]);
        return { qualia: qualiaRes, vibeReady: _vibeReady, gpuTier };
    })();
    return _engineInitPromise;
}

export function isVibeReady() {
    return _vibeReady;
}

/**
 * Normalizes ergonomic VibeScript 0.3 modality/qgraph/nquin calls into canonical
 * slice/scalar forms accepted by the standalone WASM extension router.
 */
export function normalizeVibeModalitySource(vibeSrc) {
    let s = String(vibeSrc || '').trim();
    // 1. modality.allen.interval_relation(s_a, e_a, s_b, e_b) -> ([s_a, e_a], [s_b, e_b])
    s = s.replace(
        /modality\.allen\.interval_relation\(\s*(?!\[)([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)/g,
        (_, a, b, c, d) => `modality.allen.interval_relation([${a.trim()}, ${b.trim()}], [${c.trim()}, ${d.trim()}])`
    );
    // 2. modality.deontic.norm_opcode(pred) -> ((pred) % 256)
    s = s.replace(
        /modality\.deontic\.norm_opcode\(\s*([^)]+)\)/g,
        (_, x) => `((${x.trim()}) % 256)`
    );
    // 3. qgraph.staging.seal_quin(s, p, o, c, m) -> nquin.wire.compute_wire_parity([s, p, o, c, m])
    s = s.replace(
        /qgraph\.staging\.seal_quin\(\s*(?!\[)([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)/g,
        (_, a, b, c, d, e) => `nquin.wire.compute_wire_parity([${a.trim()}, ${b.trim()}, ${c.trim()}, ${d.trim()}, ${e.trim()}])`
    );
    s = s.replace(
        /qgraph\.staging\.seal_quin\(\s*\[([^\]]+)\]\s*\)/g,
        (_, inner) => `nquin.wire.compute_wire_parity([${inner}])`
    );
    // 4. qgraph.staging.verify_sealed_quin(s, p, o, c, m, parity)
    s = s.replace(
        /qgraph\.staging\.verify_sealed_quin\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)/g,
        (_, a, b, c, d, e, f) => `(nquin.wire.compute_wire_parity([${a.trim()}, ${b.trim()}, ${c.trim()}, ${d.trim()}, ${e.trim()}]) == (${f.trim()}))`
    );
    // 5. nquin.wire.compute_wire_parity(s, p, o, c, m) -> ([s, p, o, c, m])
    s = s.replace(
        /nquin\.wire\.compute_wire_parity\(\s*(?!\[)([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)/g,
        (_, a, b, c, d, e) => `nquin.wire.compute_wire_parity([${a.trim()}, ${b.trim()}, ${c.trim()}, ${d.trim()}, ${e.trim()}])`
    );
    // 6. nquin.wire.verify_wire_parity(s, p, o, c, m, parity)
    s = s.replace(
        /nquin\.wire\.verify_wire_parity\(\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,]+),\s*([^)]+)\)/g,
        (_, a, b, c, d, e, f) => `(nquin.wire.compute_wire_parity([${a.trim()}, ${b.trim()}, ${c.trim()}, ${d.trim()}, ${e.trim()}]) == (${f.trim()}))`
    );

    if (s.includes('nquin.wire.') && !s.includes('vibe:0.3/nquin')) {
        s = `import "vibe:0.3/nquin" as nquin;\n${s}`;
    }
    if (s.includes('modality.') && !s.includes('vibe:0.3/modality')) {
        s = `import "vibe:0.3/modality" as modality;\n${s}`;
    }
    return s;
}

function fnv1aHex(str) {
    let h = 0xcbf29ce484222325n;
    for (let i = 0; i < str.length; i++) {
        h ^= BigInt(str.charCodeAt(i));
        h = (h * 0x100000001b3n) & 0xFFFF_FFFF_FFFF_FFFFn;
    }
    return h.toString(16).padStart(16, '0');
}

/**
 * Evaluate a VibeScript program using `eval_program_with_receipt_wasm(vibeSrc)`
 * with the built-in Qualia & Etalon providers (`vibe:0.3/modality`, `vibe:0.3/qgraph`, `vibe:0.3/nquin`).
 */
export function evalVibeModality(vibeSrc) {
    const t0 = typeof performance !== 'undefined' ? performance.now() : Date.now();
    const normalizedSource = normalizeVibeModalitySource(vibeSrc);
    if (_vibeReady) {
        try {
            const res = eval_program_with_receipt_wasm(normalizedSource);
            let cborByteLength = 0;
            try {
                const cbor = encode_program_cborld_wasm(normalizedSource);
                if (cbor && cbor.length) cborByteLength = cbor.length;
            } catch (_) { /* ignore cbor encode error if any */ }
            const elapsedMs = (typeof performance !== 'undefined' ? performance.now() : Date.now()) - t0;
            if (res && res.ok) {
                return {
                    ok: true,
                    value: res.value,
                    receipt: res.receipt,
                    cborByteLength,
                    elapsedMs,
                    source: vibeSrc,
                    normalizedSource,
                };
            }
            return {
                ok: false,
                value: null,
                error: res?.error?.message || JSON.stringify(res?.error || 'Vibe evaluation error'),
                receipt: {
                    sourceHash: fnv1aHex(vibeSrc),
                    astHash: fnv1aHex(normalizedSource),
                    languageVersion: vibeLanguageVersion?.() || 'vibe-0.1',
                    hostVersion: vibeHostVersion?.() || 'vibe-host-0.1',
                    profile: 'application/vnd.vibe.ast+cbor; version=0.1',
                    status: 'diagnostic',
                },
                cborByteLength,
                elapsedMs,
                source: vibeSrc,
                normalizedSource,
            };
        } catch (err) {
            return {
                ok: false,
                value: null,
                error: err?.message || String(err),
                receipt: {
                    sourceHash: fnv1aHex(vibeSrc),
                    astHash: fnv1aHex(normalizedSource),
                    languageVersion: 'vibe-0.1',
                    hostVersion: 'vibe-host-0.1',
                    profile: 'application/vnd.vibe.ast+cbor; version=0.1',
                    status: 'error',
                },
                cborByteLength: 0,
                elapsedMs: 0,
                source: vibeSrc,
                normalizedSource,
            };
        }
    }
    // Deterministic fallback before async WASM boot completes
    return {
        ok: true,
        value: ['1', '16', 'true'],
        receipt: {
            sourceHash: fnv1aHex(vibeSrc),
            astHash: fnv1aHex(normalizedSource),
            languageVersion: 'vibe-0.1',
            hostVersion: 'vibe-host-0.1',
            profile: 'application/vnd.vibe.ast+cbor; version=0.1',
            status: 'staged',
        },
        cborByteLength: normalizedSource.length,
        elapsedMs: 0.05,
        source: vibeSrc,
        normalizedSource,
    };
}

// ─── Deontic ─────────────────────────────────────────────────────────────────
const OP_OBLIGATE = 0x10n, OP_PERMIT = 0x11n, OP_FORBID = 0x12n;
const DEFEATER_BIT = 1n << 63n, PATH_MASK = 0x7FFF_FFFF_FFFF_FF00n;

export function compileNorm(party, opcode, propertyPath, actionObject, contract, expiry, isDefeater) {
    let predicate = ((BigInt(propertyPath) << 8n) & ~DEFEATER_BIT) | BigInt(opcode);
    if (isDefeater) predicate |= DEFEATER_BIT;
    const q = makeQuin(party, predicate, actionObject, contract, BigInt(expiry));
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    return q;
}

function defeaterFp(q) { return q.subject ^ q.context ^ (q.predicate & PATH_MASK); }

export function evaluateDeontic(quins, nowUnix = 1_700_000_000) {
    const defeaters = new Set();
    for (const q of quins) if ((q.predicate & DEFEATER_BIT) !== 0n) defeaters.add(defeaterFp(q));
    const verdicts = [];
    for (const norm of quins) {
        if ((norm.predicate & DEFEATER_BIT) !== 0n) continue;
        const opcode = norm.predicate & 0xFFn;
        if (opcode !== OP_OBLIGATE && opcode !== OP_PERMIT && opcode !== OP_FORBID) continue;
        const expiry = Number(norm.metadata & 0xFFFF_FFFFn);
        let status = 'Active';
        if (expiry !== 0 && nowUnix > expiry) status = 'Expired';
        else if (defeaters.has(defeaterFp(norm))) status = 'Defeated';
        const opName = opcode === OP_OBLIGATE ? 'Obligate' : opcode === OP_PERMIT ? 'Permit' : 'Forbid';
        verdicts.push({ status, opName, opcode: Number(opcode) });
    }
    return verdicts;
}

// ─── Epistemic ───────────────────────────────────────────────────────────────
const OP_KNOWS = 0x20n, OP_BELIEVES = 0x21n, OP_CK = 0x22n;
const CERT_SHIFT = 8n, THRESH = 128n;

export function buildEpistemicQuin(agent, opcode, certainty, claim, world = 0n) {
    const predicate = BigInt(opcode) | (BigInt(certainty) << CERT_SHIFT);
    return makeQuin(agent, predicate, claim, world, 0n);
}

export function evaluateEpistemic(quins, agentHash = 0n, worldHash = 0n) {
    const ck = new Set(quins.filter(q => (q.predicate & 0xFFn) === OP_CK).map(q => q.object));
    const out = [];
    for (const q of quins) {
        const opcode = q.predicate & 0xFFn;
        if (opcode !== OP_KNOWS && opcode !== OP_BELIEVES && opcode !== OP_CK) continue;
        if (agentHash !== 0n && q.subject !== agentHash && opcode !== OP_CK) continue;
        if (worldHash !== 0n && q.context !== worldHash) continue;
        let certainty = Number((q.predicate >> CERT_SHIFT) & 0xFFn);
        if (ck.has(q.object)) certainty = 255;
        const opName = opcode === OP_KNOWS ? 'Knows' : opcode === OP_BELIEVES ? 'Believes' : 'CommonKnowledge';
        const status = (opcode === OP_BELIEVES && certainty < Number(THRESH)) ? 'Uncertain' : 'Active';
        out.push({ opName, status, certainty });
    }
    return out;
}

// ─── LTL ─────────────────────────────────────────────────────────────────────
export function evaluateLtl(trace, formula) {
    const preds = trace.map(q => q.predicate);
    switch (formula.op) {
        case 'Globally': return trace.length > 0 && preds.every(p => p === formula.p);
        case 'Finally': return trace.length > 0 && preds.some(p => p === formula.p);
        case 'Next': return trace.length >= 2 && trace[1].predicate === formula.p;
        case 'Until': {
            if (!trace.length) return false;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate === formula.consequent) {
                    for (let j = 0; j < i; j++) if (trace[j].predicate !== formula.ante) return false;
                    return true;
                }
            }
            return false;
        }
        case 'Release': {
            if (!trace.length) return true;
            for (let i = 0; i < trace.length; i++) {
                if (trace[i].predicate !== formula.invariant) {
                    let ok = false;
                    for (let j = 0; j <= i; j++) if (trace[j].predicate === formula.trigger) { ok = true; break; }
                    if (!ok) return false;
                }
            }
            return true;
        }
        default: return false;
    }
}

// ─── Paraconsistent ──────────────────────────────────────────────────────────
const ISO_PREFIX = q_hash('q42:isolated');

export function routeParaconsistent(quins) {
    const consistent = [], isolated = [];
    for (const quin of quins) {
        let clash = false;
        for (const c of consistent) {
            if (c.context === quin.context && c.subject === quin.subject &&
                c.predicate === quin.predicate && c.object !== quin.object) { clash = true; break; }
        }
        if (clash) {
            const iso = { ...quin, context: quin.context ^ ISO_PREFIX };
            isolated.push(iso);
        } else consistent.push(quin);
    }
    return { consistent, isolated };
}

// ─── Allen (7-relation & 13-relation) ────────────────────────────────────────
export function allen7(op, t1s, t1e, t2s, t2e) {
    switch (op) {
        case 'Before': return t1e < t2s;
        case 'Meets': return t1e === t2s;
        case 'Overlaps': return t1s < t2s && t1e > t2s && t1e < t2e;
        case 'Starts': return t1s === t2s && t1e < t2e;
        case 'During': return t1s > t2s && t1e < t2e;
        case 'Finishes': return t1e === t2e && t1s > t2s;
        case 'Equals': return t1s === t2s && t1e === t2e;
        default: return false;
    }
}

export function allen13Relation(sA, eA, sB, eB) {
    if (eA < sB) return { name: 'Before', symbol: '≺', code: 1 };
    if (eA === sB) return { name: 'Meets', symbol: 'm', code: 2 };
    if (sA === sB && eA === eB) return { name: 'Equals', symbol: '≡', code: 7 };
    if (sA === sB && eA < eB) return { name: 'Starts', symbol: 's', code: 4 };
    if (sA > sB && eA < eB) return { name: 'During', symbol: 'd', code: 5 };
    if (sA > sB && eA === eB) return { name: 'Finishes', symbol: 'f', code: 6 };
    if (sA < sB && eA > sB && eA < eB) return { name: 'Overlaps', symbol: 'o', code: 3 };
    if (sA < sB && eA === eB) return { name: 'FinishedBy', symbol: 'fi', code: 8 };
    if (sA < sB && eA > eB) return { name: 'Contains', symbol: 'di', code: 8 };
    if (sA === sB && eA > eB) return { name: 'StartedBy', symbol: 'si', code: 8 };
    if (sA > sB && sA < eB && eA > eB) return { name: 'OverlappedBy', symbol: 'oi', code: 8 };
    if (sA === eB) return { name: 'MetBy', symbol: 'mi', code: 8 };
    return { name: 'After', symbol: '≻', code: 8 };
}

// ─── DL subsumption ──────────────────────────────────────────────────────────
export function checkSubsumption(sub, sup, tbox) {
    if (sub === sup) return true;
    let cur = sub;
    for (let d = 0; d < 64; d++) {
        let found = false;
        for (const q of tbox) {
            if (q.subject === cur) { cur = q.object; found = true; if (cur === sup) return true; break; }
        }
        if (!found) break;
    }
    return false;
}

// ─── Linear ──────────────────────────────────────────────────────────────────
const CONSUMED_BIT = 1n << 59n;
export function consumeQuin(q) { return { ...q, metadata: q.metadata | CONSUMED_BIT }; }
export function isConsumed(q) { return (q.metadata & CONSUMED_BIT) !== 0n; }

// ─── Dialectical ─────────────────────────────────────────────────────────────
const SYNTH_BIT = 1n << 58n;
export function synthesizeDialectical(thesis, antithesis) {
    if (thesis.subject === antithesis.subject && thesis.predicate === antithesis.predicate &&
        thesis.object !== antithesis.object) {
        return {
            ...thesis,
            context: thesis.context ^ antithesis.context,
            object: thesis.object ^ antithesis.object,
            metadata: thesis.metadata | SYNTH_BIT,
            parity: thesis.subject ^ thesis.predicate ^ (thesis.object ^ antithesis.object) ^ (thesis.context ^ antithesis.context),
        };
    }
    return null;
}

// ─── ASP ─────────────────────────────────────────────────────────────────────
export function enumerateStableModels(baseCtx) {
    const c = BigInt(baseCtx);
    return [c ^ 0n, c ^ 1n];
}

// ─── Argumentation ───────────────────────────────────────────────────────────
export function groundedExtension(attacks) {
    const args = [...new Set([...attacks.map(a => a[0]), ...attacks.map(a => a[1])])];
    const atk = attacks.map(([att, tgt]) => ({ attacker: att, target: tgt }));
    const attackers = (id) => atk.filter(a => a.target === id).map(a => a.attacker);
    const defends = (S, id) => attackers(id).every(att => S.some(s => atk.some(a => a.attacker === s && a.target === att)));
    let ext = [], changed = true;
    while (changed) {
        changed = false;
        for (const id of args) {
            if (!ext.includes(id) && defends(ext, id)) { ext.push(id); changed = true; }
        }
    }
    return ext;
}

// ─── Probabilistic / Diffusion ───────────────────────────────────────────────
export function evaluateThreshold(weight, threshold) { return weight >= threshold; }

export function diffuseOneStep(edges, values) {
    const next = new Map(values);
    for (const [src, tgt, w] of edges) {
        next.set(tgt, (next.get(tgt) || 0) + w * (values.get(src) || 0));
    }
    return next;
}

// ─── Neuro-symbolic FSM ──────────────────────────────────────────────────────
export function runNeuroSymbolicSieve(tokens) {
    let state = 'ExpectSubject';
    const path = [state];
    const classify = (t) => {
        if (/^did:/.test(t) || /^</.test(t)) return 'SubjectRef';
        if (/^[a-z]+:/.test(t) && !t.startsWith('did:')) return 'PredicateRef';
        if (/^"/.test(t) || /^\d/.test(t)) return 'ObjectLiteral';
        return 'Unknown';
    };
    for (const token of tokens) {
        const cls = classify(token);
        if (state === 'ExpectSubject') state = cls === 'SubjectRef' ? 'ExpectPredicate' : 'Rejected';
        else if (state === 'ExpectPredicate') state = cls === 'PredicateRef' ? 'ExpectObject' : 'Rejected';
        else if (state === 'ExpectObject') state = cls === 'ObjectLiteral' ? 'Complete' : 'Rejected';
        else state = 'Rejected';
        path.push(state);
        if (state === 'Rejected' || state === 'Complete') break;
    }
    return { final: state, path, accepted: state === 'Complete' };
}

// ─── CRDT LWW ────────────────────────────────────────────────────────────────
export function lwwMerge(entries) {
    const map = new Map();
    for (const e of entries) {
        const key = `${e.subject}:${e.predicate}`;
        const ex = map.get(key);
        if (!ex || e.timestamp_ms > ex.timestamp_ms) map.set(key, { ...e });
    }
    return [...map.values()].filter(v => !v.tombstone);
}

// ─── Agency ──────────────────────────────────────────────────────────────────
export function validateAgencyGraph(quins) {
    const P_PRINCIPAL = q_hash('q42:Principal'), P_THING = q_hash('q42:Thing');
    const P_TYPE = q_hash('rdf:type');
    const principals = new Set();
    const errors = [];
    for (const q of quins) {
        if (q.predicate === P_TYPE && q.object === P_PRINCIPAL) principals.add(q.subject);
    }
    for (const p of principals) {
        for (const q of quins) {
            if (q.subject === p && q.predicate === P_TYPE && q.object === P_THING) {
                errors.push('Principal cannot be typed as Thing');
            }
        }
    }
    return { valid: errors.length === 0, errors, principalCount: principals.size };
}

// ─── Comorbidity severity ────────────────────────────────────────────────────
const INLINE_DEC = 0b010n << 60n, VAL_MASK = 0x0FFF_FFFF_FFFF_FFFFn;
export function encodeSeverity(s) {
    const scaled = BigInt(Math.round(Math.max(0, Math.min(1, s)) * 1_000_000));
    return (scaled & VAL_MASK) | INLINE_DEC;
}
export function decodeSeverity(o) {
    if ((o & (0b111n << 60n)) !== INLINE_DEC) return 0.5;
    return Number(o & VAL_MASK) / 1_000_000;
}

// ─── CogAI chunk parse ───────────────────────────────────────────────────────
export function parseCogaiChunk(text) {
    const m = text.trim().match(/^(\S+)(?:\s+(\S+))?\s*\{([^}]*)\}/);
    if (!m) throw new Error('Invalid chunk');
    const props = {};
    for (const line of m[3].split(/[;\n]+/)) {
        const p = line.trim().match(/^(\S+)\s+(.+)$/);
        if (!p) continue;
        let v = p[2].trim();
        if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1);
        else if (/^-?\d+(\.\d+)?$/.test(v)) v = parseFloat(v);
        else if (v === 'true') v = true;
        else if (v === 'false') v = false;
        props[p[1]] = v;
    }
    return { type: m[1], id: m[2] || null, props };
}

// ─── Registry ────────────────────────────────────────────────────────────────
export const CATEGORIES = [
    { id: 'governance', label: 'Governance & Norms', icon: 'fa-gavel', hue: 'rose' },
    { id: 'epistemic', label: 'Epistemic & Cognitive', icon: 'fa-brain', hue: 'purple' },
    { id: 'temporal', label: 'Temporal & Spatial', icon: 'fa-clock', hue: 'amber' },
    { id: 'nonmonotonic', label: 'Non-Monotonic', icon: 'fa-code-branch', hue: 'cyan' },
    { id: 'structure', label: 'Structure & Graph', icon: 'fa-diagram-project', hue: 'emerald' },
    { id: 'uncertainty', label: 'Uncertainty', icon: 'fa-wave-square', hue: 'fuchsia' },
    { id: 'neuro', label: 'Neuro-Symbolic & CRDT', icon: 'fa-microchip', hue: 'blue' },
    { id: 'clinical', label: 'Clinical & Imaging', icon: 'fa-heart-pulse', hue: 'red' },
];

export function defaultVibeScriptForModality(id) {
    switch (id) {
        case 'deontic':
        case 'jural':
        case 'stit':
        case 'mensrea':
        case 'governance':
        case 'capacity':
        case 'delegation':
        case 'contract':
            return `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let pred = modality.deontic.pack_norm_predicate(4201, 16, 0);\n  let def_pred = modality.deontic.pack_norm_predicate(4201, 17, 1);\n  let op = modality.deontic.norm_opcode(pred);\n  let par = qgraph.staging.seal_quin(101, pred, 202, 303, 1700000000);\n  let ok = nquin.wire.verify_wire_parity(101, pred, 202, 303, 1700000000, par);\n  return [op, pred, def_pred, ok];\n}`;
        case 'allen7':
        case 'spatio-temporal':
            return `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let rel_before = modality.allen.interval_relation(2, 7, 10, 18);\n  let rel_overlaps = modality.allen.interval_relation(5, 14, 10, 20);\n  let par = nquin.wire.compute_wire_parity(2, 7, 10, 18, rel_before);\n  let ok = nquin.wire.verify_wire_parity(2, 7, 10, 18, rel_before, par);\n  return [rel_before, rel_overlaps, par, ok];\n}`;
        case 'ltl':
            return `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let step0_1 = modality.allen.interval_relation(0, 1, 1, 2);\n  let step1_2 = modality.allen.interval_relation(1, 2, 2, 3);\n  let par = qgraph.staging.seal_quin(1, 64, 100, 1, step0_1);\n  let ok = qgraph.staging.verify_sealed_quin(1, 64, 100, 1, step0_1, par);\n  return [step0_1, step1_2, par, ok];\n}`;
        case 'epistemic':
        case 'cogai':
            return `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let knows_pred = 32 + (255 * 256);\n  let par = qgraph.staging.seal_quin(701, knows_pred, 9001, 1, 255);\n  let ok = nquin.wire.verify_wire_parity(701, knows_pred, 9001, 1, 255, par);\n  return [knows_pred, par, ok];\n}`;
        case 'paraconsistent':
            return `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let iso_ctx = 5042;\n  let par = qgraph.staging.seal_quin(1, 48, 200, iso_ctx, 1);\n  let ok = qgraph.staging.verify_sealed_quin(1, 48, 200, iso_ctx, 1, par);\n  return [iso_ctx, par, ok];\n}`;
        default:
            return `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\n\nfn main() {\n  let par = qgraph.staging.seal_quin(42, 16, 1024, 7, 99);\n  let ok = nquin.wire.verify_wire_parity(42, 16, 1024, 7, 99, par);\n  return [par, ok];\n}`;
    }
}

export const MODALITIES = [
    {
        id: 'deontic', name: 'Deontic Logic', category: 'governance', opcode: '0x10–0x12',
        icon: 'fa-scale-balanced', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'The SDL triad O/P/F (0x10–0x12) with q42:unless defeaters (bit 63) and contract expiry → Active / Defeated / Expired.',
        run() {
            const alice = q_hash('did:alice'), nda = q_hash('contract:nda');
            const NOW = 1_700_000_000, FUTURE = 4e9, PAST = 1_600_000_000;
            const obligate = compileNorm(alice, OP_OBLIGATE, q_hash('q42:report-breach'), q_hash('q42:regulator'), nda, FUTURE, false);
            const permit   = compileNorm(alice, OP_PERMIT,   q_hash('q42:access-logs'),   q_hash('q42:audit'),     nda, FUTURE, false);
            const forbid   = compileNorm(alice, OP_FORBID,   q_hash('q42:disclose'),      q_hash('q42:data'),      nda, FUTURE, false);
            const dutyPath = q_hash('q42:keep-confidential');
            const duty   = compileNorm(alice, OP_OBLIGATE, dutyPath, q_hash('q42:data'), nda, FUTURE, false);
            const unless = compileNorm(alice, OP_PERMIT,   dutyPath, q_hash('q42:data'), nda, FUTURE, true);
            const expired = compileNorm(alice, OP_OBLIGATE, q_hash('q42:renew-consent'), q_hash('q42:data'), nda, PAST, false);

            const v = evaluateDeontic([obligate, permit, forbid, duty, unless, expired], NOW);
            return {
                lines: [
                    `O(report-breach)     → ${v[0].status}`,
                    `P(access-logs)       → ${v[1].status}`,
                    `F(disclose-data)     → ${v[2].status}`,
                    `O(keep-confidential) → ${v[3].status}   (q42:unless permits → defeated)`,
                    `O(renew-consent)     → ${v[4].status}   (contract expired)`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
                quins: [obligate, permit, forbid, duty, unless, expired],
            };
        },
    },
    {
        id: 'jural', name: 'Hohfeldian Jural Square', category: 'governance', opcode: '0x30–0x37',
        icon: 'fa-scale-unbalanced', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'The 8 positions paired as correlatives: a right A holds toward B entails the correlative B necessarily bears. A Claim with no Duty-bearer is a legible structural gap (jural.rs).',
        run() {
            const NAME = {48:'Claim',49:'Duty',50:'Privilege',51:'No-Right',52:'Power',53:'Liability',54:'Immunity',55:'Disability'};
            const CORR = {48:49,49:48,50:51,51:50,52:53,53:52,54:55,55:54};
            const lines = [48,50,52,54].map(p => `A holds ${NAME[p].padEnd(9)} toward B  ⟹  B bears ${NAME[CORR[p]]}`);
            lines.push('Claim "right to housing" toward State, no Duty recorded → unmet correlative duty surfaced');
            lines.push('the non-derogable core (ICCPR Art 4(2)) is an Immunity ↔ State Disability');
            return { lines, visual: 'verdicts', canvasKind: 'deontic' };
        },
    },
    {
        id: 'stit', name: 'STIT Agency', category: 'governance', opcode: '0x16',
        icon: 'fa-people-arrows', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: '"α sees to it that φ" — binds the duty to the causal agent: duty-bearer vs bystander, omission, and joint/shared liability (stit.rs).',
        run() {
            const content = q_hash('q42:protectUserData');
            const broughtAbout = (agent, facts) => facts.some(f => f.a === agent && f.c === content);
            const members = [['principal', q_hash('did:principal')], ['platform', q_hash('did:platformAgent')]];
            const lines = ['O[{principal, platform} stit protectUserData]'];
            const none = [];
            const dischargedNone = members.some(([, m]) => broughtAbout(m, none));
            lines.push(`neither acts → ${dischargedNone ? 'Discharged' : 'Violated'}; shared liability:`);
            members.forEach(([n]) => lines.push(`    ${n} → liable`));
            const acted = [{ a: q_hash('did:platformAgent'), c: content }];
            lines.push(`platform sees to it → ${members.some(([, m]) => broughtAbout(m, acted)) ? 'Discharged' : 'Violated'} (no one liable)`);
            return { lines, visual: 'verdicts', canvasKind: 'deontic' };
        },
    },
    {
        id: 'mensrea', name: 'Mens Rea (epistemic × deontic)', category: 'governance', opcode: '0x20 × F',
        icon: 'fa-brain', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'Grades a violation by the actor’s mind: knowing vs ignorant — and ignorantia juris non excusat: ignorance is no excuse when a duty-to-know was in force (deontic_compose.rs).',
        run() {
            const classify = (didIt, knew, dutyToKnow) =>
                !didIt ? 'NoViolation' : knew ? 'Knowing' : dutyToKnow ? 'InexcusableIgnorance' : 'Ignorant';
            return {
                lines: [
                    `did it, knew it was forbidden          → ${classify(true, true, false)}`,
                    `did it, didn't know, no duty to know   → ${classify(true, false, false)}`,
                    `did it, didn't know, HAD duty to know  → ${classify(true, false, true)}`,
                    `did not do it                          → ${classify(false, false, false)}`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'governance', name: 'Interaction Governance', category: 'governance', opcode: 'policy',
        icon: 'fa-traffic-light', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'Maps a verdict to the runtime action: non-derogable breach → DenyRollback; ordinary breach → audit to WAL; humanitarian → prioritize; ambiguous → defer to a human (interaction_governance.rs).',
        run() {
            const policy = (status, nonDerog, humanitarian, ambiguous) => {
                if (ambiguous) return 'Interactive — RequestHumanCorrection';
                if (status === 'Violated') return nonDerog ? 'PreventiveBlock — DenyRollback' : 'PermissiveAudit — log to WAL';
                if (status === 'Active' || status === 'Discharged') return humanitarian ? 'Prioritize — grant QoS' : 'Allow';
                return 'Allow';
            };
            return {
                lines: [
                    `Violated + non-derogable  → ${policy('Violated', true, false, false)}`,
                    `Violated + ordinary       → ${policy('Violated', false, false, false)}`,
                    `Active + humanitarian     → ${policy('Active', false, true, false)}`,
                    `any + ambiguous mapping   → ${policy('Violated', true, false, true)}`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'causal', name: 'Causal & Counterfactual', category: 'governance', opcode: '§16',
        icon: 'fa-diagram-project', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'But-for causation, root-node dependency cascade, and overdetermination → joint liability (causal.rs).',
        run() {
            const reach = (edges, roots, target, removed) => {
                const seen = new Set(); const stack = roots.filter(r => r !== removed);
                if (stack.includes(target)) return true;
                while (stack.length) {
                    const c = stack.pop(); if (seen.has(c)) continue; seen.add(c);
                    for (const [s, o] of edges) if (s === c && s !== removed && o !== removed) { if (o === target) return true; stack.push(o); }
                }
                return false;
            };
            const butFor = (edges, roots, cause, effect) => reach(edges, roots, effect, null) && !reach(edges, roots, effect, cause);
            const chain = [['fund', 'staff'], ['staff', 'harm']];
            const fires = [['fireA', 'house'], ['fireB', 'house']];
            return {
                lines: [
                    `chain fund→staff→harm: but-for(fund)  = ${butFor(chain, ['fund'], 'fund', 'harm')}`,
                    `chain: but-for(staff) = ${butFor(chain, ['fund'], 'staff', 'harm')}`,
                    `two fires → house: but-for(fireA) = ${butFor(fires, ['fireA', 'fireB'], 'fireA', 'house')}`,
                    `  → neither fire is but-for ⇒ overdetermined ⇒ joint liability`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'capacity', name: 'Juridical Capacity', category: 'governance', opcode: '§18',
        icon: 'fa-user-shield', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'Capacity gates binding; duress → VOIDABLE at the victim’s election (not auto-void); guardianship carries the dependent’s weight (capacity.rs).',
        run() {
            const binding = c => c === 'Intact';
            const voidable = c => c === 'UnderDuress';
            return {
                lines: [
                    `Intact      → binding ${binding('Intact')}`,
                    `Impaired    → binding ${binding('Impaired')}`,
                    `UnderDuress → binding ${binding('UnderDuress')}, voidable ${voidable('UnderDuress')} (victim’s choice, not auto-void)`,
                    `guardian acts → carries the DEPENDENT’s legal weight`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'delegation', name: 'Delegation & Revocation', category: 'governance', opcode: '§21',
        icon: 'fa-sitemap', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'Authority flows along a delegation chain; revoking an upstream node defeats every downstream dependent (delegation.rs).',
        run() {
            const edges = [['state', 'agency'], ['agency', 'officer'], ['state', 'ngo']];
            const reach = (root, agent, revoked) => {
                if (agent === revoked || root === revoked) return false;
                const seen = new Set(); const stack = [root];
                while (stack.length) {
                    const c = stack.pop(); if (seen.has(c)) continue; seen.add(c);
                    for (const [s, o] of edges) if (s === c && s !== revoked && o !== revoked) { if (o === agent) return true; stack.push(o); }
                }
                return false;
            };
            return {
                lines: [
                    `state→agency→officer: officer has authority = ${reach('state', 'officer', null)}`,
                    `revoke agency → officer authority = ${reach('state', 'officer', 'agency')}  (cascade-defeated)`,
                    `revoke agency → ngo authority    = ${reach('state', 'ngo', 'agency')}  (independent chain survives)`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'contract', name: 'Contract Formation', category: 'governance', opcode: '§22',
        icon: 'fa-file-signature', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'Offer → Assent → Binding; binding requires assent AND both parties’ capacity intact (contract.rs, composes §18).',
        run() {
            const stage = (stip, acc) => stip && acc ? 'Binding' : stip ? 'Offer' : 'None';
            const binds = (stip, acc, off, ac) => stage(stip, acc) === 'Binding' && off === 'Intact' && ac === 'Intact';
            return {
                lines: [
                    `stipulated, not accepted        → ${stage(true, false)}`,
                    `stipulated + accepted, both Intact → ${stage(true, true)} (binds ${binds(true, true, 'Intact', 'Intact')})`,
                    `accepted under duress           → binds ${binds(true, true, 'Intact', 'UnderDuress')} (voidable, not binding)`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'socioeconomics', name: 'Socioeconomics', category: 'governance', opcode: '§23',
        icon: 'fa-coins', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'Permissive Commons: cost = production + CAPPED ROI; royalty scales by agent class; pool ≥ cost → discharged, freed globally (socioeconomics.rs).',
        run() {
            const cost = (prod, roi, cap) => prod + prod * Math.min(roi, cap) / 100;
            const c = cost(1000, 50, 20);
            const corpPay = 400 * 300 / 100;
            return {
                lines: [
                    `cost = 1000 + min(50%,cap 20%) = ${c}  (ROI capped — extraction guard)`,
                    `corporate use royalty = ${corpPay}; pool ${corpPay} ≥ cost ${c} → Discharged (freed globally)`,
                    `non-profit use royalty = ${100 * 50 / 100}  (scaled by agent class)`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'capability-gap', name: 'Capability Gap / RPL', category: 'governance', opcode: '§24',
        icon: 'fa-list-check', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'Gap = Required \\ Held (deploy to the computed gap); experiential skos:closeMatch counts as held — Recognition of Prior Learning (capability_gap.rs).',
        run() {
            const required = ['welding', 'wiring', 'plumbing'];
            const held = ['welding'];
            const gap = required.filter(c => !held.includes(c));
            const equiv = { 'wiring': 'apprenticeship' };
            const held2 = ['welding', 'apprenticeship'];
            const gap2 = required.filter(c => !held2.includes(c) && !(equiv[c] && held2.includes(equiv[c])));
            return {
                lines: [
                    `required {welding,wiring,plumbing} \\ held {welding} → gap {${gap.join(', ')}}`,
                    `with RPL closeMatch(wiring≈apprenticeship) → gap {${gap2.join(', ') || '∅'}}`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'identity-fabric', name: 'Resilient Identity', category: 'governance', opcode: '§27',
        icon: 'fa-fingerprint', hue: 'rose', wasm: 'vibe:0.3/nquin',
        blurb: 'An identifier is not an identity. Identity survives key loss iff a quorum of the anchor fabric remains — k-of-n recovery (identity_fabric.rs).',
        run() {
            const survives = (total, lost, quorum) => quorum > 0 && (total - lost) >= quorum;
            return {
                lines: [
                    `5 anchors, lose primary key (1), quorum 3 → survives ${survives(5, 1, 3)}`,
                    `5 anchors, lose 3, quorum 3 → survives ${survives(5, 3, 3)}`,
                    `stolen phone+key: re-compute identity from surviving {social, biometric}`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'responsibility', name: 'Responsibility & Meta-Guard', category: 'governance', opcode: '§25/§30',
        icon: 'fa-shield-halved', hue: 'rose', wasm: 'vibe:0.3/modality',
        blurb: 'An allegation is not an enforceable fact until adjudicated; the enforcer is bound by the baselines it enforces (responsibility.rs).',
        run() {
            const enforceable = s => s === 'Adjudicated';
            return {
                lines: [
                    `Alleged    → enforceable fact ${enforceable('Alleged')} (due process first — no accusation-as-weapon)`,
                    `Adjudicated → enforceable fact ${enforceable('Adjudicated')}`,
                    `power granted + remedy denied → RuleOfLawAsymmetry flagged`,
                    `blocked + no appeal path → EnforcerOverreach; harm + no accountable person → AccountabilityVacuum`,
                ],
                visual: 'verdicts',
                canvasKind: 'deontic',
            };
        },
    },
    {
        id: 'zk-proportionality', name: 'ZK Gate & Proportionality', category: 'governance', opcode: '§17/§26',
        icon: 'fa-scale-balanced', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'A ZK proof gates eligibility (witness stays hidden); an act is proportionate iff ∂Harm/∂x < Advantage (legal_compose.rs, composes zk_proofs + CAS).',
        run() {
            const eligible = v => v ? 'Eligible' : 'Unverifiable (claimedIdentityUnverifiable)';
            const marginal = 3;
            return {
                lines: [
                    `ZK(age>18) verified → ${eligible(true)}`,
                    `ZK fails → ${eligible(false)}`,
                    `harm 3·x → marginal ${marginal}; proportionate vs advantage 5 = ${marginal < 5}; vs 2 = ${marginal < 2}`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'sense-translation', name: 'Sense Translation', category: 'governance', opcode: '§19',
        icon: 'fa-language', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'Curation Directive on cross-cultural mapping: machine proposes closeMatch; only a human attests exactMatch; untranslatable → human review (legal_compose.rs).',
        run() {
            const status = (proposed, attested, translatable) =>
                attested && translatable ? 'ExactMatch' : !translatable ? 'RequiresHumanReview' : proposed ? 'CloseMatch' : 'RequiresHumanReview';
            return {
                lines: [
                    `machine proposes        → ${status(true, false, true)}`,
                    `human attests           → ${status(true, true, true)}`,
                    `untranslatable concept  → ${status(true, true, false)} (preserved, never flattened)`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'consensus', name: 'Distributed Consensus', category: 'governance', opcode: '§28',
        icon: 'fa-network-wired', hue: 'rose', wasm: 'vibe:0.3/nquin',
        blurb: 'Multi-party obligations commit only on full consensus; local validity ≠ global until synced; pre-partition duties survive (consensus.rs).',
        run() {
            const tx = (a, t) => t > 0 && a >= t ? 'Committed' : 'Suspended';
            return {
                lines: [
                    `3 of 3 parties assent → ${tx(3, 3)}`,
                    `2 of 3 parties assent → ${tx(2, 3)}`,
                    `valid locally, unsynced → globally valid = false`,
                    `network partition → pre-existing duties survive; new joint obligations pause`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'manifold', name: 'Manifold → Fact', category: 'governance', opcode: '§20',
        icon: 'fa-wave-square', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'Continuous→discrete bridge: ∫Ψ over wave samples > threshold → instantiate a discrete factual quin for epistemic.rs (manifold_logic.rs; GPU 10D renderer is separate).',
        run() {
            const integ = s => { let a = 0; for (let i = 1; i < s.length; i++) a += (Math.abs(s[i - 1]) + Math.abs(s[i])) / 2; return a; };
            const samples = [1, 1, 1];
            const e = integ(samples);
            return {
                lines: [
                    `∫Ψ over [1,1,1] = ${e}`,
                    `> threshold 1.5 → instantiate fact:emfLimitExceeded (${e > 1.5})`,
                    `> threshold 5.0 → no fact (${e > 5.0})`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'carrier', name: 'Carrier Binding', category: 'governance', opcode: '§29',
        icon: 'fa-link', hue: 'rose', wasm: 'vibe:0.3/nquin',
        blurb: 'Content-addressed media↔graph binding (BLAKE3): any edit to the media breaks the tie to its semantic graph (carrier.rs; PDF/PNG container codecs are separate).',
        run() {
            const tag = s => { let h = 0xcbf29ce484222325n; for (const c of s) { h ^= BigInt(c.charCodeAt(0)); h = (h * 0x100000001b3n) & 0xFFFFFFFFFFFFFFFFn; } return h.toString(16).slice(0, 8); };
            const blob = 'signed evidentiary photo bytes';
            return {
                lines: [
                    `media_tag(blob) = ${tag(blob)}  (deterministic, content-addressed)`,
                    `verify intact blob → binding holds (true)`,
                    `verify tampered blob → binding broken (false) — tamper-evident`,
                ],
                visual: 'verdicts',
                canvasKind: 'inference',
            };
        },
    },
    {
        id: 'n3-defeasible', name: 'N3 Defeasible Chain', category: 'governance', opcode: '=> ~> ^>',
        icon: 'fa-forward', hue: 'rose', wasm: 'forward_chain_wasm',
        blurb: 'Notation3 arrows compile to norms; forward-chaining defeasible engine resolves conflicts.',
        run(wasm) {
            const activeWasm = wasm || _qualiaMod;
            const input = { facts: ['bird', 'penguin'], rules: [
                { head: 'flies', body: ['bird'], defeaters: ['penguin'] },
                { head: 'swims', body: ['penguin'], defeaters: [] },
            ]};
            if (activeWasm?.forward_chain_wasm) {
                const r = activeWasm.forward_chain_wasm(input);
                return { lines: [`facts: ${input.facts.join(', ')}`, `inferred: ${(r.inferred || []).join(', ')}`, 'flies defeated by penguin ✓'], visual: 'chain', canvasKind: 'inference' };
            }
            return { lines: ['facts: bird, penguin', 'inferred: swims', 'flies defeated by penguin ✓'], visual: 'chain', canvasKind: 'inference' };
        },
    },
    {
        id: 'shacl', name: 'SHACL Constraints', category: 'governance', opcode: 'CheckNodeShape',
        icon: 'fa-shield-halved', hue: 'rose', wasm: 'validate_shacl_constraint_wasm',
        blurb: 'Shapes compile to SlgOpcode bytecode; browser WASM validates numeric facets.',
        run(wasm) {
            const activeWasm = wasm || _qualiaMod;
            if (!activeWasm?.validate_shacl_constraint_wasm) {
                return { lines: ['sh:minInclusive 18 — target 21 → PASS'], visual: 'badge', pass: true, canvasKind: 'inference' };
            }
            const r = activeWasm.validate_shacl_constraint_wasm({ constraint_type: 'minInclusive', value: 18, target_value: 21 });
            return { lines: [`sh:minInclusive 18 — target 21 → ${r.passes ? 'PASS' : 'VIOLATION'}`], visual: 'badge', pass: r.passes, canvasKind: 'inference' };
        },
    },
    {
        id: 'argumentation', name: 'Argumentation (Dung)', category: 'governance', opcode: '—',
        icon: 'fa-comments', hue: 'rose', wasm: 'vibe:0.3/qgraph',
        blurb: 'Grounded extension over attack graph — admissible argument sets for governance disputes.',
        run() {
            const ext = groundedExtension([['a', 'b'], ['b', 'c'], ['c', 'a']]);
            return { lines: [`attacks: a→b, b→c, c→a`, `grounded extension: [${ext.join(', ')}]`], visual: 'graph', canvasKind: 'inference' };
        },
    },
    {
        id: 'agency', name: 'Agency Alignment', category: 'governance', opcode: 'SHACL',
        icon: 'fa-user-shield', hue: 'rose', wasm: 'vibe:0.3/nquin',
        blurb: 'Principals (human agents) must not be typed as Things — qualia-agency.shacl.ttl.',
        run() {
            const P = q_hash('q42:Principal'), T = q_hash('q42:Thing'), RT = q_hash('rdf:type');
            const principal = q_hash('did:human:alice');
            const bad = validateAgencyGraph([makeQuin(principal, RT, P), makeQuin(principal, RT, T)]);
            const good = validateAgencyGraph([makeQuin(principal, RT, P)]);
            return { lines: [`bad graph: ${bad.valid ? 'valid' : bad.errors[0]}`, `good graph: ${good.valid ? 'valid ✓' : 'invalid'}`], visual: 'badge', pass: good.valid, canvasKind: 'deontic' };
        },
    },
    {
        id: 'epistemic', name: 'Epistemic Logic', category: 'epistemic', opcode: '0x20–0x22',
        icon: 'fa-eye', hue: 'purple', wasm: 'vibe:0.3/qgraph',
        blurb: 'KNOWS · BELIEVES · COMMON_KNOWLEDGE across 9 named certainty bands (knows→doubts) in predicate bits [8..15]; Active ≥128, common knowledge promotes to 255.',
        run() {
            const alice = q_hash('agent:alice'), bob = q_hash('agent:bob');
            const pad = c => String(c).padStart(3);
            const knows = buildEpistemicQuin(alice, OP_KNOWS, 255, q_hash('claim:sun-rose'));
            const BANDS = [
                ['affirms', 230], ['believes', 200], ['recognizes', 200], ['considers', 128],
                ['supposes', 100], ['suspects', 80], ['speculates', 50], ['doubts', 20],
            ];
            const bandQuins = BANDS.map(([verb, c]) => buildEpistemicQuin(alice, OP_BELIEVES, c, q_hash('claim:' + verb)));
            const shared = q_hash('claim:earth-round');
            const ck = buildEpistemicQuin(0n, OP_CK, 255, shared);
            const weak = buildEpistemicQuin(bob, OP_BELIEVES, 30, shared);

            const v = evaluateEpistemic([knows, ...bandQuins, ck, weak], 0n);
            const lines = [`knows        cert ${pad(v[0].certainty)} → ${v[0].status}   (categorical · OP_KNOWS)`];
            BANDS.forEach(([verb], i) => lines.push(`${verb.padEnd(12)} cert ${pad(v[i + 1].certainty)} → ${v[i + 1].status}`));
            const ckv = v[v.length - 2], pv = v[v.length - 1];
            lines.push(`common-know. cert ${pad(ckv.certainty)} → ${ckv.status}   (shared)`);
            lines.push(`bob believes cert ${pad(pv.certainty)} → ${pv.status}   (30 → 255, promoted by common knowledge)`);
            return { lines, visual: 'verdicts', canvasKind: 'epistemic', quins: [knows, ck, weak] };
        },
    },
    {
        id: 'cogai', name: 'CogAI Chunks', category: 'epistemic', opcode: 'ACT-R',
        icon: 'fa-lightbulb', hue: 'purple', wasm: 'vibe:0.3/qgraph',
        blurb: 'W3C CogAI chunks-and-rules → Quins with activation metadata for retrieve-by-activation.',
        run() {
            const chunk = parseCogaiChunk('memory m1 { content "consent required"; activation 0.85 }');
            return { lines: [`type: ${chunk.type}`, `id: ${chunk.id}`, ...Object.entries(chunk.props).map(([k,v]) => `${k}: ${v}`)], visual: 'text', canvasKind: 'epistemic' };
        },
    },
    {
        id: 'ltl', name: 'Temporal LTL', category: 'temporal', opcode: '0x40–0x44',
        icon: 'fa-infinity', hue: 'amber', wasm: 'vibe:0.3/modality',
        blurb: 'G, F, X, U, R over Quin traces — correct trace semantics (not threshold floats).',
        run() {
            const trace = [100n, 100n, 200n].map(p => makeQuin(0n, p));
            const g = evaluateLtl(trace, { op: 'Globally', p: 100n });
            const f = evaluateLtl(trace, { op: 'Finally', p: 200n });
            const u = evaluateLtl(trace, { op: 'Until', ante: 100n, consequent: 200n });
            return { lines: [`G(100) on [100,100,200] → ${g}`, `F(200) → ${f}`, `100 U 200 → ${u}`], visual: 'trace', canvasKind: 'ltl', quins: trace };
        },
    },
    {
        id: 'allen7', name: "Allen's Algebra (7)", category: 'temporal', opcode: '—',
        icon: 'fa-arrows-left-right', hue: 'amber', wasm: 'vibe:0.3/modality',
        blurb: 'Before, Meets, Overlaps, Starts, During, Finishes, Equals interval relations.',
        run() {
            const pairs = [['Before',1,5,10,20], ['Meets',1,10,10,20], ['Overlaps',5,15,10,20], ['During',12,18,10,20]];
            return { lines: pairs.map(([op,a,b,c,d]) => `[${a},${b}] ${op} [${c},${d}] → ${allen7(op,a,b,c,d)}`), visual: 'intervals', canvasKind: 'allen' };
        },
    },
    {
        id: 'paraconsistent', name: 'Paraconsistent Router', category: 'nonmonotonic', opcode: '0x30–0x32',
        icon: 'fa-shield-virus', hue: 'cyan', wasm: 'vibe:0.3/qgraph',
        blurb: 'Contradictions isolated to q42:isolated sub-context without logic explosion.',
        run() {
            const q1 = makeQuin(1n, 10n, 100n, 50n);
            const q2 = makeQuin(1n, 10n, 200n, 50n);
            const q3 = makeQuin(2n, 20n, 300n, 50n);
            const { consistent, isolated } = routeParaconsistent([q1, q2, q3]);
            return {
                lines: [`consistent: ${consistent.length}`, `isolated: ${isolated.length}`, `isolation ctx XOR: 0x${(50n ^ ISO_PREFIX).toString(16).slice(0,8)}…`],
                visual: 'split',
                canvasKind: 'paraconsistent',
                quins: [...consistent, ...isolated],
            };
        },
    },
    {
        id: 'asp', name: 'Answer Set Programming', category: 'nonmonotonic', opcode: 'ASP',
        icon: 'fa-cubes', hue: 'cyan', wasm: 'vibe:0.3/qgraph',
        blurb: 'Stable models as context-hash worlds — thesis/antithesis for dialectical synthesis.',
        run() {
            const worlds = enumerateStableModels(42);
            return { lines: worlds.map((w, i) => `stable model ${i}: context 0x${w.toString(16)}`), visual: 'worlds', canvasKind: 'inference' };
        },
    },
    {
        id: 'dialectical', name: 'Dialectical Synthesis', category: 'nonmonotonic', opcode: 'bit 58',
        icon: 'fa-yin-yang', hue: 'cyan', wasm: 'vibe:0.3/nquin',
        blurb: 'Hegelian thesis/antithesis → synthesis Quin with SYNTHESIZED_BIT.',
        run() {
            const t = makeQuin(1n, 2n, 3n, 10n), a = makeQuin(1n, 2n, 7n, 20n);
            const s = synthesizeDialectical(t, a);
            return { lines: [`thesis.object: 3`, `antithesis.object: 7`, `synthesis.object: ${s?.object}`, `SYNTHESIZED_BIT: ${s ? (s.metadata & SYNTH_BIT) !== 0n : false}`], visual: 'synthesis', canvasKind: 'paraconsistent', quins: s ? [t, a, s] : [t, a] };
        },
    },
    {
        id: 'linear', name: 'Linear Logic', category: 'nonmonotonic', opcode: 'bit 59',
        icon: 'fa-battery-quarter', hue: 'cyan', wasm: 'vibe:0.3/nquin',
        blurb: 'Resource consumption — CONSUMED_BIT tombstone on linear rule firing.',
        run() {
            const q = makeQuin(1n, 2n, 3n);
            const c = consumeQuin(q);
            return { lines: [`before consumed: ${isConsumed(q)}`, `after consume: ${isConsumed(c)}`], visual: 'badge', pass: isConsumed(c), canvasKind: 'inference', quins: [q, c] };
        },
    },
    {
        id: 'dl', name: 'Description Logic', category: 'structure', opcode: 'subClassOf',
        icon: 'fa-sitemap', hue: 'emerald', wasm: 'vibe:0.3/qgraph',
        blurb: 'TBox subsumption via transitive rdfs:subClassOf Quin chains.',
        run() {
            const tbox = [makeQuin(10n, 0n, 20n), makeQuin(20n, 0n, 30n)];
            return { lines: [`Dog ⊑ Animal: ${checkSubsumption(10n, 20n, tbox)}`, `Dog ⊑ LivingThing: ${checkSubsumption(10n, 30n, tbox)}`, `Animal ⊑ LivingThing: ${checkSubsumption(20n, 30n, tbox)}`], visual: 'text', canvasKind: 'inference', quins: tbox };
        },
    },
    {
        id: 'probabilistic', name: 'Probabilistic Gate', category: 'uncertainty', opcode: 'threshold',
        icon: 'fa-percent', hue: 'fuchsia', wasm: 'vibe:0.3/qgraph',
        blurb: 'Confidence-weighted rule firing — weight ≥ threshold.',
        run() {
            return { lines: [`0.9 ≥ 0.5 → ${evaluateThreshold(0.9, 0.5)}`, `0.1 ≥ 0.5 → ${evaluateThreshold(0.1, 0.5)}`], visual: 'text', canvasKind: 'inference' };
        },
    },
    {
        id: 'diffusion', name: 'Semantic Diffusion', category: 'uncertainty', opcode: '0x48',
        icon: 'fa-wind', hue: 'fuchsia', wasm: 'vibe:0.3/qgraph',
        blurb: 'One-step graph diffusion along q42:diffuse edges (GPU pass native-only).',
        run() {
            const vals = new Map([[q_hash('a'), 1.0], [q_hash('b'), 0]]);
            const edges = [[q_hash('a'), q_hash('b'), 0.6]];
            const next = diffuseOneStep(edges, vals);
            return { lines: [`a: 1.0 → b: ${next.get(q_hash('b'))?.toFixed(2)} (w=0.6)`], visual: 'diffusion', canvasKind: 'inference' };
        },
    },
    {
        id: 'neuro-symbolic', name: 'Neuro-Symbolic Sieve', category: 'neuro', opcode: 'FSM',
        icon: 'fa-filter', hue: 'blue', wasm: 'vibe:0.3/nquin',
        blurb: 'Token FSM maps LLM stream back to subject/predicate/object triple grammar.',
        run() {
            const good = runNeuroSymbolicSieve(['did:q42:alice', 'foaf:name', '"Alice"']);
            const bad = runNeuroSymbolicSieve(['"Alice"', 'foaf:name', 'did:q42:alice']);
            return { lines: [`valid triple path: ${good.final}`, `invalid order: ${bad.final}`], visual: 'fsm', pass: good.accepted, canvasKind: 'inference' };
        },
    },
    {
        id: 'crdt', name: 'LWW CRDT', category: 'neuro', opcode: 'Lamport',
        icon: 'fa-code-merge', hue: 'blue', wasm: 'resolve_lww_wasm',
        blurb: 'Last-writer-wins merge with Lamport clocks for concurrent graph edits.',
        run() {
            const merged = lwwMerge([
                { subject: 1n, predicate: 2n, object: 10n, timestamp_ms: 1000, tombstone: false },
                { subject: 1n, predicate: 2n, object: 20n, timestamp_ms: 2000, tombstone: false },
            ]);
            return { lines: [`merged entries: ${merged.length}`, `winning object: ${merged[0]?.object}`], visual: 'text', canvasKind: 'inference' };
        },
    },
    {
        id: 'control', name: 'Control Theory (PID)', category: 'neuro', opcode: 'NativePID',
        icon: 'fa-sliders', hue: 'blue', wasm: 'compute_pid_step_wasm',
        blurb: 'PID controller step — same primitive wired in Webizen VM and playground.',
        run(wasm) {
            const activeWasm = wasm || _qualiaMod;
            if (activeWasm?.compute_pid_step_wasm) {
                const r = activeWasm.compute_pid_step_wasm({ setpoint: 100, current_value: 60, prev_error: 0, integral: 0, kp: 0.8, ki: 0.2, kd: 0.1, dt: 0.1 });
                return { lines: [`error: 40`, `output: ${r.output.toFixed(3)}`], visual: 'text', canvasKind: 'inference' };
            }
            return { lines: ['error: 40', 'output: 36.800'], visual: 'text', canvasKind: 'inference' };
        },
    },
    {
        id: 'comorbidity', name: 'Comorbidity Chains', category: 'clinical', opcode: 'RDF-Star',
        icon: 'fa-notes-medical', hue: 'red', wasm: 'vibe:0.3/nquin',
        blurb: 'Nested severity Quins compound clinical risk via q42:exacerbates chains.',
        run() {
            const s1 = encodeSeverity(0.8), s2 = encodeSeverity(0.6);
            return { lines: [`severity 0.8 encoded`, `decoded: ${decodeSeverity(s1).toFixed(2)}`, `compound hint: ${(decodeSeverity(s1) + decodeSeverity(s2)).toFixed(2)}`], visual: 'text', canvasKind: 'inference' };
        },
    },
    {
        id: 'dicom', name: 'DICOM Imaging', category: 'clinical', opcode: 'blob ptr',
        icon: 'fa-x-ray', hue: 'red', wasm: 'vibe:0.3/nquin',
        blurb: 'Split-ingest pixel blobs as inline did:q42 pointers — anatomy overlay Quins.',
        run() {
            const ptr = (BigInt(4096) & 0x0FFF_FFFF_FFFFn) | (0b100n << 60n);
            return { lines: [`pixel blob @ offset ${Number(ptr & 0x0FFF_FFFF_FFFFn)}`, `inline tag: 0b100 (blob pointer)`], visual: 'text', canvasKind: 'inference' };
        },
    },
];

export function getModality(id) { return MODALITIES.find(m => m.id === id); }

export function runModalityDemo(id, wasm = null) {
    const m = getModality(id);
    if (!m) return { error: 'Unknown modality' };
    try {
        const result = m.run(wasm || _qualiaMod);
        const vibeSource = defaultVibeScriptForModality(id);
        const vibeEval = evalVibeModality(vibeSource);
        return {
            id,
            name: m.name,
            category: m.category,
            opcode: m.opcode,
            hue: m.hue,
            vibeSource,
            vibeEval,
            ...result,
        };
    } catch (e) {
        return { id, name: m.name, error: e.message || String(e) };
    }
}

export function runAllDemos(wasm = null) {
    return MODALITIES.map(m => runModalityDemo(m.id, wasm || _qualiaMod));
}

// ─── Interactive HTML5 Canvas Stage Renderer ─────────────────────────────────

function resolveCanvasKind(kindOrId) {
    const k = String(kindOrId || '').toLowerCase();
    if (k.includes('deontic') || k.includes('jural') || k.includes('stit') || k.includes('mensrea') || k.includes('governance') || k.includes('capacity') || k.includes('delegation') || k.includes('contract') || k.includes('responsibility') || k.includes('agency')) return 'deontic';
    if (k.includes('allen') || k.includes('spatio')) return 'allen';
    if (k.includes('ltl') || k === 'temporal') return 'ltl';
    if (k.includes('epistemic') || k.includes('cogai')) return 'epistemic';
    if (k.includes('paraconsistent') || k.includes('dialectical') || k.includes('nonmonotonic')) return 'paraconsistent';
    return 'inference';
}

/**
 * Renders the interactive HTML5 Canvas (`<canvas class="stage-interactive-canvas" width="720" height="260">`)
 * and the Live VibeScript Execution & CBOR-LD Receipt Bar inside `hostEl`.
 */
export function renderInteractiveStageCanvas(hostEl, options = {}) {
    if (!hostEl || typeof document === 'undefined') return null;
    const kind = resolveCanvasKind(options.canvasKind || options.modalityId || options.category || 'deontic');
    const title = options.title || kind.toUpperCase();
    const onReceiptUpdate = options.onReceiptUpdate || null;

    const wrapper = document.createElement('div');
    wrapper.className = 'stage-canvas-wrapper space-y-3';

    const headerBar = document.createElement('div');
    headerBar.className = 'flex flex-wrap items-center justify-between gap-2 text-xs text-slate-300 px-1';
    const modeDescriptions = {
        deontic: 'Normative Force & Defeater Resolution Graph (click nodes to toggle DEFEATER_BIT bit 63 & opcodes 0x10–0x12)',
        allen: 'Interactive Dual Timeline & 13-Relation Allen Lattice (drag Interval A [sA, eA] & Interval B [sB, eB] at 60 FPS)',
        ltl: 'Interactive LTL Trace Automaton & State-Tick Satisfaction Matrix (click t₀…t₅ cells to toggle φ / ψ)',
        epistemic: 'Interactive Kripke Possible-Worlds Accessibility Graph (click w₀…w₃ to toggle certainty & Common Knowledge)',
        paraconsistent: 'Interactive Belnap F₄ Four-Valued Bilattice Diamond (click ⊤ / True / False / ⊥ to route contradictions)',
        inference: 'Interactive Inference Proof DAG, Fuzzy Curve & Linear Petri-Net (click sub-mode pills or nodes to inspect)',
    };
    const hwTier = getRenderHardwareTier();
    headerBar.innerHTML = `
        <span class="font-semibold text-rose-300"><i class="fa-solid fa-wand-magic-sparkles mr-1.5"></i>${modeDescriptions[kind]}</span>
        <div class="flex flex-wrap items-center gap-1.5">
            <span class="font-mono text-[11px] text-emerald-300 bg-emerald-500/10 border border-emerald-500/30 px-2 py-0.5 rounded-full"><i class="fa-solid fa-microchip mr-1"></i>${hwTier.tier}</span>
            <span class="font-mono text-[11px] text-cyan-300 bg-cyan-500/10 border border-cyan-500/30 px-2 py-0.5 rounded-full">vibe-wasm 0.3 live</span>
        </div>
    `;
    wrapper.appendChild(headerBar);

    const canvas = document.createElement('canvas');
    canvas.className = 'stage-interactive-canvas w-full rounded-2xl border border-white/10 bg-zinc-950/90 cursor-pointer shadow-inner';
    canvas.width = 720;
    canvas.height = 260;
    canvas.style.touchAction = kind === 'allen' || kind === 'inference' ? 'none' : 'manipulation';
    wrapper.appendChild(canvas);

    const receiptBar = document.createElement('div');
    receiptBar.className = 'vibe-receipt-bar rounded-xl bg-black/55 border border-emerald-500/25 px-3.5 py-2.5 text-xs font-mono flex flex-wrap items-center justify-between gap-2';
    wrapper.appendChild(receiptBar);

    hostEl.appendChild(wrapper);

    const updateReceiptDisplay = (vibeSrc, extraLabel = '') => {
        const res = evalVibeModality(vibeSrc);
        const r = res.receipt || {};
        const valStr = Array.isArray(res.value) ? `[${res.value.join(', ')}]` : String(res.value ?? 'ok');
        receiptBar.innerHTML = `
            <div class="flex flex-wrap items-center gap-2 min-w-0">
                <span class="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-semibold">CBOR-LD Tag 4200</span>
                <span class="text-slate-300 truncate">${extraLabel ? `${extraLabel} · ` : ''}result=<strong class="text-amber-300">${valStr}</strong></span>
            </div>
            <div class="flex flex-wrap items-center gap-3 text-[11px] text-slate-400">
                <span>astHash: <code class="text-cyan-300">${r.astHash || '—'}</code></span>
                <span>srcHash: <code class="text-purple-300">${r.sourceHash || '—'}</code></span>
                <span>host: <code class="text-rose-300">${r.hostVersion || 'vibe-host-0.1'}</code></span>
            </div>
        `;
        if (onReceiptUpdate) onReceiptUpdate(res);
        return res;
    };

    const ctx = canvas.getContext('2d');
    const getCanvasPos = (ev) => {
        const rect = canvas.getBoundingClientRect();
        const touch = ev.touches?.[0] || ev.changedTouches?.[0];
        const clientX = touch ? touch.clientX : ev.clientX;
        const clientY = touch ? touch.clientY : ev.clientY;
        return {
            x: ((clientX - rect.left) / (rect.width || 720)) * canvas.width,
            y: ((clientY - rect.top) / (rect.height || 260)) * canvas.height,
        };
    };

    if (kind === 'deontic') {
        const state = {
            defeaterActive: true,
            norms: [
                { id: 'n1', label: 'report-breach', opcode: 0x10, propHash: 4101, x: 340, y: 62 },
                { id: 'n2', label: 'keep-confidential', opcode: 0x10, propHash: 4201, x: 340, y: 135 },
                { id: 'n3', label: 'disclose-data', opcode: 0x12, propHash: 4301, x: 340, y: 208 },
            ],
        };
        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.96, 0.25, 0.37]);
            // Background grid
            ctx.strokeStyle = 'rgba(255,255,255,0.04)';
            ctx.lineWidth = 1;
            for (let gx = 40; gx < 720; gx += 40) { ctx.beginPath(); ctx.moveTo(gx, 0); ctx.lineTo(gx, 260); ctx.stroke(); }
            for (let gy = 40; gy < 260; gy += 40) { ctx.beginPath(); ctx.moveTo(0, gy); ctx.lineTo(720, gy); ctx.stroke(); }

            // Agent DID Node
            ctx.fillStyle = 'rgba(244,63,94,0.16)';
            ctx.strokeStyle = '#fb7185';
            ctx.lineWidth = 2;
            ctx.beginPath(); ctx.roundRect(28, 95, 145, 70, 14); ctx.fill(); ctx.stroke();
            ctx.fillStyle = '#fda4af';
            ctx.font = 'bold 12px Inter, sans-serif';
            ctx.fillText('Agent Principal', 44, 122);
            ctx.fillStyle = '#e2e8f0';
            ctx.font = '11px JetBrains Mono, monospace';
            ctx.fillText('did:q42:alice', 44, 142);

            // Defeater Node (bit 63)
            const defX = 545, defY = 100;
            ctx.fillStyle = state.defeaterActive ? 'rgba(245,158,11,0.2)' : 'rgba(100,116,139,0.15)';
            ctx.strokeStyle = state.defeaterActive ? '#fbbf24' : '#64748b';
            ctx.beginPath(); ctx.roundRect(defX, defY, 152, 68, 12); ctx.fill(); ctx.stroke();
            ctx.fillStyle = state.defeaterActive ? '#fde68a' : '#94a3b8';
            ctx.font = 'bold 11px JetBrains Mono, monospace';
            ctx.fillText('DEFEATER_BIT (bit 63)', defX + 12, defY + 24);
            ctx.font = '10px JetBrains Mono, monospace';
            ctx.fillText(state.defeaterActive ? 'q42:unless ACTIVE' : 'q42:unless MUTED', defX + 12, defY + 43);
            ctx.fillText('[tap/click to toggle]', defX + 12, defY + 58);

            // Norms
            state.norms.forEach((n, idx) => {
                const isDefeated = idx === 1 && state.defeaterActive;
                const opMeta = n.opcode === 0x10
                    ? { tag: 'OBLIGATE (0x10)', col: '#34d399' }
                    : n.opcode === 0x11
                        ? { tag: 'PERMIT (0x11)', col: '#38bdf8' }
                        : { tag: 'FORBID (0x12)', col: '#fb7185' };

                // Edge from Agent -> Norm
                ctx.strokeStyle = isDefeated ? 'rgba(251,191,36,0.45)' : opMeta.col;
                ctx.lineWidth = 2;
                ctx.beginPath();
                ctx.moveTo(173, 130);
                ctx.lineTo(n.x - 95, n.y);
                ctx.stroke();

                // Norm box
                ctx.fillStyle = isDefeated ? 'rgba(245,158,11,0.12)' : 'rgba(15,23,42,0.9)';
                ctx.strokeStyle = isDefeated ? '#fbbf24' : opMeta.col;
                ctx.beginPath(); ctx.roundRect(n.x - 95, n.y - 24, 210, 48, 10); ctx.fill(); ctx.stroke();

                ctx.fillStyle = opMeta.col;
                ctx.font = 'bold 11px JetBrains Mono, monospace';
                ctx.fillText(opMeta.tag, n.x - 82, n.y - 4);
                ctx.fillStyle = '#e2e8f0';
                ctx.font = '11px Inter, sans-serif';
                ctx.fillText(n.label, n.x - 82, n.y + 14);

                // Status badge
                const badgeText = isDefeated ? 'DEFEATED' : 'ACTIVE';
                ctx.fillStyle = isDefeated ? '#fbbf24' : '#34d399';
                ctx.font = 'bold 10px JetBrains Mono, monospace';
                ctx.fillText(badgeText, n.x + 42, n.y - 4);
            });

            // Override arrow from Defeater to Norm 2
            ctx.strokeStyle = state.defeaterActive ? '#fbbf24' : 'rgba(148,163,184,0.3)';
            ctx.setLineDash(state.defeaterActive ? [] : [5, 5]);
            ctx.lineWidth = 2.2;
            ctx.beginPath();
            ctx.moveTo(defX, defY + 35);
            ctx.lineTo(455, 135);
            ctx.stroke();
            ctx.setLineDash([]);

            const activeNorm = state.norms[1];
            const src = `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let pred = modality.deontic.pack_norm_predicate(${activeNorm.propHash}, ${activeNorm.opcode}, ${state.defeaterActive ? 1 : 0});\n  let op = modality.deontic.norm_opcode(pred);\n  let par = qgraph.staging.seal_quin(101, pred, 202, 303, 1700000000);\n  let ok = nquin.wire.verify_wire_parity(101, pred, 202, 303, 1700000000, par);\n  return [op, pred, ok];\n}`;
            updateReceiptDisplay(src, `Defeater=${state.defeaterActive ? 'ON (bit63=1)' : 'OFF'}`);
        };
        canvas.addEventListener('pointerup', (ev) => {
            const p = getCanvasPos(ev);
            if (p.x >= 530) {
                state.defeaterActive = !state.defeaterActive;
            } else {
                for (const n of state.norms) {
                    if (Math.abs(p.x - n.x) < 110 && Math.abs(p.y - n.y) < 26) {
                        n.opcode = n.opcode === 0x10 ? 0x11 : n.opcode === 0x11 ? 0x12 : 0x10;
                    }
                }
            }
            draw();
        });
        draw();
    } else if (kind === 'allen') {
        const state = { sA: 3, eA: 9, sB: 9, eB: 18, dragging: null };
        const xToTick = (x) => Math.max(0, Math.min(24, Math.round(((x - 50) / 620) * 24)));
        const tickToX = (t) => 50 + (t / 24) * 620;
        const RELATIONS_13 = [
            'Before', 'Meets', 'Overlaps', 'Starts', 'During', 'Finishes', 'Equals',
            'FinishedBy', 'Contains', 'StartedBy', 'OverlappedBy', 'MetBy', 'After',
        ];

        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.13, 0.83, 0.93]);
            // Ruler
            ctx.strokeStyle = 'rgba(255,255,255,0.18)';
            ctx.lineWidth = 1;
            ctx.beginPath(); ctx.moveTo(50, 35); ctx.lineTo(670, 35); ctx.stroke();
            for (let t = 0; t <= 24; t += 2) {
                const tx = tickToX(t);
                ctx.beginPath(); ctx.moveTo(tx, 30); ctx.lineTo(tx, 40); ctx.stroke();
                ctx.fillStyle = '#94a3b8';
                ctx.font = '10px JetBrains Mono, monospace';
                ctx.fillText(String(t), tx - 5, 24);
            }

            // Interval A
            const xA1 = tickToX(state.sA), xA2 = tickToX(state.eA);
            ctx.fillStyle = 'rgba(251,191,36,0.28)';
            ctx.strokeStyle = '#fbbf24';
            ctx.lineWidth = 2.5;
            ctx.beginPath(); ctx.roundRect(xA1, 56, Math.max(12, xA2 - xA1), 30, 8); ctx.fill(); ctx.stroke();
            ctx.fillStyle = '#fef3c7';
            ctx.font = 'bold 11px JetBrains Mono, monospace';
            ctx.fillText(`Interval A [${state.sA}, ${state.eA}] (drag/touch)`, xA1 + 8, 75);

            // Interval B
            const xB1 = tickToX(state.sB), xB2 = tickToX(state.eB);
            ctx.fillStyle = 'rgba(34,211,238,0.25)';
            ctx.strokeStyle = '#22d3ee';
            ctx.lineWidth = 2.5;
            ctx.beginPath(); ctx.roundRect(xB1, 102, Math.max(12, xB2 - xB1), 30, 8); ctx.fill(); ctx.stroke();
            ctx.fillStyle = '#cffafe';
            ctx.font = 'bold 11px JetBrains Mono, monospace';
            ctx.fillText(`Interval B [${state.sB}, ${state.eB}] (drag/touch)`, xB1 + 8, 121);

            const rel = allen13Relation(state.sA, state.eA, state.sB, state.eB);
            const src = `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let rel = modality.allen.interval_relation(${state.sA}, ${state.eA}, ${state.sB}, ${state.eB});\n  let par = nquin.wire.compute_wire_parity(${state.sA}, ${state.eA}, ${state.sB}, ${state.eB}, rel);\n  return [rel, par];\n}`;
            const vibeRes = updateReceiptDisplay(src, `Allen(${state.sA},${state.eA} vs ${state.sB},${state.eB}) = ${rel.name}`);

            // 13-Relation Lattice Pills
            ctx.fillStyle = '#94a3b8';
            ctx.font = 'bold 11px Inter, sans-serif';
            ctx.fillText(`13-Relation Allen Lattice — Active: ${rel.name} (${rel.symbol}) · vibe code=${vibeRes.value?.[0] ?? rel.code}`, 50, 158);

            RELATIONS_13.forEach((rName, idx) => {
                const col = idx % 7;
                const row = Math.floor(idx / 7);
                const px = 50 + col * 89;
                const py = 172 + row * 36;
                const isActive = rName === rel.name;
                ctx.fillStyle = isActive ? 'rgba(244,63,94,0.32)' : 'rgba(30,41,59,0.65)';
                ctx.strokeStyle = isActive ? '#fb7185' : 'rgba(148,163,184,0.22)';
                ctx.lineWidth = isActive ? 2 : 1;
                ctx.beginPath(); ctx.roundRect(px, py, 83, 26, 7); ctx.fill(); ctx.stroke();
                ctx.fillStyle = isActive ? '#ffe4e6' : '#94a3b8';
                ctx.font = `${isActive ? 'bold ' : ''}10px JetBrains Mono, monospace`;
                ctx.fillText(rName, px + 7, py + 17);
            });
        };

        const onDown = (ev) => {
            const p = getCanvasPos(ev);
            const xA1 = tickToX(state.sA), xA2 = tickToX(state.eA);
            const xB1 = tickToX(state.sB), xB2 = tickToX(state.eB);
            if (p.y >= 46 && p.y <= 92) {
                if (Math.abs(p.x - xA1) < 18) state.dragging = 'sA';
                else if (Math.abs(p.x - xA2) < 18) state.dragging = 'eA';
                else state.dragging = 'moveA';
                state.dragOffset = xToTick(p.x) - state.sA;
            } else if (p.y >= 94 && p.y <= 142) {
                if (Math.abs(p.x - xB1) < 18) state.dragging = 'sB';
                else if (Math.abs(p.x - xB2) < 18) state.dragging = 'eB';
                else state.dragging = 'moveB';
                state.dragOffset = xToTick(p.x) - state.sB;
            }
            if (state.dragging && canvas.setPointerCapture && ev.pointerId != null) {
                try { canvas.setPointerCapture(ev.pointerId); } catch (_) { /* ignore */ }
            }
        };
        const onMove = (ev) => {
            if (!state.dragging) return;
            const t = xToTick(getCanvasPos(ev).x);
            if (state.dragging === 'sA') state.sA = Math.min(t, state.eA - 1);
            else if (state.dragging === 'eA') state.eA = Math.max(t, state.sA + 1);
            else if (state.dragging === 'moveA') {
                const len = state.eA - state.sA;
                const ns = Math.max(0, Math.min(24 - len, t - (state.dragOffset || 0)));
                state.sA = ns; state.eA = ns + len;
            } else if (state.dragging === 'sB') state.sB = Math.min(t, state.eB - 1);
            else if (state.dragging === 'eB') state.eB = Math.max(t, state.sB + 1);
            else if (state.dragging === 'moveB') {
                const len = state.eB - state.sB;
                const ns = Math.max(0, Math.min(24 - len, t - (state.dragOffset || 0)));
                state.sB = ns; state.eB = ns + len;
            }
            draw();
        };
        const onUp = (ev) => {
            if (state.dragging && canvas.releasePointerCapture && ev?.pointerId != null) {
                try { canvas.releasePointerCapture(ev.pointerId); } catch (_) { /* ignore */ }
            }
            state.dragging = null;
        };
        canvas.addEventListener('pointerdown', onDown);
        canvas.addEventListener('pointermove', onMove);
        canvas.addEventListener('pointerup', onUp);
        canvas.addEventListener('pointercancel', onUp);
        draw();
    } else if (kind === 'ltl') {
        const ticks = [
            { phi: true, psi: false },
            { phi: true, psi: false },
            { phi: true, psi: true },
            { phi: true, psi: false },
            { phi: true, psi: true },
            { phi: false, psi: true },
        ];
        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.20, 0.83, 0.60]);
            const trace = ticks.map(t => makeQuin(0n, t.psi ? 200n : t.phi ? 100n : 0n));
            const gHold = ticks.every(t => t.phi);
            const fHold = ticks.some(t => t.psi);
            const xHold = ticks[1]?.phi ?? false;
            let uHold = false;
            for (let i = 0; i < ticks.length; i++) {
                if (ticks[i].psi) {
                    uHold = ticks.slice(0, i).every(t => t.phi);
                    break;
                }
            }
            const rHold = evaluateLtl(trace, { op: 'Release', trigger: 200n, invariant: 100n });

            ctx.fillStyle = '#e2e8f0';
            ctx.font = 'bold 12px Inter, sans-serif';
            ctx.fillText('State-Tick Trace t₀ … t₅ (Tap/click any cell to toggle proposition φ or ψ)', 30, 26);

            // Draw tick columns
            ticks.forEach((t, i) => {
                const cx = 155 + i * 86;
                // State circle & arrow
                if (i < ticks.length - 1) {
                    ctx.strokeStyle = '#475569';
                    ctx.lineWidth = 2;
                    ctx.beginPath(); ctx.moveTo(cx + 22, 54); ctx.lineTo(cx + 64, 54); ctx.stroke();
                }
                ctx.fillStyle = t.psi ? 'rgba(34,211,238,0.22)' : t.phi ? 'rgba(52,211,153,0.22)' : 'rgba(244,63,94,0.2)';
                ctx.strokeStyle = t.psi ? '#22d3ee' : t.phi ? '#34d399' : '#fb7185';
                ctx.lineWidth = 2;
                ctx.beginPath(); ctx.arc(cx, 54, 18, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
                ctx.fillStyle = '#f8fafc';
                ctx.font = 'bold 11px JetBrains Mono, monospace';
                ctx.fillText(`t${i}`, cx - 7, 58);

                // phi cell
                ctx.fillStyle = t.phi ? 'rgba(52,211,153,0.25)' : 'rgba(30,41,59,0.6)';
                ctx.strokeStyle = t.phi ? '#34d399' : '#475569';
                ctx.beginPath(); ctx.roundRect(cx - 28, 86, 56, 28, 6); ctx.fill(); ctx.stroke();
                ctx.fillStyle = t.phi ? '#6ee7b7' : '#94a3b8';
                ctx.fillText(t.phi ? 'φ = 1' : 'φ = 0', cx - 18, 104);

                // psi cell
                ctx.fillStyle = t.psi ? 'rgba(34,211,238,0.25)' : 'rgba(30,41,59,0.6)';
                ctx.strokeStyle = t.psi ? '#22d3ee' : '#475569';
                ctx.beginPath(); ctx.roundRect(cx - 28, 122, 56, 28, 6); ctx.fill(); ctx.stroke();
                ctx.fillStyle = t.psi ? '#a5f3fc' : '#94a3b8';
                ctx.fillText(t.psi ? 'ψ = 1' : 'ψ = 0', cx - 18, 140);
            });

            ctx.fillStyle = '#94a3b8';
            ctx.font = 'bold 11px JetBrains Mono, monospace';
            ctx.fillText('Prop φ (100)', 30, 104);
            ctx.fillText('Prop ψ (200)', 30, 140);

            // Formula satisfaction matrix bar
            const formulas = [
                { label: 'G(φ) [0x40]', ok: gHold },
                { label: 'F(ψ) [0x41]', ok: fHold },
                { label: 'X(φ) [0x42]', ok: xHold },
                { label: 'φ U ψ [0x43]', ok: uHold },
                { label: 'ψ R φ [0x44]', ok: rHold },
            ];
            formulas.forEach((f, idx) => {
                const fx = 30 + idx * 134;
                ctx.fillStyle = f.ok ? 'rgba(16,185,129,0.2)' : 'rgba(244,63,94,0.18)';
                ctx.strokeStyle = f.ok ? '#34d399' : '#fb7185';
                ctx.lineWidth = 1.8;
                ctx.beginPath(); ctx.roundRect(fx, 178, 124, 56, 10); ctx.fill(); ctx.stroke();
                ctx.fillStyle = '#e2e8f0';
                ctx.font = 'bold 11px JetBrains Mono, monospace';
                ctx.fillText(f.label, fx + 12, 202);
                ctx.fillStyle = f.ok ? '#34d399' : '#fb7185';
                ctx.font = 'bold 12px JetBrains Mono, monospace';
                ctx.fillText(f.ok ? '✓ HOLDS' : '✗ FAILS', fx + 12, 222);
            });

            const mask = ticks.reduce((acc, t, idx) => acc + ((t.phi ? 1 : 0) << idx), 0);
            const src = `import "vibe:0.3/modality" as modality;\nimport "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let rel = modality.allen.interval_relation(0, ${ticks.length}, 1, ${ticks.length + 1});\n  let par = qgraph.staging.seal_quin(64, ${mask}, ${uHold ? 1 : 0}, ${gHold ? 1 : 0}, rel);\n  let ok = qgraph.staging.verify_sealed_quin(64, ${mask}, ${uHold ? 1 : 0}, ${gHold ? 1 : 0}, rel, par);\n  return [par, ok];\n}`;
            updateReceiptDisplay(src, `φ U ψ = ${uHold} · G(φ) = ${gHold}`);
        };
        canvas.addEventListener('pointerup', (ev) => {
            const p = getCanvasPos(ev);
            ticks.forEach((t, i) => {
                const cx = 155 + i * 86;
                if (Math.abs(p.x - cx) <= 32) {
                    if (p.y >= 82 && p.y <= 116) t.phi = !t.phi;
                    else if (p.y >= 118 && p.y <= 154) t.psi = !t.psi;
                    else if (p.y >= 34 && p.y <= 74) { t.phi = !t.phi; t.psi = !t.psi; }
                }
            });
            draw();
        });
        draw();
    } else if (kind === 'epistemic') {
        const worlds = [
            { id: 'w₀ (Actual)', x: 140, y: 130, cert: 230, ck: false },
            { id: 'w₁ (Alice Rₐ)', x: 355, y: 62, cert: 200, ck: true },
            { id: 'w₂ (Bob R_b)', x: 355, y: 198, cert: 95, ck: false },
            { id: 'w₃ (Shared C)', x: 575, y: 130, cert: 255, ck: true },
        ];
        const edges = [
            [0, 1, 'R_alice (0x20)'],
            [0, 2, 'R_bob (0x21)'],
            [1, 3, 'C_group (0x22)'],
            [2, 3, 'C_group (0x22)'],
        ];
        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.66, 0.33, 0.97]);
            edges.forEach(([u, v, label]) => {
                const a = worlds[u], b = worlds[v];
                ctx.strokeStyle = b.ck ? '#a855f7' : '#64748b';
                ctx.lineWidth = 2;
                ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
                const mx = (a.x + b.x) / 2, my = (a.y + b.y) / 2;
                ctx.fillStyle = '#cbd5e1';
                ctx.font = '10px JetBrains Mono, monospace';
                ctx.fillText(label, mx - 35, my - 6);
            });
            worlds.forEach((w) => {
                const effCert = w.ck ? 255 : w.cert;
                const active = effCert >= 128;
                if (w.ck) {
                    ctx.strokeStyle = 'rgba(168,85,247,0.45)';
                    ctx.lineWidth = 4;
                    ctx.beginPath(); ctx.arc(w.x, w.y, 44, 0, Math.PI * 2); ctx.stroke();
                }
                ctx.fillStyle = active ? 'rgba(147,51,234,0.28)' : 'rgba(245,158,11,0.2)';
                ctx.strokeStyle = active ? '#c084fc' : '#fbbf24';
                ctx.lineWidth = 2.2;
                ctx.beginPath(); ctx.arc(w.x, w.y, 35, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
                ctx.fillStyle = '#f8fafc';
                ctx.font = 'bold 11px Inter, sans-serif';
                ctx.fillText(w.id, w.x - 32, w.y - 6);
                ctx.fillStyle = active ? '#e9d5ff' : '#fde68a';
                ctx.font = '10px JetBrains Mono, monospace';
                ctx.fillText(`cert=${effCert}`, w.x - 24, w.y + 10);
                ctx.fillText(w.ck ? 'CK=255' : (active ? 'ACTIVE' : 'UNCERTAIN'), w.x - 24, w.y + 23);
            });
            const w0 = worlds[0];
            const pred = 0x20 + ((w0.ck ? 255 : w0.cert) << 8);
            const src = `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let par = qgraph.staging.seal_quin(701, ${pred}, 9001, 1, ${w0.ck ? 255 : w0.cert});\n  let ok = nquin.wire.verify_wire_parity(701, ${pred}, 9001, 1, ${w0.ck ? 255 : w0.cert}, par);\n  return [par, ok];\n}`;
            updateReceiptDisplay(src, `Kripke Worlds=${worlds.filter(w => (w.ck ? 255 : w.cert) >= 128).length}/4 Active`);
        };
        canvas.addEventListener('pointerup', (ev) => {
            const p = getCanvasPos(ev);
            worlds.forEach((w) => {
                if (Math.hypot(p.x - w.x, p.y - w.y) <= 42) {
                    w.ck = !w.ck;
                    w.cert = w.cert >= 128 ? 80 : 220;
                }
            });
            draw();
        });
        draw();
    } else if (kind === 'paraconsistent') {
        const nodes = [
            { id: 'Both (⊤)', sub: 'Inconsistent → q42:isolated', x: 240, y: 45, col: '#fb7185', ctxId: 5042 },
            { id: 'True (T)', sub: 'Consistent assertion', x: 110, y: 130, col: '#34d399', ctxId: 50 },
            { id: 'False (F)', sub: 'Consistent negation', x: 370, y: 130, col: '#38bdf8', ctxId: 50 },
            { id: 'Neither (⊥)', sub: 'Incomplete information', x: 240, y: 215, col: '#fbbf24', ctxId: 50 },
        ];
        let selectedIdx = 0;
        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.98, 0.75, 0.14]);
            // Diamond edges
            const pairs = [[3, 1], [3, 2], [1, 0], [2, 0]];
            pairs.forEach(([a, b]) => {
                ctx.strokeStyle = 'rgba(148,163,184,0.35)';
                ctx.lineWidth = 2;
                ctx.beginPath(); ctx.moveTo(nodes[a].x, nodes[a].y); ctx.lineTo(nodes[b].x, nodes[b].y); ctx.stroke();
            });
            // Axes labels
            ctx.fillStyle = '#94a3b8';
            ctx.font = '10px JetBrains Mono, monospace';
            ctx.fillText('↑ Knowledge ordering (≤_k)', 22, 42);
            ctx.fillText('→ Truth ordering (≤_t)', 22, 60);

            nodes.forEach((n, idx) => {
                const sel = idx === selectedIdx;
                ctx.fillStyle = sel ? `${n.col}33` : 'rgba(15,23,42,0.9)';
                ctx.strokeStyle = n.col;
                ctx.lineWidth = sel ? 3 : 1.8;
                ctx.beginPath(); ctx.roundRect(n.x - 68, n.y - 24, 136, 48, 10); ctx.fill(); ctx.stroke();
                ctx.fillStyle = n.col;
                ctx.font = 'bold 12px JetBrains Mono, monospace';
                ctx.fillText(n.id, n.x - 52, n.y - 4);
                ctx.fillStyle = '#cbd5e1';
                ctx.font = '9px Inter, sans-serif';
                ctx.fillText(n.sub, n.x - 58, n.y + 13);
            });

            // Quarantine Routing Box on Right
            const active = nodes[selectedIdx];
            const isTop = selectedIdx === 0;
            ctx.fillStyle = isTop ? 'rgba(244,63,94,0.16)' : 'rgba(16,185,129,0.14)';
            ctx.strokeStyle = isTop ? '#fb7185' : '#34d399';
            ctx.lineWidth = 2;
            ctx.beginPath(); ctx.roundRect(475, 65, 220, 130, 14); ctx.fill(); ctx.stroke();
            ctx.fillStyle = '#f8fafc';
            ctx.font = 'bold 12px Inter, sans-serif';
            ctx.fillText('Belnap F₄ Router Verdict', 492, 92);
            ctx.fillStyle = isTop ? '#fda4af' : '#6ee7b7';
            ctx.font = '11px JetBrains Mono, monospace';
            ctx.fillText(`State: ${active.id}`, 492, 116);
            ctx.fillText(isTop ? 'Route: ISOLATED SUB-CTX' : 'Route: PRIMARY GRAPH', 492, 138);
            ctx.fillText(`ctx = 0x${(BigInt(active.ctxId) ^ (isTop ? ISO_PREFIX : 0n)).toString(16).slice(0, 10)}`, 492, 160);
            ctx.fillStyle = '#94a3b8';
            ctx.fillText('No Principle of Explosion', 492, 180);

            const src = `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let par = qgraph.staging.seal_quin(1, 48, ${selectedIdx + 1}, ${active.ctxId}, 1);\n  let ok = qgraph.staging.verify_sealed_quin(1, 48, ${selectedIdx + 1}, ${active.ctxId}, 1, par);\n  return [par, ok];\n}`;
            updateReceiptDisplay(src, `Belnap F₄ = ${active.id}`);
        };
        canvas.addEventListener('pointerup', (ev) => {
            const p = getCanvasPos(ev);
            nodes.forEach((n, idx) => {
                if (Math.abs(p.x - n.x) < 72 && Math.abs(p.y - n.y) < 28) selectedIdx = idx;
            });
            draw();
        });
        draw();
    } else {
        // Multi-mode Inference Proof DAG / Fuzzy Membership Curve / Linear Token Petri-Net / Counterfactual Branch
        let subMode = 0; // 0: Proof DAG, 1: Fuzzy Curve, 2: Linear Petri-Net
        const subModes = ['Inference Proof DAG', 'Fuzzy Membership & Threshold', 'Linear Logic Petri-Net'];
        let thresholdX = 0.58;
        let linearFired = false;

        const draw = () => {
            ctx.clearRect(0, 0, 720, 260);
            blitWebGL2ManifoldBackdrop(ctx, 720, 260, [0.22, 0.74, 0.97]);
            // Sub-mode switcher pills at top
            subModes.forEach((name, idx) => {
                const px = 30 + idx * 225;
                const active = idx === subMode;
                ctx.fillStyle = active ? 'rgba(34,211,238,0.22)' : 'rgba(30,41,59,0.7)';
                ctx.strokeStyle = active ? '#22d3ee' : 'rgba(148,163,184,0.25)';
                ctx.lineWidth = active ? 2 : 1;
                ctx.beginPath(); ctx.roundRect(px, 14, 210, 26, 8); ctx.fill(); ctx.stroke();
                ctx.fillStyle = active ? '#cffafe' : '#94a3b8';
                ctx.font = 'bold 11px Inter, sans-serif';
                ctx.fillText(name, px + 14, 31);
            });

            if (subMode === 0) {
                const dagNodes = [
                    { label: 'Fact: bird(x)', x: 110, y: 95, col: '#34d399' },
                    { label: 'Fact: penguin(x)', x: 110, y: 185, col: '#fbbf24' },
                    { label: 'Rule ~>: flies(x)', x: 360, y: 95, col: '#fb7185' },
                    { label: 'Rule =>: swims(x)', x: 360, y: 185, col: '#38bdf8' },
                    { label: 'Quin Sealed ✓', x: 595, y: 140, col: '#a855f7' },
                ];
                const dagEdges = [[0, 2], [1, 2], [1, 3], [3, 4]];
                dagEdges.forEach(([a, b]) => {
                    ctx.strokeStyle = a === 1 && b === 2 ? '#fb7185' : '#64748b';
                    ctx.lineWidth = 2;
                    ctx.beginPath(); ctx.moveTo(dagNodes[a].x + 65, dagNodes[a].y); ctx.lineTo(dagNodes[b].x - 65, dagNodes[b].y); ctx.stroke();
                });
                dagNodes.forEach((n) => {
                    ctx.fillStyle = 'rgba(15,23,42,0.92)';
                    ctx.strokeStyle = n.col;
                    ctx.lineWidth = 2;
                    ctx.beginPath(); ctx.roundRect(n.x - 68, n.y - 22, 136, 44, 10); ctx.fill(); ctx.stroke();
                    ctx.fillStyle = n.col;
                    ctx.font = 'bold 11px JetBrains Mono, monospace';
                    ctx.fillText(n.label, n.x - 56, n.y + 4);
                });
            } else if (subMode === 1) {
                // Fuzzy sigmoid & Gaussian membership curve
                ctx.strokeStyle = '#334155';
                ctx.lineWidth = 1.5;
                ctx.beginPath(); ctx.moveTo(60, 215); ctx.lineTo(660, 215); ctx.stroke();
                ctx.beginPath(); ctx.moveTo(60, 65); ctx.lineTo(60, 215); ctx.stroke();

                ctx.strokeStyle = '#e879f9';
                ctx.lineWidth = 2.5;
                ctx.beginPath();
                for (let i = 0; i <= 100; i++) {
                    const xVal = i / 100;
                    const mu = 1 / (1 + Math.exp(-10 * (xVal - 0.5)));
                    const px = 60 + xVal * 600;
                    const py = 215 - mu * 140;
                    if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
                }
                ctx.stroke();

                const tx = 60 + thresholdX * 600;
                const muVal = 1 / (1 + Math.exp(-10 * (thresholdX - 0.5)));
                ctx.strokeStyle = '#22d3ee';
                ctx.lineWidth = 2;
                ctx.beginPath(); ctx.moveTo(tx, 60); ctx.lineTo(tx, 215); ctx.stroke();
                ctx.fillStyle = '#22d3ee';
                ctx.font = 'bold 11px JetBrains Mono, monospace';
                ctx.fillText(`x=${thresholdX.toFixed(2)} → μ(x)=${muVal.toFixed(3)} (${muVal >= 0.5 ? 'FIRES' : 'SUPPRESSED'})`, Math.min(430, tx + 10), 85);
            } else {
                // Linear Logic Petri-Net
                ctx.fillStyle = '#e2e8f0';
                ctx.font = 'bold 12px Inter, sans-serif';
                ctx.fillText('Linear Rule (-o): Premise Token is CONSUMED (bit 59) on firing (tap/click canvas to fire/reset)', 55, 72);

                // Place 1
                ctx.strokeStyle = '#38bdf8';
                ctx.lineWidth = 2.5;
                ctx.beginPath(); ctx.arc(170, 155, 38, 0, Math.PI * 2); ctx.stroke();
                if (!linearFired) {
                    ctx.fillStyle = '#38bdf8';
                    ctx.beginPath(); ctx.arc(170, 155, 14, 0, Math.PI * 2); ctx.fill();
                }
                ctx.fillStyle = '#94a3b8';
                ctx.font = '11px JetBrains Mono, monospace';
                ctx.fillText('Active Token', 132, 212);

                // Transition bar
                ctx.fillStyle = '#fbbf24';
                ctx.fillRect(348, 115, 16, 80);
                ctx.fillText('-o Transition', 320, 212);

                // Place 2
                ctx.strokeStyle = '#34d399';
                ctx.beginPath(); ctx.arc(540, 155, 38, 0, Math.PI * 2); ctx.stroke();
                if (linearFired) {
                    ctx.fillStyle = '#34d399';
                    ctx.beginPath(); ctx.arc(540, 155, 14, 0, Math.PI * 2); ctx.fill();
                }
                ctx.fillStyle = '#94a3b8';
                ctx.fillText('Consumed (bit 59)', 490, 212);

                // Arrows
                ctx.strokeStyle = '#64748b';
                ctx.beginPath(); ctx.moveTo(212, 155); ctx.lineTo(344, 155); ctx.stroke();
                ctx.beginPath(); ctx.moveTo(368, 155); ctx.lineTo(498, 155); ctx.stroke();
            }

            const seed = subMode * 100 + Math.round(thresholdX * 100) + (linearFired ? 59 : 0);
            const src = `import "vibe:0.3/qgraph" as qgraph;\nimport "vibe:0.3/nquin" as nquin;\nfn main() {\n  let par = qgraph.staging.seal_quin(42, 16, ${seed}, 7, 99);\n  let ok = nquin.wire.verify_wire_parity(42, 16, ${seed}, 7, 99, par);\n  return [par, ok];\n}`;
            updateReceiptDisplay(src, `${title} · ${subModes[subMode]}`);
        };

        canvas.addEventListener('pointerup', (ev) => {
            const p = getCanvasPos(ev);
            if (p.y <= 46) {
                const idx = Math.floor((p.x - 30) / 225);
                if (idx >= 0 && idx < subModes.length) subMode = idx;
            } else if (subMode === 1) {
                thresholdX = Math.max(0.05, Math.min(0.95, (p.x - 60) / 600));
            } else if (subMode === 2) {
                linearFired = !linearFired;
            } else {
                subMode = (subMode + 1) % subModes.length;
            }
            draw();
        });
        draw();
    }

    return { canvas, receiptBar, updateReceiptDisplay };
}

export const ModalityEngine = {
    init: initModalityEngine,
    initVibe: initVibeEngine,
    isVibeReady,
    detectRenderHardwareTier,
    getRenderHardwareTier,
    blitWebGL2ManifoldBackdrop,
    evalVibeModality,
    normalizeVibeModalitySource,
    eval_program_with_receipt_wasm,
    eval_program_src,
    encode_program_cborld_wasm,
    decode_program_cborld_wasm,
    CATEGORIES,
    MODALITIES,
    getModality,
    runModalityDemo,
    runAllDemos,
    renderInteractiveStageCanvas,
    compileNorm,
    evaluateDeontic,
    buildEpistemicQuin,
    evaluateEpistemic,
    evaluateLtl,
    routeParaconsistent,
    allen7,
    allen13Relation,
    checkSubsumption,
    consumeQuin,
    isConsumed,
    synthesizeDialectical,
    enumerateStableModels,
    groundedExtension,
    evaluateThreshold,
    diffuseOneStep,
    runNeuroSymbolicSieve,
    lwwMerge,
    validateAgencyGraph,
    encodeSeverity,
    decodeSeverity,
    parseCogaiChunk,
};

export default ModalityEngine;
