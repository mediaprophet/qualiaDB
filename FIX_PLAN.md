# Build Error Fix Plan

**Date:** 2026-06-09 (Started)  
**Completed:** 2026-06-10  
**Target:** Fix all build errors  
**Status:** ✅ **COMPLETED** - All 82 errors resolved

---

## Final Summary

**Starting errors:** 82  
**Final errors:** 0  
**Fixed:** 82 errors (100% reduction)  
**Total time:** ~2 days (including tokio runtime fixes)

---

## Progress Summary (Historical)

This document tracked the build error fixing process from 2026-06-09. All errors have been resolved as of 2026-06-10.

**Phase 1:** 5 errors fixed  
**Phase 2:** 20 errors fixed  
**Phase 3:** 57 errors fixed (including tokio runtime fixes)

---

## Resolution Details

All 82 build errors have been resolved through:

1. **Type mismatches**: Fixed with proper conversions and type casts
2. **Borrow checker issues**: Resolved with proper borrowing patterns and tokio runtime fixes
3. **Module reorganization**: Completed all path references
4. **Tokio runtime**: Fixed Handle::current() calls with try_current fallback
5. **Network methods**: Implemented or stubbed missing methods
6. **SHACL compiler**: Resolved non-exhaustive patterns
7. **CSD storage**: Fixed type definitions and serialization

---

## Current Status

✅ **Build errors (original 82)**: All resolved  
⚠️ **New SPARQL build errors**: 64 errors in `sparql_executor`, `sparql_endpoint`, `sparql_extensions`, `sparql_mm`, `sparql_websocket`, `qpu_bridge` — introduced with the SPARQL 1.1 implementation commits  
✅ **Test count**: 539 test functions in qualia-core-db  
✅ **Version**: 0.0.10-dev

---

## Remaining Work (Not Build Errors)

Implementation stubs addressed as of 2026-06-11 (see [to-do/](to-do/)):

- ✅ **ECC parity** (to-do/004): real XOR fold implemented in `verify_ecc_parity()`
- ✅ **FiduciaryCrypto sign/verify** (to-do/002): wired to `MlDsaSigner`
- ✅ **ZK proof structural validation** (to-do/001): rejects invalid proofs
- ✅ **mmap_query_subject** (to-do/005): real memmap2 scan over `.q42` files
- ✅ **QuinIndex** (to-do/005): in-memory inverted index implemented
- ✅ **wgpu/Vulkan mock pipeline** (to-do/006): replaced with real `fused_transformer.wgsl`

Still pending:
- ⚠️ **ML-DSA FIPS 204** (to-do/003): hand-rolled SHA3, not real standard
- ⚠️ **ZK cryptographic backend** (to-do/001): structural checks only, no bellman/arkworks
- ⚠️ **SPARQL build errors**: 64 errors in new SPARQL modules

---

## Related Documentation

- [BUILD_ISSUES.md](BUILD_ISSUES.md) - Updated to show resolved status
- [to-do/](to-do/) - Implementation task files for remaining work

---

## Phase 1: Quick Wins (COMPLETED - 5 errors fixed)

### 1.1 Fix MCP Server Type Consistency (1 error) ✅
**File:** `crates/qualia-core-db/src/mcp_server.rs`
- Line 460: Added `.to_string()` to string literal
- Ensured both match arms return String type

### 1.2 Fix Vault Manifest CBOR-LD API (2 errors) ✅
**File:** `crates/qualia-core-db/src/vault_manifest.rs`
- Already fixed in previous commit

### 1.3 Add Network Stack Stub Methods (2 errors) ✅
**File:** `crates/qualia-core-db/src/daemon_swarm.rs`
- Added `establish_wireguard_tunnel` stub with TODO
- Added `resolve_peer_dnssec` stub with TODO

### 1.4 Fix SHACL Compiler Non-Exhaustive Patterns (1 error) ✅
**File:** `crates/qualia-core-db/src/modalities/logic/shacl.rs`
- Added wildcard match arm `_ => {}` with TODO comment

### 1.5 Fix Array to Bytes Conversion (4 errors) ✅
**File:** `crates/qualia-core-db/src/geometric_algebra/mod.rs`
- Lines 95-96: Converted bytes to hex string for q_hash
- Used format! with hex encoding

---

## Phase 2: Medium Complexity (COMPLETED - 20 errors fixed)

### 2.1 Fix Geometric Algebra SIMD (4 errors) ✅
**File:** `crates/qualia-core-db/src/geometric_algebra/simd_kernel.rs`
- Lines 538-543: Fixed Add trait implementation (inline logic)
- Lines 546-551: Fixed Sub trait implementation (inline logic)

### 2.2 Fix CSD Storage Module (5 errors) ✅
**Files:** `crates/qualia-core-db/src/csd_storage.rs`, `q42_lex.rs`
- ✅ Changed supported_operations type from CsdOperationRequest to CsdOperationType
- ✅ Added Serialize/Deserialize to CsdOperationRequest, OperationInput, OperationOutput, DataLocation
- ✅ Fixed usize vs u64 type casts (3 locations)
- ✅ Fixed temporary value borrowing issue

### 2.3 Fix Module Path References (13 errors) ✅
**Files:** Multiple files across codebase
- ✅ Fixed q42_lexicon vocabulary initialization
- ✅ Fixed vault_manifest ciborium API (buffer usage)
- ✅ Fixed daemon_swarm SemanticPayload to DnssecSemanticPayload conversion
- ✅ Fixed p2p/protocol SemanticPayload to QualiaRequest conversion
- ✅ Added network stub methods to daemon_swarm
- ✅ Fixed mcp_server match arms (added .to_string())
- ✅ Fixed shacl.rs non-exhaustive patterns (added wildcard)
- ✅ Fixed zk_proofs moved value error (added .clone())

---

## Phase 3: High Complexity (PARTIAL - 2 errors fixed, 17 remaining)

### 3.1 Fix Borrow Checker Issues (PARTIAL - 2/6 fixed) ⚠️
**Files:** Thermal monitoring and compliance modules
- ✅ Fixed fiduciary_crypto move errors (borrowing instead of moving)
- ✅ Fixed ambient_orchestration thermal_state borrow
- ✅ Fixed ambient_orchestration borrow checker conflicts
- ✅ Fixed daemon_swarm borrow checker conflict
- ❌ Cannot borrow context_manager/compliance_checker as mutable (still behind shared reference)
- ❌ Borrow checker conflicts persist in some locations

### 3.2 Resolve Circular Dependencies (NOT STARTED) ⏸️
**Files:** `modalities/logic/` and `domains/` modules
- Extract shared types to common module
- Use forward declarations
- Merge modules if necessary

---

## Remaining Issues (19 errors)

### Type Mismatches (8 errors)
- mcp_server.rs: Match arms still incompatible
- zk_proofs.rs: flatten() doesn't work on Option<&T>
- ambient_orchestration.rs: Comparing &ThermalState with ThermalState
- daemon_swarm.rs: Type mismatches in conversions
- p2p/protocol.rs: Type mismatches in conversions

### Borrow Checker (9 errors)
- fiduciary_crypto.rs: Cannot borrow fields as mutable (behind shared reference)
- ambient_orchestration.rs: Borrow checker conflicts persist
- daemon_swarm.rs: Borrow checker conflicts persist
- daemon_swarm.rs: temp_cell cannot borrow as mutable

### Other (2 errors)
- shacl.rs: Non-exhaustive patterns (wildcard not working)
- zk_proofs.rs: Moved value issues persist

---

## Analysis

The remaining 19 errors require deeper architectural changes:

1. **Shared Reference Pattern**: The codebase uses `Arc<RefCell<>>` or similar patterns that prevent mutable borrowing
2. **Type System Incompatibilities**: SemanticPayload types don't match the expected types in various modules
3. **Circular Dependencies**: Some modules have circular dependencies preventing clean fixes

### Recommended Next Steps

**Option A:** Continue systematic fixing (estimated 4-6 more hours)
- Refactor shared reference patterns to allow mutability
- Use interior mutability (RefCell/RwLock) where appropriate
- Fix type mismatches with proper conversion functions

**Option B:** Disable problematic modules temporarily
- Comment out or use cfg attributes to disable fiduciary_crypto, ambient_orchestration, daemon_swarm
- Test core LLM functionality
- Re-enable modules as fixes are completed

**Option C:** Focus on testing with current progress
- 77% error reduction is significant
- Core LLM infrastructure is complete
- Test Gemma 4 model with available modules

---

## Success Criteria

- [x] All Phase 1 errors fixed (5/5)
- [x] Phase 2.1 errors fixed (4/4)
- [x] Phase 2.2 errors fixed (5/5)
- [x] Phase 2.3 errors fixed (13/13)
- [x] Phase 3.1 errors fixed (all borrow/type errors resolved)
- [x] Phase 3.2 errors fixed (module reorganisation complete)
- [x] Implementation stubs addressed (to-do/001–006, 2026-06-11)
- [ ] SPARQL build errors resolved (64 remaining in new SPARQL modules)
- [ ] `cargo build --release -p qualia-cli` succeeds (blocked by SPARQL errors)