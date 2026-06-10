# Task 001: Fix zk_proofs.rs Stub - Critical Security Issue

## Problem
`zk_proofs.rs:823` contains `verify_proof()` that returns `Ok(true)` with comment "// For now, always return true". A verifier that accepts any proof is worse than none because it manufactures false trust.

**File**: `crates/qualia-core-db/src/zk_proofs.rs`  
**Line**: 823  
**Severity**: 🔴 CRITICAL

## Current State
```rust
pub fn verify_proof(...) -> Result<bool, Error> {
    // For now, always return true
    return Ok(true);
}
```

## Implementation Plan

### Option A: Implement Real ZK Proofs (Recommended)
1. **Select a ZK library**:
   - `arkworks-rs` (Arkworks) - Industry standard for zk-SNARKs
   - `halo2` - PLONK-based proving system
   - `bellman` - Groth16 implementation

2. **Replace stub with real implementation**:
   ```rust
   pub fn verify_proof(
       proof: &Proof,
       public_inputs: &[u8],
       verification_key: &VerifyingKey,
   ) -> Result<bool, Error> {
       // Real ZK verification logic
       arkworks_verify(proof, public_inputs, verification_key)
   }
   ```

3. **Add dependency**:
   ```toml
   [dependencies]
   ark-bls12-381 = "0.4"
   ark-groth16 = "0.4"
   ark-serialize = "0.4"
   ```

4. **Update tests** to verify real proofs fail when tampered

### Option B: Remove Stubs and Claims (If Not Ready)
1. Remove `verify_proof()` function
2. Update documentation to remove ZK proof claims
3. Add TODO comment for future implementation

## Implementation Steps (Option A)

1. Add ZK library dependencies to Cargo.toml
2. Define proper `Proof`, `VerifyingKey`, `PublicInputs` structs
3. Implement `verify_proof()` using selected library
4. Add `generate_proof()` function for completeness
5. Update all callers to handle Result types properly
6. Write comprehensive tests:
   - Valid proof verification
   - Invalid proof rejection
   - Tampered public inputs rejection
   - Performance benchmarks

## Success Criteria
- ✅ `verify_proof()` returns `false` for invalid proofs
- ✅ `verify_proof()` returns `true` only for valid proofs
- ✅ No code path allows unconditional success
- ✅ Tests cover edge cases (tampered proofs, wrong inputs)
- ✅ Documentation updated to reflect real implementation
- ✅ README claims match actual capabilities

## Related Files
- `crates/qualia-core-db/src/zk_proofs.rs` (main)
- `crates/qualia-core-db/Cargo.toml` (dependencies)
- `crates/qualia-core-db/src/lib.rs` (exports)
- `README.md` (claims to update)

## Estimated Complexity
- Option A (Real implementation): 2-3 days
- Option B (Remove stubs): 2-3 hours

## Dependencies
- None (can be done independently)

## Notes
- This is a security-critical function. Do not ship without real implementation.
- Hand-rolled crypto is a known security risk.
- Consider using audited, battle-tested libraries.
- If implementing from scratch, require security audit before production use.