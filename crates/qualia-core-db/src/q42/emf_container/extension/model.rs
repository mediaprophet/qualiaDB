//! Cold, bounded typed interpretation. EM cells are never redefined as RGB or audio samples.
use super::super::header::{EmfError,EmfContainerHeader};
use serde::{Deserialize, Serialize};

pub const EMF_EXTENSION_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmfExtensionLimits {
    pub max_file_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_audio_bytes: usize,
    pub max_tracks: usize,
}
impl EmfExtensionLimits {
    pub const fn conservative() -> Self {
        Self { max_file_bytes:16*1024*1024,max_metadata_bytes:64*1024,max_audio_bytes:16*1024*1024,max_tracks:16 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfRgbPrimaries { Rec709, Rec2020, DisplayP3 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum EmfWhitePoint { D65, D50 }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="lowercase")]
pub enum EmfTransfer { Linear, Srgb, Bt709, Pq, Hlg }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmfObserver { #[serde(rename="cie1931_2deg")] Cie1931TwoDegree }

/// This declares a derivative view; physical E/B/wavelength cells remain independent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag="model",rename_all="snake_case",deny_unknown_fields)]
pub enum EmfColourInterpretation {
    SpectralOnly,
    RgbPreview { primaries:EmfRgbPrimaries,white_point:EmfWhitePoint,transfer:EmfTransfer,
        observer:EmfObserver,reference_white_nits:f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfEpochScale {
    /// Relative to an authored origin, without a civil calendar claim.
    Relative,
    /// Seconds since 1970-01-01T00:00:00 UTC using POSIX leap-second exclusion.
    UnixPosix,
    /// Seconds since 1970-01-01T00:00:00 TAI.
    Tai,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfEpoch { pub scale:EmfEpochScale,pub seconds:i64,pub nanoseconds:u32 }
impl EmfEpoch {
    pub fn header_seconds(&self)->f64 { self.seconds as f64+self.nanoseconds as f64/1_000_000_000.0 }
    /// Exact epoch metadata is authoritative. The legacy f64 projection may differ
    /// by at most two ULPs from normal conversion rounding, including large epochs.
    pub fn matches_header_seconds(&self,value:f64)->bool {
        if !value.is_finite(){return false;}
        let expected=self.header_seconds();let magnitude=expected.abs();
        let next=f64::from_bits(magnitude.to_bits().saturating_add(1));
        let tolerance=((next-magnitude)*2.0).max(f64::MIN_POSITIVE);
        (value-expected).abs()<=tolerance
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfTimebase {
    pub epoch:EmfEpoch,
    pub ticks_per_second_num:u64,
    pub ticks_per_second_den:u64,
    pub slice_start_tick:i64,
    pub slice_step_ticks:u64,
}
impl EmfTimebase {
    pub fn slice_tick(&self,index:u32)->Result<i64,EmfError> {
        let delta=self.slice_step_ticks.checked_mul(u64::from(index)).ok_or(EmfError::ArithmeticOverflow)?;
        self.slice_start_tick.checked_add(i64::try_from(delta).map_err(|_|EmfError::ArithmeticOverflow)?)
            .ok_or(EmfError::ArithmeticOverflow)
    }
    pub fn validate(&self)->Result<(),EmfError> {
        if self.epoch.nanoseconds>=1_000_000_000||self.ticks_per_second_num==0||self.ticks_per_second_num>1_000_000_000_000
            ||self.ticks_per_second_den==0||self.ticks_per_second_den>1_000_000_000||self.slice_step_ticks==0 {
            return Err(EmfError::ExtensionMetadata);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfSpectralSemantics {
    /// wavelength_nm is physical; authored sigma is an explicitly legacy scalar.
    AuthoredWavelengthLegacySigma,
    /// Multiple emitters collapse to an amplitude-weighted wavelength centroid.
    /// This is not a sampled spectrum or a complete multi-band distribution.
    AmplitudeCentroidLegacySigma,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfFieldInterpretation {
    pub colour:EmfColourInterpretation,
    pub timebase:EmfTimebase,
    pub hilbert_order:u8,
    pub spectral_semantics:EmfSpectralSemantics,
}
impl EmfFieldInterpretation {
    /// Only an explicitly labelled wavelength-to-visible derivative is supported
    /// by the legacy preview. Declared RGB calibration/gamut needs its own owner.
    pub fn supports_legacy_visible_projection(&self)->bool {matches!(self.colour,EmfColourInterpretation::SpectralOnly)}
    pub(crate) fn validate_header(&self,header:&EmfContainerHeader)->Result<(),EmfError>{
        self.validate()?;
        if !self.timebase.epoch.matches_header_seconds(header.reference_epoch_s)
            ||!header.voxel_size_meters.is_finite()||header.voxel_size_meters<=0.0
            ||header.origin_meters.iter().any(|v|!v.is_finite())
            ||header.grid_dims_bricks.iter().any(|v|*v==0||*v>(1u32<<self.hilbert_order)) {
            return Err(EmfError::ExtensionMetadata);
        }
        Ok(())
    }
    pub fn validate(&self)->Result<(),EmfError> {
        if !(1..=8).contains(&self.hilbert_order){return Err(EmfError::ExtensionMetadata);}
        if let EmfColourInterpretation::RgbPreview{reference_white_nits,..}=self.colour {
            if !reference_white_nits.is_finite()||reference_white_nits<=0.0||reference_white_nits>1e9 {
                return Err(EmfError::ExtensionMetadata);
            }
        }
        self.timebase.validate()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfAudioPurpose { Recorded, Sonification, Reference }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfChannelLayout { Mono, Stereo, Discrete }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag="unit",rename_all="snake_case",deny_unknown_fields)]
pub enum EmfSignalUnits { NormalizedAmplitude, Pascal, Arbitrary{label:String} }

/// For frame i, time is start_tick*tick_den/tick_num + i*rate_den/rate_num seconds.
/// No audible-band restriction is imposed by serialization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfAudioTrackSpec {
    pub id:String,
    pub purpose:EmfAudioPurpose,
    pub channels:u16,
    pub channel_layout:EmfChannelLayout,
    pub sample_rate_num:u64,
    pub sample_rate_den:u64,
    pub start_tick:i64,
    pub units:EmfSignalUnits,
    pub units_per_sample:f64,
}
impl EmfAudioTrackSpec {
    pub fn validate(&self,index:usize)->Result<(),EmfError> {
        let bad=||EmfError::AudioTrack{track_index:index};
        if self.id.is_empty()||self.id.len()>64||!self.id.bytes().all(|b|b.is_ascii_alphanumeric()||b"_-".contains(&b))
            ||self.channels==0||self.channels>32||self.sample_rate_num==0||self.sample_rate_num>1_000_000_000_000
            ||self.sample_rate_den==0||self.sample_rate_den>1_000_000_000
            ||self.sample_rate_num>self.sample_rate_den.checked_mul(1_000_000_000).ok_or_else(bad)?
            ||!self.units_per_sample.is_finite()||self.units_per_sample<=0.0 {
            return Err(bad());
        }
        if matches!(self.channel_layout,EmfChannelLayout::Mono)&&self.channels!=1
            ||matches!(self.channel_layout,EmfChannelLayout::Stereo)&&self.channels!=2 {
            return Err(bad());
        }
        match &self.units {
            EmfSignalUnits::NormalizedAmplitude if self.units_per_sample!=1.0=>return Err(bad()),
            EmfSignalUnits::Arbitrary{label} if label.is_empty()||label.len()>64||label.chars().any(char::is_control)=>return Err(bad()),
            _=>{},
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EmfAudioCodec { PcmF32Le }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfAudioTrackDescriptor {
    pub codec:EmfAudioCodec,
    pub spec:EmfAudioTrackSpec,
    pub frames:u64,
    /// Offset relative to the beginning of the extension's PCM section.
    pub pcm_offset:u64,
    pub pcm_bytes:u64,
    /// Complete unkeyed BLAKE3 of exact original interleaved PCM-f32LE bytes.
    pub blake3:[u8;32],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmfExtensionMetadata {
    pub schema_version:u16,
    pub field:EmfFieldInterpretation,
    pub audio_tracks:Vec<EmfAudioTrackDescriptor>,
}
#[derive(Clone, Debug)]
pub struct EmfAudioTrack<'a> { pub spec:EmfAudioTrackSpec,pub samples:&'a[f32] }
