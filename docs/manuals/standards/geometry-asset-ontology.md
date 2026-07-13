# Geometry-asset ontology & SHACL surface

**Status:** normative schema (draft v0, 2026-07-04). The step-0 schema the GLB→`.10d`/q42 compiler targets.
**Place in the pipeline (Timothy's ordering):** geometry-asset **ontology + SHACL surface** *(this doc)* → the
`.10d`/q42 GLB processing method (compiler) → mesh conversion (anatomy GLBs) → the 3-D anatomy build.
**Extends, does not reinvent:** the `urn:qualia:geometry:*` namespace already defined in
`crates/qualia-core-db/src/render/assets.rs` (`Mesh`, `vertexCount`, `triangleCount`, `sourceFormat`, `bbox*`,
`centroid*`).
**Reuses, not a parallel layer:** validation compiles through the existing SHACL-extensions machinery
(`modalities/logic/shacl_extensions`, the `*_shacl.rs` `Configuration → to_opcodes → SLG-VM` pattern) — this doc
adds shapes + a `crates/qualia-core-db/shapes/geometry-asset.shacl.ttl`, not a new constraint engine.
**Serialization:** q42 / CBOR-LD at rest (the manifest); the `.10d` container for the dense compiled geometry.

---

## 1. Two layers — the semantic manifest (q42) and the compiled container (`.10d`)

A compiled geometry asset is **two coupled artifacts**, and the ontology describes both:

| Layer | Holds | Form |
|---|---|---|
| **q42 semantic manifest** | identity, provenance, units/frame, counts/bounds, component semantics, render↔analysis correspondence, sensitivity/consent, and a **content-hash reference to the `.10d` file** | NQuins / CBOR-LD (semantic control plane) |
| **`.10d` compiled container** | the **dense geometry** — quantized vertices/indices, the Tensor10D node projection, topology adjacency, the spatial index, and (later) analysis meshes + fields | the `.10d` sections in `crate::container_10d` |

**The manifest never inlines dense vertices; the `.10d` never holds semantic policy.** They are joined by the
`.10d` file's whole-file CRC-32C / content hash, cited in the manifest (`compiledDigest`). This is the hybrid
established across the design docs: q42 = scene/semantic layer, `.10d` = the compiled sidecar it references.

## 2. Vocabulary (classes + predicates)

IRIs are `urn:qualia:geometry:*`, hashed with `q_hash` (extends the existing constants in `render/assets.rs`).

**Classes** — `Mesh` *(exists)* · `Primitive` · `VertexAttribute` · `Material` · `Lod` · `TopologyFeature` ·
`AnalysisMesh` · `Field` · `CompiledContainer` (the `.10d` file as a subject).

**Core predicates** *(★ = already in `render/assets.rs`)*:
`rdf:type` ★ · `vertexCount` ★ · `triangleCount` ★ · `sourceFormat` ★ · `bboxMin{X,Y,Z}` ★ · `bboxMax{X,Y,Z}` ★ ·
`centroid{X,Y,Z}` ★ · `sourceDigest` (immutable source GLB/OBJ/STL hash) · `compiledDigest` (the `.10d` file
hash — the join) · `unit` (e.g. `metre`) · `handedness` (`right`/`left`) · `upAxis` · `coordinateFrame` ·
`hasPrimitive` · `hasMaterial` · `hasLod` (+ `lodLevel`, `lodError`) · `hasAttribute` (+ `attributeSemantic` ∈
{position, normal, tangent, uv, color, jointWeights}) · `hasTopologyFeature` (+ `featureKind` ∈ {halfEdge, bvh,
kdTree, meshlet, adjacency}) · `renderMeshOf` / `analysisMeshOf` + `correspondenceMap` (render↔analysis vertex
map) · `hasField` (+ `fieldQuantity`, `fieldLocation` ∈ {node, cell, voxel}, `timeStep`) · `sensitivityClass`
(from `wellfare-core::SensitivityClass` — health data never falls to an unrestricted overlay) · `fidelityTier`
(F0–F4) · `assuranceClass` (A0–A4) · `sectionOffset` / `sectionType` (locates a class's bytes in the `.10d`).

## 3. Mapping each class to its `.10d` section (the compiled target)

This is the concrete contract the compiler emits into, using Devin's built sections:

| Ontology class | `.10d` section (`crate::container_10d`) | Status |
|---|---|---|
| `Mesh` dense geometry (`Primitive` vertices/indices) | **`QuantizedMesh`** — `mesh_section::{encode_mesh_section, Mesh}` (u16-quantized in bbox, u16/u32 indices) | **built (P0.4)** |
| Semantic node projection of the mesh into 10-D | **`Tensor10DNodes`** — `node_section` (AoS/SoA, the 40-byte atom) | **built (P0.5)** |
| `TopologyFeature` (half-edge / adjacency / manifold) | **topology section** — `topology_section` | **built (P2.8)** |
| `TopologyFeature` (BVH / kd-tree) for scan-free picking | **spatial-index section** — `spatial_index_section` (mmap-loadable) | **built (P3.7)** |
| `AnalysisMesh` (surface/volume cells for solvers) | `.10d` `AnalysisMesh` section | **spec-reserved** (section-type slot exists; encoder pending) |
| `Field` (dense scalar/vector/tensor over nodes/cells/time) | `.10d` `Field` section | **spec-reserved** |
| `CompiledContainer` | the whole `.10d` file (header + section table + CRC-32C) — `section::{encode_container}` | **built (P0.1–P0.3)** |

So an anatomy organ GLB compiles to **one `.10d` file** carrying the quantized mesh + topology + spatial-index
sections, **plus** a q42 manifest whose `compiledDigest` cites that file. LODs (from `decimate_3`, P5.7 —
verified) become additional `QuantizedMesh` sections tagged `hasLod`/`lodLevel`/`lodError`.

## 4. The q42 asset manifest (what the compiler asserts)

Per compiled asset, valid-parity NQuins asserting: asset identity + `sourceDigest` + `compiledDigest`; units /
handedness / up-axis / coordinate-frame; counts + bounds + centroid; per-`Primitive` attribute set + material
slot; the `hasLod` chain with error certificates; the `TopologyFeature` set present in the `.10d`; render↔analysis
`correspondenceMap` where an analysis mesh exists; `sensitivityClass`; `fidelityTier`/`assuranceClass`; and — for
anatomy — the component→semantic-id link (FMA/SNOMED IRIs attached to **component ids, not overloaded Tensor10D
axes**, retiring the desktop prototype's axis-overloading).

## 5. SHACL validation surface

`crates/qualia-core-db/shapes/geometry-asset.shacl.ttl` (+ a `geometry_asset_shacl.rs` `Configuration →
to_opcodes` shim, mirroring `specialized_libs_shacl.rs`) enforces, fail-closed:

- finite coordinates; vertex indices in `[0, vertexCount)`; `triangleCount·3` index entries; counts ≤ the `.10d`
  `MAX_VERTEX_COUNT`/`MAX_TRIANGLE_COUNT` (2²²) — malicious-size guard;
- **valid NQuin parity** on every emitted fact (the `subject ^ predicate ^ object ^ context` fold — the historic
  `mesh_to_nquins` parity-zero bug must never recur);
- `compiledDigest` present and matching the `.10d` whole-file CRC-32C (the manifest cannot cite a container it
  doesn't hash);
- required `unit` + `coordinateFrame` (no unit-less geometry into a solver);
- manifold/watertight **diagnostics recorded** (via `surface_mesh_processing` measures + `topology`), never
  silently asserted as "safe";
- `sensitivityClass` present for any health-derived asset (fail-closed: absent ⇒ most-restrictive).

## 6. Fidelity/assurance + rights hooks (not decoration)

`fidelityTier` (F0 asset … F4 coupled) and `assuranceClass` (A0 exploratory … A4 regulated) are **required**
fields, independent axes — a heavy mesh is never auto-"validated". A geometry asset derived from a rights-tagged
source **inherits the most-restrictive `sensitivityClass`** of its inputs (the same high-water-mark rule as the
dispatch layer). Anatomy assets carry health sensitivity by default.

## 7. What the GLB→`.10d`/q42 compiler (step 2) must produce

Acceptance for the next step, against this schema:

1. `import_glb` honours real glTF **accessor layout** (offsets/strides/component types) — not "positions start at
   BIN byte 0" — reading `POSITION`, and where present `NORMAL`/`TANGENT`/`TEXCOORD`/indices/node-transforms.
2. The immutable **source GLB is content-addressed** (`sourceDigest`); a **page-aligned `.10d`** is emitted
   (`QuantizedMesh` + `topology` + `spatial-index` sections) with a stable `compiledDigest`.
3. **Valid-parity** q42 manifest per §4, citing `compiledDigest`, validated by the §5 SHACL surface.
4. The renderer consumes the `.10d` sections (`decode_mesh_section` → `upload_10d_mesh`) **without reparsing the
   GLB**; picking uses the spatial-index section (scan-free).
5. LOD chain via `decimate_3` (P5.7) serialized as tagged `QuantizedMesh` sections.
6. Deterministic: identical source → byte-identical `.10d` + `compiledDigest` (attestable).

## 8. Honest status

- **This doc:** the schema, unblocked, docs-only. Extends the real `urn:qualia:geometry:*` namespace and pins to
  Devin's **already-built** `.10d` sections (mesh/node/topology/spatial-index) — not vaporware.
- **Spec-reserved (named, not built):** `AnalysisMesh` + `Field` `.10d` sections (the digital-twin tier).
- **Next:** (a) the `geometry-asset.shacl.ttl` + the `Configuration` shim (machine-checkable half); (b) the
  compiler extension (§7) in `render/assets.rs` — **qualia-core-db lane, coordinate via NOTICES**; (c) the anatomy
  GLB meshes remain the standing ⚑ (the CCF/HRA VH-Male library, Timothy's to supply) for the end-to-end run.
