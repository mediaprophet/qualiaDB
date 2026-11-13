//! Protected-object v1 canonical wire adapters. Parsed values confer no authority.
//! Production secret operations are admitted only by the central managed owner.
mod codec;
mod context;
mod delivery;
mod descriptor;
mod pages;
mod recipient;
mod segment;
mod types;
mod wrap;
pub use context::{Context, ContextFields, encode_context};
pub use delivery::{Delivery, RecordHeader, encode_delivery};
pub use descriptor::{Descriptor, Inventory, encode_descriptor};
pub use pages::{GeometryPage, GeometryRow, RootPage, RootRow, encode_geometry, encode_roots};
pub use recipient::{RecipientFields, RecipientRecord, encode_recipient, recipient_key_ids};
pub use segment::{ObjectScope, encode_leaf_aad, encode_segment_info, leaf_nonce};
pub(crate) use segment::{derive_segment, open_leaf_raw, seal_leaf_raw};
pub use types::*;
pub use wrap::{Wrap, WrapContext, encode_wrap};
pub(crate) use wrap::{open_master_raw, seal_master_raw};
#[cfg(test)]
mod tests;
