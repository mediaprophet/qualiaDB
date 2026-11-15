---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# geospatial Index

## Functionality Overview
Comprehensive index of functionality for `geospatial`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `spatial.rs`
  - `struct SpatiotemporalQuadTree`
  - `impl SpatiotemporalQuadTree`
  - `fn query_region`
  - `fn embed_h3_context`
- 📄 `vector_scene.rs`
  - `struct VectorSceneFrame`
  - `VectorSceneFrame::with_tile_bounds`: set an optional rectangular local ENU clip region for streamed render tiles
  - `enum OsmVectorStyle`
  - `struct OsmRenderedFeature`
  - `fn compile_osm_feature_mesh`
  - `fn compile_osm_feature_mesh_draped`
- 📁 `vector_scene/`
  - `geometry.rs`: bounded ENU projection, DEM sampling, point markers, line ribbons, simple polygon meshes, and height-preserving triangle clipping to optional tile rectangles.

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
- **2026-10-11**: Added the Qualia-owned bounded OSM feature-to-mesh path with source identity retention, optional DEM draping, and optional ENU render-tile clipping.
