# Developing Apps on Qualia-DB

Qualia-DB is not just a database; it is a full-stack engine designed to enforce a **Principal-Agent Duty of Care**. When you build a "Qualia App" (like the Qualia Desktop Agent), you are building an interface that acts exclusively on behalf of the user, bounded by strict hardware laws.

This guide covers how to architect and build applications on top of the Qualia-DB core using the Tauri and React framework.

---

## 1. The Tauri Architecture (Rust Backend + UI Edge)

Qualia Apps utilize **Tauri** to provide a cross-platform, lightweight desktop UI while keeping the heavy lifting securely inside the native Rust daemon. 

### Why Tauri?
- **Zero-Copy IPC**: The React frontend (UI Edge) and the Rust backend (Daemon) communicate instantly via Tauri event pipes, keeping memory footprints negligible.
- **WASM Memory Constraints**: Tauri allows the frontend to run as a native webview, while the Rust backend tightly controls the `< 512MB` WASM memory floor, preventing bloated electron-style architectures.

### Communicating with the Engine
The UI Edge triggers commands in the Rust backend using Tauri's `invoke`.
```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Trigger native ingestion on the Rust daemon
await invoke('ingest_image_async', { filePath: '/path/to/asset.png', typology: 'Meme' });
```

---

## 2. Building Neurosymbolic UI Pipelines

When building features like the **Asset Library**, your app is responsible for capturing context (Spatio-Temporal Qualia) and passing it to the backend engine for processing.

### Typology Routing
Rather than letting black-box LLMs guess context, Qualia Apps use **Typology Routing**. The UI explicitly captures the human's intent (e.g., "This image is Heraldry") and passes that semantic lens down to the Rust layer.

1. **UI Selection**: The user selects a "Typology Lens" (e.g., `Meme Engine`, `Heraldry Engine`).
2. **Backend Daemon**: The Rust daemon uses this lens to route the extraction logic, dynamically targeting specific semantic facets (e.g., `Irony Tensor` vs `Tincture`).
3. **Event Emitting**: The daemon emits the structured payload back to the UI asynchronously, preventing the React thread from blocking.

---

## 3. Hardware Orchestration & Telemetry

A core philosophy of Qualia Apps is **Mechanical Sympathy**—exposing the physical realities of the hardware to the user. Applications should never hide memory limits or resource usage.

### Structuring Background Daemons
If your app performs background tasks (like the Nym Privacy Relay or ZK-STARK Proving), the Rust backend must proactively emit event states down to the client.

1. **Atomic Toggles**: Use `Arc<AtomicBool>` to control background `tokio` threads.
2. **Event Pipes**: Use `tauri::Window::emit` to push real-time telemetry (RAM backpressure, Solar Wattage, Disk Paging) to the frontend.
3. **Boundary Visualization**: In React, map these telemetry streams to visual gauges. If the backend hits a hard 50MB backpressure limit and drops packets, the UI should flash red and explicitly show the dropped packets to prove boundary compliance.

### Example: Listening to Telemetry in React
```typescript
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
  const unlisten = listen('nym-telemetry', (event) => {
    // Update local React state with native hardware metrics
    setMetrics(event.payload);
  });
  return () => { unlisten.then(f => f()); };
}, []);
```

---

## 4. Multi-Seed Identity & Credentials

Qualia Apps do not use centralized accounts. They manage **Cryptographic Human Agency Records** locally.
- Build Multi-Seed managers that allow users to import BIP39 mnemonics.
- Never store raw seeds unencrypted.
- Offload HD Wallet derivation directly to the Rust backend to ensure memory-safe cryptographic generation across different network topologies (Bitcoin, eCash, Nym).

---

By adhering to these principles, your Qualia App guarantees that the human user remains in total control of their data, their hardware, and their digital agency.
