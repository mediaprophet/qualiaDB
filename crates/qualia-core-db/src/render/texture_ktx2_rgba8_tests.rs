use super::*;
use crate::render::texture_ktx2::{Ktx2Document, KTX2_IDENTIFIER};

fn fixture(vk_format: u32, width: u32, height: u32, payload: &[u8]) -> Vec<u8> {
    let payload_offset = 136usize;
    let mut bytes = vec![0u8; payload_offset + payload.len()];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    put32(&mut bytes, 12, vk_format);
    put32(&mut bytes, 16, 1);
    put32(&mut bytes, 20, width);
    put32(&mut bytes, 24, height);
    put32(&mut bytes, 36, 1);
    put32(&mut bytes, 40, 1);
    put32(&mut bytes, 48, 104);
    put32(&mut bytes, 52, 28);
    put32(&mut bytes, 104, 28);
    put32(&mut bytes, 112, (24 << 16) | 2);
    put64(&mut bytes, 80, payload_offset as u64);
    put64(&mut bytes, 88, payload.len() as u64);
    put64(&mut bytes, 96, payload.len() as u64);
    bytes[payload_offset..].copy_from_slice(payload);
    bytes
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn decode(bytes: &[u8], out: &mut [u8]) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    let document = Ktx2Document::parse(bytes).expect("valid fixture");
    decode_ktx2_rgba8_base_level(&document, out)
}

fn decode_with_scratch(
    bytes: &[u8],
    out: &mut [u8],
    scratch: &mut [u8],
) -> Result<Rgba8Ktx2Image, Rgba8Ktx2Error> {
    let document = Ktx2Document::parse(bytes).expect("valid fixture");
    decode_ktx2_rgba8_base_level_with_scratch(&document, out, scratch)
}

fn supercompressed_fixture(
    vk_format: u32,
    scheme: u32,
    payload: &[u8],
    output_len: usize,
) -> Vec<u8> {
    let payload_offset = 132usize;
    let mut bytes = vec![0u8; payload_offset + payload.len()];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    put32(&mut bytes, 12, vk_format);
    put32(&mut bytes, 16, 1);
    put32(&mut bytes, 20, 2);
    put32(&mut bytes, 24, 1);
    put32(&mut bytes, 36, 1);
    put32(&mut bytes, 40, 1);
    put32(&mut bytes, 44, scheme);
    put32(&mut bytes, 48, 104);
    put32(&mut bytes, 52, 28);
    put32(&mut bytes, 104, 28);
    put32(&mut bytes, 112, (24 << 16) | 2);
    put64(&mut bytes, 80, payload_offset as u64);
    put64(&mut bytes, 88, payload.len() as u64);
    put64(&mut bytes, 96, output_len as u64);
    bytes[payload_offset..].copy_from_slice(payload);
    bytes
}

#[test]
fn decodes_unorm_and_srgb_base_level() {
    let texels = [1, 2, 3, 4, 5, 6, 7, 8];
    for (vk_format, expected) in [
        (VK_FORMAT_R8G8B8A8_UNORM, Rgba8Ktx2Format::Unorm),
        (VK_FORMAT_R8G8B8A8_SRGB, Rgba8Ktx2Format::Srgb),
    ] {
        let bytes = fixture(vk_format, 2, 1, &texels);
        let mut out = [0u8; 10];
        let image = decode(&bytes, &mut out).unwrap();
        assert_eq!(image.width, 2);
        assert_eq!(image.height, 1);
        assert_eq!(image.byte_len, 8);
        assert_eq!(image.format, expected);
        assert_eq!(&out[..8], &texels);
        assert_eq!(out[8..], [0, 0]);
    }
}

#[test]
fn inspector_reports_requirements_without_an_output_buffer() {
    let texels = [7u8; 24];
    let bytes = fixture(VK_FORMAT_R8G8B8A8_SRGB, 3, 2, &texels);
    let document = Ktx2Document::parse(&bytes).unwrap();
    assert_eq!(
        inspect_ktx2_rgba8_base_level(&document),
        Ok(Rgba8Ktx2Image {
            width: 3,
            height: 2,
            byte_len: 24,
            format: Rgba8Ktx2Format::Srgb,
        })
    );
}

#[test]
fn rejects_unsupported_format_and_container_modes() {
    let payload = [0u8; 4];
    let mut out = [0xA5; 4];
    let bytes = fixture(97, 1, 1, &payload);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedFormat)
    );

    let mut bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 1, 1, &payload);
    put32(&mut bytes, 32, 1);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedArray)
    );

    let mut bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 1, 1, &payload);
    put32(&mut bytes, 16, 2);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedTypeSize)
    );

    let mut bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 1, 1, &payload);
    put32(&mut bytes, 28, 1);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedDimensions)
    );

    let mut bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 1, 1, &payload);
    put32(&mut bytes, 44, 2);
    put64(&mut bytes, 80, 132);
    bytes.copy_within(136..140, 132);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedSupercompression)
    );

    let mut bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 1, 1, &payload);
    put32(&mut bytes, 36, 6);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::UnsupportedFaceCount)
    );
}

#[test]
fn short_output_and_bad_texel_length_leave_output_unchanged() {
    let mut short = [0xA5; 7];
    let bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 2, 1, &[1; 8]);
    assert_eq!(
        decode(&bytes, &mut short),
        Err(Rgba8Ktx2Error::OutputTooSmall)
    );
    assert_eq!(short, [0xA5; 7]);

    let mut out = [0xA5; 8];
    let bytes = fixture(VK_FORMAT_R8G8B8A8_UNORM, 2, 1, &[1; 4]);
    assert_eq!(
        decode(&bytes, &mut out),
        Err(Rgba8Ktx2Error::InvalidBaseLevelLength)
    );
    assert_eq!(out, [0xA5; 8]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn decodes_bounded_zstd_and_zlib_rgba8_without_losing_color_space() {
    let texels = [1, 2, 3, 4, 5, 6, 7, 8];
    let zstd_payload = zstd::bulk::compress(&texels, 1).unwrap();
    let zlib_payload = {
        use std::io::Write;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&texels).unwrap();
        encoder.finish().unwrap()
    };
    for (scheme, payload, format) in [
        (2, zstd_payload, Rgba8Ktx2Format::Srgb),
        (3, zlib_payload, Rgba8Ktx2Format::Unorm),
    ] {
        let vk_format = if format == Rgba8Ktx2Format::Srgb {
            VK_FORMAT_R8G8B8A8_SRGB
        } else {
            VK_FORMAT_R8G8B8A8_UNORM
        };
        let bytes = supercompressed_fixture(vk_format, scheme, &payload, texels.len());
        let mut out = [0xA5; 8];
        let mut scratch = [0u8; 8];
        let image = decode_with_scratch(&bytes, &mut out, &mut scratch).unwrap();
        assert_eq!(image.format, format);
        assert_eq!(out, texels);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn compressed_decode_is_bounded_and_keeps_output_unchanged_on_failure() {
    let texels = [9, 8, 7, 6, 5, 4, 3, 2];
    let payload = zstd::bulk::compress(&texels, 1).unwrap();
    let bytes = supercompressed_fixture(VK_FORMAT_R8G8B8A8_UNORM, 2, &payload, texels.len());
    let mut out = [0xA5; 8];
    assert_eq!(
        decode_with_scratch(&bytes, &mut out, &mut [0; 7]),
        Err(Rgba8Ktx2Error::CompressedScratchTooSmall)
    );
    assert_eq!(out, [0xA5; 8]);

    let bytes = supercompressed_fixture(VK_FORMAT_R8G8B8A8_UNORM, 2, &[1, 2, 3], texels.len());
    assert_eq!(
        decode_with_scratch(&bytes, &mut out, &mut [0; 8]),
        Err(Rgba8Ktx2Error::InvalidCompressedLevel)
    );
    assert_eq!(out, [0xA5; 8]);
}
