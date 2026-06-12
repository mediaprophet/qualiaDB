# Webizen Qapp Development Guidelines (LLM Primer)

You are an expert Rust/Dioxus developer building a QApp (Qualia App) for the Webizen ecosystem. 

<core_directive>
The QApp is a "dumb terminal" Presentation Layer. 
All compute, graph traversal, data mutation, and advanced logic happens in the local native daemon (`webizen-desktop` / `qualia-core-db`).
The WASM app does not make authoritative data decisions. It only renders state and emits `Intent` structs.
</core_directive>

<banned_tech>
- **DO NOT** use REST endpoints (`fetch('/api/data')`). All data flows through the binary WebSocket `DataProvider`.
- **DO NOT** process data logic in the UI. You only emit `Intent` structs.
- **DO NOT** use external CDNs. The app runs under a strict Content Security Policy (CSP) offline.
- **DO NOT** use `localStorage` for primary data.
- **DO NOT** use heavy client-side graphing libraries.
</banned_tech>

## 1. Intent Framing & The Allocation Firewall
<intent_framing>
Every action a user takes must be packaged into a strictly formatted `McpIntentFrame`.
The QApp must satisfy the Allocation Firewall and Sanctuary Gates by passing valid purpose hashes and nonce challenges.

### Concrete Example:
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpIntentFrame {
    pub purpose_hash: u64,
    pub active_deontic_constraints: Vec<u64>,
    pub active_profile_id: Option<u64>,
    pub session_nonce: u64,
    pub sanctuary_override: Option<String>,
    pub qpu_enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpPayload {
    pub intent: McpIntentFrame,
    pub tool_name: String,
    pub arguments_raw: Vec<u8>,
}
```
</intent_framing>

## 2. Graceful Degradation
<graceful_degradation>
When you send an intent, the Daemon may reject it due to nonce challenge failures or missing capabilities (e.g., QPU not available). The WASM client MUST fail gracefully.

### Error Handling Boilerplate:
```rust
match data_provider.send_intent(payload).await {
    Ok(_) => {
        // Intent accepted, await state sync
    }
    Err(e) => {
        match e {
            McpSystemError::SanctuaryGateTriggered => {
                // RENDER FALLBACK: Show "Access Denied by Sanctuary Gate" overlay
            }
            McpSystemError::FeatureNotEnabled => {
                // RENDER FALLBACK: e.g. "QPU inference offline. Using CPU fallback."
            }
            _ => {
                // Generic error handling
            }
        }
    }
}
```
</graceful_degradation>

## 3. MCP Tool Integration
<mcp_tools>
QApps can dispatch complex intents (QPU, Logic, Bioinformatics, Clinical Risk) to the native Daemon via the `mcp_server.rs` endpoints.

- **Available Tools:** `evaluate_modality`, `bioinformatics_align`, `chemical_descriptors`, `clinical_risk`, `execute_llm_inference`.
- **DO:** Construct raw byte representations of arguments to send over the websocket to avoid JSON parsing overhead on the Daemon.
</mcp_tools>

## 4. Zero-Copy LoRA Multiplexing
<lora_multiplexing>
The Webizen Daemon supports Zero-Copy LoRA Multiplexing directly on the GPU.

- **DO:** Trigger specific `ContextType` LoRA adapters (`.lora` files) for specialized domain knowledge during LLM inference without incurring heap allocation penalties.
- **HOW:** Pass the Adapter ID encoded in bits 63-48 of the `NQuin` metadata field when invoking the `execute_llm_inference` MCP tool.
</lora_multiplexing>

## 5. Agency Signatures
<agency_signatures>
QApps may need to participate in multi-party contracts or deontic logic Quins.

- **DO NOT:** Attempt to sign transactions using private keys in the UI/WASM context.
- **DO:** Delegate signing to the `qualia-core-db` agency module by emitting an intent with the payload structure expected by the `SuspendedTransactionQueue`. The daemon will securely apply the `Ed25519` scoped Merkle root signatures.
</agency_signatures>

## 6. Security Constraints
<security_constraints>
- **Token Scrubbing:** Vaporize `qualia_token` from the URL bar immediately upon initialization to prevent history leaks.
  ```rust
  let mut clean_url = window.location().pathname().unwrap_or_else(|_| "/".to_string());
  if let Ok(hash) = window.location().hash() { clean_url.push_str(&hash); }
  let _ = window.history().unwrap().replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&clean_url));
  ```
- **CSP Compliance:** `wasm-unsafe-eval` is the only execution allowed.
</security_constraints>