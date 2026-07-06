---
created: 2026-07-04
updated: 2026-07-04
update_scope: Comprehensive
---

# computational_geometry Index

## Functionality Overview

Native Rust computational geometry for QualiaDB, structured around
caller-buffered CPU/WASM kernels, topology graphs, Tensor10D/ML feature bridges,
typed WGSL acceleration, qapp/MCP tools, and a generated CGAL package-port
registry.

## File & Subdirectory Manifest

- `generated/`: Deterministic CGAL 6.2 package inventory and port status registry.
- `features.rs`: Converts half-edge mesh topology and positions into Tensor10D graph/ML features.
- `gpu.rs`: Typed batch geometry WGSL generation and deterministic CPU/WASM oracles.
- `hull.rs`: Allocation-free 2D and Tensor10D spatial convex-hull kernels.
- `mod.rs`: Public computational-geometry ABI and exports.
- `primitives.rs`: POD points and robust/compensated orientation predicates.
- `tool.rs`: Cold JSON operation boundary shared by MCP and desktop qapps.
- `topology.rs`: Caller-buffered triangle half-edge graph construction and validation.
- `triangulation_2.rs`: Simple-polygon triangulation via monotone partition + linear monotone triangulation, with ear-clipping fallback.
- `point_location.rs`: Point location in planar subdivisions — walking location in triangulations and slab decomposition (O(log n) query).
- `convex_decomposition.rs`: Convex decomposition of simple polygons — Hertel-Mehlhorn (triangulate + merge convex pairs) and triangulation-only fallback.

## Changelog

- **2026-07-04**: Created the computational-geometry foundation and comprehensive index.
- **2026-07-27**: P11.5 — Implemented monotone partition (sweep-based decomposition into y-monotone sub-polygons), fixed `triangulate_monotone` to be truly O(n) with precomputed chain assignments, integrated monotone partition + monotone triangulation into `triangulate_polygon` with ear-clipping fallback. Fixed ear clipping bugs: (1) `point_in_triangle` now checks strictly interior (not boundary), (2) collinear vertices are now clipped as degenerate ears, (3) fallback no longer produces CW triangles. Fixed `verify_triangulation` to allow zero-area triangles from collinear vertices. 29 tests in `triangulation_2`, 984 tests in `computational_geometry` — all green.
- **2026-07-27**: P11.6 — Implemented point location in planar subdivisions: (1) `walk_locate` — walking location in triangulations with `LocateResult` enum (Inside/Outside/OnEdge/OnVertex), (2) `SlabMap` — slab decomposition with O(log n) query via two binary searches (slab + edge within slab), computing edge x at query y on the fly for correctness, (3) `triangulation_to_subdivision` — converts a triangle list to subdivision edges with twin matching for face adjacency. 21 tests in `point_location`, 1005 tests in `computational_geometry` — all green.
- **2026-07-27**: P11.7 — Upgraded Minkowski sum: (1) `minkowski_sum_convex` — O(n+m) edge-merge by polar angle for convex polygons (exact, no hull approximation), with collinear vertex removal, (2) `minkowski_sum_non_convex` — decomposes non-convex polygons into triangles (via P11.5 ear clipping), computes all pairwise convex Minkowski sums, returns union point set, (3) Updated capability manifest from ApproximateMetric to ExactPredicate (convex case is exact). 23 tests in `minkowski_2`, 1007 tests in `computational_geometry` — all green.
- **2026-07-27**: P11.3 — Implemented convex decomposition: (1) `convex_decomposition_hm` — Hertel-Mehlhorn algorithm (triangulate via P11.5, then merge adjacent triangles whose union is convex, using union-find for piece tracking), produces ≤4× optimal pieces in O(n) post-triangulation, (2) `convex_decomposition_triangulation` — triangulation-only fallback (every triangle is convex, ≤n-2 pieces), (3) `is_convex_polygon` — orientation-based convexity test, (4) `verify_convex_decomposition` — validates all pieces convex + area conservation. 23 tests in `convex_decomposition`, 1043 tests in `computational_geometry` — all green.
