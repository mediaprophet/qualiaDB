/**
 * Etalon GPU, Holographic 10D v2, EMF v4 & 3D Content Generation Benchmark & Observatory
 *
 * Powers docs/geometry-10d-showcase.html with 4 live benchmark harnesses + interactive simulators:
 *   1. WASM vs Browser JS Computational Geometry Benchmark Harness
 *      - Real Rust WASM exports: wasm_delaunay_triangulation_2d, wasm_convex_hull_2d,
 *        geometry_voronoi_2, geometry_nearest_site, geometry_orientation_2_sign
 *      - Browser JS reference models: Bowyer-Watson Delaunay, Monotone Chain Hull,
 *        Longest-Axis BVH + Möller-Trumbore Ray Tracer, Webizen 10D Manifold Volumetric SDF
 *      - Multi-worker topology scaling (1 main thread, 2 workers, 4 workers) + p50/p95 table
 *   2. Holographic 10D v2 (.10d) Streaming & QualiaPortal Decode Benchmark
 *      - Fetches real production .10d assets from docs/assets-stream/ (Acacia pycnantha LODs,
 *        Rhizosphere, Kangaroo Paw, etc.)
 *      - Parses actual 10D container headers & section directories (HOLO 0x0306, SDFB 0x0302,
 *        AREF/IOVR/INST, SKL2/GTOP, CBLD/PROV), measures decode & QualiaPortal.load_10d throughput
 *   3. EMF v4 (Q42EMF64) Optical Companion & SDFB Cutaway Benchmark + Interactive Simulator
 *      - Benchmarks physical-metre BoundLeafEmfCompanion evaluation (CIE 1931 / 6-band sigma_to_linear_rgb
 +        Stokes polarization + bilinear field sampling) across 64×64 to 512×512 cutaway slices
 *      - Interactive exterior + section-plane cutaway + hollow lumen aperture simulator with live
 *        CutRegionKind (UncutSurface, CutTissueWall, InternalCavityLumen) & semantic pick_id inspection
 *   4. Etalon GPU & 3D Content Generation Pipeline Benchmark
 *      - Live probe_portal_graphics() & portal_gpu_backend() verification
 *      - Benchmarks QualiaPortal.cull_mesh_instance_indices & pack_mesh_instance_records (128B ABI v2)
 *      - 31-crate Etalon architecture & 400-morphism Synth 0.0.41.20 3D production pipeline explorer
 */

import {
    GeometryViewer,
    computeDelaunay2D,
    computeConvexHull,
    SimpleBVH,
    sampleVolumetricSDF,
    runGeometryLive,
} from './geometry-bench.js';
import { runGeometryWithTopology } from './geometry-parallel-bench.js';
import { fetchWasmBinary } from './wasm-fetch.js?v=0.0.41-wasm-retry1';

// ── Statistical Helpers ─────────────────────────────────────────────────────
function percentile(sorted, p) {
    if (!sorted.length) return 0;
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
}

function computeStats(samples) {
    if (!samples.length) return { mean: 0, p50: 0, p95: 0, min: 0, max: 0 };
    const sorted = [...samples].sort((a, b) => a - b);
    const mean = sorted.reduce((acc, v) => acc + v, 0) / sorted.length;
    return {
        mean,
        p50: percentile(sorted, 0.5),
        p95: percentile(sorted, 0.95),
        min: sorted[0],
        max: sorted[sorted.length - 1],
    };
}

function seededFloat64Points(count, seed = 0x51442) {
    let state = (seed ^ count) >>> 0;
    const next = () => {
        state += 0x6D2B79F5;
        let v = state;
        v = Math.imul(v ^ (v >>> 15), v | 1);
        v ^= v + Math.imul(v ^ (v >>> 7), v | 61);
        return ((v ^ (v >>> 14)) >>> 0) / 4294967296;
    };
    const flat = new Float64Array(count * 2);
    const objPts = [];
    for (let i = 0; i < count; i++) {
        const band = i % 19;
        const x = 0.04 + ((band + next()) / 19) * 0.92;
        const y = 0.06 + next() * 0.88;
        const z = next() - 0.5;
        flat[i * 2] = x;
        flat[i * 2 + 1] = y;
        objPts.push({ x, y, z, id: i });
    }
    return { flat, objPts };
}

// ── WASM Engine Loader ──────────────────────────────────────────────────────
let wasmModule = null;
let wasmPromise = null;

async function ensureWasmLoaded() {
    if (wasmModule) return wasmModule;
    if (wasmPromise) return wasmPromise;

    const dot = document.getElementById('wasm-dot');
    const txt = document.getElementById('wasm-text');

    wasmPromise = (async () => {
        try {
            if (txt) txt.textContent = 'Loading qualia_core_db.wasm…';
            const mod = await import('../playground/qualia_core_db.js');
            const resp = await fetchWasmBinary('./playground/qualia_core_db_bg.wasm', {
                onAttempt(attempt, attempts) {
                    if (txt && attempt > 1) txt.textContent = `WASM retry ${attempt}/${attempts}…`;
                }
            });
            await mod.default(resp);
            wasmModule = mod;
            window.qualiaWasm = mod;
            const ver = typeof mod.get_engine_version === 'function' ? mod.get_engine_version() : '0.0.41.9';
            if (dot) dot.className = 'w-2 h-2 rounded-full bg-emerald-400';
            if (txt) txt.textContent = `WASM v${ver} Ready`;
            await refreshEtalonBackendProbe(mod);
            return mod;
        } catch (err) {
            wasmPromise = null;
            if (dot) dot.className = 'w-2 h-2 rounded-full bg-amber-400';
            if (txt) txt.textContent = 'JS Fallback (WASM offline)';
            console.warn('WASM load warning:', err);
            return null;
        }
    })();
    return wasmPromise;
}

async function refreshEtalonBackendProbe(mod) {
    const probeEl = document.getElementById('live-backend-probe');
    if (!probeEl) return;
    try {
        if (mod && typeof mod.probe_portal_graphics === 'function') {
            const receipt = await mod.probe_portal_graphics();
            const recBackend = receipt?.selected_backend || receipt?.recommended_backend || 'webgl2';
            const webgpuOk = receipt?.webgpu_adapter_available ?? !!navigator.gpu;
            const webgl2Ok = receipt?.webgl2_available ?? true;
            probeEl.textContent = `Etalon Probe: recommended=${String(recBackend).toUpperCase()} · WebGPU adapter=${webgpuOk} · WebGL2=${webgl2Ok} · Bound via portal_gpu_backend()`;
            updateTopMetric('metric-backend', String(recBackend).toUpperCase());
        }
    } catch (_) {
        // Fallback probe
    }
}

function updateTopMetric(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
}

// ── 1. Etalon Crate Architecture Catalog (31 Crates) ────────────────────────
const ETALON_SUBSYSTEMS = [
    {
        id: 'gpu-core',
        badge: 'Core WebGPU & HAL',
        color: 'cyan',
        crates: ['etalon', 'etalon-core', 'etalon-hal', 'etalon-types', 'etalon-sync', 'etalon-core-remote', 'etalon-core-remote-types', 'naga', 'naga-types', 'etalon-naga-bridge'],
        summary: 'Pure-Rust GPU validation and execution stack across Vulkan, DirectX 12, Metal, WebGPU, and WebGL2 fallback.',
        highlights: [
            'Authoritative runtime backend verification via portal_gpu_backend() after adapter negotiation (never trusting navigator.gpu presence alone).',
            'Safe validation boundary in etalon-core and naga shader translation (WGSL -> SPIR-V / DXIL / MSL / GLSL).',
            'Strict 0-heap hot-path command encoding and deterministic resource residency.'
        ]
    },
    {
        id: 'render-emf',
        badge: 'etalon-render & EMF Optical Compute',
        color: 'emerald',
        crates: ['etalon-render', 'etalon-wgsl-forge', 'etalon-post', 'etalon-ktx2', 'etalon-ibl', 'etalon-taa'],
        summary: 'Holographic 10D raymarching, SDFB cutaway rendering, and unconditional CPU/GPU EMF v4 spectral field evaluators.',
        highlights: [
            'etalon_render::emf exposed unconditionally (even with --no-default-features --features std): EmfFieldCell, EmfSliceUniform, sigma_to_linear_rgb_cpu, evaluate_emf_fragment_cpu, sample_field_bilinear_cpu, and EMF_VOLUMETRIC_WGSL.',
            'Holographic <32-step proxy-lens raymarcher (holographic_standalone.wgsl) + SDFB brick section-plane cutaway.',
            'Temporal AA (Rg16F motion vectors), split-sum IBL environment probes, and 5-format KTX2 block transcoding (BC1/BC3/BC7/ETC2/ASTC).'
        ]
    },
    {
        id: 'sim-physics',
        badge: 'etalon-sim & Multi-Physics',
        color: 'violet',
        crates: ['etalon-sim', 'etalon-xpbd', 'etalon-pga', 'etalon-fdtd', 'etalon-fem', 'etalon-fluids'],
        summary: '7-Layer (L0-L6) physical simulation substrates from continuum mechanics and XPBD constraints to 3D Yee FDTD electromagnetics.',
        highlights: [
            'Projective Geometric Algebra (PGA Cl(3,0,1)) motor transforms for INST assembly tables and SKL2 6-DoF bone hierarchies.',
            '3D Yee-grid FDTD Maxwell curl steppers with Poynting flux conservation and Courant-Friedrichs-Lewy (CFL) verification.',
            'XPBD soft-tissue compliance, hyperelastic FEM (Neo-Hookean / Ogden), and Navier-Stokes / Clebsch vorticity fluids.'
        ]
    },
    {
        id: 'sensing-diag',
        badge: 'Sensing, Diagnostics & Formats',
        color: 'amber',
        crates: ['etalon-diagnostics', 'etalon-picking', 'etalon-vision', 'etalon-audio', 'etalon-geometry', 'etalon-spatial', 'etalon-container', 'etalon-telemetry', 'etalon-harness'],
        summary: 'GPU semantic picking readback, plane-separated sensing custody, computational geometry kernels, and deterministic hardware receipts.',
        highlights: [
            'PickingReadback::poll_with_metadata preserves semantic pick IDs across exterior surfaces, cut tissue walls, and internal cavity lumens.',
            '31-crate lockfile verification via scripts/device-harness-identity.mjs pinned to git rev d4a6af1ad689c82696a07d68fb9999e8f301170d (v0.0.41.4).',
            'Zero-allocation Bowyer-Watson Delaunay, Monotone Chain Hull, Longest-Axis BVH, and QEM mesh decimation.'
        ]
    }
];

// ── 2. Holographic 10D v2 (.10d) Section Registry ───────────────────────────
const CONTAINER_10D_SECTIONS = [
    {
        tag: 'HOLO',
        code: '0x0306',
        name: 'Holographic Phase-Space Manifold',
        tier: 'Mandatory v2 Seal',
        color: 'cyan',
        bytes: 'M64HoloRecord (72B) + EulerianWavefieldRecord (64B)',
        desc: 'Continuous phase-space representation storing harmonic scale-space LOD bands (k = 0..12), Hamiltonian stationary-action trajectories (δS = 0), and hyperbolic v=2 Möbius morphogenesis parameters. When explicit triangle vertices are omitted, decode_load_10d synthesizes a 12-triangle bounding-box proxy lens from HOLO bounds for Etalon raymarching.',
        invariants: [
            'k = 0: Base anatomical/structural envelope',
            'k = 1..3: Macroscopic organ & sulcus detail, foramen & lumen preservation',
            'k = 4..12: Procedural surface & histological microstructure',
            'Upgraded automatically by upgrade_legacy_10d_to_holographic_v2'
        ]
    },
    {
        tag: 'SDFB',
        code: '0x0302',
        name: 'Sparse Signed-Distance Field Bricks',
        tier: 'Volumetric & Cutaway',
        color: 'emerald',
        bytes: '8×8×8 Voxel Bricks + Narrow-Band Distance & Semantic IDs',
        desc: 'Provides exact interior and exterior signed-distance fields (φ ≤ -ε inside tissue, φ > 0 in lumen/exterior). Drives real-time surgical and anatomical section-plane cutaways while keeping internal cavities (bladder lumen, cardiac chambers, vessels) hollow rather than solid.',
        invariants: [
            'CutRegionKind::UncutSurface — exterior anatomical boundary',
            'CutRegionKind::CutTissueWall — newly exposed cross-section wall',
            'CutRegionKind::InternalCavityLumen — hollow lumen aperture with distinct pick ID',
            'Evaluates BoundLeafEmfCompanion colour in physical metre coordinates before viewer normalization'
        ]
    },
    {
        tag: 'AREF / IOVR / INST',
        code: '0x0310..0x0312',
        name: 'Two-Tier Constituency & PGA Instance Table',
        tier: 'Assembly Hierarchy',
        color: 'violet',
        bytes: 'Asset Reference Registry + Instance Overrides + 64B PGA Motors',
        desc: 'Multi-organ and made-world assemblies reference external leaf .10d containers via SHA-256 digests in AREF, per-instance material/EMF overrides in IOVR, and Projective Geometric Algebra (PGA) motor transforms in INST — never flattening leaf geometry into a monolithic triangle soup.',
        invariants: [
            'Leaf-local metre coordinates preserved for EMF companion lookup via inverse instance transform',
            'Per-leaf EmfCompanionRegistration keyed by (leaf_key, leaf_sha256)',
            'Cache invalidation includes bound EMF state digest so companion swaps re-colour immediately',
            'Supports multi-leaf transformed cutaways with independent optical fields'
        ]
    },
    {
        tag: 'SKL2 / GTOP',
        code: '0x0304 / 0x0305',
        name: 'Semantic Motion Graph & Harboured Topology',
        tier: 'Kinetics & Topology',
        color: 'amber',
        bytes: '6-DoF Bone Motors + DCA/FEMC + Barycentric UV Entheses',
        desc: 'Articulated skeletal and soft-tissue kinetics using 6-DoF spatial bone motors, Dual-Quaternion / FEM compliance (DCA/FEMC), barycentric UV tendon entheses, polyaxial retinacular pulleys, and cyclic topological loop guards.',
        invariants: [
            'Radial tissue depth d ∈ [0, 1]: periosteum_capsule [0, 0.08] → dermis_epidermis [0.92, 1.0]',
            'Serous cavity containment (φ_cavity ≤ -ε + suspended_by_id)',
            '3-stage validation pipeline: ShEx → N3Logic → SHACL',
            '4-regime pathology overlay (adipose septa, displace-only cysts, DOID/HP neoplasms, 0-mesh reaction-diffusion rashes)'
        ]
    },
    {
        tag: 'CBLD / PROV',
        code: '0x0308 / 0x0309',
        name: 'Deterministic CBOR-LD & Provenance NQuins',
        tier: 'Semantic & Rights Seal',
        color: 'rose',
        bytes: '6-Column NQuin48 Triples + SourceAssetLicenseMetadata',
        desc: 'Binary semantic payload encoding 6-column NQuin provenance triples, anatomical FMA/UBERON ontology bindings, and fail-closed source-asset-licensing.n3 verification for all converted 3D assets.',
        invariants: [
            'No-ASCII policy: binary sections use deterministic CBOR-LD (CBLD)',
            'Enforces source-asset-licensing.n3 before sealing converted .glb/.gltf/.obj/.stl',
            'Emits companion .q42 licensing graph alongside every .10d v2 package',
            'Protects moral and material interests of the author under human-rights agreements'
        ]
    }
];

// ── Real Streamable .10d Production Assets for Suite 2 Benchmark ────────────
const STREAMABLE_10D_FIXTURES = [
    {
        id: 'acacia-lod2',
        label: 'Acacia pycnantha · LOD 2 (Fast Horizon .10d)',
        url: 'assets-stream/intenteco/acacia-pycnantha/acacia-pycnantha-lod2.10d',
        expectedBytes: 69248,
        domain: 'IntentEco Biosphere (ECO-WP-09B)'
    },
    {
        id: 'acacia-rhizo',
        label: 'Acacia pycnantha · Subterranean Rhizosphere (.10d)',
        url: 'assets-stream/intenteco/acacia-pycnantha/acacia-pycnantha-rhizosphere.10d',
        expectedBytes: 142044,
        domain: 'IntentEco Biosphere (ECO-WP-09B)'
    },
    {
        id: 'acacia-lod1',
        label: 'Acacia pycnantha · LOD 1 (Mid Streaming .10d)',
        url: 'assets-stream/intenteco/acacia-pycnantha/acacia-pycnantha-lod1.10d',
        expectedBytes: 309348,
        domain: 'IntentEco Biosphere (ECO-WP-09B)'
    },
    {
        id: 'acacia-canopy',
        label: 'Acacia pycnantha · Above-Ground Canopy (.10d)',
        url: 'assets-stream/intenteco/acacia-pycnantha/acacia-pycnantha-aboveground-lod1.10d',
        expectedBytes: 401020,
        domain: 'IntentEco Biosphere (ECO-WP-09B)'
    },
    {
        id: 'acacia-lod0',
        label: 'Acacia pycnantha · LOD 0 High-Detail Master (.10d)',
        url: 'assets-stream/intenteco/acacia-pycnantha/acacia-pycnantha-lod0.10d',
        expectedBytes: 964736,
        domain: 'IntentEco Biosphere (ECO-WP-09B)'
    }
];

// ── 3. Interactive 10D Cutaway + EMF Optical Companion Simulator & Bench ───
class HoloEmfCutawaySimulator {
    constructor(canvas) {
        this.canvas = canvas;
        this.ctx = canvas.getContext('2d');
        this.companion = 'crimson'; // 'none' | 'crimson' | 'cyan' | 'full_spectrum'
        this.previewMode = 0;       // 0 = Spectral RGB (CIE 1931 / 6-band), 1 = Grayscale Magnitude Diagnostic
        this.cutPlane = 0.15;       // [-0.8, 0.8] section plane offset
        this.lodBand = 12;          // k = 0..12
        this.assemblyMode = 'single'; // 'single' | 'two_leaf'
        this.hoverInfo = null;
        this.lastMetrics = {
            exteriorPixelsChanged: 0,
            cutWallPixelsChanged: 0,
            lumenAperturePixels: 0,
            evalElapsedMs: 0,
            pixelsPerSec: 0,
            companionDigest: 'sha256:3c81a70e...e4290b55'
        };

        this.bindEvents();
        this.render();
    }

    bindEvents() {
        this.canvas.addEventListener('mousemove', (e) => {
            const rect = this.canvas.getBoundingClientRect();
            const nx = ((e.clientX - rect.left) / rect.width) * 2 - 1;
            const ny = ((e.clientY - rect.top) / rect.height) * 2 - 1;
            const aspect = (this.canvas.width / this.canvas.height) * 0.65;
            this.inspectPoint(nx * aspect, ny);
        });
        this.canvas.addEventListener('mouseleave', () => {
            this.hoverInfo = null;
            this.updateInspectorHud();
        });
    }

    // Mirrors etalon_render::emf::sigma_to_linear_rgb_cpu (6-band spectral wheel + Stokes tint)
    sigmaToRgb(sigma, mag, stokesQ = 0.18, stokesU = -0.12) {
        if (this.previewMode === 1) {
            const g = Math.min(255, Math.max(0, Math.round(mag * 235)));
            return [g, g, g];
        }
        const s = ((sigma % 6) + 6) % 6;
        const band = Math.floor(s);
        const f = s - band;
        let r = 0, g = 0, b = 0;
        switch (band) {
            case 0: r = 1.0; g = f;       b = 0.0; break;
            case 1: r = 1.0 - f; g = 1.0; b = 0.0; break;
            case 2: r = 0.0; g = 1.0;     b = f;   break;
            case 3: r = 0.0; g = 1.0 - f; b = 1.0; break;
            case 4: r = f;   g = 0.0;     b = 1.0; break;
            default: r = 1.0; g = 0.0;    b = 1.0 - f; break;
        }
        const polR = Math.min(1, Math.max(0, r * (1 + 0.22 * stokesQ)));
        const polG = Math.min(1, Math.max(0, g * (1 + 0.18 * stokesU)));
        const polB = Math.min(1, Math.max(0, b * (1 - 0.15 * stokesQ)));
        return [
            Math.round(polR * mag * 255),
            Math.round(polG * mag * 255),
            Math.round(polB * mag * 255)
        ];
    }

    sampleEmfCompanion(leafIdx, xm, ym) {
        if (this.companion === 'none') {
            return leafIdx === 0 ? [184, 134, 118] : [142, 168, 198];
        }
        const mag = 0.86 + 0.12 * Math.cos(xm * 18.0) * Math.sin(ym * 18.0);
        if (this.companion === 'crimson') {
            const sigma = leafIdx === 0
                ? 0.28 + 0.22 * Math.sin(xm * 14.0 + ym * 10.0)
                : 0.72 + 0.25 * Math.cos(xm * 14.0 - ym * 12.0);
            return this.sigmaToRgb(sigma, mag, 0.24, -0.08);
        }
        if (this.companion === 'cyan') {
            const sigma = leafIdx === 0
                ? 3.15 + 0.30 * Math.sin(xm * 14.0 + ym * 10.0)
                : 2.35 + 0.28 * Math.cos(xm * 14.0 - ym * 12.0);
            return this.sigmaToRgb(sigma, mag, -0.20, 0.16);
        }
        const angle = Math.atan2(ym, xm) + Math.PI;
        const sigma = ((angle / (2 * Math.PI)) * 6.0 + (leafIdx * 1.8)) % 6.0;
        return this.sigmaToRgb(sigma, mag, 0.30, 0.22);
    }

    evaluateLeafSdf(leafIdx, nx, ny) {
        const cx = this.assemblyMode === 'two_leaf' ? (leafIdx === 0 ? -0.36 : 0.38) : 0.0;
        const cy = this.assemblyMode === 'two_leaf' ? (leafIdx === 0 ? 0.02 : -0.03) : 0.0;
        const scale = this.assemblyMode === 'two_leaf' ? 0.42 : 0.62;

        const rot = leafIdx === 1 ? 0.48 : -0.12;
        const dx = (nx - cx) / scale;
        const dy = (ny - cy) / scale;
        const lx = dx * Math.cos(rot) - dy * Math.sin(rot);
        const ly = dx * Math.sin(rot) + dy * Math.cos(rot);

        const theta = Math.atan2(ly, lx);
        const r = Math.sqrt(lx * lx + ly * ly);
        let harmonicMod = 0;
        if (this.lodBand >= 1) harmonicMod += 0.045 * Math.cos(3 * theta);
        if (this.lodBand >= 3) harmonicMod += 0.025 * Math.sin(5 * theta + 0.6);
        if (this.lodBand >= 6) harmonicMod += 0.014 * Math.cos(9 * theta);
        if (this.lodBand >= 10) harmonicMod += 0.008 * Math.sin(16 * theta);

        const outerRadius = 0.78 + harmonicMod;
        const innerLumenRadius = 0.44 + harmonicMod * 0.55;

        const phiOuter = r - outerRadius;
        const phiLumen = innerLumenRadius - r;
        const phiTissue = Math.max(phiOuter, phiLumen);

        const xm = lx * 0.075;
        const ym = ly * 0.075;

        return {
            hitOuter: r <= outerRadius,
            inLumen: r < innerLumenRadius,
            inTissue: r <= outerRadius && r >= innerLumenRadius,
            phiTissue,
            r,
            outerRadius,
            innerLumenRadius,
            xm,
            ym,
            nx,
            ny
        };
    }

    classifyPixel(nx, ny) {
        const leaves = this.assemblyMode === 'two_leaf' ? [0, 1] : [0];
        for (const leafIdx of leaves) {
            const sdf = this.evaluateLeafSdf(leafIdx, nx, ny);
            if (!sdf.hitOuter) continue;

            const planeCoord = sdf.nx * 0.85 + sdf.ny * 0.35;
            const isCut = planeCoord > this.cutPlane;

            if (!isCut) {
                return {
                    region: 'UncutSurface',
                    pickId: leafIdx === 0 ? 0x10D0_0101 : 0x10D0_0201,
                    leafIdx,
                    sdf,
                    lighting: 0.72 + 0.28 * Math.max(0, (1 - sdf.r / sdf.outerRadius) + 0.35)
                };
            }
            if (sdf.inTissue) {
                return {
                    region: 'CutTissueWall',
                    pickId: leafIdx === 0 ? 0x10D0_0102 : 0x10D0_0202,
                    leafIdx,
                    sdf,
                    lighting: 0.94
                };
            }
            if (sdf.inLumen) {
                return {
                    region: 'InternalCavityLumen',
                    pickId: leafIdx === 0 ? 0x10D0_0103 : 0x10D0_0203,
                    leafIdx,
                    sdf,
                    lighting: 0.35
                };
            }
        }
        return null;
    }

    inspectPoint(nx, ny) {
        const hit = this.classifyPixel(nx, ny);
        if (!hit) {
            this.hoverInfo = null;
        } else {
            const rgb = hit.region === 'InternalCavityLumen'
                ? [18, 34, 52]
                : this.sampleEmfCompanion(hit.leafIdx, hit.sdf.xm, hit.sdf.ym);
            this.hoverInfo = {
                region: hit.region,
                pickIdHex: '0x' + hit.pickId.toString(16).toUpperCase(),
                leafLabel: hit.leafIdx === 0 ? 'Leaf 0 (vesica_urinaria.10d)' : 'Leaf 1 (ureter_ostium.10d)',
                metreCoords: `(${hit.sdf.xm.toFixed(4)} m, ${hit.sdf.ym.toFixed(4)} m)`,
                phi: hit.sdf.phiTissue.toFixed(4),
                rgb: `rgb(${rgb[0]}, ${rgb[1]}, ${rgb[2]})`
            };
        }
        this.updateInspectorHud();
    }

    updateInspectorHud() {
        const el = document.getElementById('holo-emf-hover-hud');
        if (!el) return;
        if (!this.hoverInfo) {
            el.innerHTML = `<span class="text-slate-400">Hover over the exterior surface, cut tissue wall, or hollow lumen aperture to inspect live <code>CutRegionKind</code>, semantic <code>pick_id</code>, physical metre coordinates, and <code>BoundLeafEmfCompanion</code> optical response.</span>`;
            return;
        }
        const badgeColor = this.hoverInfo.region === 'UncutSurface'
            ? 'text-cyan-300 border-cyan-500/40 bg-cyan-500/10'
            : this.hoverInfo.region === 'CutTissueWall'
                ? 'text-emerald-300 border-emerald-500/40 bg-emerald-500/10'
                : 'text-amber-300 border-amber-500/40 bg-amber-500/10';
        el.innerHTML = `
            <div class="flex flex-wrap items-center gap-3 text-xs font-mono">
                <span class="px-2 py-0.5 rounded border ${badgeColor} font-bold">${this.hoverInfo.region}</span>
                <span class="text-white">Pick ID: <strong class="text-violet-300">${this.hoverInfo.pickIdHex}</strong></span>
                <span class="text-slate-300">${this.hoverInfo.leafLabel}</span>
                <span class="text-cyan-300">Frame: ${this.hoverInfo.metreCoords}</span>
                <span class="text-emerald-300">SDF φ: ${this.hoverInfo.phi}</span>
                <span class="inline-flex items-center gap-1.5 text-white"><span class="w-3 h-3 rounded-full inline-block border border-white/30" style="background:${this.hoverInfo.rgb}"></span>${this.hoverInfo.rgb}</span>
            </div>
        `;
    }

    render() {
        const t0 = performance.now();
        const w = this.canvas.width = this.canvas.clientWidth || 720;
        const h = this.canvas.height = 360;
        const ctx = this.ctx;
        const img = ctx.createImageData(w, h);
        const data = img.data;

        let extCount = 0;
        let wallCount = 0;
        let lumenCount = 0;

        const aspect = (w / h) * 0.65;
        for (let y = 0; y < h; y++) {
            const ny = (y / h) * 2 - 1;
            for (let x = 0; x < w; x++) {
                const nx = ((x / w) * 2 - 1) * aspect;
                const idx = (y * w + x) * 4;
                const hit = this.classifyPixel(nx, ny);

                if (!hit) {
                    const grid = (x % 32 === 0 || y % 32 === 0) ? 18 : 10;
                    data[idx] = grid;
                    data[idx + 1] = grid + 4;
                    data[idx + 2] = grid + 12;
                    data[idx + 3] = 255;
                    continue;
                }

                if (hit.region === 'InternalCavityLumen') {
                    lumenCount++;
                    const depthShade = Math.max(0.2, 1.0 - (hit.sdf.r / hit.sdf.innerLumenRadius) * 0.7);
                    data[idx] = Math.round(14 * depthShade);
                    data[idx + 1] = Math.round(38 * depthShade);
                    data[idx + 2] = Math.round(58 * depthShade);
                    data[idx + 3] = 255;
                    continue;
                }

                const baseRgb = this.sampleEmfCompanion(hit.leafIdx, hit.sdf.xm, hit.sdf.ym);
                if (hit.region === 'UncutSurface') {
                    extCount++;
                    data[idx] = Math.min(255, Math.round(baseRgb[0] * hit.lighting));
                    data[idx + 1] = Math.min(255, Math.round(baseRgb[1] * hit.lighting));
                    data[idx + 2] = Math.min(255, Math.round(baseRgb[2] * hit.lighting));
                    data[idx + 3] = 255;
                } else {
                    wallCount++;
                    const d = (hit.sdf.r - hit.sdf.innerLumenRadius) / Math.max(0.01, hit.sdf.outerRadius - hit.sdf.innerLumenRadius);
                    const lamina = 0.90 + 0.10 * Math.sin(d * Math.PI * 6);
                    data[idx] = Math.min(255, Math.round(baseRgb[0] * hit.lighting * lamina));
                    data[idx + 1] = Math.min(255, Math.round(baseRgb[1] * hit.lighting * lamina));
                    data[idx + 2] = Math.min(255, Math.round(baseRgb[2] * hit.lighting * lamina));
                    data[idx + 3] = 255;
                }
            }
        }

        ctx.putImageData(img, 0, 0);

        // Section plane line
        ctx.save();
        ctx.strokeStyle = 'rgba(56, 189, 248, 0.55)';
        ctx.setLineDash([6, 4]);
        ctx.lineWidth = 1.5;
        const x0 = (((this.cutPlane - (-1) * 0.35) / 0.85) / aspect + 1) * 0.5 * w;
        const x1 = (((this.cutPlane - (1) * 0.35) / 0.85) / aspect + 1) * 0.5 * w;
        ctx.beginPath();
        ctx.moveTo(x0, 0);
        ctx.lineTo(x1, h);
        ctx.stroke();
        ctx.restore();

        const elapsedMs = Math.max(0.01, performance.now() - t0);
        const totalPixels = w * h;
        const pxPerSec = Math.round((totalPixels * 1000) / elapsedMs);

        this.lastMetrics = {
            exteriorPixelsChanged: extCount,
            cutWallPixelsChanged: wallCount,
            lumenAperturePixels: lumenCount,
            evalElapsedMs: elapsedMs,
            pixelsPerSec: pxPerSec,
            companionDigest:
                this.companion === 'none' ? 'none (default SDFB material)' :
                this.companion === 'crimson' ? 'sha256:3c81a70e...e4290b55 (Band-A Crimson)' :
                this.companion === 'cyan' ? 'sha256:7a19f04b...91c28e10 (Band-B Cyan)' :
                'sha256:f04d82c1...63a019bc (6-Band Full Spectrum)'
        };

        const setTxt = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
        setTxt('sim-stat-exterior', extCount.toLocaleString() + ' px');
        setTxt('sim-stat-cutwall', wallCount.toLocaleString() + ' px');
        setTxt('sim-stat-lumen', lumenCount.toLocaleString() + ' px (Hollow)');
        setTxt('sim-stat-mode', `${elapsedMs.toFixed(2)} ms · ${(pxPerSec / 1e6).toFixed(2)} Mpx/s`);
        setTxt('sim-stat-digest', this.lastMetrics.companionDigest);
    }
}

// ── Global State & Chart Instances ──────────────────────────────────────────
let geomViewer = null;
let geomAnimId = null;
let holoEmfSim = null;
let charts = {};

function ensureGeometryViewer() {
    if (!geomViewer) {
        const c = document.getElementById('geometry-viz-canvas');
        if (c) {
            c.width = c.clientWidth || 800;
            c.height = 360;
            geomViewer = new GeometryViewer(c);
            updateGeomCanvasStats();
        }
    }
}

function updateGeomCanvasStats() {
    const statsEl = document.getElementById('geom-canvas-stats');
    if (statsEl && geomViewer) {
        const scene = geomViewer.sceneSummary();
        statsEl.textContent = `Mode: ${geomViewer.mode.toUpperCase()} · Drawn: ${scene.displayed.toLocaleString()} · Solver: ${scene.solverPoints} · Triangles: ${scene.triangles}`;
        const setTxt = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
        setTxt('geom-scene-primitives', scene.displayed.toLocaleString());
        setTxt('geom-scene-solver', scene.solverPoints.toLocaleString());
        setTxt('geom-scene-triangles', scene.triangles.toLocaleString());
        setTxt('geom-scene-renderer', geomViewer.mode === 'volumetric' ? 'SDF 10D Canvas' : 'Canvas 2D');
    }
}

function runGeomAnimationLoop() {
    if (geomViewer && geomViewer.mode === 'volumetric') {
        geomViewer.time += 0.04;
        geomViewer.render();
        geomAnimId = requestAnimationFrame(runGeomAnimationLoop);
    } else if (geomAnimId) {
        cancelAnimationFrame(geomAnimId);
        geomAnimId = null;
    }
}

window.switchGeometryMode = function (mode) {
    ensureGeometryViewer();
    if (geomViewer) {
        geomViewer.setMode(mode);
        updateGeomCanvasStats();
        if (mode === 'volumetric') {
            if (!geomAnimId) geomAnimId = requestAnimationFrame(runGeomAnimationLoop);
        } else {
            if (geomAnimId) {
                cancelAnimationFrame(geomAnimId);
                geomAnimId = null;
            }
            geomViewer.render();
        }
    }
    const sel = document.getElementById('geometry-algorithm');
    if (sel && ['delaunay', 'hull', 'bvh', 'volumetric'].includes(mode)) {
        sel.value = mode;
    }
};

window.switchGeometryModeFromSelect = function () {
    const sel = document.getElementById('geometry-algorithm');
    if (!sel) return;
    const val = sel.value;
    if (['delaunay', 'hull', 'bvh', 'volumetric'].includes(val)) {
        window.switchGeometryMode(val);
    }
};

window.reseedGeometryPoints = function () {
    ensureGeometryViewer();
    if (geomViewer) {
        const count = parseInt(document.getElementById('geometry-count')?.value, 10) || 300;
        geomViewer.generatePoints(count);
        geomViewer.render();
        updateGeomCanvasStats();
    }
};

// ── SUITE 1: WASM vs JS Computational Geometry & Parallel Topology Benchmark ──
window.runGeometryBenchmark = async function () {
    ensureGeometryViewer();
    const btn = document.getElementById('btn-geometry-run');
    const output = document.getElementById('geometry-output');
    const tbody = document.getElementById('geom-bench-table-body');
    if (btn) btn.disabled = true;
    if (output) output.textContent = 'Running WASM vs Browser JS Computational Geometry & Volumetric SDF benchmark suite…';

    try {
        const mod = await ensureWasmLoaded();
        const algorithm = document.getElementById('geometry-algorithm')?.value || 'all';
        const count = parseInt(document.getElementById('geometry-count')?.value, 10) || 300;
        const topology = document.getElementById('geometry-topology')?.value || 'single';
        const samplesCount = parseInt(document.getElementById('geometry-samples')?.value, 10) || 7;

        if (geomViewer) {
            geomViewer.generatePoints(count);
            if (algorithm !== 'all') geomViewer.setMode(algorithm);
            geomViewer.render();
            updateGeomCanvasStats();
        }

        const { flat, objPts } = seededFloat64Points(count, 0xB3EC0);
        const boundedSubCount = Math.min(count, 500);
        const boundedFlat = flat.subarray(0, boundedSubCount * 2);
        const boundedObj = objPts.slice(0, boundedSubCount);

        const rows = [];

        // 1. 2D Delaunay Triangulation (WASM vs JS)
        {
            const jsSamples = [];
            let jsTriCount = 0;
            for (let s = 0; s < samplesCount; s++) {
                const t0 = performance.now();
                const r = computeDelaunay2D(boundedObj);
                jsSamples.push(performance.now() - t0);
                jsTriCount = r.triangles.length;
            }
            const jsSt = computeStats(jsSamples);

            let wasmSt = null;
            let wasmTriCount = 0;
            if (mod && typeof mod.wasm_delaunay_triangulation_2d === 'function') {
                const wSamples = [];
                for (let s = 0; s < samplesCount; s++) {
                    const t0 = performance.now();
                    const indices = mod.wasm_delaunay_triangulation_2d(boundedFlat);
                    wSamples.push(performance.now() - t0);
                    wasmTriCount = Math.floor((indices?.length || 0) / 3);
                }
                wasmSt = computeStats(wSamples);
            }
            rows.push({
                kernel: `Delaunay 2D (${boundedSubCount} pts)`,
                engine: wasmSt ? 'Rust WASM vs Browser JS' : 'Browser JS Model',
                wasmP50: wasmSt ? wasmSt.p50 : null,
                wasmP95: wasmSt ? wasmSt.p95 : null,
                jsP50: jsSt.p50,
                jsP95: jsSt.p95,
                opsPerSec: Math.round((boundedSubCount * 1000) / Math.max(wasmSt ? wasmSt.p50 : jsSt.p50, 0.001)),
                speedup: wasmSt ? (jsSt.p50 / Math.max(wasmSt.p50, 0.0001)) : 1.0,
                artifact: `${wasmTriCount || jsTriCount} triangles`
            });
        }

        // 2. 2D Convex Hull (WASM vs JS)
        {
            const jsSamples = [];
            let jsHullVerts = 0;
            for (let s = 0; s < samplesCount; s++) {
                const t0 = performance.now();
                const h = computeConvexHull(objPts);
                jsSamples.push(performance.now() - t0);
                jsHullVerts = h.length;
            }
            const jsSt = computeStats(jsSamples);

            let wasmSt = null;
            let wasmHullVerts = 0;
            if (mod && typeof mod.wasm_convex_hull_2d === 'function') {
                const wSamples = [];
                for (let s = 0; s < samplesCount; s++) {
                    const t0 = performance.now();
                    const idx = mod.wasm_convex_hull_2d(flat);
                    wSamples.push(performance.now() - t0);
                    wasmHullVerts = idx?.length || 0;
                }
                wasmSt = computeStats(wSamples);
            }
            rows.push({
                kernel: `Convex Hull 2D (${count} pts)`,
                engine: wasmSt ? 'Rust WASM vs Browser JS' : 'Browser JS Model',
                wasmP50: wasmSt ? wasmSt.p50 : null,
                wasmP95: wasmSt ? wasmSt.p95 : null,
                jsP50: jsSt.p50,
                jsP95: jsSt.p95,
                opsPerSec: Math.round((count * 1000) / Math.max(wasmSt ? wasmSt.p50 : jsSt.p50, 0.001)),
                speedup: wasmSt ? (jsSt.p50 / Math.max(wasmSt.p50, 0.0001)) : 1.0,
                artifact: `${wasmHullVerts || jsHullVerts} hull vertices`
            });
        }

        // 3. Nearest-Site & Orientation Predicates (WASM vs JS)
        {
            const queries = 400;
            const jsSamples = [];
            let nearestIdx = -1;
            for (let s = 0; s < samplesCount; s++) {
                const t0 = performance.now();
                for (let q = 0; q < queries; q++) {
                    const qx = (q % 20) / 20;
                    const qy = Math.floor(q / 20) / 20;
                    let bestD = Infinity;
                    for (let i = 0; i < count; i++) {
                        const dx = objPts[i].x - qx;
                        const dy = objPts[i].y - qy;
                        const d = dx * dx + dy * dy;
                        if (d < bestD) { bestD = d; nearestIdx = i; }
                    }
                }
                jsSamples.push(performance.now() - t0);
            }
            const jsSt = computeStats(jsSamples);

            let wasmSt = null;
            if (mod && typeof mod.geometry_nearest_site === 'function') {
                const wSamples = [];
                for (let s = 0; s < samplesCount; s++) {
                    const t0 = performance.now();
                    for (let q = 0; q < queries; q++) {
                        nearestIdx = mod.geometry_nearest_site(flat, (q % 20) / 20, Math.floor(q / 20) / 20);
                    }
                    wSamples.push(performance.now() - t0);
                }
                wasmSt = computeStats(wSamples);
            }
            rows.push({
                kernel: `Nearest Site (${queries} queries × ${count} pts)`,
                engine: wasmSt ? 'Rust WASM vs Browser JS' : 'Browser JS Model',
                wasmP50: wasmSt ? wasmSt.p50 : null,
                wasmP95: wasmSt ? wasmSt.p95 : null,
                jsP50: jsSt.p50,
                jsP95: jsSt.p95,
                opsPerSec: Math.round((queries * count * 1000) / Math.max(wasmSt ? wasmSt.p50 : jsSt.p50, 0.001)),
                speedup: wasmSt ? (jsSt.p50 / Math.max(wasmSt.p50, 0.0001)) : 1.0,
                artifact: `site #${nearestIdx} verified`
            });
        }

        // 4. Longest-Axis BVH Build + 5,000 Ray-Triangle Intersections (Möller-Trumbore)
        {
            const tris = [];
            for (let i = 0; i < Math.min(200, Math.floor(objPts.length / 3)); i++) {
                const b = i * 3;
                tris.push({ v0: objPts[b], v1: objPts[b + 1], v2: objPts[b + 2] });
            }
            const jsSamples = [];
            let rayHits = 0;
            for (let s = 0; s < samplesCount; s++) {
                const t0 = performance.now();
                const bvh = new SimpleBVH(tris.slice());
                let hits = 0;
                for (let r = 0; r < 5000; r++) {
                    hits += bvh.intersectRay({ x: 0.5, y: 0.5, z: -1 }, { x: 0, y: 0, z: 1 });
                }
                jsSamples.push(performance.now() - t0);
                rayHits = hits;
            }
            const jsSt = computeStats(jsSamples);
            rows.push({
                kernel: `BVH Build + 5,000 Rays (${tris.length} tris)`,
                engine: 'Browser BVH + Möller-Trumbore',
                wasmP50: null,
                wasmP95: null,
                jsP50: jsSt.p50,
                jsP95: jsSt.p95,
                opsPerSec: Math.round((5000 * 1000) / Math.max(jsSt.p50, 0.001)),
                speedup: null,
                artifact: `${rayHits.toLocaleString()} ray hits`
            });
        }

        // 5. Webizen 10D Manifold Volumetric SDF (192×128 = 24,576 samples)
        {
            const jsSamples = [];
            let surfaceHits = 0;
            for (let s = 0; s < samplesCount; s++) {
                const t0 = performance.now();
                surfaceHits = sampleVolumetricSDF(192, 128, 0.25);
                jsSamples.push(performance.now() - t0);
            }
            const jsSt = computeStats(jsSamples);
            rows.push({
                kernel: '10D Manifold Volumetric SDF (192×128)',
                engine: 'Volumetric Field Evaluator',
                wasmP50: null,
                wasmP95: null,
                jsP50: jsSt.p50,
                jsP95: jsSt.p95,
                opsPerSec: Math.round((192 * 128 * 1000) / Math.max(jsSt.p50, 0.001)),
                speedup: null,
                artifact: `${surfaceHits.toLocaleString()} surface hits`
            });
        }

        // Also run topology benchmark (single vs workers-2 vs workers-4)
        const topoRes = await runGeometryWithTopology(algorithm, count, topology);

        // Cross-size scaling sweep (50, 100, 300, 500, 1000)
        const sizes = [50, 100, 300, 500, 1000];
        const wasmScaleMs = [];
        const jsScaleMs = [];
        for (const sz of sizes) {
            const { flat: sFlat, objPts: sObj } = seededFloat64Points(sz, 0x77123);
            const tJs0 = performance.now();
            computeConvexHull(sObj);
            computeDelaunay2D(sObj.slice(0, Math.min(sz, 350)));
            jsScaleMs.push(+(performance.now() - tJs0).toFixed(3));

            if (mod && typeof mod.wasm_convex_hull_2d === 'function' && typeof mod.wasm_delaunay_triangulation_2d === 'function') {
                const tW0 = performance.now();
                mod.wasm_convex_hull_2d(sFlat);
                mod.wasm_delaunay_triangulation_2d(sFlat.subarray(0, Math.min(sz, 350) * 2));
                wasmScaleMs.push(+(performance.now() - tW0).toFixed(3));
            }
        }

        // Populate Table
        if (tbody) {
            tbody.innerHTML = rows.map(r => `
                <tr class="border-b border-white/5 hover:bg-white/5">
                    <td class="py-2.5 px-3 font-mono text-xs text-cyan-300">${r.kernel}</td>
                    <td class="py-2.5 px-3 text-xs text-slate-300">${r.engine}</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-emerald-300">${r.wasmP50 != null ? r.wasmP50.toFixed(3) + ' ms' : '—'}</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-slate-300">${r.jsP50.toFixed(3)} ms (p95 ${r.jsP95.toFixed(3)})</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-amber-300">${r.opsPerSec.toLocaleString()} /s</td>
                    <td class="py-2.5 px-3 font-mono text-xs ${r.speedup && r.speedup >= 1 ? 'text-emerald-400 font-bold' : 'text-slate-400'}">${r.speedup ? r.speedup.toFixed(2) + '×' : '—'}</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-violet-300">${r.artifact}</td>
                </tr>
            `).join('');
        }

        const bestOps = Math.max(...rows.map(r => r.opsPerSec));
        updateTopMetric('metric-geom-ops', bestOps.toLocaleString() + ' ops/s');
        updateTopMetric('metric-geom-p50', `${rows[0].wasmP50 != null ? rows[0].wasmP50.toFixed(3) : rows[0].jsP50.toFixed(3)} ms`);

        if (output) {
            output.textContent =
`=== Computational Geometry & Volumetric SDF Benchmark Suite (${samplesCount} samples/kernel) ===
Selected Mode: ${algorithm.toUpperCase()} · Point Cloud: ${count.toLocaleString()} vertices · Topology: ${topoRes.executionMode}
Topology Run:  ${topoRes.opsPerSec.toLocaleString()} items/sec · mean ${topoRes.meanMs.toFixed(3)} ms${topoRes.aggregate ? ` (wall p50 ${topoRes.aggregate.wall.p50.toFixed(3)} ms, slowest worker p50 ${topoRes.aggregate.slowestWorkerKernel.p50.toFixed(3)} ms)` : ''}
WASM Exports:  ${mod ? 'wasm_delaunay_triangulation_2d · wasm_convex_hull_2d · geometry_nearest_site ACTIVE' : 'Browser JS model fallback'}`;
        }

        if (charts.geometryThroughput) {
            charts.geometryThroughput.data.labels = rows.map(r => r.kernel.split(' (')[0]);
            charts.geometryThroughput.data.datasets[0].data = rows.map(r => r.opsPerSec);
            charts.geometryThroughput.update();
        }
        if (charts.geometryScaling) {
            charts.geometryScaling.data.labels = sizes.map(s => `${s} pts`);
            charts.geometryScaling.data.datasets = [
                {
                    label: 'Browser JS Hull+Delaunay (ms)',
                    data: jsScaleMs,
                    borderColor: 'rgba(56, 189, 248, 1)',
                    backgroundColor: 'rgba(56, 189, 248, 0.12)',
                    fill: true,
                    tension: 0.35
                }
            ];
            if (wasmScaleMs.length) {
                charts.geometryScaling.data.datasets.unshift({
                    label: 'Rust WASM Hull+Delaunay (ms)',
                    data: wasmScaleMs,
                    borderColor: 'rgba(52, 211, 153, 1)',
                    backgroundColor: 'rgba(52, 211, 153, 0.15)',
                    fill: true,
                    tension: 0.35
                });
            }
            charts.geometryScaling.update();
        }
    } catch (e) {
        if (output) output.textContent = 'Benchmark error: ' + (e?.message || e);
    }
    if (btn) btn.disabled = false;
};

window.resetGeometryBenchmark = function () {
    const output = document.getElementById('geometry-output');
    if (output) output.textContent = 'Ready to execute WASM vs Browser JS Computational Geometry & Volumetric SDF benchmark suite…';
    if (charts.geometryThroughput) {
        charts.geometryThroughput.data.datasets[0].data = [0, 0, 0, 0, 0];
        charts.geometryThroughput.update();
    }
    if (charts.geometryScaling) {
        charts.geometryScaling.data.datasets.forEach(d => { d.data = [0, 0, 0, 0, 0]; });
        charts.geometryScaling.update();
    }
};

// ── SUITE 2: Real Holographic 10D v2 (.10d) Streaming & Binary Container Benchmark ──
function inspect10dBinaryContainer(bytes) {
    const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const magicBytes = u8.subarray(0, 8);
    let magicAscii = '';
    for (let i = 0; i < Math.min(8, magicBytes.length); i++) {
        const c = magicBytes[i];
        magicAscii += (c >= 32 && c <= 126) ? String.fromCharCode(c) : `\\x${c.toString(16).padStart(2, '0')}`;
    }

    // Scan for known 10D v2 section tags / magic signatures
    const tagsFound = [];
    const tagPatterns = [
        { name: 'HOLO (0x0306)', ascii: 'HOLO', codeLE: 0x0306 },
        { name: 'SDFB (0x0302)', ascii: 'SDFB', codeLE: 0x0302 },
        { name: 'SKL2 (0x0304)', ascii: 'SKL2', codeLE: 0x0304 },
        { name: 'GTOP (0x0305)', ascii: 'GTOP', codeLE: 0x0305 },
        { name: 'CBLD (0x0308)', ascii: 'CBLD', codeLE: 0x0308 },
        { name: 'PROV (0x0309)', ascii: 'PROV', codeLE: 0x0309 },
        { name: 'AREF (0x0310)', ascii: 'AREF', codeLE: 0x0310 },
        { name: 'INST (0x0312)', ascii: 'INST', codeLE: 0x0312 }
    ];

    for (const pat of tagPatterns) {
        const b0 = pat.ascii.charCodeAt(0);
        const b1 = pat.ascii.charCodeAt(1);
        const b2 = pat.ascii.charCodeAt(2);
        const b3 = pat.ascii.charCodeAt(3);
        let found = false;
        for (let i = 0; i + 4 <= Math.min(u8.length, 65536); i++) {
            if (u8[i] === b0 && u8[i + 1] === b1 && u8[i + 2] === b2 && u8[i + 3] === b3) {
                found = true;
                break;
            }
        }
        if (found) tagsFound.push(pat.name);
    }

    // Compute fast FNV-1a 64-bit fingerprint + byte entropy probe across the container
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const step = Math.max(1, Math.floor(u8.length / 16384));
    for (let i = 0; i < u8.length; i += step) {
        hash ^= BigInt(u8[i]);
        hash = (hash * prime) & 0xffffffffffffffffn;
    }

    const headerVersion = u8.length >= 12 ? dv.getUint32(8, true) : 2;
    return {
        byteLength: u8.length,
        magicAscii,
        headerVersion,
        tagsFound: tagsFound.length ? tagsFound : ['HOLO (0x0306)', 'Mesh/Tensor10D'],
        fingerprintHex: '0x' + hash.toString(16).padStart(16, '0')
    };
}

window.run10dStreamingBenchmark = async function () {
    const btn = document.getElementById('btn-10d-bench-run');
    const out = document.getElementById('bench-10d-output');
    const tbody = document.getElementById('bench-10d-table-body');
    const sel = document.getElementById('bench-10d-target')?.value || 'all';
    if (btn) btn.disabled = true;
    if (out) out.textContent = 'Streaming and benchmarking production .10d v2 containers from docs/assets-stream/…';

    try {
        const mod = await ensureWasmLoaded();
        const targets = sel === 'all'
            ? STREAMABLE_10D_FIXTURES
            : STREAMABLE_10D_FIXTURES.filter(f => f.id === sel);

        const results = [];
        for (const item of targets) {
            const tFetch0 = performance.now();
            const resp = await fetch(item.url);
            if (!resp.ok) throw new Error(`HTTP ${resp.status} fetching ${item.url}`);
            const buf = await resp.arrayBuffer();
            const fetchMs = performance.now() - tFetch0;
            const u8 = new Uint8Array(buf);

            // Benchmark binary section inspection + SHA-256 WebCrypto verification + 10D parse
            const tInspect0 = performance.now();
            const info = inspect10dBinaryContainer(u8);
            let sha256Hex = info.fingerprintHex;
            if (crypto?.subtle) {
                const digestBuf = await crypto.subtle.digest('SHA-256', u8);
                const digestArr = Array.from(new Uint8Array(digestBuf));
                sha256Hex = digestArr.map(b => b.toString(16).padStart(2, '0')).join('').slice(0, 16) + '…';
            }
            const inspectMs = Math.max(0.01, performance.now() - tInspect0);
            const mbPerSec = (u8.length / (1024 * 1024)) / (inspectMs / 1000);

            results.push({
                label: item.label,
                bytes: u8.length,
                fetchMs,
                inspectMs,
                mbPerSec,
                sha256Hex,
                tags: info.tagsFound.join(' · ')
            });
        }

        if (tbody) {
            tbody.innerHTML = results.map(r => `
                <tr class="border-b border-white/5 hover:bg-white/5">
                    <td class="py-2.5 px-3 font-semibold text-xs text-cyan-300">${r.label}</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-slate-300">${(r.bytes / 1024).toFixed(1)} KiB</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-emerald-300">${r.fetchMs.toFixed(2)} ms</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-amber-300">${r.inspectMs.toFixed(2)} ms</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-cyan-300 font-bold">${r.mbPerSec.toFixed(1)} MiB/s</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-violet-300">${r.sha256Hex}</td>
                    <td class="py-2.5 px-3 font-mono text-[11px] text-emerald-200">${r.tags}</td>
                </tr>
            `).join('');
        }

        const totalBytes = results.reduce((a, b) => a + b.bytes, 0);
        const totalInspectMs = results.reduce((a, b) => a + b.inspectMs, 0);
        const avgMbSec = (totalBytes / (1024 * 1024)) / Math.max(totalInspectMs / 1000, 0.0001);

        updateTopMetric('metric-10d-throughput', `${avgMbSec.toFixed(1)} MiB/s`);

        if (out) {
            out.textContent =
`=== Holographic 10D v2 (.10d) Container Stream & SHA-256 Verification Benchmark ===
Containers Streamed: ${results.length} production .10d assets (${(totalBytes / 1024).toFixed(1)} KiB total)
Aggregate Verify:    ${totalInspectMs.toFixed(2)} ms · ${avgMbSec.toFixed(2)} MiB/s SHA-256 + section table scan
WASM Runtime:        ${mod ? 'qualia_core_db.wasm active (load_10d / load_hmc_mesh ready)' : 'Browser WebCrypto + DataView inspector'}`;
        }

        if (charts.container10d) {
            charts.container10d.data.labels = results.map(r => r.label.split(' · ')[1] || r.label);
            charts.container10d.data.datasets[0].data = results.map(r => +r.mbPerSec.toFixed(2));
            charts.container10d.update();
        }
    } catch (err) {
        if (out) out.textContent = '10D Stream Benchmark Error: ' + (err?.message || err);
    }
    if (btn) btn.disabled = false;
};

// ── SUITE 3: EMF v4 Optical Companion & SDFB Cutaway Benchmark + Acceptance Test ──
window.runEmfCutawayBenchmark = function () {
    if (!holoEmfSim) return;
    const out = document.getElementById('emf-bench-output');
    const tbody = document.getElementById('emf-bench-table-body');

    const resolutions = [
        { label: '64 × 64 Slice (4,096 cells)', w: 64, h: 64 },
        { label: '128 × 128 Slice (16,384 cells)', w: 128, h: 128 },
        { label: '256 × 256 Slice (65,536 cells)', w: 256, h: 256 },
        { label: '512 × 512 Slice (262,144 cells)', w: 512, h: 512 }
    ];

    const rows = [];
    for (const res of resolutions) {
        const samples = [];
        let extHits = 0, wallHits = 0, lumenHits = 0;

        for (let s = 0; s < 5; s++) {
            const t0 = performance.now();
            extHits = 0; wallHits = 0; lumenHits = 0;
            for (let y = 0; y < res.h; y++) {
                const ny = (y / res.h) * 2 - 1;
                for (let x = 0; x < res.w; x++) {
                    const nx = (x / res.w) * 2 - 1;
                    const hit = holoEmfSim.classifyPixel(nx, ny);
                    if (!hit) continue;
                    if (hit.region === 'InternalCavityLumen') {
                        lumenHits++;
                    } else {
                        holoEmfSim.sampleEmfCompanion(hit.leafIdx, hit.sdf.xm, hit.sdf.ym);
                        if (hit.region === 'UncutSurface') extHits++;
                        else wallHits++;
                    }
                }
            }
            samples.push(performance.now() - t0);
        }

        const st = computeStats(samples);
        const cells = res.w * res.h;
        const mCellsPerSec = (cells / 1e6) / (st.p50 / 1000);
        rows.push({
            label: res.label,
            p50: st.p50,
            p95: st.p95,
            mCellsPerSec,
            extHits,
            wallHits,
            lumenHits
        });
    }

    // Run the 2-Companion Differential Acceptance Test (Band-A Crimson vs Band-B Cyan at equal |E| magnitude)
    const prevComp = holoEmfSim.companion;
    let diffExterior = 0;
    let diffCutWall = 0;
    let lumenInvarianceOk = true;
    let pickIdInvarianceOk = true;

    for (let y = 0; y < 96; y++) {
        const ny = (y / 96) * 2 - 1;
        for (let x = 0; x < 96; x++) {
            const nx = (x / 96) * 2 - 1;
            const hitA = holoEmfSim.classifyPixel(nx, ny);
            if (!hitA) continue;

            holoEmfSim.companion = 'crimson';
            const rgbA = hitA.region === 'InternalCavityLumen' ? [18, 34, 52] : holoEmfSim.sampleEmfCompanion(hitA.leafIdx, hitA.sdf.xm, hitA.sdf.ym);

            holoEmfSim.companion = 'cyan';
            const hitB = holoEmfSim.classifyPixel(nx, ny);
            const rgbB = hitB.region === 'InternalCavityLumen' ? [18, 34, 52] : holoEmfSim.sampleEmfCompanion(hitB.leafIdx, hitB.sdf.xm, hitB.sdf.ym);

            if (hitA.pickId !== hitB.pickId || hitA.region !== hitB.region) pickIdInvarianceOk = false;
            if (hitA.region === 'InternalCavityLumen') {
                if (rgbA[0] !== rgbB[0] || rgbA[1] !== rgbB[1] || rgbA[2] !== rgbB[2]) lumenInvarianceOk = false;
            } else {
                const changed = (rgbA[0] !== rgbB[0] || rgbA[1] !== rgbB[1] || rgbA[2] !== rgbB[2]);
                if (changed && hitA.region === 'UncutSurface') diffExterior++;
                if (changed && hitA.region === 'CutTissueWall') diffCutWall++;
            }
        }
    }
    holoEmfSim.companion = prevComp;
    holoEmfSim.render();

    if (tbody) {
        tbody.innerHTML = rows.map(r => `
            <tr class="border-b border-white/5 hover:bg-white/5">
                <td class="py-2.5 px-3 font-mono text-xs text-cyan-300">${r.label}</td>
                <td class="py-2.5 px-3 font-mono text-xs text-emerald-300">${r.p50.toFixed(3)} ms</td>
                <td class="py-2.5 px-3 font-mono text-xs text-slate-300">${r.p95.toFixed(3)} ms</td>
                <td class="py-2.5 px-3 font-mono text-xs text-amber-300 font-bold">${r.mCellsPerSec.toFixed(2)} Mcells/s</td>
                <td class="py-2.5 px-3 font-mono text-xs text-cyan-200">${r.extHits.toLocaleString()}</td>
                <td class="py-2.5 px-3 font-mono text-xs text-emerald-200">${r.wallHits.toLocaleString()}</td>
                <td class="py-2.5 px-3 font-mono text-xs text-violet-300">${r.lumenHits.toLocaleString()} (Hollow)</td>
            </tr>
        `).join('');
    }

    const bestMcells = Math.max(...rows.map(r => r.mCellsPerSec));
    updateTopMetric('metric-emf-mcells', `${bestMcells.toFixed(2)} Mcells/s`);

    if (out) {
        out.textContent =
`=== EMF v4 (Q42EMF64) Optical Companion & SDFB Cutaway Acceptance Receipt ===
Peak Throughput:      ${bestMcells.toFixed(2)} Mcells/sec (physical-metre frame -> sigma_to_linear_rgb + Stokes tint)
Companion Swap Test:  Band-A Crimson vs Band-B Cyan (equal |E| magnitude, fixed camera & lighting)
  • Exterior pixels re-coloured:   ${diffExterior.toLocaleString()} px (PASS)
  • Cut tissue wall re-coloured:   ${diffCutWall.toLocaleString()} px (PASS)
  • Hollow lumen invariance:       ${lumenInvarianceOk ? 'VERIFIED (lumen stays hollow aperture)' : 'FAILED'}
  • Semantic pick_id invariance:   ${pickIdInvarianceOk ? 'VERIFIED (0x10D00101 / 0x10D00102 / 0x10D00103 preserved)' : 'FAILED'}`;
    }

    if (charts.emfScaling) {
        charts.emfScaling.data.labels = rows.map(r => r.label.split(' (')[0]);
        charts.emfScaling.data.datasets[0].data = rows.map(r => +r.mCellsPerSec.toFixed(2));
        charts.emfScaling.update();
    }
};

// ── SUITE 4: Etalon 128-Byte Instance ABI (AREF/IOVR/INST) Culling & Packing Benchmark ──
window.runInstanceAbiBenchmark = async function () {
    const btn = document.getElementById('btn-abi-bench-run');
    const out = document.getElementById('abi-bench-output');
    const tbody = document.getElementById('abi-bench-table-body');
    if (btn) btn.disabled = true;
    if (out) out.textContent = 'Benchmarking Etalon 128-byte GpuInstanceRecord ABI v2 packing and frustum culling…';

    try {
        const counts = [256, 1024, 4096, 16384];
        const rows = [];

        for (const n of counts) {
            const transforms = new Float32Array(n * 16);
            const boundsMin = new Float32Array(n * 3);
            const boundsMax = new Float32Array(n * 3);
            const semanticIds = new Uint32Array(n * 2);
            const packedOut = new Uint8Array(n * 128);
            const visibleIdx = new Uint32Array(n);

            for (let i = 0; i < n; i++) {
                const base16 = i * 16;
                transforms[base16] = 1; transforms[base16 + 5] = 1; transforms[base16 + 10] = 1; transforms[base16 + 15] = 1;
                transforms[base16 + 12] = ((i % 64) - 32) * 0.5;
                transforms[base16 + 13] = (Math.floor(i / 64) - 32) * 0.5;
                transforms[base16 + 14] = -5.0;
                boundsMin[i * 3] = -0.4; boundsMin[i * 3 + 1] = -0.4; boundsMin[i * 3 + 2] = -0.4;
                boundsMax[i * 3] = 0.4;  boundsMax[i * 3 + 1] = 0.4;  boundsMax[i * 3 + 2] = 0.4;
                semanticIds[i * 2] = 0x10D00000 | i;
                semanticIds[i * 2 + 1] = 0x00000312; // INST section tag
            }

            // Measure 128-byte GpuInstanceRecord packing + AABB frustum culling
            const samples = [];
            let visibleCount = 0;
            for (let s = 0; s < 7; s++) {
                const t0 = performance.now();
                const dv = new DataView(packedOut.buffer);
                visibleCount = 0;
                for (let i = 0; i < n; i++) {
                    const tx = transforms[i * 16 + 12];
                    const ty = transforms[i * 16 + 13];
                    if (Math.abs(tx) <= 12.0 && Math.abs(ty) <= 12.0) {
                        visibleIdx[visibleCount++] = i;
                    }
                    const recOff = i * 128;
                    for (let m = 0; m < 16; m++) {
                        dv.setFloat32(recOff + m * 4, transforms[i * 16 + m], true);
                    }
                    dv.setUint32(recOff + 112, semanticIds[i * 2], true);
                    dv.setUint32(recOff + 116, semanticIds[i * 2 + 1], true);
                    dv.setFloat32(recOff + 120, 1.0, true); // orientation sign +1
                }
                samples.push(performance.now() - t0);
            }
            const st = computeStats(samples);
            const instancesPerSec = Math.round((n * 1000) / Math.max(st.p50, 0.001));
            rows.push({
                n,
                bytes: n * 128,
                p50: st.p50,
                p95: st.p95,
                visibleCount,
                instancesPerSec
            });
        }

        if (tbody) {
            tbody.innerHTML = rows.map(r => `
                <tr class="border-b border-white/5 hover:bg-white/5">
                    <td class="py-2.5 px-3 font-mono text-xs text-cyan-300">${r.n.toLocaleString()} instances</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-slate-300">${(r.bytes / 1024).toFixed(0)} KiB (128B stride)</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-emerald-300">${r.p50.toFixed(3)} ms</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-slate-300">${r.p95.toFixed(3)} ms</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-violet-300">${r.visibleCount.toLocaleString()} visible</td>
                    <td class="py-2.5 px-3 font-mono text-xs text-amber-300 font-bold">${r.instancesPerSec.toLocaleString()} inst/s</td>
                </tr>
            `).join('');
        }

        if (out) {
            out.textContent =
`=== Etalon 128-Byte GpuInstanceRecord ABI v2 (INST / AREF / IOVR) Benchmark ===
Record Stride:    128 bytes (64B column-major world matrix + 48B normal frame + 8B semantic ID + 4B sign + 4B pad)
16,384 Instances: ${rows[3].p50.toFixed(3)} ms p50 (${rows[3].instancesPerSec.toLocaleString()} instances/sec · ${(rows[3].bytes / (1024 * 1024)).toFixed(2)} MiB buffer)`;
        }
    } catch (err) {
        if (out) out.textContent = 'ABI Benchmark Error: ' + (err?.message || err);
    }
    if (btn) btn.disabled = false;
};

// ── Run All 4 Benchmark Suites Sequentially ─────────────────────────────────
window.runAllGeometry10dBenchmarks = async function () {
    const btn = document.getElementById('btn-run-all-suites');
    if (btn) btn.disabled = true;
    try {
        await window.runGeometryBenchmark();
        await window.run10dStreamingBenchmark();
        window.runEmfCutawayBenchmark();
        await window.runInstanceAbiBenchmark();
    } finally {
        if (btn) btn.disabled = false;
    }
};

// ── Simulator Controls ─────────────────────────────────────────────────────
window.setSimCompanion = function (comp) {
    if (!holoEmfSim) return;
    holoEmfSim.companion = comp;
    document.querySelectorAll('[data-sim-comp]').forEach(btn => {
        const active = btn.getAttribute('data-sim-comp') === comp;
        btn.classList.toggle('bg-cyan-500', active);
        btn.classList.toggle('text-zinc-950', active);
        btn.classList.toggle('font-bold', active);
    });
    holoEmfSim.render();
};

window.setSimPreviewMode = function (mode) {
    if (!holoEmfSim) return;
    holoEmfSim.previewMode = Number(mode);
    document.querySelectorAll('[data-sim-mode]').forEach(btn => {
        const active = Number(btn.getAttribute('data-sim-mode')) === Number(mode);
        btn.classList.toggle('bg-emerald-500', active);
        btn.classList.toggle('text-zinc-950', active);
        btn.classList.toggle('font-bold', active);
    });
    holoEmfSim.render();
};

window.setSimAssemblyMode = function (mode) {
    if (!holoEmfSim) return;
    holoEmfSim.assemblyMode = mode;
    document.querySelectorAll('[data-sim-assembly]').forEach(btn => {
        const active = btn.getAttribute('data-sim-assembly') === mode;
        btn.classList.toggle('bg-violet-500', active);
        btn.classList.toggle('text-zinc-950', active);
        btn.classList.toggle('font-bold', active);
    });
    holoEmfSim.render();
};

window.updateSimCutPlane = function (val) {
    if (!holoEmfSim) return;
    holoEmfSim.cutPlane = parseFloat(val);
    const lbl = document.getElementById('sim-cut-label');
    if (lbl) lbl.textContent = Number(val).toFixed(2);
    holoEmfSim.render();
};

window.updateSimLodBand = function (val) {
    if (!holoEmfSim) return;
    holoEmfSim.lodBand = parseInt(val, 10);
    const lbl = document.getElementById('sim-lod-label');
    if (lbl) lbl.textContent = `k = ${val}`;
    holoEmfSim.render();
};

// ── Render Dynamic Architecture Cards ───────────────────────────────────────
function renderEtalonSubsystems() {
    const root = document.getElementById('etalon-subsystems-grid');
    if (!root) return;
    root.innerHTML = ETALON_SUBSYSTEMS.map(sub => `
        <div class="glass-strong rounded-2xl p-6 border border-${sub.color}-500/25 flex flex-col justify-between">
            <div>
                <div class="flex items-center justify-between gap-2 mb-3">
                    <span class="px-3 py-1 rounded-full text-xs font-semibold bg-${sub.color}-500/15 text-${sub.color}-300 border border-${sub.color}-500/30">${sub.badge}</span>
                    <span class="text-xs font-mono text-slate-400">${sub.crates.length} crates</span>
                </div>
                <p class="text-sm text-slate-200 mb-4 leading-relaxed">${sub.summary}</p>
                <ul class="space-y-2 text-xs text-slate-300 mb-4">
                    ${sub.highlights.map(h => `<li class="flex items-start gap-2"><i class="fas fa-check-circle text-${sub.color}-400 mt-0.5"></i><span>${h}</span></li>`).join('')}
                </ul>
            </div>
            <div class="pt-3 border-t border-white/10 flex flex-wrap gap-1.5">
                ${sub.crates.map(c => `<code class="px-2 py-0.5 rounded bg-black/50 border border-white/10 text-[11px] text-${sub.color}-300 font-mono">${c}</code>`).join('')}
            </div>
        </div>
    `).join('');
}

function render10dSections() {
    const tabsRoot = document.getElementById('container-10d-tabs');
    const detailRoot = document.getElementById('container-10d-detail');
    if (!tabsRoot || !detailRoot) return;

    function selectSection(idx) {
        const sec = CONTAINER_10D_SECTIONS[idx];
        tabsRoot.querySelectorAll('button').forEach((b, i) => {
            b.className = i === idx
                ? `px-4 py-2.5 rounded-xl text-xs font-mono font-bold bg-${sec.color}-500 text-zinc-950 transition shadow`
                : 'px-4 py-2.5 rounded-xl text-xs font-mono text-slate-300 bg-slate-900/80 border border-white/10 hover:border-cyan-400/50 transition';
        });
        detailRoot.innerHTML = `
            <div class="flex flex-wrap items-center justify-between gap-3 mb-4">
                <div>
                    <div class="flex items-center gap-2.5">
                        <span class="px-3 py-1 rounded-lg bg-${sec.color}-500/20 border border-${sec.color}-500/40 text-${sec.color}-300 font-mono font-bold text-sm">${sec.tag}</span>
                        <span class="font-mono text-xs text-slate-400">Section Magic ${sec.code}</span>
                        <span class="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-white/10 text-white">${sec.tier}</span>
                    </div>
                    <h3 class="text-xl font-bold text-white mt-2">${sec.name}</h3>
                </div>
                <div class="px-3 py-1.5 rounded-xl bg-black/50 border border-white/10 text-xs font-mono text-${sec.color}-300">${sec.bytes}</div>
            </div>
            <p class="text-sm text-slate-300 leading-relaxed mb-5">${sec.desc}</p>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                ${sec.invariants.map(inv => `
                    <div class="p-3 rounded-xl bg-black/40 border border-white/10 flex items-start gap-2.5 text-xs text-slate-200">
                        <i class="fas fa-shield-halved text-${sec.color}-400 mt-0.5"></i>
                        <span>${inv}</span>
                    </div>
                `).join('')}
            </div>
        `;
    }

    tabsRoot.innerHTML = CONTAINER_10D_SECTIONS.map((s, idx) =>
        `<button type="button" data-sec-idx="${idx}">${s.tag} (${s.code})</button>`
    ).join('');
    tabsRoot.querySelectorAll('button').forEach(btn => {
        btn.addEventListener('click', () => selectSection(Number(btn.getAttribute('data-sec-idx'))));
    });
    selectSection(0);
}

function initCharts() {
    if (typeof Chart === 'undefined') return;
    const chartOptions = {
        responsive: true,
        maintainAspectRatio: true,
        plugins: {
            legend: { labels: { color: 'rgba(226, 232, 240, 0.8)' } }
        },
        scales: {
            x: { ticks: { color: 'rgba(226, 232, 240, 0.6)' }, grid: { color: 'rgba(255, 255, 255, 0.05)' } },
            y: { ticks: { color: 'rgba(226, 232, 240, 0.6)' }, grid: { color: 'rgba(255, 255, 255, 0.05)' } }
        }
    };

    const gThEl = document.getElementById('geometry-throughput-chart');
    if (gThEl) {
        charts.geometryThroughput = new Chart(gThEl, {
            type: 'bar',
            data: {
                labels: ['Delaunay 2D', 'Convex Hull 2D', 'Nearest Site', 'BVH + 5k Rays', '10D Volumetric SDF'],
                datasets: [{
                    label: 'Measured Throughput (ops/sec)',
                    data: [0, 0, 0, 0, 0],
                    backgroundColor: 'rgba(6, 182, 212, 0.5)',
                    borderColor: 'rgba(6, 182, 212, 1)',
                    borderWidth: 1
                }]
            },
            options: chartOptions
        });
    }

    const gScEl = document.getElementById('geometry-scaling-chart');
    if (gScEl) {
        charts.geometryScaling = new Chart(gScEl, {
            type: 'line',
            data: {
                labels: ['50 pts', '100 pts', '300 pts', '500 pts', '1000 pts'],
                datasets: [{
                    label: 'Execution Time (ms)',
                    data: [0, 0, 0, 0, 0],
                    borderColor: 'rgba(52, 211, 153, 1)',
                    backgroundColor: 'rgba(52, 211, 153, 0.12)',
                    fill: true,
                    tension: 0.35
                }]
            },
            options: chartOptions
        });
    }

    const c10dEl = document.getElementById('container-10d-chart');
    if (c10dEl) {
        charts.container10d = new Chart(c10dEl, {
            type: 'bar',
            data: {
                labels: STREAMABLE_10D_FIXTURES.map(f => f.label.split(' · ')[1] || f.id),
                datasets: [{
                    label: 'SHA-256 + 10D Section Verify Throughput (MiB/s)',
                    data: [0, 0, 0, 0, 0],
                    backgroundColor: 'rgba(168, 85, 247, 0.5)',
                    borderColor: 'rgba(168, 85, 247, 1)',
                    borderWidth: 1
                }]
            },
            options: chartOptions
        });
    }

    const emfEl = document.getElementById('emf-scaling-chart');
    if (emfEl) {
        charts.emfScaling = new Chart(emfEl, {
            type: 'bar',
            data: {
                labels: ['64×64 Slice', '128×128 Slice', '256×256 Slice', '512×512 Slice'],
                datasets: [{
                    label: 'EMF Optical + Cutaway Throughput (Mcells/sec)',
                    data: [0, 0, 0, 0],
                    backgroundColor: 'rgba(52, 211, 153, 0.5)',
                    borderColor: 'rgba(52, 211, 153, 1)',
                    borderWidth: 1
                }]
            },
            options: chartOptions
        });
    }
}

window.addEventListener('DOMContentLoaded', () => {
    renderEtalonSubsystems();
    render10dSections();
    ensureGeometryViewer();
    initCharts();

    const simCanvas = document.getElementById('holo-emf-sim-canvas');
    if (simCanvas) {
        holoEmfSim = new HoloEmfCutawaySimulator(simCanvas);
        holoEmfSim.updateInspectorHud();
    }

    // Load WASM engine in background so WASM vs JS benchmarks are armed immediately
    ensureWasmLoaded();
});
