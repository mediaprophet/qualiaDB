use super::*;

fn minimal() -> [u8; 144] {
    let mut b = [0u8; 144];
    b[..12].copy_from_slice(&KTX2_IDENTIFIER);
    put32(&mut b, 12, 37); // VK_FORMAT_R8G8B8A8_UNORM
    put32(&mut b, 16, 1);
    put32(&mut b, 20, 1);
    put32(&mut b, 36, 1);
    put32(&mut b, 40, 1);
    put32(&mut b, 48, 104); // DFD
    put32(&mut b, 52, 28);
    put32(&mut b, 104, 28); // DFD total size
    put32(&mut b, 112, (24 << 16) | 2); // DFD version + block size
    put64(&mut b, 80, 136); // level payload (after 4 zero padding bytes)
    put64(&mut b, 88, 4);
    put64(&mut b, 96, 4);
    b[136..140].copy_from_slice(&[1, 2, 3, 4]);
    b
}

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn parses_borrowed_level_and_metadata() {
    let bytes = minimal();
    let doc = Ktx2Document::parse(&bytes).unwrap();
    assert_eq!(doc.pixel_width, 1);
    assert_eq!(doc.layer_count(), 1);
    assert_eq!(doc.level_count(), 1);
    assert_eq!(doc.data_format_descriptor().len(), 28);
    assert_eq!(doc.level(0).unwrap().bytes, &[1, 2, 3, 4]);
    assert!(doc.level(1).is_none());
}

#[test]
fn rejects_truncated_or_wrong_identifier() {
    assert_eq!(
        Ktx2Document::parse(&[0; 79]),
        Err(Ktx2Error::TruncatedHeader)
    );
    let mut bytes = minimal();
    bytes[0] = 0;
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidIdentifier)
    );
}

#[test]
fn rejects_section_out_of_bounds_and_overlap() {
    let mut bytes = minimal();
    put32(&mut bytes, 48, 120);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::SectionOutOfBounds)
    );
    let mut bytes = minimal();
    put64(&mut bytes, 80, 128);
    put64(&mut bytes, 88, 4);
    assert_eq!(Ktx2Document::parse(&bytes), Err(Ktx2Error::SectionOverlap));
}

#[test]
fn rejects_level_index_overflow_and_invalid_level_ranges() {
    let mut bytes = minimal();
    put32(&mut bytes, 40, u32::MAX);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::UnsupportedLevelCount)
    );
    let mut bytes = minimal();
    put32(&mut bytes, 20, 2);
    put32(&mut bytes, 40, 2);
    assert_eq!(
        Ktx2Document::parse(&bytes[..120]),
        Err(Ktx2Error::TruncatedLevelIndex)
    );
    let mut bytes = minimal();
    put64(&mut bytes, 80, 138);
    put64(&mut bytes, 88, 8);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidLevelRange)
    );
}

#[test]
fn rejects_metadata_out_of_order() {
    let mut bytes = minimal();
    put32(&mut bytes, 56, 104);
    put32(&mut bytes, 60, 4);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidSectionOrder)
    );
}

#[test]
fn rejects_malformed_dfd_envelope_and_block_framing() {
    let mut bytes = minimal();
    put32(&mut bytes, 104, 4);
    assert_eq!(Ktx2Document::parse(&bytes), Err(Ktx2Error::InvalidDfd));

    let mut bytes = minimal();
    put32(&mut bytes, 112, (20 << 16) | 2);
    assert_eq!(Ktx2Document::parse(&bytes), Err(Ktx2Error::InvalidDfd));
}

#[test]
fn enforces_standard_supercompression_metadata_contracts() {
    let mut bytes = minimal();
    put32(&mut bytes, 44, 1); // BasisLZ requires vkFormat=UNDEFINED and SGD.
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidSupercompressionParameters)
    );

    let mut bytes = [0u8; 152];
    bytes[..144].copy_from_slice(&minimal());
    put32(&mut bytes, 12, 0);
    put32(&mut bytes, 44, 1);
    put64(&mut bytes, 64, 136);
    put64(&mut bytes, 72, 8);
    put64(&mut bytes, 80, 144);
    put64(&mut bytes, 96, 0);
    assert!(Ktx2Document::parse(&bytes).is_ok());
    put64(&mut bytes, 96, 4);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidSupercompressionParameters)
    );

    let mut bytes = minimal();
    put32(&mut bytes, 44, 2); // Zstandard does not use SGD.
    put64(&mut bytes, 64, 136);
    put64(&mut bytes, 72, 8);
    bytes[132..136].fill(0);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidSupercompressionParameters)
    );
}

#[test]
fn requires_uncompressed_level_alignment_and_zero_padding() {
    let mut bytes = minimal();
    put64(&mut bytes, 80, 137);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidLevelAlignment)
    );

    let mut bytes = minimal();
    bytes[132] = 1;
    assert_eq!(Ktx2Document::parse(&bytes), Err(Ktx2Error::InvalidPadding));
}

#[test]
fn requires_dfd_then_kvd_then_aligned_sgd() {
    let mut bytes = minimal();
    put32(&mut bytes, 56, 136);
    put32(&mut bytes, 60, 4);
    assert_eq!(
        Ktx2Document::parse(&bytes),
        Err(Ktx2Error::InvalidSectionOrder)
    );
}

#[test]
fn exposes_supercompression_and_color_capabilities_without_claiming_decode() {
    let mut bytes = minimal();
    put32(&mut bytes, 12, 43); // RGBA8 sRGB
    put32(&mut bytes, 44, 2); // Zstd
    put64(&mut bytes, 80, 132);
    assert_eq!(
        Ktx2Document::parse(&bytes).unwrap().supercompression(),
        Ktx2Supercompression::Zstd
    );
    let document = Ktx2Document::parse(&bytes).unwrap();
    assert_eq!(document.color_space(), Ktx2ColorSpace::Srgb);
    assert_eq!(document.transcode_source(), None);

    let mut compressed = minimal();
    put32(&mut compressed, 12, 131); // BC1 UNORM
    let document = Ktx2Document::parse(&compressed).unwrap();
    assert_eq!(document.color_space(), Ktx2ColorSpace::Linear);
    assert_eq!(
        document.transcode_source(),
        Some(Ktx2TranscodeSource::CompressedFormat(131))
    );
}

#[test]
fn basis_lz_is_structurally_valid_but_requires_a_transcoder() {
    let mut bytes = [0u8; 152];
    bytes[..144].copy_from_slice(&minimal());
    put32(&mut bytes, 12, 0);
    put32(&mut bytes, 44, 1);
    put64(&mut bytes, 64, 136);
    put64(&mut bytes, 72, 8);
    put64(&mut bytes, 80, 144);
    put64(&mut bytes, 88, 4);
    put64(&mut bytes, 96, 0);
    let document = Ktx2Document::parse(&bytes).unwrap();
    assert_eq!(document.supercompression(), Ktx2Supercompression::BasisLz);
    assert_eq!(document.color_space(), Ktx2ColorSpace::Unknown);
    assert_eq!(document.transcode_source(), Some(Ktx2TranscodeSource::BasisLz));
}
