//! Private metadata decryption stage. No public plaintext/master accessor.
use super::ManagedRecipientUnwrap;
use crate::crypto::network::{
    errors::CryptoError as E,
    object_stream::{ObjectScope, open_leaf_raw},
    secret_lease::SecretLease,
    types::KeyPurpose,
};
use zeroize::Zeroize;
pub(crate) struct UnwrappedMetadata<'a> {
    pub(super) master: SecretLease,
    pub(super) scope: ObjectScope,
    pub(super) operation: ManagedRecipientUnwrap<'a>,
    pub(super) next: u64,
    pub(super) poisoned: bool,
}
impl UnwrappedMetadata<'_> {
    /// Private caller scratch only; the policy compositor must separately verify
    /// inclusion and current application authority before a consumer sees bytes.
    pub(crate) fn decrypt_next(&mut self, buffer: &mut [u8], tag: &[u8; 16]) -> Result<(), E> {
        if self.poisoned {
            return Err(E::Closed);
        }
        if buffer.len() != self.scope.leaf_length(self.next)? {
            return Err(E::Range);
        }
        struct Guard<'s, 'b, 'a> {
            owner: &'s mut UnwrappedMetadata<'a>,
            buffer: &'b mut [u8],
            keep: bool,
        }
        impl Drop for Guard<'_, '_, '_> {
            fn drop(&mut self) {
                if !self.keep {
                    self.owner.poisoned = true;
                    self.owner.master.destroy();
                    self.buffer.zeroize();
                }
            }
        }
        let mut g = Guard {
            owner: self,
            buffer,
            keep: false,
        };
        let o = &mut g.owner;
        o.operation.check((o.operation.clock)())?;
        let key: &[u8; 32] = o
            .master
            .expose(KeyPurpose::ObjectContent)?
            .try_into()
            .map_err(|_| E::Range)?;
        open_leaf_raw(key, o.scope, o.next, g.buffer, tag)?;
        {
            let state = o.operation.control.lock()?;
            o.operation.check_locked(&state, (o.operation.clock)())?;
        }
        o.next += 1;
        g.keep = true;
        Ok(())
    }
}
