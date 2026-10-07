//! Versioned `.10d` material and submesh sidecar.
//!
//! MAT3 stores linear factors, stable texture digests, per-slot UV transforms and sampler state;
//! readers remain compatible with MAT1 and MAT2. Texture payloads remain in HMC/asset dependencies.
//! Materials and submesh ranges are cold-path data encoded with caller buffers and bounded counts.

pub const MATERIAL_SECTION_VERSION: u16 = 3;
pub const MATERIAL_SECTION_HEADER_SIZE: usize = 24;
pub const MATERIAL_RECORD_SIZE: usize = 448;
pub const MATERIAL_V1_RECORD_SIZE: usize = 256;
pub const SUBMESH_RANGE_SIZE: usize = 24;
pub const MAX_MATERIALS: usize = 4096;
pub const MAX_SUBMESH_RANGES: usize = 1_048_576;
pub const MAX_EMISSIVE_FACTOR: f32 = 65_504.0;
pub const TEXTURE_SAMPLER_RECORD_SIZE: usize = 8;
pub(super) const MAGIC: [u8; 4] = *b"MAT3";
pub(super) const PREVIOUS_MAGIC: [u8; 4] = *b"MAT2";
pub(super) const PREVIOUS_VERSION: u16 = 2;
pub(super) const LEGACY_MAGIC: [u8; 4] = *b"MAT1";
pub(super) const LEGACY_VERSION: u16 = 1;
const FLAG_DOUBLE_SIDED: u16 = 1 << 0;
const FLAG_CASTS_SHADOW: u16 = 1 << 1;
const FLAG_RECEIVES_SHADOW: u16 = 1 << 2;
const FLAG_PICKABLE: u16 = 1 << 3;
const FLAG_VERTEX_COLOR_MULTIPLY: u16 = 1 << 4;
const KNOWN_FLAGS: u16 = FLAG_DOUBLE_SIDED
    | FLAG_CASTS_SHADOW
    | FLAG_RECEIVES_SHADOW
    | FLAG_PICKABLE
    | FLAG_VERTEX_COLOR_MULTIPLY;

/// Material shading model. Stylized shading uses the same material and resource contract.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadingModel {
    MetallicRoughness = 0,
    Stylized = 1,
}

impl ShadingModel {
    fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::MetallicRoughness),
            1 => Some(Self::Stylized),
            _ => None,
        }
    }
}

/// How alpha affects coverage and blending.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpacityMode {
    Opaque = 0,
    Mask = 1,
    Blend = 2,
}

/// glTF-compatible UV transform. `tex_coord` selects the source vertex set; current runtime supports
/// set zero and rejects other sets until their vertex streams are present end-to-end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureTransform {
    pub offset: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub tex_coord: u32,
}

/// glTF-compatible texture address mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureAddressMode {
    ClampToEdge = 0,
    MirroredRepeat = 1,
    Repeat = 2,
}

/// Magnification filter. glTF permits nearest or linear.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFilterMode {
    Nearest = 0,
    Linear = 1,
}

/// Minification filter including its mip selection policy.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureMinFilter {
    Nearest = 0,
    Linear = 1,
    NearestMipmapNearest = 2,
    LinearMipmapNearest = 3,
    NearestMipmapLinear = 4,
    LinearMipmapLinear = 5,
}

/// Per-map sampler state. Defaults follow glTF: repeat, linear magnification and trilinear minification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureSampler {
    pub wrap_s: TextureAddressMode,
    pub wrap_t: TextureAddressMode,
    pub mag_filter: TextureFilterMode,
    pub min_filter: TextureMinFilter,
}

impl TextureSampler {
    pub const GLTF_DEFAULT: Self = Self {
        wrap_s: TextureAddressMode::Repeat,
        wrap_t: TextureAddressMode::Repeat,
        mag_filter: TextureFilterMode::Linear,
        min_filter: TextureMinFilter::LinearMipmapLinear,
    };

    /// Stable index into the bounded 3×3×2×6 WebGPU sampler cache.
    pub const fn cache_index(self) -> usize {
        (((self.wrap_s as usize * 3 + self.wrap_t as usize) * 2 + self.mag_filter as usize) * 6)
            + self.min_filter as usize
    }

    pub const fn needs_filtering(self) -> bool {
        matches!(self.mag_filter, TextureFilterMode::Linear)
            || matches!(
                self.min_filter,
                TextureMinFilter::Linear
                    | TextureMinFilter::LinearMipmapNearest
                    | TextureMinFilter::NearestMipmapLinear
                    | TextureMinFilter::LinearMipmapLinear
            )
    }

    pub const fn nearest_variant(self) -> Self {
        Self {
            mag_filter: TextureFilterMode::Nearest,
            min_filter: TextureMinFilter::NearestMipmapNearest,
            ..self
        }
    }

    pub const fn filtering_variant(self) -> Self {
        Self {
            mag_filter: TextureFilterMode::Linear,
            min_filter: TextureMinFilter::LinearMipmapLinear,
            ..self
        }
    }
}

impl TextureTransform {
    pub const IDENTITY: Self = Self {
        offset: [0.0; 2],
        scale: [1.0; 2],
        rotation: 0.0,
        tex_coord: 0,
    };
}

impl OpacityMode {
    fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Opaque),
            1 => Some(Self::Mask),
            2 => Some(Self::Blend),
            _ => None,
        }
    }
}

/// Stable, renderer-independent material record. Colours are linear; each texture field is a
/// SHA-256 HMC/resource digest, or all-zero when that slot is unbound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialRecord {
    pub id: u64,
    pub shading_model: ShadingModel,
    pub opacity_mode: OpacityMode,
    pub double_sided: bool,
    pub casts_shadow: bool,
    pub receives_shadow: bool,
    pub pickable: bool,
    pub multiply_vertex_color: bool,
    pub base_color: [f32; 4],
    pub emissive: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub specular: f32,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub alpha_cutoff: f32,
    pub base_color_texture: [u8; 32],
    pub normal_texture: [u8; 32],
    /// glTF metallic-roughness texture dependency.
    pub metallic_roughness_texture: [u8; 32],
    /// glTF occlusion texture dependency; may differ from metallic-roughness.
    pub occlusion_texture: [u8; 32],
    pub emissive_texture: [u8; 32],
    /// Optional authored toon-ramp dependency; used only by `Stylized` materials.
    pub stylized_ramp_texture: [u8; 32],
    /// Base colour, normal, metallic-roughness, occlusion, emissive, stylized ramp.
    pub texture_transforms: [TextureTransform; 6],
    /// Base colour, normal, metallic-roughness, occlusion, emissive, stylized ramp.
    pub texture_samplers: [TextureSampler; 6],
}

impl MaterialRecord {
    /// Compatibility material for legacy static meshes without a material section.
    pub const fn legacy_default() -> Self {
        Self {
            id: crate::q_hash("urn:qualia:material:legacy-default"),
            shading_model: ShadingModel::MetallicRoughness,
            opacity_mode: OpacityMode::Opaque,
            double_sided: false,
            casts_shadow: true,
            receives_shadow: true,
            pickable: true,
            multiply_vertex_color: true,
            base_color: [1.0, 1.0, 1.0, 1.0],
            emissive: [0.0, 0.0, 0.0],
            metallic: 0.0,
            roughness: 1.0,
            specular: 1.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            alpha_cutoff: 0.5,
            base_color_texture: [0; 32],
            normal_texture: [0; 32],
            metallic_roughness_texture: [0; 32],
            occlusion_texture: [0; 32],
            emissive_texture: [0; 32],
            stylized_ramp_texture: [0; 32],
            texture_transforms: [TextureTransform::IDENTITY; 6],
            texture_samplers: [TextureSampler::GLTF_DEFAULT; 6],
        }
    }
}

/// A contiguous triangle-index interval bound to one stable material identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmeshRange {
    pub first_index: u32,
    pub index_count: u32,
    pub material_id: u64,
    /// Optional stable semantic entity hash; zero means no per-submesh identity.
    pub semantic_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialSectionError {
    Empty,
    CountLimit,
    SizeOverflow,
    OutputTooSmall { needed: usize, got: usize },
    Header,
    Version(u16),
    Layout,
    UnknownFlags(u16),
    InvalidModel(u8),
    InvalidOpacity(u8),
    InvalidMaterial { index: usize },
    MaterialOrder { index: usize },
    UnknownMaterial { range: usize, id: u64 },
    InvalidRange { index: usize },
    RangeOrder { index: usize },
    PayloadLength { expected: usize, got: usize },
    OutputCountTooSmall,
}

impl std::fmt::Display for MaterialSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".10d MAT3/MAT2/MAT1 material section: {self:?}")
    }
}

impl std::error::Error for MaterialSectionError {}

/// Exact encoded size. `None` means an invalid count or arithmetic overflow.
pub fn encoded_len(material_count: usize, range_count: usize) -> Option<usize> {
    if material_count == 0
        || range_count == 0
        || material_count > MAX_MATERIALS
        || range_count > MAX_SUBMESH_RANGES
    {
        return None;
    }
    MATERIAL_SECTION_HEADER_SIZE
        .checked_add(material_count.checked_mul(MATERIAL_RECORD_SIZE)?)?
        .checked_add(range_count.checked_mul(SUBMESH_RANGE_SIZE)?)
}

/// Write the current canonical MAT3 section into a caller-owned output buffer.
pub fn encode_material_section(
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    mesh_index_count: u32,
    out: &mut [u8],
) -> Result<usize, MaterialSectionError> {
    super::material_section_codec::encode(materials, ranges, mesh_index_count, out)
}

/// Decode MAT1, MAT2 or MAT3 into caller-owned arrays. Both outputs must fit the encoded counts.
pub fn decode_material_section_into(
    bytes: &[u8],
    mesh_index_count: u32,
    materials_out: &mut [MaterialRecord],
    ranges_out: &mut [SubmeshRange],
) -> Result<(usize, usize), MaterialSectionError> {
    super::material_section_codec::decode(bytes, mesh_index_count, materials_out, ranges_out)
}

/// Inspect validated MAT1/MAT2/MAT3 layout/counts without allocating output records.
pub fn material_section_counts(bytes: &[u8]) -> Result<(usize, usize), MaterialSectionError> {
    super::material_section_codec::counts(bytes)
}

/// Validate material/range arrays with the exact rules used by MAT3 encoding.
pub fn validate_material_bindings(
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    mesh_index_count: u32,
) -> Result<(), MaterialSectionError> {
    validate_records(materials, ranges, mesh_index_count)
}

pub(super) fn validate_records(
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    mesh_index_count: u32,
) -> Result<(), MaterialSectionError> {
    if materials.is_empty() || ranges.is_empty() {
        return Err(MaterialSectionError::Empty);
    }
    if materials.len() > MAX_MATERIALS || ranges.len() > MAX_SUBMESH_RANGES {
        return Err(MaterialSectionError::CountLimit);
    }
    for (index, material) in materials.iter().enumerate() {
        let floats = material
            .base_color
            .iter()
            .chain(material.emissive.iter())
            .copied()
            .chain([
                material.metallic,
                material.roughness,
                material.specular,
                material.normal_scale,
                material.occlusion_strength,
                material.alpha_cutoff,
            ]);
        if material.id == 0
            || floats.clone().any(|value| !value.is_finite())
            || material
                .base_color
                .iter()
                .any(|&value| !(0.0..=1.0).contains(&value))
            || material
                .emissive
                .iter()
                .any(|&value| !(0.0..=MAX_EMISSIVE_FACTOR).contains(&value))
            || !(0.0..=1.0).contains(&material.metallic)
            || !(0.0..=1.0).contains(&material.roughness)
            || !(0.0..=1.0).contains(&material.specular)
            || !(0.0..=8.0).contains(&material.normal_scale)
            || !(0.0..=1.0).contains(&material.occlusion_strength)
            || !(0.0..=1.0).contains(&material.alpha_cutoff)
            || material.texture_transforms.iter().any(|transform| {
                transform
                    .offset
                    .iter()
                    .chain(transform.scale.iter())
                    .chain(std::iter::once(&transform.rotation))
                    .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
                    || transform.tex_coord != 0
            })
            || (index > 0 && materials[index - 1].id >= material.id)
        {
            return Err(if index > 0 && materials[index - 1].id >= material.id {
                MaterialSectionError::MaterialOrder { index }
            } else {
                MaterialSectionError::InvalidMaterial { index }
            });
        }
    }
    let mut previous_end = 0u32;
    for (index, range) in ranges.iter().enumerate() {
        if materials
            .binary_search_by_key(&range.material_id, |material| material.id)
            .is_err()
        {
            return Err(MaterialSectionError::UnknownMaterial {
                range: index,
                id: range.material_id,
            });
        }
        let Some(end) = range.first_index.checked_add(range.index_count) else {
            return Err(MaterialSectionError::InvalidRange { index });
        };
        if range.index_count == 0
            || range.first_index % 3 != 0
            || range.index_count % 3 != 0
            || end > mesh_index_count
        {
            return Err(MaterialSectionError::InvalidRange { index });
        }
        if range.first_index != previous_end {
            return Err(MaterialSectionError::RangeOrder { index });
        }
        previous_end = end;
    }
    if previous_end != mesh_index_count {
        return Err(MaterialSectionError::RangeOrder {
            index: ranges.len(),
        });
    }
    Ok(())
}

pub(super) fn encode_material(material: &MaterialRecord, out: &mut [u8]) {
    out[..8].copy_from_slice(&material.id.to_le_bytes());
    out[8] = material.shading_model as u8;
    out[9] = material.opacity_mode as u8;
    let flags = u16::from(material.double_sided) * FLAG_DOUBLE_SIDED
        | u16::from(material.casts_shadow) * FLAG_CASTS_SHADOW
        | u16::from(material.receives_shadow) * FLAG_RECEIVES_SHADOW
        | u16::from(material.pickable) * FLAG_PICKABLE
        | u16::from(material.multiply_vertex_color) * FLAG_VERTEX_COLOR_MULTIPLY;
    out[10..12].copy_from_slice(&flags.to_le_bytes());
    let mut offset = 12;
    for value in material.base_color.into_iter().chain(material.emissive) {
        write_f32(out, &mut offset, value);
    }
    for value in [
        material.metallic,
        material.roughness,
        material.specular,
        material.normal_scale,
        material.occlusion_strength,
        material.alpha_cutoff,
    ] {
        write_f32(out, &mut offset, value);
    }
    for digest in [
        material.base_color_texture,
        material.normal_texture,
        material.metallic_roughness_texture,
        material.occlusion_texture,
        material.emissive_texture,
        material.stylized_ramp_texture,
    ] {
        out[offset..offset + 32].copy_from_slice(&digest);
        offset += 32;
    }
}

pub(super) fn decode_material(
    bytes: &[u8],
    index: usize,
) -> Result<MaterialRecord, MaterialSectionError> {
    let model =
        ShadingModel::from_raw(bytes[8]).ok_or(MaterialSectionError::InvalidModel(bytes[8]))?;
    let opacity =
        OpacityMode::from_raw(bytes[9]).ok_or(MaterialSectionError::InvalidOpacity(bytes[9]))?;
    let flags = read_u16(bytes, 10);
    if flags & !KNOWN_FLAGS != 0 {
        return Err(MaterialSectionError::UnknownFlags(flags));
    }
    let mut offset = 12;
    let mut values = [0.0f32; 13];
    for value in &mut values {
        *value = read_f32(bytes, offset);
        offset += 4;
    }
    let record = MaterialRecord {
        id: read_u64(bytes, 0),
        shading_model: model,
        opacity_mode: opacity,
        double_sided: flags & FLAG_DOUBLE_SIDED != 0,
        casts_shadow: flags & FLAG_CASTS_SHADOW != 0,
        receives_shadow: flags & FLAG_RECEIVES_SHADOW != 0,
        pickable: flags & FLAG_PICKABLE != 0,
        multiply_vertex_color: flags & FLAG_VERTEX_COLOR_MULTIPLY != 0,
        base_color: values[..4].try_into().expect("fixed material base color"),
        emissive: values[4..7].try_into().expect("fixed material emissive"),
        metallic: values[7],
        roughness: values[8],
        specular: values[9],
        normal_scale: values[10],
        occlusion_strength: values[11],
        alpha_cutoff: values[12],
        base_color_texture: bytes[64..96].try_into().expect("checked record bounds"),
        normal_texture: bytes[96..128].try_into().expect("checked record bounds"),
        metallic_roughness_texture: bytes[128..160].try_into().expect("checked record bounds"),
        occlusion_texture: bytes[160..192].try_into().expect("checked record bounds"),
        emissive_texture: bytes[192..224].try_into().expect("checked record bounds"),
        stylized_ramp_texture: bytes[224..256].try_into().expect("checked record bounds"),
        texture_transforms: [TextureTransform::IDENTITY; 6],
        texture_samplers: [TextureSampler::GLTF_DEFAULT; 6],
    };
    if record.id == 0 {
        return Err(MaterialSectionError::InvalidMaterial { index });
    }
    Ok(record)
}

fn write_f32(out: &mut [u8], offset: &mut usize, value: f32) {
    out[*offset..*offset + 4].copy_from_slice(&value.to_le_bytes());
    *offset += 4;
}
pub(super) fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
pub(super) fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked record bounds"),
    )
}
pub(super) fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("checked record bounds"),
    )
}
fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked record bounds"),
    )
}

#[cfg(test)]
#[path = "material_section_tests.rs"]
mod tests;
