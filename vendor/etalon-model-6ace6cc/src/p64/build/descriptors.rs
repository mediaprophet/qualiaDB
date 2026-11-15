#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildProfile {
    GgufLayerPacked,
    Safetensor,
    Vision,
}

/// The caller supplies descriptors in final output order. Names, including embedded
/// NUL bytes, are copied unchanged and followed by a terminating NUL.
#[derive(Clone, Copy, Debug)]
pub struct TensorDescriptor<'a> {
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
    /// Opaque legacy metadata; no alternate payload is synthesized or validated.
    pub alt_blob_offset: u32,
    pub blob: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub struct BuildInput<'a> {
    pub profile: BuildProfile,
    /// Zero selects the historical 16 KiB default. Otherwise 8 through 30.
    pub page_log2: u16,
    /// Structural flag bits must match the profile; other bits are preserved.
    pub flags: u16,
    pub header_reserved: [u8; 20],
    pub hparams_bytes: &'a [u8; 64],
    pub manifold_bytes: &'a [u8],
    pub tokenizer_bytes: &'a [u8],
    pub tensors: &'a [TensorDescriptor<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildBudget {
    pub output_bytes: usize,
    /// Charges the used entry scratch (tensor count times 64), not spare capacity.
    /// Fixed stack state and immutable caller inputs are excluded.
    pub scratch_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildError {
    EmptyTensors,
    InvalidPageSize,
    ProfileFlags,
    ProfileSections,
    ManifoldCount,
    InvalidRank(usize),
    InvalidManifold(usize),
    ArithmeticOverflow,
    OffsetLimit,
    ScratchCapacity {
        required_entries: usize,
        available_entries: usize,
    },
    ScratchBudget {
        required_bytes: usize,
        available_bytes: usize,
    },
    OutputBudget {
        required_bytes: usize,
        available_bytes: usize,
    },
    OutputCapacity {
        required_bytes: usize,
        available_bytes: usize,
    },
}

impl core::fmt::Display for BuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "P64 build error: {self:?}")
    }
}

impl core::error::Error for BuildError {}
