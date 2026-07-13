---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `clock.rs`
  - `struct FixedStepClock`
  - `impl FixedStepClock`
  - `fn new`
  - `fn timestep`
  - `fn accumulator`
  - `fn push_elapsed`
  - `fn advances_only_on_full_ticks`
- 📄 `diffusion.rs`
  - `struct DiffusionConfig`
  - `impl DiffusionConfig`
  - `fn cell_count`
  - `fn raw_byte_len`
  - `struct DiffusionField`
  - `impl DiffusionField`
  - `fn new`
- 📄 `error.rs`
  - `enum RuntimeError`
  - `impl Display`
  - `fn fmt`
  - `impl Error`
- 📄 `kernel.rs`
  - `trait ComputeBackend`
  - `fn step`
  - `fn reconfigure`
  - `fn shared_frames`
  - `struct LedgerRecord`
  - `impl LedgerRecord`
  - `fn from_snapshot`
  - `trait LedgerSink`
  - `fn record`
  - `struct ChannelLedgerSink`
  - `impl ChannelLedgerSink`
  - `fn bounded`
  - `impl LedgerSink`
  - `enum RuntimeCommand`
  - `struct NullLedgerSink`
  - *(...and 10 more)*
- 📄 `lib.rs`
- 📄 `snapshot.rs`
  - `enum FrameHandle`
  - `struct SharedFrameBuffer`
  - `impl SharedFrameBuffer`
  - `fn new`
  - `fn byte_len`
  - `fn with_slot`
  - `struct SimulationSnapshot`
- 📄 `wgpu_backend.rs`
  - `struct VertexInput`
  - `struct VertexOutput`
  - `fn vertex_main`
  - `fn fragment_main`
  - `struct DiffusionUniforms`
  - `fn idx`
  - `fn left_of`
  - `fn right_of`
  - `fn up_of`
  - `fn down_of`
  - `fn main`
  - `struct Vertex`
  - `struct WgpuDiffusionBackend`
  - `fn new`
  - `fn create_render_pipeline`
  - *(...and 6 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
