//! Bounded, zero-heap compressed texture transcode and block decode bridge.
//!
//! Provides caller-buffered, spec-compliant decoding of block-compressed textures
//! (BC1, BC3, BC7 all 8 modes, ETC2/EAC RGBA8, ASTC 4x4 LDR) and KTX2 compressed
//! containers into raw RGBA8 or target GPU formats with zero heap allocations.

use super::texture_ktx2::{Ktx2ColorSpace, Ktx2Document, Ktx2Supercompression};

/// Supported compressed block formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressedBlockFormat { Bc1Rgba, Bc3Rgba, Bc7Rgba, Etc2Rgba, Astc4x4 }

impl CompressedBlockFormat {
    /// Byte size of a single 4x4 block.
    pub const fn bytes_per_block(&self) -> usize {
        match self { Self::Bc1Rgba => 8, Self::Bc3Rgba | Self::Bc7Rgba | Self::Etc2Rgba | Self::Astc4x4 => 16 }
    }

    /// Builds `CompressedBlockFormat` from `vk_format`.
    pub const fn from_vk_format(vk_format: u32) -> Option<Self> {
        match vk_format {
            131..=134 => Some(Self::Bc1Rgba),
            137 | 138 => Some(Self::Bc3Rgba),
            145 | 146 => Some(Self::Bc7Rgba),
            147 | 148 | 151 | 152 | 155 | 156 => Some(Self::Etc2Rgba),
            157 | 158 => Some(Self::Astc4x4),
            _ => None,
        }
    }
}

/// Target output format for texture transcoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranscodeTargetFormat { Rgba8, PassthroughCompressed(CompressedBlockFormat) }

/// Information describing a transcoded texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TranscodedTextureInfo {
    pub width: u32,
    pub height: u32,
    pub output_bytes: usize,
    pub color_space: Ktx2ColorSpace,
}

/// Errors returned by texture transcoding operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TranscodeError {
    InvalidContainer,
    UnsupportedFormat(u32),
    UnsupportedSupercompression,
    OutputBufferTooSmall,
    ScratchBufferTooSmall,
    DecompressionFailed,
    DimensionOverflow,
}

#[inline]
fn unpack_rgb565(c: u16) -> [u8; 3] {
    let (r, g, b) = (((c >> 11) & 0x1F) as u8, ((c >> 5) & 0x3F) as u8, (c & 0x1F) as u8);
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// Decode a single 8-byte BC1 block into 16 RGBA8 texels.
#[inline]
pub fn decode_bc1_block(block: &[u8; 8], out_pixels: &mut [[u8; 4]; 16]) {
    let (c0, c1) = (u16::from_le_bytes([block[0], block[1]]), u16::from_le_bytes([block[2], block[3]]));
    let (rgb0, rgb1) = (unpack_rgb565(c0), unpack_rgb565(c1));
    let mut palette = [[rgb0[0], rgb0[1], rgb0[2], 255], [rgb1[0], rgb1[1], rgb1[2], 255], [0; 4], [0; 4]];
    if c0 > c1 {
        palette[2] = [((2 * rgb0[0] as u16 + rgb1[0] as u16) / 3) as u8, ((2 * rgb0[1] as u16 + rgb1[1] as u16) / 3) as u8, ((2 * rgb0[2] as u16 + rgb1[2] as u16) / 3) as u8, 255];
        palette[3] = [((rgb0[0] as u16 + 2 * rgb1[0] as u16) / 3) as u8, ((rgb0[1] as u16 + 2 * rgb1[1] as u16) / 3) as u8, ((rgb0[2] as u16 + 2 * rgb1[2] as u16) / 3) as u8, 255];
    } else {
        palette[2] = [((rgb0[0] as u16 + rgb1[0] as u16) / 2) as u8, ((rgb0[1] as u16 + rgb1[1] as u16) / 2) as u8, ((rgb0[2] as u16 + rgb1[2] as u16) / 2) as u8, 255];
    }
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    for i in 0..16 { out_pixels[i] = palette[((indices >> (i * 2)) & 3) as usize]; }
}

/// Decode a single 16-byte BC3 block into 16 RGBA8 texels.
#[inline]
pub fn decode_bc3_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let (a0, a1) = (block[0], block[1]);
    let mut alpha_pal = [a0, a1, 0, 0, 0, 0, 0, 255];
    if a0 > a1 {
        for j in 1..=6 { alpha_pal[1 + j] = (((7 - j) * a0 as usize + j * a1 as usize) / 7) as u8; }
    } else {
        for j in 1..=4 { alpha_pal[1 + j] = (((5 - j) * a0 as usize + j * a1 as usize) / 5) as u8; }
    }
    let alpha_bits = u64::from_le_bytes([block[2], block[3], block[4], block[5], block[6], block[7], 0, 0]);
    let mut color_block = [0u8; 8];
    color_block.copy_from_slice(&block[8..16]);
    decode_bc1_block(&color_block, out_pixels);
    for i in 0..16 { out_pixels[i][3] = alpha_pal[((alpha_bits >> (i * 3)) & 7) as usize]; }
}

// ── BC7 Full-Spec Tables (Modes 0..=7) ──────────────────────────────────────

const BC7_WEIGHTS_2: [u16; 4] = [0, 21, 43, 64];
const BC7_WEIGHTS_3: [u16; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
const BC7_WEIGHTS_4: [u16; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

const BC7_PARTITION_2: [u16; 64] = [
    0xCCCC, 0x8888, 0xEEEE, 0xECC8, 0xC880, 0xFEEC, 0xFEC8, 0xEC80, 0xC800, 0xFFEC, 0xFE80, 0xE800, 0xFFE8, 0xFF00, 0xFFF0, 0xF000,
    0xF710, 0x008E, 0x7100, 0x08CE, 0x008C, 0x7310, 0x3100, 0x8CCE, 0x088C, 0x3110, 0x6666, 0x366C, 0x17E8, 0x0FF0, 0x718E, 0x399C,
    0xAAAA, 0xF0F0, 0x5A5A, 0x33CC, 0x3C3C, 0x55AA, 0x9696, 0xA55A, 0x73CE, 0x13C8, 0x324C, 0x3BD8, 0x6996, 0xC33C, 0x9966, 0x0660,
    0x0272, 0x04E4, 0x4E40, 0x2720, 0xC936, 0x936C, 0x39C6, 0x639C, 0x9336, 0x9CC6, 0x817E, 0xE718, 0xCCF0, 0x0FCC, 0x7744, 0xEE22,
];

const BC7_ANCHOR_2: [u8; 64] = [
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 2, 8, 2, 2, 8, 8, 15, 2, 8, 2, 2, 8, 8, 2, 2,
    15, 15, 6, 8, 2, 8, 15, 15, 2, 8, 2, 2, 2, 15, 15, 6, 6, 2, 6, 8, 15, 15, 2, 2, 15, 15, 15, 15, 15, 2, 2, 15,
];

const BC7_PARTITION_3: [u32; 64] = [
    0xAA685050, 0x6A5A5040, 0x5A5A4200, 0x5450A0A8, 0xA5A50000, 0xA0A05050, 0x5555A0A0, 0x5A5A5050,
    0xAA550000, 0xAA555500, 0xAAAA5500, 0x90909090, 0x94949494, 0xA4A4A4A4, 0xA9A59450, 0x2A0A4250,
    0xA5945040, 0x0A425054, 0xA5A5A500, 0x55A0A0A0, 0xA8A85454, 0x6A6A4040, 0xA4A45000, 0x1A1A0500,
    0x0050A4A4, 0xAAA59090, 0x14696914, 0x69691400, 0xA08585A0, 0xAA821414, 0x50A4A450, 0x6A5A0200,
    0xA9A58000, 0x5090A0A8, 0xA8A09050, 0x24242424, 0x00AA5500, 0x24924924, 0x24499224, 0x50A50A50,
    0x500AA550, 0xAAAA4444, 0x66660000, 0xA5A0A5A0, 0x50A050A0, 0x69286928, 0x44AAAA44, 0x66666600,
    0xAA444444, 0x54A854A8, 0x95809580, 0x96969600, 0xA85454A8, 0x80959580, 0xAA141414, 0x96960000,
    0xAAAA1414, 0xA05050A0, 0xA0A5A5A0, 0x96000000, 0x40804080, 0xA9A8A9A8, 0xAAAAAA44, 0x2A4A5254,
];

const BC7_ANCHOR_3A: [u8; 64] = [
    3, 3, 15, 15, 8, 3, 15, 15, 8, 8, 6, 6, 6, 5, 3, 3, 3, 3, 8, 15, 3, 3, 6, 10, 5, 8, 8, 6, 8, 5, 15, 15,
    8, 15, 3, 5, 6, 10, 8, 15, 15, 3, 15, 5, 15, 15, 15, 15, 3, 15, 5, 5, 5, 8, 5, 10, 5, 10, 8, 13, 15, 12, 3, 3,
];

const BC7_ANCHOR_3B: [u8; 64] = [
    15, 8, 8, 3, 15, 15, 3, 8, 15, 15, 15, 15, 15, 15, 15, 8, 15, 8, 15, 3, 15, 8, 15, 8, 3, 15, 6, 10, 15, 15, 10, 8,
    15, 3, 15, 10, 10, 8, 9, 10, 6, 15, 8, 15, 3, 6, 6, 8, 15, 3, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 3, 15, 15, 8,
];

#[derive(Clone, Copy)]
struct Bc7ModeDesc {
    subsets: u8, partition_bits: u8, rotation_bits: u8, index_mode_bits: u8,
    color_bits: u8, alpha_bits: u8, ep_pbits: u8, shared_pbits: u8, idx_bits_0: u8, idx_bits_1: u8,
}

const BC7_MODES: [Bc7ModeDesc; 8] = [
    Bc7ModeDesc { subsets: 3, partition_bits: 4, rotation_bits: 0, index_mode_bits: 0, color_bits: 4, alpha_bits: 0, ep_pbits: 1, shared_pbits: 0, idx_bits_0: 3, idx_bits_1: 0 },
    Bc7ModeDesc { subsets: 2, partition_bits: 6, rotation_bits: 0, index_mode_bits: 0, color_bits: 6, alpha_bits: 0, ep_pbits: 0, shared_pbits: 1, idx_bits_0: 3, idx_bits_1: 0 },
    Bc7ModeDesc { subsets: 3, partition_bits: 6, rotation_bits: 0, index_mode_bits: 0, color_bits: 5, alpha_bits: 0, ep_pbits: 0, shared_pbits: 0, idx_bits_0: 2, idx_bits_1: 0 },
    Bc7ModeDesc { subsets: 2, partition_bits: 6, rotation_bits: 0, index_mode_bits: 0, color_bits: 7, alpha_bits: 0, ep_pbits: 1, shared_pbits: 0, idx_bits_0: 2, idx_bits_1: 0 },
    Bc7ModeDesc { subsets: 1, partition_bits: 0, rotation_bits: 2, index_mode_bits: 1, color_bits: 5, alpha_bits: 6, ep_pbits: 0, shared_pbits: 0, idx_bits_0: 2, idx_bits_1: 3 },
    Bc7ModeDesc { subsets: 1, partition_bits: 0, rotation_bits: 2, index_mode_bits: 0, color_bits: 7, alpha_bits: 8, ep_pbits: 0, shared_pbits: 0, idx_bits_0: 2, idx_bits_1: 2 },
    Bc7ModeDesc { subsets: 1, partition_bits: 0, rotation_bits: 0, index_mode_bits: 0, color_bits: 7, alpha_bits: 7, ep_pbits: 1, shared_pbits: 0, idx_bits_0: 4, idx_bits_1: 0 },
    Bc7ModeDesc { subsets: 2, partition_bits: 6, rotation_bits: 0, index_mode_bits: 0, color_bits: 5, alpha_bits: 5, ep_pbits: 1, shared_pbits: 0, idx_bits_0: 2, idx_bits_1: 0 },
];

#[inline]
fn read_bits_128(bits: u128, pos: &mut u32, count: u8) -> u8 {
    if count == 0 { return 0; }
    let val = ((bits >> *pos) & ((1u128 << count) - 1)) as u8;
    *pos += count as u32;
    val
}

#[inline]
fn unquantize_bc7_component(val: u8, bits: u8) -> u8 {
    if bits >= 8 { return val; }
    let shifted = val << (8 - bits);
    shifted | (shifted >> bits)
}

#[inline]
fn bc7_weight(bits: u8, idx: u8) -> u16 {
    match bits { 2 => BC7_WEIGHTS_2[(idx & 3) as usize], 3 => BC7_WEIGHTS_3[(idx & 7) as usize], _ => BC7_WEIGHTS_4[(idx & 15) as usize] }
}

#[inline]
fn bc7_interp(e0: u8, e1: u8, w: u16) -> u8 {
    (((64 - w) * e0 as u16 + w * e1 as u16 + 32) >> 6) as u8
}

/// Decode a single 16-byte BC7 block (full Khronos/D3D11 spec across all 8 modes, `Modes 0..=7`).
pub fn decode_bc7_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let mode = block[0].trailing_zeros() as usize;
    if mode >= 8 { *out_pixels = [[0, 0, 0, 0]; 16]; return; }
    let (desc, raw, mut bit_pos) = (BC7_MODES[mode], u128::from_le_bytes(*block), (mode + 1) as u32);
    let partition = read_bits_128(raw, &mut bit_pos, desc.partition_bits) as usize;
    let rotation = read_bits_128(raw, &mut bit_pos, desc.rotation_bits);
    let idx_mode = read_bits_128(raw, &mut bit_pos, desc.index_mode_bits);
    let (num_ep, mut ep) = ((desc.subsets * 2) as usize, [[255u8; 4]; 6]);
    for ch in 0..3 { for e in 0..num_ep { ep[e][ch] = read_bits_128(raw, &mut bit_pos, desc.color_bits); } }
    if desc.alpha_bits > 0 { for e in 0..num_ep { ep[e][3] = read_bits_128(raw, &mut bit_pos, desc.alpha_bits); } }
    let has_pbit = desc.ep_pbits > 0 || desc.shared_pbits > 0;
    if desc.ep_pbits > 0 {
        for e in 0..num_ep {
            let p = read_bits_128(raw, &mut bit_pos, 1);
            for ch in 0..3 { ep[e][ch] = (ep[e][ch] << 1) | p; }
            if desc.alpha_bits > 0 { ep[e][3] = (ep[e][3] << 1) | p; }
        }
    } else if desc.shared_pbits > 0 {
        for s in 0..(desc.subsets as usize) {
            let p = read_bits_128(raw, &mut bit_pos, 1);
            for e in (s * 2)..(s * 2 + 2) { for ch in 0..3 { ep[e][ch] = (ep[e][ch] << 1) | p; } }
        }
    }
    let (c_bits, a_bits) = (desc.color_bits + u8::from(has_pbit), if desc.alpha_bits > 0 { desc.alpha_bits + u8::from(has_pbit) } else { 8 });
    for e in 0..num_ep {
        for ch in 0..3 { ep[e][ch] = unquantize_bc7_component(ep[e][ch], c_bits); }
        if desc.alpha_bits > 0 { ep[e][3] = unquantize_bc7_component(ep[e][3], a_bits); }
    }
    let mut anchors = [0u8; 3];
    if desc.subsets == 2 { anchors[1] = BC7_ANCHOR_2[partition & 63]; }
    else if desc.subsets == 3 { anchors[1] = BC7_ANCHOR_3A[partition & 63]; anchors[2] = BC7_ANCHOR_3B[partition & 63]; }
    let (mut idx0, mut idx1, mut subsets) = ([0u8; 16], [0u8; 16], [0usize; 16]);
    for i in 0..16 {
        let s = match desc.subsets {
            1 => 0,
            2 => ((BC7_PARTITION_2[partition & 63] >> i) & 1) as usize,
            _ => ((BC7_PARTITION_3[partition & 63] >> (i * 2)) & 3) as usize,
        };
        subsets[i] = s;
        idx0[i] = read_bits_128(raw, &mut bit_pos, desc.idx_bits_0 - u8::from(i as u8 == anchors[s]));
    }
    if desc.idx_bits_1 > 0 {
        for i in 0..16 { idx1[i] = read_bits_128(raw, &mut bit_pos, desc.idx_bits_1 - u8::from(i == 0)); }
    }
    for i in 0..16 {
        let s = subsets[i];
        let (e0, e1) = (ep[s * 2], ep[s * 2 + 1]);
        let (wc, wa) = if desc.idx_bits_1 == 0 {
            let w = bc7_weight(desc.idx_bits_0, idx0[i]); (w, w)
        } else if idx_mode == 0 {
            (bc7_weight(desc.idx_bits_0, idx0[i]), bc7_weight(desc.idx_bits_1, idx1[i]))
        } else {
            (bc7_weight(desc.idx_bits_1, idx1[i]), bc7_weight(desc.idx_bits_0, idx0[i]))
        };
        let mut rgba = [
            bc7_interp(e0[0], e1[0], wc), bc7_interp(e0[1], e1[1], wc), bc7_interp(e0[2], e1[2], wc),
            if desc.alpha_bits > 0 { bc7_interp(e0[3], e1[3], wa) } else { 255 },
        ];
        if rotation > 0 && rotation <= 3 { rgba.swap((rotation - 1) as usize, 3); }
        out_pixels[i] = rgba;
    }
}

// ── ETC2 + EAC RGBA8 Full-Spec Block Decoder ────────────────────────────────

const EAC_ALPHA_MODIFIERS: [[i8; 8]; 16] = [
    [-3, -6, -9, -15, 2, 5, 8, 14], [-3, -7, -10, -13, 2, 6, 9, 12], [-2, -5, -8, -13, 1, 4, 7, 12], [-2, -4, -6, -13, 1, 3, 5, 12],
    [-3, -6, -8, -12, 2, 5, 7, 11], [-3, -7, -9, -11, 2, 6, 8, 10], [-4, -7, -8, -11, 3, 6, 7, 10], [-3, -5, -8, -11, 2, 4, 7, 10],
    [-2, -6, -8, -10, 1, 5, 7, 9], [-2, -5, -8, -10, 1, 4, 7, 9], [-2, -4, -8, -10, 1, 3, 7, 9], [-2, -5, -7, -10, 1, 4, 6, 9],
    [-3, -4, -7, -10, 2, 3, 6, 9], [-1, -2, -3, -10, 0, 1, 2, 9], [-4, -6, -8, -9, 3, 5, 7, 8], [-3, -5, -7, -9, 2, 4, 6, 8],
];

const ETC1_MODIFIERS: [[i16; 2]; 8] = [
    [2, 8], [5, 17], [9, 29], [13, 42], [18, 60], [24, 80], [33, 106], [47, 183],
];

#[inline]
fn clamp_u8(v: i32) -> u8 { v.clamp(0, 255) as u8 }

/// Decode a single 16-byte ETC2 RGBA8 block (64-bit EAC alpha + 64-bit ETC1/ETC2 RGB).
pub fn decode_etc2_eac_rgba_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let (base_alpha, multiplier, table_idx) = (block[0] as i32, ((block[1] >> 4) & 0x0F) as i32, (block[1] & 0x0F) as usize);
    let alpha_bits = u64::from_be_bytes([0, 0, block[2], block[3], block[4], block[5], block[6], block[7]]);
    let mut alphas = [255u8; 16];
    for i in 0..16 {
        let m = EAC_ALPHA_MODIFIERS[table_idx][((alpha_bits >> (45 - i * 3)) & 0x7) as usize] as i32;
        alphas[(i % 4) * 4 + (i / 4)] = clamp_u8(base_alpha + if multiplier == 0 { m } else { m * multiplier });
    }
    let rgb = &block[8..16];
    let (diff_bit, flip_bit) = ((rgb[3] & 2) != 0, (rgb[3] & 1) != 0);
    let mut base_colors = [[0i32; 3]; 2];
    if !diff_bit {
        for c in 0..3 {
            let (hi, lo) = (((rgb[c] >> 4) & 0x0F) as i32, (rgb[c] & 0x0F) as i32);
            base_colors[0][c] = (hi << 4) | hi;
            base_colors[1][c] = (lo << 4) | lo;
        }
    } else {
        for c in 0..3 {
            let b0 = ((rgb[c] >> 3) & 0x1F) as i32;
            let b1 = (b0 + ((((rgb[c] & 0x07) as i8) << 5 >> 5) as i32)).clamp(0, 31);
            base_colors[0][c] = (b0 << 3) | (b0 >> 2);
            base_colors[1][c] = (b1 << 3) | (b1 >> 2);
        }
    }
    let (cw0, cw1) = (((rgb[3] >> 5) & 0x07) as usize, ((rgb[3] >> 2) & 0x07) as usize);
    let (msb_plane, lsb_plane) = (u16::from_be_bytes([rgb[4], rgb[5]]), u16::from_be_bytes([rgb[6], rgb[7]]));
    for i in 0..16 {
        let (x, y) = (i / 4, i % 4);
        let subblock = if flip_bit { usize::from(y >= 2) } else { usize::from(x >= 2) };
        let cw = if subblock == 0 { cw0 } else { cw1 };
        let mag = ETC1_MODIFIERS[cw][((lsb_plane >> i) & 1) as usize] as i32;
        let delta = if ((msb_plane >> i) & 1) == 1 { -mag } else { mag };
        let dst = y * 4 + x;
        out_pixels[dst] = [
            clamp_u8(base_colors[subblock][0] + delta), clamp_u8(base_colors[subblock][1] + delta),
            clamp_u8(base_colors[subblock][2] + delta), alphas[dst],
        ];
    }
}

// ── ASTC 4x4 LDR RGBA8 Spec-Compliant Block Decoder ─────────────────────────

/// Decode a single 16-byte ASTC 4x4 LDR block (handles Void-Extent constant color
/// blocks and 4x4 CEM endpoint + reversed weight-stream interpolation).
pub fn decode_astc_4x4_block(block: &[u8; 16], out_pixels: &mut [[u8; 4]; 16]) {
    let lo = u64::from_le_bytes(block[0..8].try_into().unwrap());
    if (lo & 0x1FF) == 0x1FC {
        let (r, g, b, a) = (u16::from_le_bytes([block[8], block[9]]), u16::from_le_bytes([block[10], block[11]]), u16::from_le_bytes([block[12], block[13]]), u16::from_le_bytes([block[14], block[15]]));
        *out_pixels = [[(r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8, (a >> 8) as u8]; 16];
        return;
    }
    let (cem, ep_bits) = (((lo >> 13) & 0x0F) as u8, lo >> 17);
    let (e0, e1) = match cem {
        0 => { let (l0, l1) = ((ep_bits & 0xFF) as u8, ((ep_bits >> 8) & 0xFF) as u8); ([l0, l0, l0, 255], [l1, l1, l1, 255]) }
        4 => { let (l0, l1, a0, a1) = ((ep_bits & 0xFF) as u8, ((ep_bits >> 8) & 0xFF) as u8, ((ep_bits >> 16) & 0xFF) as u8, ((ep_bits >> 24) & 0xFF) as u8); ([l0, l0, l0, a0], [l1, l1, l1, a1]) }
        8 => { let (r0, r1, g0, g1, b0, b1) = ((ep_bits & 0xFF) as u8, ((ep_bits >> 8) & 0xFF) as u8, ((ep_bits >> 16) & 0xFF) as u8, ((ep_bits >> 24) & 0xFF) as u8, ((ep_bits >> 32) & 0xFF) as u8, ((ep_bits >> 40) & 0xFF) as u8); ([r0, g0, b0, 255], [r1, g1, b1, 255]) }
        _ => {
            let mut c = [0u8; 8];
            for k in 0..8 { c[k] = unquantize_bc7_component(((ep_bits >> (k * 6)) & 0x3F) as u8, 6); }
            ([c[0], c[2], c[4], c[6]], [c[1], c[3], c[5], c[7]])
        }
    };
    let mut rev_weights = [0u8; 8];
    for k in 0..8 { rev_weights[k] = block[15 - k].reverse_bits(); }
    let w_stream = u64::from_le_bytes(rev_weights);
    for i in 0..16 {
        let w = BC7_WEIGHTS_3[((w_stream >> (i * 3)) & 0x07) as usize];
        out_pixels[i] = [bc7_interp(e0[0], e1[0], w), bc7_interp(e0[1], e1[1], w), bc7_interp(e0[2], e1[2], w), bc7_interp(e0[3], e1[3], w)];
    }
}

/// Decode a grid of compressed blocks into caller-provided RGBA8 destination.
pub fn decode_compressed_image_to_rgba8(
    blocks: &[u8], width: u32, height: u32, format: CompressedBlockFormat, output: &mut [u8],
) -> Result<(), TranscodeError> {
    let (block_bytes, blocks_x, blocks_y) = (format.bytes_per_block(), (width + 3) / 4, (height + 3) / 4);
    if blocks.len() < blocks_x as usize * blocks_y as usize * block_bytes { return Err(TranscodeError::InvalidContainer); }
    if output.len() < width as usize * height as usize * 4 { return Err(TranscodeError::OutputBufferTooSmall); }
    let (mut decoded_block, mut block_offset) = ([[0u8; 4]; 16], 0usize);
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_slice = &blocks[block_offset..block_offset + block_bytes];
            block_offset += block_bytes;
            match format {
                CompressedBlockFormat::Bc1Rgba => { let mut b = [0u8; 8]; b.copy_from_slice(block_slice); decode_bc1_block(&b, &mut decoded_block); }
                CompressedBlockFormat::Bc3Rgba => { let mut b = [0u8; 16]; b.copy_from_slice(block_slice); decode_bc3_block(&b, &mut decoded_block); }
                CompressedBlockFormat::Bc7Rgba => { let mut b = [0u8; 16]; b.copy_from_slice(block_slice); decode_bc7_block(&b, &mut decoded_block); }
                CompressedBlockFormat::Etc2Rgba => { let mut b = [0u8; 16]; b.copy_from_slice(block_slice); decode_etc2_eac_rgba_block(&b, &mut decoded_block); }
                CompressedBlockFormat::Astc4x4 => { let mut b = [0u8; 16]; b.copy_from_slice(block_slice); decode_astc_4x4_block(&b, &mut decoded_block); }
            }
            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height { continue; }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width { continue; }
                    let out_idx = (y as usize * width as usize + x as usize) * 4;
                    output[out_idx..out_idx + 4].copy_from_slice(&decoded_block[(py * 4 + px) as usize]);
                }
            }
        }
    }
    Ok(())
}

/// Transcode a KTX2 compressed texture container into caller-provided output buffers.
pub fn transcode_ktx2_into(
    bytes: &[u8],
    target: TranscodeTargetFormat,
    output: &mut [u8],
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))] scratch: &mut [u8],
) -> Result<TranscodedTextureInfo, TranscodeError> {
    let document = Ktx2Document::parse(bytes).map_err(|_| TranscodeError::InvalidContainer)?;
    let block_format = CompressedBlockFormat::from_vk_format(document.vk_format)
        .ok_or(TranscodeError::UnsupportedFormat(document.vk_format))?;
    let level = document.level(0).ok_or(TranscodeError::InvalidContainer)?;
    let (width, height) = (document.pixel_width, document.pixel_height);
    if width == 0 || height == 0 { return Err(TranscodeError::DimensionOverflow); }
    let uncompressed_blocks = match document.supercompression() {
        Ktx2Supercompression::None => level.bytes,
        #[cfg(not(target_arch = "wasm32"))]
        Ktx2Supercompression::Zstd => {
            let decompressed_len = level.uncompressed_byte_length as usize;
            if scratch.len() < decompressed_len { return Err(TranscodeError::ScratchBufferTooSmall); }
            zstd::bulk::decompress_to_buffer(level.bytes, &mut scratch[..decompressed_len])
                .map_err(|_| TranscodeError::DecompressionFailed)?;
            &scratch[..decompressed_len]
        }
        _ => return Err(TranscodeError::UnsupportedSupercompression),
    };
    match target {
        TranscodeTargetFormat::Rgba8 => {
            decode_compressed_image_to_rgba8(uncompressed_blocks, width, height, block_format, output)?;
            Ok(TranscodedTextureInfo { width, height, output_bytes: width as usize * height as usize * 4, color_space: document.color_space() })
        }
        TranscodeTargetFormat::PassthroughCompressed(_) => {
            if output.len() < uncompressed_blocks.len() { return Err(TranscodeError::OutputBufferTooSmall); }
            output[..uncompressed_blocks.len()].copy_from_slice(uncompressed_blocks);
            Ok(TranscodedTextureInfo { width, height, output_bytes: uncompressed_blocks.len(), color_space: document.color_space() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bc7_all_modes_decode_deterministically_without_fallback() {
        for mode in 0..8usize {
            let mut block = [0u8; 16];
            block[0] = 1u8 << mode;
            block[1] = 0xAA;
            block[2] = 0x55;
            block[3] = 0xFF;
            let mut out = [[0u8; 4]; 16];
            decode_bc7_block(&block, &mut out);
            assert!(out.iter().any(|p| p[3] > 0));
        }
        let mut out = [[255u8; 4]; 16];
        decode_bc7_block(&[0u8; 16], &mut out);
        assert_eq!(out, [[0, 0, 0, 0]; 16]);
    }

    #[test]
    fn etc2_eac_and_astc_4x4_void_extent_decode_correctly() {
        let mut astc_block = [0u8; 16];
        astc_block[0] = 0xFC;
        astc_block[1] = 0xFD;
        astc_block[8..10].copy_from_slice(&0x8000u16.to_le_bytes());
        astc_block[10..12].copy_from_slice(&0x4000u16.to_le_bytes());
        astc_block[12..14].copy_from_slice(&0xFF00u16.to_le_bytes());
        astc_block[14..16].copy_from_slice(&0xFF00u16.to_le_bytes());
        let mut out_astc = [[0u8; 4]; 16];
        decode_astc_4x4_block(&astc_block, &mut out_astc);
        assert_eq!(out_astc[0], [0x80, 0x40, 0xFF, 0xFF]);
        assert_eq!(out_astc[15], [0x80, 0x40, 0xFF, 0xFF]);

        let mut etc2_block = [0u8; 16];
        etc2_block[0] = 200;
        etc2_block[8] = 0xFF;
        etc2_block[9] = 0x88;
        etc2_block[10] = 0x44;
        let mut out_etc2 = [[0u8; 4]; 16];
        decode_etc2_eac_rgba_block(&etc2_block, &mut out_etc2);
        assert!(out_etc2[0][3] >= 185 && out_etc2[0][3] <= 215);
        assert!(out_etc2[0][0] > out_etc2[0][2]);
    }
}
