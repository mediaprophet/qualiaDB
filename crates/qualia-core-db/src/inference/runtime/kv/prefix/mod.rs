//! Reference-counted bridge between graph prefix identities and paged KV storage.
//!
//! Registry entries own one reference to every physical page. Request block tables retain an
//! additional reference while attached, so eviction never invalidates an active request.

pub mod radix;
mod store;

pub use radix::{RadixError, RadixNode, RadixPrefixIndex, RADIX_PAGE_TOKENS};
pub use store::{PrefixKvError, PrefixKvStore};

#[cfg(test)]
mod tests;
