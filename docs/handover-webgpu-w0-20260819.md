# Handover: WebGPU Support — W0 Complete

**Date:** 2026-08-19 | **Branch:** `0.0.17-dev` | **Prev:** `handover-webgpu-plan-20260818.md`

---

## What Was Done

### 1. Plan Update (§7) — `docs/vibescript-full-impl-PLAN.md`
- Replaced WebGL2-only §7 with dual-track WebGPU + WebGL2 plan
- Added phases W0 (invoke surface), W3 (unified fallback), W6 (compute), W7 (hot-reload), W8 (detection)
- Added invoke ID table, dependency graph, verification criteria

### 2. W0: WebGPU capability.invoke Surface
**New file:** `crates/qualia-core-db/src/poet_host/invoke/render/gpu.rs`

12 invoke handlers wrapping `render::gpu::PortalGpu`:

| Invoke ID | Handler | PortalGpu method |
|-----------|---------|------------------|
| `Render.gpu_adapter_info` | `gpu_adapter_info` | `gpu_context::try_shared_gpu()` → `GpuAdapterCaps` |
| `Render.gpu_init` | `gpu_init` | `PortalGpu::new_offscreen(width, height, particle_cap)` |
| `Render.gpu_render_frame` | `gpu_render_frame` | `PortalGpu::render(time, &telemetry)` |
| `Render.gpu_read_pixels` | `gpu_read_pixels` | `PortalGpu::read_rgba8_into(&mut buf)` |
| `Render.gpu_upload_mesh` | `gpu_upload_mesh` | `PortalGpu::upload_mesh(positions, indices)` |
| `Render.gpu_upload_tensor` | `gpu_upload_tensor` | `PortalGpu::upload_tensor_buffer(bytes)` |
| `Render.gpu_set_camera` | `gpu_set_camera` | `PortalGpu::set_camera(yaw, pitch, zoom)` |
| `Render.gpu_pick` | `gpu_pick` | `PortalGpu::queue_pick(x, y)` |
| `Render.gpu_poll_pick` | `gpu_poll_pick` | `PortalGpu::poll_pick_readback()` |
| `Render.gpu_resize` | `gpu_resize` | `PortalGpu::resize(width, height)` |
| `Render.gpu_set_ambient` | `gpu_set_ambient` | `PortalGpu::set_ambient_enabled(bool)` |
| `Render.gpu_destroy` | `gpu_destroy` | slot removal |

**Design:** Slot-map (`Vec<Option<GpuSlot>>` with `Mutex`) manages `PortalGpu` instances by `u64` handle. Mutex poisoning recovery via `lock_slots()`. All handlers `#[cfg(not(target_arch = "wasm32"))]` with wasm stubs.

**Modified files:**
- `poet_host/invoke/render/mod.rs` — registered `gpu` module, re-exported handlers
- `poet_host/invoke/ids.rs` — added 12 `GPU_*` constants, wired into `ALL_BOUND` and `seam_for`
- `poet_host/invoke/mod.rs` — added 12 dispatch arms

### 3. Bug Fix: Pick Readback Alignment
**File:** `crates/qualia-core-db/src/render/gpu/mod.rs`

**Root cause:** `record_pick_copy` used `bytes_per_row: Some(4)` for 1-pixel pick texture copy. wgpu requires `bytes_per_row` aligned to `COPY_BYTES_PER_ROW_ALIGNMENT` (256 bytes).

**Fix (2 lines):**
- Line 695: staging buffer sized to `padded_bytes_per_row(1)` (256) instead of 4
- Line 1296: `bytes_per_row: Some(padded_bytes_per_row(1))` instead of `Some(4)`

Readback still reads only `mapped[0..4]` — padding bytes ignored.

### 4. Tests — 5 passing
- `g_gpu_adapter_info_returns_record` — adapter query via VibeScript
- `g_gpu_init_and_render_cycle` — init + render + destroy
- `g_gpu_set_camera` — camera orbit + render
- `g_gpu_resize_and_ambient` — resize + ambient toggle + render
- `g_gpu_pick_readback_alignment` — pick + render + poll (regression test for alignment fix)

Tests call handlers directly (VibeScript's `Member` expr is namespace resolution, not record field access).

---

## What's Next

### W3: Unified Invoke with WebGL2 Fallback (high)
- `Render.gpu_init` auto-detects backend; falls back to WebGL2/Naga path when WebGPU unavailable
- Requires wiring `render::anatomy::webgl2.rs` as alternate backend behind same invoke IDs
- May need a `backend` field in `gpu_init` return to indicate which path was selected

### W6: WebGPU Compute Pipeline Invoke (medium)
- Expose `wgsl_forge` compute dispatch for GPU physics
- New invoke IDs: `Render.gpu_compute_dispatch`, `Render.gpu_compute_readback`
- Wraps `wgsl_forge::execute::wgpu` execution path

### W7: Shader Hot-Reload (medium)
- Runtime WGSL recompilation without pipeline rebuild
- PortalGpu currently bakes shaders at construction; needs shader module swap

### W8: Backend Detection (medium)
- Runtime probe selecting WebGPU vs WebGL2 based on adapter availability
- `gpu_adapter_info` already reports availability; W8 makes `gpu_init` use it

### LLM Scripting (from plan §7)
- The plan also mentions "Advanced LLM Agent Interface" workstreams
- This involves VibeScript extensions for LLM agent scripting (prompt chains, tool use, multi-step reasoning)
- Not yet started; needs its own planning pass
- Related existing infra: `poet_host/invoke/` dispatch, `inference/` crate, `gguf_bridge/`

---

## Key Files Reference

| File | Role |
|------|------|
| `crates/qualia-core-db/src/poet_host/invoke/render/gpu.rs` | W0 invoke handlers (new) |
| `crates/qualia-core-db/src/render/gpu/mod.rs` | `PortalGpu` — core WebGPU renderer |
| `crates/qualia-core-db/src/render/anatomy/webgl2.rs` | WebGL2 fallback renderer |
| `crates/qualia-core-db/src/render/naga_bridge.rs` | WGSL → GLSL ES 300 compiler |
| `crates/qualia-core-db/src/render/naga_sanitize.rs` | Naga IR sanitizer for WebGL2 |
| `crates/qualia-core-db/src/gpu_context.rs` | `shared_gpu()`, `SharedGpuContext`, `GpuAdapterCaps` |
| `crates/qualia-core-db/src/poet_host/invoke/ids.rs` | Invoke ID constants + `ALL_BOUND` + `seam_for` |
| `crates/qualia-core-db/src/poet_host/invoke/mod.rs` | Dispatch switch |
| `docs/vibescript-full-impl-PLAN.md` | Updated plan with W0-W8 phases |

---

## Build & Test Commands

```powershell
# Check compilation
cargo check -p qualia-core-db

# Run GPU invoke tests
cargo test -p qualia-core-db --lib poet_host::invoke::render::gpu

# Run all invoke tests
cargo test -p qualia-core-db --lib poet_host::invoke

# Run ids tests
cargo test -p qualia-core-db --lib poet_host::invoke::ids
```

---

## Constraints Reminder
- **Zero heap in hot paths** (Tier 1): render frame loop, query kernels, GPU ABI buffers
- **Tier-2 cold construction**: invoke handlers, mesh upload, portal init — allocation allowed
- **42MB Sentinel**: all GPU memory under 42MB ceiling
- **48-byte Super-Quin**: all semantic data fits in NQuin
- Tests that need a GPU skip gracefully when `try_shared_gpu()` returns `None`
