//! Measurement index links and diagnostic validation of Quin parity.
//!
//! The index-link encoder uses zero metadata so both current parity contracts
//! agree. It links a record to host-declared event/payload identities without
//! encoding raw samples, granting trust or deciding measurement semantics.

#[path = "quin_index/encode.rs"]
mod encode;
pub use encode::{
    measurement_record_to_quins, proposed_predicates, MeasurementQuinError, MeasurementQuinMapping,
    MAX_MEASUREMENT_LINK_QUINS,
};

use crate::{frame_layout, NQuin};

/// Explicit results under the two currently conflicting parity conventions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasurementQuinParityValidation {
    nquin_ecc_valid: bool,
    frame_layout_valid: bool,
}

impl MeasurementQuinParityValidation {
    pub const fn nquin_ecc_valid(self) -> bool {
        self.nquin_ecc_valid
    }

    pub const fn frame_layout_valid(self) -> bool {
        self.frame_layout_valid
    }

    /// A caller may proceed only after choosing the authoritative contract;
    /// this helper makes no such choice and returns true only when both agree.
    pub const fn conventions_agree_valid(self) -> bool {
        self.nquin_ecc_valid && self.frame_layout_valid
    }
}

/// Validate an existing Quin under both published parity implementations.
///
/// This is diagnostic only. It does not validate measurement semantics,
/// identifiers, provenance, object tags, grants, or the meaning of metadata.
pub fn validate_measurement_quin_parity(quin: &NQuin) -> MeasurementQuinParityValidation {
    MeasurementQuinParityValidation {
        nquin_ecc_valid: quin.verify_ecc_parity(),
        frame_layout_valid: frame_layout::parity_valid(quin),
    }
}

#[cfg(test)]
#[path = "quin_index_tests.rs"]
mod tests;
