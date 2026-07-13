---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# crypto Index

## Functionality Overview
Comprehensive index of functionality for `crypto`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `deontic_circuit.rs`
  - `struct DeonticAccessCircuit`
  - `impl ConstraintSynthesizer`
  - `fn generate_constraints`
  - `struct ZkAccessVerifier`
  - `impl ZkAccessVerifier`
  - `fn new`
  - `fn verify_access`
  - `impl Default`
  - `fn default`
  - `fn generate_deontic_crs`
  - `fn test_circuit_setup`
  - `fn test_deontic_proof_roundtrip_and_soundness`
- 📄 `fiduciary_crypto.rs`
  - `struct MlDsaSigner`
  - `struct MlDsaPrivateKey`
  - `struct MlDsaPublicKey`
  - `struct MlDsaSignature`
  - `struct MlDsaKeyManager`
  - `struct KeyRotationPolicy`
  - `struct CryptoContext`
  - `struct FiduciaryCrypto`
  - `struct ContextManager`
  - `struct ComplianceChecker`
  - `struct FiduciaryStandards`
  - `struct AuditEntry`
  - `impl MlDsaSigner`
  - `fn generate_keypair`
  - `fn from_keypair`
  - *(...and 49 more)*
- 📄 `mod.rs`
- 📄 `pq_kem_shim.rs`
  - `enum KemCiphertext`
  - `impl KemCiphertext`
  - `fn as_bytes`
  - `fn from_bytes`
  - `enum KemVariant`
  - `enum KemPublicKey`
  - `impl KemPublicKey`
  - `enum KemSecretKey`
  - `impl KemSecretKey`
  - `struct KemSharedSecret`
  - `enum KemError`
  - `trait PostQuantumSerialize`
  - `fn to_fixed_bytes`
  - `fn from_fixed_bytes`
  - `impl PostQuantumSerialize`
  - *(...and 7 more)*
- 📄 `sanctuary_crypto.rs`
  - `enum SanctuaryAeadAlgorithm`
  - `enum SanctuaryCryptoError`
  - `struct SanctuaryKeyMaterial`
  - `impl fmt`
  - `fn fmt`
  - `fn derive_sanctuary_key_material`
  - `fn derive_lane_cipher_key`
  - `fn derive_chunk_nonce`
  - `fn derive_chacha_nonce`
  - `fn derive_xchacha_nonce`
  - `fn encrypt_sanctuary_chunk_in_place`
  - `fn decrypt_sanctuary_chunk_in_place`
  - `fn encrypt_sanctuary_chunk`
  - `fn decrypt_sanctuary_chunk`
  - `fn derive_compact_nonce`
  - *(...and 7 more)*
- 📄 `verifiable_credential.rs`
  - `struct Credential`
  - `enum VcError`
  - `fn digest`
  - `fn issue`
  - `fn verify`
  - `fn verify_grounded`
  - `fn key`
  - `fn quin`
  - `fn sample`
  - `fn issue_and_verify_roundtrip`
  - `fn tampered_claim_fails_verification`
  - `fn wrong_issuer_key_fails`
  - `fn expired_credential_fails`
  - `fn ungrounded_ai_issuer_is_rejected_but_grounded_one_is_accepted`
- 📄 `zk_proofs.rs`
  - `struct ZkProofSystem`
  - `struct ProvingKey`
  - `struct VerifyingKey`
  - `struct CircuitParameters`
  - `enum EllipticCurve`
  - `struct CircuitBuilder`
  - `struct ArithmeticCircuit`
  - `struct CircuitVariable`
  - `enum VariableType`
  - `struct CircuitConstraint`
  - `enum CircuitExpression`
  - `struct FieldElement`
  - `struct ProofGenerator`
  - `struct WitnessGenerator`
  - `struct ProvingEngine`
  - *(...and 77 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
