# Q42 AcousticPlane — Internal Draft Standard

**Version:** 0.1 (draft)  
**Date:** 2026-06-17  
**Status:** Internal draft — not submitted externally  
**Branch:** `0.0.18-dev`  
**Normative code:** `crates/qualia-core-db/src/audio/`, `sonic_token.rs`, `portal_acoustic.rs`

---

## Abstract

This draft defines the **U3 AcousticPlane** binary contracts for Qualia WASM: 64-bit Sonic Tokens, the `AcousticUniform` worklet payload, the 1024-byte SharedArrayBuffer (`Q3AS`) layout, and the phenomenal **σ parity** mapping shared with the visual spectral pipeline. It does **not** define LLM waveform generation — U0 emits structure (tokens), never PCM.

---

## 1. Scope & non-goals

### In scope

- Hot-path symbolic events (Sonic Tokens)
- Parametric synthesis driven by `Tensor10D` SOA
- Binaural staging from tensor position + camera yaw
- Cold STFT sidecar header (`Q4AU`)
- Browser transport (SAB + float mirror)

### Out of scope (v0.1)

- KEMAR HRTF table assets (analytic head model only)
- Full CQT mmap ingest path
- MP3/AAC as storage truth
- Neural audio from U0

---

## 2. Compute universe placement

| Universe | ID | Ledger | Notes |
|----------|-----|--------|-------|
| U0 | `LlmInference` | KV + weights | May push Sonic Tokens; **never PCM** |
| U1 | `Tensor10D` | SOA pin | U3 **aliases** U1 partition (read-only) |
| U2 | `Viewport` | Render targets | Visual σ → CIE |
| U3 | `AcousticPlane` | Same as U1 | Muted when `OperationalMode::Reserve` |

---

## 3. Sonic Token (64 bits)

**Size:** 8 bytes (`#[repr(C)]`, `Pod`).  
**Magic flag:** `0x0053` (`'S'`) in bits 48–63 for validated events.

### 3.1 Bit layout

```
Bits     Width   Field
──────────────────────────────────────
[0..7]     8     delta_time
[8..11]    4     event_type
[12..15]   4     channel
[16..23]   8     note
[24..31]   8     velocity
[32..47]  16     tensor_index
[48..63]  16     flags (SONIC_MAGIC = 0x53 in low byte)
```

### 3.2 Event types

| Value | Name | Semantics |
|-------|------|-----------|
| `0` | NoteOn | MIDI-like onset tied to `tensor_index` |
| `1` | NoteOff | Release |
| `2` | ControlChange | CC lane |
| `3` | Parametric | Engine pulse (no note number) |

### 3.3 Encoding (reference)

```rust
raw = delta_time
    | ((event_type & 0x0f) << 8)
    | ((channel & 0x0f) << 12)
    | ((note as u64) << 16)
    | ((velocity as u64) << 24)
    | ((tensor_index as u64 & 0xffff) << 32)
    | ((flags as u64 & 0xffff) << 48);
```

### 3.4 Transport

- **Ring:** SPSC `rtrb`, capacity 128 (`SONIC_RING_CAP`)
- **Drain:** `drain_sonic_tokens(max)` → JS array of `u64`
- **SAB slots:** 16 × 8 B at offset 384 (see §5)

---

## 4. AcousticUniform (328 bytes)

**Alignment:** 4-byte (`f32` fields).  
**Float export count:** 82 = 18 scalars + 64 preview bins.

### 4.1 C struct (logical)

```rust
#[repr(C)]
pub struct AcousticUniform {
    pub alpha: f32,
    pub mu: f32,
    pub position: [f32; 3],
    pub track_v: f32,
    pub manifold_w: f32,
    pub epistemic_q: f32,
    pub fm_index: f32,
    pub frequency_hz: f32,
    pub enabled: u32,
    pub gain_l: f32,
    pub gain_r: f32,
    pub itd_seconds: f32,
    pub azimuth_rad: f32,
    pub elevation_rad: f32,
    pub room_damp: f32,
    pub stft_frame: f32,
    pub preview_bins: [f32; 64],
}
```

`size_of::<AcousticUniform>() == 328`.

### 4.2 Binaural fields

Populated by `apply_binaural_to_uniform(uniform, listener_yaw)`:

- Source: `position` from tensor `(x, y, z)`
- Listener yaw: `portal` camera yaw (radians)
- Model: analytic ITD/ILD (`audio/hrtf.rs`) — not KEMAR tables in v0.1

---

## 5. Acoustic SharedArrayBuffer (`Q3AS`, 1024 bytes)

**Magic:** `0x5133_4153` (`"Q3AS"`)  
**Version:** `1`

| Region | Offset | Size | Content |
|--------|--------|------|---------|
| Header | 0 | 32 | `AcousticSabHeader` (28 B pod + 4 B pad) |
| Uniform | 32 | 352 | `AcousticUniform` pod (328 B used) |
| Tokens | 384 | 128 | 16 × 8 B Sonic Token ring |
| Float mirror | 512 | 328 | `f32[82]` for worklet `Float32Array` view |
| Reserved | 840 | 184 | Zero (future STFT frame ptr) |

### 5.1 Header fields

| Field | Type | Semantics |
|-------|------|-----------|
| `magic` | `u32` | `0x5133_4153` |
| `version` | `u16` | `1` |
| `uniform_seq` | `u16` | Incremented each `publish_acoustic_sab` |
| `token_write` | `u32` | Ring write index |
| `token_read` | `u32` | Ring read index (worklet) |
| `stft_frame` | `u32` | Sidecar frame hint |
| `sample_rate` | `u32` | Default `48000` |

Worklet polls `uniform_seq` (offset 6, `u16` LE) against last seen sequence.

---

## 6. Phenomenal σ parity

Shared fractal wavelength:

```
fract(σ) = σ - floor(σ)
λ_nm     = 400 + fract(σ) × 300        // 400–700 nm
t        = clamp((λ_nm - 400) / 300, 0, 1)
f_hz     = clamp(1760 × (1 - t) + 110 × t, 55, 8000)
```

**Visual twin:** `portal_spectral::sigma_to_cie_xyz(σ)` uses the same λ band.  
**Auditory carrier:** `phenomenal_voice_frequency_hz` blends `f_hz` (72%) with bin-peak heuristic (28%).

**Invariant:** `f_hz(σ) == f_hz(σ + n)` for integer `n`.

---

## 7. STFT sidecar header (`Q4AU`, 20 bytes)

```rust
#[repr(C)]
pub struct AudioSpectralSidecarHeader {
    pub magic: u32,       // 0x5134_4155 "Q4AU"
    pub version: u16,     // 1
    pub _pad: u16,
    pub bin_count: u32,   // typically 64
    pub frame_count: u32,
    pub sample_rate: u32, // typically 48000
}
// Payload: frame_count × bin_count × f32 LE
```

Cold bake: `bake_stft_sidecar_demo(frames)` in portal WASM.  
Future: mmap under `{storage}/spectral/audio/{hash}.bin` at ingest (P-A4 remainder).

---

## 8. Conformance

Implementations MUST:

1. Keep hot-path audio APIs zero-heap (`&mut [T]` out buffers).
2. Never store MP3/AAC as semantic truth — sheets + tokens only.
3. Preserve σ parity formulas byte-identical to `portal_acoustic.rs` oracles.
4. Reject SAB buffers whose `byteLength !== 1024`.

### Test vectors

```powershell
cargo test -p qualia-core-db audio:: --lib
cargo test -p qualia-core-db phenomenal_hrtf phenomenal_sigma_visual_audio_parity --lib
node docs/tests/phenomenal-verify.mjs
```

---

## 9. Changelog

| Version | Date | Change |
|---------|------|--------|
| 0.1 | 2026-06-17 | Initial draft — shipped U3 on Pages |

---

*See [`standards-backlog.md`](standards-backlog.md) for external submission timeline.*