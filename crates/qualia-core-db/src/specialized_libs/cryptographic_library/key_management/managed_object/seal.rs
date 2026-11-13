//! Monotonic, single-use sealing under the central publication owner.
use super::ManagedObjectPublication;
use crate::crypto::network::{
    errors::CryptoError as E,
    object_stream::{ObjectScope, seal_leaf_raw},
};
use zeroize::Zeroize;
pub struct ManagedObjectSealer<'s, 'a> {
    pub(super) publication: &'s mut ManagedObjectPublication<'a>,
    pub(super) slot: usize,
    pub(super) scope: ObjectScope,
    pub(super) next: u64,
    pub(super) finished: bool,
}
impl ManagedObjectSealer<'_, '_> {
    pub fn scope(&self) -> ObjectScope {
        self.scope
    }
    pub fn next_leaf(&self) -> u64 {
        self.next
    }
    /// Buffer becomes ciphertext only on success. Capacity/index refusal does not
    /// consume an index; cryptographic failure/cancel/unwind poisons the attempt.
    pub fn seal_next(&mut self, buffer: &mut [u8], tag: &mut [u8; 16]) -> Result<u64, E> {
        if self.publication.poisoned {
            return Err(E::Closed);
        }
        if buffer.len() != self.scope.leaf_length(self.next)? {
            return Err(E::Range);
        }
        let index = self.next;
        self.next += 1;
        struct Output<'a> {
            buffer: &'a mut [u8],
            tag: &'a mut [u8; 16],
            keep: bool,
        }
        impl Drop for Output<'_> {
            fn drop(&mut self) {
                if !self.keep {
                    self.buffer.zeroize();
                    self.tag.zeroize();
                }
            }
        }
        let mut output = Output {
            buffer,
            tag,
            keep: false,
        };
        let scope = self.scope;
        let slot = self.slot;
        let result = self.publication.attempt(|p| {
            p.check_now()?;
            seal_leaf_raw(
                &p.workspace.masters[slot],
                scope,
                index,
                output.buffer,
                output.tag,
            )?;
            let state = p.control.lock()?;
            p.check_locked(&state, (p.clock)())?;
            Ok(index)
        });
        if result.is_ok() {
            output.keep = true;
        }
        result
    }
    /// Explicit completion. Dropping before completion destroys the whole attempt.
    pub fn finish(mut self) -> Result<(), E> {
        if self.publication.poisoned || self.next != self.scope.leaf_count() {
            return Err(E::Incomplete);
        }
        self.publication.check_now()?;
        self.publication.workspace.states[self.slot] = 2;
        self.finished = true;
        Ok(())
    }
}
impl Drop for ManagedObjectSealer<'_, '_> {
    fn drop(&mut self) {
        if !self.finished {
            self.publication.abort();
        }
    }
}
