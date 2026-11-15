use crate::bundle::stream::proof::ProofError;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    Length,
    Capacity,
    Magic,
    Version,
    Flags,
    Reserved,
    Unsupported,
    Bounds,
    Overflow,
    IdentityMismatch,
    PageMismatch,
    DigestMismatch,
    Proof(ProofError),
}
impl core::fmt::Display for StorageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "storage candidate {:?}", self)
    }
}
impl core::error::Error for StorageError {}
