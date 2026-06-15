# QualiaDB Audit Report

Date: 2026-06-16

This report covers a targeted code audit of the QualiaDB repository focused on concrete correctness, safety, and build-health issues. It is high-signal, not exhaustive: the repo is large, so this should be treated as a source-backed audit of the most important problems surfaced by review plus build/test execution, not as a proof that no additional issues exist.

## Scope And Verification

Commands run:

- `cargo test -p qualia-core-db --lib`
- `cargo check --workspace`
- Targeted reruns of failing tests

Observed results:

- `cargo test -p qualia-core-db --lib` finished with `768 passed; 3 failed`
- Failing tests:
  - `acoustic_ble_mesh::tests::test_node_discovery`
  - `ambient_orchestration::tests::test_device_discovery`
  - `modalities::graph_theory::tests::test_centrality_calculation`
- `cargo check --workspace` completed, but emitted a very large warning surface including an ignored dependency, undefined feature gates, FFI-unsound signatures, and several zero-allocation violations.

Post-remediation verification:

- `cargo test -p qualia-core-db --lib` now passes with `777 passed; 0 failed`
- Targeted reruns for the previously failing mesh, ambient orchestration, graph centrality, and webizen tests all passed

## Executive Summary

The audit initially surfaced two top-priority security issues: an unsound FFI ownership boundary in `directml_bridge.rs` and a weak PIN-to-key derivation path in `agency.rs`. Both were patched during this audit pass and verified with targeted compile/test runs. Eight additional correctness and configuration issues in `acoustic_ble_mesh.rs`, `webizen.rs`, `ambient_orchestration.rs`, `daemon_graph.rs`, `modalities/graph_theory.rs`, `modalities/logic/core.rs`, `specialized_libs/linear_algebra.rs`, and the DNSSEC dependency wiring were also fixed during the same pass. The most important remaining open issue is the repo's still-large warning surface outside the remediated audit findings.

## Remediation Status

- Resolved during audit: `directml_bridge.rs` FFI ownership/ABI unsafety
- Resolved during audit: `agency.rs` single-round SHA-256 lane-key derivation
- Resolved during audit: `acoustic_ble_mesh.rs` discovery/state mismatch
- Resolved during audit: `webizen.rs` dead `NativeRk4Step` `calculus` gate
- Resolved during audit: `ambient_orchestration.rs` stale hardware-dependent discovery test
- Resolved during audit: `modalities/graph_theory.rs` flawed centrality test topology
- Resolved during audit: `modalities/graph_theory.rs` heap-backed batch boundary and zero-allocation claim mismatch
- Resolved during audit: `modalities/logic/core.rs` allocating defeasible-pruning and diagnostics paths
- Resolved during audit: `daemon_graph.rs` same-batch ontology deduplication bug
- Mitigated during audit: `daemon_graph.rs` test flakiness from shared process-global graph state
- Resolved during audit: `specialized_libs/linear_algebra.rs` dead mutex-lock warning
- Resolved during audit: `trust-dns-resolver` DNSSEC feature wiring in `Cargo.toml`
- Still open: large warning surface outside the remediated audit findings

## Findings

### 1. Resolved During Audit: `directml_bridge.rs` had undefined behavior across its FFI boundary

Files:

- `crates/qualia-core-db/src/directml_bridge.rs:56-62`
- `crates/qualia-core-db/src/directml_bridge.rs:84-100`
- `crates/qualia-core-db/src/directml_bridge.rs:117-140`
- `crates/qualia-core-db/src/directml_bridge.rs:206-216`

Original problem:

- `create_d3d12_device_ffi()` leaks a `Box<DmlDevice>` but returns a pointer to the inner `d3d12` field, not to the `DmlDevice` allocation itself.
- `destroy_d3d12_device_ffi()` later reconstructs `Box<DmlDevice>` from that field pointer.
- That is an invalid `Box::from_raw` and is undefined behavior.
- The same file also exposes `extern "C"` functions returning Rust `Result<... , DmlError>`, which is not ABI-safe and is explicitly called out by the compiler during `cargo check`.

Original impact:

- Foreign callers can trigger memory corruption, invalid frees, or crashes.
- Even if this looks unused today, the boundary is not safe to expose.

Resolution:

- The FFI surface now returns owning opaque pointers to `DmlDevice`/`IocpHandle`.
- `extern "C"` functions now use a `#[repr(C)]` `DmlStatus` enum plus out-pointers instead of returning Rust `Result`.
- Destruction now reconstructs boxes from the owning allocation pointer, eliminating the invalid `Box::from_raw` path.
- Verified with `cargo check -p qualia-core-db` and a focused calculus host test rerun.

### 2. Resolved During Audit: `NativeRk4Step` is no longer dead code behind a nonexistent `calculus` feature

Files:

- `crates/qualia-core-db/src/webizen.rs:569-596`
- `crates/qualia-core-db/Cargo.toml:7-13`

Original problem:

- `webizen.rs` gates RK4 execution behind `#[cfg(feature = "calculus")]`.
- `crates/qualia-core-db/Cargo.toml` does not define a `calculus` feature.
- `cargo check` warns about the unexpected cfg value.

Original impact:

- The RK4 execution block can never compile in.
- The fallback branch always logs that calculus is not enabled, so the opcode is silently disabled even though the calculus module exists.

Resolution:

- Removed the dead `#[cfg(feature = "calculus")]` / `#[cfg(not(feature = "calculus"))]` split from the RK4 dispatch block in `webizen.rs`.
- Wired the opcode through the existing always-compiled calculus module and adapted the implementation to the real `VmFrame` register model instead of the nonexistent `current_quin` field.
- Verified with `cargo check -p qualia-core-db` and `cargo test -p qualia-core-db webizen::tests::test_async_retrieve_logic -- --exact --nocapture`.

### 3. Resolved During Audit: sanctuary-mode lane keys were derived with a single SHA-256 round

File:

- `crates/qualia-core-db/src/agency.rs:96-109`

Original problem:

- `derive_lane_key(pin, salt)` uses one SHA-256 hash over the PIN and salt.
- The comment above it says production requires `PBKDF2-HMAC-SHA256` with 310,000 iterations.

Original impact:

- Offline brute force against low-entropy PINs is practical.
- This directly weakens the deniable-encryption / sanctuary-lane story.

Resolution:

- `derive_lane_key()` now uses `PBKDF2-HMAC-SHA256` with 310,000 iterations and a 32-byte output key.
- `pbkdf2` was added to `crates/qualia-core-db/Cargo.toml`.
- Verified with `cargo check -p qualia-core-db` and focused `agency` tests.

### 4. Resolved During Audit: mesh discovery now updates network state instead of leaving status at zero nodes

Files:

- `crates/qualia-core-db/src/acoustic_ble_mesh.rs:1176-1205`
- `crates/qualia-core-db/src/acoustic_ble_mesh.rs:1245-1249`
- `crates/qualia-core-db/src/acoustic_ble_mesh.rs:1343-1384`
- `crates/qualia-core-db/src/acoustic_ble_mesh.rs:1412-1447`
- `crates/qualia-core-db/src/acoustic_ble_mesh.rs:2125-2133`

Original problem:

- `MeshNetworkManager::discover_nodes()` returns discovered acoustic and BLE nodes, but neither `AcousticNetwork::discover_nodes()` nor `BleNetwork::discover_nodes()` inserts them into `self.nodes`.
- `get_network_status()` reports counts from `self.acoustic_network.get_node_count()` and `self.ble_network.get_node_count()`, which only read those maps.

Original evidence:

- `cargo test -p qualia-core-db acoustic_ble_mesh::tests::test_node_discovery -- --exact --nocapture`
- Failure: `assertion failed: status.total_nodes > 0`

Original impact:

- Discovery appears to work to callers that inspect the returned vector, but network status remains wrong.
- Any downstream routing/health logic depending on stored node counts will be inconsistent.

Resolution:

- `AcousticNetwork::discover_nodes()` and `BleNetwork::discover_nodes()` now insert discovered nodes into their backing `self.nodes` maps before returning them.
- Verified with `cargo test -p qualia-core-db acoustic_ble_mesh::tests::test_node_discovery -- --exact --nocapture`.

### 5. Resolved During Audit: ambient device discovery test now checks hardware-backed invariants instead of a dummy count

Files:

- `crates/qualia-core-db/src/ambient_orchestration.rs:533-620`
- `crates/qualia-core-db/src/ambient_orchestration.rs:1106-1113`

Original problem:

- `discover_devices()` now uses `sysinfo` to register one aggregate host plus up to eight logical CPU-core devices.
- The test still asserts `devices.len() == 10` and comments "10 dummy devices".

Original evidence:

- `cargo test -p qualia-core-db ambient_orchestration::tests::test_device_discovery -- --exact --nocapture`
- Failure on this machine: `left: 9 right: 10`

Original impact:

- The lib test suite is red even though the implementation changed to real hardware enumeration.
- The test is no longer expressing the contract of the function.

Resolution:

- The test now asserts stable invariants for real hardware discovery: discovery returns at least one device, includes `local_host`, registers a consistent device count, and exposes status for every discovered device.
- Verified with `cargo test -p qualia-core-db ambient_orchestration::tests::test_device_discovery -- --exact --nocapture`.

### 6. Resolved During Audit: the graph centrality test now uses a topology where node 2 is actually a bridge

Files:

- `crates/qualia-core-db/src/modalities/graph_theory.rs:32-76`
- `crates/qualia-core-db/src/modalities/graph_theory.rs:79-149`
- `crates/qualia-core-db/src/modalities/graph_theory.rs:438-452`

Original problem:

- The test graph is `1 -> 2`, `2 -> 3`, and `1 -> 3`.
- The test comment says node 2 is "between nodes 1 and 3", but in that graph the direct edge `1 -> 3` is the shortest path, so node 2 is not on that shortest path.
- The test therefore expects a higher betweenness score for node 2 when the encoded graph does not justify it.

Original evidence:

- `cargo test -p qualia-core-db modalities::graph_theory::tests::test_centrality_calculation -- --exact --nocapture`
- Failure: `assertion failed: node2_centrality > node1_centrality`

Original impact:

- The suite is red.
- It is unclear whether the intended semantics are directed, undirected, or "relationship graph regardless of edge direction".

Resolution:

- Removed the direct `1 -> 3` edge from the test graph so node 2 is the only bridge on the path from 1 to 3.
- Verified with `cargo test -p qualia-core-db modalities::graph_theory::tests::test_centrality_calculation -- --exact --nocapture`.

### 7. Resolved During Audit: `graph_theory.rs` is now explicitly quarantined as heap-backed batch analysis

Files:

- `crates/qualia-core-db/src/modalities/graph_theory.rs:1-15`
- `crates/qualia-core-db/src/modalities/graph_theory.rs:361-393`

Original problem:

- The file header says "Zero-allocation centrality, community detection, and motif finding".
- The implementation is based on `HashMap`, `Vec`, `VecDeque`, and `HashSet`.

Original impact:

- This directly conflicts with the repo's stated "zero heap in hot paths" rule.
- It also makes the file's header comment misleading to future contributors.

Resolution:

- Removed the false zero-allocation claim and documented the module as heap-backed, bounded batch analysis rather than a hot-path primitive.
- Added `MAX_HEAP_GRAPH_ANALYSIS_QUINS` and `GraphAnalysisError::InputTooLarge` so oversized topology jobs are rejected deterministically instead of silently expanding heap usage on constrained nodes.
- `analyze_graph_topology()` now returns `Result<GraphAnalysisResult, GraphAnalysisError>`, making the quarantine boundary explicit to callers.
- Added a regression test covering the oversized-input guard.
- Verified with `cargo test -p qualia-core-db modalities::graph_theory::tests --lib` and `cargo test -p qualia-core-db --lib`.

### 8. Mitigated During Audit: daemon graph tests now isolate the process-global graph state

Files:

- `crates/qualia-core-db/src/daemon_graph.rs:13-17`
- `crates/qualia-core-db/src/daemon_graph.rs:209-251`

Original problem:

- The daemon graph is a process-static `OnceLock<RwLock<Vec<NQuin>>>`.
- Tests mutate that same shared graph and do not isolate state per test case.

Evidence:

- An earlier full-suite run reported `daemon_graph::tests::replace_graph_from_flat_bytes_round_trip` as failed.
- The same test passed when rerun in isolation.

Original impact:

- This is a strong signal of order dependence / shared-state flakiness under the default parallel test runner.
- The same design can also leak state across daemon operations if callers expect per-session isolation.

Mitigation:

- Added explicit test reset helpers around the shared graph and marked the `daemon_graph` tests `#[serial_test::serial]` so the module no longer shares dirty graph state across its own test cases.
- Verified with `cargo test -p qualia-core-db daemon_graph:: --lib`.

Residual risk:

- The production design is still process-global by intent, so a deeper instance-owned graph refactor could still be worthwhile if daemon sessions ever need hard isolation inside one process.

### 9. Resolved During Audit: ontology graph deduplication now updates as each quin is accepted

File:

- `crates/qualia-core-db/src/daemon_graph.rs:143-156`

Original problem:

- `extend_with_ontology_quins()` builds `existing` once from the current graph.
- It never updates that set after pushing new quins.
- If the incoming `quins` vector contains duplicates, the second duplicate is not filtered because `existing` is stale.

Original impact:

- Duplicate quins can accumulate during a single extension call even though the function claims to deduplicate.

Resolution:

- `extend_with_ontology_quins()` now mutates the dedupe set as each quin is inserted and only appends when `existing.insert(key)` succeeds.
- The append path now also respects the existing `MAX_GRAPH_QUINS` guard via `push_quin()`.
- Added a regression test that feeds the same quin twice in one batch and verifies only one row is stored.

### 10. Resolved During Audit: `logic/core.rs` no longer allocates in defeasible pruning or differential diagnostics

Files:

- `crates/qualia-core-db/src/modalities/logic/core.rs:94-99`
- `crates/qualia-core-db/src/modalities/logic/core.rs:442-465`
- `crates/qualia-core-db/src/modalities/logic/core.rs:492-543`

Original problem:

- `prune_defeasible_claims()` took `&mut Vec<NQuin>` and allocated a `HashSet`.
- `execute_differential_diagnostics()` returned `Vec<NQuin>`.
- `compile_diagnostic_constraint()` and `compile_mock_constraint()` also allocated rule vectors before loading them into the VM.

Original impact:

- These paths directly conflicted with the "caller supplies buffers / no heap in hot paths" contract documented in `AGENTS.md`.
- The issue was architectural, not cosmetic.

Resolution:

- `prune_defeasible_claims()` now operates on `&mut [NQuin]`, performs stable in-place compaction without heap allocation, and clears the tail of the caller buffer after pruning.
- `execute_differential_diagnostics()` now writes into a caller-supplied `&mut [NQuin]` and returns `Result<usize, DiagnosticError>`, with `DiagnosticError::OutputBufferFull` for bounded overflow handling.
- `compile_mock_constraint()` and `compile_diagnostic_constraint()` now return fixed opcode slices backed by static arrays instead of allocating `Vec<WebizenOpcode>`.
- `WebizenVM::load_bytecode()` now clears stale bytecode slots before copying the active instruction slice, preventing residual opcodes from a prior load.
- Verified with `cargo check -p qualia-core-db` and `cargo test -p qualia-core-db --lib`.

### 11. Resolved During Audit: Cargo now recognizes the DNSSEC resolver dependency configuration

Files:

- `crates/qualia-core-db/Cargo.toml:167-168`
- `crates/qualia-core-db/src/daemon_swarm.rs:851-860`

Original problem:

- `cargo check --workspace` warns: `qualia-core-db ... ignoring invalid dependency trust-dns which is missing a lib target`.
- The code path in `daemon_swarm.rs` expects DNSSEC-validating resolution through `trust_dns_resolver`.
- The invalid dependency means Cargo is discarding part of the intended DNS dependency configuration entirely.

Original impact:

- At minimum, the build is misleading and partially misconfigured.
- At worst, the intended DNSSEC / TLS feature set is not actually being applied to the resolver path.

Resolution:

- Removed the invalid `trust-dns` dependency entry and moved the intended DNSSEC/TLS feature wiring onto `trust-dns-resolver`.
- `trust-dns-resolver` is now declared with `dnssec-ring` and `dns-over-rustls`, which matches the crate feature set used by `daemon_swarm.rs`.
- Verified with `cargo check -p qualia-core-db`, and the old "ignoring invalid dependency trust-dns" warning no longer appears.

### 12. Resolved During Audit: `LinearAlgebraLibrary::initialize()` no longer performs a dead mutex lock

File:

- `crates/qualia-core-db/src/specialized_libs/linear_algebra.rs:1577-1581`

Original problem:

- `self.zk_proofs.lock().unwrap();` is a no-op because the guard is immediately dropped.
- The compiler warns about this exact issue.

Resolution:

- Removed the dead lock acquisition from `initialize()` because it was not acting as a real synchronization barrier and had no side effects.
- Verified with `cargo check -p qualia-core-db` and `cargo test -p qualia-core-db --lib`.

Impact:

- If the lock was intended as an initialization barrier, it does not serve that purpose.
- If it was only a liveness check, the code is unclear and misleading.

Recommendation:

- Either bind the guard for the intended scope or remove the call.

## Suggested Priority Order

1. Eliminate the zero-heap violations in core logic paths.
2. Triage the daemon graph shared-state hazards and the remaining warning surface.

## Notes

- The warning count is very high; many are duplicate unused items, but several are real correctness signals and should not be ignored simply because the workspace still compiles.
- The full suite failure count and the source-backed issues above already justify a follow-up stabilization pass before adding more modality surface area.
