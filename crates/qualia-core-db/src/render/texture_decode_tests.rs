use super::*;

fn png_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[10, 20, 30, 40, 50, 60, 70, 80])
            .unwrap();
    }
    bytes
}

fn ktx2_rgba8_fixture() -> Vec<u8> {
    use super::super::texture_ktx2::KTX2_IDENTIFIER;
    let mut bytes = vec![0u8; 144];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    bytes[12..16].copy_from_slice(&37u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&2u32.to_le_bytes());
    bytes[24..28].copy_from_slice(&1u32.to_le_bytes());
    bytes[36..40].copy_from_slice(&1u32.to_le_bytes());
    bytes[40..44].copy_from_slice(&1u32.to_le_bytes());
    bytes[48..52].copy_from_slice(&104u32.to_le_bytes());
    bytes[52..56].copy_from_slice(&28u32.to_le_bytes());
    bytes[104..108].copy_from_slice(&28u32.to_le_bytes());
    bytes[112..116].copy_from_slice(&((24u32 << 16) | 2).to_le_bytes());
    bytes[80..88].copy_from_slice(&136u64.to_le_bytes());
    bytes[88..96].copy_from_slice(&8u64.to_le_bytes());
    bytes[96..104].copy_from_slice(&8u64.to_le_bytes());
    bytes[136..144].copy_from_slice(&[10, 20, 30, 40, 50, 60, 70, 80]);
    bytes
}

fn resource<'a>(bytes: &'a [u8], mime_type: &'a str) -> HmcTextureResource<'a> {
    HmcTextureResource {
        digest: [0; 32],
        mime_type,
        bytes,
    }
}

#[test]
fn png_decodes_to_caller_buffer_and_preserves_alpha() {
    let bytes = png_fixture();
    let image = resource(&bytes, "image/png");
    let requirements =
        inspect_hmc_texture_requirements(&image, TextureDecodeLimits::default()).unwrap();
    assert_eq!(requirements.0.width, 2);
    assert_eq!(requirements.0.height, 1);
    assert_eq!(requirements.0.rgba8_bytes, 8);
    let mut output = [0; 8];
    let mut scratch = [0; 32];
    let info = decode_hmc_texture_rgba8_into(
        &image,
        TextureDecodeLimits::default(),
        &mut output,
        &mut scratch,
    )
    .unwrap();
    assert_eq!((info.width, info.height), (2, 1));
    assert_eq!(requirements.0, info);
    assert_eq!(&output, &[10, 20, 30, 40, 50, 60, 70, 80]);
}

#[test]
fn ktx2_rgba8_preflights_and_decodes_caller_buffer_with_mime_parameters() {
    let bytes = ktx2_rgba8_fixture();
    let image = resource(&bytes, " Image/KTX2 ; profile=rgba8 ");
    let requirements =
        inspect_hmc_texture_requirements(&image, TextureDecodeLimits::default()).unwrap();
    assert_eq!(requirements.0.width, 2);
    assert_eq!(requirements.0.height, 1);
    assert_eq!(requirements.0.rgba8_bytes, 8);
    assert_eq!(requirements.1, 0);
    let mut output = [0; 8];
    let info =
        decode_hmc_texture_rgba8_into(&image, TextureDecodeLimits::default(), &mut output, &mut [])
            .unwrap();
    assert_eq!(requirements.0, info);
    assert_eq!(output, [10, 20, 30, 40, 50, 60, 70, 80]);
}

#[test]
fn png_decode_fails_before_writing_when_output_or_limits_are_small() {
    let bytes = png_fixture();
    let image = resource(&bytes, "image/png");
    let mut output = [0; 7];
    let mut scratch = [0; 32];
    assert_eq!(
        decode_hmc_texture_rgba8_into(
            &image,
            TextureDecodeLimits::default(),
            &mut output,
            &mut scratch,
        ),
        Err(TextureDecodeError::OutputBufferTooSmall)
    );
    let limits = TextureDecodeLimits {
        max_decoded_bytes: 7,
        ..TextureDecodeLimits::default()
    };
    assert_eq!(
        decode_hmc_texture_rgba8_into(&image, limits, &mut [0; 8], &mut [0; 32]),
        Err(TextureDecodeError::DecodedImageExceedsLimit)
    );
    assert_eq!(
        decode_hmc_texture_rgba8(&image, limits),
        Err(TextureDecodeError::DecodedImageExceedsLimit)
    );
}

#[test]
fn unsupported_ktx2_format_is_not_claimed_as_decoded() {
    let mut bytes = ktx2_rgba8_fixture();
    bytes[12..16].copy_from_slice(&97u32.to_le_bytes()); // VK_FORMAT_R16G16B16A16_SFLOAT
    let image = resource(&bytes, "image/ktx2");
    assert_eq!(
        decode_hmc_texture_rgba8_into(
            &image,
            TextureDecodeLimits::default(),
            &mut [0; 16],
            &mut [0; 16],
        ),
        Err(TextureDecodeError::UnsupportedImageFormat)
    );
}
