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

## Changelog

- **2026-07-04**: Created the computational-geometry foundation and comprehensive index.
