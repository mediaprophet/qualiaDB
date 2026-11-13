//! Local regressions; independent primitive/envelope vectors are a separate gate.
use super::*;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::ML_DSA_65_PK_LEN;
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

const DIGEST: [u8; 48] = [0x42; 48];

struct Fixture {
    ml_sk: ml_dsa_65::PrivateKey,
    ed_sk: SigningKey,
    ml_pk: [u8; ML_DSA_65_PK_LEN],
    ed_pk: [u8; 32],
}
impl Fixture {
    fn new() -> Self {
        let (pk, sk) = ml_dsa_65::KG::keygen_from_seed(&[1; 32]);
        let ed_sk = SigningKey::from_bytes(&[2; 32]);
        let ed_pk = ed_sk.verifying_key().to_bytes();
        Self {
            ml_sk: sk,
            ed_sk,
            ml_pk: pk.into_bytes(),
            ed_pk,
        }
    }
    fn public(&self) -> PairedPublicKeys<'_> {
        PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    fn signing(&self) -> SigningKeys<'_> {
        SigningKeys {
            ml_dsa_65: &self.ml_sk,
            ed25519: &self.ed_sk,
        }
    }
    fn binding(&self) -> Binding<'static> {
        Binding {
            descriptor_digest: &DIGEST,
            key_ids: self.public().key_ids(),
            controller_epoch: 7,
            purpose: b"publication",
            audience: b"public",
        }
    }
    fn signed(&self) -> ([u8; MAX_ENVELOPE_BYTES], usize) {
        let mut out = [0; MAX_ENVELOPE_BYTES];
        let n = sign_pair_raw(
            &self.binding(),
            &self.signing(),
            &mut Workspace::new(),
            &mut out,
        )
        .unwrap();
        (out, n)
    }
}
fn syntax_binding() -> Binding<'static> {
    Binding {
        descriptor_digest: &DIGEST,
        key_ids: KeyIds {
            ml_dsa_65: [0x11; 48],
            ed25519: [0x22; 48],
        },
        controller_epoch: 0,
        purpose: b"p",
        audience: b"a",
    }
}
fn syntax_envelope() -> ([u8; MAX_ENVELOPE_BYTES], usize) {
    let mut payload = [0; MAX_BINDING_BYTES];
    let n = encode_binding(&syntax_binding(), &mut payload).unwrap();
    let mut out = [0; MAX_ENVELOPE_BYTES];
    let len = encode_pair(&payload[..n], &PairSignatures::new(), &mut out).unwrap();
    (out, len)
}

#[test]
fn canonical_binding_and_maximum_fields() {
    let binding = syntax_binding();
    let mut out = [0; MAX_BINDING_BYTES];
    let n = encode_binding(&binding, &mut out).unwrap();
    assert_eq!(&out[..3], &[0xa7, 0x00, 0x70]);
    assert_eq!(&out[3..19], b"qbdl-stream-pq-1");
    assert_eq!(decode_binding(&out[..n]).unwrap(), binding);
    let purpose = [7; MAX_PURPOSE_BYTES];
    let audience = [8; MAX_AUDIENCE_BYTES];
    let maximal = Binding {
        controller_epoch: u64::MAX,
        purpose: &purpose,
        audience: &audience,
        ..binding
    };
    let n = encode_binding(&maximal, &mut out).unwrap();
    assert!(n <= MAX_BINDING_BYTES);
    assert_eq!(decode_binding(&out[..n]).unwrap(), maximal);
    let mut wire = [0; MAX_ENVELOPE_BYTES];
    let len = encode_pair(&out[..n], &PairSignatures::new(), &mut wire).unwrap();
    assert_eq!(len, envelope::encoded_len(n).unwrap());
    assert_eq!(*decode_pair(&wire[..len]).unwrap().binding(), maximal);
    assert_eq!(
        encode_binding(
            &Binding {
                purpose: b"",
                ..binding
            },
            &mut out
        ),
        Err(CryptoError::Malformed)
    );
    assert_eq!(
        encode_binding(
            &Binding {
                audience: &[0; 129],
                ..binding
            },
            &mut out
        ),
        Err(CryptoError::Capacity)
    );
}

#[test]
fn nonminimal_duplicate_indefinite_and_trailing_binding_refused() {
    let mut out = [0; MAX_BINDING_BYTES];
    let n = encode_binding(&syntax_binding(), &mut out).unwrap();
    let mut nonminimal = out[..n].to_vec();
    nonminimal.splice(1..2, [0x18, 0x00]); // key 0 encoded nonminimally
    assert!(decode_binding(&nonminimal).is_err());
    let mut duplicate = out[..n].to_vec();
    duplicate[19] = 0; // key 1 replaced with duplicate key 0
    assert!(decode_binding(&duplicate).is_err());
    let mut indefinite = out[..n].to_vec();
    indefinite[0] = 0xbf;
    assert!(decode_binding(&indefinite).is_err());
    let mut trailing = out[..n].to_vec();
    trailing.push(0);
    assert!(decode_binding(&trailing).is_err());
}

#[test]
fn exact_signing_structure_and_capacity() {
    let binding = syntax_binding();
    let mut payload = [0; MAX_BINDING_BYTES];
    let n = encode_binding(&binding, &mut payload).unwrap();
    let mut structure = [0; MAX_SIG_STRUCTURE_BYTES];
    let len = sig_structure_into(
        ML_DSA_65,
        &binding.key_ids.ml_dsa_65,
        &payload[..n],
        &mut structure,
    )
    .unwrap();
    assert_eq!(&structure[..12], b"\x84\x6aSignature1");
    assert_eq!(&structure[12..16], &[0x58, 55, 0xa2, 1]);
    assert!(structure[..len]
        .windows(EXTERNAL_AAD.len())
        .any(|w| w == EXTERNAL_AAD));
    assert_eq!(
        sig_structure_into(-8, &binding.key_ids.ed25519, &payload[..n], &mut structure),
        Err(CryptoError::Unsupported)
    );
    assert_eq!(
        sig_structure_into(
            ED25519,
            &binding.key_ids.ml_dsa_65,
            &payload[..n],
            &mut structure
        ),
        Err(CryptoError::Unauthorized)
    );
    assert_eq!(
        encode_pair(&payload[..n], &PairSignatures::new(), &mut [0; 8]),
        Err(CryptoError::Capacity)
    );
}

#[test]
fn strict_envelope_refuses_every_truncated_prefix_and_extra_bytes() {
    let (out, len) = syntax_envelope();
    assert!(decode_pair(&out[..len]).is_ok());
    for end in 0..len {
        assert!(decode_pair(&out[..end]).is_err(), "prefix {end}");
    }
    let mut extra = out[..len].to_vec();
    extra.push(0);
    assert!(decode_pair(&extra).is_err());
    assert!(decode_pair(&[0; MAX_ENVELOPE_BYTES + 1]).is_err());
}

#[test]
fn stripping_algorithm_header_and_critical_header_attacks_fail() {
    let (out, len) = syntax_envelope();
    // The first member starts 82 d2 84 58 37 a2 01 38 30 04 58 30.
    assert_eq!(
        &out[..12],
        &[0x82, 0xd2, 0x84, 0x58, 55, 0xa2, 1, 0x38, 0x30, 4, 0x58, 48]
    );
    for (index, value) in [(0, 0x81), (1, 0xc0), (5, 0xa3), (9, 1), (9, 2), (8, 0x12)] {
        let mut changed = out;
        changed[index] = value;
        assert!(decode_pair(&changed[..len]).is_err(), "index {index}");
    }
    // Swap whole roles; array remains valid CBOR but profile order is invalid.
    let split = out[..len]
        .windows(4)
        .rposition(|w| w == [0xd2, 0x84, 0x58, 54])
        .unwrap();
    let mut swapped = Vec::from([0x82]);
    swapped.extend_from_slice(&out[split..len]);
    swapped.extend_from_slice(&out[1..split]);
    assert!(decode_pair(&swapped).is_err());
}

#[test]
fn mismatched_pair_payload_and_protected_kid_fail() {
    let (out, len) = syntax_envelope();
    let parsed = decode_pair(&out[..len]).unwrap();
    let p = parsed.payload();
    let start = out[..len].windows(p.len()).rposition(|w| w == p).unwrap();
    let mut changed = out;
    changed[start + p.len() - 1] ^= 1; // second audience only
    assert!(decode_pair(&changed[..len]).is_err());
    let mut changed = out;
    changed[12] ^= 1; // first protected key ID, not binding
    assert!(decode_pair(&changed[..len]).is_err());
}

#[test]
fn actual_pair_verifies_and_each_signature_is_required() {
    let f = Fixture::new();
    let (out, len) = f.signed();
    let expected = f.binding();
    let mut workspace = Workspace::new();
    assert!(verify_pair(&out[..len], &expected, &f.public(), &mut workspace).is_ok());
    let parsed = decode_pair(&out[..len]).unwrap();
    let ml_at = out[..len]
        .windows(3309)
        .position(|w| w == parsed.ml_dsa_signature)
        .unwrap();
    for index in [ml_at, len - 1] {
        let mut corrupt = out;
        corrupt[index] ^= 1;
        assert_eq!(
            verify_pair(&corrupt[..len], &expected, &f.public(), &mut workspace).err(),
            Some(CryptoError::Unauthorized)
        );
    }
    let mut stripped = out;
    stripped[len - 64..len].fill(0);
    assert!(verify_pair(&stripped[..len], &expected, &f.public(), &mut workspace).is_err());
}

#[test]
fn caller_expected_scope_epoch_digest_and_pair_are_enforced() {
    let f = Fixture::new();
    let (out, len) = f.signed();
    let expected = f.binding();
    let mut workspace = Workspace::new();
    for wrong in [
        Binding {
            controller_epoch: 8,
            ..expected
        },
        Binding {
            purpose: b"route",
            ..expected
        },
        Binding {
            audience: b"another",
            ..expected
        },
        Binding {
            descriptor_digest: &[9; 48],
            ..expected
        },
    ] {
        assert_eq!(
            verify_pair(&out[..len], &wrong, &f.public(), &mut workspace).err(),
            Some(CryptoError::Unauthorized)
        );
    }
    let other_ed = SigningKey::from_bytes(&[3; 32]).verifying_key().to_bytes();
    let wrong_keys = PairedPublicKeys {
        ml_dsa_65: &f.ml_pk,
        ed25519: &other_ed,
    };
    assert!(verify_pair(&out[..len], &expected, &wrong_keys, &mut workspace).is_err());
    let wrong_binding = Binding {
        key_ids: wrong_keys.key_ids(),
        ..expected
    };
    assert_eq!(
        sign_pair_raw(
            &wrong_binding,
            &f.signing(),
            &mut workspace,
            &mut [0; MAX_ENVELOPE_BYTES]
        ),
        Err(CryptoError::Unauthorized)
    );
}

#[test]
fn legacy_contextual_signatures_cannot_be_relabelled_cose() {
    let f = Fixture::new();
    let binding = f.binding();
    let mut payload = [0; MAX_BINDING_BYTES];
    let n = encode_binding(&binding, &mut payload).unwrap();
    let (_, legacy_sk) = ml_dsa_65::KG::keygen_from_seed(&[1; 32]);
    let mut signatures = PairSignatures::new();
    crate::crypto::network::mldsa::sign(
        &legacy_sk.into_bytes(),
        &payload[..n],
        EXTERNAL_AAD,
        &mut signatures.ml_dsa_65,
    )
    .unwrap();
    crate::crypto::network::ed25519::sign_with_context(
        &[2; 32],
        &payload[..n],
        EXTERNAL_AAD,
        &mut signatures.ed25519,
    )
    .unwrap();
    let mut out = [0; MAX_ENVELOPE_BYTES];
    let len = encode_pair(&payload[..n], &signatures, &mut out).unwrap();
    assert!(verify_pair(&out[..len], &binding, &f.public(), &mut Workspace::new()).is_err());
    assert!(!crate::crypto::network::dual_sign::cose_sign1_frozen());
}
