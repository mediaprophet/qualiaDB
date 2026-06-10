# Task 003: Fix ML-DSA Implementation - Replace Hand-Rolled Crypto

## Problem
The "ML-DSA" (ml_dsa_sign/ml_dsa_verify) is a hand-rolled SHA3 construction marked "simplified version for demonstration", not the actual Dilithium/FIPS-204 scheme. Hand-rolled crypto that looks like a standard is a classic security footgun.

**File**: `crates/qualia-core-db/src/fiduciary_crypto.rs` (likely)  
**Severity**: 🔴 HIGH

## Current State
```rust
// Hand-rolled SHA3 construction - NOT actual Dilithium/FIPS-204
pub fn ml_dsa_sign(message: &[u8], secret_key: &[u8]) -> Vec<u8] {
    // Simplified version for demonstration
    let hash = sha3_256(message);
    // ... some operations that look like ML-DSA but aren't
}

pub fn ml_dsa_verify(message: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    // Simplified version for demonstration
    // ... verification logic
}
```

## Implementation Plan

### Option A: Use pqcrypto-ml-dsa Library (Recommended)
Use the official Rust implementation of ML-DSA (FIPS 204).

1. **Add dependency**:
   ```toml
   [dependencies]
   pqcrypto-ml-dsa = "0.1"
   pqcrypto-traits = "0.3"
   ```

2. **Replace hand-rolled implementation**:
   ```rust
   use pqcrypto_ml_dsa::ml_dsa_65;

   pub fn ml_dsa_sign(message: &[u8], secret_key: &[u8]) -> Result<Vec<u8>, Error> {
       let sk = ml_dsa_65::SecretKey::from_bytes(secret_key)
           .map_err(|_| Error::InvalidKey)?;
       let signature = ml_dsa_65::sign(&sk, message);
       Ok(signature.as_bytes().to_vec())
   }

   pub fn ml_dsa_verify(
       message: &[u8],
       signature: &[u8],
       public_key: &[u8]
   ) -> Result<bool, Error> {
       let pk = ml_dsa_65::PublicKey::from_bytes(public_key)
           .map_err(|_| Error::InvalidKey)?;
       let sig = ml_dsa_65::Signature::from_bytes(signature)
           .map_err(|_| Error::InvalidSignature)?;
       Ok(ml_dsa_65::verify(&pk, message, &sig))
   }
   ```

3. **Support multiple security levels** (if needed):
   - ML-DSA-44 (NIST Level 2)
   - ML-DSA-65 (NIST Level 3)
   - ML-DSA-87 (NIST Level 5)

### Option B: Remove Post-Quantum Claims (If Not Ready)
1. Remove ml_dsa_sign/ml_dsa_verify functions
2. Update documentation to remove post-quantum claims
3. Add TODO comment for future implementation
4. Revert to Ed25519 for all signing operations

## Implementation Steps (Option A)

1. Add pqcrypto-ml-dsa dependency to Cargo.toml
2. Define proper key types (SecretKey, PublicKey)
3. Implement ml_dsa_sign() using pqcrypto library
4. Implement ml_dsa_verify() using pqcrypto library
5. Add key generation function:
   ```rust
   pub fn ml_dsa_generate_keypair() -> (Vec<u8>, Vec<u8>)
   ```
6. Add context binding support (if required by spec)
7. Update all callers to use Result types
8. Write comprehensive tests:
   - Valid signature verification
   - Invalid signature rejection
   - Tampered message rejection
   - Key pair generation and round-trip
   - Performance benchmarks (post-quantum is slower)

## Success Criteria
- ✅ Uses actual FIPS 204 ML-DSA algorithm
- ✅ Verifies signatures correctly
- ✅ Rejects invalid signatures
- ✅ No hand-rolled crypto operations
- ✅ Tests cover edge cases
- ✅ Documentation updated to reflect post-quantum capabilities

## Related Files
- `crates/qualia-core-db/src/fiduciary_crypto.rs` (main)
- `crates/qualia-core-db/Cargo.toml` (dependencies)
- `crates/qualia-core-db/src/lib.rs` (exports)
- `README.md` (post-quantum claims)

## Estimated Complexity
- Option A (Real ML-DSA): 2-3 days
- Option B (Remove claims): 2-3 hours

## Dependencies
- Can be done independently
- May coordinate with Tasks 001-002 (comprehensive security audit)

## Notes
- Post-quantum signatures are larger and slower than Ed25519
- Consider hybrid approach (Ed25519 + ML-DSA) for performance
- This is security-critical - hand-rolled crypto must be removed
- pqcrypto-ml-dsa is the official Rust implementation
- FIPS 204 is the NIST standard for post-quantum signatures
- If not ready for ML-DSA, remove post-quantum claims from documentation