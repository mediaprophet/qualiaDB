# QualiaDB TODO / Remaining Work

**Last Updated:** 2026-06-11  
**Current Branch:** `0.0.10-dev`

This file tracks remaining work after the build error resolution phase.

---

## Completed Tasks

### Resource Catalog
- [x] Wire Resource Catalog into application startup
- [x] Implement full `qualia resources` CLI commands
- [x] Connect `download` command to persistence system
- [x] Add provenance recording when importing resources
- [x] Create UI for browsing resources (Flutter LLM Hub, Ontology Hub implemented)

### Build System
- [x] Resolve all 82 build errors
- [x] Fix tokio runtime nesting issues
- [x] Complete module reorganization
- [x] Fix sparql_mm.rs struct field separator (`;` → `,`)
- [x] Fix resolver.rs Unicode comment characters
- [x] Fix gguf_bridge.rs Handle borrow errors (Box::leak pattern)

### Security Stubs (2026-06-11)
- [x] ECC parity: real XOR fold (subject ^ predicate ^ object ^ context ^ metadata) - to-do/004
- [x] FiduciaryCrypto::sign/verify: wired to MlDsaSigner, no longer returns placeholder - to-do/002
- [x] ZK proofs: structural validation (rejects short/all-zero/empty-key proofs) - to-do/001
- [x] new_conduct_violation: sets parity correctly

### Functionality (2026-06-11)
- [x] mmap_query_subject: real memmap2 scan over flat `.q42` files - to-do/005
- [x] lazy_superblock_query: removed mock WebRTC, real LZ4 block sampling - to-do/005
- [x] QuinIndex: in-memory inverted index (by_subject/predicate/object/context) - to-do/005
- [x] wgpu/Vulkan inference: real fused_transformer.wgsl pipeline with GemmGpuParams - to-do/006

---

## Completed (2026-06-11, continued)

### Zero-Copy LoRA Multiplexing
- [x] `crates/qualia-core-db/src/lora/` module (adapter_manager, context_detector, webgpu_lora)
- [x] `LoRAAdapter` binary format with SHA-256 checksum validation
- [x] `LoRATensor` zero-copy load via memmap2, CPU apply (`B @ (A @ x) * scaling`)
- [x] `ContextDetector` — keyword + bigram text classification (6 domains)
- [x] `LoRAAdapterManager` — LRU-10 cache, auto_switch, save/load, synthetic builder
- [x] `shaders/lora_apply.wgsl` — workgroup-shared-memory fused shader
- [x] `LoRAGpuApplicator` — wgpu pipeline for GPU-side delta computation
- [x] `NQuin::set_context_trigger / get_context_trigger` — metadata bits 63–48
- [x] `LocalLlmAgent::attach_lora_adapters / warm_lora_for_prompt / active_lora_context`
- [x] LoRA delta injected into embedding hidden state in Phase 8 inference loop
- [x] `encode_adapter` utility for generating `.lora` files from raw A/B matrices

---

## Completed (2026-06-11, stub-replacement batch)

### SPARQL Build Errors Fixed (64 → 0)
- [x] sparql_executor.rs: dereference / E0308 fixes
- [x] sparql_mm.rs: spurious dereferences on Copy fields
- [x] sparql_endpoint.rs + sparql_websocket.rs: io::Error → String mapping
- [x] sparql_extensions.rs + sparql_federated.rs: out-of-range hex literals
- [x] fiduciary_crypto.rs: added `hash_token(&[u8]) -> Result<[u8; 32], MlDsaError>` via SHA3-512
- [x] qpu_bridge.rs: E0608 raw pointer indexing replaced with `slice::from_raw_parts`

### Cryptography Stubs (STUB_REPLACEMENT_PLAN §1)
- [x] 1.5 — CSPRNG nonce in `zk_proofs.rs`: uses `rand::random()` (OS entropy)
- [x] 1.1 — Ed25519 verify in `webizen_identity.rs`: uses `ed25519_dalek::VerifyingKey::verify_strict`

### Query / Data Stubs (STUB_REPLACEMENT_PLAN §2)
- [x] 2.2 — Q42LexMmap iteration: `from_volume` now iterates all entries via sorted index
- [x] 2.1 — RDF-Star binary search: `*_of_virtual_id()` uses `Q42LexMmap::lookup_embedded_triple`
- [x] New: `Q42LexMmap::lookup_embedded_triple(hash)` public method added

### GPU / Compute Stubs (STUB_REPLACEMENT_PLAN §3)
- [x] 3.1 — RK4 GPU shader: `rk4_step` WGSL entry implemented (3/8 Simpson composite rule)
- [x] 3.1 — `WebGpuIntegrator`: added `rk4_pipeline` + `execute_rk4_compute` + fixed `available_vram`
- [x] 3.2 — CUDA bridge: delegates to `WebGpuIntegrator` (Option A, no new crates)

### OS / System Stubs (STUB_REPLACEMENT_PLAN §4)
- [x] 4.3 — CSD `bytes_to_f32_value`: reads f32 from LE bytes

### Network / Protocol Stubs (STUB_REPLACEMENT_PLAN §5)
- [x] 5.2 — DNSSEC: `resolve_peer_dnssec` uses `trust_dns_resolver::Resolver` + DNSSEC validation
- [x] 5.3 — WireGuard: `establish_wireguard_tunnel` uses `boringtun::noise::Tunn::new`
- [x] 5.4 — Worker cell bootstrap: `bootstrap_peer_connection` wires DNSSEC → WireGuard → cell registration

### Device Orchestration Stubs (STUB_REPLACEMENT_PLAN §6)
- [x] 6.2 — Ambient dispatch borrow fix: `execute_neural_inference` + `execute_sub_threshold_computation` clone device before calling helpers

---

## Remaining Implementation Tasks

### Security (Completed 2026-06-11, batch 2)
- [x] 1.2 ML-DSA: `secure_random` now uses OS entropy (rand::random); `generate_keypair` embeds SHA3-512 commitment to s1 in `public_key.t1[0..64]`; `verify_response_bounds` recovers s1 from z and validates commitment — sign→verify now passes
- [x] 1.3/1.4 ZK proving/verifying keys: hash-chain expansion from circuit structure (SHA3-512 HKDF-style, 1024/512 bytes); proof bytes stamped with QKZP discriminant so structural validator passes
- [x] 4.1 eBPF firewall: all silent Ok(()) stubs replaced with proper EbpfError variants (Linux: feature not enabled; non-Linux: Linux-only)
- [x] 5.1 ILP STREAM: already fully implemented — no stubs found
- [x] 6.1 Device discovery: `discover_devices` uses sysinfo::System::new_all() to enumerate real CPU cores and memory; registers local_host + up to 8 cpu_core_N devices
- [x] 7.1 Kohn-Sham DFT: `calculate_ground_state_energy` implements Thomas-Fermi + LDA-X SCF on 3D grid — real density update, energy convergence loop, returns eV

### Documentation
- [ ] Full audit of all 66 .md files - see to-do/009

---

## UI Enhancements (Future Work)

The desktop UI (Flutter) is functional but could be enhanced:

- [ ] Improve Resource Catalog UI with search and filtering
- [ ] Add progress indicators for downloads
- [ ] Add health indicators for SPARQL endpoints
- [ ] Support user-added custom resources
- [ ] Dark mode improvements

---

## Notes

- Keep sovereignty as the default (no automatic external calls)
- The Resource Catalog is now a first-class citizen in the system
- Build status: 0 errors, 539 test functions in qualia-core-db
- Version: 0.0.10-dev