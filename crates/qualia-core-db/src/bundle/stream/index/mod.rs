//! Bounded structural QBDL directory admission. This does not authenticate bytes.
mod admit;
mod cbor;
mod error;
mod header;
mod limits;
mod owned;
mod probe;

pub use admit::{AdmittedIndex, EntryScratch, IndexSizing, admit_index, measure_index};
pub use cbor::{EntryIter, EntryView};
pub use error::IndexError;
pub use header::{CheckedHeader, admit_header};
pub use limits::{ExtentLimit, IndexReadLimits};
pub(crate) use owned::decode_owned_index;
pub use owned::materialize_entries;
pub use probe::HmcHeaderProbe;

#[cfg(test)]
mod tests;
