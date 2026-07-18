//! Map parsed [`WavTags`] → bounded provenance key/value pairs for NQuin emission.
//!
//! Keys use Dublin Core terms (`dcterms:*`), which the semantic layer can
//! `q_hash` into predicate slots. The result is a **fixed-size, stack array**
//! plus a count of how many leading entries are populated — no heap, no growth,
//! suitable for emitting provenance NQuins alongside decoded audio.

use super::riff_tags::WavTags;

/// Maximum number of provenance pairs [`tags_to_provenance`] can emit — one per
/// field of [`WavTags`].
pub const MAX_PROVENANCE_PAIRS: usize = 8;

/// Convert [`WavTags`] into up to [`MAX_PROVENANCE_PAIRS`] `(key, value)` pairs.
///
/// Returns the fixed array together with the number of populated leading
/// entries (`(&out[..n])` are the meaningful ones). Only tags that were present
/// contribute a pair, so the count reflects real metadata. Values are borrowed
/// from the same bytes the tags were parsed from.
pub fn tags_to_provenance<'a>(
    tags: &WavTags<'a>,
) -> ([(&'static str, &'a str); MAX_PROVENANCE_PAIRS], usize) {
    let mut out: [(&'static str, &'a str); MAX_PROVENANCE_PAIRS] = [("", ""); MAX_PROVENANCE_PAIRS];
    let mut n = 0usize;
    for (key, value) in [
        ("dcterms:title", tags.title),
        ("dcterms:creator", tags.artist),
        ("dcterms:description", tags.comment),
        ("dcterms:date", tags.created),
        ("dcterms:isPartOf", tags.product),
        ("dcterms:type", tags.genre),
        ("dcterms:provenance", tags.software),
        ("dcterms:rights", tags.copyright),
    ] {
        if let Some(v) = value {
            out[n] = (key, v);
            n += 1;
        }
    }
    (out, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::metadata::riff_tags::WavTags;

    #[test]
    fn maps_present_tags_only() {
        let tags = WavTags {
            title: Some("Rainfall"),
            artist: Some("T. Holborn"),
            created: Some("2026-07-18"),
            ..WavTags::default()
        };
        let (pairs, n) = tags_to_provenance(&tags);
        assert_eq!(n, 3);
        assert_eq!(pairs[0], ("dcterms:title", "Rainfall"));
        assert_eq!(pairs[1], ("dcterms:creator", "T. Holborn"));
        assert_eq!(pairs[2], ("dcterms:date", "2026-07-18"));
    }

    #[test]
    fn empty_tags_emit_nothing() {
        let (_pairs, n) = tags_to_provenance(&WavTags::default());
        assert_eq!(n, 0);
    }

    #[test]
    fn all_tags_fill_bound() {
        let tags = WavTags {
            title: Some("a"),
            artist: Some("b"),
            comment: Some("c"),
            created: Some("d"),
            product: Some("e"),
            genre: Some("f"),
            software: Some("g"),
            copyright: Some("h"),
        };
        let (_pairs, n) = tags_to_provenance(&tags);
        assert_eq!(n, MAX_PROVENANCE_PAIRS);
    }
}
