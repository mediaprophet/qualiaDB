//! MAT3 writer and versioned MAT1/MAT2/MAT3 reader for the `.10d` material part.

use super::material_section::{
    decode_material, encode_material, encoded_len, read_u16, read_u32, read_u64, validate_records,
    MaterialRecord, MaterialSectionError, SubmeshRange, TextureAddressMode, TextureFilterMode,
    TextureMinFilter, LEGACY_MAGIC, LEGACY_VERSION, MAGIC, MATERIAL_RECORD_SIZE,
    MATERIAL_SECTION_HEADER_SIZE, MATERIAL_SECTION_VERSION, MATERIAL_V1_RECORD_SIZE, MAX_MATERIALS,
    MAX_SUBMESH_RANGES, PREVIOUS_MAGIC, PREVIOUS_VERSION, SUBMESH_RANGE_SIZE,
    TEXTURE_SAMPLER_RECORD_SIZE,
};

const TRANSFORM_OFFSET: usize = MATERIAL_V1_RECORD_SIZE;
const TRANSFORM_SIZE: usize = 24;
const TRANSFORM_COUNT: usize = 6;
const SAMPLER_OFFSET: usize = TRANSFORM_OFFSET + TRANSFORM_SIZE * TRANSFORM_COUNT;
const SAMPLER_SIZE: usize = TEXTURE_SAMPLER_RECORD_SIZE;
const SAMPLER_COUNT: usize = 6;
const RESERVED_START: usize = SAMPLER_OFFSET + SAMPLER_SIZE * SAMPLER_COUNT;

pub(super) fn encode(
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    mesh_index_count: u32,
    out: &mut [u8],
) -> Result<usize, MaterialSectionError> {
    let needed = encoded_len(materials.len(), ranges.len()).ok_or_else(|| {
        if materials.is_empty() || ranges.is_empty() {
            MaterialSectionError::Empty
        } else if materials.len() > MAX_MATERIALS || ranges.len() > MAX_SUBMESH_RANGES {
            MaterialSectionError::CountLimit
        } else {
            MaterialSectionError::SizeOverflow
        }
    })?;
    if out.len() < needed {
        return Err(MaterialSectionError::OutputTooSmall {
            needed,
            got: out.len(),
        });
    }
    validate_records(materials, ranges, mesh_index_count)?;
    out[..needed].fill(0);
    out[..4].copy_from_slice(&MAGIC);
    out[4..6].copy_from_slice(&MATERIAL_SECTION_VERSION.to_le_bytes());
    out[8..12].copy_from_slice(&(materials.len() as u32).to_le_bytes());
    out[12..16].copy_from_slice(&(ranges.len() as u32).to_le_bytes());
    out[16..20].copy_from_slice(&(MATERIAL_RECORD_SIZE as u32).to_le_bytes());
    out[20..24].copy_from_slice(&(SUBMESH_RANGE_SIZE as u32).to_le_bytes());

    let mut offset = MATERIAL_SECTION_HEADER_SIZE;
    for material in materials {
        let record = &mut out[offset..offset + MATERIAL_RECORD_SIZE];
        encode_material(material, &mut record[..MATERIAL_V1_RECORD_SIZE]);
        for (index, transform) in material.texture_transforms.iter().enumerate() {
            let start = TRANSFORM_OFFSET + index * TRANSFORM_SIZE;
            for (component, value) in transform
                .offset
                .into_iter()
                .chain(transform.scale)
                .chain([transform.rotation])
                .enumerate()
            {
                record[start + component * 4..start + component * 4 + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
            record[start + 20..start + 24].copy_from_slice(&transform.tex_coord.to_le_bytes());
        }
        for (index, sampler) in material.texture_samplers.iter().enumerate() {
            let start = SAMPLER_OFFSET + index * SAMPLER_SIZE;
            record[start] = sampler.wrap_s as u8;
            record[start + 1] = sampler.wrap_t as u8;
            record[start + 2] = sampler.mag_filter as u8;
            record[start + 3] = sampler.min_filter as u8;
        }
        offset += MATERIAL_RECORD_SIZE;
    }
    for range in ranges {
        out[offset..offset + 4].copy_from_slice(&range.first_index.to_le_bytes());
        out[offset + 4..offset + 8].copy_from_slice(&range.index_count.to_le_bytes());
        out[offset + 8..offset + 16].copy_from_slice(&range.material_id.to_le_bytes());
        out[offset + 16..offset + 24].copy_from_slice(&range.semantic_id.to_le_bytes());
        offset += SUBMESH_RANGE_SIZE;
    }
    Ok(offset)
}

pub(super) fn decode(
    bytes: &[u8],
    mesh_index_count: u32,
    materials_out: &mut [MaterialRecord],
    ranges_out: &mut [SubmeshRange],
) -> Result<(usize, usize), MaterialSectionError> {
    let (material_count, range_count) = counts(bytes)?;
    if materials_out.len() < material_count || ranges_out.len() < range_count {
        return Err(MaterialSectionError::OutputCountTooSmall);
    }
    let has_transforms = bytes[..4] != LEGACY_MAGIC;
    let has_samplers = bytes[..4] == MAGIC;
    let record_size = if has_transforms {
        MATERIAL_RECORD_SIZE
    } else {
        MATERIAL_V1_RECORD_SIZE
    };
    let mut offset = MATERIAL_SECTION_HEADER_SIZE;
    for (index, out) in materials_out.iter_mut().take(material_count).enumerate() {
        let record = &bytes[offset..offset + record_size];
        *out = decode_material(&record[..MATERIAL_V1_RECORD_SIZE], index)?;
        if has_transforms {
            for slot in 0..TRANSFORM_COUNT {
                let start = TRANSFORM_OFFSET + slot * TRANSFORM_SIZE;
                let mut floats = [0.0; 5];
                for (component, value) in floats.iter_mut().enumerate() {
                    *value = f32::from_le_bytes(
                        record[start + component * 4..start + component * 4 + 4]
                            .try_into()
                            .expect("fixed transform field"),
                    );
                }
                out.texture_transforms[slot] = super::material_section::TextureTransform {
                    offset: [floats[0], floats[1]],
                    scale: [floats[2], floats[3]],
                    rotation: floats[4],
                    tex_coord: read_u32(record, start + 20),
                };
            }
            for slot in 0..SAMPLER_COUNT {
                if !has_samplers {
                    break;
                }
                let start = SAMPLER_OFFSET + slot * SAMPLER_SIZE;
                out.texture_samplers[slot] = super::material_section::TextureSampler {
                    wrap_s: match record[start] {
                        0 => TextureAddressMode::ClampToEdge,
                        1 => TextureAddressMode::MirroredRepeat,
                        2 => TextureAddressMode::Repeat,
                        _ => return Err(MaterialSectionError::Layout),
                    },
                    wrap_t: match record[start + 1] {
                        0 => TextureAddressMode::ClampToEdge,
                        1 => TextureAddressMode::MirroredRepeat,
                        2 => TextureAddressMode::Repeat,
                        _ => return Err(MaterialSectionError::Layout),
                    },
                    mag_filter: match record[start + 2] {
                        0 => TextureFilterMode::Nearest,
                        1 => TextureFilterMode::Linear,
                        _ => return Err(MaterialSectionError::Layout),
                    },
                    min_filter: match record[start + 3] {
                        0 => TextureMinFilter::Nearest,
                        1 => TextureMinFilter::Linear,
                        2 => TextureMinFilter::NearestMipmapNearest,
                        3 => TextureMinFilter::LinearMipmapNearest,
                        4 => TextureMinFilter::NearestMipmapLinear,
                        5 => TextureMinFilter::LinearMipmapLinear,
                        _ => return Err(MaterialSectionError::Layout),
                    },
                };
                if record[start + 4..start + SAMPLER_SIZE]
                    .iter()
                    .any(|&byte| byte != 0)
                {
                    return Err(MaterialSectionError::Layout);
                }
            }
            let reserved_start = if has_samplers {
                RESERVED_START
            } else {
                SAMPLER_OFFSET
            };
            if record[reserved_start..].iter().any(|&byte| byte != 0) {
                return Err(MaterialSectionError::Layout);
            }
        }
        offset += record_size;
    }
    for out in ranges_out.iter_mut().take(range_count) {
        *out = SubmeshRange {
            first_index: read_u32(bytes, offset),
            index_count: read_u32(bytes, offset + 4),
            material_id: read_u64(bytes, offset + 8),
            semantic_id: read_u64(bytes, offset + 16),
        };
        offset += SUBMESH_RANGE_SIZE;
    }
    validate_records(
        &materials_out[..material_count],
        &ranges_out[..range_count],
        mesh_index_count,
    )?;
    Ok((material_count, range_count))
}

pub(super) fn counts(bytes: &[u8]) -> Result<(usize, usize), MaterialSectionError> {
    if bytes.len() < MATERIAL_SECTION_HEADER_SIZE {
        return Err(MaterialSectionError::Header);
    }
    let (record_size, version) = match &bytes[..4] {
        magic if magic == MAGIC => (MATERIAL_RECORD_SIZE, MATERIAL_SECTION_VERSION),
        magic if magic == PREVIOUS_MAGIC => (MATERIAL_RECORD_SIZE, PREVIOUS_VERSION),
        magic if magic == LEGACY_MAGIC => (MATERIAL_V1_RECORD_SIZE, LEGACY_VERSION),
        _ => return Err(MaterialSectionError::Header),
    };
    let found_version = read_u16(bytes, 4);
    if found_version != version {
        return Err(MaterialSectionError::Version(found_version));
    }
    if read_u16(bytes, 6) != 0
        || read_u32(bytes, 16) as usize != record_size
        || read_u32(bytes, 20) as usize != SUBMESH_RANGE_SIZE
    {
        return Err(MaterialSectionError::Layout);
    }
    let material_count = read_u32(bytes, 8) as usize;
    let range_count = read_u32(bytes, 12) as usize;
    if material_count == 0
        || range_count == 0
        || material_count > MAX_MATERIALS
        || range_count > MAX_SUBMESH_RANGES
    {
        return Err(MaterialSectionError::CountLimit);
    }
    let expected = MATERIAL_SECTION_HEADER_SIZE
        .checked_add(
            material_count
                .checked_mul(record_size)
                .ok_or(MaterialSectionError::SizeOverflow)?,
        )
        .and_then(|size| size.checked_add(range_count.checked_mul(SUBMESH_RANGE_SIZE)?))
        .ok_or(MaterialSectionError::SizeOverflow)?;
    if bytes.len() != expected {
        return Err(MaterialSectionError::PayloadLength {
            expected,
            got: bytes.len(),
        });
    }
    Ok((material_count, range_count))
}
