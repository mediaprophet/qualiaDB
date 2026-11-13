//! Managed-only production signing boundary; verification has no custody imports.
use super::{encode_pair, raw_signing::sign_payload, Binding, Workspace};
use crate::crypto::network::errors::CryptoError;
use crate::specialized_libs::cryptographic_library::ManagedPairedSigning;

/// Public signing requires the existing library owner's single-use capability.
/// Cancellation/generation updates are serialized against final publication.
/// Scratch signatures are erased on every error; output is untouched unless
/// publication succeeds. The host must provide a trusted clock at admission.
pub fn sign_pair(
    binding: &Binding<'_>,
    capability: ManagedPairedSigning<'_>,
    workspace: &mut Workspace,
    out: &mut [u8],
) -> Result<usize, CryptoError> {
    let result = (|| {
        capability.check_binding(binding)?;
        let n = sign_payload(binding, &capability.keys(), workspace, out.len())?;
        capability.publish(|| encode_pair(&workspace.binding[..n], &workspace.signatures, out))
    })();
    if result.is_err() {
        workspace.signatures.ml_dsa_65.fill(0);
        workspace.signatures.ed25519.fill(0);
    }
    result
}
