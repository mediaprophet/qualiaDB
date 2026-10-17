//! Wave-Native Spatio-Temporal Acoustic & Electromagnetic Wave-Field Container.
//!
//! Provides zero-Unicode semantic grounding for oral traditions, unwritten mother-tongues,
//! and sacred liturgical languages. Continuous acoustic and EMF physical wave streams are
//! directly addressable by 48-byte Super-Quins without intermediate Latin/Unicode text.
//!
//! Reference: `docs/plans/holo/Spatio-Temporal Linguistics Informatics Framework.md` §3

use bytemuck::{Pod, Zeroable};

/// Magic bytes: "QWAV" (0x5157_4156 in little-endian)
pub const WAVE_TENSOR_MAGIC: u32 = 0x5157_4156;

/// Carrier Modality Discriminators (16-bit header field)
pub const CARRIER_PHONETIC_ACOUSTIC: u16 = 0x0001;
pub const CARRIER_ORTHOGRAPHIC_TEXT: u16 = 0x0002;
pub const CARRIER_ICONOGRAPHIC_GLYPHIC: u16 = 0x0003;
pub const CARRIER_HERALDIC_BLAZON: u16 = 0x0004;
pub const CARRIER_SOMATIC_GESTURAL: u16 = 0x0005;
pub const CARRIER_CARTOGRAPHIC_CIVIC: u16 = 0x0006;
pub const CARRIER_SONIC_LITURGICAL: u16 = 0x0007;
pub const CARRIER_EMF_HYPERSPECTRAL: u16 = 0x0008;

/// Binary Header for Multi-Band Wave-Field Semantic Blocks in .q42 / HMC containers (64 Bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct WaveTensorBlockHeader {
    /// Magic identifier (0x5157_4156 "QWAV")
    pub magic: u32,
    /// Modality Mask (Sonic: 0x01, EMF: 0x02, Kinematic: 0x04)
    pub modality_mask: u16,
    /// Channel count (e.g. 1=mono, 2=stereo, 4=ambisonic, 16=array)
    pub channel_count: u16,
    /// Audio / sensor sampling rate in Hz (e.g. 96_000, 192_000)
    pub sample_rate_hz: u32,
    /// Bit depth per sample (e.g. 16, 24, 32)
    pub bit_depth: u16,
    /// Cyclical / Liturgical Season Mask (e.g. bits for lunar month, ceremonial season)
    pub cyclical_season_mask: u16,
    /// Custodial Scope DID Hash (hr:CustodialScope fiduciary gate)
    pub custodial_scope_id: u64,
    /// 4D Geodesic Bounding Box (H3 spatial cell hash + vertical AHD elevation)
    pub geodesic_h3_cell: u64,
    /// Bitemporal Valid Start Epoch in integer nanoseconds
    pub valid_start_nanos: u64,
    /// Bitemporal Valid End Epoch in integer nanoseconds
    pub valid_end_nanos: u64,
    /// Byte offsets for Bands [Band 0: Raw Wave, Band 1: Spectral-Prosodic, Band 2: Segment Markers, Band 3: NQuin Table]
    pub band_offsets: [u32; 4],
}

impl Default for WaveTensorBlockHeader {
    fn default() -> Self {
        Self {
            magic: WAVE_TENSOR_MAGIC,
            modality_mask: 0x01, // Sonic
            channel_count: 1,
            sample_rate_hz: 96_000,
            bit_depth: 24,
            cyclical_season_mask: 0,
            custodial_scope_id: 0,
            geodesic_h3_cell: 0,
            valid_start_nanos: 0,
            valid_end_nanos: u64::MAX,
            band_offsets: [64, 0, 0, 0],
        }
    }
}

impl WaveTensorBlockHeader {
    /// Validates header magic and internal sanity.
    #[inline]
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic != WAVE_TENSOR_MAGIC {
            return Err("Invalid WaveTensorBlock magic");
        }
        if self.sample_rate_hz == 0 {
            return Err("Sample rate cannot be zero");
        }
        if self.valid_start_nanos > self.valid_end_nanos {
            return Err("Valid start epoch exceeds valid end epoch");
        }
        Ok(())
    }
}

/// Time-frequency bounding box addressing continuous physical wave fields
/// without intermediate text or Unicode transcription.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeFrequencyBox {
    /// Start time offset in integer nanoseconds from block epoch
    pub start_nanos: u64,
    /// End time offset in integer nanoseconds from block epoch
    pub end_nanos: u64,
    /// Lower bound frequency in Hz (e.g. 50.0 Hz)
    pub min_freq_hz: f32,
    /// Upper bound frequency in Hz (e.g. 4000.0 Hz)
    pub max_freq_hz: f32,
    /// Harmonic overtone mask (e.g. fundamental F0 + F1..F3 formant pattern)
    pub harmonic_mask: u32,
    /// Alignment padding
    pub _padding: u32,
}

impl Default for TimeFrequencyBox {
    fn default() -> Self {
        Self {
            start_nanos: 0,
            end_nanos: 0,
            min_freq_hz: 0.0,
            max_freq_hz: 8000.0,
            harmonic_mask: 0x01,
            _padding: 0,
        }
    }
}

impl TimeFrequencyBox {
    /// Computes a deterministic, collision-resistant 60-bit concept token directly from the
    /// time-frequency bounds and acoustic signature, with zero intermediate Unicode text.
    #[inline]
    pub fn to_concept_token(&self, block_geodesic_h3: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash = (hash ^ block_geodesic_h3).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.start_nanos).wrapping_mul(0x100000001b3);
        hash = (hash ^ self.end_nanos).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.min_freq_hz.to_bits() as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.max_freq_hz.to_bits() as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ (self.harmonic_mask as u64)).wrapping_mul(0x100000001b3);
        // Truncate to 60 bits for Universal Super-Quin object payload
        hash & 0x0FFF_FFFF_FFFF_FFFF
    }

    /// Evaluates whether another time-frequency box overlaps in both time and frequency.
    #[inline]
    pub fn overlaps(&self, other: &TimeFrequencyBox) -> bool {
        let time_overlap = self.start_nanos <= other.end_nanos && self.end_nanos >= other.start_nanos;
        let freq_overlap = self.min_freq_hz <= other.max_freq_hz && self.max_freq_hz >= other.min_freq_hz;
        time_overlap && freq_overlap
    }

    /// Evaluates harmonic resonance similarity score (0.0 to 1.0) between two acoustic boxes.
    #[inline]
    pub fn resonance_score(&self, other: &TimeFrequencyBox) -> f32 {
        if !self.overlaps(other) {
            return 0.0;
        }
        let matching_bits = (self.harmonic_mask & other.harmonic_mask).count_ones();
        let total_bits = (self.harmonic_mask | other.harmonic_mask).count_ones().max(1);
        (matching_bits as f32) / (total_bits as f32)
    }
}
