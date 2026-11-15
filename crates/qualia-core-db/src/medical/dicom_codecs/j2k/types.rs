//! Core ISO/IEC 15444-1 (J2K) & ISO/IEC 15444-15 (HTJ2K) marker types, error enum, and limits.
use super::super::encapsulated::EncapError;

pub const TS_J2K_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
pub const TS_J2K: &str = "1.2.840.10008.1.2.4.91";
pub const TS_HTJ2K_LOSSLESS: &str = "1.2.840.10008.1.2.4.201";
pub const TS_HTJ2K_RPCL: &str = "1.2.840.10008.1.2.4.202";
pub const TS_HTJ2K: &str = "1.2.840.10008.1.2.4.203";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum J2kError {
    Encap(EncapError),
    Unsupported(&'static str),
    OutputTooSmall,
    ByteLimit,
    FrameLimit,
    DimensionLimit,
    TileLimit,
    Overflow,
    CorruptStream(&'static str),
    DimensionMismatch,
    BitPrecisionMismatch,
    LosslessModeViolation,
}

impl From<EncapError> for J2kError {
    fn from(e: EncapError) -> Self {
        match e {
            EncapError::ByteLimit => Self::ByteLimit,
            EncapError::FrameLimit => Self::FrameLimit,
            EncapError::DimensionLimit => Self::DimensionLimit,
            EncapError::Overflow => Self::Overflow,
            EncapError::Unsupported(s) => Self::Unsupported(s),
            other => Self::Encap(other),
        }
    }
}
impl std::fmt::Display for J2kError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DICOM JPEG 2000: {self:?}")
    }
}
impl std::error::Error for J2kError {}

#[derive(Clone, Copy, Debug)]
pub struct J2kLimits {
    pub max_part10_bytes: usize,
    pub max_decoded_bytes: usize,
    pub max_compressed_bytes: usize,
    pub max_frames: u32,
    pub max_fragments: usize,
    pub max_width: u16,
    pub max_height: u16,
    pub max_tiles: usize,
    pub max_decomp_levels: u8,
}

impl Default for J2kLimits {
    fn default() -> Self {
        Self {
            max_part10_bytes: 128 * 1024 * 1024,
            max_decoded_bytes: 256 * 1024 * 1024,
            max_compressed_bytes: 64 * 1024 * 1024,
            max_frames: 4096,
            max_fragments: 16384,
            max_width: 16384,
            max_height: 16384,
            max_tiles: 4096,
            max_decomp_levels: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressionOrder {
    Lrcp,
    Rlcp,
    Rpcl,
    Pcrl,
    Cprl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveletTransform {
    Irreversible97,
    Reversible53,
}

#[derive(Clone, Copy, Debug)]
pub struct ComponentSiz {
    pub precision: u8,
    pub signed: bool,
    pub xrsiz: u8,
    pub yrsiz: u8,
}

#[derive(Clone, Debug)]
pub struct SizHeader {
    pub xsiz: u32,
    pub ysiz: u32,
    pub xosiz: u32,
    pub yosiz: u32,
    pub xtsiz: u32,
    pub ytsiz: u32,
    pub xtosiz: u32,
    pub ytosiz: u32,
    pub components: Vec<ComponentSiz>,
}

impl SizHeader {
    pub fn width(&self) -> u32 {
        self.xsiz - self.xosiz
    }
    pub fn height(&self) -> u32 {
        self.ysiz - self.yosiz
    }
    pub fn num_tiles_x(&self) -> u32 {
        (self.xsiz - self.xtosiz).div_ceil(self.xtsiz)
    }
    pub fn num_tiles_y(&self) -> u32 {
        (self.ysiz - self.ytosiz).div_ceil(self.ytsiz)
    }
}

#[derive(Clone, Debug)]
pub struct CodingStyle {
    pub sop_markers: bool,
    pub eph_markers: bool,
    pub progression: ProgressionOrder,
    pub layers: u16,
    pub mct: u8,
    pub decomp_levels: u8,
    pub cblk_w_shift: u8,
    pub cblk_h_shift: u8,
    pub cblk_style: u8,
    pub transform: WaveletTransform,
    pub precinct_sizes: Vec<(u8, u8)>,
}

impl CodingStyle {
    pub fn uses_htj2k_only(&self) -> bool {
        (self.cblk_style & 0x40) != 0
    }
    #[allow(dead_code)]
    pub fn cblk_max_width(&self) -> u32 {
        1u32 << (self.cblk_w_shift + 2)
    }
    #[allow(dead_code)]
    pub fn cblk_max_height(&self) -> u32 {
        1u32 << (self.cblk_h_shift + 2)
    }
    pub fn precinct_exp(&self, res: usize) -> (u8, u8) {
        self.precinct_sizes.get(res).copied().unwrap_or((15, 15))
    }
}

#[derive(Clone, Debug)]
pub struct QuantizationParams {
    pub style: u8,
    pub guard_bits: u8,
    pub steps: Vec<(u8, u16)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CapHeader {
    pub pcap: u32,
    pub ccap_htj2k: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct TilePart<'a> {
    pub tile_idx: u16,
    pub part_idx: u8,
    pub num_parts: u8,
    pub cod: Option<CodingStyle>,
    pub qcd: Option<QuantizationParams>,
    pub bitstream: &'a [u8],
}

pub struct ParsedCodestream<'a> {
    pub siz: SizHeader,
    pub cod: CodingStyle,
    pub coc: Vec<(u16, CodingStyle)>,
    pub qcd: QuantizationParams,
    pub qcc: Vec<(u16, QuantizationParams)>,
    pub cap: CapHeader,
    pub tile_parts: Vec<TilePart<'a>>,
}
