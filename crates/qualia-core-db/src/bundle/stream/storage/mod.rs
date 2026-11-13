//! Verified integrity candidates plus bounded native live-session storage.
//! Neither integrity marks nor file barriers grant serving authority.
mod bitmap;
mod checkpoint;
mod error;
mod identity;
mod leaf;
mod wire;
pub use bitmap::{BITMAP_BYTES, CandidateBitmapPage, LEAVES_PER_PAGE, OwnedIntegrityPage, MarkStatus, ResumeHint};
pub use checkpoint::{CHECKPOINT_BYTES, UntrustedCheckpoint, decode_checkpoint, encode_checkpoint};
pub use error::StorageError;
pub use identity::{
    IDENTITY_BYTES, IdentityFields, StorageClass, StorageIdentity, decode_identity, encode_identity,
};
pub use leaf::{VerifiedLeafView, verify_leaf_view};
#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
