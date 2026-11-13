//! Caller-owned private bounded master storage. No public key material access.
use crate::crypto::network::object_stream::MAX_OBJECTS;
use zeroize::Zeroize;
pub struct PublicationWorkspace {
    pub(super) masters: [[u8; 32]; MAX_OBJECTS + 1],
    pub(super) states: [u8; MAX_OBJECTS + 1],
}
impl PublicationWorkspace {
    pub fn new() -> Self {
        Self {
            masters: [[0; 32]; MAX_OBJECTS + 1],
            states: [0; MAX_OBJECTS + 1],
        }
    }
    pub const fn required_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
    pub(super) fn clear(&mut self) {
        self.masters.zeroize();
        self.states.fill(0);
    }
}
impl Default for PublicationWorkspace {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for PublicationWorkspace {
    fn drop(&mut self) {
        self.clear();
    }
}
