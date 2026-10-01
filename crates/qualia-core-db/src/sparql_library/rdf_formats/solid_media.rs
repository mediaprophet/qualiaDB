//! Solid / LDP RDF media-type negotiation for Web Civics packages.
//!
//! Community Solid Server and Solid clients exchange RDF with standard MIME
//! types (`text/turtle`, `application/ld+json`, `text/n3`, …). This module maps
//! those types onto [`RdfFormat`] and picks a response type from an `Accept`
//! header without pulling GPU/LLM dependencies.

use super::RdfFormat;

/// Solid-facing RDF representation entry (content type + Qualia format id).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolidRdfMedia {
    pub content_type: &'static str,
    pub format: RdfFormat,
    /// Preferred for Solid read/write of ordinary resources (LDP RDF Source).
    pub solid_primary: bool,
}

/// Media types Qualia webcivics / Solid adapters must support for RDF Sources.
pub const SOLID_RDF_MEDIA: &[SolidRdfMedia] = &[
    SolidRdfMedia {
        content_type: "text/turtle",
        format: RdfFormat::Turtle,
        solid_primary: true,
    },
    SolidRdfMedia {
        content_type: "application/ld+json",
        format: RdfFormat::JsonLd,
        solid_primary: true,
    },
    SolidRdfMedia {
        content_type: "text/n3",
        format: RdfFormat::N3,
        solid_primary: true,
    },
    SolidRdfMedia {
        content_type: "text/rdf+n3",
        format: RdfFormat::N3,
        solid_primary: false,
    },
    SolidRdfMedia {
        content_type: "application/n-triples",
        format: RdfFormat::NTriples,
        solid_primary: false,
    },
    SolidRdfMedia {
        content_type: "application/n-quads",
        format: RdfFormat::NQuads,
        solid_primary: false,
    },
    SolidRdfMedia {
        content_type: "application/trig",
        format: RdfFormat::TriG,
        solid_primary: false,
    },
];

/// Default Content-Type when a Solid client does not send `Accept`.
pub const SOLID_DEFAULT_CONTENT_TYPE: &str = "text/turtle";

impl RdfFormat {
    /// Resolve a Content-Type / Accept media type (parameters stripped).
    pub fn from_media_type(media: &str) -> Option<Self> {
        let base = media
            .split(';')
            .next()
            .unwrap_or(media)
            .trim()
            .to_ascii_lowercase();
        SOLID_RDF_MEDIA
            .iter()
            .find(|m| m.content_type == base)
            .map(|m| m.format)
            .or_else(|| Self::from_str(&base))
    }

    /// Canonical Solid Content-Type for this format (primary when several exist).
    pub fn solid_content_type(self) -> &'static str {
        SOLID_RDF_MEDIA
            .iter()
            .find(|m| m.format == self && m.solid_primary)
            .or_else(|| SOLID_RDF_MEDIA.iter().find(|m| m.format == self))
            .map(|m| m.content_type)
            .unwrap_or("application/octet-stream")
    }
}

/// Parse a single Accept token (`type/subtype;q=0.8`) into (media, q).
fn accept_token(token: &str) -> Option<(&str, f32)> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let mut media = token;
    let mut q = 1.0_f32;
    for part in token.split(';').map(str::trim) {
        if let Some(rest) = part.strip_prefix("q=") {
            if let Ok(v) = rest.parse::<f32>() {
                q = v.clamp(0.0, 1.0);
            }
        } else if part.contains('/') {
            media = part;
        }
    }
    Some((media, q))
}

/// Choose the best RDF Content-Type for a Solid `Accept` header.
///
/// Preference order among equal `q`: Turtle → JSON-LD → N3 → N-Triples → N-Quads → TriG.
/// `*/*` and `text/*` / `application/*` fall back to [`SOLID_DEFAULT_CONTENT_TYPE`].
pub fn negotiate_solid_accept(accept: &str) -> &'static str {
    if accept.trim().is_empty() {
        return SOLID_DEFAULT_CONTENT_TYPE;
    }
    let preference = [
        "text/turtle",
        "application/ld+json",
        "text/n3",
        "text/rdf+n3",
        "application/n-triples",
        "application/n-quads",
        "application/trig",
    ];
    let mut best: Option<(&'static str, f32, usize)> = None;
    for raw in accept.split(',') {
        let Some((media, q)) = accept_token(raw) else {
            continue;
        };
        let media_l = media.to_ascii_lowercase();
        if media_l == "*/*" || media_l == "text/*" || media_l == "application/*" {
            let cand = SOLID_DEFAULT_CONTENT_TYPE;
            let pref = 0usize;
            if best.map(|(_, bq, bp)| q > bq || (q == bq && pref < bp))
                .unwrap_or(true)
            {
                best = Some((cand, q, pref));
            }
            continue;
        }
        if let Some((idx, ct)) = preference
            .iter()
            .enumerate()
            .find(|(_, p)| **p == media_l)
            .map(|(i, p)| (i, *p))
        {
            if best
                .map(|(_, bq, bp)| q > bq || (q == bq && idx < bp))
                .unwrap_or(true)
            {
                best = Some((ct, q, idx));
            }
        }
    }
    best.map(|(ct, _, _)| ct)
        .unwrap_or(SOLID_DEFAULT_CONTENT_TYPE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_maps_solid_primaries() {
        assert_eq!(
            RdfFormat::from_media_type("text/turtle; charset=utf-8"),
            Some(RdfFormat::Turtle)
        );
        assert_eq!(
            RdfFormat::from_media_type("application/ld+json"),
            Some(RdfFormat::JsonLd)
        );
        assert_eq!(RdfFormat::from_media_type("text/n3"), Some(RdfFormat::N3));
        assert_eq!(
            RdfFormat::from_media_type("text/rdf+n3"),
            Some(RdfFormat::N3)
        );
    }

    #[test]
    fn negotiate_prefers_turtle_then_jsonld() {
        assert_eq!(
            negotiate_solid_accept("application/ld+json, text/turtle;q=0.9"),
            "application/ld+json"
        );
        assert_eq!(
            negotiate_solid_accept("text/turtle, application/ld+json;q=0.8"),
            "text/turtle"
        );
        assert_eq!(negotiate_solid_accept("*/*"), "text/turtle");
        assert_eq!(negotiate_solid_accept(""), "text/turtle");
    }

    #[test]
    fn solid_content_type_round_maps() {
        assert_eq!(RdfFormat::Turtle.solid_content_type(), "text/turtle");
        assert_eq!(
            RdfFormat::JsonLd.solid_content_type(),
            "application/ld+json"
        );
        assert_eq!(RdfFormat::N3.solid_content_type(), "text/n3");
    }
}
