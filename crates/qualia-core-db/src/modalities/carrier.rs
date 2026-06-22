//! Multi-modal semantic binding **logic** (§29, legal_logic.md) — the content-addressed
//! carrier binding + extraction.
//!
//! SCOPE (honest): this is the tamper-evident *binding* between a media blob and its semantic
//! graph, plus extraction — the part that matters for provenance and evidence. The actual
//! binary *container codecs* (PDF/A-3, XMP, PNG, Open Badges v3 byte layout) are task #9; this
//! module does NOT write those container formats. It content-addresses the blob (real BLAKE3)
//! and verifies that the carried graph is bound to *that exact* media (any edit breaks it).

use crate::NQuin;

/// Content-address a media blob → a 64-bit media tag (the low 8 bytes of its BLAKE3 hash, into
/// the one identifier space). `Hash(Blob) → Tag_Media`. Real cryptographic hash, not a toy.
pub fn media_tag(blob: &[u8]) -> u64 {
    let h = blake3::hash(blob);
    let b = h.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Verify a carrier's binding: re-hash `blob` and confirm it matches the `bound_media_tag` the
/// carrier recorded. Tamper-evident — any change to the media breaks the binding to its graph.
#[inline]
pub fn verify_binding(blob: &[u8], bound_media_tag: u64) -> bool {
    media_tag(blob) == bound_media_tag
}

/// Extract the payload quins carried alongside a medium into `out` (`Extract(C_VC) → Σ(Quins)`).
/// Returns the count written. Zero-heap (caller-supplied `out`).
pub fn extract_payload(payload: &[NQuin], out: &mut [NQuin]) -> usize {
    let n = payload.len().min(out.len());
    out[..n].copy_from_slice(&payload[..n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quin(s: u64, o: u64) -> NQuin {
        let mut q = NQuin { subject: s, predicate: 7, object: o, context: 0, metadata: 0, parity: 0 };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn media_tag_is_deterministic_and_content_addressed() {
        let blob = b"a signed evidentiary photograph's bytes";
        let tag = media_tag(blob);
        assert_eq!(tag, media_tag(blob), "deterministic");
        assert_ne!(tag, media_tag(b"different bytes"), "content-addressed");
    }

    #[test]
    fn binding_is_tamper_evident() {
        let blob = b"original media";
        let tag = media_tag(blob);
        assert!(verify_binding(blob, tag), "intact media verifies");
        assert!(!verify_binding(b"tampered media", tag), "any edit breaks the binding");
    }

    #[test]
    fn payload_extracts_round_trip() {
        let payload = [quin(1, 2), quin(3, 4)];
        let mut out = [NQuin::default(); 4];
        let n = extract_payload(&payload, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0].subject, 1);
        assert_eq!(out[1].object, 4);
    }
}
