# Task 002: Fix fiduciary_crypto.rs Signature Verification Stub

## Problem
`fiduciary_crypto.rs:565` has `Verifier::verify()` that ignores the signature entirely and returns `Ok(true)`. The corresponding `sign()` returns a placeholder signature `h: vec![0u8;32]`.

**File**: `crates/qualia-core-db/src/fiduciary_crypto.rs`  
**Line**: 565  
**Severity**: 🔴 CRITICAL

## Current State
```rust
impl Verifier {
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, Error> {
        // Ignores signature entirely
        return Ok(true);
    }
}

impl Signer {
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        // Placeholder signature
        return vec![0u8; 32];
    }
}
```

## Implementation Plan

### Use Ed25519 (Recommended)
Qualia already uses Ed25519 in `agency.rs` for author-scoped merkle roots. Leverage existing infrastructure.

1. **Use existing ed25519-dalek dependency** (already in project)
2. **Replace stub with real Ed25519 verification**:
   ```rust
   use ed25519_dalek::{Verifier, Signer, SigningKey, VerifyingKey};

   pub struct RealVerifier {
       public_key: VerifyingKey,
   }

   impl RealVerifier {
       pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, Error> {
           let sig = ed25519_dalek::Signature::from_bytes(signature)
               .map_err(|e| Error::InvalidSignature)?;
           self.public_key.verify(message, &sig)
               .map(|_| true)
               .map_err(|e| Error::VerificationFailed(e.to_string()))
       }
   }
   ```

3. **Replace placeholder sign with real signing**:
   ```rust
   pub struct RealSigner {
       secret_key: SigningKey,
   }

   impl RealSigner {
       pub fn sign(&self, message: &[u8]) -> Vec<u8> {
           self.secret_key.sign(message).to_bytes().to_vec()
       }
   }
   ```

## Implementation Steps

1. Check if `ed25519-dalek` is already in dependencies
2. Define proper `PublicKey` and `SecretKey` types
3. Implement `Verifier::verify()` with real Ed25519 verification
4. Implement `Signer::sign()` with real Ed25519 signing
5. Add key generation functions:
   ```rust
   pub fn generate_keypair() -> (SigningKey, VerifyingKey)
   ```
6. Update all callers to use real verification
7. Write comprehensive tests:
   - Valid signature verification
   - Invalid signature rejection
   - Tampered message rejection
   - Key pair generation and round-trip

## Success Criteria
- ✅ `verify()` returns `false` for invalid signatures
- ✅ `verify()` returns `true` only for valid signatures
- ✅ `sign()` generates actual cryptographic signatures
- ✅ Tampered messages are rejected
- ✅ Tests cover edge cases
- ✅ Integration with existing Ed25519 infrastructure in `agency.rs`

## Related Files
- `crates/qualia-core-db/src/fiduciary_crypto.rs` (main)
- `crates/qualia-core-db/src/agency.rs` (existing Ed25519 usage)
- `crates/qualia-core-db/Cargo.toml` (dependencies)
- `crates/qualia-core-db/src/lib.rs` (exports)

## Estimated Complexity
- 1-2 days (using existing ed25519-dalek)

## Dependencies
- Can be done independently
- May coordinate with Task 001 (security audit of all crypto)

## Notes
- Ed25519 is already used elsewhere in the project (agency.rs)
- Leverage existing patterns rather than introducing new crypto libraries
- Ensure deterministic signing if needed for CRDT operations
- Consider key storage and rotation policies
- This is security-critical - no silent acceptance failures