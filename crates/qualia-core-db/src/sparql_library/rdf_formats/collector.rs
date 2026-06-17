//! Fixed-capacity quin collector for RDF parse hot paths.

use crate::NQuin;
use crate::sparql_library::quin_sink::QuinSink;
use std::io;

pub const MAX_RDF_QUINS: usize = 8_192;
pub const QUIN_BUFFER_FULL_MSG: &str = "quin buffer full";

/// Stack-backed quin buffer — no heap growth during parse callbacks.
pub struct QuinCollector {
    pub buf: [NQuin; MAX_RDF_QUINS],
    pub count: usize,
    pub truncated: bool,
}

impl QuinCollector {
    pub fn new() -> Self {
        Self {
            buf: [NQuin::default(); MAX_RDF_QUINS],
            count: 0,
            truncated: false,
        }
    }

    #[inline]
    pub fn push(&mut self, mut q: NQuin) {
        if q.parity == 0 {
            q.parity = NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata);
        }
        if self.count < MAX_RDF_QUINS {
            self.buf[self.count] = q;
            self.count += 1;
        } else {
            self.truncated = true;
        }
    }

    pub fn as_slice(&self) -> &[NQuin] {
        &self.buf[..self.count]
    }
}

impl QuinSink for QuinCollector {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        if self.count >= MAX_RDF_QUINS {
            self.truncated = true;
            return Err(io::Error::new(io::ErrorKind::Other, QUIN_BUFFER_FULL_MSG));
        }
        QuinCollector::push(self, q);
        Ok(())
    }
}