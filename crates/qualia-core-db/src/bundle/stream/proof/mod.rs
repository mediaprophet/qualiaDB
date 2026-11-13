//! Entry-relative SHA-384 proofs. Integrity against a supplied root is not authority.
mod builder;
mod codec;
mod hash;
mod types;
mod verify;
pub use builder::{OutboardNode, TreeBuilder};
pub use codec::{decode_proof, encode_proof};
pub use hash::{empty_root, leaf_hash};
pub use types::{
    ProofError, Representation, TreeSpec, LEAF_BYTES, MAX_LOGICAL_BYTES, MAX_SIBLINGS,
};
pub use verify::verify_leaf;

mod span;
pub use span::{verify_leaf_span, SpanLimits, VerifiedLeafSpan, MAX_SPAN_BYTES, MAX_SPAN_LEAVES};

mod path;
pub(crate) use path::{sibling_ranges, ProofSiblingRange, SiblingSide};
