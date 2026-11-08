import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const docsDir = path.resolve(__dirname, '..');

test('All overhauled demo pages wire WASM + VibeScript + WebGPU/WebGL2 + Mobile Responsive CSS', async () => {
    const pages = [
        'spatial.html',
        'scientific-computing.html',
        'science-playground.html',
        'forge-showcase.html',
        'graphics.html',
        'compute-engine.html',
        'logic-showcase.html',
        'modalities-showcase.html',
    ];

    for (const p of pages) {
        const html = fs.readFileSync(path.join(docsDir, p), 'utf8');
        assert.ok(html.includes('@media (max-width:'), `${p} must include mobile @media breakpoints`);
        assert.ok(
            html.includes('<canvas') || html.includes('id="visual-stage"'),
            `${p} must include interactive visual <canvas> or #visual-stage mount`,
        );
        assert.ok(!/sovereign/i.test(html), `${p} must not contain forbidden terminology`);
    }

    // Check WebGL2 (#version 300 es) + vibe_wasm across the spatial, modality, and page scripts
    const spatialMesh = fs.readFileSync(path.join(docsDir, 'js', 'spatial-mesh-viewport.js'), 'utf8');
    assert.ok(spatialMesh.includes('#version 300 es'), 'spatial-mesh-viewport.js must include WebGL2 #version 300 es shader');

    const modalityEngine = fs.readFileSync(path.join(docsDir, 'js', 'modality-engine.js'), 'utf8');
    assert.ok(modalityEngine.includes('#version 300 es'), 'modality-engine.js must include WebGL2 #version 300 es shader');
    assert.ok(modalityEngine.includes('vibe_wasm.js'), 'modality-engine.js must import vibe_wasm.js');

    // Verify buildIndexedMesh generates valid B-Rep topologies & Wavefront OBJ buffers
    const { buildIndexedMesh } = await import('../js/spatial-mesh-viewport.js');
    for (const kind of ['icosahedron', 'cube', 'sphere', 'torus', 'knot']) {
        const mesh = buildIndexedMesh(kind, 1.4, 2);
        assert.ok(mesh.vertices.length >= 12, `${kind} mesh must have vertices`);
        assert.ok(mesh.faces.length >= 12, `${kind} mesh must have triangle faces`);
        assert.ok(mesh.edges.length >= 18, `${kind} mesh must have edges`);
        assert.ok(mesh.objBytes.byteLength > 100, `${kind} mesh must produce Wavefront OBJ byte buffer`);
    }

    // Verify vibe-wasm executes both top-level and main() programs across Etalon & Qualia providers
    const vibeMod = await import('../pkg/vibe/vibe_wasm.js');
    const wasmBytes = fs.readFileSync(path.join(docsDir, 'pkg', 'vibe', 'vibe_wasm_bg.wasm'));
    await vibeMod.default({ module_or_path: wasmBytes });

    const topLevelSrc = `
import "vibe:0.3/render" as render;
import "vibe:0.3/mech" as mech;
import "vibe:0.3/forge" as forge;
import "vibe:0.3/gpu" as gpu;
import "vibe:0.3/signal" as signal;
import "vibe:0.3/p64" as p64;
import "vibe:0.3/nquin" as nquin;
import "vibe:0.3/infer" as infer;
import "vibe:0.3/qgraph" as qgraph;
import "vibe:0.3/modality" as modality;

let chi = render.spatial.brep_euler_characteristic(42, 120, 80);
let vol = mech.geometry.aabb_volume(-1.0, -1.0, -1.0, 1.0, 1.0, 1.0);
let wg = forge.shader.wgsl_workgroup_threads(16, 16, 1);
let occ = gpu.compute.workgroup_occupancy(256, 1024);
let rms = signal.dsp.rms_energy(3.0, 4.0);
let qbytes = p64.runtime.quant_row_bytes(4096, 4);
let par = nquin.wire.compute_wire_parity(1, 2, 3, 4, 5);
let kv = infer.kv.kv_cache_bytes(32, 8, 128, 2048, 2);
let sealed = qgraph.staging.seal_quin(10, 20, 30, 40, 50);
let allen = modality.allen.interval_relation(0, 10, 5, 15);
return chi + allen;
`;
    const res = vibeMod.eval_program_with_receipt_wasm(topLevelSrc);
    assert.equal(res.ok, true, `Expected top-level VibeScript program to succeed, got: ${JSON.stringify(res.error)}`);
    assert.ok(res.receipt && res.receipt.astHash.length === 16, 'Expected 16-hex-char CBOR-LD AST hash receipt');
});
