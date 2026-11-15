//! Canonical binding and role-bound public-key references.
use super::cbor::{Reader, Writer};
use super::{
    Binding, KeyIds, ED25519, MAX_AUDIENCE_BYTES, MAX_BINDING_BYTES, MAX_PURPOSE_BYTES, ML_DSA_65,
    PROFILE, SCHEMA_VERSION,
};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::{ED25519_PK_LEN, ML_DSA_65_PK_LEN};
use sha2::{Digest, Sha384};

const KEY_ID_DOMAIN: &[u8] = b"QBDL-COSE-KEY-ID-V1\0";
// Canonical public COSE_Key maps followed by raw key bytes. NOT thumbprints.
const MLDSA_KEY_PREFIX: &[u8] = &[0xa3, 1, 7, 3, 0x38, 0x30, 0x20, 0x59, 0x07, 0xa0];
const ED_KEY_PREFIX: &[u8] = &[0xa4, 1, 1, 3, 0x32, 0x20, 6, 0x21, 0x58, 0x20];

/// Borrowed primitive encodings; the caller separately admits trusted authority.
pub struct PairedPublicKeys<'a> {
    pub ml_dsa_65: &'a [u8; ML_DSA_65_PK_LEN],
    pub ed25519: &'a [u8; ED25519_PK_LEN],
}
impl PairedPublicKeys<'_> {
    pub fn key_ids(&self) -> KeyIds {
        KeyIds {
            ml_dsa_65: key_id(MLDSA_KEY_PREFIX, self.ml_dsa_65),
            ed25519: key_id(ED_KEY_PREFIX, self.ed25519),
        }
    }
}
fn key_id(prefix: &[u8], key: &[u8]) -> [u8; 48] {
    let mut hash = Sha384::new();
    hash.update(KEY_ID_DOMAIN);
    hash.update(prefix);
    hash.update(key);
    hash.finalize().into()
}
pub(super) fn validate(binding: &Binding<'_>) -> Result<(), CryptoError> {
    if binding.purpose.is_empty() || binding.audience.is_empty() {
        return Err(CryptoError::Malformed);
    }
    if binding.purpose.len() > MAX_PURPOSE_BYTES || binding.audience.len() > MAX_AUDIENCE_BYTES {
        return Err(CryptoError::Capacity);
    }
    Ok(())
}

/// Encode the seven-field binding. Output is invalid on any error.
pub fn encode_binding(binding: &Binding<'_>, out: &mut [u8]) -> Result<usize, CryptoError> {
    validate(binding)?;
    let cap = out.len().min(MAX_BINDING_BYTES);
    let mut w = Writer::new(&mut out[..cap]);
    w.head(5, 7)?;
    w.head(0, 0)?;
    w.text(PROFILE)?;
    w.head(0, 1)?;
    w.head(0, SCHEMA_VERSION)?;
    w.head(0, 2)?;
    w.bytes(binding.descriptor_digest)?;
    w.head(0, 3)?;
    w.head(4, 2)?;
    for (alg, kid) in [
        (ML_DSA_65, &binding.key_ids.ml_dsa_65),
        (ED25519, &binding.key_ids.ed25519),
    ] {
        w.head(4, 2)?;
        w.integer(alg)?;
        w.bytes(kid)?;
    }
    w.head(0, 4)?;
    w.head(0, binding.controller_epoch)?;
    w.head(0, 5)?;
    w.bytes(binding.purpose)?;
    w.head(0, 6)?;
    w.bytes(binding.audience)?;
    Ok(w.len())
}

/// Parse canonical syntax only. Returned fields are explicitly UNVERIFIED.
pub fn decode_binding(input: &[u8]) -> Result<Binding<'_>, CryptoError> {
    if input.len() > MAX_BINDING_BYTES {
        return Err(CryptoError::Capacity);
    }
    let mut r = Reader::new(input);
    r.expect(5, 7)?;
    r.expect(0, 0)?;
    r.text_eq(PROFILE)?;
    r.expect(0, 1)?;
    if r.head(0)? != SCHEMA_VERSION {
        return Err(CryptoError::UnknownProfile);
    }
    r.expect(0, 2)?;
    let descriptor_digest = array48(r.bytes(48)?)?;
    r.expect(0, 3)?;
    r.expect(4, 2)?;
    r.expect(4, 2)?;
    r.negative(ML_DSA_65)?;
    let ml_dsa_65 = *array48(r.bytes(48)?)?;
    r.expect(4, 2)?;
    r.negative(ED25519)?;
    let ed25519 = *array48(r.bytes(48)?)?;
    r.expect(0, 4)?;
    let controller_epoch = r.head(0)?;
    r.expect(0, 5)?;
    let purpose = r.bytes(MAX_PURPOSE_BYTES)?;
    r.expect(0, 6)?;
    let audience = r.bytes(MAX_AUDIENCE_BYTES)?;
    r.finish()?;
    let binding = Binding {
        descriptor_digest,
        key_ids: KeyIds { ml_dsa_65, ed25519 },
        controller_epoch,
        purpose,
        audience,
    };
    validate(&binding)?;
    Ok(binding)
}
fn array48(bytes: &[u8]) -> Result<&[u8; 48], CryptoError> {
    bytes.try_into().map_err(|_| CryptoError::Malformed)
}
