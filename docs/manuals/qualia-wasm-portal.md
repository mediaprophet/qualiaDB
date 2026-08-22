# Qualia WASM Portal — Operator & Integrator Manual

**Product name:** Qualia — *Semantic Subjectivity Bifurcation Portal*  
**Version:** 0.0.33
**Branch:** `0.0.33`
**Artifact:** `docs/pkg/qualia/qualia.js` + `qualia_bg.wasm`  
**Companion:** [`wasm-viewport-migration-plan.md`](../plans/wasm-viewport-migration-plan.md), [`q42-acoustic-plane-draft.md`](standards/q42-acoustic-plane-draft.md)

---

## 1. What this is

The Qualia WASM portal is a **single** `cdylib` that bundles:

| Layer | Responsibility |
|-------|----------------|
| **Qualia engine** | NQuin evaluators, SHACL, modalities, 10D tensor SOA, daemon slice ingest |
| **Webizen viewport** | wgpu ambient + projector + bloom; σ→CIE spectral coloring; PGA motors |
| **U3 AcousticPlane** | Symbolic Sonic Tokens + parametric DSP + binaural HRTF + inverse-STFT grains |

JavaScript is **glue only**: `import init`, `new QualiaPortal(canvas)`, `resize`, `requestAnimationFrame(tick)`. No geometry, no particle engines, no spectral math in the hot path.

```
┌──────────── BAKE (cold, heap OK) ────────────┐
│  NQuin → Tensor10D SOA + optional STFT sidecar │
└────────────────────┬───────────────────────────┘
                     │ upload once
┌────────────────────▼───────────────────────────┐
│  GpuContext + VramLedger (U0 LLM / U1 tensor / │
│  U2 viewport / U3 acoustic — one device)       │
└────────┬───────────────────┬───────────────────┘
         │                   │
    projector.wgsl      AudioWorklet (stereo)
    ambient + bloom     SAB or MessagePort
```

---

## 2. Hardware tiers

`QualiaPortal::new(canvas)` probes WebGPU and selects a tier:

| Tier | Badge | Render path | Audio |
|------|-------|-------------|-------|
| **T0** | CPU fallback | canvas2d particles (`ambient-viz.js` lazy) | MessagePort uniform sync |
| **T1** | Tensor projection | 2D tensor canvas | Same |
| **T2** | WebGPU phenomenal | projector → ambient → bloom | SAB zero-copy when `crossOriginIsolated` |

Operational modes (`Full` / `Eco` / `Reserve`) throttle bloom and particle draw counts via `VramLedger`. **U3 is muted in Reserve.**

---

## 3. Build & deploy

### 3.1 Slim portal (GitHub Pages / spatial demo)

```powershell
# From repo root
$env:RUSTFLAGS = "-C target-feature=+simd128"
wasm-pack build crates/qualia-core-db `
  --target web --release --out-dir crates/qualia-core-db/pkg-qualia `
  --no-default-features -- --features portal

# Publish to docs
Copy-Item crates/qualia-core-db/pkg-qualia/qualia_core_db.js docs/pkg/qualia/qualia.js
Copy-Item crates/qualia-core-db/pkg-qualia/qualia_core_db_bg.wasm docs/pkg/qualia/qualia_bg.wasm
Copy-Item crates/qualia-core-db/pkg-qualia/qualia_core_db.d.ts docs/pkg/qualia/qualia.d.ts
```

Or use `scripts/package-qualia-wasm.ps1` when execution policy allows.

### 3.2 Verification

```powershell
cargo test -p qualia-core-db phenomenal_contract --lib
cargo test -p qualia-core-db audio:: --lib
node docs/tests/phenomenal-verify.mjs --wasm-api docs/pkg/qualia/qualia.d.ts
node docs/tests/wasm-size-check.mjs docs/pkg/qualia/qualia_bg.wasm
```

### 3.3 Local preview (COOP/COEP for SharedArrayBuffer)

```powershell
npx --yes serve docs -p 4173
```

Open `http://localhost:4173/spatial.html`. The page registers `coi-serviceworker.js` on first load; **reload once** after the service worker activates to gain `crossOriginIsolated` and SAB zero-copy audio.

---

## 4. JavaScript integration

### 4.1 Minimal mount

```javascript
import init, { QualiaPortal } from './pkg/qualia/qualia.js';
import { mountPortal, mountAcousticPlane, setAcousticEnabled } from './js/qualia-shell.js';

await init();
const canvas = document.getElementById('ambient-canvas');
const portal = await mountPortal(canvas);

function frame() {
  portal.tick(canvas, 16.67);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
```

### 4.2 Daemon-linked phenomenal path

`qualia-shell.js` provides:

- `connectPortalToDaemon(portal)` — `GET /tensor/slice` + Lamport SSE `/tensor/events`
- Ed25519 standpoint gate (`crypto.subtle` sign of canonical `{nonce|class|t_slice|t_window}`)
- Badge states: `Offline` → `Slice unavailable` → `Auth Failed` → `Live`

### 4.3 U3 sonification

```javascript
import { mountAcousticPlane, setAcousticEnabled } from './js/qualia-shell.js';
import { ensureCrossOriginIsolation, isCrossOriginIsolated } from './js/qualia-coi.js';

await ensureCrossOriginIsolation({ quiet: true });
await mountAcousticPlane(portal, { useSab: isCrossOriginIsolated() });
setAcousticEnabled(true, portal);
portal.set_acoustic_enabled(true);
```

Sync loop (inside `mountAcousticPlane`):

1. `publish_acoustic_sab(sab)` **or** `acoustic_uniform_floats()` → worklet
2. `bake_stft_sidecar_demo(32)` every ~2 s → preview bins refresh
3. `drain_sonic_tokens(n)` → token accents on the worklet thread

---

## 5. `QualiaPortal` API reference

### 5.1 Viewport & navigation

| Method | Description |
|--------|-------------|
| `new(canvas)` | Tier detect + paint loop init |
| `resize(canvas, w, h)` | Canvas + GPU surface resize |
| `tick(canvas, dt_ms)` | Frame: telemetry refresh, GPU passes, pick readback |
| `tier()` | `0` CPU / `1` tensor / `2` WebGPU |
| `operational_mode()` | `Full` / `Eco` / `Reserve` enum as `u32` |
| `set_camera(yaw, pitch, zoom)` | Orbit lens → 128 B `CameraUniform` |
| `set_standpoint(class, epistemic_q, t_slice, t_window, identifier_did)` | Human-Centric observer → 128 B `ObserverStandpoint` |
| `upload_tensor_buffer(bytes)` | Pin resident SOA + GPU rebind |
| `select_node_at(x, y, w, h)` | Queue GPU `R32Uint` pick |
| `poll_selected_node()` | Index after next `tick`, or `-1` pending |
| `navigate_to_node(index)` | Camera fly-to node `(x,y,z)` |
| `collapse_node_q(index)` | Set node `q` → 0, re-upload SOA |
| `encode_geometry(json)` | Spatial encode + tensor upload |

### 5.2 U3 AcousticPlane

| Method | Description |
|--------|-------------|
| `set_acoustic_enabled(bool)` | Enable/mute U3 (off in Reserve) |
| `acoustic_enabled()` | Current mute state |
| `acoustic_uniform_float_count()` | `82` (18 scalars + 64 preview bins) |
| `acoustic_uniform_floats()` | `Float32Array` for MessagePort path |
| `acoustic_uniform_bytes()` | Raw `AcousticUniform` pod bytes |
| `acoustic_sab_byte_length()` | `1024` |
| `create_acoustic_sab()` | Zeroed `SharedArrayBuffer` with `Q3AS` header |
| `publish_acoustic_sab(sab)` | Write uniform + float mirror + drain tokens into SAB |
| `sonic_token_pending()` | Ring occupancy |
| `drain_sonic_tokens(max)` | Pop up to `max` raw `u64` tokens |
| `push_sonic_token_raw(raw)` | Inject token (testing / sonify hooks) |
| `bake_stft_sidecar_demo(frames)` | Cold STFT sidecar for selected node |

Full binary layouts: [`q42-acoustic-plane-draft.md`](standards/q42-acoustic-plane-draft.md).

### 5.3 AcousticUniform float layout (82)

| Index | Field | Meaning |
|-------|-------|---------|
| 0 | `alpha` | Linear energy / gain staging |
| 1 | `mu` | Modulation / provenance phase |
| 2–4 | `position` | Tensor `x, y, z` → binaural source |
| 5 | `track_v` | Topological class `v` |
| 6 | `manifold_w` | Manifold index `w` → room damp |
| 7 | `epistemic_q` | Epistemic aperture → FM depth |
| 8 | `fm_index` | Parametric FM amount |
| 9 | `frequency_hz` | Carrier (σ parity blended) |
| 10 | `enabled` | `> 0` = audible |
| 11–12 | `gain_l`, `gain_r` | Binaural staging |
| 13 | `itd_seconds` | Inter-aural time difference |
| 14–15 | `azimuth_rad`, `elevation_rad` | Head-relative angles |
| 16 | `room_damp` | High-frequency absorption from `w` |
| 17 | `stft_frame` | Sidecar frame hint for grains |
| 18–81 | `preview_bins[64]` | Stack preview / inverse-STFT source |

---

## 6. Phenomenal σ parity (vision ↔ hearing)

The same `σ` field drives **both** U2 and U3:

| Modality | Projection | Module |
|----------|------------|--------|
| **Visual** | λ = 400 + fract(σ)×300 nm → CIE XYZ → linear sRGB | `portal_spectral.rs`, `spectral.wgsl` |
| **Auditory** | same λ band → Hz = lerp(1760, 110, t) where t = (λ−400)/300 | `portal_acoustic.rs` |

`fract(σ)` is invariant under integer wraps: `sigma_to_wavelength_nm(σ) == sigma_to_wavelength_nm(σ + n)`.

CI oracle: `cargo test -p qualia-core-db phenomenal_sigma_visual_audio_parity --lib`

---

## 7. Cross-origin isolation (COOP/COEP)

| Requirement | Why |
|-------------|-----|
| `crossOriginIsolated === true` | `SharedArrayBuffer` for `create_acoustic_sab` / `publish_acoustic_sab` |
| `coi-serviceworker.js` | Injects `Cross-Origin-Opener-Policy` + `Cross-Origin-Embedder-Policy` on reload |
| MessagePort fallback | When isolation unavailable, `acoustic_uniform_floats()` still works |

**Note:** wgpu WebGPU does **not** require SAB. Only U3 zero-copy uses it.

---

## 8. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Badge stuck `Offline` | Daemon not on `:4242` | `cargo run -p qualia-cli -- daemon start` |
| `Auth Failed` after standpoint | Ed25519 mismatch | Reset to Spectator or pair dev signing key |
| No audio after click | Gesture policy | User click must call `mountAcousticPlane`; check console |
| `SAB zero-copy` not shown | COI pending | Reload once after SW install |
| WebGPU tier 0 | Safari / old GPU | Expected; badge shows honest fallback |
| `bake_stft_sidecar_demo` fails | No tensor uploaded | Run encode / daemon slice first |

---

## 9. Related documents

| Document | Role |
|----------|------|
| [`wasm-api.md`](wasm-api.md) | Full WASM surface (playground + portal) |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | Build matrix, CI, daemon |
| [`standards/q42-acoustic-plane-draft.md`](standards/q42-acoustic-plane-draft.md) | Normative Sonic Token + SAB layouts |
| [`standards/q42-10d-tensor-standard.md`](standards/q42-10d-tensor-standard.md) | 10D semantics + phenomenal σ |
| [`adr/0007-u3-acoustic-plane-symbolic-audio.md`](adr/0007-u3-acoustic-plane-symbolic-audio.md) | Architectural decision record |
| [`../plans/AUDIO_PROJECT_STATUS.md`](../plans/AUDIO_PROJECT_STATUS.md) | Implementation status & backlog |

---

*Qualia — where meaning meets the manifold.*
