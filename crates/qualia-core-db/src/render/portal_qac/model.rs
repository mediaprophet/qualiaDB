use crate::audio::qac::{
    metadata::Transform,
    signal::{self, Scalar, Timing},
};
use crate::q42::{field_envelope as env, field_semantics as sem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Envelope(env::Error),
    Signal(signal::Error),
    Limit,
    Overflow,
    Index,
    Unsupported,
    Invariant,
}
impl From<env::Error> for Error {
    fn from(e: env::Error) -> Self {
        Self::Envelope(e)
    }
}
impl From<signal::Error> for Error {
    fn from(e: signal::Error) -> Self {
        Self::Signal(e)
    }
}

/// Explicit source, cold scratch/metadata and hot-window budgets.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub envelope: env::Limits,
    pub shared: sem::Limits,
    pub signal: signal::Limits,
    /// Includes the retained source, temporary graph states, and copied metadata.
    pub memory_bytes: usize,
    pub window_records: u64,
    pub window_samples: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            envelope: env::Limits::default(),
            shared: sem::Limits::default(),
            signal: signal::Limits::default(),
            memory_bytes: 320 << 20,
            window_records: 65536,
            window_samples: 1_048_576,
        }
    }
}

/// Scientific declarations, retained exactly before any JS number conversion.
/// Scale/offset are declarations, never automatically applied to stored samples.
#[derive(Clone, Debug)]
pub struct Metadata {
    pub selected_entry: u32,
    pub descriptor_id: u32,
    pub clock_id: u32,
    pub provenance_id: u32,
    pub epistemic_role: &'static str,
    pub stream_id: u32,
    pub frame_id: u32,
    pub mode: u16,
    pub scalar: Scalar,
    pub channels: u32,
    pub bins: u32,
    pub records: u64,
    pub timing: Timing,
    pub transform: Option<Transform>,
    pub window_coefficients_le: Option<Box<[u8]>>,
    pub channel_order: Box<str>,
    pub purpose: Box<str>,
    pub relation: Box<str>,
    pub quantity: Box<str>,
    pub units: Box<str>,
    pub model_uri: Box<str>,
    pub units_per_stored_sample: f64,
    pub stored_offset: f64,
    pub index_digest: [u8; 32],
}

pub const fn provenance_role_str(code: u32) -> Option<&'static str> {
    match code {
        0 => Some("measured"),
        1 => Some("model"),
        2 => Some("derived"),
        3 => Some("command"),
        4 => Some("opaque-preserved"),
        _ => None,
    }
}
