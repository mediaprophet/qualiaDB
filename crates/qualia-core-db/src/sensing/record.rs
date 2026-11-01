//! Bounded persistence/index records for physical measurements.
//!
//! These records point to raw sample payloads and optional provenance,
//! calibration, and projection records. They do not embed or derive physical
//! coordinates, authenticate referenced material, or claim that a measurement
//! was verified. The wire encoding is independent of NQuin and uses explicit
//! little-endian fields rather than the overloaded Quin object/metadata lanes.
//! This experimental encoding is not yet an integrated or stable container
//! format; it requires review against the existing persistence interfaces.
//! A payload binding proves only that the selected MID2 declaration, field
//! semantics, and byte range agree. Callers must obtain payload bytes through
//! the checked `.10d` envelope resolver and enforce their container-ID binding,
//! section integrity/CRC, and whole-envelope admission policy before treating
//! referenced bytes as authentic or trusted. Neither this index nor a
//! nonzero container ID authenticates content.

mod decode;
mod encode;
mod types;

pub const MEASUREMENT_INDEX_VERSION: u16 = 1;
pub const MEASUREMENT_INDEX_RECORD_SIZE: usize = 264;
pub const MAX_MEASUREMENT_INDEX_RECORDS: usize = 256;

pub use decode::decode_measurement_index_record;
pub use encode::encode_measurement_index_records;
pub use types::{
    CalibrationArtifactReference, CalibrationDeclaration, MeasurementQuality, MeasurementRecord,
    PayloadReference, ProjectionDeclaration, ProjectionReference, ProvenanceDeclaration,
    QualityMetrics, RecordClockStamp, RecordError, ResolvedFieldPayloadBinding,
};

#[cfg(test)]
#[path = "record_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "record/decode_tests.rs"]
mod decode_tests;

#[cfg(test)]
#[path = "record/quin_link_tests.rs"]
mod quin_link_tests;
