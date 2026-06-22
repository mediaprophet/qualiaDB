//! Shared quin sink trait for streaming RDF parsers.

use crate::NQuin;
use std::io;

/// Accepts quins from format parsers without mandating heap growth.
pub trait QuinSink {
    fn push(&mut self, q: NQuin) -> io::Result<()>;

    /// Record a term's `hash → lexical string` so the value can be recovered later
    /// (literal text, IRIs). Default is a no-op — sinks that do not build a lexicon
    /// (or callers that do not need recovery) pay nothing. The streaming ingest sink
    /// implements this to populate the `.q42` front-of-file Q42LEX section.
    fn push_lex(&mut self, _hash: u64, _term: &str) {}
}

impl QuinSink for crate::external_sort::ExternalSorter {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        crate::external_sort::ExternalSorter::push(self, q)
    }

    fn push_lex(&mut self, hash: u64, term: &str) {
        crate::external_sort::ExternalSorter::push_lex(self, hash, term)
    }
}