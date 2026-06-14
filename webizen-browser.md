# Webizen Browser: Project Plan

> [!NOTE]
> **Vision:** To build a next-generation browser that doesn't just render HTML, but actively interprets the semantic, epistemic, and deontic layer of the web using the QualiaDB local engine, anchored deeply in the ontological agency of the natural person.

## 1. Architectural Overview

Instead of maintaining a massive C++ browser engine fork, the Webizen Browser uses a "Headless Engine + Custom Shell" architecture, leveraging Rust from end to end.

* **The Web Engine (Rendering):** OS-Native WebView (WebView2 on Windows, WebKit on macOS/Linux) provided by Tauri. Zero maintenance required.
* **The UI Shell (Chrome):** Built with **Dioxus**, providing an incredibly lightweight, virtual-DOM-free 100% Rust frontend for managing tabs, the omnibox, and Webizen-specific sidebars.
* **The Core (Backend):** A pure Rust backend powered by Tauri, embedding the **QualiaDB 42MB Sentinel**.
* **The Zero-Heap Bridge (Custom Protocol):** Bypassing Tauri's standard JSON-RPC IPC bottleneck entirely. The frontend content scripts pre-hash DOM elements into `NQuin` structures and send them via `fetch` to a registered custom protocol (`qualia://`). The Rust backend intercepts these natively, feeding them directly into the Webizen Bytecode VM without string serialization overhead.

---

## 2. Core Webizen Features (The "Why")

* **Epistemic Badging & Semantic Provenance:** Local LLMs (via `gguf_sharder.rs`) extract claims. The browser displays a subtle omnibox badge or sidebar counter for detected contradictions. Users can toggle a deep-dive overlay to see the Dialectical Synthesis. Crucially, this UX includes a **Provenance Tree**, visually rendering the cryptographic signatures or WebIDs of the asserting entities so the user understands *who* is making the claim.
* **Human-Centric Agency (Anti-SSI):** The identity layer explicitly rejects the commercialized "Self-Sovereign Identity" (SSI) and wallet-as-identity models. It enforces the legal and ontological agency of the **natural person**. Deontic contracts are treated as revocable delegations of human rights, binding Verifiable Credentials (VCs) directly to the user's W3C WebID.
* **ACP-Compliant Deontic Contracts:** Intercepts N3/SHACL rules and evaluates them using `deontic_logic.rs`. It translates internal `OP_OBLIGATE` and `OP_PERMIT` states directly into W3C **Access Control Policy (ACP)** structures. By utilizing granular Policies and Matchers (`allOf`, `anyOf`, `noneOf`), the browser generates highly expressive, interoperable access constraints mapping perfectly to human-centric legal boundaries on modern Solid pods.
* **Two-Tier Persistence & Aggregated Sync:**
  * *Tier 1 (Ephemeral & Real-Time):* Scraped text and tokenized claims exist in the `SlgArena`. Concurrent semantic updates are handled via local CRDTs. If conflicting updates arrive from different devices, `OP_ISOLATE` traps them, treating them not as destructive overrides, but as Dialectical Synthesis opportunities for the user.
  * *Tier 2 (Persistent Vault):* Processed claims and accepted contracts are committed to the sovereign Knowledge Graph. A **Solid Patch Aggregator** buffers these SQLite state changes in a local Write-Ahead Log (WAL). To prevent rate-limiting and pod churn, the engine runs a local optimization pass every 30-60 seconds to compress redundancies before issuing a single, cohesive N3 `PATCH` payload to the remote Solid Pod.

---

## 3. Implementation Phases

### Phase 1: Foundation & Shell Prototype (Weeks 1-2)
**Goal:** Achieve basic web browsing capabilities within a custom UI.
* Initialize a new Tauri project using **Dioxus**.
* Implement the core browser UI (Tabs, Omnibox, Back/Forward controls).
* Wire the UI to spawn and manage Tauri WebViews for each tab.
* **Milestone:** A functional, lightweight browser that can load and navigate standard websites (~50MB initial binary size).

### Phase 2: QualiaDB Engine & Model Hydration (Weeks 3-4)
**Goal:** Embed the QualiaDB engine and establish the Zero-Bloat Hydration Strategy.
* Add `qualia-core-db` to the Tauri backend.
* Initialize the 42MB `SlgArena` and the Webizen Bytecode VM.
* Implement the `qualia://` custom URI scheme protocol.
* **Model Hydration:** Implement a background worker thread that downloads the necessary GGUF LLM weights (~1.5GB-4GB) on first launch to the local AppDir. Utilize strict `mmap` protocols so inference reads directly from disk without allocating massive RAM blocks.
* **Milestone:** The browser can parse DOM content, pre-hash it, pipe it over the custom protocol, and run `mmap` LLM inference seamlessly.

### Phase 3: The Semantic Overlay UX (Weeks 5-6)
**Goal:** Surface Qualia's logic evaluations in the user interface.
* Build the **Webizen Sidebar** and omnibox badge components in Dioxus `rsx!`.
* Hook up the Epistemic Logic module (`OP_KNOWS`, `OP_BELIEVES`) to asynchronously push certainty scores.
* Implement the deep-dive toggle to visually highlight contradictions and display the **Provenance Tree**.
* **Milestone:** Users see subtle contextual indicators of page truthiness and can expand them for full Dialectical Synthesis with entity provenance.

### Phase 4: Web3, Solid, ACP & Agency (Weeks 7-8)
**Goal:** Anchor identity and handle digital agreements natively.
* Integrate **W3C Solid** protocols, **WebID** resolution, and Verifiable Credentials.
* Natively translate `deontic_logic.rs` evaluation into standard **ACP** policies and matchers.
* Build the native "Consent/Contract" modal replacing legacy cookie banners.
* Implement the **Solid Patch Aggregator** background sync engine to decentralized pods.
* **Milestone:** The browser acts as an agent representing a natural person, seamlessly negotiating and securely aggregating ACP permissions to decentralized pods.

---

## 4. Technical Constraints & Rules

> [!IMPORTANT]
> **Strict Adherence to Qualia Architecture**
> * **Custom Protocol Bypass:** Transmit `NQuin` blobs via `qualia://` `fetch` requests to absolutely prevent IPC JSON-RPC heap allocations.
> * **Ontological Agency:** Identity must prioritize the legal rights of the natural person. Strictly separate WebID/VC delegations from pure cryptocurrency wallet signing mechanics.
> * **Zero-Bloat Install:** The core application binary must remain small. Model weights must be dynamically hydrated and memory-mapped (`mmap`) off-disk.

---

## 5. Resource Management & Graceful Degradation

To prevent the background paraconsistent evaluations and local LLM inference from draining laptop batteries or saturating CPU cycles, the browser implements strict operational modes driven by the OS thermal and power state APIs:

| Operational Mode | Epistemic Layer Status | Deontic Layer Status | Sync Frequency |
| --- | --- | --- | --- |
| **Full Synthesis** | Real-time LLM parsing & Provenance Trees | Active ACP Translation | Continuous Real-time |
| **Eco-Interpretive** | LLM paused; deterministic logic cache only | Active ACP Translation | Batched (5-minute intervals) |
| **Sovereign Core** | Local parsing paused completely | Local ACP matching only | Deferred until plugged into power |
