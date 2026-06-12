# QualiaDB WebAssembly (WASM) API & Integration Guide

The `qualia-core-db` package provides a highly optimized, `no_std`-compatible WebAssembly target (`wasm32-unknown-unknown`). This enables secure, client-side semantic web querying, Local LLM inference, and Fiduciary Cryptography directly within browser sandboxes or edge computing environments.

This document details the exposed WASM bindings and the **Native-First Offloading** architecture.

---

## 1. Building the WASM Target

To compile the `qualia-core-db` crate for the browser:

```bash
cargo build --target wasm32-unknown-unknown
# OR via wasm-pack for JS/TS bindings
wasm-pack build crates/qualia-core-db --target web --out-dir pkg
```

**Features enabled for WASM:**
- `serde-wasm-bindgen`, `js-sys`, `web-sys` (with `WebSocket`, `Event`, `MessageEvent` APIs)
- The OPFS (Origin Private File System) is used automatically for BIDX caching.

---

## 2. LLM Inference & The Extension Bus

Local LLM Inference in the browser presents unique challenges due to memory limits (typically 2GB-4GB per tab) and the single-threaded nature of the JavaScript event loop. Qualia solves this via the **Native-First WASM-LLM Offloading** architecture.

### 2.1 The Sync/Async Impedance Mismatch

The core `qualia-core-db` engine is strictly synchronous to maintain zero-allocation invariants in the hot paths. However, blocking the WASM thread blocks the JS event loop, causing the browser tab to freeze.

**Solution:** The `ExtensionBus` acts as an asynchronous bridge. When the UI requests inference, the WASM layer intercepts the call, forwards it to a local Qualia Native Daemon via WebSocket (`ws://127.0.0.1:4242`), and **returns immediately**.

### 2.2 Initializing the Bus

Before calling inference, the UI must initialize the bus. This establishes the WebSocket connection and completes the `did:q42` handshake.

```javascript
import init, { init_extension_bus } from './pkg/qualia_core_db.js';

async function bootstrap() {
    await init();
    
    // Connect to the Qualia Native Daemon (if running)
    // The DID is used for the Sanctuary Mode handshake
    init_extension_bus("did:q42:local-user");
}
```

### 2.3 Streaming Inference

The core streaming function expects a `'static` callback. When the Extension Bus receives tokens from the native daemon, it asynchronously invokes this callback.

```javascript
import { infer_local_model_streaming } from './pkg/qualia_core_db.js';

function triggerInference() {
    const prompt = "Explain quantum cryptography.";
    const graphContext = "{}"; // JSON serialized semantic graph context
    
    // This call returns < 1ms, preventing UI freeze
    infer_local_model_streaming(prompt, graphContext, (tokenDelta) => {
        // This callback is fired asynchronously as tokens arrive
        document.getElementById('output').innerText += tokenDelta;
    });
}
```

### 2.4 Fallback Behavior
If the Qualia Native Daemon is not running on port `4242` or the user denies the connection, `qualia-core-db` will gracefully fall back to the in-browser **WebGPU** execution pipeline (subject to browser RAM limits).

---

## 3. Fiduciary Cryptography & Agency Validations

All governance structures are fully operational in the WASM build.

### Intent Mediation (`AgentIntent`)
Before the LLM is allowed to execute a prompt, the intent is validated against the active `CapabilityProfile`.

```rust
// Exposed to JS via #[wasm_bindgen]
pub fn validate_intent_wasm(intent_json: &str) -> Result<String, JsValue> {
    // ... Returns serialized WebizenVerdict (Allow, Deny, Sanitise, etc.)
}
```

### Key Management
Ed25519 signatures and FNV-1a IRI hashing are strictly identical across WASM and Native builds, guaranteeing cryptographic determinism for multi-party contracts.

---

## 4. Architectural Rules for WASM Integration

1. **No SharedArrayBuffer Requirements:** The Extension Bus utilizes `std::thread_local!` and `std::cell::RefCell` to store closures securely without requiring multi-threading headers, ensuring cross-origin isolation (COOP/COEP) constraints are not strictly required for standard operations.
2. **Sanctuary Mode Enforcement:** Any request for Classified (0x02) data routed over the `ExtensionBus` must be accompanied by a valid `did:q42` signature. The native daemon will drop unauthenticated connections.
