# GGUF decode residency — perf-push progress log

Workstream: get native gguf decode from ~18.5 tok/s toward the 60 tok/s target by removing
per-layer CPU↔GPU readback stalls. Branch: `0.0.21-la` (worktree `.worktrees/qualia-la`).
Honest engineering record (§9). Numbers are real or marked "not measured".

---

## 2026-06-27 · Step 0 — diagnosis (DONE) · the bottleneck + a corrected premise

**What I set out to do.** Execute the approved "Option 1, Phase 1": move the intra-layer residual
add to the GPU so each transformer layer does 1 readback instead of 2.

**What the code actually says (the correction).** Phase 1 as conceived does *not* buy the win, and
here is why — traced to specific lines:

- The native decode path is `gguf_bridge/forward.rs::dispatch_transformer_layer` (hidden lives in a
  CPU `&mut [f32]`).
- It already calls two **single-submit fused** GPU paths:
  - attention: the fused Q+o tail (one submit + one readback), then a CPU residual add;
  - FFN: `ffn.rs::dispatch_ffn_fused_resident` — gate/up/SiLU/down in **one** submit + **one**
    readback (`ffn.rs:312`–`326`), then a CPU residual add.
- So native is already at **2 readbacks/layer**, not 64. The two readbacks are **not** caused by the
  residual add being on the CPU — they are caused by the **RMSNorm pre-norm being on the CPU**
  (`prepare_pre_norm_input`, called at `ffn.rs:180` and in the attention path). The pre-norm needs
  the hidden vector in CPU memory *between* attention and FFN, which forces the readback regardless
  of where the residual add runs. Moving just the residual to the GPU removes zero readbacks.

**Where the real win is.** A resident-activation forward: keep `hidden` in VRAM across the layer,
do **RMSNorm on the GPU** (`ELEM_OP_RMS_NORM`), residual on the GPU (`ELEM_OP_ADD_RESIDUAL`), and
read back **once at the end of the whole stack** instead of twice per layer. This path already
exists and is proven — but **only for `wasm32`**:
- `forward.rs::dispatch_transformer_forward_async` + `encode_attn_ffn_tail_gpu` (the super-arena
  decode forward) are `#[cfg(target_arch = "wasm32")]`.
- The encode primitives they use (`encode_elem`, `encode_residual_add_gpu`, `encode_gemm_bufs`,
  `encode_attention_pass_gpu`) and the `WasmGpuPipeline` encoder live in
  `#[cfg(target_arch = "wasm32")] mod mc8_wasm` (`mod.rs:378`) — wasm-only.
- The wasm readback model is browser-async (`await_wgpu_map`); native uses `poll_wait` + `block_on`
  (as `dispatch_ffn_fused_resident` already does).

**The good news (de-risks the port).** The *GPU pipelines* the resident forward needs already build
on native: `elem_rms_norm_pipeline`, `elem_silu_mul_pipeline`, `elem_add_residual_pipeline` and the
`mc8_elem_bind_layout` are created under `#[cfg(not(target_arch = "wasm32"))]` (`init.rs:377`+), and
the super-arena staging buffer *fields* exist on native (`mod.rs:1009`–`1012`). What's missing on
native is the **driver + encode methods**, not the shaders.

**Measured results.** Decode baseline ~18.5 tok/s (prior session, SmolLM2-360M, A2000). No new
measurement this step — diagnosis only. No code changed; the live decode path is untouched.

**The honest scope.** This is not a "flip a cfg flag" job. The native port = (a) factor the
synchronous leaf encode primitives + a shared pass-encoder out of the wasm-only module so they
compile on native; (b) write a native resident-decode driver that chains
RMSNorm→Q/K/V→attn→o→GPU-residual→RMSNorm→gate/up→SiLU→down→GPU-residual per layer keeping `hidden`
in VRAM, with one `poll_wait` readback after the last layer; (c) gate it behind a default-off
`QUALIA_LLM_RESIDENT_DECODE` toggle so the working 18.5 tok/s path (and #48 coherence) is never at
risk; (d) verify with the `a1a` coherence test, then measure tok/s. ~200–400 lines across
`mod.rs`/`init.rs`/`forward.rs` + a new native encode module, with a real release-build + GPU +
model verify loop.

**⚑ Where I need the human (Timothy).** One direction call: the approved "Phase 1 (2→1 readback)"
turns out not to apply — the readbacks are RMSNorm-bound, so the meaningful work is the larger
native resident-forward port described above. That's a focused multi-hundred-line port with a full
verify loop. Confirm you want me to drive that port now, and I'll build it incrementally
(compile-checked, default-off, verified against `a1a` before the toggle flips). No other out-of-band
datum needed.

**Next step.** On your go: factor the native encode primitives + pass-encoder (foundation), then the
resident-decode driver, then `a1a` + tok/s.

---

## 2026-06-27 · Step 1 — native resident-decode driver BUILT + VERIFIED · honest measured result

**Status: done (built, coherence-verified, measured) — but a NET-NEUTRAL perf result, kept opt-in.**

**What was built.** A native resident-activation decode forward, gated behind a default-OFF
`QUALIA_LLM_RESIDENT_DECODE` toggle (the legacy CPU-hidden path is byte-identical when off).
- `inference/inference_bench.rs`: `resident_decode_enabled()` / `set_resident_decode()` toggle.
- `gguf_bridge/mod.rs` + `init.rs`: new native `resident_hidden_buf` field (the residual stream
  kept in VRAM across the stack) + `mod resident_decode`.
- `gguf_bridge/resident_decode.rs` (new, ~700 lines): GPU encode helpers (`rd_rms_norm`,
  `rd_residual_inplace`, `rd_elem`, native `rd_upload_norm_weights`) + resident variants of the three
  fused passes (`rd_attention_kv_preproject`, `rd_attention_q_o`, `rd_ffn`) — the proven native encode
  patterns minus the CPU upload and minus the readback — and the driver
  `dispatch_transformer_forward_resident`. Per layer: GPU RMSNorm→KV-proj→Q-attn→o-proj→GPU-residual
  →GPU RMSNorm→gate/up→SiLU→down→GPU-residual, with `hidden` never leaving VRAM. **One** readback,
  after the last layer. All-or-nothing: any ineligibility returns `None` with CPU `hidden` untouched
  → caller falls back to legacy from the identical input (safe).
- `gguf_bridge/forward.rs`: `dispatch_transformer_forward` routes to the resident driver at its top
  when the toggle is on; falls through otherwise.

**Coherence — VERIFIED EXACT.** `a1c_q8_gemm_decode_coherent` (SmolLM2-360M Q8, A2000), prompt
"Once upon a time, there was a", 24 tokens:
- default: `" young woman named Sarah who had always been fascinated by the world of art. She spent hours poring over art books,"`
- resident (`QUALIA_LLM_RESIDENT_DECODE=1`): **byte-identical** to default.
- Execution proven by a one-shot marker: `[resident-decode] ACTIVE — 32 layers resident, 1
  readback/token (was 2/layer)` (i.e. it did NOT silently fall back). Not just coherent — identical
  to the legacy path. The #48 coherence guard holds.

**Measured perf (`a0_decode_profile`, SmolLM2-360M Q8, A2000, 16 tokens) — the honest result:**

| metric | default (CPU-hidden) | resident (VRAM-hidden) |
|---|---|---|
| decode | **16.26 tok/s** | **15.94 tok/s** (−2%, within noise) |
| GPU submit→wait round-trips | 107 /tok | **44 /tok** (−63 ≈ the 64 per-layer readbacks) |
| est. fence (sync) overhead | 13.9 ms/tok (23%) | **5.0 ms/tok (8%)** |
| forward (32 layers) | 58.7 ms/tok | 60.0 ms/tok |

**What this means (no spin).** The readback elimination did exactly what it was designed to —
round-trips 107→44, fence overhead 13.9→5.0 ms/tok. But **net tok/s is flat**: the ~9 ms of sync
stall removed was paid straight back by doing RMSNorm + the residual add on the GPU. The RMSNorm
shader is `@workgroup_size(1)` (a *serial* per-row reduction), each layer now has more, smaller
submits, and the residual add needs a scratch + copy-back (WebGPU forbids aliasing read & read-write
bindings). The profiler's verdict is unchanged: **the kernels are the wall** (attention 35 ms/tok,
FFN 23 ms/tok = 96% of the token). Readbacks were a real but secondary cost (~23% → 8%), not the
thing standing between 16 and 60 tok/s.

**Decision.** Toggle stays **OFF** — shipping it as default would be a ~2% regression. The path is
correct, verified, and a clean foundation, but it does not pay for itself **yet**. Two ways to make
it a net win, both deferred pending your call:
1. Fold the per-layer ops into one submission (per-layer 256-aligned param slots, like the FFN's
   `ffn_fused_params`) + parallelize the RMSNorm shader (it's serial today). This should recover the
   added overhead and bank the ~13 ms fence as real tok/s (~16 → ~18–19 est., not measured).
2. Recognize that even a perfect resident path caps at ~+18% (the fence ceiling) — and put the effort
   into the **kernels** (coop_gemv / attention shader), which own 96% of the token and are the only
   road to 60 tok/s.

**⚑ Where I need the human (Timothy).** A direction call, now backed by numbers: the resident-decode
work is **done and honest but net-neutral** because decode is kernel-bound, not readback-bound.
Do you want me to (a) push the resident path to a real (~+15%) win via single-submit-per-layer +
parallel RMSNorm, or (b) leave it as the verified opt-in foundation and pivot to the kernels
(attention/GEMV), which is where the 60 tok/s actually lives? My recommendation: **(b)** — bank this
as a correct, coherence-proven foundation (it composes with kernel wins later), and aim the next
session at the attention/GEMV shaders. Nothing is shipped as default; no regression. One out-of-band
datum: none.

**Next step.** Per your call above. No code change to the live path either way (toggle stays off).
