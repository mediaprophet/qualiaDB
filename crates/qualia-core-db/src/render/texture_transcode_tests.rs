//! Unit and qualification tests for compressed texture block decoding and KTX2 transcoding.

use super::texture_transcode::*;
use crate::render::texture_ktx2::KTX2_IDENTIFIER;

#[test]
fn test_decode_bc1_block_endpoints_and_interpolations() {
    // c0 = pure red (0xF800), c1 = pure green (0x07E0)
    // c0 > c1, so palette has:
    // [0] = red
    // [1] = green
    // [2] = 2/3 red + 1/3 green
    // [3] = 1/3 red + 2/3 green
    let mut block = [0u8; 8];
    block[0..2].copy_from_slice(&0xF800u16.to_le_bytes());
    block[2..4].copy_from_slice(&0x07E0u16.to_le_bytes());
    // Assign pixel 0 -> code 0, pixel 1 -> code 1, pixel 2 -> code 2, pixel 3 -> code 3
    // index bits: 0b11_10_01_00 = 0xE4
    block[4] = 0xE4;

    let mut out_pixels = [[0u8; 4]; 16];
    decode_bc1_block(&block, &mut out_pixels);

    // Pixel 0: red
    assert_eq!(out_pixels[0], [255, 0, 0, 255]);
    // Pixel 1: green
    assert_eq!(out_pixels[1], [0, 255, 0, 255]);
    // Pixel 2: 2/3 red + 1/3 green
    assert!(out_pixels[2][0] > 160 && out_pixels[2][1] > 70);
    // Pixel 3: 1/3 red + 2/3 green
    assert!(out_pixels[3][0] > 70 && out_pixels[3][1] > 160);
}

#[test]
fn test_decode_bc3_block_alpha_and_color() {
    let mut block = [0u8; 16];
    // Alpha: a0 = 200, a1 = 100
    block[0] = 200;
    block[1] = 100;
    // Color: c0 = blue (0x001F), c1 = black (0x0000)
    block[8..10].copy_from_slice(&0x001Fu16.to_le_bytes());
    block[10..12].copy_from_slice(&0x0000u16.to_le_bytes());

    let mut out_pixels = [[0u8; 4]; 16];
    decode_bc3_block(&block, &mut out_pixels);

    // First pixel alpha should be a0 (200)
    assert_eq!(out_pixels[0][3], 200);
    // Color should have blue channel
    assert_eq!(out_pixels[0][2], 255);
}

#[test]
fn test_decode_compressed_image_to_rgba8_grid() {
    let width = 8;
    let height = 8;
    // 8x8 requires 2x2 = 4 BC1 blocks of 8 bytes = 32 bytes
    let mut blocks = vec![0u8; 32];
    for chunk in blocks.chunks_exact_mut(8) {
        chunk[0..2].copy_from_slice(&0xF800u16.to_le_bytes()); // red
        chunk[2..4].copy_from_slice(&0xF800u16.to_le_bytes()); // red
    }

    let mut output = vec![0u8; width * height * 4];
    decode_compressed_image_to_rgba8(
        &blocks,
        width as u32,
        height as u32,
        CompressedBlockFormat::Bc1Rgba,
        &mut output,
    )
    .expect("decode 8x8 bc1 grid");

    // All pixels should be pure opaque red
    for px in output.chunks_exact(4) {
        assert_eq!(px, [255, 0, 0, 255]);
    }
}

#[test]
fn test_transcode_ktx2_into_uncompressed_bc1() {
    let width = 4u32;
    let height = 4u32;
    let mut bytes = vec![0u8; 144];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    // vk_format = 131 (VK_FORMAT_BC1_RGB_UNORM_BLOCK)
    bytes[12..16].copy_from_slice(&131u32.to_le_bytes());
    // type_size = 1
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    // pixel_width = 4
    bytes[20..24].copy_from_slice(&width.to_le_bytes());
    // pixel_height = 4
    bytes[24..28].copy_from_slice(&height.to_le_bytes());
    // face_count = 1
    bytes[36..40].copy_from_slice(&1u32.to_le_bytes());
    // level_count = 1
    bytes[40..44].copy_from_slice(&1u32.to_le_bytes());
    // supercompression_scheme = 0 (None)
    bytes[44..48].copy_from_slice(&0u32.to_le_bytes());
    // dfd_byte_offset = 104
    bytes[48..52].copy_from_slice(&104u32.to_le_bytes());
    // dfd_byte_length = 28
    bytes[52..56].copy_from_slice(&28u32.to_le_bytes());

    // Level index 0 (at offset 80): offset = 136, byte_length = 8, uncompressed = 8
    bytes[80..88].copy_from_slice(&136u64.to_le_bytes());
    bytes[88..96].copy_from_slice(&8u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&8u64.to_le_bytes());

    // DFD header at offset 104
    bytes[104..108].copy_from_slice(&28u32.to_le_bytes());
    bytes[112..116].copy_from_slice(&((24u32 << 16) | 2).to_le_bytes());

    // Payload at 136: pure red BC1 block
    bytes[136..138].copy_from_slice(&0xF800u16.to_le_bytes());
    bytes[138..140].copy_from_slice(&0xF800u16.to_le_bytes());

    let mut output = vec![0u8; 4 * 4 * 4];
    let mut scratch = vec![0u8; 64];

    let info = transcode_ktx2_into(
        &bytes,
        TranscodeTargetFormat::Rgba8,
        &mut output,
        &mut scratch,
    )
    .expect("transcode ktx2 bc1");

    assert_eq!(info.width, 4);
    assert_eq!(info.height, 4);
    assert_eq!(info.output_bytes, 64);

    for px in output.chunks_exact(4) {
        assert_eq!(px, [255, 0, 0, 255]);
    }
}
