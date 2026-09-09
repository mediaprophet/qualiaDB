//! Decode untrusted label bytes into [`LabelFields`].

use super::types::{
    Confidentiality, LabelFields, MAX_COMPARTMENTS, MAX_LABEL_BYTES, MAX_PURPOSES,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Layout: version(1) || conf(1) || restrictions(2) || n_comp(1) || n_purp(1)
/// || audience(48) || issuer(48) || compartments || purposes.
pub const HEADER_LEN: usize = 1 + 1 + 2 + 1 + 1 + 48 + 48;

pub fn decode_label_into(original: &[u8], out: &mut LabelFields) -> Result<(), QdnfError> {
    *out = LabelFields::blank();
    if original.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if original.len() > MAX_LABEL_BYTES {
        return Err(QdnfError::Capacity);
    }
    if original.len() < HEADER_LEN {
        return Err(QdnfError::Truncated);
    }
    if original[0] != 1 {
        return Err(QdnfError::Unsupported);
    }
    out.confidentiality = Confidentiality::from_wire(original[1])?;
    out.restriction_bits = u16::from_be_bytes([original[2], original[3]]);
    let n_comp = original[4] as usize;
    let n_purp = original[5] as usize;
    if n_comp > MAX_COMPARTMENTS || n_purp > MAX_PURPOSES {
        return Err(QdnfError::Capacity);
    }
    let extra = n_comp
        .checked_mul(48)
        .and_then(|c| n_purp.checked_mul(48).and_then(|p| c.checked_add(p)))
        .ok_or(QdnfError::Range)?;
    let need = HEADER_LEN.checked_add(extra).ok_or(QdnfError::Range)?;
    if original.len() != need {
        return Err(QdnfError::Malformed);
    }
    out.audience = copy_digest(&original[6..54])?;
    out.issuer = copy_digest(&original[54..102])?;
    if out.issuer.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let mut off = HEADER_LEN;
    let mut i = 0usize;
    while i < n_comp {
        let d = copy_digest(&original[off..off + 48])?;
        if d.is_zero() {
            return Err(QdnfError::Malformed);
        }
        let mut j = 0usize;
        while j < i {
            if out.compartments[j] == d {
                return Err(QdnfError::Overlap);
            }
            j += 1;
        }
        out.compartments[i] = d;
        off += 48;
        i += 1;
    }
    out.compartment_count = n_comp as u8;
    let mut p = 0usize;
    while p < n_purp {
        let d = copy_digest(&original[off..off + 48])?;
        if d.is_zero() {
            return Err(QdnfError::Malformed);
        }
        let mut j = 0usize;
        while j < p {
            if out.purposes[j] == d {
                return Err(QdnfError::Overlap);
            }
            j += 1;
        }
        out.purposes[p] = d;
        off += 48;
        p += 1;
    }
    out.purpose_count = n_purp as u8;
    Ok(())
}

fn copy_digest(bytes: &[u8]) -> Result<StrongDigest, QdnfError> {
    StrongDigest::from_bytes(bytes)
}

fn write_digest(dst: &mut [u8], digest: StrongDigest) {
    dst.copy_from_slice(&digest.0);
}

/// Encode `fields` to exact original bytes for verification.
pub fn encode_label_into(fields: &LabelFields, dst: &mut [u8]) -> Result<usize, QdnfError> {
    let n_comp = fields.compartment_count as usize;
    let n_purp = fields.purpose_count as usize;
    if n_comp > MAX_COMPARTMENTS || n_purp > MAX_PURPOSES {
        return Err(QdnfError::Capacity);
    }
    let extra = n_comp
        .checked_mul(48)
        .and_then(|c| n_purp.checked_mul(48).and_then(|p| c.checked_add(p)))
        .ok_or(QdnfError::Range)?;
    let need = HEADER_LEN.checked_add(extra).ok_or(QdnfError::Range)?;
    if need > MAX_LABEL_BYTES {
        return Err(QdnfError::Capacity);
    }
    if dst.len() < need {
        return Err(QdnfError::Capacity);
    }
    dst[0] = 1;
    dst[1] = fields.confidentiality.to_wire();
    let rb = fields.restriction_bits.to_be_bytes();
    dst[2] = rb[0];
    dst[3] = rb[1];
    dst[4] = fields.compartment_count;
    dst[5] = fields.purpose_count;
    write_digest(&mut dst[6..54], fields.audience);
    write_digest(&mut dst[54..102], fields.issuer);
    let mut off = HEADER_LEN;
    let mut i = 0usize;
    while i < n_comp {
        write_digest(&mut dst[off..off + 48], fields.compartments[i]);
        off += 48;
        i += 1;
    }
    let mut p = 0usize;
    while p < n_purp {
        write_digest(&mut dst[off..off + 48], fields.purposes[p]);
        off += 48;
        p += 1;
    }
    Ok(need)
}
