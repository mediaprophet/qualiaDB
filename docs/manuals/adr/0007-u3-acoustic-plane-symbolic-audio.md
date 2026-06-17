# ADR 0007: U3 AcousticPlane — Symbolic & Spectral-First Audio

## Status
Accepted (2026-06-17)

## Context

QualiaDB's phenomenal viewport (U2) projects the shared `[α, μ, σ]` truth layer from `Tensor10D` SOA into linear RGB via CIE Gaussian CMFs. Users expect the same semantic manifold to be **heard** as well as seen — without introducing a second truth stack or routing neural PCM from the LLM inference universe (U0).

Competing approaches considered:

1. **LLM waveform generation** — U0 emits PCM buffers. Rejected: blows 42 MB Sentinel budget, adds non-deterministic latency, violates zero-heap hot paths.
2. **`<audio>` tag + MP3 URLs** — Simple but lossy; psychoacoustic discard destroys sovereignty over dynamic range (`α`).
3. **Web Audio API from JS** — Reintroduces V8 spectral math the migration plan retires.
4. **Symbolic tokens + spectral sheets + parametric DSP** — Matches visual cold/hot split (bake → resident → render).

Browser constraints add a transport decision: AudioWorklet runs off the main thread; uniform updates must be either `postMessage` or `SharedArrayBuffer` (requires COOP/COEP).

## Decision

Introduce **ComputeUniverse::AcousticPlane (U3)** as a read-only alias of the U1 `Tensor10D` ledger partition with its own hot-path consumer:

| Layer | Mechanism |
|-------|-----------|
| **Truth (cold)** | STFT/CQT mmap sidecars (`Q4AU` header) + inline `α, μ, σ` in tensor |
| **Events (hot)** | 64-bit **Sonic Tokens** over SPSC ring (U0/U1 → U3) |
| **Synthesis (hot)** | Browser **AudioWorklet**: parametric carrier + overlap-add inverse-STFT grains + analytic binaural HRTF |
| **Transport** | 1024 B `Q3AS` SharedArrayBuffer when `crossOriginIsolated`; else `Float32Array` MessagePort |

### Locked invariants

1. **U0 never emits PCM** — only Sonic Tokens and graph structure.
2. **σ parity** — visual λ (400–700 nm) and auditory Hz (1760–110) derive from the same `fract(σ)` mapping (`portal_acoustic.rs`).
3. **U3 muted in Reserve** — acoustic synthesis yields to LLM KV retention under VRAM pressure.
4. **Analytic HRTF in hot path** — KEMAR tables, if adopted, are cold-path assets only.
5. **Sonic Token layout** — 8-byte `Pod`; tensor index at bits 32–47; `SONIC_MAGIC` `0x53` in flags.

## Consequences

### Positive

- Phenomenal multi-modal fidelity: orbit camera → pan moves; select node → σ drives timbre in both modalities.
- Zero-copy uniform path on COI-enabled Pages deploys.
- CI oracles (`phenomenal_sigma_visual_audio_parity`, `phenomenal_hrtf`, 22 `audio::` tests) prevent layout drift.
- Same resident tensor buffer feeds U2 draw calls and U3 binaural position — no duplicate scene graph.

### Negative

- COOP/COEP requires service worker + reload on GitHub Pages; users without isolation still work but pay MessagePort copy cost.
- Full CQT mmap ingest and KEMAR HRTF deferred — analytic model is less spatially accurate than measured HRTF.
- `AUDIO_PROJECT_STATUS.md` and webizen `audio_contract.rs` salvage docs required migration to qualiaDB ownership.

### Neutral

- WASM portal size grew (~436 KB raw); still within slim portal budget after gzip.
- Desktop host (webizen-browser) does not yet mirror U3 — Pages path is golden master until PR-C10.

## References

- [`docs/manuals/qualia-wasm-portal.md`](../qualia-wasm-portal.md)
- [`docs/manuals/standards/q42-acoustic-plane-draft.md`](../standards/q42-acoustic-plane-draft.md)
- [`docs/plans/wasm-viewport-migration-plan.md`](../../plans/wasm-viewport-migration-plan.md) — Track B5, P-F1/P-F2
- ADR 0001 — 48-byte Quin alignment (Sonic Tokens are separate 8-byte events, not Quins)