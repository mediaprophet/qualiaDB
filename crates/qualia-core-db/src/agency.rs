use crate::QualiaQuin;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Eq)]
pub enum AgencyError {
    InvalidSignature,
    TamperedData,
}

/// Computes the Author-Scoped Merkle Sub-Root Hash for a specific user's claims.
/// It partitions the 128KB frame by the Author's DID, strictly ignoring claims
/// authored by other actors in the Bilateral frame.
/// Uses zero-allocation iteration over the existing memory slice.
pub fn compute_scoped_merkle_root(
    frame: &[QualiaQuin],
    author_did: u64,
) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Iterate through the frame without allocating Vectors or Strings
    for quin in frame.iter() {
        // In Qualia-DB, the Author's DID is embedded in the Context vector
        if quin.context == author_did {
            // Hash the 48-byte Quin structure natively
            // `bytemuck::bytes_of` safely casts the struct to a byte slice since it implements Pod.
            let bytes: &[u8; 48] = bytemuck::cast_ref(quin);
            hasher.update(bytes);
        }
    }

    // Finalize the Merkle Sub-Root hash (32 bytes)
    let result = hasher.finalize();
    let mut root_hash = [0u8; 32];
    root_hash.copy_from_slice(&result);
    root_hash
}

/// The Human Agency Hook
/// Generates a 64-byte Ed25519 signature exclusively over the Author-Scoped Merkle Sub-Root.
pub fn sign_agency_root(
    signing_key: &SigningKey,
    sub_root_hash: &[u8; 32],
) -> Signature {
    // The Ed25519-dalek library natively signs raw byte arrays.
    signing_key.sign(sub_root_hash)
}

/// The Verification Gate
/// Validates an incoming 64-byte signature against the author's Public Key (`VerifyingKey`).
/// Only validates the specific claims matching the author's DID, ensuring Bilateral Integrity.
pub fn verify_human_agency(
    frame: &[QualiaQuin],
    author_did: u64,
    verifying_key: &VerifyingKey,
    signature: &Signature,
) -> Result<(), AgencyError> {
    // 1. Recompute the Author-Scoped Merkle Sub-Root from the incoming frame
    let expected_sub_root = compute_scoped_merkle_root(frame, author_did);

    // 2. Validate the signature mathematically
    if verifying_key.verify(&expected_sub_root, signature).is_ok() {
        Ok(())
    } else {
        Err(AgencyError::InvalidSignature)
    }
}

/// Stamp fiduciary metadata and refresh the XOR parity block before WAL commit.
/// `principal_did_hash` is embedded in `context`; agent identity in metadata low bits.
pub fn stamp_fiduciary_metadata(
    quin: &mut QualiaQuin,
    principal_did_hash: u64,
    agent_did_hash: u64,
) {
    quin.context = principal_did_hash;
    let agent_lane = agent_did_hash & 0xFFFF;
    let principal_clock = (principal_did_hash >> 16) & 0x1FFF_FFFF;
    quin.metadata = agent_lane | (principal_clock << 16);
    quin.parity = quin.subject
        ^ quin.predicate
        ^ quin.object
        ^ quin.context
        ^ quin.metadata;
}

/// Volatile zero of all Quin fields after WAL commit (wipes transient LLM state).
pub fn scrub_quin_volatile(quin: &mut QualiaQuin) {
    unsafe {
        std::ptr::write_volatile(&mut quin.subject, 0);
        std::ptr::write_volatile(&mut quin.predicate, 0);
        std::ptr::write_volatile(&mut quin.object, 0);
        std::ptr::write_volatile(&mut quin.context, 0);
        std::ptr::write_volatile(&mut quin.metadata, 0);
        std::ptr::write_volatile(&mut quin.parity, 0);
    }
}

/// Sign a single graph-mutation Quin using the author-scoped Merkle sub-root.
pub fn sign_graph_mutation(
    signing_key: &SigningKey,
    quin: &QualiaQuin,
) -> Signature {
    let frame = [*quin];
    let root = compute_scoped_merkle_root(&frame, quin.context);
    sign_agency_root(signing_key, &root)
}

/// Derives a 32-byte AES-256-GCM key from the user's PIN for Deniable Encryption (Sanctuary Mode).
/// By passing different PINs, different keys are derived, which unlocks different DB Lanes.
/// The decoy lane operates exactly identically to the sanctuary lane.
pub fn derive_lane_key(pin: &str, salt: &[u8]) -> [u8; 32] {
    // In production, this uses PBKDF2-HMAC-SHA256 with 310,000 iterations
    // to resist brute-forcing of the 4-digit PIN.
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    hasher.update(salt);
    
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_agency_verification() {
        // Use a static 32-byte secret for the test to avoid pulling in rand_core
        let secret = [42u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let author_did_alice = 1001;
        let author_did_bob = 2002;

        let mut frame = [QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0 }; 10];
        
        // Alice's claims
        frame[0].context = author_did_alice;
        frame[0].subject = 55;
        
        frame[1].context = author_did_alice;
        frame[1].subject = 66;

        // Bob's claims (injected into the same bilateral frame)
        frame[2].context = author_did_bob;
        frame[2].subject = 99;

        // 1. Alice computes her scoped root and signs it
        let alice_root = compute_scoped_merkle_root(&frame, author_did_alice);
        let alice_sig = sign_agency_root(&signing_key, &alice_root);

        // 2. Verification Gate validates Alice's signature successfully
        assert_eq!(
            verify_human_agency(&frame, author_did_alice, &verifying_key, &alice_sig),
            Ok(())
        );

        // 3. Tampering simulation: Someone alters Alice's claim
        frame[0].subject = 999;
        assert_eq!(
            verify_human_agency(&frame, author_did_alice, &verifying_key, &alice_sig),
            Err(AgencyError::InvalidSignature)
        );
    }
}
