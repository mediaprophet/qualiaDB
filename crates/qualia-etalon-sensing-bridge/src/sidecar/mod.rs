//! Bounded experimental sidecar. Decoding never establishes trust or permission.
mod cursor;
mod decode;
mod encode;

pub use decode::decode_sidecar;
pub use encode::{encode_sidecar, reencode_untrusted_sidecar};

use etalon_sense_core::observation::ObservationEnvelope;
use qualia_core_db::sensing::record::{MeasurementRecord, RecordError};

pub const SIDECAR_SIZE: usize = 768;
const MAGIC: &[u8; 4] = b"QES1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidecarError {
    BufferSize,
    InvalidEncoding,
    InvalidObservation,
    Record(RecordError),
}

/// Decoded declarations only. The historical verification flag is an
/// unauthenticated stored claim. The reconstructed envelope downgrades that
/// evidence to `SignaturePresented`; any policy outcome is historical data.
pub struct UntrustedSidecar {
    pub(super) record: MeasurementRecord,
    pub(super) observation: ObservationEnvelope,
    pub(super) evidence_verification_claimed: bool,
}

impl UntrustedSidecar {
    pub const fn record(&self) -> &MeasurementRecord {
        &self.record
    }
    /// Historical, untrusted declarations. Any policy outcome is not a
    /// current authorization; verified evidence is downgraded on decode.
    pub const fn historical_observation(&self) -> &ObservationEnvelope {
        &self.observation
    }
    pub const fn evidence_verification_claimed(&self) -> bool {
        self.evidence_verification_claimed
    }
}

#[cfg(test)]
mod tests;
