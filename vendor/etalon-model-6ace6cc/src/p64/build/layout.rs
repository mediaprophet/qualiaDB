use super::{BuildInput, BuildProfile, TensorDescriptor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutProfile {
    Standard(BuildProfile),
    /// Legacy FFN rewrites page-align every blob and retain raw schedule bytes.
    FfnLegacy,
}

#[derive(Clone, Copy, Debug)]
pub struct TensorLayout<'a> {
    pub name: &'a [u8],
    pub role_id: u16,
    pub dtype: u16,
    pub manifold_idx: u32,
    pub rank: u32,
    pub dimensions: [u32; 4],
    pub source_offset: u64,
    pub source_name_hash: u64,
    pub alt_dtype: u16,
    pub precision_views_mask: u16,
    pub alt_blob_offset: u32,
    pub blob_len: u64,
}

impl<'a> From<TensorDescriptor<'a>> for TensorLayout<'a> {
    fn from(value: TensorDescriptor<'a>) -> Self {
        Self {
            name: value.name,
            role_id: value.role_id,
            dtype: value.dtype,
            manifold_idx: value.manifold_idx,
            rank: value.rank,
            dimensions: value.dimensions,
            source_offset: value.source_offset,
            source_name_hash: value.source_name_hash,
            alt_dtype: value.alt_dtype,
            precision_views_mask: value.precision_views_mask,
            alt_blob_offset: value.alt_blob_offset,
            blob_len: value.blob.len() as u64,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutInput<'a> {
    pub profile: LayoutProfile,
    pub page_log2: u16,
    pub flags: u16,
    pub header_reserved: [u8; 20],
    pub hparams_bytes: &'a [u8; 64],
    pub manifold_bytes: &'a [u8],
    pub tokenizer_bytes: &'a [u8],
    /// Empty for standard profiles; exactly n_layer * 64 bytes for FfnLegacy.
    pub schedule_bytes: &'a [u8],
    pub tensors: &'a [TensorLayout<'a>],
}

#[derive(Clone, Copy, Debug)]
enum Tensors<'a> {
    Prepared(&'a [TensorDescriptor<'a>]),
    Sized(&'a [TensorLayout<'a>]),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PlanInput<'a> {
    pub profile: LayoutProfile,
    pub page_log2: u16,
    pub flags: u16,
    pub header_reserved: [u8; 20],
    pub hparams_bytes: &'a [u8; 64],
    pub manifold_bytes: &'a [u8],
    pub tokenizer_bytes: &'a [u8],
    pub schedule_bytes: &'a [u8],
    tensors: Tensors<'a>,
}

impl<'a> PlanInput<'a> {
    pub fn count(&self) -> usize {
        match self.tensors {
            Tensors::Prepared(v) => v.len(),
            Tensors::Sized(v) => v.len(),
        }
    }
    pub fn tensor(&self, index: usize) -> TensorLayout<'a> {
        match self.tensors {
            Tensors::Prepared(v) => v[index].into(),
            Tensors::Sized(v) => v[index],
        }
    }
    pub fn layers(&self) -> u32 {
        u32::from_le_bytes(self.hparams_bytes[..4].try_into().unwrap())
    }
    pub fn has_schedule(&self) -> bool {
        matches!(
            self.profile,
            LayoutProfile::FfnLegacy | LayoutProfile::Standard(BuildProfile::GgufLayerPacked)
        )
    }
}

impl<'a> From<BuildInput<'a>> for PlanInput<'a> {
    fn from(v: BuildInput<'a>) -> Self {
        Self {
            profile: LayoutProfile::Standard(v.profile),
            page_log2: v.page_log2,
            flags: v.flags,
            header_reserved: v.header_reserved,
            hparams_bytes: v.hparams_bytes,
            manifold_bytes: v.manifold_bytes,
            tokenizer_bytes: v.tokenizer_bytes,
            schedule_bytes: &[],
            tensors: Tensors::Prepared(v.tensors),
        }
    }
}

impl<'a> From<LayoutInput<'a>> for PlanInput<'a> {
    fn from(v: LayoutInput<'a>) -> Self {
        Self {
            profile: v.profile,
            page_log2: v.page_log2,
            flags: v.flags,
            header_reserved: v.header_reserved,
            hparams_bytes: v.hparams_bytes,
            manifold_bytes: v.manifold_bytes,
            tokenizer_bytes: v.tokenizer_bytes,
            schedule_bytes: v.schedule_bytes,
            tensors: Tensors::Sized(v.tensors),
        }
    }
}
