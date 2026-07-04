# QualiaDB native computational geometry — capability-reference workbench

Status: foundation implemented  
**Authoritative build plan:** [`native-computational-geometry.md`](native-computational-geometry.md)
(full implementation, the `.10d` container, the browser/renderer API, and the three.js-alternative creation
surface). This document is the **capability-reference workbench**: it tracks CGAL's package set as a
public-domain coverage checklist and CC0 test oracle so the native build has a completeness target.  
CGAL capability reference: CGAL 6.2 (`v6.2`, pinned commit recorded by the generator) — used as a **reference
inventory and CC0 golden-output oracle, never a source of derived code** (CGAL's algorithm source is
GPL/LGPL; its `doc/` and `test/` are CC0).  
Native destination:
`crates/qualia-core-db/src/specialized_libs/computational_geometry/`

## Objective

Build a complete computational-geometry capability **natively in Rust on the QualiaDB / Webizen engine** — a
substantially different implementation, not a transliteration of CGAL — using CGAL's package inventory only as
a coverage checklist, then optimize it for this ecosystem's actual execution environments:

- deterministic CPU and SIMD;
- browser/WASM;
- wgpu 29 and typed WGSL Forge schedules;
- Q42 graph/topology reasoning;
- `.10d` quantized mesh geometry;
- Tensor10D manifold operations;
- graph ML, spatial retrieval, and inference;
- P64-resident models and geometry-conditioned operators.

This is not a renderer utility project. Meshes, arrangements, triangulations,
cell complexes, spatial indexes, proximity structures, and topology are graph
data structures first. Rendering is one consumer.

## Reference workbench

`scripts/cgal-port/port_cgal.py` creates a deterministic inventory from the
pinned upstream tree. It currently records 133 package metadata units, their
dependency names, documentation/test/example counts, upstream licence labels,
and Qualia coverage status.

Generated files:

- `computational_geometry/generated/cgal_packages.rs` — compiled capability
  registry used by MCP/qapps;
- `resources/cgal-port/cgal-6.2-packages.json` — full continuation manifest.

Statuses are deliberately strict:

- `planned`: inventory only;
- `foundation`: shared operation exists but not every documented named API and
  conformance case is covered;
- `ported`: documented operation surface and applicable conformance cases are
  implemented and verified. (Status string as emitted by the generator script —
  see `scripts/cgal-port/port_cgal.py` `PortStatus::Ported`. A follow-up rename
  to `covered` / `PortStatus::Covered` would align the script's identifiers
  with the reference-not-port framing; flagged, not done in this pass.)

The generator does not claim that inventory equals implementation.

## Native data plane

### Predicates and primitives

`Point2` and `Point3` are POD values. Predicates use a fast filtered path and a
compensated near-degenerate path. Tensor10D spatial coordinates are `f32`, so
their products fit exactly in the `f64` predicate path for finite values.
Future exact constructions use bounded expansion/rational workspaces supplied
by the caller.

### Topology graphs

`HalfEdge` is a 16-byte POD record:

| field | role |
|---|---|
| `origin` | source vertex index |
| `twin` | reverse half-edge or `u32::MAX` |
| `next` | next edge around the face |
| `face` | incident face index |

`build_triangle_half_edges` uses a caller-owned open-addressing table. It has no
heap allocation, detects bad indices/degenerate faces/non-manifold directions,
and exposes boundaries explicitly. This structure is shared by mesh algorithms,
graph traversal, ML neighborhood construction, and renderer preparation.

### `.10d` container (geometry section)

The on-disk volumetric format is **`.10d`** (the 10-D volumetric tensor container; retires the `g16`/`Q42M`
placeholders — see [`native-computational-geometry.md`](native-computational-geometry.md) §3). Today only its
compact quantized-mesh **geometry section** exists: 48-byte header, quantized `u16` positions in a
bounding-box frame, and `u16` or `u32` triangle indices. Geometry algorithms operate on typed views; asset
boundaries encode/decode it; the renderer SDK accepts it directly. The full `.10d` container (section table,
chunking, compression, spatial index, fields/properties, hierarchy) is specified in the authoritative plan.

### Tensor10D features

`encode_topology_features_10d` preserves geometry and semantics in one record:

| Tensor10D lane | geometry/graph feature |
|---|---|
| `q` | epistemic/world branch |
| `v` | topology class; boundary clique currently uses `3` |
| `w` | domain/model head |
| `x,y,z` | spatial position |
| `t` | time/provenance slice |
| `alpha` | normalized vertex degree |
| `mu` | boundary flag |
| `sigma` | raw directed degree |

This is immediately consumable by the existing tensor projector and P64
projection kernels, while retaining source vertex identity.

## Acceleration policy

GPU acceleration is selected by algorithm shape, not by marketing category.

Good WGSL Forge candidates:

- batched orientation/incircle predicates with uncertain cases compacted for
  exact CPU fallback;
- AABB generation, Morton codes, distance fields, winding/ray batches;
- point classification, nearest-neighbor distance batches, and spatial joins;
- mesh transforms, normals, curvature estimates, and per-face reductions;
- scan/sort/compaction stages used by hull, triangulation, and BVH builders;
- adjacency gather/scatter, stencil, and neighborhood feature extraction.

Hybrid candidates:

- convex hull and Delaunay: GPU partition/filter, deterministic CPU merge and
  exact fallback;
- boolean/corefinement: GPU broad phase, CPU robust intersection/topology edit;
- arrangements and Nef structures: GPU candidate generation, CPU exact graph
  mutation.

CPU-first candidates:

- exact algebraic constructions;
- small dynamic topology edits;
- branch-heavy degeneracy handling;
- operations below measured dispatch thresholds.

Every accelerated operation needs:

1. the scalar/caller-buffered oracle;
2. a typed Forge kernel or graph;
3. Naga validation;
4. CPU/GPU differential vectors including degeneracies;
5. adapter-keyed tuning evidence;
6. a deterministic CPU/WASM fallback.

## ML, P64, and inference uses

The geometry reference work should feed existing model infrastructure instead of inventing
a second model container.

- Half-edge and cell-complex neighborhoods become gather/scatter indices for
  graph neural networks and geometric message passing.
- Spatial indexes provide bounded candidate sets for retrieval and sparse
  attention.
- Tensor10D geometry features can be projected with existing P64 kernels and
  combined with resident token/model embeddings.
- Point clouds and meshes can supply positional/context features for
  multimodal inference.
- Curvature, normals, topology class, components, and boundary state become
  deterministic features for training/evaluation.
- BVH/ray-query and proximity kernels can accelerate both rendering and
  geometry-conditioned inference.
- Geometry kernels and P64 weights remain distinct typed inputs: geometry is
  not serialized as fake model weights.

## Public surfaces

The same operation contract is exposed through:

- Rust:
  `qualia_core_db::specialized_libs::computational_geometry`;
- MCP: `computational_geometry`;
- desktop qapp host:
  `invoke("run_computational_geometry", { request })`;
- renderer SDK: `VolumetricRenderer::upload_10d_mesh` (consumes a `.10d` QuantizedMesh section).

Initial JSON operations are:

- `orientation_2`;
- `convex_hull_2`;
- `triangle_topology`;
- `package_inventory`.

JSON allocation occurs only at the explicit tool boundary. The underlying
algorithms keep caller-owned workspaces.

## Coverage sequence

1. Kernels and number types: predicates, constructions, interval/exact
   arithmetic, 2D/3D primitives.
2. Core graph structures: half-edge/surface mesh, polygon soup, BGL-compatible
   adjacency views, combinatorial/generalized maps.
3. Spatial query layer: AABB tree, boxes, kd/orth trees, distances,
   intersections, nearest-neighbor search.
4. 2D algorithms: polygons, hulls, arrangements, triangulation, Voronoi,
   Minkowski/boolean operations.
5. 3D algorithms: hulls, triangulation, surface mesh processing, booleans,
   remeshing, simplification.
6. Reconstruction and meshing: point-set processing, alpha shapes/wrap,
   isosurfacing, mesh generation.
7. Higher-dimensional and kinetic packages.
8. Visualization adapters only after the underlying graph operation is
   certified.

Within each step, dependencies in the generated manifest determine order.

## Verification commands

```powershell
python scripts/cgal-port/port_cgal.py --fetch
cargo test -p qualia-core-db computational_geometry --lib --no-default-features
cargo test -p qualia-core-db mcp_server --lib
cargo check -p webizen-render -p webizen-desktop
```

WASM verification is required as kernels are added:

```powershell
cargo check -p qualia-core-db --target wasm32-unknown-unknown `
  --no-default-features --features wasm-scientific
```
