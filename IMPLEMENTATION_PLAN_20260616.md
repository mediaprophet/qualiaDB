# Implementation Plan - 2026-06-16

## Overview
Implement the architectural specifications from:
1. `crypto-concepts.md` - Cryptographic key derivation and nonce strategies
2. `10d/q42-10d-volumetric-tensor-spec.md` - 10D volumetric tensor system

---

## Phase 1: Cryptographic Infrastructure Updates (crypto-concepts.md)

### 1.1 PBKDF2 Key Derivation Enhancement
**Goal:** Implement 48-byte key derivation for sanctuary lanes

**Tasks:**
1. Update `fiduciary_crypto.rs` or key vault module to support 48-byte PBKDF2 output
2. Implement key slicing: [32 bytes cipher key | 16 bytes volume root tweak]
3. Add `derive_sanctuary_key_material(pin, salt)` function returning `(cipher_key, volume_tweak)`
4. Update Q42 volume initialization to use derived volume tweak

**Files to Modify:**
- `crates/qualia-core-db/src/fiduciary_crypto.rs`
- `crates/qualia-core-db/src/key_vault.rs`
- `crates/qualia-core-db/src/q42_volume.rs`

**Dependencies:** Already have `fips204` crate, may need `argon2` or `pbkdf2` crate

### 1.2 Implicit Domain-Separated Nonce Derivation
**Goal:** Eliminate nonce storage, derive nonces on-the-fly via XOR

**Tasks:**
1. Implement `derive_chunk_nonce(volume_tweak, chunk_index_or_offset)` function
2. Update Q42 volume encryption/decryption to use derived nonces
3. Remove nonce storage from Q42 volume metadata
4. Add tests for nonce uniqueness across volume

**Files to Modify:**
- `crates/qualia-core-db/src/q42_volume.rs`
- `crates/qualia-core-db/src/storage_driver.rs`

**Zero-Heap Constraint:** Ensure nonce derivation uses only stack registers, no heap allocation

### 1.3 AEAD Cipher Integration
**Goal:** Ensure 32-byte key works with existing AEAD ciphers

**Tasks:**
1. Verify AES-256-GCM and XChaCha20-Poly1305 compatibility
2. Update encryption tests to use 32-byte derived key
3. Add performance benchmarks for nonce derivation vs stored nonces

**Files to Modify:**
- `crates/qualia-core-db/src/fiduciary_crypto.rs`
- `crates/qualia-core-db/src/specialized_libs/cryptographic_library.rs`

---

## Phase 2: 10D Volumetric Tensor Implementation (q42-10d-volumetric-tensor-spec.md)

### 2.1 Core 10D Tensor Structure
**Goal:** Define the 10D tensor data structure [q, v, w, x, y, z, t, α, μ, σ]

**Tasks:**
1. Create `src/tensor/mod.rs` with 10D tensor struct
2. Implement coordinate types for each dimension
3. Add serialization/deserialization for tensor storage
4. Implement zero-heap memory layout (fixed-size arrays)

**Files to Create:**
- `crates/qualia-core-db/src/tensor/mod.rs`
- `crates/qualia-core-db/src/tensor/coordinate.rs`
- `crates/qualia-core-db/src/tensor/payload.rs`

**Data Structure:**
```rust
#[repr(C)]
pub struct Tensor10D {
    pub q: f32,  // Quantum Context
    pub v: f32,  // Topological Class
    pub w: f32,  // Manifold Index
    pub x: f32,  // Semantic X
    pub y: f32,  // Semantic Y
    pub z: f32,  // Semantic Z
    pub t: f32,  // Temporal State
    pub alpha: f32,  // Amplitude
    pub mu: f32,   // Modulation
    pub sigma: f32, // Spectral Signature
}
```

### 2.2 Quantum Context (q) Implementation
**Goal:** Implement epistemic superposition and wavefunction collapse

**Tasks:**
1. Implement `q` state management (0 = ground truth, >0 = parallel contexts)
2. Add context cloning for "what-if" scenarios
3. Implement wavefunction collapse mechanism
4. Add GSR integration for context resolution

**Files to Create:**
- `crates/qualia-core-db/src/tensor/quantum_context.rs`

**Dependencies:** May need async runtime for GSR communication

### 2.3 Topological Classes (v) Implementation
**Goal:** Implement geometric physics rules for manifold classification

**Tasks:**
1. Implement distance metrics for each topological class:
   - v=0: Euclidean distance
   - v=1: Cyclic/toroidal distance (modulo)
   - v=2: Hyperbolic/tree distance (exponential)
   - v=3+: Sovereign boundary cliques (byte comparison)
2. Add distance calculation functions
3. Implement bounding-volume query support

**Files to Create:**
- `crates/qualia-core-db/src/tensor/topology.rs`

### 2.4 Manifold Bifurcation (w) Implementation
**Goal:** Implement multi-head attention analog for knowledge universes

**Tasks:**
1. Define manifold domains (medical, legal, personal, environmental, socioeconomic)
2. Implement cross-manifold projection functions
3. Add GPU texture array layer mapping for batched operations
4. Implement correlation queries across multiple w

**Files to Create:**
- `crates/qualia-core-db/src/tensor/manifold.rs`

### 2.5 Spacetime Dimensions (x, y, z, t) Implementation
**Goal:** Implement semantic topology and temporal ledger

**Tasks:**
1. Implement semantic embedding coordinate system
2. Add kNN and ray-casting functions
3. Implement temporal slicing for provenance
4. Add frame evolution logging for GSR resolutions

**Files to Create:**
- `crates/qualia-core-db/src/tensor/spacetime.rs`

### 2.6 Spectral-Logical Payload [α, μ, σ] Implementation
**Goal:** Replace RGB with EM spectrum-based payload

**Tasks:**
1. Implement spectral payload structure [α, μ, σ]
2. Add multi-modal spectral decomposition (visual SPD, audio STFT/CQT)
3. Implement amplitude (α) as confidence/energy density
4. Implement modulation (μ) as DID/metadata carrier
5. Implement spectral signature (σ) as logical class
6. Add device-specific projection at render time

**Files to Create:**
- `crates/qualia-core-db/src/tensor/spectral.rs`
- `crates/qualia-core-db/src/tensor/multimodal.rs`

**Dependencies:** May need FFT library for spectral decomposition

### 2.7 Ground-State Resolver (GSR) Integration
**Goal:** Implement Tier 3 QPU escrow system

**Tasks:**
1. Implement GSR client for QPU communication
2. Add proof-of-demand mesh aggregation
3. Implement classical exhaustion fallback
4. Add stateless escrow for long-tail queries
5. Implement axiom caching for epistemic frame evolution

**Files to Create:**
- `crates/qualia-core-db/src/tensor/gsr.rs`

**Dependencies:** Network stack for QPU mesh communication

### 2.8 Hardware-Tier Dispatching
**Goal:** Route queries to appropriate hardware (CPU SIMD, GPU, QPU)

**Tasks:**
1. Implement hardware capability detection
2. Add dispatch logic for tensor operations
3. Implement fallback chains (GPU → CPU SIMD → scalar)
4. Add performance monitoring and adaptive routing

**Files to Modify:**
- `crates/qualia-core-db/src/qpu_dispatcher.rs`
- `crates/qualia-core-db/src/platform_scheduler.rs`

### 2.9 Zero-Heap Guarantees
**Goal:** Ensure all tensor operations are zero-heap on hot paths

**Tasks:**
1. Audit all tensor operations for heap allocations
2. Replace dynamic allocations with fixed-size stack arrays
3. Implement memory pool for temporary buffers
4. Add zero-heap tests and benchmarks

**Files to Modify:**
- All tensor module files
- `crates/qualia-core-db/src/slabs.rs` (may need enhancement)

### 2.10 Integration with Existing Q42 Volume
**Goal:** Integrate 10D tensor into existing Q42 volume structure

**Tasks:**
1. Update `q42_volume.rs` to use 10D tensor instead of 9D
2. Migrate existing data structures to new coordinate system
3. Add migration path for existing volumes
4. Update compression and serialization to handle new payload

**Files to Modify:**
- `crates/qualia-core-db/src/q42_volume.rs`
- `crates/qualia-core-db/src/q42_reader.rs`
- `crates/qualia-core-db/src/q42_lexicon.rs`

---

## Phase 3: Testing & Validation

### 3.1 Cryptographic Tests
- PBKDF2 48-byte derivation tests
- Nonce uniqueness tests
- AEAD encryption/decryption with derived nonces
- Performance benchmarks (stored vs derived nonces)

### 3.2 Tensor Tests
- 10D tensor serialization/deserialization
- Distance metric tests for each topological class
- Cross-manifold projection tests
- Temporal slicing and provenance tests
- Spectral payload encoding/decoding tests
- GSR integration tests (mocked)

### 3.3 Integration Tests
- End-to-end Q42 volume with 10D tensors
- Multi-modal data ingestion (visual, audio, sensor)
- Hardware-tier dispatch validation
- Zero-heap hot path validation

---

## Phase 4: Documentation

### 4.1 Update Documentation
- Update ARCHITECTURE.md with 10D tensor system
- Document PBKDF2 derivation strategy
- Add nonce derivation algorithm documentation
- Update API documentation for tensor operations

### 4.2 Migration Guide
- Document migration path from 9D to 10D
- Document performance characteristics
- Add troubleshooting guide

---

## Estimated Timeline

**Phase 1:** 2-3 days (cryptographic infrastructure)
**Phase 2:** 5-7 days (10D tensor implementation)
**Phase 3:** 3-4 days (testing and validation)
**Phase 4:** 1-2 days (documentation)

**Total:** 11-16 days

---

## Dependencies to Add

**Crate Dependencies:**
- `pbkdf2` or `argon2` for key derivation
- `rust-fft` or similar for spectral decomposition
- `ndarray` or similar for tensor operations (optional, may use custom implementation)

**Feature Flags:**
- `tensor-10d` - Enable 10D tensor system
- `gsr-integration` - Enable QPU GSR integration
- `spectral-payload` - Enable spectral payload processing

---

## Risk Assessment

**High Risk:**
- GSR integration depends on external QPU mesh network
- Spectral decomposition may introduce performance overhead

**Medium Risk:**
- Migration from 9D to 10D may break existing data
- Zero-heap constraints may limit algorithm choices

**Low Risk:**
- PBKDF2 derivation is straightforward
- Topological distance metrics are well-understood

---

## Success Criteria

1. ✅ 48-byte PBKDF2 derivation implemented
2. ✅ Implicit nonce derivation working (no stored nonces)
3. ✅ 10D tensor structure defined and zero-heap
4. ✅ All topological distance metrics implemented
5. ✅ Spectral payload replacing RGB in pipeline
6. ✅ GSR integration with fallback to classical
7. ✅ Hardware-tier dispatching functional
8. ✅ All tests passing
9. ✅ Documentation updated
10. ✅ Performance benchmarks show improvement or parity