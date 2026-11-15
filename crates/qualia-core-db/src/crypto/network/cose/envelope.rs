//! Strict paired Sign1 envelope and RFC 9052 signing structure.
use super::cbor::{Reader, Writer};
use super::{
    decode_binding, Binding, PairSignatures, ED25519, EXTERNAL_AAD, MAX_BINDING_BYTES,
    MAX_ENVELOPE_BYTES, MAX_PROTECTED_BYTES, MAX_SIG_STRUCTURE_BYTES, ML_DSA_65,
};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::{ED25519_SIG_LEN, ML_DSA_65_SIG_LEN};

/// Parsed syntax only, never a signature or authority grant.
pub struct ParsedPair<'a> {
    pub(super) binding: Binding<'a>,
    pub(super) payload: &'a [u8],
    pub(super) ml_dsa_signature: &'a [u8; ML_DSA_65_SIG_LEN],
    pub(super) ed_signature: &'a [u8; ED25519_SIG_LEN],
}
impl<'a> ParsedPair<'a> {
    pub fn binding(&self) -> &Binding<'a> {
        &self.binding
    }
    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }
}

fn protected(alg: i64, kid: &[u8; 48], out: &mut [u8]) -> Result<usize, CryptoError> {
    if alg != ML_DSA_65 && alg != ED25519 {
        return Err(CryptoError::Unsupported);
    }
    let mut w = Writer::new(out);
    w.head(5, 2)?;
    w.head(0, 1)?;
    w.integer(alg)?;
    w.head(0, 4)?;
    w.bytes(kid)?;
    Ok(w.len())
}
fn parse_protected(input: &[u8], alg: i64) -> Result<&[u8; 48], CryptoError> {
    let mut r = Reader::new(input);
    // Exact ordered map: rejects duplicates, crit and all unreviewed extensions.
    r.expect(5, 2)?;
    r.expect(0, 1)?;
    r.negative(alg)?;
    r.expect(0, 4)?;
    let kid = r
        .bytes(48)?
        .try_into()
        .map_err(|_| CryptoError::Malformed)?;
    r.finish()?;
    Ok(kid)
}

/// RFC 9052 Signature1 bytes, with fixed profile AAD and canonical protection.
/// This function only encodes; its result conveys no signature verification.
pub fn sig_structure_into(
    alg: i64,
    kid: &[u8; 48],
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    let binding = decode_binding(payload)?;
    let expected = match alg {
        ML_DSA_65 => &binding.key_ids.ml_dsa_65,
        ED25519 => &binding.key_ids.ed25519,
        _ => return Err(CryptoError::Unsupported),
    };
    if kid != expected {
        return Err(CryptoError::Unauthorized);
    }
    let mut header = [0; MAX_PROTECTED_BYTES];
    let n = protected(alg, kid, &mut header)?;
    let cap = out.len().min(MAX_SIG_STRUCTURE_BYTES);
    let mut w = Writer::new(&mut out[..cap]);
    w.head(4, 4)?;
    w.text("Signature1")?;
    w.bytes(&header[..n])?;
    w.bytes(EXTERNAL_AAD)?;
    w.bytes(payload)?;
    Ok(w.len())
}

pub(super) fn encoded_len(payload_len: usize) -> Result<usize, CryptoError> {
    if payload_len > MAX_BINDING_BYTES {
        return Err(CryptoError::Capacity);
    }
    let head = if payload_len < 24 {
        1
    } else if payload_len < 256 {
        2
    } else {
        3
    };
    // Two tag/array/protected/map/signature envelopes + outer array.
    // Protected lengths are 55 (-49) and 54 (-19); signature heads are 3/2.
    Ok(3498 + 2 * (head + payload_len))
}

/// Serialize canonical binding bytes and supplied signatures; does not verify.
/// No prefix is valid after an error. Detached/extension encodings are excluded.
pub fn encode_pair(
    payload: &[u8],
    signatures: &PairSignatures,
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    let binding = decode_binding(payload)?;
    let need = encoded_len(payload.len())?;
    if out.len() < need {
        return Err(CryptoError::Capacity);
    }
    let mut w = Writer::new(&mut out[..need]);
    w.head(4, 2)?;
    write_sign1(
        &mut w,
        ML_DSA_65,
        &binding.key_ids.ml_dsa_65,
        payload,
        &signatures.ml_dsa_65,
    )?;
    write_sign1(
        &mut w,
        ED25519,
        &binding.key_ids.ed25519,
        payload,
        &signatures.ed25519,
    )?;
    debug_assert_eq!(w.len(), need);
    Ok(w.len())
}
fn write_sign1(
    w: &mut Writer<'_>,
    alg: i64,
    kid: &[u8; 48],
    payload: &[u8],
    signature: &[u8],
) -> Result<(), CryptoError> {
    let mut header = [0; MAX_PROTECTED_BYTES];
    let n = protected(alg, kid, &mut header)?;
    w.head(6, 18)?;
    w.head(4, 4)?;
    w.bytes(&header[..n])?;
    w.head(5, 0)?;
    w.bytes(payload)?;
    w.bytes(signature)
}

struct Member<'a> {
    kid: &'a [u8; 48],
    payload: &'a [u8],
    signature: &'a [u8],
}
fn read_sign1<'a>(r: &mut Reader<'a>, alg: i64, sig_len: usize) -> Result<Member<'a>, CryptoError> {
    r.expect(6, 18)?;
    r.expect(4, 4)?;
    let kid = parse_protected(r.bytes(MAX_PROTECTED_BYTES)?, alg)?;
    r.expect(5, 0)?;
    let payload = r.bytes(MAX_BINDING_BYTES)?;
    let signature = r.bytes(sig_len)?;
    if signature.len() != sig_len {
        return Err(CryptoError::Malformed);
    }
    Ok(Member {
        kid,
        payload,
        signature,
    })
}

/// Parse syntax and common payload/key role consistency, without crypto checks.
pub fn decode_pair(input: &[u8]) -> Result<ParsedPair<'_>, CryptoError> {
    if input.len() > MAX_ENVELOPE_BYTES {
        return Err(CryptoError::Capacity);
    }
    let mut r = Reader::new(input);
    r.expect(4, 2)?;
    let ml = read_sign1(&mut r, ML_DSA_65, ML_DSA_65_SIG_LEN)?;
    let ed = read_sign1(&mut r, ED25519, ED25519_SIG_LEN)?;
    r.finish()?;
    if ml.payload != ed.payload {
        return Err(CryptoError::Unauthorized);
    }
    let binding = decode_binding(ml.payload)?;
    if ml.kid != &binding.key_ids.ml_dsa_65 || ed.kid != &binding.key_ids.ed25519 {
        return Err(CryptoError::Unauthorized);
    }
    Ok(ParsedPair {
        binding,
        payload: ml.payload,
        ml_dsa_signature: ml
            .signature
            .try_into()
            .map_err(|_| CryptoError::Malformed)?,
        ed_signature: ed
            .signature
            .try_into()
            .map_err(|_| CryptoError::Malformed)?,
    })
}
