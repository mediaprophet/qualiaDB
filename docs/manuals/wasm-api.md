# QualiaDB WebAssembly API & Integration Guide

**Version:** 0.0.29 | **Branch:** `0.0.29`
**Primary artifact:** `docs/pkg/qualia/qualia.js` + `qualia_bg.wasm` (`--features portal`)  
**Playground artifact:** `docs/playground/qualia_core_db.js` (`--features wasm-full`)  
**Portal manual:** [`qualia-wasm-portal.md`](qualia-wasm-portal.md)

The `qualia-core-db` crate compiles to `wasm32-unknown-unknown` with two feature profiles:

| Profile | Features | Use case |
|---------|----------|----------|
| **Portal slim** | `portal` | GitHub Pages, spatial demo, QApp embed — viewport + acoustic (kept under the 2 MB / 800 KB size budget) |
| **Full playground** | `wasm-full` | API explorer, logic evaluators, scientific modalities, **and the browser LLM** |

> The **browser LLM lives in the `wasm-full` playground bundle**, not the slim portal — keeping every
> spatial page lean. LLM demos (`llmdemo/`, `online-llm-demo.html`, `wasm-llm-test.html`, `benchmark.html`)
> all import from `playground/qualia_core_db.js`.

---

## 1. Building WASM targets

### 1.0 Ontology MCP (smallest)

For ontology sites and browser-local agent interactivity:

```powershell
wasm-pack build crates/webizen-lite-wasm --target web --out-dir pkg --release
```

This is a separate crate backed by the `wasm-ontology` kernel. It excludes the
portal, WebGPU, scientific and LLM profiles. See
[`wasm-capability-profiles.md`](wasm-capability-profiles.md).

### 1.1 Portal slim (recommended for demos)

```powershell
$env:RUSTFLAGS = "-C target-feature=+simd128"
wasm-pack build crates/qualia-core-db `
  --target web --release `
  --out-dir crates/qualia-core-db/pkg-qualia `
  --no-default-features -- --features portal
```

Publish as `docs/pkg/qualia/qualia.{js,bg.wasm,d.ts}`.

### 1.2 Full playground

```bash
wasm-pack build crates/qualia-core-db \
  --target web --release \
  --out-dir docs/playground \
  --out-name qualia_core_db \
  --no-default-features --no-typescript \
  -- --features wasm-full
```

### 1.3 Smoke checks

```powershell
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features portal
node docs/tests/phenomenal-verify.mjs --wasm-api docs/pkg/qualia/qualia.d.ts
```

**Portal features enabled:** `serde-wasm-bindgen`, `js-sys`, `web-sys` (canvas, WebGPU, `SharedArrayBuffer`), embedded viewport WGSL.

---

## 2. QualiaPortal (viewport + acoustic)

The primary browser constructor is `QualiaPortal` — not the legacy free-function playground exports.

```javascript
import init, { QualiaPortal } from './pkg/qualia/qualia.js';

await init();
const portal = new QualiaPortal(canvas);
portal.resize(canvas, width, height);

function frame() {
  portal.tick(canvas, 16.67);
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
```

### 2.1 Tier & mode

```javascript
const tier = portal.tier();           // 0 = CPU, 1 = tensor, 2 = WebGPU
const mode = portal.operational_mode(); // Full / Eco / Reserve
```

### 2.2 Tensor & navigation

```javascript
portal.upload_tensor_buffer(uint8Array);
portal.select_node_at(x, y, canvasW, canvasH);
const idx = portal.poll_selected_node(); // after next tick
portal.navigate_to_node(idx);
portal.collapse_node_q(idx);
portal.encode_geometry(jsonString);
```

### 2.3 Human-Centric standpoint

```javascript
portal.set_standpoint(
  standpointClass,  // 0=spectator, 1=ephemeral, 2=identifier, 3=vault
  epistemicQ,
  tSlice,
  tWindow,
  identifierDid
);
portal.set_camera(yaw, pitch, zoom);
```

### 2.4 U3 AcousticPlane

```javascript
portal.set_acoustic_enabled(true);

// MessagePort path (always works)
const floats = portal.acoustic_uniform_floats(); // Float32Array[82]

// SharedArrayBuffer path (requires crossOriginIsolated)
const sab = portal.create_acoustic_sab();
portal.publish_acoustic_sab(sab);

const pending = portal.sonic_token_pending();
const tokens = portal.drain_sonic_tokens(Math.min(pending, 16));
const sidecar = portal.bake_stft_sidecar_demo(32); // Uint8Array
```

Integrate with `docs/js/qualia-shell.js` → `mountAcousticPlane(portal)` and `docs/js/qualia-audio-worklet.js`.

**Binary layouts:** [`standards/q42-acoustic-plane-draft.md`](standards/q42-acoustic-plane-draft.md).

---

## 3. Free-function exports (playground / wasm-full)

These remain available in the full build and are re-exported for evaluator demos:

| Export | Purpose |
|--------|---------|
| `spatial_encode_wasm(json)` | Geometry → Quin + tensor buffer |
| `geosparql_operation_wasm(json)` | WKT + op → result |
| `export_tensor_buffer_wasm(max)` | Binary SOA for GPU upload |
| `sample_browser_telemetry_wasm()` | Normalized vitals `f32[]` |
| `validate_shacl_constraint_wasm(...)` | SHACL evaluation |
| `parse_n3logic_wasm(...)` | N3 logic parse |

Portal pages should prefer `QualiaPortal` methods over duplicating these calls.

---

## 4. LLM inference & Extension Bus

Local LLM inference uses the **in-process** `gguf_bridge` + WebGPU path — not an external Ollama
server, llama.cpp daemon, or Python. Pure Rust→WASM. Phase 5 decode is **~5.9 tok/s** on
SmolLM2-360M (Q4_K_M), coherent, on a stock NVIDIA Ampere via Chrome WebGPU.

### 4.1 Browser WebGPU path — AOT P64

Compile GGUF once to canonical P64, cache the result in OPFS, then initialize
the WebGPU engine from the validated P64:

```javascript
import init, {
  initialize_webgpu_engine,
  inferWasmAsync,
  compileGgufToP64,
  p64FormatVersion,
} from './playground/qualia_core_db.js';
import { loadOrCompileP64 } from './js/opfs-model-cache.js';

await init();
const { bytes } = await loadOrCompileP64(modelUrl, modelName, {
  compile: compileGgufToP64,
  formatVersion: p64FormatVersion(),
});
await initialize_webgpu_engine(bytes);

await inferWasmAsync('The capital of France is', (tokenDelta) => {
  outputEl.textContent += tokenDelta;
});
```

`initialize_webgpu_engine` also accepts GGUF bytes as a fallback. Historical
exports `compileGgufToQ42` and `q42FormatVersion` remain aliases and still emit
or report P64 v3. The checked-in playground bundle includes both naming
surfaces. See the [Q42/P64 Inference Pipeline](p64-q42-inference-pipeline.md)
for the remaining model-backed browser release checks and the
[P64 Weight Container Standard](standards/p64-weight-container-standard.md)
for the binary format.

### 4.2 Extension Bus (hybrid — native daemon offload)

The **Extension Bus** bridges WASM sync code to the native daemon (`ws://127.0.0.1:4242`):

```javascript
import init, { init_extension_bus, infer_local_model_streaming } from './pkg/qualia_core_db.js';

await init();
init_extension_bus("did:q42:local-user");
infer_local_model_streaming(prompt, graphContext, (tokenDelta) => {
  outputEl.textContent += tokenDelta;
});
```

If the daemon is unreachable, the engine falls back to the in-browser WebGPU path (§4.1, RAM-limited).

---

## 5. SharedArrayBuffer & COOP/COEP

**Updated 2026-06-17:** U3 acoustic zero-copy **does** use `SharedArrayBuffer` when available.

| Path | COI required? |
|------|----------------|
| Viewport WebGPU | No |
| Daemon WebSocket / fetch | No |
| `acoustic_uniform_floats()` MessagePort | No |
| `create_acoustic_sab()` / `publish_acoustic_sab()` | **Yes** (`crossOriginIsolated`) |

Register `docs/js/coi-serviceworker.js` via `qualia-coi.js` or the inline bootstrap in `spatial.html`.

---

## 6. Fiduciary cryptography & governance

Intent mediation, Ed25519 agency, and ML-DSA-65 (FIPS-204) paths are identical across WASM and native builds where features are enabled.

```javascript
// Full build only — example
validate_intent_wasm(intentJson); // → WebizenVerdict JSON
```

Daemon tensor slice auth uses `crypto.subtle` Ed25519 on a canonical `{nonce|class|t_slice|t_window}` string — see `qualia-shell.js`.

---

## 7. Integration rules

1. **One WASM module** on portal pages — `qualia_bg.wasm` only; do not load playground + portal simultaneously.
2. **No heap in hot paths** — pass `Float32Array` / `Uint8Array` views; avoid per-frame `JSON.stringify` on tensor data.
3. **Tier honesty** — display `portal.tier()` in the UI badge; never claim WebGPU when tier is 0.
4. **σ parity** — if you customize spectral shaders, keep `portal_acoustic.rs` Hz mapping aligned.

---

## 8. Related documents

| Doc | Content |
|-----|---------|
| [`qualia-wasm-portal.md`](qualia-wasm-portal.md) | Full operator manual |
| [`DEVELOPMENT.md`](DEVELOPMENT.md) | CI, daemon, Flutter |
| [`adr/0007-u3-acoustic-plane-symbolic-audio.md`](adr/0007-u3-acoustic-plane-symbolic-audio.md) | Why symbolic audio |
