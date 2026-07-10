# Qualia Ecosystem Complete Functionality Manual

**Generated On:** 2026-07-09
**Scope:** Entire Project Workspace (QualiaDB + Client Core + Desktop/Studio + Extensions + Wellfare)

Qualia is a zero-allocation, mechanically sympathetic semantic database and multi-agent collaboration ecosystem. This manual provides an exhaustive, up-to-date breakdown of the functionality currently available across the *entire project*, derived directly from the source code and directory indices.

---

## 1. Core Database Engine (`qualia-core-db`)

The foundational semantic graph engine bridges the Semantic Web with hardware-aligned execution paths, enforcing strict constraints to ensure bounded memory and deterministic performance.

*   **Zero-Heap & Super-Quin:** Enforces strict zero-allocation in hot paths. Semantic data is packed into a 48-byte `NQuin` struct.
*   **Storage & Persistence:** Natively integrates APFS clonefile/madvise (`mmap.rs`), Linux ZNS NVMe zone-append operations (`zns_storage.rs`), and a tamper-evident Write-Ahead Log (`wal.rs`) backing all graph mutations. 
*   **Webizen Logic VM:** A bounded, non-recursive execution engine evaluating rules across Answer Set Programming, Deontic Logic (contracts, responsibility, jural correlativity), Epistemic Logic (distributed knowledge, socratic degradation), Paraconsistent routing, Linear Temporal Logic (LTL), Region Connection Calculus (RCC8), and Fuzzy Type-2 sets.
*   **Computational Economics & Financial Modeling:** Comprehensive suite covering macro/micro models, game theory, market design, econometrics, forensic economics, derivatives pricing, asset pricing, behavioral finance, portfolio optimization, and risk management (Monte Carlo VaR, TVM, ILP micropayments).
*   **Advanced Mathematics & Statistics:** Full libraries for statistical computing, machine learning, multi-variable calculus, symbolic algebra (integration, ODEs, limits, series), and high-performance linear algebra.
*   **Other Specialized Domain Engines:** Integrated capabilities for bioinformatics (Smith-Waterman, k-mer, FASTA), physics simulations (kinematics, thermodynamics, PINN), organic & quantum chemistry, and clinical risk evaluation (Framingham, CHA₂DS₂-VASc, SCORE2).
*   **Identity & Cryptography:** Shamir's Secret Sharing, Zero-Knowledge capability proofs, post-quantum ML-DSA signatures, and Verifiable Credentials.

---

## 2. Client API & Local Orchestration (`qualia-client-core`)

The client core acts as the intermediary between the database engine and user-facing applications, managing local agents, capabilities, and external bridging.

*   **Multi-Agent Chat Protocols:** Manages local agent interactions (`chat_agents.rs`), maintaining conversation graphs and citations (`chat_graph.rs`), ontology scoping (`chat_ontology.rs`), and secure relaying (`chat_relay.rs`). 
*   **Local Job Scheduler:** Orchestrates asynchronous background tasks (`local_job_scheduler.rs`) like ontology baking, model downloads, and telemetry packaging.
*   **Model Lifecycle Management:** Native handling of GGUF LLM models (`model_lifecycle.rs`), managing preferences (`model_preferences.rs`), hardware bounds, RAM usage sampling, and active execution contexts.
*   **QApp Ecosystem:** Manages the installation, validation, and registry of modular "QApps" (`qapp_manifest.rs`, `qapp_registry.rs`). Enforces execution boundaries and capability sandboxing (`qapp_api.rs`) via manifest schemas.
*   **QPU Dispatching:** Connects semantic queries to real Quantum Processing Units. Translates requirements into formats for D-Wave, IonQ, Rigetti, IBM, Quantinuum, and Azure Quantum execution (`qpu_dispatcher.rs`, `qpu_oracle.rs`, `qpu_pipeline.rs`).
*   **Social Networking:** Decentralized connection protocols and front-door DIDs (`social_connect.rs`, `dns_resolver.rs`).

---

## 3. Desktop Workbenches (`webizen-desktop` & `webizen-studio`)

The primary graphical user interfaces for deploying, inspecting, and managing the Qualia ecosystem, built with Tauri, Rust, and Dioxus.

*   **Webizen Studio (`webizen-studio`):**
    *   **Pane Registry:** A modular, multi-pane dashboard architecture (`pane_registry.rs`) allowing users to drag, drop, and configure specific views (e.g., Graph visualization, Anatomy Context, QApp explorer).
    *   **Theme Engine:** Dynamic theme resolution and stylesheet compilation for a visually responsive UI (`theme_engine.rs`).
    *   **Canvas Presentation:** Interactive workspace coordinate mapping and layout strategies (`studio_canvas.rs`).
*   **Webizen Desktop Daemon (`webizen-desktop`):**
    *   **System Integration:** Tauri commands mapping UI interactions to local core logic (`main.rs`).
    *   **Telemetry Bridge:** Exposes real-time system metrics to the frontend, including memory pressure, network ripple, epistemic density, quantum activity, and logic flashes (`telemetry_bridge.rs`, `telemetry_hooks.rs`).
    *   **Settings Server & Runtime Health:** Local portal serving configurations, ledger sink diagnostics, and runtime snapshots (`settings_server.rs`, `runtime.rs`).

---

## 4. Hardware & Physics Extensions (`qualia-extensions`)

Experimental bridges to highly specialized execution backends, expanding the core engine's capabilities.

*   **Physics-Informed Neural Networks (PINN):** Integrates native execution of Ternary PINN models for simulating continuous physical domains (`pinn_extension.rs`). Provides boundary condition mapping and equation-type compilation.
*   **Spiking Neural Networks (SNN):** A temporal, neuromorphic network simulator featuring noisy-gradient CRDT weight synchronization, spike encoding, and synaptic plasticity modeling (`snn_extension.rs`).
*   **QPU Extension Bridge:** An alternative API client layer providing granular control over quantum circuit translation and provider pricing models (`qpu_extension.rs`).

---

## 5. Domain Applications: Wellfare Health Vault (`wellfare-core`)

A specialized component demonstrating QualiaDB's capability as a secure, semantic data vault for physiological and health data.

*   **Data Ingestion & Parsing:** Natively parses CSV dumps for weight, sleep, heart rate, and step counts (e.g., Samsung Health exports) (`parser.rs`).
*   **Semantic Translation:** Converts parsed biometrics into structured RDF/Turtle graphs mapping to medical ontologies (`rdf.rs`, `wasm.rs`).
*   **Rule Evaluation:** Uses the core Webizen VM and N3Logic to evaluate health metrics. Flags conditions like *tachycardia*, *sleep debt*, or *adrenal fatigue* using threshold checks (`n3_rules.rs`, `webizen.rs`).
*   **Validation:** Applies SHACL shape validation to guarantee the structural integrity and bounds of medical records (`shapes.rs`).
