//! Narrow reuse of the same manager while a content publication owns its borrow.
use super::super::PairedSigningRequest;
use super::ManagedObjectPublication;
use crate::crypto::network::{
    cose,
    errors::CryptoError as E,
    object_stream::{Descriptor, PUBLISHER_PURPOSE},
};
use zeroize::Zeroizing;
impl ManagedObjectPublication<'_> {
    /// Signs a structural candidate only. Recognized publisher/application rights
    /// and durable publication remain separate compositor gates.
    pub(crate) fn sign_descriptor(
        &mut self,
        descriptor: Descriptor<'_>,
        request: PairedSigningRequest<'_>,
        workspace: &mut cose::Workspace,
        out: &mut [u8],
    ) -> Result<usize, E> {
        if self.poisoned
            || self.workspace.states[..1 + self.fields.objects as usize]
                .iter()
                .any(|s| *s != 2)
        {
            return Err(E::Incomplete);
        }
        let context = self.context.ok_or(E::Unauthorized)?;
        let binding = request.binding;
        if descriptor.context().encoded() != context.encoded()
            || *binding.descriptor_digest != descriptor.id().0
            || binding.purpose != PUBLISHER_PURPOSE
            || binding.audience != self.fields.scope_token.as_slice()
        {
            return Err(E::Unauthorized);
        }
        if out.len() < cose::MAX_ENVELOPE_BYTES {
            return Err(E::Capacity);
        }
        self.attempt(|p| {
            p.check_now()?;
            let mut candidate = Zeroizing::new([0; cose::MAX_ENVELOPE_BYTES]);
            // No content-control lock is held: managed COSE takes this same fence.
            let capability = p.owner.begin_paired_signing(request, p.control, p.clock)?;
            let n = cose::sign_pair(&binding, capability, workspace, &mut candidate[..])?;
            let state = p.control.lock()?;
            p.check_locked(&state, (p.clock)())?;
            out[..n].copy_from_slice(&candidate[..n]);
            Ok(n)
        })
    }
}
