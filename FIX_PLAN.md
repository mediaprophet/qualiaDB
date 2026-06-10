# Build Error Fix Plan

**Date:** 2026-06-09  
**Target:** Fix all 44 remaining build errors  
**Estimated Time:** 14-21 hours

---

## Progress Summary

**Starting errors:** 82  
**Current errors:** 44  
**Fixed:** 38 errors (46% reduction)  
**Commits:** 2 (98f7684, 3c60fcb)

---

## Phase 1: Quick Wins (COMPLETED - 4 errors fixed)

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

## Phase 2: Medium Complexity (PARTIAL - 2 errors fixed)

### 2.1 Fix Geometric Algebra SIMD (4 errors) ✅
**File:** `crates/qualia-core-db/src/geometric_algebra/simd_kernel.rs`
- Lines 538-543: Fixed Add trait implementation (inline logic)
- Lines 546-551: Fixed Sub trait implementation (inline logic)

### 2.2 Fix CSD Storage Module (3 errors) ⚠️ IN PROGRESS
**Files:** `crates/qualia-core-db/src/csd_storage.rs`, `q42_lex.rs`
- ✅ Added Serialize/Deserialize to CsdOperationRequest
- ✅ Added Serialize/Deserialize to OperationInput
- ✅ Added Serialize/Deserialize to OperationOutput
- ⚠️ Remaining issues: Type mismatches, usize vs u64, temporary value borrowing
- Status: Deeper structural issues requiring extensive refactoring

### 2.3 Fix Module Path References (10+ errors) ⏸️ PENDING
**Files:** Multiple files across codebase
- Systematically search and replace old path references
- Update all imports from old module structure
- Verify no circular dependencies

### 2.4 Specialized Libs Cleanup (3 errors) ⏸️ PENDING
**File:** `crates/qualia-core-db/src/lib.rs`
- Either fully implement missing modules
- Or clean up references and remove from build

---

## Phase 3: High Complexity (NOT STARTED - 14 errors)

### 3.1 Fix Borrow Checker Issues (6 errors) ⏸️ PENDING
**Files:** Thermal monitoring and compliance modules
- Refactor shared references from Arc<Mutex> to Arc<RwLock>
- Change ownership transfer to borrowing patterns
- Update all call sites

### 3.2 Resolve Circular Dependencies (8 errors) ⏸️ PENDING
**Files:** `modalities/logic/` and `domains/` modules
- Extract shared types to common module
- Use forward declarations
- Merge modules if necessary

---

## Remaining Work

### Immediate Blockers
The CSD storage module has deeper structural issues that require:
1. Type system redesign (CsdOperationType vs CsdOperationRequest)
2. Consistent type usage (usize vs u64)
3. Borrowing pattern fixes (temporary value lifetime issues)

### Recommended Approach
Given the complexity of remaining errors (44), consider:

**Option A:** Continue systematic fixing (estimated 8-12 more hours)
- Focus on CSD module refactoring first
- Then address borrow checker issues
- Finally resolve circular dependencies

**Option B:** Temporarily disable problematic modules
- Disable CSD storage, thermal monitoring, specialized libs
- Test LLM functionality with core modules only
- Re-enable modules incrementally as fixes are completed

---

## Success Criteria

- [x] All Phase 1 errors fixed (4/4)
- [x] Phase 2.1 errors fixed (4/4)
- [ ] Phase 2.2 errors fixed (0/3 - structural issues)
- [ ] Phase 2.3 errors fixed (0/10+)
- [ ] Phase 2.4 errors fixed (0/3)
- [ ] Phase 3.1 errors fixed (0/6)
- [ ] Phase 3.2 errors fixed (0/8)
- [ ] `cargo build --release -p qualia-cli` succeeds
- [ ] No new warnings introduced