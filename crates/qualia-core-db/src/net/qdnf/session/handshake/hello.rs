//! Oversized ClientHello / ServerHello codec: hybrid share plus dual proof.

use crate::crypto::network::digest::sha384;
use crate::crypto::network::dual_sign::{DualProof, sign_dual, verify_dual};
use crate::crypto::network::ed25519::public_from_seed;
use crate::crypto::network::pq_handshake::{
    INITIATOR_SHARE_WIRE_LEN, decode_initiator_share, encode_initiator_share, qsession_domain,
};
use crate::crypto::network::share_encoding::{
    RESPONDER_SHARE_WIRE_LEN, decode_responder_share, encode_responder_share,
};
use crate::crypto::network::types::{
    ED25519_PK_LEN, ED25519_SIG_LEN, ML_DSA_65_PK_LEN, ML_DSA_65_SIG_LEN, ML_DSA_65_SK_LEN,
};
use crate::net::qdnf::bearer::fragment::MAX_HANDSHAKE_BYTES;
use crate::net::qdnf::bearer::mtu::MAX_QDNF_MTU;
use crate::net::qdnf::crypto::handshake::{InitiatorShare, ResponderShare};
use crate::net::qdnf::errors::QdnfError;

/// `share || ML-DSA-65 pk || Ed25519 pk || dual proof`.
const HELLO_CERT_LEN: usize = ML_DSA_65_PK_LEN + ED25519_PK_LEN;
const HELLO_PROOF_LEN: usize = ML_DSA_65_SIG_LEN + ED25519_SIG_LEN;

pub const CLIENT_HELLO_WIRE_LEN: usize =
    INITIATOR_SHARE_WIRE_LEN + HELLO_CERT_LEN + HELLO_PROOF_LEN;
pub const SERVER_HELLO_WIRE_LEN: usize =
    RESPONDER_SHARE_WIRE_LEN + HELLO_CERT_LEN + HELLO_PROOF_LEN;

const _: () = assert!(CLIENT_HELLO_WIRE_LEN > MAX_QDNF_MTU as usize);
const _: () = assert!(SERVER_HELLO_WIRE_LEN > MAX_QDNF_MTU as usize);
const _: () = assert!(CLIENT_HELLO_WIRE_LEN <= MAX_HANDSHAKE_BYTES);
const _: () = assert!(SERVER_HELLO_WIRE_LEN <= MAX_HANDSHAKE_BYTES);

fn signed_prefix_len(share_len: usize) -> Result<usize, QdnfError> {
    share_len
        .checked_add(HELLO_CERT_LEN)
        .ok_or(QdnfError::Range)
}

fn append_hello(
    share: &[u8],
    mldsa_pk: &[u8; ML_DSA_65_PK_LEN],
    ed_pk: &[u8; ED25519_PK_LEN],
    proof: &DualProof,
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let n = share
        .len()
        .checked_add(HELLO_CERT_LEN)
        .and_then(|v| v.checked_add(HELLO_PROOF_LEN))
        .ok_or(QdnfError::Range)?;
    if out.len() < n {
        return Err(QdnfError::Capacity);
    }
    let mut off = 0usize;
    out[off..off + share.len()].copy_from_slice(share);
    off += share.len();
    out[off..off + ML_DSA_65_PK_LEN].copy_from_slice(mldsa_pk);
    off += ML_DSA_65_PK_LEN;
    out[off..off + ED25519_PK_LEN].copy_from_slice(ed_pk);
    off += ED25519_PK_LEN;
    out[off..off + ML_DSA_65_SIG_LEN].copy_from_slice(&proof.mldsa_sig);
    off += ML_DSA_65_SIG_LEN;
    out[off..off + ED25519_SIG_LEN].copy_from_slice(&proof.ed25519_sig);
    off += ED25519_SIG_LEN;
    Ok(off)
}

fn parse_proof_and_certs(
    src: &[u8],
    share_len: usize,
) -> Result<([u8; ML_DSA_65_PK_LEN], [u8; ED25519_PK_LEN], DualProof), QdnfError> {
    let need = share_len
        .checked_add(HELLO_CERT_LEN)
        .and_then(|v| v.checked_add(HELLO_PROOF_LEN))
        .ok_or(QdnfError::Range)?;
    if src.len() < need {
        return Err(QdnfError::Truncated);
    }
    if src.len() != need {
        return Err(QdnfError::Malformed);
    }
    let mut mldsa_pk = [0u8; ML_DSA_65_PK_LEN];
    let mut ed_pk = [0u8; ED25519_PK_LEN];
    let mut proof = DualProof {
        mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
        ed25519_sig: [0u8; ED25519_SIG_LEN],
    };
    let mut off = share_len;
    mldsa_pk.copy_from_slice(&src[off..off + ML_DSA_65_PK_LEN]);
    off += ML_DSA_65_PK_LEN;
    ed_pk.copy_from_slice(&src[off..off + ED25519_PK_LEN]);
    off += ED25519_PK_LEN;
    proof
        .mldsa_sig
        .copy_from_slice(&src[off..off + ML_DSA_65_SIG_LEN]);
    off += ML_DSA_65_SIG_LEN;
    proof
        .ed25519_sig
        .copy_from_slice(&src[off..off + ED25519_SIG_LEN]);
    let _ = off;
    Ok((mldsa_pk, ed_pk, proof))
}

fn sign_hello_body(
    share: &[u8],
    mldsa_sk: &[u8; ML_DSA_65_SK_LEN],
    ed_seed: &[u8; 32],
    mldsa_pk: &[u8; ML_DSA_65_PK_LEN],
    ed_pk: &[u8; ED25519_PK_LEN],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let signed = signed_prefix_len(share.len())?;
    if out.len() < signed {
        return Err(QdnfError::Capacity);
    }
    out[..share.len()].copy_from_slice(share);
    out[share.len()..share.len() + ML_DSA_65_PK_LEN].copy_from_slice(mldsa_pk);
    out[share.len() + ML_DSA_65_PK_LEN..signed].copy_from_slice(ed_pk);
    let mut proof = DualProof {
        mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
        ed25519_sig: [0u8; ED25519_SIG_LEN],
    };
    sign_dual(
        mldsa_sk,
        ed_seed,
        // Ed25519 context binder is 512 bytes; dual-sign the SHA-384 of the prefix.
        &sha384(&out[..signed]).0,
        qsession_domain(),
        &mut proof,
    )?;
    append_hello(share, mldsa_pk, ed_pk, &proof, out)
}

fn verify_hello_body(src: &[u8], share_len: usize) -> Result<(), QdnfError> {
    let signed = signed_prefix_len(share_len)?;
    let (mldsa_pk, ed_pk, proof) = parse_proof_and_certs(src, share_len)?;
    verify_dual(
        &mldsa_pk,
        &ed_pk,
        &sha384(&src[..signed]).0,
        qsession_domain(),
        &proof,
    )
}

/// Encode ClientHello with the certs that match the signing secrets.
pub fn encode_client_hello_with_certs(
    share: &InitiatorShare,
    mldsa_sk: &[u8; ML_DSA_65_SK_LEN],
    mldsa_pk: &[u8; ML_DSA_65_PK_LEN],
    ed_seed: &[u8; 32],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let mut share_wire = [0u8; INITIATOR_SHARE_WIRE_LEN];
    encode_initiator_share(share, &mut share_wire)?;
    let ed_pk = public_from_seed(ed_seed);
    sign_hello_body(&share_wire, mldsa_sk, ed_seed, mldsa_pk, &ed_pk, out)
}

/// Encode ServerHello with the certs that match the signing secrets.
pub fn encode_server_hello_with_certs(
    share: &ResponderShare,
    mldsa_sk: &[u8; ML_DSA_65_SK_LEN],
    mldsa_pk: &[u8; ML_DSA_65_PK_LEN],
    ed_seed: &[u8; 32],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let mut share_wire = [0u8; RESPONDER_SHARE_WIRE_LEN];
    encode_responder_share(&share.ml_kem_ct, &share.x25519_pk, &mut share_wire)?;
    let ed_pk = public_from_seed(ed_seed);
    sign_hello_body(&share_wire, mldsa_sk, ed_seed, mldsa_pk, &ed_pk, out)
}

pub fn decode_client_hello(src: &[u8]) -> Result<InitiatorShare, QdnfError> {
    if src.len() < INITIATOR_SHARE_WIRE_LEN {
        return Err(QdnfError::Truncated);
    }
    verify_hello_body(src, INITIATOR_SHARE_WIRE_LEN)?;
    decode_initiator_share(&src[..INITIATOR_SHARE_WIRE_LEN])
}

pub fn decode_server_hello(src: &[u8]) -> Result<ResponderShare, QdnfError> {
    if src.len() < RESPONDER_SHARE_WIRE_LEN {
        return Err(QdnfError::Truncated);
    }
    verify_hello_body(src, RESPONDER_SHARE_WIRE_LEN)?;
    let (ct, pk) = decode_responder_share(&src[..RESPONDER_SHARE_WIRE_LEN])?;
    Ok(ResponderShare {
        ml_kem_ct: ct,
        x25519_pk: pk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::MlKem768Secret;
    use crate::crypto::network::mldsa;
    use crate::net::qdnf::crypto::handshake::initiator_share;

    #[test]
    fn client_hello_wire_len_includes_ed25519_sig() {
        let (sk, pk) = MlKem768Secret::generate().unwrap();
        let _ = sk;
        let x = [3u8; 32];
        let share = initiator_share(&x, &pk).unwrap();
        let (mldsa_sk, mldsa_pk) = mldsa::generate_keypair().unwrap();
        let ed = [9u8; 32];
        let mut out = [0u8; CLIENT_HELLO_WIRE_LEN];
        let n = encode_client_hello_with_certs(&share, &mldsa_sk, &mldsa_pk, &ed, &mut out).unwrap();
        assert_eq!(n, CLIENT_HELLO_WIRE_LEN);
        decode_client_hello(&out[..n]).unwrap();
    }
}
