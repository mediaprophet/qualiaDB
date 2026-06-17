# Audio Project Status & Next Session Notes

**Date:** 2026-06-17  
**Branch:** `0.0.17-dev`  
**Status:** **Complete on Pages** — U3 + CQT bake/link + KemarLite HRTF; desktop parity (PR-C10) remains
**Companion:** [`wasm-viewport-migration-plan.md`](wasm-viewport-migration-plan.md) Track B5, P-F1/P-F2  
**Manual:** [`../manuals/qualia-wasm-portal.md`](../manuals/qualia-wasm-portal.md)  
**Standard:** [`../manuals/standards/q42-acoustic-plane-draft.md`](../manuals/standards/q42-acoustic-plane-draft.md)  
**ADR:** [`../manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md`](../manuals/adr/0007-u3-acoustic-plane-symbolic-audio.md)

---

## Executive summary

Universe **U3 (`AcousticPlane`)** is live in `qualia-core-db` and on GitHub Pages (`spatial.html` → **Enable U3 Sonification**). The manifold hears what it sees: shared `[α, μ, σ]` truth with phenomenal λ↔Hz parity.

| Layer | Status |
|-------|--------|
| Sonic Tokens + SPSC ring | ✅ |
| `AcousticUniform` / 82-float contract | ✅ |
| Analytic binaural HRTF + camera yaw | ✅ |
| STFT cold bake + preview bins | ✅ |
| `Q3AS` SharedArrayBuffer + COI bootstrap | ✅ |
| AudioWorklet stereo + overlap-add grains | ✅ |
| CI (`audio::`, `phenomenal_hrtf`, wasm API) | ✅ 28 + phenomenal-verify |
| CQT bake + sidecar link + portal pin | ✅ |
| KemarLite embedded HRTF (8-azimuth) | ✅ |
| Full measured KEMAR asset tables | ⬜ optional upgrade |
| Multi-track `AudioScene` DAW | ⬜ deferred |

---

## Architecture (three layers)

| Layer | Visual | Audio | Heap? |
|-------|--------|-------|-------|
| **Truth (cold)** | SPD mmap | STFT/CQT sheet (`Q4AU`) | Bake only |
| **Resident (pin)** | `Tensor10D` SOA | Same SOA; U3 aliases U1 | `VramLedger` |
| **Render (hot)** | `spectral.wgsl` | AudioWorklet + SAB/MessagePort | Pre-allocated |

**Hard rule:** U0 never emits PCM — only Sonic Tokens and graph structure.

---

## Shipped modules

| Module | Path |
|--------|------|
| Sonic Token | `crates/qualia-core-db/src/sonic_token.rs` |
| U3 plane + ring | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| DSP kernel | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| HRTF | `crates/qualia-core-db/src/audio/hrtf.rs` |
| STFT bake | `crates/qualia-core-db/src/audio/stft_bake.rs` |
| CQT bake | `crates/qualia-core-db/src/audio/cqt_bake.rs` |
| Sidecar link | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| SAB layout | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| Spectral sheet | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| σ parity | `crates/qualia-core-db/src/portal_acoustic.rs` |
| Portal WASM exports | `crates/qualia-core-db/src/portal.rs` |
| Worklet | `docs/js/qualia-audio-worklet.js` |
| Shell glue | `docs/js/qualia-shell.js`, `qualia-coi.js`, `coi-serviceworker.js` |

---

## Sonic Token layout (normative)

```
Bits [0..7]    delta_time
     [8..11]   event_type (0=NoteOn, 1=NoteOff, 2=CC, 3=Parametric)
     [12..15]  channel
     [16..23]  note
     [24..31]  velocity
     [32..47]  tensor_index (16-bit)
     [48..63]  flags (SONIC_MAGIC 0x53)
```

---

## Verification

```powershell
cargo test -p qualia-core-db audio:: --lib
cargo test -p qualia-core-db phenomenal_hrtf phenomenal_sigma_visual_audio_parity --lib
node docs/tests/phenomenal-verify.mjs --wasm-api docs/pkg/qualia/qualia.d.ts
```

---

## Remaining work (priority order)

1. **Desktop parity** — wire U3 in webizen-browser host (PR-C10) — separate from audio core.
2. **Full KEMAR dataset** — optional cold asset replacing KemarLite interpolation.
3. **Offline renderer** — `OfflineAudioContext` export from token log + sheet refs.
4. **Legacy PCM import** — cold transcode → STFT/CQT sheet (one-way).
5. **External standards submission** — vectors at `docs/manuals/standards/vectors/acoustic-plane-v0.1.json`.

---

## Progress log

| Date | Update |
|------|--------|
| 2026-06-17 | Architecture locked (symbolic + spectral-first) |
| 2026-06-17 | **Implementation shipped** — U3, worklet, SAB, σ parity CI, docs + ADR 0007 |
| 2026-06-17 | **PR-B5f** — CQT bake, `audio_sidecar_link`, portal sidecar pin, KemarLite HRTF, test vectors |