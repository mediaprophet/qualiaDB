//! `.10d` ProvenanceSidecar section (P1) — the provenance half of an asset,
//! bundled **physically inside the container** so context is byte-inseparable
//! from the data it attests.
//!
//! The hypermedia library records provenance *semantically* (an asset's
//! `prov:wasDerivedFrom` / `hasProvenance` edges — see [`crate::hypermedia`]).
//! That is queryable, but the source bytes / licence / verifiable-credential
//! live *outside* the sealed `.10d`, so a `.10d` copied on its own loses them.
//! This section carries them **in-envelope**: the immutable source bytes the
//! asset was derived from, its media type, its **licence** (the never-strip-
//! context field), and an optional **verifiable credential** attesting the
//! chain — all under the `.10d`'s own section-table CRC-32C.
//!
//! **Validate-before-use.** [`validate_provenance`] is the gate a consumer runs
//! before trusting the asset as citable: the carried source bytes must hash to
//! the declared `source_digest` (self-authenticating — the bytes really are the
//! attested source), and a licence must be present (context was not stripped).
//! The renderer's governance path already keys "citable" off the presence of a
//! provenance section (`render/portal/mod.rs` sets `has_attestation`); this
//! section makes that attestation real and checkable rather than merely
//! reserved.
//!
//! **Layout:** a 32-byte [`ProvenanceMiniHeader`] (magic + version + flags +
//! `source_digest` + field lengths) followed by the concatenated fields —
//! `[source_bytes][source_media_type utf8][licence utf8][vc bytes]`. The
//! mini-header is `repr(C)`, naturally aligned, no implicit padding. Two
//! encodes of the same sidecar are byte-identical; the section-table CRC-32C
//! catches a flipped bit.

use bytemuck::{bytes_of, pod_read_unaligned, Pod, Zeroable};

use crate::container_10d::crc32c::crc32c;

/// Section payload mini-header size in bytes.
pub const PROVENANCE_MINI_HEADER_SIZE: usize = 80;

/// Magic tag at the head of a provenance-section payload (`b"PRV1"`, LE).
pub const PROVENANCE_MAGIC: u32 = u32::from_le_bytes(*b"PRV1");

/// Provenance-section payload version.
pub const PROVENANCE_SECTION_VERSION: u16 = 2;

/// `flags` bit 0: a verifiable credential is present (`vc_len` must be > 0).
pub const FLAG_HAS_VC: u16 = 0x0001;

/// Upper bound per variable-length field — bounds a hostile/malformed file.
/// 16 MiB comfortably holds a source document, its media-type label, a licence
/// string, and a VC while staying well under the 42 MB Sentinel ceiling.
pub const MAX_PROVENANCE_FIELD: usize = 16 * 1024 * 1024;

/// The 80-byte ProvenanceSidecar-section mini-header. `repr(C)`, naturally
/// aligned, no implicit padding.
///
/// ```text
/// offset  size  field
/// 0       4     magic:u32          (PROVENANCE_MAGIC)
/// 4       2     version:u16        (PROVENANCE_SECTION_VERSION)
/// 6       2     flags:u16          (bit 0 = a VC is present)
/// 8       4     source_digest:u32  (CRC-32C over source_bytes — the gate anchor)
/// 12      4     reserved_u32       (must be zero) - moves before u64 to align to 16
/// 16      8     timestamp_epoch_s:u64 (Date of harvest/authoring)
/// 24      32    version_hash:[u8; 32] (Cryptographic version control hash e.g., SHA256)
/// 56      4     source_len:u32
/// 60      4     media_len:u32      (source media-type utf8 length)
/// 64      4     licence_len:u32
/// 68      4     vc_len:u32
/// 72      4     metadata_len:u32   (Schema.org / Dublin Core semantic JSON-LD length)
/// 76      4     reserved_pad:u32   (padding for 80-byte alignment)
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct ProvenanceMiniHeader {
    pub magic: u32,
    pub version: u16,
    pub flags: u16,
    pub source_digest: u32,
    pub reserved_u32: u32,
    pub timestamp_epoch_s: u64,
    pub version_hash: [u8; 32],
    pub source_len: u32,
    pub media_len: u32,
    pub licence_len: u32,
    pub vc_len: u32,
    pub metadata_len: u32,
    pub reserved_pad: u32,
}

/// An owned provenance sidecar to bundle into a `.10d` — the source bytes an
/// asset was derived from, their media type, the licence (required), and an
/// optional verifiable credential.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProvenanceSidecar {
    /// The immutable original bytes the asset was derived from.
    pub source_bytes: Vec<u8>,
    /// The source's media type (e.g. `model/gltf-binary`, `text/markdown`).
    pub source_media_type: String,
    /// The licence the source is available under (e.g. `CC-BY-4.0`, `CC0`).
    /// Required — a provenance record with no licence is a stripped context.
    pub licence: String,
    /// An optional verifiable credential (CBOR / JWT bytes) attesting the
    /// derivation chain. Empty = none.
    pub vc: Vec<u8>,
    /// Schema.org / Dublin Core semantic CBOR-LD metadata payload.
    pub semantic_metadata: Vec<u8>,
    /// Creation or harvest timestamp (UNIX epoch seconds).
    pub timestamp_epoch_s: u64,
    /// Cryptographic version control hash (e.g., SHA256 of the git commit or asset).
    pub version_hash: [u8; 32],
}

impl ProvenanceSidecar {
    pub fn new(
        source_bytes: impl Into<Vec<u8>>,
        source_media_type: impl Into<String>,
        licence: impl Into<String>,
    ) -> Self {
        Self {
            source_bytes: source_bytes.into(),
            source_media_type: source_media_type.into(),
            licence: licence.into(),
            vc: Vec::new(),
            semantic_metadata: Vec::new(),
            timestamp_epoch_s: 0,
            version_hash: [0; 32],
        }
    }

    pub fn with_vc(mut self, vc: impl Into<Vec<u8>>) -> Self {
        self.vc = vc.into();
        self
    }

    pub fn with_metadata(
        mut self,
        metadata: impl Into<Vec<u8>>,
        timestamp: u64,
        hash: [u8; 32],
    ) -> Self {
        self.semantic_metadata = metadata.into();
        self.timestamp_epoch_s = timestamp;
        self.version_hash = hash;
        self
    }

    /// The CRC-32C digest of the source bytes — the value stored in the header
    /// and re-checked by [`validate_provenance`].
    #[inline]
    pub fn source_digest(&self) -> u32 {
        crc32c(&self.source_bytes)
    }
}

/// Provenance-section read/write/validate error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceSectionError {
    /// The payload is too short for the mini-header.
    PayloadTooShort { got: usize, need: usize },
    /// The payload magic is not [`PROVENANCE_MAGIC`].
    BadMagic { got: u32 },
    /// The payload version is not [`PROVENANCE_SECTION_VERSION`].
    UnsupportedVersion { got: u16 },
    /// The mini-header `reserved_u32` is non-zero.
    NonZeroReserved,
    /// Unknown flags bit set (only bit 0 is defined in v1).
    UnknownFlags { got: u16 },
    /// A variable-length field exceeds [`MAX_PROVENANCE_FIELD`].
    FieldTooLarge {
        field: &'static str,
        got: usize,
        max: usize,
    },
    /// The payload is too short for the declared field lengths.
    PayloadTruncated { expected: usize, got: usize },
    /// The output buffer is too small.
    OutputBufferTooSmall { needed: usize, have: usize },
    /// The `FLAG_HAS_VC` bit and `vc_len` disagree.
    VcFlagInconsistent { has_vc: bool, vc_len: u32 },
    /// A field declared as utf8 (media type / licence) is not valid utf8.
    NonUtf8 { field: &'static str },
    /// Validate gate: the carried source bytes do not hash to the declared
    /// `source_digest` — the sidecar's source is not authentic to its claim.
    SourceDigestMismatch { expected: u32, got: u32 },
    /// Validate gate: no licence is present (context was stripped).
    MissingLicence,
}

impl std::fmt::Display for ProvenanceSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => {
                write!(f, "10d PRV payload too short: got {got}, need {need}")
            }
            Self::BadMagic { got } => write!(
                f,
                "10d PRV bad magic {got:#010x} (expected {PROVENANCE_MAGIC:#010x})"
            ),
            Self::UnsupportedVersion { got } => write!(f, "10d PRV unsupported version {got}"),
            Self::NonZeroReserved => write!(f, "10d PRV non-zero reserved_u32"),
            Self::UnknownFlags { got } => write!(
                f,
                "10d PRV unknown flags bits {got:#06x} (only bit 0 defined in v1)"
            ),
            Self::FieldTooLarge { field, got, max } => {
                write!(f, "10d PRV field {field:?} too large: {got} > {max}")
            }
            Self::PayloadTruncated { expected, got } => write!(
                f,
                "10d PRV payload truncated: expected {expected}, got {got}"
            ),
            Self::OutputBufferTooSmall { needed, have } => write!(
                f,
                "10d PRV output buffer too small: need {needed}, have {have}"
            ),
            Self::VcFlagInconsistent { has_vc, vc_len } => write!(
                f,
                "10d PRV vc flag inconsistent: has_vc={has_vc}, vc_len={vc_len}"
            ),
            Self::NonUtf8 { field } => write!(f, "10d PRV field {field:?} is not valid utf8"),
            Self::SourceDigestMismatch { expected, got } => write!(
                f,
                "10d PRV source-digest mismatch: expected {expected:#010x}, got {got:#010x}"
            ),
            Self::MissingLicence => write!(f, "10d PRV missing licence (context stripped)"),
        }
    }
}

impl std::error::Error for ProvenanceSectionError {}

/// Encoded payload length in bytes for a sidecar (mini-header + fields).
#[inline]
pub fn encoded_len(s: &ProvenanceSidecar) -> usize {
    PROVENANCE_MINI_HEADER_SIZE
        + s.source_bytes.len()
        + s.source_media_type.len()
        + s.licence.len()
        + s.vc.len()
        + s.semantic_metadata.len()
}

/// Encode a provenance sidecar into a caller-supplied buffer. Returns the
/// number of bytes written. Deterministic (two encodes are byte-identical).
pub fn encode_provenance_section(
    s: &ProvenanceSidecar,
    out: &mut [u8],
) -> Result<usize, ProvenanceSectionError> {
    let check = |field, len: usize| -> Result<(), ProvenanceSectionError> {
        if len > MAX_PROVENANCE_FIELD {
            Err(ProvenanceSectionError::FieldTooLarge {
                field,
                got: len,
                max: MAX_PROVENANCE_FIELD,
            })
        } else {
            Ok(())
        }
    };
    check("source", s.source_bytes.len())?;
    check("media_type", s.source_media_type.len())?;
    check("licence", s.licence.len())?;
    check("vc", s.vc.len())?;
    check("semantic_metadata", s.semantic_metadata.len())?;

    let total = encoded_len(s);
    if out.len() < total {
        return Err(ProvenanceSectionError::OutputBufferTooSmall {
            needed: total,
            have: out.len(),
        });
    }

    let flags = if s.vc.is_empty() { 0 } else { FLAG_HAS_VC };
    let header = ProvenanceMiniHeader {
        magic: PROVENANCE_MAGIC,
        version: PROVENANCE_SECTION_VERSION,
        flags,
        source_digest: s.source_digest(),
        reserved_u32: 0,
        timestamp_epoch_s: s.timestamp_epoch_s,
        version_hash: s.version_hash,
        source_len: s.source_bytes.len() as u32,
        media_len: s.source_media_type.len() as u32,
        licence_len: s.licence.len() as u32,
        vc_len: s.vc.len() as u32,
        metadata_len: s.semantic_metadata.len() as u32,
        reserved_pad: 0,
    };

    let mut cursor = 0;
    out[cursor..cursor + PROVENANCE_MINI_HEADER_SIZE].copy_from_slice(bytes_of(&header));
    cursor += PROVENANCE_MINI_HEADER_SIZE;
    out[cursor..cursor + s.source_bytes.len()].copy_from_slice(&s.source_bytes);
    cursor += s.source_bytes.len();
    out[cursor..cursor + s.source_media_type.len()].copy_from_slice(s.source_media_type.as_bytes());
    cursor += s.source_media_type.len();
    out[cursor..cursor + s.licence.len()].copy_from_slice(s.licence.as_bytes());
    cursor += s.licence.len();
    out[cursor..cursor + s.vc.len()].copy_from_slice(&s.vc);
    cursor += s.vc.len();
    out[cursor..cursor + s.semantic_metadata.len()].copy_from_slice(&s.semantic_metadata);
    cursor += s.semantic_metadata.len();

    debug_assert_eq!(cursor, total);
    Ok(total)
}

/// A zero-copy read view over a decoded provenance-section payload.
#[derive(Debug, Clone, Copy)]
pub struct ProvenanceSidecarView<'a> {
    header: ProvenanceMiniHeader,
    source_bytes: &'a [u8],
    source_media_type: &'a str,
    licence: &'a str,
    vc: &'a [u8],
    semantic_metadata: &'a [u8],
}

impl<'a> ProvenanceSidecarView<'a> {
    /// The immutable source bytes the asset was derived from.
    #[inline]
    pub fn source_bytes(&self) -> &'a [u8] {
        self.source_bytes
    }
    /// The source's media type.
    #[inline]
    pub fn source_media_type(&self) -> &'a str {
        self.source_media_type
    }
    /// The licence the source is available under.
    #[inline]
    pub fn licence(&self) -> &'a str {
        self.licence
    }
    /// The attached verifiable credential, if any.
    #[inline]
    pub fn vc(&self) -> Option<&'a [u8]> {
        if self.vc.is_empty() {
            None
        } else {
            Some(self.vc)
        }
    }
    /// Schema.org / Dublin Core semantic CBOR-LD metadata payload.
    #[inline]
    pub fn semantic_metadata(&self) -> &'a [u8] {
        self.semantic_metadata
    }
    /// Harvest or creation timestamp.
    #[inline]
    pub fn timestamp_epoch_s(&self) -> u64 {
        self.header.timestamp_epoch_s
    }
    /// Cryptographic version control hash.
    #[inline]
    pub fn version_hash(&self) -> &[u8; 32] {
        &self.header.version_hash
    }
    /// The declared source digest (CRC-32C over the source bytes).
    #[inline]
    pub fn source_digest(&self) -> u32 {
        self.header.source_digest
    }
}

/// Parse the mini-header and slice the fields out of a provenance-section
/// payload (zero-copy). Validates magic, version, reserved, flag consistency,
/// field bounds, and utf8 — but does **not** run the trust gate; call
/// [`validate_provenance`] before using the sidecar as an attestation.
pub fn decode_provenance_section(
    payload: &[u8],
) -> Result<ProvenanceSidecarView<'_>, ProvenanceSectionError> {
    if payload.len() < PROVENANCE_MINI_HEADER_SIZE {
        return Err(ProvenanceSectionError::PayloadTooShort {
            got: payload.len(),
            need: PROVENANCE_MINI_HEADER_SIZE,
        });
    }
    // `pod_read_unaligned` (not `from_bytes`): a section payload — or a raw
    // test buffer — is not guaranteed aligned to the header's 4-byte alignment,
    // and `from_bytes` panics on misalignment. This copies the 32 header bytes
    // into an aligned value.
    let header: ProvenanceMiniHeader = pod_read_unaligned(&payload[..PROVENANCE_MINI_HEADER_SIZE]);
    if header.magic != PROVENANCE_MAGIC {
        return Err(ProvenanceSectionError::BadMagic { got: header.magic });
    }
    if header.version != PROVENANCE_SECTION_VERSION {
        return Err(ProvenanceSectionError::UnsupportedVersion {
            got: header.version,
        });
    }
    if header.reserved_u32 != 0 {
        return Err(ProvenanceSectionError::NonZeroReserved);
    }
    if header.flags & !FLAG_HAS_VC != 0 {
        return Err(ProvenanceSectionError::UnknownFlags { got: header.flags });
    }
    let has_vc = header.flags & FLAG_HAS_VC != 0;
    if has_vc != (header.vc_len > 0) {
        return Err(ProvenanceSectionError::VcFlagInconsistent {
            has_vc,
            vc_len: header.vc_len,
        });
    }

    let source_len = header.source_len as usize;
    let media_len = header.media_len as usize;
    let licence_len = header.licence_len as usize;
    let vc_len = header.vc_len as usize;
    let metadata_len = header.metadata_len as usize;
    for (field, len) in [
        ("source", source_len),
        ("media_type", media_len),
        ("licence", licence_len),
        ("vc", vc_len),
        ("semantic_metadata", metadata_len),
    ] {
        if len > MAX_PROVENANCE_FIELD {
            return Err(ProvenanceSectionError::FieldTooLarge {
                field,
                got: len,
                max: MAX_PROVENANCE_FIELD,
            });
        }
    }

    let expected =
        PROVENANCE_MINI_HEADER_SIZE + source_len + media_len + licence_len + vc_len + metadata_len;
    if payload.len() < expected {
        return Err(ProvenanceSectionError::PayloadTruncated {
            expected,
            got: payload.len(),
        });
    }

    let mut cursor = PROVENANCE_MINI_HEADER_SIZE;
    let source_bytes = &payload[cursor..cursor + source_len];
    cursor += source_len;
    let media_raw = &payload[cursor..cursor + media_len];
    cursor += media_len;
    let licence_raw = &payload[cursor..cursor + licence_len];
    cursor += licence_len;
    let vc = &payload[cursor..cursor + vc_len];
    cursor += vc_len;
    let metadata_raw = &payload[cursor..cursor + metadata_len];

    let source_media_type =
        std::str::from_utf8(media_raw).map_err(|_| ProvenanceSectionError::NonUtf8 {
            field: "media_type",
        })?;
    let licence = std::str::from_utf8(licence_raw)
        .map_err(|_| ProvenanceSectionError::NonUtf8 { field: "licence" })?;
    let semantic_metadata = metadata_raw;

    Ok(ProvenanceSidecarView {
        header,
        source_bytes,
        source_media_type,
        licence,
        vc,
        semantic_metadata,
    })
}

/// The **validate-before-use gate**: return `Ok` only if the sidecar can be
/// trusted as the asset's provenance. Two independent checks:
///
/// 1. **Self-authenticating source** — the carried `source_bytes` hash to the
///    declared `source_digest` (the bytes really are the attested source; a
///    swapped source is rejected).
/// 2. **Context present** — a non-empty licence (a provenance record with no
///    licence is a stripped context, not an attestation).
///
/// A consumer runs this before treating an asset as citable/attested (mirroring
/// the renderer's `has_attestation` governance gate).
pub fn validate_provenance(view: &ProvenanceSidecarView<'_>) -> Result<(), ProvenanceSectionError> {
    let got = crc32c(view.source_bytes);
    if got != view.header.source_digest {
        return Err(ProvenanceSectionError::SourceDigestMismatch {
            expected: view.header.source_digest,
            got,
        });
    }
    if view.licence.trim().is_empty() {
        return Err(ProvenanceSectionError::MissingLicence);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ProvenanceSidecar {
        ProvenanceSidecar::new(
            b"<the original source GLB bytes>".to_vec(),
            "model/gltf-binary",
            "CC-BY-4.0",
        )
        .with_vc(b"{\"vc\":\"attested\"}".to_vec())
        .with_metadata(
            b"\xA2\x68@context\x78\x1Dhttps://schema.org/\x65@type\x67Dataset".to_vec(),
            1690000000,
            [0xAA; 32],
        )
    }

    #[test]
    fn round_trips_and_validates() {
        let s = sample();
        let mut buf = vec![0u8; encoded_len(&s)];
        let n = encode_provenance_section(&s, &mut buf).unwrap();
        assert_eq!(n, buf.len());

        let view = decode_provenance_section(&buf).unwrap();
        assert_eq!(view.source_bytes(), s.source_bytes.as_slice());
        assert_eq!(view.source_media_type(), "model/gltf-binary");
        assert_eq!(view.licence(), "CC-BY-4.0");
        assert_eq!(view.vc(), Some(s.vc.as_slice()));
        assert_eq!(
            view.semantic_metadata(),
            b"\xA2\x68@context\x78\x1Dhttps://schema.org/\x65@type\x67Dataset"
        );
        assert_eq!(view.timestamp_epoch_s(), 1690000000);
        assert_eq!(view.version_hash(), &[0xAA; 32]);
        assert_eq!(view.source_digest(), crc32c(&s.source_bytes));
        // The gate passes for an authentic, licensed sidecar.
        validate_provenance(&view).unwrap();
    }

    #[test]
    fn deterministic_encoding() {
        let s = sample();
        let mut a = vec![0u8; encoded_len(&s)];
        let mut b = vec![0u8; encoded_len(&s)];
        encode_provenance_section(&s, &mut a).unwrap();
        encode_provenance_section(&s, &mut b).unwrap();
        assert_eq!(a, b, "two encodes of the same sidecar are byte-identical");
    }

    #[test]
    fn no_vc_clears_the_flag() {
        let s = ProvenanceSidecar::new(b"src".to_vec(), "text/plain", "CC0");
        let mut buf = vec![0u8; encoded_len(&s)];
        encode_provenance_section(&s, &mut buf).unwrap();
        let view = decode_provenance_section(&buf).unwrap();
        assert_eq!(view.vc(), None);
        validate_provenance(&view).unwrap();
    }

    #[test]
    fn tampered_source_bytes_fail_the_gate() {
        let s = sample();
        let mut buf = vec![0u8; encoded_len(&s)];
        encode_provenance_section(&s, &mut buf).unwrap();
        // Flip a byte in the source-bytes region (after the 32-byte header).
        buf[PROVENANCE_MINI_HEADER_SIZE] ^= 0xFF;
        let view = decode_provenance_section(&buf).unwrap();
        assert!(matches!(
            validate_provenance(&view),
            Err(ProvenanceSectionError::SourceDigestMismatch { .. })
        ));
    }

    #[test]
    fn a_stripped_licence_fails_the_gate() {
        let s = ProvenanceSidecar::new(b"src".to_vec(), "text/plain", "");
        let mut buf = vec![0u8; encoded_len(&s)];
        encode_provenance_section(&s, &mut buf).unwrap();
        let view = decode_provenance_section(&buf).unwrap();
        assert_eq!(
            validate_provenance(&view),
            Err(ProvenanceSectionError::MissingLicence)
        );
    }

    #[test]
    fn round_trips_through_the_real_container_section_table() {
        use crate::container_10d::header::Container10dHeader;
        use crate::container_10d::section::{
            encode_container, parse_section_table, AlignmentTier, SectionInput, SectionType,
        };

        // A provenance sidecar bundled alongside a (stand-in) mesh section.
        let sidecar = sample();
        let mut prov_payload = vec![0u8; encoded_len(&sidecar)];
        encode_provenance_section(&sidecar, &mut prov_payload).unwrap();
        let mesh_payload = [0xAAu8; 64];

        let inputs = [
            SectionInput {
                section_type: SectionType::QuantizedMesh,
                alignment_tier: AlignmentTier::Word,
                stride: 0,
                element_count: 0,
                payload: &mesh_payload,
            },
            SectionInput {
                // type 7 — accepted by the encoder now that it is implemented.
                section_type: SectionType::ProvenanceSidecar,
                alignment_tier: AlignmentTier::Word,
                stride: 0,
                element_count: 0,
                payload: &prov_payload,
            },
        ];

        let h = Container10dHeader::proposed();
        let mut out = vec![0u8; 4096];
        let n = encode_container(&h, &inputs, &mut out).expect("encode container w/ provenance");
        let parsed = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed).expect("table parse (CRC-checked)");

        // The provenance section is present and readable straight out of the .10d.
        let prov = descs
            .iter()
            .find(|d| d.section_type == SectionType::ProvenanceSidecar as u8)
            .expect("provenance section in table");
        let payload = &out[prov.byte_offset as usize..][..prov.byte_length as usize];
        let view = decode_provenance_section(payload).expect("decode from container");
        validate_provenance(&view).expect("validate-before-use passes for the bundled sidecar");
        assert_eq!(view.licence(), "CC-BY-4.0");
        assert_eq!(view.source_bytes(), sidecar.source_bytes.as_slice());
    }

    #[test]
    fn bad_magic_and_short_payload_are_rejected() {
        assert!(matches!(
            decode_provenance_section(&[0u8; 8]),
            Err(ProvenanceSectionError::PayloadTooShort { .. })
        ));
        let mut buf = [0u8; PROVENANCE_MINI_HEADER_SIZE];
        // zeroed header ⇒ magic 0 ⇒ BadMagic
        assert!(matches!(
            decode_provenance_section(&buf),
            Err(ProvenanceSectionError::BadMagic { .. })
        ));
        // set a good magic but truncated declared source_len
        let bad = ProvenanceMiniHeader {
            magic: PROVENANCE_MAGIC,
            version: PROVENANCE_SECTION_VERSION,
            flags: 0,
            source_digest: 0,
            reserved_u32: 0,
            timestamp_epoch_s: 0,
            version_hash: [0; 32],
            source_len: 100,
            media_len: 0,
            licence_len: 0,
            vc_len: 0,
            metadata_len: 0,
            reserved_pad: 0,
        };
        buf.copy_from_slice(bytes_of(&bad));
        assert!(matches!(
            decode_provenance_section(&buf),
            Err(ProvenanceSectionError::PayloadTruncated { .. })
        ));
    }
}
