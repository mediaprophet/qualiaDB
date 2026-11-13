//! Internal raw profile operations over exact COSE bytes, without key custody.
//! Public production signing remains in the managed_signing adapter.
#[cfg(test)]
use super::encode_pair;
use super::{
    encode_binding, sig_structure_into, Binding, KeyIds, PairedPublicKeys, Workspace, ED25519,
    ML_DSA_65,
};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::signing_profile::{self, RawSigningKeys};
use fips204::traits::{SerDes, Signer};

#[cfg(test)]
pub(crate) type SigningKeys<'a> = RawSigningKeys<'a>;
impl RawSigningKeys<'_> {
    pub(crate) fn key_ids(&self) -> KeyIds {
        let ml = self.ml_dsa_65.get_public_key().into_bytes();
        let ed = self.ed25519.verifying_key().to_bytes();
        PairedPublicKeys {
            ml_dsa_65: &ml,
            ed25519: &ed,
        }
        .key_ids()
    }
}

/// Internal fixtures/adapters only. No public arbitrary-key signing escape hatch.
#[cfg(test)]
pub(crate) fn sign_pair_raw(
    binding: &Binding<'_>,
    keys: &SigningKeys<'_>,
    workspace: &mut Workspace,
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    let n = sign_payload(binding, keys, workspace, out.len())?;
    encode_pair(&workspace.binding[..n], &workspace.signatures, out)
}

pub(super) fn sign_payload(
    binding: &Binding<'_>,
    keys: &RawSigningKeys<'_>,
    workspace: &mut Workspace,
    output_len: usize,
) -> Result<usize, CryptoError> {
    if binding.key_ids != keys.key_ids() {
        return Err(CryptoError::Unauthorized);
    }
    let n = encode_binding(binding, &mut workspace.binding)?;
    if output_len < super::envelope::encoded_len(n)? {
        return Err(CryptoError::Capacity);
    }
    let payload = &workspace.binding[..n];
    let len = sig_structure_into(
        ML_DSA_65,
        &binding.key_ids.ml_dsa_65,
        payload,
        &mut workspace.sig_structure,
    )?;
    signing_profile::sign_mldsa65_empty_context_into(
        keys.ml_dsa_65,
        &workspace.sig_structure[..len],
        &mut workspace.signatures.ml_dsa_65,
    )?;
    let len = sig_structure_into(
        ED25519,
        &binding.key_ids.ed25519,
        payload,
        &mut workspace.sig_structure,
    )?;
    signing_profile::sign_ed25519_raw_into(
        keys.ed25519,
        &workspace.sig_structure[..len],
        &mut workspace.signatures.ed25519,
    );
    Ok(n)
}
