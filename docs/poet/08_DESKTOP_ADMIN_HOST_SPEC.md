# Webizen-Desktop Administration Panel & Embedded Browser Specification

**Document ID:** `POET-SPEC-008`  
**Status:** Canonical Platform Specification  
**Scope:** Node administration, hardware monitoring, habitat package management, and embedded browser subsystem in `webizen-desktop`.

---

## 1. Overview & Desktop Platform Architecture

`webizen-desktop` is refactored from a tangled application view into a dedicated **Node Administration Hub, Package Manager, and Habitat Host**:

```
+===================================================================================+
|                     WEBIZEN-DESKTOP PLATFORM ARCHITECTURE                         |
+===================================================================================+
|  [Node Administration Panel]    [Habitat Package Manager]   [Embedded Browser]     |
|  - CPU / GPU VRAM / RAM Usage   - Install / Update / Launch  - Sandboxed Webview   |
|  - Thermal Governor Status      - POET & Custom Habitats     - Browser tab bar     |
|  - 42MB Sentinel Integrity      - Permission isolation       - Semantic Ingest     |
|  - DID Key Vault & Keystore     - .hcf package verification  - NLP Gazetteer scan  |
+===================================================================================+
                                         |
                                         v
+-----------------------------------------------------------------------------------+
|  [Daemon Supervisor & Host Bridge] -> Launches & connects local `qualia` daemon,  |
|  manages resident GGUF/P64 VRAM weights, and routes loopback HTTP / SSE / WS.     |
+-----------------------------------------------------------------------------------+
```

---

## 2. Node Administration Panel

The Administration Panel provides full visibility and control over the host node:

### 2.1 Hardware & Runtime Telemetry
- **Resource Gauges:** Live visual meters for Host CPU usage, GPU VRAM allocation (resident model vs. scratch buffers), and system RAM.
- **42MB Sentinel Monitor:** Real-time gauge verifying that hot-path execution memory stays strictly below the 42 × 1024 × 1024 byte ceiling.
- **Thermal Governor:** Active status badge (`Cool`, `Warm`, `Critical`) controlling the 3-core triad execution budget.
- **Daemon Lifecycle Supervisor:** Start, stop, restart, and inspect log streams of the background `qualia` daemon.

### 2.2 Node Settings & Keystore Vault
- **DID Key Vault:** Secure local keystore (`keystore.bin`) management, export/import of DID keypairs, and passkey configuration.
- **Storage & Paths:** Configurable paths for the 42MB SLG Arena, Semantic Library repository, and local GGUF/P64 model storage.
- **Network Configuration:** Local daemon loopback port, peer-to-peer discovery settings, and optional mixnet/relay proxies.

---

## 3. Habitat Package Manager

The Package Manager enables installing, updating, and launching spatial habitats:

### 3.1 Package Operations
- **Packaged Installation:** Import and install signed `.hcf` / `.hmc` habitat packages.
- **Integrity Verification:** Verify cryptographic checksums, author DIDs, and declared permission scopes before activation.
- **Habitat Launcher:**
  - **Launch Desktop Native:** Spawns a dedicated native window with full GPU acceleration.
  - **Launch Browser Web:** Launches the browser-native WASM bundle at `http://127.0.0.1:8080/`.
- **Pre-Packaged Flagship:** POET comes bundled as the default flagship habitat.

---

## 4. Embedded Web Browser Subsystem

The Embedded Browser Subsystem bridges the Webizen NOS with the broader World Wide Web:

### 4.1 Browser Engine & Sandboxing
- Sandboxed Chromium / Webview engine running in isolated memory partitions.
- Full browser controls: Back, Forward, Reload, URL address bar, bookmark manager, and history viewer.

### 4.2 Semantic Web Ingest & Knowledge Extraction
- **One-Click Page Ingest:** Ingest the active web page's text content, DOM hierarchy, and metadata directly into the **Semantic Library** as a knowledge Quin.
- **Live NLP Gazetteer Overlay:** Automatically scan rendered HTML text nodes with the local NLP Gazetteer to highlight recognized entities with clickable IRI chips.

---

## 5. Desktop & Browser Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-ADM-001` | **Node Hardware Telemetry** | Real-time visual gauges for CPU, GPU VRAM, RAM, and Thermal Governor state. | `webizen-desktop` |
| `POET-ADM-002` | **42MB Sentinel Monitor** | Live telemetry monitoring 42MB Prolog Sentinel memory ceiling compliance. | `webizen-desktop`, `qualia-core-db` |
| `POET-ADM-003` | **Daemon Process Supervisor** | Desktop interface to start, stop, restart, and view logs of the local `qualia` daemon. | `webizen-desktop` |
| `POET-ADM-004` | **DID Keystore Vault Hub** | Manage local DID keypairs, import/export keys, and configure encryption passkeys. | `webizen-desktop`, `agency.rs` |
| `POET-ADM-005` | **Habitat Package Manager** | Interface to install, update, inspect permissions, and manage `.hcf` habitat packages. | `webizen-desktop`, `manifest.rs` |
| `POET-ADM-006` | **Dual-Launch Trigger** | Launch habitats in native desktop windows or launch browser WASM interface. | `webizen-desktop` |
| `POET-WEB-001` | **Embedded Webview Host** | Sandboxed web browser tab in Desktop and webview container on POET canvas. | `webizen-desktop`, `crates/poet` |
| `POET-WEB-002` | **One-Click Page Ingest** | Extract and ingest active web page content into the Semantic Library as knowledge Quins. | `webizen-desktop`, `Document.ingest` |
| `POET-WEB-003` | **NLP Gazetteer Web Overlay**| Highlight extracted entity terms directly on rendered web page text. | `webizen-desktop`, `nlp.analyze` |
