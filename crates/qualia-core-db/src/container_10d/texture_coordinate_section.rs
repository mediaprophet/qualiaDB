//! Versioned, vertex-aligned UV0 sidecar for imported material texture coordinates.

pub const TEXTURE_COORDINATE_SECTION_VERSION: u16 = 1;
pub const TEXTURE_COORDINATE_SECTION_HEADER_SIZE: usize = 16;
const MAGIC: [u8; 4] = *b"UV01";
const RECORD_SIZE: usize = 8;
pub const MAX_TEXTURE_COORDINATES: usize = super::mesh_section::MAX_VERTEX_COUNT as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextureCoordinateSectionError {
    Empty,
    CountLimit,
    SizeOverflow,
    OutputTooSmall,
    Header,
    Version(u16),
    Flags(u16),
    CountMismatch { expected: usize, got: usize },
    PayloadLength { expected: usize, got: usize },
    NonFinite { index: usize },
    OutOfRange { index: usize },
    OutputCountTooSmall,
}

impl std::fmt::Display for TextureCoordinateSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".10d UV01 texture-coordinate section: {self:?}")
    }
}

impl std::error::Error for TextureCoordinateSectionError {}

pub fn encoded_len(count: usize) -> Option<usize> {
    if count == 0 || count > MAX_TEXTURE_COORDINATES {
        return None;
    }
    TEXTURE_COORDINATE_SECTION_HEADER_SIZE.checked_add(count.checked_mul(RECORD_SIZE)?)
}

/// Write UV0 pairs as little-endian float32 after validating the complete stream.
pub fn encode_texture_coordinate_section(
    coordinates: &[[f32; 2]],
    out: &mut [u8],
) -> Result<usize, TextureCoordinateSectionError> {
    let needed = encoded_len(coordinates.len()).ok_or(if coordinates.is_empty() {
        TextureCoordinateSectionError::Empty
    } else {
        TextureCoordinateSectionError::CountLimit
    })?;
    if out.len() < needed {
        return Err(TextureCoordinateSectionError::OutputTooSmall);
    }
    validate_coordinates(coordinates)?;
    out[..4].copy_from_slice(&MAGIC);
    out[4..6].copy_from_slice(&TEXTURE_COORDINATE_SECTION_VERSION.to_le_bytes());
    out[6..8].copy_from_slice(&0u16.to_le_bytes());
    out[8..12].copy_from_slice(&(coordinates.len() as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(RECORD_SIZE as u32).to_le_bytes());
    for (index, coordinate) in coordinates.iter().enumerate() {
        let start = TEXTURE_COORDINATE_SECTION_HEADER_SIZE + index * RECORD_SIZE;
        out[start..start + 4].copy_from_slice(&coordinate[0].to_le_bytes());
        out[start + 4..start + 8].copy_from_slice(&coordinate[1].to_le_bytes());
    }
    Ok(needed)
}

/// Validate and decode UV0 pairs into a caller-owned buffer. Values outside ±1,000,000 are
/// rejected because they exceed useful UV precision and can destabilize texture sampling.
pub fn decode_texture_coordinates_into(
    payload: &[u8],
    expected_vertex_count: usize,
    out: &mut [[f32; 2]],
) -> Result<usize, TextureCoordinateSectionError> {
    if payload.len() < TEXTURE_COORDINATE_SECTION_HEADER_SIZE
        || payload[..4] != MAGIC
        || u32::from_le_bytes(payload[12..16].try_into().unwrap()) != RECORD_SIZE as u32
    {
        return Err(TextureCoordinateSectionError::Header);
    }
    let version = u16::from_le_bytes(payload[4..6].try_into().unwrap());
    if version != TEXTURE_COORDINATE_SECTION_VERSION {
        return Err(TextureCoordinateSectionError::Version(version));
    }
    let flags = u16::from_le_bytes(payload[6..8].try_into().unwrap());
    if flags != 0 {
        return Err(TextureCoordinateSectionError::Flags(flags));
    }
    let count = u32::from_le_bytes(payload[8..12].try_into().unwrap()) as usize;
    if count != expected_vertex_count {
        return Err(TextureCoordinateSectionError::CountMismatch {
            expected: expected_vertex_count,
            got: count,
        });
    }
    if count > MAX_TEXTURE_COORDINATES {
        return Err(TextureCoordinateSectionError::CountLimit);
    }
    if out.len() < count {
        return Err(TextureCoordinateSectionError::OutputCountTooSmall);
    }
    let expected_len = encoded_len(count).ok_or(TextureCoordinateSectionError::CountLimit)?;
    if payload.len() != expected_len {
        return Err(TextureCoordinateSectionError::PayloadLength {
            expected: expected_len,
            got: payload.len(),
        });
    }
    for index in 0..count {
        let start = TEXTURE_COORDINATE_SECTION_HEADER_SIZE + index * RECORD_SIZE;
        let u = f32::from_le_bytes(payload[start..start + 4].try_into().unwrap());
        let v = f32::from_le_bytes(payload[start + 4..start + 8].try_into().unwrap());
        validate_coordinate([u, v], index)?;
    }
    for (index, coordinate) in out.iter_mut().take(count).enumerate() {
        let start = TEXTURE_COORDINATE_SECTION_HEADER_SIZE + index * RECORD_SIZE;
        coordinate[0] = f32::from_le_bytes(payload[start..start + 4].try_into().unwrap());
        coordinate[1] = f32::from_le_bytes(payload[start + 4..start + 8].try_into().unwrap());
    }
    Ok(count)
}

pub fn texture_coordinate_section_count(
    payload: &[u8],
) -> Result<usize, TextureCoordinateSectionError> {
    if payload.len() < TEXTURE_COORDINATE_SECTION_HEADER_SIZE
        || payload[..4] != MAGIC
        || u32::from_le_bytes(payload[12..16].try_into().unwrap()) != RECORD_SIZE as u32
    {
        return Err(TextureCoordinateSectionError::Header);
    }
    let version = u16::from_le_bytes(payload[4..6].try_into().unwrap());
    if version != TEXTURE_COORDINATE_SECTION_VERSION {
        return Err(TextureCoordinateSectionError::Version(version));
    }
    let flags = u16::from_le_bytes(payload[6..8].try_into().unwrap());
    if flags != 0 {
        return Err(TextureCoordinateSectionError::Flags(flags));
    }
    let count = u32::from_le_bytes(payload[8..12].try_into().unwrap()) as usize;
    if count == 0 || count > MAX_TEXTURE_COORDINATES {
        return Err(TextureCoordinateSectionError::CountLimit);
    }
    let expected_len = encoded_len(count).ok_or(TextureCoordinateSectionError::CountLimit)?;
    if payload.len() != expected_len {
        return Err(TextureCoordinateSectionError::PayloadLength {
            expected: expected_len,
            got: payload.len(),
        });
    }
    Ok(count)
}

fn validate_coordinates(coordinates: &[[f32; 2]]) -> Result<(), TextureCoordinateSectionError> {
    for (index, &coordinate) in coordinates.iter().enumerate() {
        validate_coordinate(coordinate, index)?;
    }
    Ok(())
}

fn validate_coordinate(
    coordinate: [f32; 2],
    index: usize,
) -> Result<(), TextureCoordinateSectionError> {
    if coordinate.iter().any(|value| !value.is_finite()) {
        return Err(TextureCoordinateSectionError::NonFinite { index });
    }
    if coordinate.iter().any(|value| value.abs() > 1_000_000.0) {
        return Err(TextureCoordinateSectionError::OutOfRange { index });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv01_round_trips_pairs_and_rejects_corrupt_float_values() {
        let input = [[0.0, 1.0], [-0.25, 3.5]];
        let mut encoded = vec![0; encoded_len(input.len()).unwrap()];
        let written = encode_texture_coordinate_section(&input, &mut encoded).unwrap();
        assert_eq!(written, encoded.len());
        let mut output = [[0.0; 2]; 2];
        assert_eq!(
            decode_texture_coordinates_into(&encoded, 2, &mut output),
            Ok(2)
        );
        assert_eq!(output, input);
        encoded[TEXTURE_COORDINATE_SECTION_HEADER_SIZE..TEXTURE_COORDINATE_SECTION_HEADER_SIZE + 4]
            .copy_from_slice(&f32::NAN.to_le_bytes());
        assert_eq!(
            decode_texture_coordinates_into(&encoded, 2, &mut output),
            Err(TextureCoordinateSectionError::NonFinite { index: 0 })
        );
    }
}
