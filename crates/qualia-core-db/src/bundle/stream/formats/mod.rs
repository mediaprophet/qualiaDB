//! Selective inner-format checks over entry-root-authenticated byte ranges.
//! These are integrity views, not current disclosure grants or whole-file receipts.
mod emf;
mod range;
pub use emf::{EmfChunkView, EmfDirectoryView};
pub use range::{AuthenticatedRange, FormatError, FormatLimits, PayloadInterval};
mod m64;
#[cfg(test)]
mod tests;
pub use m64::{M64ChunkView, M64DirectoryView};
mod ten_d;
pub use ten_d::{TenDDirectoryView, TenDSectionView};
#[cfg(test)]
mod ten_d_tests;

#[cfg(test)]
mod m64_tests;

#[cfg(test)]
mod brick_tests;

mod geometry;
#[cfg(test)]
mod geometry_tests;

mod q42_ranges;
pub use q42_ranges::Q42AuthenticatedRangeSource;
#[cfg(test)]
mod q42_range_tests;

mod emf_v3;
pub use emf_v3::EmfV3DirectoryView;
#[cfg(test)] mod emf_v3_tests;
