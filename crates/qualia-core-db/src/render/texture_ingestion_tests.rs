use super::*;

fn dummy_png() -> Vec<u8> {
    let mut v = PNG_SIGNATURE.to_vec();
    v.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    v
}

fn dummy_jpeg() -> Vec<u8> {
    let mut v = JPEG_SIGNATURE.to_vec();
    v.extend_from_slice(&[0xE0, 0x00, 0x10]);
    v
}

fn dummy_ktx2() -> Vec<u8> {
    let mut v = KTX2_IDENTIFIER.to_vec();
    v.extend_from_slice(&[0; 32]);
    v
}

fn dummy_webp() -> Vec<u8> {
    let mut v = WEBP_RIFF.to_vec();
    v.extend_from_slice(&[10, 0, 0, 0]);
    v.extend_from_slice(&WEBP_MAGIC);
    v
}

#[test]
fn validates_png_signature() {
    let png = dummy_png();
    assert!(validate_image_signature("image/png", &png).is_ok());
    assert!(validate_image_signature("image/png; charset=binary", &png).is_ok());
    assert_eq!(
        validate_image_signature("image/jpeg", &png),
        Err(TextureIngestionError::MimeSignatureMismatch)
    );
}

#[test]
fn validates_jpeg_signature() {
    let jpeg = dummy_jpeg();
    assert!(validate_image_signature("image/jpeg", &jpeg).is_ok());
    assert!(validate_image_signature("image/jpg", &jpeg).is_ok());
    assert_eq!(
        validate_image_signature("image/png", &jpeg),
        Err(TextureIngestionError::MimeSignatureMismatch)
    );
}

#[test]
fn validates_ktx2_signature() {
    let ktx2 = dummy_ktx2();
    assert!(validate_image_signature("image/ktx2", &ktx2).is_ok());
    assert_eq!(
        validate_image_signature("image/png", &ktx2),
        Err(TextureIngestionError::MimeSignatureMismatch)
    );
}

#[test]
fn validates_webp_signature() {
    let webp = dummy_webp();
    assert!(validate_image_signature("image/webp", &webp).is_ok());
    assert_eq!(
        validate_image_signature("image/png", &webp),
        Err(TextureIngestionError::MimeSignatureMismatch)
    );
}

#[test]
fn rejects_unsupported_mime() {
    let bytes = [1, 2, 3, 4];
    assert_eq!(
        validate_image_signature("image/bmp", &bytes),
        Err(TextureIngestionError::UnsupportedMime)
    );
}

#[test]
fn decodes_base64_data_uri_png() {
    let png = dummy_png();
    let b64 = BASE64_STANDARD.encode(&png);
    let uri = format!("data:image/png;base64,{b64}");

    let (mime, bytes) = parse_data_uri(&uri, 1024).unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(bytes, png);
}

#[test]
fn decodes_percent_encoded_data_uri() {
    let jpeg = dummy_jpeg();
    let mut encoded = String::new();
    for b in &jpeg {
        encoded.push_str(&format!("%{:02X}", b));
    }
    let uri = format!("data:image/jpeg,{encoded}");

    let (mime, bytes) = parse_data_uri(&uri, 1024).unwrap();
    assert_eq!(mime, "image/jpeg");
    assert_eq!(bytes, jpeg);
}

#[test]
fn data_uri_exceeding_budget_is_rejected() {
    let png = dummy_png();
    let b64 = BASE64_STANDARD.encode(&png);
    let uri = format!("data:image/png;base64,{b64}");

    assert_eq!(
        parse_data_uri(&uri, png.len() - 1),
        Err(TextureIngestionError::EncodedInputTooLarge)
    );
}

#[test]
fn data_uri_with_mismatched_signature_is_rejected() {
    let png = dummy_png();
    let b64 = BASE64_STANDARD.encode(&png);
    // Claim it is JPEG, but payload is PNG
    let uri = format!("data:image/jpeg;base64,{b64}");

    assert_eq!(
        parse_data_uri(&uri, 1024),
        Err(TextureIngestionError::MimeSignatureMismatch)
    );
}

#[test]
fn ingest_payload_from_raw_bytes_and_uri() {
    let limits = TextureDecodeLimits::default();
    let png = dummy_png();
    let ingested_raw = ingest_texture_payload("image/png", Some(&png), limits).unwrap();
    assert_eq!(ingested_raw.mime_type, "image/png");
    assert_eq!(ingested_raw.bytes, png);

    let b64 = BASE64_STANDARD.encode(&png);
    let uri = format!("data:image/png;base64,{b64}");
    let ingested_uri = ingest_texture_payload(&uri, None, limits).unwrap();
    assert_eq!(ingested_uri.digest, ingested_raw.digest);
    assert_eq!(ingested_uri.bytes, png);
}
