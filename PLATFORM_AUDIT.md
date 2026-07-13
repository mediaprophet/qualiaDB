# Webizen Platform — Complete Functionality Audit

**Date:** 2026-07-08
**Repository:** C:\Projects\qualia-27062026 (v0.0.24)
**Purpose:** Catalog every piece of functionality that has been built, identify what works and what's broken, and provide the foundation for building a proper desktop application.

---

## Table of Contents

1. [Platform Architecture](#1-platform-architecture)
2. [10D Epistemic Engine](#2-10d-epistemic-engine)
3. [GPU Rendering Pipeline](#3-gpu-rendering-pipeline)
4. [Computational Geometry](#4-computational-geometry)
5. [Computational Economics](#5-computational-economics)
6. [Logic & Modalities](#6-logic--modalities)
7. [Governance & Webizen VM](#7-governance--webizen-vm)
8. [WellFair — Human Rights & Health](#8-wellfair--human-rights--health)
9. [Wallet & Financial Systems](#9-wallet--financial-systems)
10. [Social Web — Chat, Mesh, Relay](#10-social-web--chat-mesh-relay)
11. [Cooperative Projects](#11-cooperative-projects)
12. [Chora — Spatio-Temporal Commons](#12-chora--spatio-temporal-commons)
13. [Hypermedia Library & Semantic Library](#13-hypermedia-library--semantic-library)
14. [QApp System](#14-qapp-system)
15. [Companion Gateway & Mobile](#15-companion-gateway--mobile)
16. [Network & Privacy Layer](#16-network--privacy-layer)
17. [Inference & LLM Pipeline](#17-inference--llm-pipeline)
18. [Identity & Cryptography](#18-identity--cryptography)
19. [Specialized Libraries](#19-specialized-libraries)
20. [Domain Libraries](#20-domain-libraries)
21. [Current Desktop Application](#21-current-desktop-application)
22. [Current WASM Frontend](#22-current-wasm-frontend)
23. [What's Broken](#23-whats-broken)
24. [What's Missing](#24-whats-missing)

---

## 1. Platform Architecture

**Paradigm:** Edge-Native 10D Epistemic Manifold Host — a geometrically executable epistemic engine that manages real-time physical simulation of human-centric knowledge, sovereignty, and multi-modal perception.

**Four Pillars:**
1. **High-Dimensional Relational Processor** — 10D packed vectors `[q, v, w, x, y, z, t, α, μ, σ]`, geometric projections, algebraic traversal
2. **Spectral Synthesis Conductor** — all perception data as continuous physical spectrum with invariants `[α, μ, σ]`
3. **Gravito-Thermodynamic Engine** — semantic weights as mass, activation as thermal energy, logic as physical forces
4. **Epistemic Anchor Coordinator** — quantum contexts, wavefunction collapse, decentralized reality interface

**Workspace Crates (19):**
| Crate | Purpose |
|-------|---------|
| `qualia-core-db` | Core semantic database, 10D tensor runtime, GPU render, logic, governance |
| `qualia-cli` | CLI + graph daemon (port 4242) |
| `qualia-solid-bridge` | W3C Solid Protocol interop |
| `qualia-client-core` | Client core: state, wallet, chat, WellFair API, engine, QApps |
| `qualia-extensions` | Extension system |
| `webizen-component-harvester` | Component harvesting |
| `webizen-render` | N-Dimensional renderer SDK (PGA, wgpu, volumetric) |
| `webizen-studio` | Dioxus WASM frontend (150+ qapp components) |
| `webizen-runtime` | Simulation runtime (diffusion, fixed-step clock) |
| `webizen-desktop` | Tauri desktop application (300+ commands) |
| `webizen-web` | Web interface |
| `qualia-mobile-harness` | Mobile PWA harness |
| `wellfare-core` | WellFair domain models (anatomy, clinical, finance, projects) |
| `qualia-cooperative-core` | Cooperative governance (work items, agency, provenance) |
| `qualia-semantic-library` | Document → knowledge base pipeline (HMC containers) |
| `webizen-lite-wasm` | Lightweight WASM build |
| `qualia-q-forge` | WGSL shader forge |

**Constraints:**
- Zero heap in hot paths (stack-allocated f64, u64 indices)
- 48-byte Super-Quin (6 × u64 per semantic datum)
- 42MB Sentinel (single execution pass)
- 512MB Edge Floor (total system)
- Deterministic, non-recursive

---

## 2. 10D Epistemic Engine

**Status: ✅ Core complete, partial features**

The 10D tensor structure `[q, v, w, x, y, z, t, α, μ, σ]`:
- `q` — Quantum context (0 = ground truth, >0 = probabilistic)
- `v` — Algebraic variety (semantic dimension)
- `w` — Cross-domain bifurcation
- `x, y, z` — Spatial coordinates
- `t` — Temporal coordinate
- `α` — Amplitude (semantic mass/gravity)
- `μ` — Modulation (carrier information)
- `σ` — Spectral signature

### 10D Container Format (.10d)
- **Header** — 64-byte POD, `repr(C)`, ABI versioned ✅
- **Axis Role Taxonomy** — q,v,w selectors; x,y,z,t,α,σ coordinates; μ carrier ✅
- **Metric Completeness Descriptor** — verify against reality ✅
- **Section Table** — encode/parse ✅
- **QuantizedMesh Section** — u16-quantized vertices, u16/u32 indices ✅
- **Node Section** — Tensor10D nodes (AOS/SOA layouts) ✅
- **Provenance Section** — source bytes, licence, optional VC ✅
- **CRC-32C Integrity** — whole-file + per-section ✅
- **Topology Section** — gated to wasm-scientific ⚠️
- **Spatial Index Section** — gated to wasm-scientific ⚠️
- **P0.2-P0.8 features** — spec-reserved, not implemented ❌

### Tensor Runtime
- Buffer export and manifold operations ✅
- Spawn/decay temporal ramps (fade-in/out over valid-time windows) ✅
- Temporal scrub and frame-diff API (replay-to-state) ✅
- CBOR compiler ✅
- RDF-star support ✅

### Query Engine
- Mini parser (owns opcodes 0x00-0x04) ✅
- Query engine ✅
- Lexicon/token management ✅
- Hash → URI resolver ✅
- Temporal graph ✅
- Visual model bridge ✅
- Query compiler — gated to wasm-logic ⚠️
- SHACL compiler — gated to wasm-logic ⚠️
- Ontology loader — native-only ⚠️

---

## 3. GPU Rendering Pipeline

**Status: ✅ Core complete**

### PortalGpu (qualia-core-db/src/render/gpu/)
- Cross-platform WebGPU viewport ✅
- HDR bloom chain ✅
- Depth/bloom/mesh/picking pipelines ✅
- Offscreen rendering (PNG export for webview) ✅
- Surface rendering (direct to HWND swapchain) ✅
- Tensor buffer upload ✅
- Mesh upload (including .10d format) ✅
- Camera control (orbit, zoom, pan) ✅
- GPU + CPU picking oracles ✅
- Colour-by-field mapping ✅

### VolumetricRenderer (webizen-render)
- SDK facade over PortalGpu ✅
- Offscreen + surface creation ✅
- Scene contract (SceneNode, SceneEdge, SceneFace, SceneCamera) ✅
- Tensor10DProjection, EpistemicState ✅
- Spectral-to-colour mapping ✅

### Spectral Colour Science (P7.0-P7.8)
- SPD/CMF kernels ✅
- Gamut mapping ✅
- Metamer theory ✅
- GPU colour projection ✅

### Shaders (WGSL, embedded)
- Spectral, ambient, projector, mesh, bloom, epistemic, screen ✅

### PGA (Projective Geometric Algebra)
- Motors, rotors, translators ✅
- Motor encoder for transformations ✅
- Buffer alignment utilities ✅

### Asset Import
- OBJ/STL/GLB import with semantic NQuin generation ✅
- Compile mesh to .10d containers ✅

### Telemetry
- 48-byte SystemTelemetry for GPU uniforms ✅
- AmbientUniforms, ObserverStandpoint, ParticleInstance ✅
- Metrics: memory_pressure, network_ripple, baking_crystallization, logic_flashes, llm_heat, quantum_activity, spectral_shift, temporal_pulse, epistemic_density, manifold_pressure ✅

### GPU Context
- Shared GPU device + queue (process-wide) ✅
- Compute universes (U0 LLM, U1 Tensor10D, U2 Viewport, U3 AcousticPlane) ✅
- VRAM ledger with partitioned slots ✅
- Operational modes (Full, Eco, Reserve) ✅
- Native-only ❌ (no WASM GPU context)

### Browser Portal (WASM)
- QualiaPortal with wasm_bindgen exports ✅
- Canvas creation, panic hooks, WebEngine init ✅
- WASM path renders to canvas (no pixel readback) ⚠️

### Runtime Simulation
- SimulationKernel with ComputeBackend trait ✅
- FixedStepClock for deterministic timing ✅
- Diffusion field (width, height, diffusion_rate) ✅
- Snapshot management (CPU RGBA, GPU texture) ✅
- WGPU compute backend ✅
- Channel-based control plane ✅
- Ledger recording ✅

---

## 4. Computational Geometry

**Status: ⚠️ Partial — 80+ modules, depth varies**

Clean-room Rust implementation of core computational geometry (de Berg et al. reference), available to both native and WASM with 10D tensor integration.

### Implemented Algorithms
- **Convex hull** (2D, 3D) ✅
- **Delaunay triangulation** + Ruppert refinement ✅
- **Advancing front** (2D/3D meshing, deterministic ordering) ✅
- **Bentley-Ottmann** segment intersection sweep ✅
- **Boolean operations** (2D, 3D) ✅
- **DCEL overlay** (double-connected edge list subdivision) ✅
- **Persistent homology** (VR filtration → barcode) ✅
- **CKNN Laplacian** (k-nearest neighbor → Laplace-Beltrami) ✅

### Additional Modules
- anisotropic_remesh, ddg_operators, deterministic_geometry, fem_certificate, geometry_integration, math_geometry, mixed_cell_topology, motion_planning, parametric_cad, screened_poisson, tet_quality_improve

### WASM Exports
- `wasm_convex_hull_2d` ✅
- `wasm_delaunay_triangulation_2d` ✅

### Tauri Command
- `run_computational_geometry()` ✅

### Capability Manifests
- P9.4 qapp/MCP capability manifests ✅
- P9.5 primitives, transforms, scene graph ✅
- P10.5 caller-owned arenas with byte budgets ✅

---

## 5. Computational Economics

**Status: ⚠️ Partial — 38 submodules**

### Implemented
- **Accounting** — Double-entry bookkeeping, trial balance ✅
- **Game theory** — Nash equilibria, duopoly models ✅
- **Derivatives** — Black-Scholes, binomial trees ✅
- **Risk** — VaR, CVaR, scenario analysis ✅
- **Welfare** — Gini coefficient, poverty metrics ✅
- **Forensic economics** — Malfeasance detection, narrative divergence ✅
- **Input-output** — Leontief inverse, key sector analysis ✅
- **Time series** — ARIMA, GBM simulation, drawdown ✅

### Additional Modules
agent_based, asset_pricing, behavioral, capabilities, categorical, dynamic_programming, econometrics, environmental_resource, error, fixed_income, labor_household, macro_models, market_data, market_design, markov, mechanism, network_economics, ontology_bridge, paper_trading, portfolio, public_finance, spatial_economics, yield_curve

---

## 6. Logic & Modalities

**Status: ✅ Core modalities complete, 30+ modalities**

### Fully Implemented
- **Deontic logic** (OP_OBLIGATE, OP_PERMIT, OP_FORBID) ✅
- **Epistemic logic** (OP_KNOWS, OP_BELIEVES, OP_COMMON_KNOWLEDGE) ✅
- **Paraconsistent logic** (contradiction handling) ✅
- **Temporal LTL** (globally, finally, next, until, release) ✅
- **ASP** (Answer Set Programming, stable models) ✅
- **Defeasible reasoning** ✅
- **Argumentation frameworks** ✅
- **Fuzzy logic** (type-1, type-2, quantifiers) ✅
- **Allen Interval Algebra** (7 relations) ✅

### Partial
- Description logic ⚠️
- CTL ⚠️
- Causal reasoning (but-for cause, overdetermination) ⚠️

### Additional
- Jural correlatives (rights, duties, powers) ✅
- Contract formation ✅
- Delegation and revocation ✅
- Consensus mechanisms ✅
- 10D manifold logic ✅
- SHACL → SlgOpcode compiler ✅

---

## 7. Governance & Webizen VM

**Status: ✅ Core complete**

### Webizen Bytecode VM
- 42MB SLG Arena ✅
- Guard-rule grounding ✅
- Forward chaining ✅
- SIMD dispatch ✅
- Gated to wasm-scientific ⚠️

### Governance Modules
- Deontic composition ✅
- Illocutionary acts ✅
- Modal kind definitions ✅
- Provenance tracking ✅
- Webizen validation ✅
- Webizen synchronization (native-only) ✅
- Web civics (native-only) ⚠️
- Coordination (gated) ⚠️

### MCP Server (Model Context Protocol)
- MCP server with sanctuary gate ✅
- Tool descriptors ✅
- Stable MCP tools ✅
- Tool implementations (query_graph, query_sparql, etc.) ⚠️
- Cooperation gate (gated to modalities) ⚠️

---

## 8. WellFair — Human Rights & Health

**Status: ✅ Extensive — 48+ modules, 100+ Tauri commands**

WellFair is the sensitive personal information environment — health records, consent, guardianship, sanctuary vault, dead-man's switches, incapacity switches, companion sync, and more.

### Anatomy Subsystem (14 modules)
- **Factor model** (pathology/condition/medication/food/herb/nutrient/supplement/lifestyle/environmental) ✅
- **17 body systems** (circulatory, respiratory, digestive, endocrine, nervous, immune, integumentary, musculoskeletal, reproductive, urinary, lymphatic, ECS, ENS, glymphatic, sensory, excretory, hematopoietic) ✅
- **Burden accumulation** + interaction detection (compounding/opposing/herb-drug) ✅
- **Temporal kinetics** (onset/clearance, recovery bands) ✅
- **Physiological state** (reproductive, trimester, cycle phase) ✅
- **Anatomy view** (person/clinician lens, burden→sigma mapping) ✅
- **Scorecard** (aspects, weights, contributions, score bands) ✅
- **Knowledge base** (factor knowledge, JSON import, provenance) ✅
- **Investigative pathway** (hypothesis, ranked steps, value-of-information) ✅
- **Birth record** (guardianship credential, steward, biometric class, agency stage) ✅
- **Maternal-fetal dyad** (emerging child, parentage, rights stage) ✅

### Health Records
- Record envelopes (owner/author/proxy DIDs, epistemic status, evidence type, sensitivity class, valid time, predecessor, blob hash, tombstone) ✅
- Health record journal ✅
- Samsung Health CSV import (weight, sleep, heart rate, steps) ✅
- Companion device sync ✅

### Consent & Agency
- Consent store (persisted grants, revocation, expiry, JSONL) ✅
- Consent credentials (grant, revoke, list, present) ✅
- Agency delegation (principal/agent/domain/authority/trigger, ABAC evaluation) ✅
- Authority profiles (modality, trigger, accountability) ✅
- Control stages (GuardianSole, CoSigned, PrincipalSole) ✅

### Sanctuary Vault
- PIN-based lock/unlock ✅
- Decoy session support ✅
- Protected journal kinds ✅
- Keychain wrapping (native) ✅
- Recovery via Shamir secret sharing ✅

### Safeguards
- Dead-man's switch (arm, attest, enact, release) ✅
- Incapacity switch (arm, activate, regain capacity) ✅
- DEK recovery (split, reconstruct, peer envelope) ✅
- Decoy retention (curate, review, mode) ✅

### Transparency & Disclosure
- Transparency CCs ✅
- Disclosure chain ✅
- Actors-with-access ✅
- Leak tracing ✅
- Duty of inquiry assessment ✅

### Clinical
- Clinical reports + attachments ✅
- Conditions + allergies ✅
- Disputed diagnoses ✅
- Medication administration + catalog ✅
- Diet entries ✅
- Medication reminders ✅

### Life & Welfare
- Life events ✅
- Welfare cases + case tasks ✅
- Government letters + attachments ✅
- Assistance needs + welfare streams ✅
- Housing safety ✅

### Wellbeing
- Mental wellbeing observations ✅
- Therapy notes ✅
- PHQ-9/GAD-7 assessments ✅
- Sleep analytics (debt report, heatmap) ✅

### Finance (within WellFair)
- Ledger entries (income/expense/transfer) ✅
- Derived balance ✅
- Projects + contributions + obligations ✅

### Body Assets (3D Anatomy)
- CCF/HRA GLB → compiled .10d cache ✅
- Body assets status ✅
- Cached organ percepts ✅
- Body render start/stop ✅

### Sync & Backup
- Versioned, replay-safe sync protocol ✅
- Quarantined inbox ✅
- Content hashing, Lamport clocks ✅
- HTTP relay server (native) ✅
- Export/import backup ✅

### Tauri Commands (100+)
All WellFair functionality is exposed via Tauri commands in `webizen-desktop/src/commands/mod.rs` and `qualia-client-core/src/wellfair/api.rs`.

---

## 9. Wallet & Financial Systems

**Status: ✅ Implemented**

### Wallet (qualia-client-core/src/wallet/)
- **HD wallet** — BIP32 address derivation for BTC, XEC, ETH, NYM ✅
- **Bitcoin Cash transactions** — TxIn, TxOut, varint encoding ✅
- **P2PKH signing** — SIGHASH_ALL|FORKID, double SHA256, DER signatures ✅
- **Chronik API client** — UTXO fetching, transaction broadcast, history ✅
- **Coin selection** — UTXO selection ✅
- **Ledger** — Wallet ledger entries ✅
- **Semantic tokens** — Semantic token operations ✅

### ILP Payment Routing
- Interledger Protocol payment routing ✅

---

## 10. Social Web — Chat, Mesh, Relay

**Status: ✅ Core implemented**

### Chat Sessions
- Chat session persistence (WAL quins + JSON sidecar) ✅
- Lamport clocks ✅
- Group/solo sessions ✅
- ChatEnvironment with ontology scope ✅

### Group Chat Relay
- Envelope signing ✅
- HTTP publish/pull ✅
- Cursor tracking ✅
- Signature verification ✅

### Chat over Mesh
- SocialWebNet mesh ✅
- Reliable-channel frames ✅
- CBOR encoding ✅
- At-least-once delivery ✅
- Mesh service (native-only) ✅

### Additional Chat
- Chat graph operations ✅
- Chat inference ✅
- Chat agents ✅
- Chat files ✅
- Chat ontology ✅
- Chat retrieval ✅
- Solid chat integration ✅

### Social Mesh
- Social mesh ✅
- Social peers ✅
- Social connect ✅
- Mesh channel (reliable datagram delivery) ✅

---

## 11. Cooperative Projects

**Status: ✅ Core implemented**

### Work Items (qualia-cooperative-core)
- WorkItem (immutable core) ✅
- WorkItemStatusEvent (immutable status transitions) ✅
- WorkItemType, WorkItemStatus, WorkItemPriority ✅
- Replay-safe Kanban board (merge_status_events, current_status, derive_board) ✅
- Envelope builders ✅

### Agency Delegation
- AgencyDelegation with full ABAC evaluation ✅
- Precedence (Primary/Secondary/LocalTemporary) ✅
- ConsentState (Pending/Granted/Withdrawn/NotRequired) ✅
- ControlStage (GuardianSole/CoSigned/PrincipalSole) ✅
- TransferStage ✅
- delegation_permits (fail-closed ABAC evaluator) ✅

### Provenance
- JudgementProvenance DAG with Reliance ✅
- Recursion limit ✅

### Taxonomy
- Sphere, TermId ✅

### QApp Package
- Package manifest ✅
- PWA packaging ✅

---

## 12. Chora — Spatio-Temporal Commons

**Status: ✅ Commands implemented, UI partial**

### Tauri Commands (11)
- `chora_list_worlds` ✅
- `chora_get_world` ✅
- `chora_save_world` ✅
- `chora_delete_world` ✅
- `chora_seed_demo` ✅
- `chora_navigation` ✅
- `chora_set_temporal` ✅
- `chora_set_active_world` ✅
- `chora_query_region` ✅
- `chora_publish_asset` ✅
- `chora_pull_assets` ✅

### WASM Panel
- `WellfairChoraPanel` ✅ (but has the use_effect loop bug)

---

## 13. Hypermedia Library & Semantic Library

**Status: ⚠️ Partial**

### WellFair Hypermedia Store
- Hypermedia store ✅
- Blob store ✅
- Document ingestion ✅
- Library search (by content + time) ✅
- Library listing ✅

### Semantic Library (qualia-semantic-library)
- HMC container format (ZIP-based, self-describing manifests) ✅
- Four-stage pipeline: ingest → LLM → library → reorganize ✅
- File acquisition + hashing ✅
- Content extraction ✅
- Text chunking ✅
- CML (Context Markup Language) processing ✅
- Ollama HTTP backend for embeddings ✅
- Library index with dedup/ranking ✅
- Disk layout reorganization ✅
- CLI tool (`qsl`) ✅
- **Not integrated with WellFair hypermedia store** ❌
- **No web UI** ❌

---

## 14. QApp System

**Status: ✅ Infrastructure complete, 1 bundled QApp**

### QApp Infrastructure
- QApp manifest format (yaml-ld-q42) ✅
- QApp installation (atomic, version compatibility) ✅
- QApp registry ✅
- QApp versioning ✅
- QApp paths ✅
- QApp MCP (Model Context Protocol) ✅
- QApp API ✅
- QApp publishing (PWA) ✅
- QApps protocol server (port 4567) ✅
- Bundled QApps loader ✅

### Tauri Commands
- `list_installed_qapps` ✅
- `generate_qapp_credential` ✅
- `verify_and_install_qapp` ✅
- `launch_installed_qapp` ✅
- `qapp_analyze` ✅
- `wellfair_publish_qapp_pwa` ✅

### Bundled QApps
- **Anatomy** (v0.0.12) — health visualization, chat handoff, DICOM overlay, knowledge catalog ✅
  - Launch modes: static-web, wasm-local, flutter-app-vault, online-daemon-aware
  - Uses Babylon.js 3D library
  - SHACL shapes for health data

### QApp Specification
- `docs/manuals/qapps_specification.md` ✅
- `docs/manuals/qapp_llmHelper.md` ✅
- `docs/manuals/qapp-vault-developer-guide.md` ✅

### Studio QApp Components (150+)
150+ academic discipline QApp components exist in `webizen-studio/src/components/` — from Aesthetics to Zoology, plus ~40 platform/developer QApps.

---

## 15. Companion Gateway & Mobile

**Status: ✅ Core implemented**

### Companion Gateway
- WebSocket gateway for mobile companion apps ✅
- Ed25519 signature verification for pairing ✅
- Challenge-response authentication ✅
- QR code generation for pairing ✅
- Live share request handling ✅
- Health bundle ingestion ✅

### Routes
- `GET /mobile/stream` — WebSocket endpoint ✅
- `GET /mobile/qr` — QR code SVG ✅
- `POST /wellfair/companion/ingest` — Health bundle POST ✅
- `GET /api/wellfair/companion/pairing` — Pairing info ✅

### Mobile Harness
- `qualia-mobile-harness` crate ✅
- PWA support ✅

---

## 16. Network & Privacy Layer

**Status: ⚠️ Mixed**

### Implemented
- Network disclosure registry (egress consent) ✅
- Sonic token handling ✅
- Nym Mixnet: Sphinx Packet routing ✅
- Userspace WireGuard Proxy (127.0.0.1:1080) ✅
- P2P Node (libp2p, PeerId, multi-addresses) ✅
- Gun.eco: WebSocket Graph bridge ✅
- DNS resolver ✅
- Cloudflare integration ✅
- QDP HTTP, QDP resolver, QDP server ✅
- Semantic handshake ✅
- QLink ✅
- Connection identifier ✅
- Handshake protocol ✅
- Magic link ✅
- Mail transport (SMTP/IMAP via lettre) ✅
- Mail rules ✅

### Stubs
- Acoustic BLE mesh ❌
- eBPF filter ❌
- eBPF firewall ❌
- Host topology ❌
- Nym adapter ❌
- WebTorrent (routes exist but implementation unclear) ⚠️

### Solid Protocol
- `qualia-solid-bridge` crate ✅
- Solid LDP ✅
- Sync to Solid pod ✅

---

## 17. Inference & LLM Pipeline

**Status: ⚠️ Partial — infrastructure built, some mocked**

### Inference Runtime
- GGUF bridge ✅
- Inference agent ✅
- Orchestrator ✅
- Thermal telemetry ✅
- Tensor roles ✅

### LLM Offload
- SPSC ring buffer for Webizen sentinel interception ⚠️ (mocked tokens, not real inference)

### Model Lifecycle
- Active model record management ✅
- Model preferences ✅
- Inference backend coordination ✅
- Model discovery ✅

### DirectML
- DirectML 1.15 linked from vendor ✅
- DXC (DirectX Shader Compiler) copied beside binaries ✅
- Flash-attention shader (fused_attention.wgsl) ✅
- Probe best adapter memory ✅

### QPU
- QPU dispatcher ✅
- QPU oracle ✅
- QPU pipeline ✅
- QPU problem formulation and job queue (native-only) ✅
- QAOA, SPSA solvers ✅

### Engine
- PDF processing (simulated) ⚠️
- CSV/JSON parsing ✅
- RDF serialization ✅
- Q42 compilation ✅
- Semantic processing ✅

---

## 18. Identity & Cryptography

**Status: ✅ Core implemented**

### Identity
- Agency management ✅
- Identifier ✅
- Key vault (native-only) ✅
- Profiles ✅
- Webizen identifiers ✅
- Node identity ✅
- User profile ✅

### Cryptography
- Fiduciary crypto ✅
- Sanctuary audit ✅
- ZK proofs ✅
- PQ KEM (post-quantum key encapsulation) ✅
- Verifiable credentials ✅
- ML-DSA signatures ✅
- Envelope encryption (native-only) ✅
- Shamir secret sharing recovery ✅

---

## 19. Specialized Libraries

### Fully/Partially Implemented
| Library | Status | WASM |
|---------|--------|------|
| Computational Geometry | ⚠️ Partial (80+ modules) | Both |
| Computational Economics | ⚠️ Partial (38 submodules) | Both |
| Linear Algebra | ⚠️ Partial (BFV HE, DP) | Native |
| Cryptographic Library | ⚠️ Partial | Native |
| Symbolic Algebra | ⚠️ Partial (CAS) | Both |
| Physics Simulation | ❌ Stub | Native |
| Category Theory | ❌ Stub | Native |
| Chemistry Modeling | ❌ Stub | Native |
| Constructibility | ❌ Stub | Native |
| Engineering Analysis | ❌ Stub | Native |
| Financial Modeling | ❌ Stub | Native |
| Machine Learning | ❌ Stub (fail-closed) | Native |
| Medical Computing | ❌ Stub | Native |
| Multivar Calculus | ❌ Stub | Native |
| Polynomial Algebra | ❌ Stub | Native |
| QPU Bridge | ⚠️ Partial | Native |
| Quantum Biology | ❌ Stub | Native |
| Statistical Computing | ❌ Stub | Native |

---

## 20. Domain Libraries

### Implemented
| Domain | What | Status | WASM |
|--------|------|--------|------|
| Geospatial | 12 adapters (DEM, OSM, STAC, CKAN, GBIF, etc.), terrain, AR anchors | ⚠️ Partial | Both |
| Financial | Economics (input-output, macro flows, node pricing, resilience, stochastic), tax schema | ⚠️ Partial | Both |
| Biological | Bioinformatics (Smith-Waterman, k-mer, DNA translation) | ⚠️ Partial | Both |
| Chemical | Organic chemistry (SMILES, InChI, molecular properties, Lipinski) | ⚠️ Partial | Both |
| Mathematical | Geometric algebra | ⚠️ Partial | Both |
| Physical | Thermodynamics | ⚠️ Partial | Both |

---

## 21. Current Desktop Application

### Architecture
- **Tauri 2.11** desktop app with webview loading WASM frontend
- **Native GPU surface** (child HWND + wgpu::Surface) for direct 10D rendering
- **Settings server** on port 8080 (REST API, SSE telemetry, companion WS, Studio WASM serving)
- **Graph daemon** on port 4242 (SPARQL, health, chat relay, WebTorrent, telemetry ingest)
- **QApp protocol** on port 4567 (qualia:// content serving)

### System Tray
- Open Webizen Studio
- Settings
- Toggle Ambient Visualizations
- Revoke Sessions
- Quit
- Sanctuary submenu (Lock, Unlock, Vault Status)
- Daemon submenu (Status, Restart, Stop)
- Health submenu (Medication Reminders, Quick Backup, Diagnostics)
- Sync submenu (Sync with Relay, View Sync Inbox)
- Help submenu (About, Check for Updates, View Logs, Open Settings Portal)

### Protocol Handlers
- `qualia://` — serves QApp assets from QAPPS directory
- `webizen://` — routes for diffusion frames, render preview, anatomy body PNG/JSON, organ 10D files

### Tauri Commands: 300+
Grouped by domain:
- QApp Vault (4)
- Hardware/System (2)
- Daemon (5)
- Config (2)
- WellFair (100+)
- Chora (11)
- DNS/Handshake/QLink (7)
- Computational Geometry (1)
- QApp Analysis/Compute (4)
- GPU/Mesh (3)
- Render Preview (4)
- Anatomy Body Render (2)
- Binary IPC/GLB Testing (6)
- Telemetry Bridge (2)
- Native Surface/GPU (4)
- 10D Container Browser (2)
- Mesh Commands (3)

### Settings Server Routes (port 8080)
- Health, status, config
- Manifest (workspace, history, undo)
- Telemetry (SSE stream)
- Jobs (list, enqueue, cancel)
- SPARQL (endpoints, query)
- Assets (catalog, recommend, enqueue)
- Studio (generate_pane)
- Companion (ingest, stream, QR, pairing)
- Command proxy (`POST /api/invoke/{cmd}`)
- Static files (Studio WASM, portal)

---

## 22. Current WASM Frontend

### Routes (16)
- `/` — Dashboard
- `/anatomy-test` — Anatomy testing
- `/qapps` — QApps management
- `/browser` — Web browser
- `/settings` — Settings
- `/about` — About
- `/context-studio` — Context workspace
- `/qapp-studio` — QApp studio
- `/qapp-studio/:app_id` — Edit specific qapp
- `/render-preview` — Render preview
- `/scene-interaction` — Scene interaction
- `/nexus` — Nexus research
- `/wellfair` — WellFair shell
- `/chora` — Chora panel
- `/10d-browser` — 10D browser
- `/gpu-viewport` — GPU viewport
- `/:..path` — Dynamic page (qapp routing)

### Components
- 150+ academic discipline QApp components
- 43 WellFair panels
- Core: dashboard, browser_panes, native_gpu_viewport, ten_d_browser, settings, about, etc.

### Render Subsystem
- Renderer trait (viewport, camera, clear, project, line, point, fill_polygon)
- Canvas2D (WASM-only, CPU rendering)
- Native wgpu renderer (desktop-only)
- Scene graph, mesh transforms, spring physics, animation loop
- QualiaDB scene builder
- Spatial bridge, tensor buffer management

### Studio Canvas
- DynamicPage with pane system
- 40+ default pane layouts for known qapps
- Grid-based positioning (96×64 points)
- Drag, resize, snap
- Workspace history (undo/redo)
- Theme binding per pane

### QApp Engine
- Dual-mode Tauri invoke (WASM: `window.__TAURI__.core.invoke()`, Native: REST proxy)
- Ontology-driven form builder

---

## 23. What's Broken

### Critical
1. **WellFair lockup** — `use_effect` + `spawn` pattern creates infinite re-render loop. Effect runs → spawns task → task sets signal → signal triggers re-render → effect runs again. 36 panels have this pattern. When navigating to WellFair, 5+ panels fire simultaneously, flooding the WASM event loop.
2. **GPU surface adapter mismatch** — was creating separate wgpu instance for surface (fixed in last commit, but may still have issues with presentation support)
3. **LLM offload mocked** — uses mocked tokens instead of real inference
4. **PDF ingestion simulated** — returns simulated bookmarks, not real parsing
5. **Several solvers disabled** — calculus, linear_algebra, optimization, quantum_optimizers, symbolic_logic have build errors (ExecutionError/SolverState refs)

### Architecture
6. **Monolithic WASM blob** — entire frontend is one WASM file; no code splitting, no lazy loading, all 150+ components loaded upfront
7. **No proper error boundaries** — a single component failure can crash the entire app
8. **No loading states** — inconsistent indicators across panels
9. **No offline mode** — no proper offline state handling
10. **No persistence** — theme changes and pane layouts not saved

### Integration Gaps
11. **Chat ↔ WellFair** — not integrated
12. **Render ↔ Runtime** — no real-time scene updates from simulation
13. **WellFair ↔ QualiaDB** — optional bindings not fully utilized
14. **Cooperative ↔ Finance/Projects** — obligation derivation not wired to UI
15. **Semantic Library ↔ WellFair** — hypermedia store not integrated with semantic library

### Missing Scripts
16. `scripts/bundle-desktop-deps.ps1` — referenced in tauri.conf.json but doesn't exist
17. Updater endpoint still references decommissioned `mediaprophet.github.io/webizen-browser`

---

## 24. What's Missing

### For a Proper Desktop Application
1. **Native application shell** — proper window management, menu bar, toolbar, status bar
2. **Tabbed interface** — multiple qapps open simultaneously in tabs
3. **Address bar / navigation** — URL-based qapp loading
4. **QApp isolation** — each qapp in its own context, not all in one WASM blob
5. **QApp marketplace/browser** — discover and install qapps
6. **Proper system tray integration** — background runner with full menu
7. **Cross-platform** — currently Windows-only GPU surface, needs macOS
8. **Auto-update** — working updater (current endpoint is dead)
9. **Settings UI** — proper native settings, not just a web page
10. **Keyboard shortcuts** — navigation, tab switching, etc.

### For the Platform
11. **Real LLM inference** — not mocked
12. **Real PDF parsing** — not simulated
13. **WebTorrent implementation** — routes exist but no implementation
14. **Video chat** — social web works has chat but no video
15. **More bundled QApps** — only Anatomy is bundled
16. **QApp developer tools** — visual editor for creating qapps
17. **10D container P0.2-P0.8** — spec-reserved features not implemented
18. **P2P sync** — infrastructure exists but not wired for real use
19. **Accessibility** — missing ARIA labels, keyboard navigation, screen reader support
20. **Internationalization** — no i18n

---

*This audit was compiled from four parallel subagent explorations of the entire codebase, plus direct investigation of the old webizen-browser repository (v0.0.3, v0.0.4) and the ARCHITECTURE_OVERVIEW.md from that era.*
