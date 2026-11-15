//! Verification-only adapter; no key manager, private-key custody, or grant owner.
use super::{
    decode_pair, sig_structure_into, Binding, PairedPublicKeys, Workspace, ED25519, ML_DSA_65,
};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::signing_profile;

/// Both signatures match the caller's expected binding under the supplied pair.
/// NOT proof of trusted controller authority, current consent or policy.
pub struct VerifiedPair<'a> {
    binding: Binding<'a>,
}
impl<'a> VerifiedPair<'a> {
    pub fn binding(&self) -> &Binding<'a> {
        &self.binding
    }
}

/// Verify BOTH signatures and every expected binding field. No authority is minted.
/// Unknown/expired/revoked controllers must be rejected independently by QBS-04.
pub fn verify_pair<'a>(
    input: &'a [u8],
    expected: &Binding<'_>,
    keys: &PairedPublicKeys<'_>,
    workspace: &mut Workspace,
) -> Result<VerifiedPair<'a>, CryptoError> {
    super::binding::validate(expected)?;
    let parsed = decode_pair(input)?;
    if parsed.binding != *expected || parsed.binding.key_ids != keys.key_ids() {
        return Err(CryptoError::Unauthorized);
    }
    let len = sig_structure_into(
        ML_DSA_65,
        &parsed.binding.key_ids.ml_dsa_65,
        parsed.payload,
        &mut workspace.sig_structure,
    )?;
    signing_profile::verify_mldsa65_empty_context(
        keys.ml_dsa_65,
        &workspace.sig_structure[..len],
        parsed.ml_dsa_signature,
    )?;
    let len = sig_structure_into(
        ED25519,
        &parsed.binding.key_ids.ed25519,
        parsed.payload,
        &mut workspace.sig_structure,
    )?;
    signing_profile::verify_ed25519_raw_strict(
        keys.ed25519,
        &workspace.sig_structure[..len],
        parsed.ed_signature,
    )?;
    Ok(VerifiedPair {
        binding: parsed.binding,
    })
}
