use crate::NQuin;
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
pub fn compute_scoped_merkle_root(frame: &[NQuin], author_did: u64) -> [u8; 32] {
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
pub fn sign_agency_root(signing_key: &SigningKey, sub_root_hash: &[u8; 32]) -> Signature {
    // The Ed25519-dalek library natively signs raw byte arrays.
    signing_key.sign(sub_root_hash)
}

/// The Verification Gate
/// Validates an incoming 64-byte signature against the author's Public Key (`VerifyingKey`).
/// Only validates the specific claims matching the author's DID, ensuring Bilateral Integrity.
pub fn verify_human_agency(
    frame: &[NQuin],
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

/// Sign an Author-Scoped Merkle Sub-Root with post-quantum ML-DSA-65 (FIPS-204).
///
/// Caller supplies the fixed-size 3309-byte output buffer for the signature.
/// Zero heap allocations.
pub fn sign_agency_root_pq(
    signing_key: &[u8; crate::crypto::network::types::ML_DSA_65_SK_LEN],
    sub_root_hash: &[u8; 32],
    out_sig: &mut [u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN],
) -> Result<(), AgencyError> {
    use fips204::ml_dsa_65::PrivateKey;
    use fips204::traits::{SerDes, Signer};

    let Ok(sk) = PrivateKey::try_from_bytes(*signing_key) else {
        return Err(AgencyError::InvalidSignature);
    };
    let Ok(sig) = sk.try_sign(sub_root_hash, b"qualia:agency:pq:v1") else {
        return Err(AgencyError::InvalidSignature);
    };
    out_sig.copy_from_slice(&sig);
    Ok(())
}

/// Verify an Author-Scoped Merkle Sub-Root with post-quantum ML-DSA-65 (FIPS-204).
///
/// Validates an incoming 3309-byte FIPS-204 signature against the author's
/// 1952-byte ML-DSA-65 public key over the author-scoped Merkle sub-root.
/// Guaranteed zero heap allocations in hot path.
pub fn verify_human_agency_pq(
    frame: &[NQuin],
    author_did: u64,
    public_key: &[u8; crate::crypto::network::types::ML_DSA_65_PK_LEN],
    signature: &[u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN],
) -> Result<(), AgencyError> {
    use fips204::ml_dsa_65::PublicKey;
    use fips204::traits::{SerDes, Verifier};

    let expected_sub_root = compute_scoped_merkle_root(frame, author_did);

    let Ok(pk) = PublicKey::try_from_bytes(*public_key) else {
        return Err(AgencyError::InvalidSignature);
    };

    if pk.verify(&expected_sub_root, signature, b"qualia:agency:pq:v1") {
        Ok(())
    } else {
        Err(AgencyError::InvalidSignature)
    }
}

/// Sign an Author-Scoped Merkle Sub-Root with hybrid DualProof (ML-DSA-65 + Ed25519).
///
/// Produces both post-quantum and classical signatures over the identical Merkle sub-root.
/// Zero heap allocations.
pub fn sign_agency_root_dual(
    mldsa_sk: &[u8; crate::crypto::network::types::ML_DSA_65_SK_LEN],
    ed25519_seed: &[u8; 32],
    sub_root_hash: &[u8; 32],
    out: &mut crate::crypto::network::dual_sign::DualProof,
) -> Result<(), AgencyError> {
    crate::crypto::network::dual_sign::sign_dual(
        mldsa_sk,
        ed25519_seed,
        sub_root_hash,
        b"qualia:agency:dual:v1",
        out,
    )
    .map_err(|_| AgencyError::InvalidSignature)
}

/// Verify an Author-Scoped Merkle Sub-Root with hybrid DualProof (ML-DSA-65 + Ed25519).
///
/// Both signatures must verify mathematically over the recomputed Merkle sub-root.
/// If either fails, the transaction is rejected.
/// Guaranteed zero heap allocations in hot path.
pub fn verify_human_agency_dual(
    frame: &[NQuin],
    author_did: u64,
    ed25519_pk: &[u8; crate::crypto::network::types::ED25519_PK_LEN],
    mldsa_pk: &[u8; crate::crypto::network::types::ML_DSA_65_PK_LEN],
    proof: &crate::crypto::network::dual_sign::DualProof,
) -> Result<(), AgencyError> {
    let expected_sub_root = compute_scoped_merkle_root(frame, author_did);

    crate::crypto::network::dual_sign::verify_dual(
        mldsa_pk,
        ed25519_pk,
        &expected_sub_root,
        b"qualia:agency:dual:v1",
        proof,
    )
    .map_err(|_| AgencyError::InvalidSignature)
}

/// Stamp fiduciary metadata and refresh the XOR parity block before WAL commit.
/// `principal_did_hash` is embedded in `context`; agent identity in metadata low bits.
pub fn stamp_fiduciary_metadata(quin: &mut NQuin, principal_did_hash: u64, agent_did_hash: u64) {
    quin.context = principal_did_hash;
    let agent_lane = agent_did_hash & 0xFFFF;
    let principal_clock = (principal_did_hash >> 16) & 0x1FFF_FFFF;
    quin.metadata = agent_lane | (principal_clock << 16);
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context ^ quin.metadata;
}

/// Volatile zero of all Quin fields after WAL commit (wipes transient LLM state).
pub fn scrub_quin_volatile(quin: &mut NQuin) {
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
pub fn sign_graph_mutation(signing_key: &SigningKey, quin: &NQuin) -> Signature {
    let frame = [*quin];
    let root = compute_scoped_merkle_root(&frame, quin.context);
    sign_agency_root(signing_key, &root)
}

/// Derives a 32-byte AES-256-GCM key from the user's PIN for Deniable Encryption (Sanctuary Mode).
/// By passing different PINs, different keys are derived, which unlocks different DB Lanes.
/// The decoy lane operates exactly identically to the sanctuary lane.
#[cfg(all(feature = "sanctuary-crypto", not(target_arch = "wasm32")))]
const LANE_KEY_LENGTH: usize = 32;

#[cfg(all(feature = "sanctuary-crypto", not(target_arch = "wasm32")))]
pub fn derive_lane_key(pin: &str, salt: &[u8]) -> [u8; LANE_KEY_LENGTH] {
    crate::sanctuary_crypto::derive_lane_cipher_key(
        pin.as_bytes(),
        salt,
        crate::sanctuary_crypto::DEFAULT_PBKDF2_ITERATIONS,
    )
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

        let mut frame = [NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 10];

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

    #[test]
    fn test_human_agency_pq_and_dual_verification() {
        use crate::crypto::network::dual_sign::DualProof;
        use crate::crypto::network::mldsa::generate_keypair;
        use crate::crypto::network::types::{ED25519_SIG_LEN, ML_DSA_65_SIG_LEN};

        let author_did_alice = 5001;
        let mut frame = [NQuin {
            subject: 42,
            predicate: 100,
            object: 200,
            context: author_did_alice,
            metadata: 0,
            parity: 0,
        }; 4];

        let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
        let ed_seed = [99u8; 32];
        let ed_signing = SigningKey::from_bytes(&ed_seed);
        let ed_pk_bytes: [u8; 32] = ed_signing.verifying_key().to_bytes();

        let alice_root = compute_scoped_merkle_root(&frame, author_did_alice);

        // 1. Post-Quantum ML-DSA-65 test
        let mut pq_sig = [0u8; ML_DSA_65_SIG_LEN];
        sign_agency_root_pq(&mldsa_sk, &alice_root, &mut pq_sig).unwrap();
        assert_eq!(
            verify_human_agency_pq(&frame, author_did_alice, &mldsa_pk, &pq_sig),
            Ok(())
        );

        // 2. Hybrid DualProof test
        let mut dual_proof = DualProof {
            mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
            ed25519_sig: [0u8; ED25519_SIG_LEN],
        };
        sign_agency_root_dual(&mldsa_sk, &ed_seed, &alice_root, &mut dual_proof).unwrap();
        assert_eq!(
            verify_human_agency_dual(
                &frame,
                author_did_alice,
                &ed_pk_bytes,
                &mldsa_pk,
                &dual_proof
            ),
            Ok(())
        );

        // 3. Tampering detection
        frame[0].subject = 9999;
        assert_eq!(
            verify_human_agency_pq(&frame, author_did_alice, &mldsa_pk, &pq_sig),
            Err(AgencyError::InvalidSignature)
        );
        assert_eq!(
            verify_human_agency_dual(
                &frame,
                author_did_alice,
                &ed_pk_bytes,
                &mldsa_pk,
                &dual_proof
            ),
            Err(AgencyError::InvalidSignature)
        );
    }

    #[cfg(all(feature = "sanctuary-crypto", not(target_arch = "wasm32")))]
    #[test]
    fn derive_lane_key_is_deterministic_and_salt_bound() {
        let pin = "1234";
        let salt_a = b"sanctuary";
        let salt_b = b"decoy";

        let key_a1 = derive_lane_key(pin, salt_a);
        let key_a2 = derive_lane_key(pin, salt_a);
        let key_b = derive_lane_key(pin, salt_b);
        let key_c = derive_lane_key("4321", salt_a);

        assert_eq!(key_a1, key_a2);
        assert_ne!(key_a1, key_b);
        assert_ne!(key_a1, key_c);
        assert_eq!(
            key_a1,
            crate::sanctuary_crypto::derive_lane_cipher_key(
                pin.as_bytes(),
                salt_a,
                crate::sanctuary_crypto::DEFAULT_PBKDF2_ITERATIONS,
            )
        );
    }
}
