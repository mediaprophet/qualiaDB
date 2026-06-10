# Build Error Fix Plan

**Date:** 2026-06-09  
**Target:** Fix all 44 remaining build errors  
**Estimated Time:** 14-21 hours

---

## Phase 1: Quick Wins (2-3 hours) - 10 errors

### 1.1 Fix MCP Server Type Consistency (1 error)
**File:** `crates/qualia-core-db/src/mcp_server.rs`
- Line 460: Add `.to_string()` to string literal
- Ensure both match arms return String type

### 1.2 Fix Vault Manifest CBOR-LD API (2 errors)
**File:** `crates/qualia-core-db/src/vault_manifest.rs`
- Line 120: Update `ciborium::into_writer` call with buffer
- Line 143: Update `ciborium::into_writer` call with buffer

### 1.3 Add Network Stack Stub Methods (2 errors)
**File:** Network-related modules (find exact location)
- Add `establish_wireguard_tunnel` stub
- Add `resolve_peer_dnssec` stub

### 1.4 Fix SHACL Compiler Non-Exhaustive Patterns (1 error)
**File:** `crates/qualia-core-db/src/modalities/logic/shacl.rs`
- Remove placeholder `_ => {}` match arm
- Remove commented-out enum variants or implement them

### 1.5 Fix Array to Bytes Conversion (4 errors)
**File:** `crates/qualia-core-db/src/geometric_algebra/mod.rs`
- Line 96: Implement proper array-to-bytes conversion for [f32; 3]

---

## Phase 2: Medium Complexity (4-6 hours) - 20 errors

### 2.1 Fix Geometric Algebra SIMD (4 errors)
**File:** `crates/qualia-core-db/src/geometric_algebra/simd_kernel.rs`
- Lines 117, 207: Fix SIMD pointer access
- Lines 542, 550: Fix Add/Sub trait implementations
- Add proper alignment attributes if needed

### 2.2 Fix CSD Storage Module (3 errors)
**Files:** `crates/qualia-core-db/src/csd_storage.rs`, `q42_lex.rs`
- Ensure all CSD types have Serialize/Deserialize
- Add iteration API to Q42LexMmap (or implement alternative)
- Update Q42Lexicon::from_volume to use iteration API

### 2.3 Fix Module Path References (10+ errors)
**Files:** Multiple files across codebase
- Systematically search and replace old path references
- Update all imports from old module structure
- Verify no circular dependencies

### 2.4 Specialized Libs Cleanup (3 errors)
**File:** `crates/qualia-core-db/src/lib.rs`
- Either fully implement missing modules
- Or clean up references and remove from build

---

## Phase 3: High Complexity (8-12 hours) - 14 errors

### 3.1 Fix Borrow Checker Issues (6 errors)
**Files:** Thermal monitoring and compliance modules
- Refactor shared references from Arc<Mutex> to Arc<RwLock>
- Change ownership transfer to borrowing patterns
- Update all call sites

### 3.2 Resolve Circular Dependencies (8 errors)
**Files:** `modalities/logic/` and `domains/` modules
- Extract shared types to common module
- Use forward declarations
- Merge modules if necessary

---

## Execution Order

1. Phase 1.1 → Phase 1.2 → Phase 1.3 → Phase 1.4 → Phase 1.5
2. Verify build after Phase 1
3. Phase 2.1 → Phase 2.2 → Phase 2.3 → Phase 2.4
4. Verify build after Phase 2
5. Phase 3.1 → Phase 3.2
6. Final build verification

---

## Success Criteria

- All 44 build errors resolved
- `cargo build --release -p qualia-cli` succeeds
- No new warnings introduced
- All changes committed to git