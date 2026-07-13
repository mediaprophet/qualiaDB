# Native computational geometry & 3D creation for the QualiaDB / Webizen 10D ecosystem

**Status:** full-implementation plan (2026-07-04).
**Supersedes the naming in**
[`../reports/qualia-computational-geometry-gpu-review.md`](../reports/qualia-computational-geometry-gpu-review.md).
**Harmonized with** [`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md),
[`../manuals/computational-3d-assets-and-digital-twins.md`](../manuals/computational-3d-assets-and-digital-twins.md),
and [`native-auditory-language-and-music-intelligence.md`](native-auditory-language-and-music-intelligence.md)
— see §12 for the dependency map and shared-stance coordination.
**Native destination:** `crates/qualia-core-db/src/specialized_libs/computational_geometry/`.
**Container format:** `.10d` (the 10-D volumetric tensor file — retires the `g16` and the erroneous legacy
`Q42M` pre-release mesh build artifact; the normative byte-level layout is in §4.0, implemented P0.1–P0.6;
`.10d` is the compiled geometry/analysis-mesh/field sidecar of the 3D-assets manual, see §12.1).

## 0. What this is, and what it is *not*

This is the **native geometric-operations substrate of the QualiaDB / Webizen 10-D tensor**
`[q, v, w, x, y, z, t, α, μ, σ]` — computational geometry not as a 3-D mesh library, but as the engine that
makes the tensor's own promise real: *queries become geometric projections, distances, temporal slices,
context collapses, and spectral blends*. It runs on the filtered-predicate kernel, the `.10d` container, Q42
semantic identity, P64 weights, the `wgpu`/WGSL-Forge accelerator, and the native/browser renderer, and is
exposed through an ergonomic API so that **people can make things — spatial, spectral, probabilistic,
geometric — in the browser and in qapps**, on a sovereign, human-centric, offline-first, attestable stack.
3-D creation (the three.js-class maker surface) is one prominent face of it, not the whole.

It is deliberately positioned as **a native alternative to a whole category of tools at once**: the 3D-web
creation layer (three.js and friends), the geometry-processing/kernel layer (research-grade
computational-geometry capability), and the CAD/mesh-authoring layer — none of which are sovereign, zero-heap,
provenance-bearing, rights-aware, or fused
with a semantic/epistemic engine. This is the union those tools never form.

**The functionality-specification reference is a textbook, not a codebase.** The algorithm families, their
correctness properties, and their degeneracy behaviours are specified against **de Berg, Cheong, van Kreveld &
Overmars, *Computational Geometry: Algorithms and Applications* (3rd ed.)** — a public, authoritative
description of the algorithms, used as the spec and the source of correctness invariants, **never** as a source
of code. No third-party library source is copied or derived from. The output in `computational_geometry/` is an
independent, clean-room implementation on a **substantially different** foundation: a different number model,
array/index (not pointer-chasing) data structures, a zero-heap memory discipline, GPU-and-WASM execution, 10-D
projection, `.10d`/Q42/P64 integration, and a browser creation API. The textbook's chapter structure (convex
hulls, line-segment intersection & the doubly-connected edge list, polygon triangulation, orthogonal range
searching, Voronoi diagrams, Delaunay triangulations, arrangements, 3-D convex hulls, Minkowski sums, …) is a
**coverage checklist**, not a transliteration map; modules are organized around this ecosystem's needs.

## 1. Geometry across the whole manifold (why this reaches every dimension)

The 10-D tensor unifies semantics, the **EMF/spectral payload**, and **epistemic/probabilistic** state into
one manifold, so the *same* geometric kernel serves all of them. This is scope (not yet implemented), and it
is why the library is foundational rather than a rendering add-on:

- **Spectral / EMF geometry (visual, audio, and the full spectrum).** A spectrum `S(λ)` or `S(f)` is a
  geometric object — a curve/point in spectral space. The operations the `[α, μ, σ]` payload needs are
  geometric: **metamers** (many spectra → one CIE XYZ) are an **affine fibre** of the colour-matching
  projection; a **gamut / object-colour solid** is a **convex polytope**, and gamut-mapping is closest-point
  projection onto a convex body; the σ **spectral blend** is interpolation on the spectral manifold, not a
  naïve RGB lerp; CIE colour-matching is a linear projection; **audio STFT/CQT** is a time-frequency
  **surface** (the CQT a log-frequency reparameterization), on which pitch-shift, source separation, and the μ
  guard-band / WDM partitioning are geometric edits. One geometric spectral engine across visual + audio +
  IR/UV, instead of ad-hoc per-modality code.
- **Probability / inference geometry.** Probability distributions form **statistical manifolds** (information
  geometry: the Fisher metric, KL as a Bregman divergence); a categorical distribution lives on the
  **probability simplex**. Density and interpolation are **Delaunay / Voronoi / natural-neighbour** and the
  **CkNN** construction that converges to the **Laplace–Beltrami** operator — exactly what the
  gravito-thermodynamic baking needs for manifold consistency. The **`v`-class** (Euclidean / cyclic /
  hyperbolic / clique) and the "topological baking" are **topological data analysis** — alpha complexes and
  persistent homology over the point cloud. kNN inference is a nearest-neighbour **geometric query** (the
  spec's "distance < threshold ⇒ related, zero graph traversal"); the **`q`** what-if branches and the
  GSR/QUBO ground-state are geometric region-selection and energy-minimization. Probabilistic inference here
  *is* geometry.
- **Compute composition.** The kernel both leverages and feeds the other libraries: a predicate **is** a
  determinant (`specialized_libs` linear algebra), a transform **is** a matrix op; the half-edge / adjacency
  neighbourhoods become gather/scatter indices for **graph-ML / GNN message passing**; the spatial indexes
  bound candidate sets for **retrieval and sparse attention**; the `.10d` fields feed **physics / FEM**; and
  all of it runs on the shared `wgpu`/Forge substrate with CPU/WASM oracles. Geometry is a hub in the compute
  web, not a leaf.

The practical consequence for this plan: the kernel, spatial indexes, and simplicial-complex machinery in §§3,
7 are **shared across the spatial, spectral, and probabilistic faces** — a spectral-operator family and a
TDA / information-geometry family are first-class alongside mesh operations, not follow-ons.

## 2. Why not just use three.js / an existing geometry kernel

| Existing tool | What it gives | Why it is not enough here |
|---|---|---|
| **three.js** | Browser scene-graph + rendering (JS, WebGL/WebGPU) | Not sovereign; JS/heap; not a geometry kernel (no robust predicates, triangulation, boolean, meshing); no semantic identity, provenance, rights, or offline-first attestation; not integrated with a knowledge engine. |
| **Research C++ geometry kernels** | Mature geometry-processing algorithms | C++ template metaprogramming; copyleft source; allocation-heavy exact arithmetic; GPU-hostile; no browser path; no semantic/provenance/rights integration; not a creation surface. |
| **CAD / mesh libs** (OpenCASCADE, libigl, …) | Specific geometry pipelines | Same gaps: not browser-native, not sovereign, not fused with the 10-D/Q42/P64/rights engine, not an authoring platform for ordinary people. |

The native library is the **intersection none of them occupy**: a geometry kernel with research-grade robustness,
three.js-class in-browser creation ergonomics, and the 10-D/Q42/P64/renderer/rights engine underneath — so a
person can build spatial and computational things through a qapp, offline, on hardware they own, and keep the
result as their own provenance-bearing asset.

## 3. The native data plane (substantially different from existing kernels by construction)

- **Kernel / number model.** A `GeometryKernel` trait with an associated field type. Default: **filtered
  `f64` predicates** (a static error filter on the common path; a compensated / bounded-expansion exact path
  only near degeneracy — zero-heap hot path, GPU-executable, and *exact on the f32-sourced Tensor10D path* by
  construction). An exact-construction kernel (bounded rational/expansion workspaces, caller-supplied) sits
  behind the same trait for the few operations (polyhedral boolean/corefinement) that require exact cascaded
  constructions. **Determinism is a contract**, not an option: identical input → bit-identical combinatorial
  output on every platform, so a geometry result is hashable, WAL-able, and attestable. This is the opposite of
  the platform-and-instantiation-dependent behaviour of template-based kernels, and it is *required* by this
  ecosystem's provenance model.
- **Primitives.** `Point2`/`Point3` are POD (`repr(C)`, bytemuck) `f64` values; predicates return a
  three-valued sign. Already implemented: the filtered/compensated `orientation_2` and its Tensor10D `(x,y)`
  projection. To add: `orient3d`, `incircle`, `insphere`, and the general-`f64` exact fallback.
- **Data structures — array/index, not pointer-chasing.** Traditional handle/circulator combinatorial maps are
  heap-resident and cache-hostile. Ours are **caller-buffered, array/index-based, GPU-uploadable**: the 16-byte
  POD `HalfEdge` with a caller-owned open-addressing edge table (implemented, with full degeneracy / manifold /
  non-manifold / duplicate detection); surface-mesh, polygon-soup, and adjacency views built the same way.
- **10-D projection is first-class.** `encode_topology_features_10d` already maps a mesh's topology (position,
  degree, boundary, variety class `v`, domain `w`, time `t`, `α`/`μ`/`σ`) into `Tensor10D` records consumable
  by the renderer's projector, graph-ML, retrieval, and P64 projection. Geometry and semantics travel together;
  geometry is never serialized as fake model weights.

## 4. The `.10d` container (the on-disk substrate)

The volumetric file format is `.10d` — a **self-describing, chunked, spatially-indexed, compressible,
property-bearing** container for the 10-D tensor and its geometry/field payloads. It retires the `g16`
placeholder and the erroneous legacy `Q42M` pre-release mesh build artifact (the quantized triangle mesh
becomes one *section type* inside a `.10d` file). Design:

- **Section table** — `(offset, len, element_stride, dtype, alignment_tier, semantic_role, quant_descriptor,
  crc32c)` per stream; self-describing, no external schema.
- **Tiered alignment**, each derived from a consumer: **16 B** GPU vec/element · **64 B** header + SIMD +
  cache line · **256 B** GPU-stageable section start (`minStorageBufferOffsetAlignment`) · **page** for
  mmap-able sections (declare it; default 16 KiB for Apple-Silicon portability). Portable `wgpu` **stages**
  (copies) sections to GPU buffers — the format optimizes for one-shot staging + cheap decode-on-upload, not a
  non-portable direct mmap→bind.
- **Numeric role per stream** — `Quantized(u16, bbox_frame)` (render), `F32` (GPU-resident), or `F64` (robust
  analysis). This is where the *render-mesh vs analysis-mesh* split lives: one format, distinct numeric roles.
- **Compression, geometry-aware and deterministic** — per-chunk filter chain: quantize → Morton/Hilbert
  reorder → predict (parallelogram / Edgebreaker-class) → entropy; index codecs (delta+entropy). Must preserve
  canonical/deterministic bytes (attestable) and decode cheaply on stage. Reference algorithms are permissively
  licensed and reimplemented natively, license-clean.
- **Persisted spatial index** — BVH / kd / octree section + Morton order, built once at compile time and
  mmap-loaded, so ray / point-in-region / frustum / nearest queries run without a full scan and drive
  chunk-selective loading.
- **Self-description + hierarchy** — an attribute dictionary and a hierarchical node tree (`body → system →
  organ → component`) mapping onto the anatomy structure and the Q42 identity graph; address and load subtrees.
- **Fields / materials / units** — per-vertex/per-cell scalar/vector/tensor fields (stress, temperature,
  displacement…) with dtype + units + tensor-rank + time-step; named material regions with properties +
  provenance + uncertainty. The digital-twin / physics substrate.
- **10-D projection descriptor** — how the geometry embeds into the manifold (which lanes carry `x,y,z,t,…`)
  and which projection is canonical for which consumer (Volume3D render, an n-D physics/manifold view), so the
  projector and physics kernels read it directly.
- **Integrity / provenance** — CRC-32C per section + a whole-file content hash + Q42 identity linkage; LE,
  versioned, capability-flagged; canonical deterministic encoding (byte-identical for identical input).

### 4.0 Normative byte-level layout (v1 — implemented P0.1–P0.6)

This is the **executable spec** — the byte-level layout the conformance harness
(`container_10d/conformance.rs`) pins with golden vectors and a layout-drift gate. If any field offset,
encoding order, or CRC algorithm here drifts, the conformance test breaks and the format version MUST bump.
The design-level §4 above is the intent; this subsection is the contract. All offsets are little-endian.

#### 4.0.1 Container header (64 bytes, `Container10dHeader`)

```text
offset  size  field                  type        notes
0       4     magic                  [u8;4]      b"10d\0" = [0x31, 0x30, 0x64, 0x00]
4       2     version                u16 LE      = 1 (v1)
6       2     flags                  u16 LE      bit 0 = default-disposition-Refuse (set by proposed())
8       10    axis_roles             [u8;10]     Option A: q,v,w=Selector(1); x,y,z,t,α,σ=Coordinate(2); μ=Carrier(4)
18      2     pad0                   [u8;2]      must be zero
20      32    metric_descriptor      [u8;32]     4 × MetricBranchDescriptor (8B each) — see 4.0.2
52      4     header_crc32c          u32 LE      whole-file CRC-32C with this field zeroed during compute (P0.3)
56      4     section_table_offset   u32 LE      byte offset of the section table from file start
60      4     section_count          u32 LE      number of SectionDescriptor entries in the table
```

`Container10dHeader::proposed()` produces the normative default header (magic, version=1, flags=Refuse,
Option A axis roles, the honest metric descriptor matching `full_distance`'s four v-branches). The golden
bare-header bytes + pinned CRC-32C `0xD6DDABF5` are in `conformance.rs::GOLDEN_BARE_HEADER`.

#### 4.0.2 Metric branch descriptor (8 bytes, `MetricBranchDescriptor`, ×4 in the header)

```text
offset  size  field          type    notes
0       1     v_class        u8      0 / 1 / 2 / 255 (255 = the v≥3 catch-all branch)
1       1     metric_kind    u8      1=Euclidean, 2=Cyclic, 3=Hyperbolic, 4=BoundaryClique
2       2     folded_axes    u16 LE  bitmask of axis indices folded out of this branch's distance sum
4       4     reserved       [u8;4]  must be zero
```

The four branches pin `full_distance`'s actual v-branch behaviour (the "queryability claim == code" gate,
P0.1): v=0 Euclidean folds x,y,z,t,α,μ,σ (q,v,w only); v=1 Cyclic folds x,y,z; v=2 Hyperbolic folds x,y,z;
v≥3 BoundaryClique folds nothing. `verify_descriptor_against_reality` enforces this at parse time — a
descriptor claiming queryability the kernel contradicts is rejected.

#### 4.0.3 Section table (N × 24 bytes, `SectionDescriptor`)

Located at `section_table_offset`; `section_count` entries. The writer sorts entries by `section_type`
(canonical encoding — permuted input produces byte-identical output) and rejects duplicates.

```text
offset  size  field            type    notes
0       1     section_type     u8      1=QuantizedMesh, 2=Tensor10DNodes, 3=Reconstruction (implemented);
                                       4–9 = SpecReserved* (defined for forward-compat, writer rejects today)
1       1     alignment_tier   u8      1=Word(4B), 2=CacheLine(16B), 3=Page(64B), 4=GpuVec(16B)
2       2     reserved16       u16 LE  must be zero
4       4     byte_offset      u32 LE  payload offset from file start (must be tier-aligned)
8       4     byte_length      u32 LE  payload length in bytes
12      4     stride           u32 LE  element stride (0 = packed/implicit)
16      4     element_count    u32 LE  element count (0 = packed/implicit)
20      4     crc32c           u32 LE  CRC-32C over the payload bytes (P0.2 per-section integrity)
```

Reader gates (P0.2): overlapping sections, out-of-bounds, misaligned, stride-inconsistent, non-zero
reserved16, non-zero padding, and per-section CRC mismatch are all rejected.

#### 4.0.4 CRC-32C (RFC 3721, pinned `0xE3069283`)

`container_10d/crc32c.rs` — the shared CRC-32C implementation. Pinned to RFC 3721's polynomial `0xE3069283`.
Used two ways: (a) per-section `crc32c` in each `SectionDescriptor` (P0.2); (b) whole-file content hash in
`header_crc32c`, computed with the `header_crc32c` field itself zeroed (P0.3, `integrity.rs`). The whole-file
CRC is the seal; `verify_whole_file_crc32c` is the gate. `p64_weight.rs` delegates to this same module
(byte-identical p64 round-trip verified after delegation).

#### 4.0.5 QuantizedMesh section (P0.4 — `container_10d/mesh_section.rs`)

`section_type = 1`. Payload = 40-byte mini-header + quantized vertex data + triangle indices.

```text
offset  size  field            type        notes
0       2     flags            u16 LE      bit 0 = FLAG_U16_INDICES (else u32); all other bits must be zero
2       2     reserved_u16     u16 LE      must be zero
4       4     vertex_count     u32 LE      ≤ 2^22 (MAX_VERTEX_COUNT)
8       4     triangle_count   u32 LE      ≤ 2^22 (MAX_TRIANGLE_COUNT)
12      12    min              [f32;3]     dequantization frame: position = min + (q/65535)*(max-min)
24      12    max              [f32;3]     (bbox recomputed from positions at encode time — faithful frame)
36      4     reserved_u32     u32 LE      must be zero (future: LOD tier, material index)
40      6×N   vertices         [u16;3]×N   u16-quantized per axis within [min,max] (6 bytes/vertex vs 12 f32)
40+6N   6×T   triangles        [u16;3]×T   if FLAG_U16_INDICES; else [u32;3]×T (12 bytes/tri)
```

`MAX_VERTEX_COUNT` / `MAX_TRIANGLE_COUNT` = 2^22 (4M each) — bounds against hostile files; the practical
ceiling is the 42MB Sentinel (40MB / 6 bytes ≈ 6.7M vertices, so 4M is comfortable). Quantization error is
`bbox_extent / 65535` per axis — sub-micron at organ scale, visually lossless. Deterministic (two encodes
byte-identical). **This replaces the erroneous legacy pre-release mesh build artifact** (`render/mesh_asset.rs`
— `Q42M` magic / `encode_mesh_q42` / `decode_mesh_q42` / `MeshBufferHeader`), which was refactored out
entirely with no backward-compat (never shipped, an erroneous build artifact). The 40-byte mini-header is 8
bytes smaller than the legacy 48-byte header (no per-format magic — the section-type tag replaces it; no
per-format version — the container version replaces it).

#### 4.0.6 Tensor10DNodes section (P0.5 — `container_10d/node_section.rs`)

`section_type = 2`. Payload = 16-byte mini-header + node records (the 40-byte epistemic atom).

```text
offset  size  field            type        notes
0       4     node_count       u32 LE      ≤ 2^20 (MAX_NODE_COUNT = 1M × 40B = 40MB, under the 42MB Sentinel)
4       1     layout           u8          0 = AoS, 1 = SoA (any other value rejected)
5       1     reserved_u8      u8          must be zero (future: q-superposition render/export mask)
6       2     reserved_u16     u16 LE      must be zero (future: per-axis SoA lane offset table)
8       8     reserved_u64     u64 LE      must be zero (future: GSR-result back-pointer)
16      …     node records     see below   layout-dependent
```

**AoS layout** (`layout = 0`): N × 40-byte `Tensor10D` records back-to-back — the natural `Tensor10D` layout,
identical to `tensor/buffer_export.rs::write_tensor_buffer` minus its `Q42*` header (which the `.10d`
container header replaces). Each record is `[q,v,w,x,y,z,t,α,μ,σ]` as ten `f32` LE values.

**SoA layout** (`layout = 1`): ten contiguous lanes, one per axis in `AXIS_ORDER` order (lane 0 = all `q`
values, lane 1 = all `v`, …, lane 9 = all `σ`). The "page-friendly" layout where any single axis is a
contiguous strided read. Each lane is `node_count × f32` LE.

`write_node_section_aos` / `write_node_section_soa` (zero-heap, caller-buffered); `read_node` (dispatches on
layout); `read_node_soa_lane` (per-axis lane read); `transpose_aos_to_soa` / `transpose_soa_to_aos`
(out-of-place, zero-heap — the in-place transpose of a 10×N f32 matrix needs N*40 scratch, so the honest
zero-heap primitive is the out-of-place path); `write_node_q_at` (wavefunction-collapse semantics matching
`buffer_export::write_tensor_q_at` — returns the previous `q`, works on both layouts, Sandbox `q=999` handled).
**Spec-reserved (NOT yet implemented):** the mini-header reserved fields for a per-axis SoA lane offset
table, a q-superposition render/export mask (Sandbox nodes not citable as provenance until collapsed), and a
GSR-result back-pointer — governance/attestation layer, not the atom. The parser rejects non-zero reserved
today (fail-closed).

#### 4.0.7 Conformance vectors (P0.7 — `container_10d/conformance.rs`)

The executable spec's double lock: (1) `assert_layout_invariants()` — centralized size + `offset_of!` gate
for every field in `Container10dHeader` (64B / 12 fields), `MetricBranchDescriptor` (8B), `SectionDescriptor`
(24B / 8 fields), `NodeMiniHeader` (16B / 5 fields), `MeshMiniHeader` (40B / 8 fields), plus the format
constants (`HEADER_BYTE_SIZE=64`, `SECTION_DESCRIPTOR_SIZE=24`, `NODE_MINI_HEADER_SIZE=16`,
`MESH_MINI_HEADER_SIZE=40`, `TENSOR10D_SIZE=40`). (2) Golden vectors — pinned byte sequences + pinned CRC-32C
hashes, with `encode∘decode = identity` asserted: `GOLDEN_BARE_HEADER` (CRC `0xD6DDABF5`),
`GOLDEN_NODE_ONLY_CRC` (`0x6865D565`), `GOLDEN_MESH_ONLY_CRC` (`0x18B5DD86`). If any field offset, encoding
order, or CRC algorithm drifts, the golden vector won't reproduce and the test breaks; if the golden bytes
are silently edited, the pinned hash won't match.

### 4.1 The capabilities that *define* `.10d` (not "a better glTF")

`.10d` is a **semantic-epistemic substrate that also renders geometry** — a living, witnessed, rights-bearing
record, not a mesh format with metadata bolted on. Static mesh containers have no axis on which certainty,
consent, knowledge-domain, or time-of-knowledge could even be written; `.10d` stores those **as coordinates in
the same 40-byte record as position**. Most of this is **already implemented** in the `qualia-core-db` tensor
/ render stack (marked ✅); the container's job is to *serialize a live runtime*. Each capability names the
`.10d` format requirement it imposes:

- **The 40-byte epistemic atom (the format's soul)** ✅ `Tensor10D`. Normative node record is the fixed
  `[q,v,w,x,y,z,t,α,μ,σ]` stride, page-friendly and **structure-of-arrays** so any single axis is a contiguous
  strided read.
- **Axis-role taxonomy (normative header — the prerequisite for honest queryability).** `.10d` MUST declare
  each axis as a **COORDINATE** that participates in distance (`x,y,z,t,α,μ,σ`), a **SELECTOR** excluded from
  every distance sum (`v,w,q`), or a **CARRIER** (`μ`, the in-band provenance lane). *Honest limitation to
  encode:* today only the `v=0` Euclidean branch of `full_distance` folds the full coordinate set; the
  cyclic/hyperbolic/clique branches use `x,y,z`/byte-equality alone. The spec must either make the
  non-Euclidean metrics axis-complete or document this — never assert queryability the kernel contradicts.
- **Per-region non-Euclidean physics selected per query by `v`** ✅ `full_distance` (Euclidean / toroidal /
  hyperbolic / clique). `v` is a first-class selector byte; the GPU volume-search kernel ships all four metric
  branches under a normative **GPU==CPU determinism** guarantee.
- **Fail-closed civic refusal** ✅ `render/authoring.rs::plan_view`. A normative **Governance section**:
  per-view `{manifold_id, kind, sensitivity, requires_attestation}`; an embedded expiry-stamped deontic-norm
  table of 48-byte NQuins (byte-identical to `deontic.rs`); a mandatory header flag **default disposition =
  Refuse** so a reader that ignores the section still fails closed. Sensitivity class rides in the NQuin
  context bits under the parity fold, so a derived/decimated view inherits the **max (most restrictive)** class
  of its sources arithmetically — stripping it corrupts parity.
- **Living epistemic state (`q` superposition → GSR collapse → new `t`-slice)** ✅ `quantum.rs`, `gsr.rs`,
  `write_tensor_q_at`. `q` is per-node; reserved states `{0=GroundTruth, ParallelContext, InEscrow,
  ≥999=Sandbox}`; render/export default to a ground-truth-only mask; a GSR-result section keyed by
  `problem_id` (content hash) records the non-destructive collapse; **Sandbox nodes are not citable as
  provenance until collapsed** (a machine-checked property).
- **`α` = confidence + gravitational bake-mass + HDR light-energy + audio gain, one lane** ✅ `bake_pipeline.rs`.
  Header records the `α→mass` map and bake parameters (temperature/diffusion, pressure/density) so a re-bake is
  reproducible; the format **must not** record a baked white point / tone curve as ground truth (last-mile
  device policy). A re-bake provenance section stamps each generation with its `t`, the evidence delta, and a
  back-pointer to the prior generation.
- **One `σ` spectral truth → vision AND audition, rights-gated per standpoint** ✅ `spectral.wgsl`,
  `acoustic.rs`, `telemetry.rs`. `σ` is a first-class `f32` lane (not a texture ref), with the λ-mapping and
  fract/integer semantics fixed normatively; high-density SPD (vision) / STFT-CQT (audio) rasters live in
  content-addressed mmap **sidecar sheets** (shared 20-byte header), with a 64-bin inline preview; per-node
  `σ/μ` sensitivity classes let the projector lawfully attenuate detail per standpoint.
- **`μ` welds provenance + consent into the signal (the canonical provenance lane).** Normative bit-layout
  (DID-ref / consent flags / parity), preserved bit-exact through quantization/transcode; any LOD/decimation
  **must re-derive `μ` parity**, not drop it (detectable-corruption, not silent downgrade). Compact
  DID+consent+parity preview rides the stride; full signatures in an mmap provenance sidecar bound to the Q42
  deontic/consent plane. (Streaming, spectral, and multi-agent capabilities *reference* this lane, not restate
  it.)
- **Append-only `t`-ledger with native "what was known at time X"** ✅ `q42_integration.rs::temporal_query_into`,
  `kv_provenance.rs`. A **temporal-index section** over the `t` lane (bucketed offsets) makes a time-window
  scrub O(log n); the container declares a time base (state-version vs wall-clock); the ledger is an
  append-only run of frame records `{identity, t, parent_page_id, author-DID-ref}`, and the format **forbids
  in-place mutation of any node whose `t` is sealed**. This one ledger also carries attestations and re-bake
  generations.
- **Cross-`w` systemic *proposals, not diagnoses* (the WellFair reason-to-exist).** A **manifold-head table**
  (per active `w`: offset/count + optional stored `w_i→w_j` projection matrix as a small dense `f32` block for
  GPU upload); a cross-`w` projection descriptor declaring which head-pairs are legal to project and by which
  rights rule, with per-`w` consent binding so medical→public fails closed. Systemic implications are written
  as `q>0` nodes (`requires_attestation=true`, `α` confidence, `μ/t` provenance); **no global merge exists** —
  cross-`w` conflict is surfaced as a query, never baked into geometry.
- **Digital-twin dual-mesh (F0–F4 / A0–A4).** Wire `fea_mesh_index_id` to a real analysis-mesh sidecar +
  a page-aligned, independently-checksummed **correspondence-map** section (render-primitive ↔ analysis-element,
  `w` selecting mesh domain); a field sidecar keyed by the `σ` physical-class code with `{dtype, SI-unit vector,
  frame, validity-domain}` + per-cell `α/μ` and **F/A** labels; the runtime rejects a dimensionally-inadmissible
  or out-of-validity read before a solver/clinician sees it.

## 5. The API & creation layer (the "make things" surface)

This is what turns a geometry kernel into a **creation platform**, and it is a first-class deliverable, not an
afterthought. The same operation contract is exposed at four levels:

1. **Rust** — `qualia_core_db::specialized_libs::computational_geometry`, caller-buffered and zero-heap on the
   hot path.
2. **Browser / WASM** — the `wasm-scientific` build exposes the kernel and creation API in the browser, driving
   the **native renderer** (PortalGpu / `webizen-render`, the 10-D display engine). Wiring the browser WebGPU
   `<canvas>` mount (currently missing — see the GPU review) is part of this layer.
3. **qapp / MCP / agent** — the `computational_geometry` MCP tool and the `run_computational_geometry` qapp
   command, so qapps and agents invoke geometry ops; capability manifests express per-op resource limits and
   whether an op is scalar / SIMD / wgpu / CUDA / exact-fallback.
4. **Renderer SDK** — `.10d` mesh upload, colour-by-field, integer picking, LOD/temporal scrub — the display
   half of the 10-D engine.

### 5.1 Native-first dispatch (not WASM-diminished)

**The dispatch rule:** when a local native installation is present, the engine **must use the full capabilities
of the local software environment** — native SIMD, native `wgpu` (Vulkan / Metal / DX12), optional CUDA, `f64`
exact-arithmetic fallback, `mmap`-able sidecar sheets, direct GPU buffer staging — rather than being routed
through the WASM path in a way that would diminish performance. **WASM is the browser/fallback target, not a
performance ceiling imposed on native.** The four levels above are *targets selected by where the code is
running*, not a single lowest-common-denominator path:

- **Native desktop / edge-native build** → the Rust level runs at full native capability (SIMD, `f64` exact,
  native `wgpu` adapters, CUDA where present, `mmap` sidecars). The `.10d` container and the geometry kernel
  are compiled `no_std`-clean and ungated for WASM, but on native they dispatch to the heaviest acceleration
  the host offers. This is the default for any installation that has a native binary.
- **Browser / no native binary** → the `wasm-scientific` WASM build is the real target, driving the browser
  WebGPU canvas. This is the sovereign-browser path — full capability *for the browser*, not a diminished
  version of native. The WASM build is the same code, compiled for `wasm32-unknown-unknown`, with platform
  features that are unavailable on WASM (`mmap`, `f64` exact in some paths, CUDA) falling back to their
  portable equivalents.
- **Headless / no adapter** → the CPU scalar oracle is the real target, not a stub. Determinism is preserved.

The conformance vectors (§4.0, P0.7) are the **byte-identical decode guarantee across all targets**: a `.10d`
file written on native decodes byte-identically on WASM, and vice versa. The *execution* path differs (native
uses full acceleration; WASM uses browser WebGPU; headless uses CPU), but the *bytes* do not. This is the
honest separation: **one format, one byte-stream, target-appropriate execution** — never native-diminished-to-
WASM, never WASM-only.

On top of these sits an **authoring / creative ergonomics API** — the three.js-alternative surface: scene
construction, primitive generation, transforms, boolean and mesh operations, procedural generation, and
interaction (pick / drag / edit), all through the sovereign stack, so a qapp author or an ordinary maker can
build and manipulate 3-D / spatial / geometric scenes and keep the result as their own `.10d` asset with
provenance.

## 6. Acceleration (from the GPU review)

Selected by algorithm shape, not category. **Good Forge candidates:** batched orientation/incircle predicates
(uncertain cases compacted for exact CPU fallback), AABB/Morton generation, distance/winding/ray batches,
point classification, nearest-neighbour and spatial joins, mesh transforms/normals/curvature, and the
scan/sort/compaction stages of hull/triangulation/BVH builders. **Hybrid:** hull and Delaunay (GPU
partition/filter, deterministic CPU merge + exact fallback); boolean/corefinement (GPU broad phase, CPU robust
topology edit). **CPU-first:** exact constructions, small dynamic topology edits, branch-heavy degeneracy
handling. Every accelerated op requires a scalar/caller-buffered oracle, a typed Forge kernel, Naga
validation, CPU/GPU differential vectors including degeneracies, adapter-keyed tuning, and a deterministic
CPU/WASM fallback. Robust topology never accepts silent `f32` disagreement.

## 7. Implementation phases (dependency-ordered, native — not textbook-chapter order)

- **P0 — Kernel & primitives.** `GeometryKernel` trait; filtered `f64` + exact fallback; `orient2d`
  (done), `orient3d`, `incircle`, `insphere`; POD primitives. *Foundation exists; extend.*
- **P1 — Topology & mesh structures.** Half-edge (done), surface mesh, polygon soup, adjacency/BGL-style views,
  combinatorial maps as needed.
- **P2 — Spatial query layer.** BVH / kd / octree, AABB trees, Morton order, distances, intersections,
  nearest-neighbour — persisted into `.10d`.
- **P3 — 2-D algorithms.** Convex hull (done: `convex_hull_2`), Delaunay + constrained/conforming
  triangulation, Voronoi, Minkowski / boolean.
- **P4 — 3-D algorithms.** Hulls, Delaunay / **tetrahedralization**, surface-mesh processing, boolean /
  corefinement, remeshing, and **decimation/simplification with error metrics** (the anatomy LOD tier and the
  `.10d` compression LODs; feeds the `authoring.rs` budget → 3-D→2-D degradation, the accessibility rail).
- **P5 — Reconstruction & meshing.** Point-set processing, alpha shapes / wrap, isosurfacing, mesh generation,
  and the **density-aware / Laplace–Beltrami-consistent construction** the gravito-thermodynamic baking needs
  for manifold consistency.
- **P6 — API & creation layer.** The browser/WASM API, the WebGPU canvas mount, the renderer-SDK integration,
  the qapp/MCP surface, and the authoring ergonomics — the three.js-alternative maker surface (§5).

Cross-cutting throughout: Forge/GPU acceleration, WASM parity, golden-oracle validation, and
determinism/attestation.

## 8. Verification, validation, and license discipline

- **Golden oracles.** The de Berg et al. textbook is the algorithm spec; its stated correctness properties and
  degeneracy behaviours are the oracle. Native algorithms are validated against independently-constructed golden
  vectors and first-principles invariants (strong-convexity postconditions, empty-circumcircle /
  empty-circumsphere, simplex-class partitions, exact arbitrary-precision cross-checks) — a clean-room
  implementation validated against public algorithm descriptions, never derived from any library's source.
- **Differential + determinism.** CPU scalar oracle per op; CPU/GPU differential including degeneracies;
  canonical/deterministic bytes for `.10d` (hash-stable → attestable); Naga validation for shaders.
- **Targets.** Native default, native CUDA (optional), browser `wasm-scientific`, portal WebGPU, Vulkan/Metal/
  DX12 overrides, and headless/no-adapter — each an explicit gate.

```powershell
cargo test -p qualia-core-db computational_geometry --lib
cargo test -p qualia-core-db mcp_server --lib
cargo check -p webizen-render -p webizen-desktop
cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-scientific
```

## 9. Honest status (2026-07-04)

- **Real, in tree:** the filtered/compensated `orientation_2` (exact on the f32-tensor path), allocation-free
  `convex_hull_2` over `Point2` and over the Tensor10D `(x,y)` projection (returns source indices), the 16-byte
  half-edge topology with a caller-owned workspace and full degeneracy/manifold checks,
  `encode_topology_features_10d`, one typed GPU orientation batch with a CPU oracle, and the MCP/qapp routes.
- **`.10d` container v1 — implemented (P0.1–P0.7 scaffold):** the 64-byte `Container10dHeader` (magic, version,
  flags, Option A axis roles, honest metric descriptor matching `full_distance`'s four v-branches) + the
  self-describing section table (24-byte `SectionDescriptor`, tiered alignment, canonical encoding, per-section
  CRC) + shared CRC-32C (RFC 3721, pinned `0xE3069283`) + whole-file content hash + the QuantizedMesh section
  (P0.4 — 40-byte mini-header, u16-quantized vertices in bbox, u16/u32 indices, **replaces the erroneous legacy
  `Q42M` build artifact** which was refactored out entirely with no backward-compat) + the Tensor10DNodes
  section (P0.5 — 16-byte mini-header, AoS + SoA layouts, `write_node_q_at` wavefunction-collapse semantics) +
  the renderer upload path (P0.6 — `upload_10d_mesh` consuming the `.10d` mesh section) + the conformance
  harness (P0.7 — layout-drift gate + golden vectors with pinned CRCs, `encode∘decode = identity`). **92
  container_10d tests green** (28 P0.1 + 20 P0.2 + 10 P0.3 + 13 P0.4 + 16 P0.5 + 5 P0.7); `render --lib` 99
  passed; `cargo check -p webizen-render -p webizen-desktop` green; WASM lib gate green
  (`cargo check --target wasm32-unknown-unknown --no-default-features --features wasm-scientific`). The
  normative byte-level layout is in §4.0; the native-first dispatch rule is in §5.1.
- **Not there yet — do not report as complete:** `orient3d`/`incircle`/`insphere` and the general-`f64` exact
  fallback; BVH/kd/octree spatial index; Delaunay / triangulation / Voronoi; 3-D hulls / tetrahedralization;
  boolean / corefinement / remeshing; mesh decimation; reconstruction & meshing (incl. the baking's
  density-aware construction); the `.10d` capabilities beyond the implemented sections (compression, persisted
  spatial index, hierarchical node tree, fields/materials/units, the Governance / temporal-index /
  correspondence-map / provenance sidecar sections — §4.1); the browser WebGPU canvas mount and the
  authoring/creation API; the WASM conformance-vector runtime half (P0.8 — gated on a pre-existing
  `getrandom`/`wasm_js` test-harness issue, not a `container_10d` issue). This is a real container foundation
  + a real kernel foundation for one package family, not the library or the platform.

## 10. Governance & ethos

The geometry substrate carries the same discipline as the rest of the ecosystem: **sovereign, offline-first,
no silent egress**; **deterministic decisions → attestable geometry**; `.10d` assets carry provenance and Q42
identity; qapp-authored creations are the maker's own, with their licence and lineage recorded, never
silently the platform's. Fidelity and assurance stay separate — a heavier geometric or physical computation
never labels itself "safe," "certified," or "clinically valid" without the evidence and competent-human
review that class requires. The point of the whole thing is to let people *make*, on their own terms, on
hardware they own, without handing their work to anyone.

## 11. Build methodology — swarm implementation

The library is ~a hundred well-scoped, mostly-independent functions, each with a **textbook specification**
(the de Berg et al. chapter/section for the algorithm) and a **first-principles golden oracle** (its stated
correctness properties). That is the ideal shape for a **swarm of sub-agents**, using the proven
*isolated-files-plus-integrator* pattern from this repo's prior multi-agent work.

**Per-agent contract.** Each sub-agent takes one unit from the textbook's algorithm-coverage checklist and:
1. reads the **gist** — the textbook description as the spec and its stated correctness properties as the
   oracle (never any library's source);
2. implements it **natively in Rust on the `GeometryKernel` trait**, in **one isolated new file** (disjoint
   file sets across agents, so they never collide), caller-buffered and zero-heap on the hot path;
3. ships its own `#[cfg(test)]` **validated against the golden vectors**, including the degenerate /
   near-degenerate cases, plus a CPU oracle for any GPU op.

**Integrator (main loop = me).** Owns the shared surface — the `GeometryKernel` trait, `mod.rs` re-exports,
the `.10d` container — and for every landed unit: compiles green, runs the golden oracle tests, and
**adversarially re-verifies correctness**. This last step is non-negotiable for geometry: a single wrong
predicate sign yields *invalid topology*, not a small numeric error, so swarm output is **not trusted until**
it passes degeneracy vectors, CPU/GPU differential, and the determinism / canonical-bytes check. Coordination
via `NOTICES.md` claims; no agent is spawned into a live hand-edited lane.

**Order.** The dependency-ordered phases (§7) set the fan-out: the kernel and core structures are a small
barriered first wave (everything depends on them); after that, spatial index, 2-D, 3-D, and
reconstruction/meshing functions parallelize widely within each phase. The spectral-operator and
TDA/information-geometry families (§1) fan out the same way once the kernel and simplicial machinery exist.

**Phase 0 — the `.10d` refactor (coherent, not piecemeal).** Land the `.10d` container + the
`Q42M`→`.10d` rename across the renderer upload path as the foundation every agent writes into. **(2026-07-04:
DONE — P0.1–P0.7 implemented; the erroneous legacy `Q42M` build artifact refactored out entirely; the
normative byte-level layout is §4.0; 92 container_10d tests green. The remaining P0.8 WASM conformance-vector
runtime half is gated on a pre-existing `getrandom`/`wasm_js` test-harness issue, out of P0 scope.)** The
fan-out below (P1–P6) now builds on this landed foundation.

**Cost & trigger.** A swarm of this size is token-expensive and explicitly opt-in: this section is the
**plan**; execution (spawning the agents) is triggered on your word, per phase, so scale is always a
deliberate choice.

## 12. Harmonization with the companion plans (visual, auditory, 3D-assets)

This plan does not stand alone. Three companion documents define requirements the computational-geometry
substrate must serve, and this section pins the dependencies so the four plans move coherently rather than
colliding on the same files:

- [`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md) —
  image understanding, native image generation, image-to-3D, compiled spatial assets, tiered digital twins.
- [`../manuals/computational-3d-assets-and-digital-twins.md`](../manuals/computational-3d-assets-and-digital-twins.md)
  — the architecture/capability manual for 3D assets and digital twins: the compiled-asset-bundle
  architecture and the two-axis F/A tier model.
- [`native-auditory-language-and-music-intelligence.md`](native-auditory-language-and-music-intelligence.md)
  — acoustic event understanding, speech/language, music analysis/production, audio generation, and shared
  "eyes and ears" perception.

### 12.1 `.10d` IS the compiled geometry / analysis-mesh / field sidecar

The 3D-assets manual (§3.1) and the visual plan (Phase 9) call for a **content-addressed, page-aligned,
checksummed, GPU/SIMD-friendly compiled geometry sidecar** linked from Q42 — distinct from Q42 (semantic
control plane) and P64 (model weights). **The `.10d` container is that sidecar.** The mapping:

| 3D-assets manual artifact (§3.1) | `.10d` section type | Status |
|---|---|---|
| Geometry sidecar (vertex streams, indices, primitive ranges, hierarchy, material slots, LODs, meshlets/BVH, adjacency, hashes) | `QuantizedMesh` (P0.4, §4.0.5) + future `BVH`/`Meshlet`/`Adjacency`/`Hierarchy` sections | QuantizedMesh implemented; spatial-index/meshlet/adjacency sections are P2.7/P2.8 |
| Analysis-mesh sidecar (surface/volume cells, node/element IDs, groups, constraints, material regions, quality, source-geometry correspondence) | Future `AnalysisMesh` section + the `correspondence-map` section (§4.1 digital-twin dual-mesh) | Not yet implemented — visual plan Phase 11 dependency |
| Field sidecar (dense scalar/vector/tensor results over nodes/cells/voxels/time/spectral) | Future `Field` section (§4.1 fields/materials/units) | Not yet implemented — visual plan Phase 11 dependency |
| Semantic/spatiotemporal projection of an asset/observation/result | `Tensor10DNodes` (P0.5, §4.0.6) | Implemented |

The manual's invariant properties for sidecars (§3.1: content digest + exact byte length; schema
version + endian + scalar type + units + coordinate frame; 4 KiB/page-friendly section offsets with checked
bounds; independently checksummed sections; SoA views; bounded counts + overflow-safe parsing; immutable
source-to-compiled lineage; stable component/primitive/element IDs shared with Q42) are **the same
invariants the `.10d` container enforces** (§4.0: 64-byte versioned header, 24-byte section table with
tiered alignment, per-section CRC-32C, SoA node layout, MAX_VERTEX/MAX_TRIANGLE/MAX_NODE bounds,
canonical/deterministic encoding). The `.10d` v1 spec (§4.0) is the executable contract for the manual's
sidecar architecture.

**Q42 ↔ `.10d` linkage.** Q42 holds the semantic/control plane (asset/scene/component/material/LOD
identities, units, coordinate frame, load/BC references, solver receipts, provenance — manual §3.2). The
`.10d` container holds the dense geometry/field bytes. The linkage is by content hash: the Q42 manifest
records the `.10d` file's whole-file CRC-32C (§4.0.4) and per-section CRCs, so a Q42 asset record resolves
to a byte-verified `.10d` payload. The manual's `fea_mesh_index_id` field (currently silently zeroed) must
be wired to this linkage or superseded by a versioned Q42 predicate that points at the `.10d` analysis-mesh
section — a visual-plan Phase 9/11 deliverable that depends on the `.10d` AnalysisMesh section existing
first.

### 12.2 The two-axis F/A tier model applies to geometry outputs

The 3D-assets manual (§4) defines two orthogonal axes every computational run must declare: **fidelity
tier** (F0 Asset → F4 Coupled/high-fidelity) and **assurance class** (A0 Exploratory → A4
Safety/regulated support). **The computational-geometry kernel's outputs carry both axes.** A geometric
claim — "this mesh is manifold," "this point is inside this hull," "this decimation preserved topology" —
is an F-tier computation with an A-class evidence requirement:

- **F0 Asset** — convex hull, AABB, picking, component graph, basic geometry queries. The implemented
  `convex_hull_2` / `orientation_2` / half-edge topology are F0. A0 (exploratory) minimum; A1
  (reproducible) is free because the kernel is deterministic.
- **F1 Interactive** — collision/bounds admission, coarse kinematics, BVH broad-phase. The P2 spatial
  query layer targets F1.
- **F2 Analytical** — screening computation: stress invariants, fatigue estimates, 1-D thermal. Geometry
  feeds F2 by providing the analysis-mesh correspondence map (§4.1 dual-mesh).
- **F3 Numerical** — mesh/grid simulation with convergence evidence. The P4/P5 mesh processing
  (decimation, reconstruction, remeshing) feeds F3 by producing the meshes F3 solvers run on.
- **F4 Coupled/high-fidelity** — nonlinear/transient/stochastic/multiphysics. Out of scope for the
  geometry kernel itself; the geometry kernel provides the substrate, the solver provides the F4 claim.

**The assurance axis is the geometry kernel's honesty gate.** A geometric predicate that returns
`Collinear` on a near-degenerate input under filtered `f64` is **A1 reproducible** but not **A2 verified**
unless the exact ladder (P1.4–P1.7) confirms it. The kernel's filtered → compensated → exact ladder
(§7, P1) is the mechanism that lifts a predicate from A1 to A2. A3 (validated) and A4 (safety/regulated)
require external evidence and competent-human sign-off the kernel cannot provide alone — the manual's
"software output alone does not certify" rule (§4.2). **The kernel must never label its own output A3/A4.**

### 12.3 Visual plan dependencies on computational geometry

The visual plan's Phase 9/10/11 directly require computational-geometry capabilities. The dependency map:

| Visual plan deliverable | Computational-geometry plan task | Status |
|---|---|---|
| Phase 9: mesh validation — manifold/watertight diagnostics, self-intersection, degenerate faces, winding | P2.5 (connectivity & invariants: components, boundary loops, Euler characteristic, genus) + P4 boolean/corefinement (self-intersection detection) | P2.1 half-edge done; P2.5 + P4 planned |
| Phase 9: real mesh simplification/decimation with error metrics + persistent LOD correspondence | P4 decimation/simplification with error metrics | Planned (P4) |
| Phase 9: BVH/meshlet/adjacency sections in the compiled geometry sidecar | P2.7 (GPU-stageability + `.10d` topology sections) + P3.3 (static BVH/AABB-tree) | Planned (P2.7, P3.3) |
| Phase 9: stable component/primitive/element IDs shared with Q42 + picking | P2.2 (surface-mesh view) + the `.10d` section's stable IDs | P2.1 half-edge done; P2.2 planned |
| Phase 10: mesh extraction, repair, decimation, UV/material bake from reconstruction | P5 (reconstruction & meshing: alpha shapes, wrap, isosurfacing) + P4 (repair/remeshing/decimation) | Planned (P4, P5) |
| Phase 11: `AnalysisMeshView`, `FieldView`, render↔analysis mesh correspondence map | `.10d` AnalysisMesh + Field + correspondence-map sections (§4.1 digital-twin dual-mesh) | Not yet implemented — §4.1 capability |
| Phase 11: surface/volume mesh schemas (triangle/quad/tetra/hexa cells, named sets, material regions) | P2.2 (surface-mesh view) + P5 (volume meshing/tetrahedralization) | Planned (P2.2, P5) |
| Phase 12: verified numerical solvers need mesh-convergence / model-form uncertainty | P4 decimation with error metrics (the geometric-error budget) | Planned (P4) |

**Sequencing implication.** The visual plan's Phase 9 (compiled spatial assets) is the first visual-plan
phase that blocks on computational geometry — specifically on P2 (topology & spatial query) for mesh
validation and P4 (3D algorithms) for decimation. The visual plan's Phase 11 (computational 3D /
digital-twin substrate) blocks on the `.10d` AnalysisMesh + Field sections (§4.1, not yet implemented).
The computational-geometry P1→P2→P3→P4→P5 phase order (§7) is the critical path that unblocks the visual
plan's Phase 9→10→11→12 in sequence.

### 12.4 Auditory plan dependencies — spectral geometry and shared perception

The auditory plan connects to computational geometry through two seams:

**Spectral geometry (§1 of this plan).** The auditory plan's STFT/CQT/partial/chroma/spectral-flux
features are **geometric objects on the time-frequency surface** — this plan's §1 calls out "audio
STFT/CQT is a time-frequency surface (the CQT a log-frequency reparameterization), on which pitch-shift,
source separation, and the μ guard-band / WDM partitioning are geometric edits." The computational-geometry
kernel's 2-D algorithms (P3: Delaunay/Voronoi on spectral points; convex hull of a gamut; closest-point
projection for metamers) serve the auditory plan's spectral analysis. The σ spectral lane in `Tensor10D`
is the shared coordinate — one spectral truth for vision and audition (auditory plan §3.1: "U2 and U3
derive visual and auditory projections from the same spectral signature"). The computational-geometry
kernel does not own the STFT/CQT computation (that's the auditory plan's streaming feature engine); it
owns the **geometric operations on the spectral surface** those features produce.

**Shared "eyes and ears" perception (auditory plan §11).** The auditory plan's shared media-clock,
audiovisual scene/event graph, and cross-modal temporal localization require a shared timeline. The
`.10d` container's `t` lane (the append-only `t`-ledger, §4.1) is the shared temporal coordinate a
visual-observation node and an auditory-observation node can both carry — the same `t`-index section
(§4.1 temporal-index) serves both modalities. The computational-geometry kernel's job here is narrow:
provide the spatial/temporal coordinate substrate (the `Tensor10D` `[x,y,z,t]` lanes + the `.10d`
temporal-index section) that the auditory plan's cross-modal correlation runs over. The correlation
logic itself is the auditory plan's; the geometry kernel provides the manifold.

### 12.5 Shared architectural stance — extend Qualia's substrate, not adopt an external framework

All four plans share the same architectural decision: **extend Qualia's native compute substrate (Forge,
wgpu, P64, Q42, `.10d`, the `GeometryKernel` trait) rather than adopt an external framework as the
production runtime.** The visual plan (§2.1) and auditory plan (§2.1) both reject Candle/Burn as the
production runtime for the same reason this plan (§2) rejects adopting an external geometry kernel as source: they bring a competing
tensor/device/memory/certification stack that weakens the substrate the project is strengthening. The
native-first dispatch principle (§5.1) is the same stance expressed at the dispatch layer: native uses
full native capability, WASM is the browser target not a performance ceiling. The four plans are
**one architecture with four faces** (geometry, vision, audio, 3D-assets), not four frameworks glued
together.

### 12.6 File-level coordination — where the plans touch the same code

The plans converge on a small set of shared files. Coordination via `NOTICES.md` claims; no agent is
spawned into a live hand-edited lane (§11 rule):

| Shared file / module | This plan's touch | Companion plan's touch |
|---|---|---|
| `container_10d/` (the `.10d` container) | Owns it — P0.1–P0.7 done, P2.7/P2.8 add topology/spatial-index sections | Visual plan Phase 9 consumes `.10d` as the compiled geometry sidecar; Phase 11 consumes AnalysisMesh/Field sections |
| `render/assets.rs` (GLB/OBJ/STL ingest) | P2.3 polygon-soup ingestion + repair feeds the canonical importer | Visual plan Phase 9 owns the canonical scene IR + GLB extension |
| `render/mod.rs` + `webizen-render/volumetric.rs` (renderer upload) | P0.6 `upload_10d_mesh` done; P2.7 GPU-stageability certifies the upload path | Visual plan Phase 9 extends renderer for preserved attributes/materials |
| `q42/` (semantic control plane) | `.10d` ↔ Q42 linkage by content hash (§12.1) | Visual plan Phase 9 wires `fea_mesh_index_id` / Q42 asset manifest; Phase 11 adds Q42 vocabulary for units/loads/BC/convergence |
| `specialized_libs/computational_geometry/` | Owns it — P1.2 kernel trait done; P1.3+ predicates, P2+ topology/spatial/3D/reconstruction | Visual plan Phase 11 `ComputationalAsset`/`AnalysisMeshView`/`FieldView` adapters wrap the geometry kernel's outputs |
| `tensor/Tensor10D` (the 40-byte atom) | The `[x,y,z,t,α,μ,σ]` coordinate the geometry kernel projects; the `.10d` node section serializes it | Auditory plan's σ spectral lane + U3 acoustic plane; visual plan's U2 visual plane — same atom, same lanes |
| `wgsl_forge/` (GPU compute DAG) | P1.9 GPU predicate batches + P3.6 GPU candidate-generation | Visual plan's Conv2D/Conv3D/resize/pooling/image-sample ops; auditory plan's streaming Conv1D/mel/overlap-add |

**The rule for concurrent work:** the `.10d` container, the `GeometryKernel` trait, and `Tensor10D` are
the integrator-owned shared surface (§11). Companion-plan work that touches these files coordinates through
`NOTICES.md` and the integrator (me), not by direct parallel edit. Companion-plan work that *consumes* them
(visual plan Phase 9/10/11, auditory plan's spectral-geometry ops) proceeds independently once the
geometry-plan deliverables they depend on are landed.
