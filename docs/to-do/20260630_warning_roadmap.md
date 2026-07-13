# Compiler Warning Reduction Roadmap

**Date:** 2026-06-30
**Current state:** 630 warnings (down from 677), 491 tests passing, 54 features implemented

---

## Category C: Genuinely Dead (~129 warnings — REMOVE NOW)

Dead imports that can be safely removed with zero functional impact.

| File | Lines | Imports to Remove | Est. Warnings |
|------|-------|-------------------|---------------|
| `specialized_libs/medical_computing/mod.rs` | 9-12 | `StatisticalComputingLibrary`, `FiduciaryCrypto`, `ZkProofSystem`, `ZnsZoneManager` | ~54 |
| `specialized_libs/linear_algebra/computation.rs` | 1 | `SolversError` | ~7 |
| `specialized_libs/linear_algebra/core_types.rs` | 1, 4 | `SolversError`, `std::ops::{Add,Mul,Sub}` | ~7 |
| `specialized_libs/linear_algebra/optimization.rs` | 1, 4 | `SolversError`, `std::ops::{Add,Mul,Sub}` | ~7 |
| `specialized_libs/linear_algebra/performance.rs` | 1, 4 | `SolversError`, `std::ops::{Add,Mul,Sub}` | ~7 |
| `specialized_libs/linear_algebra/privacy.rs` | 1, 4 | `SolversError`, `std::ops::{Add,Mul,Sub}` | ~7 |
| `specialized_libs/linear_algebra/storage.rs` | 1, 4 | `SolversError`, `std::ops::{Add,Mul,Sub}` | ~8 |
| `specialized_libs/physics_simulation.rs` | 11-12 | `AmbientOrchestrationManager`, `CsdManager` | ~20 |
| `specialized_libs/financial_modeling/mod.rs` | 9-12 | `StatisticalComputingLibrary`, `FiduciaryCrypto`, `ZkProofSystem`, `ZnsZoneManager` | ~4 |
| `specialized_libs/cryptographic_library/mod.rs` | 9 | `EbpfFirewall` | ~1 |
| `specialized_libs/machine_learning.rs` | 9-11 | `AmbientOrchestrationManager`, `CsdManager`, `ZnsZoneManager` | ~3 |
| `specialized_libs/statistical_computing.rs` | 9, 17 | `CsdManager`, `std::time::Instant` | ~2 |

**Action:** Remove ~28 import lines → eliminate ~129 warnings.

---

## Category B: Intentional Stubs (~140 warnings — LEAVE OR ANNOTATE)

Correctly platform/hardware-gated. Options:
1. Leave as-is (warnings remain)
2. Add `#[allow(dead_code)]` at module level to suppress
3. Add `#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]` where appropriate

| Area | Count | Reason |
|------|-------|--------|
| Services (daemon_swarm, webtorrent, chat_relay, RPC) | 28 | Desktop-only daemons |
| Inference (DirectStorage, Metal, GPU profiler) | 21 | Platform-specific (Win/Mac/GPU) |
| Net (Nym mixnet, eBPF, BLE mesh) | 23 | Desktop-only networking |
| Solvers (QPU, disabled modules) | 16 | QPU internal feature-gated |
| SPARQL (WebSocket, federated, DID) | 42 | Network/auth stubs |
| Physics (mesh, ZNS/CSD) | 15-20 | Hardware-dependent |
| Engineering/Chemistry (ZNS/CSD fields) | 7 | Hardware-dependent |

---

## Category A: Implementable Features (~20-25)

### Simple Wiring (5)
1. **LA: SIMD capability detection** — `SIMDCapabilities` runtime CPU feature detection
2. **LA: Matrix cache eviction** — LRU/LFU/FIFO policies in `MatrixCache::evict()`
3. **SPARQL: `sign_with_did()`** — wire to identity/key-vault layer
4. **Financial: Black-Scholes options pricing** — well-defined formula
5. **Net: `initialize_nym_proxy()`** — Nym SOCKS5 proxy init (desktop-only)

### Moderate (8)
6. **LA: Parallel executor + load balancer** — work-stealing matrix ops
7. **LA: Matrix transformer** — layout conversion/compression
8. **Financial: Compliance rule engine** — KYC/AML/position limits
9. **Financial: Report distribution** — email/FTP/webhook channels
10. **Crypto: Key search engine** — full-text/semantic search over key metadata
11. **Crypto: Encryption policy enforcement** — FIPS/HIPAA/GDPR compliance
12. **Engineering: `ReliabilityAnalyzer::analyze()`** — general reliability
13. **Services: `SwarmVerify::verify()` + `IlpDispatcher::dispatch_payment()`** (desktop-only)

### Research-Grade (7)
14. **LA: Privacy engine** — ✅ implemented 2026-07-01: feature-gated pure-Rust
    BFV packed arithmetic, 48-byte external-ciphertext references, calibrated
    Laplace/Gaussian mechanisms, and basic/advanced/RDP budget accounting.
15. **Financial: Monte Carlo stress testing** — scenario analysis
16. **ML: Model converter** — PyTorch↔TF↔ONNX
17. **ML: Model compression** — ✅ implemented 2026-07-01: model-agnostic
    symmetric-int8 PTQ with error evidence, exact unstructured/output-channel
    pruning with packed masks and SGD recovery, and measured teacher-student
    distillation for the existing MLP/linear training boundary.
18. **Statistical: Query optimizer** — SQL-like cost-based optimization
19. **Engineering: CFD solver** — Navier-Stokes finite-volume
20. **Chemistry: Quantum electronic-structure** — Hartree-Fock/DFT
21. **Solvers: QPU job queue** — quantum optimization

---

## Recommended Execution Order

1. **Phase 1 (immediate):** Remove all Category C dead imports → ~129 warnings gone
2. **Phase 2 (next):** Implement 5 simple-wiring features → +~15 warnings gone, +~25 tests
3. **Phase 3:** Implement moderate features (pick 3-5 highest value)
4. **Phase 4:** Annotate Category B stubs with `#[allow(dead_code)]` where appropriate
5. **Phase 5 (long-term):** Research-grade features as needed
