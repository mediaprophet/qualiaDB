---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# chora Index

## Functionality Overview

Implements the client-side Chora spatio-temporal commons: persistent many-world
configurations, active world/time navigation, region queries, signed planted-asset
synchronisation, flagship world seeds, permissive data-layer catalogues, and compilation
of layer geometry into sealed `.10d` assets for the native renderer.

## File & Subdirectory Manifest

- `api.rs`: Adds Chora methods to `WebizenHostApi`, including world CRUD, active-world and
  temporal state, spatio-temporal region queries, asset streaming, render-surface
  description, and signed plant/pull operations.
- `asset_pipeline.rs`: Downloads or generates supported open-data layers, builds coloured
  meshes, seals them as provenance-bearing `.10d` containers, and returns renderer-ready
  buffers.
- `flagship_worlds.rs`: Defines and seeds History, Biosphere, Council, SDG, and GLAM world
  configurations.
- `layers/`: Layer catalogue, NASA GIBS acquisition, star fields, and deterministic mesh
  generators. See `layers/DIRECTORY_INDEX.md`.
- `mod.rs`: Chora module boundary and public re-exports.

## Changelog

- **2026-07-29**: Created a semantic index for the Chora world, layer, asset, and navigation
  implementation during the Webizen capability/UI naturalisation audit.
