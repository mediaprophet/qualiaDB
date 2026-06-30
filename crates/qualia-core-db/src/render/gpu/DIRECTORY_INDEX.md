---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# gpu Index

## Functionality Overview
Comprehensive index of functionality for `gpu`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bloom.rs`
- 📄 `mod.rs`
  - `struct BloomParamsGpu`
  - `struct CompositeParamsGpu`
  - `struct BloomUniformBlock`
  - `struct BloomChain`
  - `struct MeshGpu`
  - `struct PortalGpu`
  - `impl PortalGpu`
  - `fn new_offscreen`
  - `fn try_new_async`
  - `fn from_device`
  - `fn upload_tensor_buffer`
  - `fn tensor_node_count`
  - `fn set_artefact_joint`
  - `fn set_artefact_world`
  - `fn artefact_refused`
  - *(...and 24 more)*
- 📄 `particles.rs`
  - `fn particle_cap_for_mode`
- 📄 `resources.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
