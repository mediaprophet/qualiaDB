//! Shared quin sink trait for streaming RDF parsers.

use crate::NQuin;
use std::io;

/// Accepts quins from format parsers without mandating heap growth.
pub trait QuinSink {
    fn push(&mut self, q: NQuin) -> io::Result<()>;
}

impl QuinSink for crate::external_sort::ExternalSorter {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        crate::external_sort::ExternalSorter::push(self, q)
    }
}