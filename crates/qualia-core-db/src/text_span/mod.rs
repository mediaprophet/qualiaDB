//! Byte-span annotations for document → Quin provenance (P6).
//!
//! UTF-8 offsets are canonical. Content hash is FNV-1a 60-bit (lexicon).

use crate::lexicon::generate_60bit_token;
use crate::NQuin;

/// Inclusive-exclusive UTF-8 byte range into a source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start_utf8: u32,
    pub end_utf8: u32,
    pub content_hash: u64,
}

impl TextSpan {
    pub fn from_source(source: &str, start_utf8: u32, end_utf8: u32) -> Option<Self> {
        let start = start_utf8 as usize;
        let end = end_utf8 as usize;
        if start > end || end > source.len() {
            return None;
        }
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return None;
        }
        Some(Self {
            start_utf8,
            end_utf8,
            content_hash: generate_60bit_token(source[start..end].as_bytes()),
        })
    }

    pub fn slice<'a>(&self, source: &'a str) -> Option<&'a str> {
        let start = self.start_utf8 as usize;
        let end = self.end_utf8 as usize;
        if start <= end && end <= source.len() {
            Some(&source[start..end])
        } else {
            None
        }
    }
}

/// Predicate for "this Quin is an annotation of a source span".
pub const SPAN_ANNOTATES: &str = "https://qualiadb.org/schema/annotatesSpan";

/// Build a sealed annotation Quin: subject = term, predicate = annotatesSpan,
/// object packs start<<32|end (fits documents under 4 GiB), context = content hash.
pub fn annotation_quin(term_iri: &str, span: TextSpan, source_hash: u64) -> NQuin {
    let subject = generate_60bit_token(term_iri.as_bytes());
    let predicate = generate_60bit_token(SPAN_ANNOTATES.as_bytes());
    let object = ((span.start_utf8 as u64) << 32) | (span.end_utf8 as u64);
    let context = span.content_hash ^ source_hash;
    let metadata = 0;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_rejects_bad_range() {
        assert!(TextSpan::from_source("hi", 0, 5).is_none());
        assert!(TextSpan::from_source("hi", 2, 1).is_none());
    }

    #[test]
    fn annotation_quin_has_valid_parity() {
        let src = "North Spring is the reference site.";
        let span = TextSpan::from_source(src, 0, 12).unwrap();
        assert_eq!(span.slice(src), Some("North Spring"));
        let q = annotation_quin("https://qualiadb.org/clinic/NorthSpring", span, 1);
        assert!(q.verify_ecc_parity());
        assert_ne!(q.parity, 0);
    }
}
