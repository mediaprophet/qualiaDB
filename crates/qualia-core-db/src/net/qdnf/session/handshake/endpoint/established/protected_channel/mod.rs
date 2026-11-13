//! Shared QBS09/QBS10 persistent packet owner; record assembly stays above it.
mod owner;
mod receive;
mod runtime_bridge;
mod scope;
mod send;
use crate::net::qdnf::errors::QdnfError as E;
pub use owner::{ChannelBuildError, ProtectedChannel};
pub(crate) use scope::{ChannelIo, FrameIo};
pub(crate) const RECORD_AAD_BYTES: usize = 88;
pub(crate) const CHUNK_BYTES: usize = 1024;
pub(crate) const FRAME_BYTES: usize = 80 + RECORD_AAD_BYTES + 24 + CHUNK_BYTES;
#[cfg(test)]
mod tests;
