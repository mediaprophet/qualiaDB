# Qualia Apps (Qapps) Specification

_Branch: `0.0.11-dev` | Last updated: 2026-06-11_

This document outlines the architecture, packaging, and edge-deployment specifications for Qualia Apps (Qapps). A Qapp is an installable, sovereign WebAssembly (WASM) application designed to operate within the Qualia-DB ecosystem.

## 1. Qapp Architecture

Qapps bridge the gap between traditional PWAs and fully native applications by leveraging modern Web APIs to interact securely with the Qualia-DB engine. 

- **Tethered vs. Sovereign Modes**: Depending on the platform, a Qapp functions either as a "Tethered Glass" (a thin client relying on a Desktop Hub) or a "Tier-1 Sovereign Edge Node" (maintaining a local persistent Write-Ahead Log).
- **Packaging**: Qapps are packaged as signed `.zip` archives containing the WASM payload, static assets, and a manifest.
- **Installation**: Qapps are verified via cryptographic signatures (`credentialSig`) and installed into the Webizen Qapp Vault.

---

## 2. Android WASM Edge (Tier-1 Sovereign Node)

Recent updates to Android Chrome (v132+) have enabled the File System Access API (`showDirectoryPicker`) for installed Progressive Web Apps, elevating Android WASM Qapps from thin clients to Tier-1 Sovereign Edge Nodes.

### Build Specifications

To compile a Qapp for the Android Edge, the following Cargo feature flags are recommended:

```toml
[features]
default = [
    "android_pwa_edge",     # Enables File System Access API bridging
    "wal_persistence",      # Compiles the core WAL engine
    "crdt_dvv_eae",         # Full conflict resolution for offline edits
    "websocket_streamer",   # For LLM inference delegation to the desktop
    "wasm_simd"             # For vectorised logic evaluation on the phone CPU
]
# Excludes: local_gpu_inference (RAM ceiling), ebpf_firewall
```

### Feature Support Matrix for Android WASM:

| Component | Status | Details |
|---|:---:|---|
| Storage Engine | ✅ | Full local `qualia-core-db` execution. `.q42` SuperBlocks are written directly to the Android user file system. |
| Webizen VM | ✅ | Full N3Logic and SHACL evaluation runs locally on the phone's CPU via WASM SIMD. |
| LLM Inference | ❌ | Delegated. OS limits (JetSam/OOM) prevent local WebGPU loading of GGUF models. Inference is routed over the SocialWebNet tunnel to the desktop hub. |
| Network Sync | ✅ | The CRDT engine runs locally, queuing changes in the local WAL and syncing via WebSocket when the desktop hub is reachable. |

---

## 3. Android File System Implementation Architecture

To bypass the browser's volatile Origin Private File System (OPFS) and write directly to the Android disk, the WASM bridge must implement a specific initialization flow:

### 3.1. Sovereign Vault Initialization
On the first run, the user must explicitly grant the Qapp access to a directory. 
- **Action**: Dioxus UI prompts the user to "Bind Local Folder" and calls `window.showDirectoryPicker({ mode: 'readwrite' })`.
- **Result**: The OS file picker appears, and the user selects a secure folder.

### 3.2. The IndexedDB Handle Vault
Because the Qapp is installed (added to Home Screen), Chrome grants persistent permissions. 
- The `FileSystemDirectoryHandle` is saved cryptographically into the browser's IndexedDB.
- **Wake Cycle**: On every subsequent launch, the WASM engine retrieves the handle and calls `requestPermission({ mode: 'readwrite' })`. This is auto-approved by Chrome silently.

### 3.3. Zero-Allocation Synchronous Writes
Standard JS file APIs are asynchronous and incompatible with the zero-allocation Webizen VM.
- **Execution**: The directory handle is passed into a Web Worker.
- **Access**: The Web Worker invokes `createSyncAccessHandle()`, providing a high-performance synchronous file descriptor. This allows the Rust engine to seek, read, and write exact byte offsets in `.q42` SuperBlocks without blocking the main UI thread.

---

## 4. Operational Friction & Mitigations

Android imposes strict behavioral limits on PWAs that Qapps must engineer around:

1. **Background Execution Death**: Android freezes Web Workers within minutes of the app being backgrounded.
   - **Mitigation**: All CRDT resolution and WAL flushing must execute aggressively in the foreground, hooked to the `visibilitychange` API to panic-flush on minimize.
2. **Storage Quota Deception**: `navigator.storage.estimate()` is inaccurate for user-selected folders.
   - **Mitigation**: Implement robust `QuotaExceededError` handling during WAL flushes, as the physical disk may be full.
3. **App Data Clearance**: If a user clears app data via Android Settings, the IndexedDB vault is erased. The `.q42` files remain intact, but the Qapp loses the directory pointer.
   - **Mitigation**: The initialization sequence must detect a missing handle and prompt the user to "Re-link your existing database folder" rather than creating a new one.

---

## 5. Qapp Vault and Packager

The Qualia Desktop Environment includes a **Qapp Vault** for sideloading and managing Qapps. 

### Qapp Archive Format (`.qapp.zip`)
- **`manifest.json`**: Declares required capabilities, intent permissions, and the application's unique `did:q42` identifier.
- **`payload.wasm`**: The compiled application logic.
- **`assets/`**: Static CSS, fonts, and images.
- **`signature.sig`**: Ed25519 cryptographic signature proving the origin of the Qapp, signed by the developer's Decentralised Identifier.

The Webizen desktop daemon exposes APIs to `verifyAndInstallQapp` ensuring that only mathematically proven and safely permissioned Qapps are loaded into the Principal's ecosystem.
