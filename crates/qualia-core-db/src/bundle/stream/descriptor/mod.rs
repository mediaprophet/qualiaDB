//! Deterministic plaintext QASD candidates. Structural completion is not authority.
mod cbor;
mod digest;
mod inventory;
mod pages;
mod refs;
mod roots;
mod types;
pub use digest::{DigestDomain, HASH_SCRATCH_BYTES, domain_digest};
pub use inventory::InventoryVerifier;
pub use pages::{
    decode_context_page, decode_entry_page, encode_context_page_into, encode_entry_page_into,
};
pub use refs::PageRefsView;
pub use roots::{
    BodyInput, BodyView, ContextInput, ContextView, body_encoded_len, context_encoded_len,
    decode_body, decode_context, encode_body_into, encode_context_into,
};
pub use types::{
    CandidateIdentity, ContextEntry, ContextPageHeader, ContextParameters, DescriptorError,
    EntryPageHeader, EntryRecord, LEAF_BYTES, MAX_ENCODED, MAX_ENTRIES, MAX_METADATA, MAX_PAGES,
    PAGE_ROWS, PROFILE, PageRef, SourceCommit,
};
pub(crate) use types::{check_ref, pages_for};
#[cfg(test)]
mod tests;
