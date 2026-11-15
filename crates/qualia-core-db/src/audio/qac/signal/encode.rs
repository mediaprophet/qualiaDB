use super::*;
use crate::q42::{
    field_envelope::{EnvelopeInput, Family, StructuralView},
    field_semantics,
};
/// Explicit candidate writer using caller-owned staging and final output. All
/// bytes/metadata are preserved exactly. On every error final output is unchanged;
/// staging may change and must never be published on error. No hidden allocation.
pub fn encode(
    input: EnvelopeInput<'_>,
    envelope_limits: crate::q42::field_envelope::Limits,
    shared_limits: field_semantics::Limits,
    limits: Limits,
    scratch: &mut field_semantics::Scratch<'_>,
    staging: &mut [u8],
    out: &mut [u8],
) -> Result<usize, Error> {
    if input.family != Family::QacV1Candidate {
        return Err(Error::Unsupported);
    }
    let n = input.encoded_len(envelope_limits)?;
    if out.len() < n {
        return Err(Error::Envelope(crate::q42::field_envelope::Error::Extent));
    }
    let encoded = input.encode_into(staging, envelope_limits)?;
    let view = StructuralView::open(encoded, Family::QacV1Candidate, envelope_limits)?;
    admit(&view, 1, scratch, shared_limits, limits)?;
    out[..n].copy_from_slice(encoded);
    Ok(n)
}
