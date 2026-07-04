# QualiaDB Q-Forge Project Plan

## Objective
To build `qualia-q-forge`, a highly optimized, pure Rust native quantum execution library. This crate unifies the core functionality of aging repositories (`quantrs2-sim`, `qiskit-qasm2`, `openqasm-rust`) into a modern, zero-allocation architecture tailored specifically for the QualiaDB ecosystem. 

**Key Directives:**
1. **Zero Python:** Pure Rust implementation with no Python bindings or dependencies.
2. **Zero-Allocation:** Hot paths (e.g., simulation loops, AST parsing) must not use `Vec`, `String`, or `Box`. All semantic data must fit into `NQuin` architectures.
3. **`no_std` Compatibility:** The core simulator and parser should ideally be `#![no_std]` to support embedded/WASM targets, dropping down to `alloc` only when absolutely necessary (and never in the hot path).

---

## Phase 1: Foundation (Current)
- [x] Create `qualia-q-forge` workspace crate.
- [x] Establish `#![no_std]` foundations and memory-strict design patterns.
- [x] Define zero-allocation OpenQASM 3 Abstract Syntax Tree (AST) using `q_hash()` for identifier resolution.
- [x] Define the `LocalSimulator` trait bounded by caller-supplied output buffers (`&mut [f64]`).

## Phase 2: OpenQASM 3 Parser implementation
- [ ] Port/write a recursive descent or `winnow`-based parser that reads OpenQASM 3 source strings directly into the `QasmProgram` fixed-size arrays.
- [ ] Map QASM gates and declarations strictly into `QasmStatement` variants without heap overhead.
- [ ] Implement compile-time validation for OpenQASM 3 structural requirements.

## Phase 3: Local StateVector Simulator
- [ ] Implement a unified StateVector backend (absorbing `quantrs2-sim` logic).
- [ ] Implement basic gate executions (Pauli X/Y/Z, Hadamard, CNOT) operating on caller-provided interleaved complex amplitude buffers (`&mut [(f64, f64)]`).
- [ ] Optimize tensor product and matrix multiplication logic for cache locality and SIMD execution (if applicable, without breaking `#![no_std]` guarantees).

## Phase 4: Semantic Integration & Extension Hooks
- [ ] Finalize the bridge inside `qualia-extensions::qpu_extension.rs` to route incoming jobs directly to the `qualia-q-forge` parser and simulator.
- [ ] Convert resulting state vectors into deterministic `NQuin` topological pointers.
- [ ] Rigorous integration testing against QGroup heuristic scheduling arrays.
