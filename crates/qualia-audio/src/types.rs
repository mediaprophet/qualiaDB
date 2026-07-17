//! Stable auditory ABI (fixed-layout, caller-buffered where hot).

/// Maximum events written per `infer_chunk`.
pub const MAX_EVENTS: usize = 64;
/// Maximum transcript tokens per chunk.
pub const MAX_TRANSCRIPT_TOKENS: usize = 128;
/// Max embed dim written.
pub const MAX_EMBED_DIM: usize = 64;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    I16 = 0,
    I24Packed = 1,
    I32 = 2,
    F32 = 3,
}

/// Borrowed audio view — no ownership of samples.
#[derive(Debug, Clone, Copy)]
pub struct AudioView<'a> {
    pub bytes: &'a [u8],
    pub frames: u32,
    pub channels: u16,
    pub sample_rate: u32,
    /// Bytes between frame starts (≥ channels × sample size).
    pub frame_stride_bytes: u32,
    pub format: SampleFormat,
}

impl<'a> AudioView<'a> {
    #[inline]
    pub fn bytes_per_sample(self) -> u32 {
        match self.format {
            SampleFormat::I16 => 2,
            SampleFormat::I24Packed => 3,
            SampleFormat::I32 | SampleFormat::F32 => 4,
        }
    }

    pub fn is_well_formed(self) -> bool {
        if self.frames == 0 || self.channels == 0 || self.sample_rate == 0 {
            return false;
        }
        let need = self
            .frame_stride_bytes
            .saturating_mul(self.frames.saturating_sub(1))
            .saturating_add(self.channels as u32 * self.bytes_per_sample());
        self.bytes.len() as u64 >= need as u64
    }
}

/// One acoustic event proposal (epistemic, not ground truth).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditoryEvent {
    pub class_hash: u64,
    pub source_hash: u64,
    pub confidence_u16: u16,
    pub channel: u16,
    pub start_frame: u64,
    pub end_frame: u64,
    pub track_id: u32,
    pub flags: u32,
}

impl AuditoryEvent {
    pub const FLAG_REFERENCE_BACKEND: u32 = 1;
    pub const FLAG_LOW_ASSURANCE: u32 = 2;
    pub const FLAG_VAD: u32 = 4;

    pub fn empty() -> Self {
        Self {
            class_hash: 0,
            source_hash: 0,
            confidence_u16: 0,
            channel: 0,
            start_frame: 0,
            end_frame: 0,
            track_id: 0,
            flags: 0,
        }
    }

    pub fn confidence_f32(self) -> f32 {
        self.confidence_u16 as f32 / 65535.0
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptToken {
    pub form_hash: u64,
    pub proposed_meaning_hash: u64,
    pub confidence_u16: u16,
    pub language_slot: u16,
    pub start_frame: u64,
    pub end_frame: u64,
    pub speaker_track: u32,
    pub flags: u32,
}

impl TranscriptToken {
    pub fn empty() -> Self {
        Self {
            form_hash: 0,
            proposed_meaning_hash: 0,
            confidence_u16: 0,
            language_slot: 0,
            start_frame: 0,
            end_frame: 0,
            speaker_track: 0,
            flags: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditoryCapabilities {
    pub max_events: u16,
    pub embed_dim: u16,
    pub supports_vad: bool,
    pub supports_transcript: bool,
    pub is_reference_backend: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditoryOutputCounts {
    pub events: usize,
    pub tokens: usize,
    pub embedding_written: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioError {
    MalformedAudio,
    OutputBufferTooSmall,
    WorkspaceTooSmall,
    BackendUnavailable,
    UnsupportedFormat,
    Cancelled,
    PermissionDenied,
}

/// Backend-agnostic auditory model.
pub trait AuditoryModel {
    fn capabilities(&self) -> AuditoryCapabilities;

    fn infer_chunk(
        &mut self,
        audio: AudioView<'_>,
        events_out: &mut [AuditoryEvent],
        tokens_out: &mut [TranscriptToken],
        embedding_out: &mut [f32],
        workspace: &mut [u8],
    ) -> Result<AuditoryOutputCounts, AudioError>;
}
