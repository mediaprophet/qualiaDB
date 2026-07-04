# Architectural-enhancement decision-path index

**Status:** classification index (2026-07-04).
**Source:** 27 architectural-enhancement specs drafted earlier, staged in
`C:\Projects\Local_LIbraries\qualia-db-old-files\local\architectural-enhancements` (originals untouched).
**Why this exists:** these are old drafts whose specific implementation details have drifted, but whose
**analysis / decision-path** — the architectural choice each argues for, and whether the code took it — has
lasting value. Each was read and checked against the current codebase.

**Where the full copies live:**
- **pending** (still-valid unbuilt directions) → `docs/plans/architectural-enhancements/` *(tracked, here)*.
- **realized** and **superseded** (historical decision-path / rejected-alternative rationale) →
  `legacy-docs/architectural-enhancements/` *(gitignored reference archive)*.

**Legend:** ✅ realized · ○ pending · ⊘ superseded (code went another way).

---

## ○ Pending — still-valid unbuilt directions (5) → `docs/plans/architectural-enhancements/`

These decision-paths were neither built nor reversed; their premises still hold. The most actionable of the set.

1. **O(1)-Memory CRDTs** (`O1_Memory_CRDTs_Implementation_Spec.md`) — fix unbounded CRDT tombstone growth via
   Dotted Version Vectors + Epoch-Based Anti-Entropy (seal tombstones into signed epoch archives once a
   peer-confirmation threshold is met, then reclaim). *Code today ships a cruder Lamport-LWW + in-place
   epoch-compaction (`foundation/crdt.rs`, `sync.rs` `MASK_EPOCH_COMPACT`) with no DVV / sealed-epoch ledger — a
   valid unrealized upgrade.*
2. **Intermittent Computing** (`Intermittent_Computing_Implementation_Spec.md`) — an interrupt-driven snapshot
   engine that microsecond-captures CPU registers + the whole 42 MB `SlgArena` to NVM on power-loss/impact for
   exact bytecode-position resumption. *No `intermittent/` module; durability today is WAL + CoW file snapshots,
   not volatile register+arena capture. The fixed 42 MB ceiling it relies on is real — the direction stands.*
3. **NVMe Computational-Storage Pushdown** (`NVMe_Computational_Storage_Pushdown_Implementation_Spec.md`) —
   offload filter/aggregate/join to execute on-device on NVMe CSDs, streaming results back. *`csd_storage.rs`
   exists and is wired (CsdManager, device discovery, op enum), but in-device compute is a simulated skeleton
   (returns zero-filled buffers; matmul falls back to host CPU) — the offload API is live, the actual pushdown is
   future work.*
4. **Formal Verification** (`Formal_Verification_Implementation_Spec.md`) — a machine-checked proof tier
   (Coq/LEAN) over the governance VM proving safety invariants (no classified→public lane, memory/PC bounds), for
   a "mathematically proven fiduciary engine." *No formal-methods layer exists; governance is enforced at runtime
   (SHACL + N3Logic + zk-gated tokens + signed WAL). Complementary, not built — a valid future direction.*
5. **Spatial-Web Anchoring** (`Spatial_Web_Anchoring_Implementation_Spec.md`) — GPS-free "digital dead drops":
   hash a physical space's point-cloud (UWB + visual positioning) and use the space-hash as the NQuin encryption
   key, so data decrypts only when an authorized DID physically re-occupies the location. *No mechanism in-tree;
   existing spatial code is coordinate indexing, orthogonal to physical-presence cryptography. Premise not
   reversed — pending.*

---

## ✅ Realized — decision landed in code (18) → `legacy-docs/architectural-enhancements/`

The architectural choice each argues for is implemented. Kept as the design-rationale for *why the code is the
way it is*.

**CBOR-LD / Q42 semantic wire (the whole cluster is realized):**
- **CBOR_LD_Q42_Revised_Analysis / CBOR_vs_CBOR_LD_Analysis / CBOR_LD_Q42_Implementation_Summary /
  CBOR_LD_Negatives_Analysis** — adopt CBOR-LD but resolve terms against a native **Q42 embedded lexicon**
  (O(1) hash, zero-alloc, no JSON-LD/remote context/network). → `q42/q42_lexicon.rs` (`Q42Lexicon`,
  `Q42CborLdParser`, `resolve_term`), `q42_lex.rs` mmap, wired across ~44 files (p2p protocol `qcborld`,
  `daemon_swarm.rs`, `identity/vault_manifest.rs`). The argued-against remote-JSON-LD path was *not* built.
- **DNSSEC_SocialWireGuard_Implementation** — bootstrap peers by resolving a DNSSEC-signed CBOR-LD payload
  straight into a 48-byte Quin, then a routing-constraint-gated WireGuard tunnel. → `services/daemon_swarm.rs`
  (real `dig +dnssec`, `parse_cbor_ld_to_payload`, `establish_wireguard_tunnel`). Evolved: renamed
  SocialWebNet; the spec's "Mock: assume authorized" became a **real fail-closed** routing-mask check against the
  rights ontology.
- **Q42_ENHANCEMENT_PLANNING** — Q42 overlays: bi-temporal PROV-O, GeoSPARQL/KML, Merkle-DAG with contestability
  forks, credential-gated AES layers, ODRL/SKOS rights, Dynamic Epistemic Logic (JTB), SHACL-validated — as
  named-graph overlays, *rejecting* NQuin struct changes / CP-ABE / FHE / OWL. → `epistemic.rs`,
  `platform/git_bridge.rs` (DagNode/fork/merge), `query/temporal_graph.rs`, SPARQL `AS OF`, the `ontologies/*.ttl`.
  Only its own Phase-5 deferrals (CP-ABE, FHE) remain unbuilt, as planned.

**Phase-2 foundation layer (realized as a set):**
- **Architectural_Enhancement_Priority_Assessment / Phase_2_Implementation_Plan /
  Phase_2_Implementation_Completion_Summary / Architectural_Enhancement_Roadmap** — build the foundational
  substrate (ZNS storage, ML-DSA crypto, eBPF firewall, CSD, ZK proofs, ambient orchestration, acoustic/BLE mesh)
  under/alongside the specialized libraries. → all named modules exist (see below). *Caveat: the summaries'
  "10×/50×" and "military-grade / 95% coverage" figures are unverified marketing, and some named pieces are still
  scaffold — the architectural decision is realized; the performance claims are not evidence.*
- **Fiduciary_Cryptography** — post-quantum **ML-DSA** signatures for fiduciary data, not classical crypto. →
  `crypto/fiduciary_crypto.rs` real FIPS-204 `ml_dsa_65` (spec wanted ML-DSA-87 — a parameter choice).
- **Zero_Knowledge_Semantic_Proofs** — zk-SNARK semantic verification over Quins. → `crypto/zk_proofs.rs` real
  **Groth16 over BLS12-381** (stronger than the spec's proposed Bellman/BN254 hash-commitment), plus
  `zk_predicates.rs` (threshold/range) and `deontic_circuit.rs`. *Trusted-setup ceremony pinning / Merkle-set
  membership still pending hardening.*
- **Allocation_Firewall** — eBPF/XDP allocation firewall with Allow/Deny/RateLimit/Redirect + zero-copy socket
  bypass. → `net/ebpf_firewall.rs`, and `net/ebpf_filter.rs` extended it **cross-platform** (Linux bpf, Windows
  WFP, macOS XPC, Android VpnService) past the spec's Linux-only premise.
- **Hardware_Sympathetic_Storage** — ZNS zone manager (write-pointer/sequential-append) behind a runtime storage
  driver. → `zns_storage.rs` + `storage_driver.rs` (`open_storage()`). *Low-level ZNS I/O scheduler still returns
  simulated completions — decision realized, on-hardware path scaffolded.*
- **Zero_Copy_LoRA_Multiplexing** — one memmap base model + streamed context-triggered LoRA adapters, additive
  low-rank delta, LRU cache, GPU apply. → `lora/{mod,adapter_manager,context_detector,webgpu_lora}.rs` +
  `shaders/lora_apply.wgsl`, wired into `inference_agent.rs`. *Only the NQuin context-trigger metadata-bit
  accessor is unlanded (context detected from prompt text instead).*
- **Ambient_Sub_Threshold_Orchestration** — power/thermal/battery-budgeted edge-AI scheduling over
  NNAPI/CoreML/ONNX. → `inference/ambient_orchestration.rs` (2417 lines, 15 tests). *NNAPI/CoreML C-FFI execution
  still simulated — decision realized, raw platform bindings stubbed.*
- **Zero_Infrastructure_Acoustic_BLE_Mesh_Syncing** — dual-mode ultrasonic + BLE mesh, DTN store-and-forward,
  hybrid selection, zero-heap. → `net/acoustic_ble_mesh.rs` (2684 lines). *PHY simulated (thread::sleep, synthetic
  nodes; no real cpal/btleplug transceiver) — structural module realized, hardware layer stub.*
- **Cryptographic_Halo** — homomorphic encryption on encrypted Quins (BFV). → `specialized_libs/linear_algebra/
  privacy/bfv.rs` real `BfvEngine` (default `privacy-he` feature, 42 MiB bound). *The spec's specific "FHE over
  WebGPU" GPU mechanism was **not** taken — the FHE is CPU-only.*

---

## ⊘ Superseded — the code deliberately went another way (4) → `legacy-docs/architectural-enhancements/`

Valuable as *rejected-alternative* rationale (the roads not taken, and why).

1. **Unforgeable_Agency** (`Unforgeable_Agency_Implementation_Spec.md`) — bind agency to biometrics
   (fingerprint/face/voice/iris) inside a hardware TEE. **Rejected** in favour of a survivable **k-of-n relational
   identity fabric** where biometrics are one revocable anchor and unforgeability comes from ML-DSA + Groth16 —
   `modalities/identity_fabric.rs` ("an identifier is not an identity", Shamir reconstruct, survives anchor loss).
   *Directly consistent with this session's DID-is-identifier / biometric-sovereignty work — the rejection is the
   correct one.*
2. **WASI_Component_Model** (`WASI_Component_Model_Implementation_Spec.md`) — sandbox Qapps via WASI Preview-3
   Component Model (WIT + wasmtime + NQuin capability handles). **Superseded** by a simpler plain-WASM-bundle + PWA
   path with a least-privilege `Capability` enum (`qapp_package/manifest.rs`). Capability-security goal kept, WASI
   mechanism dropped.
3. **Spatiotemporal_Fractal_Indexing** (`Spatiotemporal_Fractal_Indexing_Implementation_Spec.md`) — Morton codes
   as a **replacement** for R-/KD-trees (1-D binary search on interleaved lat/lon/time). **Superseded**: Morton
   encoding was adopted (`computational_geometry/spatial_order.rs`) only as a *build/sort helper* for the trees it
   meant to eliminate; spatiotemporal reasoning went to Allen's interval algebra + RCC8 instead.
4. **Pre_Library_Implementation_Priority** (`Pre_Library_Implementation_Priority.md`) — gate *all* library work
   behind completing 13 enhancements, **WASI-sandboxing first**. **Reversed**: the specialized libs were built
   directly (9 active, 79 tests), the WASI-first gate was never taken, and the other foundations landed
   independently — the sequencing/gating discipline is obsolete.

---

## Cross-cutting observation (worth noting honestly)

A recurring pattern across the **realized** set: the *architectural decision* is implemented and the structural
Rust exists, but the **low-level hardware / PHY / FFI layer is often still simulated** — ZNS I/O scheduler,
NNAPI/CoreML FFI, acoustic/BLE transceiver, NVMe in-device compute all return synthetic completions. So "realized"
here means *the decision is made and the code path exists*, not *the hardware is fully exercised*. Several of
these are the natural next hardening steps if/when the corresponding hardware is in play.
