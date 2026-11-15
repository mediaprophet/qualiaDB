pub const MAX_ARRAY_DEPTH: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionPolicy {
    V2V3,
    AtLeast2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseLimits {
    /// Absolute metadata byte limit, including the 24-byte header.
    pub max_metadata_bytes: usize,
    /// Outer array depth is one. Zero allows only non-array values.
    pub max_nesting_depth: usize,
    /// Cumulative KV entries, logical values, tensor entries and dimensions.
    /// Reading the fixed header itself consumes no work units.
    pub max_work: u64,
    pub versions: VersionPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: u32,
    pub tensor_count: u64,
    pub kv_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub kind: ParseErrorKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidLimits,
    Truncated,
    MetadataLimit,
    DepthLimit,
    WorkLimit,
    ArithmeticOverflow,
    UnknownValueType(u32),
    InvalidAlignment,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "GGUF metadata error at {}: {:?}", self.offset, self.kind)
    }
}
impl core::error::Error for ParseError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KvView<'a> {
    pub key: &'a [u8],
    pub value_type: u32,
    /// Raw value payload, including string lengths and array type/count prefixes.
    pub encoded_value: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TensorView<'a> {
    pub name: &'a [u8],
    pub rank: u32,
    pub ggml_type: u32,
    pub offset: u64,
    pub(super) dimensions: &'a [u8],
}

impl<'a> TensorView<'a> {
    pub fn dimension(&self, index: usize) -> Option<u64> {
        let start = index.checked_mul(8)?;
        Some(u64::from_le_bytes(
            self.dimensions
                .get(start..start.checked_add(8)?)?
                .try_into()
                .ok()?,
        ))
    }
    pub fn dimensions_bytes(&self) -> &'a [u8] {
        self.dimensions
    }
    pub fn name_hash(&self) -> u64 {
        fnv1a64(self.name)
    }
}

/// Full 64-bit FNV-1a over raw bytes. No semantic URI payload masking is applied.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}
