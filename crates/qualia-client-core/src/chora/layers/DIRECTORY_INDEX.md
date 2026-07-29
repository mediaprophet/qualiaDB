---
created: 2026-07-29
updated: 2026-07-29
update_scope: Comprehensive
---

# layers Index

## Functionality Overview

Defines Chora's attributed permissive-commons layer catalogue and the bounded acquisition
or generation helpers that turn Earth, stellar, and planetary sources into renderer and
`.10d` inputs.

## File & Subdirectory Manifest

- `catalog.rs`: Layer definitions, categories, sources, licences, preview metadata, and
  catalogue lookup/filter helpers.
- `mesh_gen.rs`: Deterministic coloured sphere and related mesh generators used by Earth
  and planetary layers.
- `mod.rs`: Module routing and public layer exports.
- `nasa_gibs.rs`: NASA GIBS request, download, image decoding, and geographic texture
  sampling support.
- `starfield.rs`: Embedded bright-star data and deterministic synthetic star-field
  generation.

## Changelog

- **2026-07-29**: Created the layer-catalogue index and recorded source, attribution, mesh,
  and star-field responsibilities.
