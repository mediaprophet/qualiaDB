//! Borrowed source-bound pose publication. This is view state, not a player,
//! source store, authority grant, renderer or frame clock.
mod model;
mod validation;
pub use model::{decode_bound_model, validate_resident_mesh};
pub use validation::{validate_pose, SourceFrameState};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFrameBinding {
    pub model_sha256: [u8; 32],
    pub source_sha256: [u8; 32],
    pub source_generation: u64,
    pub mesh_generation: u64,
    /// Advances on seek/clip/rate changes without redecoding the resident model.
    pub transport_epoch: u64,
    pub vertex_count: u32,
    pub index_count: u32,
}

/// A complete single-model authored-space pose, borrowed from the native player.
/// The host owns exact timeline ticks (timeline-editing's existing timebase).
pub struct SourceMeshFrame<'a> {
    pub binding: SourceFrameBinding,
    pub frame_sequence: u64,
    pub timeline_tick: u64,
    pub positions: &'a [[f32; 3]],
    pub normals: &'a [[f32; 3]],
    pub tangents: &'a [[f32; 4]],
}

/// Admission/submission only. This is neither a device-present receipt nor
/// visual acceptance; asynchronous device failure cannot be rolled back here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceFrameReceipt {
    pub binding: SourceFrameBinding,
    pub frame_sequence: u64,
    pub timeline_tick: u64,
    pub bounds: [[f32; 3]; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFrameError {
    Input,
    Budget,
    Model,
    Integrity,
    ResidentMismatch,
    Stale,
    Sequence,
    Attributes,
    Backend,
    Upload,
}
impl core::fmt::Display for SourceFrameError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Input => "invalid source-frame binding or exact clock",
            Self::Budget => "source-frame cold admission budget exceeded",
            Self::Model => "source-frame canonical model refused",
            Self::Integrity => "source-frame model integrity refused",
            Self::ResidentMismatch => "source-frame requires the exact full authored resident mesh",
            Self::Stale => "source-frame source, mesh or transport epoch changed",
            Self::Sequence => "source-frame sequence is stale or exhausted",
            Self::Attributes => "source-frame surface attributes refused",
            Self::Backend => "source-frame requires the full WebGPU backend",
            Self::Upload => "source-frame upload refused before publication",
        })
    }
}
impl std::error::Error for SourceFrameError {}

/// Cold model verification is separate from zero-allocation pose publication.
#[derive(Clone, Copy, Debug)]
pub struct SourceFrameBindLimits {
    pub max_model_bytes: usize,
    pub max_vertices: usize,
    pub max_indices: usize,
    pub max_cold_bytes: usize,
}
impl Default for SourceFrameBindLimits {
    fn default() -> Self {
        Self {
            max_model_bytes: 64 * 1024 * 1024,
            max_vertices: 1_000_000,
            max_indices: 6_000_000,
            max_cold_bytes: 192 * 1024 * 1024,
        }
    }
}
