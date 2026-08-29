# POET Architecture Overview: Network Operating System & Habitat Environment

**Document ID:** `POET-SPEC-000`  
**Status:** Canonical Architectural Specification  
**Scope:** Whole-system architecture for POET, Webizen-Desktop, the Qualia Semantic Substrate, and third-party habitats.

---

## 1. The Network Operating System (NOS) Paradigm

POET is not a conventional single-purpose application or a thin database CRUD viewer. It is a **Spatial Network Operating System (NOS) Environment** (referred to as a *Semantic Habitat*). 

Unlike legacy operating systems that manage files and processes on a single physical machine, and unlike cloud web apps that centralize user data on corporate servers, the Webizen NOS operates across a decentralized, peer-to-peer semantic fabric governed by four foundational pillars:

```
+-----------------------------------------------------------------------------------+
|                           THE 4 PILLARS OF WEBIZEN NOS                            |
+--------------------------+--------------------------+-----------------------------+
| 1. Decentralized IDs     | 2. Zero-Heap Semantics   | 3. Local-First Execution    |
| Decentralized Identifiers| 48-byte Super-Quin       | Autonomous offline runtime, |
| (DIDs), acting like URI- | bit-packed substrate,    | peer-to-peer syncing, local |
| schemed UUIDs across an  | 42MB Prolog Sentinel     | GGUF/P64 AI, zero cloud     |
| array of artifacts.      | safety guarantee.        | vendor lock-in.             |
+--------------------------+--------------------------+-----------------------------+
|                           4. Deontic Governance & Fiduciary Integrity             |
| Formal machine-verified rights, obligations, and contracts (Obligate/Permit/      |
| Forbid) with mathematically auditable evidence and dispute resolution.            |
+-----------------------------------------------------------------------------------+
```

### 1.1 Decentralized Identifiers (DIDs), Linked RDF Documents & Dual-Layer Serialization

The design of Decentralized Identifiers (DIDs) in this system directly inherits the architectural model originated in W3C Web Payments and formalized in the W3C Credentials Community Group (2014, co-founded by Timothy Holborn):

1. **Decentralized Identifier Schema:**  
   A DID acts as a decentralized identifier (much like a UUID with a standard URI scheme). An end-user (the natural person) generates and holds an **array of DIDs** associated with various artifacts, verification keys, roles, interaction channels, and context graphs.
2. **Every DID Binds an Attached RDF Document:**  
   Every DID resolves to an attached **RDF Graph Document** (DID Document). Crucially, this RDF document can recursively reference other Decentralized Identifiers, establishing a verifiable linked-data graph of entities, verification methods, service endpoints, cryptographic public keys, and artifact associations.
3. **Dual-Layer Serialization Architecture:**  
   The system enforces a clear separation of concerns between compute efficiency and human usability:
   - **Internal / Compute / Storage Layer (Compact & High-Performance):**  
     Encoded in **CBOR-LD** and/or **N3 / Super-Quin binary arrays (`[NQuin]`)**. This representation is mathematically deterministic, zero-heap compliant, memory-bounded (42MB Sentinel), and optimized for SIMD/GPU parallel compute pipelines.
   - **Human Presentation Layer (Readable & Expressive):**  
     Dynamically serialized and rendered on demand into **Turtle (`.ttl`)**, JSON-LD, or interactive visual graph trees. This ensures transparency and human-readability without burdening high-speed evaluator loops with string serialization overhead.
4. **Natural Person Identity Synthesis (Anti-Reductionism):**  
   Because the human user possesses an array of DIDs linked to diverse artifacts across the semantic graph, this relational substrate provides the mathematical foundation upon which algorithms can later model, correlate, and verify the multi-faceted identity and provenance of the human being.

### 1.2 The Provenance of DIDs: Cool URIs, Multi-Transport Resolution & Permissive Commons

To ensure architectural integrity, the system design formally incorporates the foundational motivation behind Decentralized Identifiers:

```
+-----------------------------------------------------------------------------------+
|                     THE COOL URI & MULTI-TRANSPORT EVOLUTION                      |
+-----------------------------------------------------------------------------------+
|                                                                                   |
|   Legacy HTTP-Locked Web (Fragile)                                                |
|   https://domain.org/path/resource ---> Domain lapses / 404 ---> Semantic Rot     |
|                                                                                   |
|   Decentralized Identifier & Permissive Commons Resolution (Resilient)            |
|   did:method:12345 (Persistent Cool URI)                                          |
|         |                                                                         |
|         +---> Resolves via HTTP Gateway (Fallback)                                |
|         +---> Resolves via WebTorrent / BitTorrent (Distributed Media)            |
|         +---> Resolves via IPFS / Content-Addressed Storage (Immutable Assets)    |
|         +---> Resolves via Git Repositories (Versioned Source & Schema)           |
|         +---> Resolves via Local Peer-to-Peer Mesh (Offline Autonomous)           |
|                                                                                   |
+-----------------------------------------------------------------------------------+
```

1. **The Cool URI Problem & Semantic Link-Rot:**  
   As Tim Berners-Lee articulated (*"Cool URIs don't change"*, W3C), the Semantic Web requires persistent URIs to ground ontologies, namespaces, and knowledge graphs. However, the traditional Web is structurally tethered to HTTP hostnames. When servers go offline, companies dissolve, or domain registrations lapse, URLs rot—breaking semantic graph links and fracturing access to the underlying resources.
2. **Permissive Commons & Non-HTTP Resource Transport:**  
   Humanity's shared knowledge (commons informatics) must not depend on fragile centralized web hosts. DIDs were architected as persistent, protocol-independent identifiers capable of resolving semantic RDF documents across non-HTTP distributed transports, including **WebTorrent, Git, IPFS, and local peer-to-peer storage meshes**.
3. **Anti-Reductionism & Human Dignity:**  
   The system strictly rejects reductionist paradigms that attempt to define or collapse a natural human being into a single static serial number or identifier. A human being is never an identifier; a human being holds an evolving array of decentralized identifiers bound to artifacts, relationships, and context graphs. This distinction protects human rights and ensures that identity is modeled dynamically through multi-dimensional relations rather than coercive single-token labeling.

---

## 2. System Topology: Webizen-Desktop & POET Habitat

The architecture divides responsibilities cleanly between the **Administration & Host Platform** (`webizen-desktop`) and the **Spatial User Environment** (`poet`):

```
+===================================================================================+
|                     WEBIZEN-DESKTOP (Node Administration Hub)                     |
+-----------------------------------------------------------------------------------+
| - System Tray & Lifecycle Supervisor (Daemon startup, health, background services)|
| - Hardware & Resource Monitor (CPU, GPU VRAM, RAM, Thermal Governor, 42MB Sentinel)|
| - Node Configuration & Keystore Vault (DID Key Management, Storage Paths, Network)|
| - Habitat & Application Package Manager (Install, Update, Isolate, Launch Habitats)|
| - Embedded Browser Subsystem (Sandboxed Webview host for web apps & legacy web)   |
| - Local Loopback & Inter-Process Daemon Bridge (HTTP / SSE / WebSocket / IPC)     |
+===================================================================================+
                                         |
               +-------------------------+-------------------------+
               |                                                   |
               v [Desktop Native Mode]                             v [Browser WASM Mode]
+===================================================================================+
|                      POET HYPERCANVAS (Flagship User Habitat)                     |
+-----------------------------------------------------------------------------------+
| - Chora Spatial Infinite Canvas (2D / 2.5D / 3D Containers & Manifolds)           |
| - 4-Way Tool-Chest, Palettes, Radial Menus, and 4D Temporal Scrubbing Ribbon      |
| - Project Delivery Workspace (Interactive Kanban, Task Graph, Gantt, Economics)   |
| - Creative Studio Workspace (Multi-Track Audio Mixer, 3D Scene Graph, Shaders)    |
| - Governance & Agreement Workspace (Contract Builder, Deontic Logic, Disputes)    |
| - Social & Communications Suite (Threaded Chat, Mentions, Library Attachments)    |
| - Person-Controlled Health Workspace (Timeline, Vitals Trends, Granular Consent)  |
| - Knowledge & Semantic Library (Visual Graph Explorer, Ontology Ingest & Mappings)|
| - Embedded Webview Containers (Sandboxed web browser frames inside canvas)        |
+===================================================================================+
```

---

## 3. Dual-Launch Runtime Model

Every habitat built for the Webizen NOS — starting with POET — must support a seamless **Dual-Launch Model**:

### 3.1 Desktop Windowed Mode (Native)
- Hosted through `webizen-desktop` (Tauri / native windowing).
- Direct access to local filesystem workspaces, direct GPU acceleration (`wgpu`, WGSL compute pipelines), process-wide resident GGUF/P64 model weights in VRAM, and system-level device interfaces (audio capture, hardware inputs).
- Operates with zero cloud round-trips.

### 3.2 Browser Web Mode (WASM)
- Hosted in any modern web browser via WebAssembly (`wasm32-unknown-unknown`) and web standards (`web-sys`, WebGPU/WebGL2).
- Connects transparently to the local `qualia` daemon over loopback (`http://127.0.0.1:8080`) via REST, Pulse Server-Sent Events (SSE), and WebSockets.
- Gracefully adapts feature availability when running standalone without a local daemon (falling back to in-browser storage and client-side compute shaders).

---

## 4. Habitat Extensibility & Package Ecosystem

POET is the standard, flagship habitat, but the NOS architecture is strictly open and unbundled:
1. **Package Specification (`.hcf` / `.hmc`):** Habitat packages are cryptographically signed, checksummed CBOR/HCF containers containing UI manifests, view definitions, semantic lens bindings, and asset descriptors.
2. **Third-Party Habitats:** Any developer or community can construct alternative habitats (e.g., specialized CAD workspaces, clinical EHR interfaces, gaming/simulation spaces, or minimalist productivity docks) and distribute them as installable packages.
3. **Installation & Isolation:** `webizen-desktop` acts as the package manager, verifying publisher DIDs, inspecting permission/capability grants, and executing habitats within isolated security contexts.

---

## 5. Embedded Browser Subsystem Architecture

Modern users require access to the wider web alongside native spatial habitats. The NOS incorporates an **Embedded Browser Subsystem**:

```
+-----------------------------------------------------------------------------------+
|                            EMBEDDED BROWSER SUBSYSTEM                             |
+-----------------------------------------------------------------------------------+
|  Desktop Admin Mode:                                                              |
|  - Full sandboxed Chromium / Webview browser tab inside Webizen-Desktop.          |
|  - Bookmark manager, history audit, and semantic page text extraction pipeline.   |
|                                                                                   |
|  POET Canvas Mode:                                                                |
|  - Sandboxed <WebviewContainer> rendered as a draggable 2D/3D spatial surface.    |
|  - Web-page to Semantic Library bridge: One-click "Ingest Page as Knowledge Quin".|
|  - NLP Gazetteer live extraction on rendered HTML text nodes.                     |
+-----------------------------------------------------------------------------------+
```

---

## 6. Architecture Requirements

| Requirement ID | Title | Description | Target Component |
|---|---|---|---|
| `POET-NOS-001` | **Decentralized NOS Model** | System must run completely local-first with DID-based addressing, zero required cloud dependencies, and peer-to-peer data transport. | `qualia-core-db`, `poet` |
| `POET-NOS-002` | **Dual-Launch Parity** | The POET UI must compile and run identically in Desktop native windows (Tauri) and Browser WASM without code duplication. | `crates/poet`, `webizen-desktop` |
| `POET-NOS-003` | **Desktop Admin Refactoring** | `webizen-desktop` must function as a dedicated node administration hub, hardware monitor, settings manager, and habitat package manager. | `crates/webizen-desktop` |
| `POET-NOS-004` | **Packaged Habitat Deployment** | POET and alternative user-authored habitats must be packagable, installable, and launchable via standardized `.hcf` packages. | `crates/webizen-desktop`, `crates/poet` |
| `POET-NOS-005` | **Embedded Webview Integration** | Embedded browser windows must be launchable within Desktop tabs and within spatial POET canvas containers with semantic page ingest. | `webizen-desktop`, `crates/poet` |
| `POET-NOS-006` | **Zero-Heap Core Integrity** | Hot semantic processing loops must maintain zero heap allocations and obey the 42MB Sentinel limit. | `qualia-core-db` |
| `POET-NOS-007` | **Real-Time Pulse Mesh** | Events and telemetry must broadcast across daemon subscribers and UI containers via Pulse SSE and WebSocket channels. | `qualia-core-db`, `poet` |
| `POET-NOS-008` | **DID RDF Document & Dual Serialization** | Every DID must bind an attached RDF document (with recursive references) using CBOR-LD/N3 for compute and Turtle for users. | `qualia-core-db`, `poet` |
| `POET-NOS-009` | **Multi-Transport Cool URI Resolution** | Support persistent semantic resource resolution across non-HTTP transports (Git, WebTorrent, IPFS, local peer mesh) for permissive commons. | `qualia-core-db`, `poet` |
