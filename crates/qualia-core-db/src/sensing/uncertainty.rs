//! Typed, field-bound uncertainty declarations for sensing data.
//!
//! The record reuses the selected FIEL descriptor and MID2 typed-field record
//! represented by `PhysicalFieldSemantics`. It preserves their unit IDs
//! independently, adds no Quin metadata convention, and makes unknown explicit.
//! Numeric declarations are caller assertions; this module verifies shape and
//! finite values, not scientific validity, calibration, probability quality,
//! source authority, or evidence authenticity.

use core::num::{NonZeroU32, NonZeroU64};

use crate::NQuin;

use super::{
    projection_map::{AxisUncertainty, MappedProjection},
    PhysicalFieldSemantics, SemanticId,
};

/// q_hash-based predicates proposed for a future sensing uncertainty vocabulary.
/// These names are not asserted to exist in an already registered ontology.
pub mod proposed_predicates {
    use crate::q_hash;

    pub const RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    pub const UNCERTAINTY_CLASS: u64 = q_hash("urn:qualia:sensing:UncertaintyRecord");
    pub const HAS_FIELD_ID: u64 = q_hash("urn:qualia:sensing:fieldId");
    pub const HAS_FIELD_KIND: u64 = q_hash("urn:qualia:sensing:mid2FieldKind");
    pub const HAS_FIELD_NAME_HASH: u64 = q_hash("urn:qualia:sensing:fielNameHash");
    pub const HAS_LEGACY_UNIT_HASH: u64 = q_hash("urn:qualia:sensing:fielUnitHash");
    pub const HAS_TYPED_UNIT_ID: u64 = q_hash("urn:qualia:sensing:mid2UnitId");
    pub const HAS_ESTIMATE_UNIT_ID: u64 = q_hash("urn:qualia:sensing:estimateUnitId");
    pub const HAS_AXIS_UNIT_ID: u64 = q_hash("urn:qualia:sensing:projectionAxisUnitId");
    pub const HAS_QUANTITY_ID: u64 = q_hash("urn:qualia:sensing:physicalQuantityId");
    pub const HAS_SCOPE_KIND: u64 = q_hash("urn:qualia:sensing:uncertaintyScopeKind");
    pub const HAS_MAPPING_ID: u64 = q_hash("urn:qualia:sensing:projectionMappingId");
    pub const HAS_AXIS_INDEX: u64 = q_hash("urn:qualia:sensing:projectionAxisIndex");
    pub const HAS_PROFILE_ID: u64 = q_hash("urn:qualia:sensing:projectionProfileId");
    pub const HAS_DOMAIN_ID: u64 = q_hash("urn:qualia:sensing:projectionDomainId");
    pub const HAS_COORDINATE_CONVENTION_ID: u64 =
        q_hash("urn:qualia:sensing:coordinateConventionId");
    pub const HAS_METHOD_ID: u64 = q_hash("urn:qualia:sensing:methodId");
    pub const HAS_METHOD_VERSION: u64 = q_hash("urn:qualia:sensing:methodVersion");
    pub const HAS_EVIDENCE_CITATION: u64 = q_hash("urn:qualia:sensing:evidenceCitationId");
    pub const HAS_ESTIMATE_KIND: u64 = q_hash("urn:qualia:sensing:estimateKind");
    pub const HAS_ABSOLUTE_BOUND: u64 = q_hash("urn:qualia:sensing:absoluteBound");
    pub const HAS_RELATIVE_BOUND_MILLIONTHS: u64 =
        q_hash("urn:qualia:sensing:relativeBoundMillionths");
    pub const HAS_INTERVAL_LOWER: u64 = q_hash("urn:qualia:sensing:intervalLower");
    pub const HAS_INTERVAL_UPPER: u64 = q_hash("urn:qualia:sensing:intervalUpper");
    pub const HAS_COVERAGE_MILLIONTHS: u64 = q_hash("urn:qualia:sensing:coverageMillionths");
    pub const HAS_METHOD_STATE: u64 = q_hash("urn:qualia:sensing:methodState");
    pub const HAS_EVIDENCE_STATE: u64 = q_hash("urn:qualia:sensing:evidenceState");
    pub const HAS_TYPED_UNIT_STATE: u64 = q_hash("urn:qualia:sensing:typedUnitState");
    pub const HAS_LEGACY_UNIT_STATE: u64 = q_hash("urn:qualia:sensing:legacyUnitState");
    pub const HAS_QUANTITY_STATE: u64 = q_hash("urn:qualia:sensing:quantityState");
}

const ID_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
pub const UNCERTAINTY_SCALE: u32 = 1_000_000;
pub const MAX_UNCERTAINTY_QUINS: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MethodVersion {
    method_id: SemanticId,
    version: NonZeroU32,
}

impl MethodVersion {
    pub fn new(method_id: SemanticId, version: u32) -> Result<Self, UncertaintyError> {
        require_id(method_id)?;
        let version = NonZeroU32::new(version).ok_or(UncertaintyError::InvalidMethod)?;
        Ok(Self { method_id, version })
    }

    pub const fn method_id(self) -> SemanticId {
        self.method_id
    }
    pub const fn version(self) -> u32 {
        self.version.get()
    }
}

/// A numeric physical error bound, a dimensionless relative bound, or a
/// confidence interval. Confidence level is a method parameter; it is not a
/// probability that the measurement or proposition is true.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UncertaintyEstimate {
    Unknown,
    AbsoluteBound {
        value: f64,
        unit_id: SemanticId,
    },
    /// Dimensionless ratio in millionths; values above one million (100%) are
    /// valid and are not silently clamped.
    RelativeBoundMillionths(u32),
    ConfidenceInterval {
        lower: f64,
        upper: f64,
        coverage_millionths: u32,
        unit_id: SemanticId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceReference(NonZeroU64);

impl EvidenceReference {
    pub const fn new(citation_id: u64) -> Result<Self, UncertaintyError> {
        if citation_id == 0 || citation_id > ID_MASK {
            Err(UncertaintyError::InvalidIdentifier)
        } else {
            match NonZeroU64::new(citation_id) {
                Some(id) => Ok(Self(id)),
                None => Err(UncertaintyError::InvalidIdentifier),
            }
        }
    }

    pub const fn citation_id(self) -> u64 {
        self.0.get()
    }
}

/// Where the uncertainty applies. Projection coordinates remain distinct from
/// the source field's physical quantity and units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UncertaintyScope {
    PhysicalField,
    ProjectionAxis {
        mapping_id: u64,
        axis_index: u8,
        axis_unit_id: SemanticId,
        profile_id: SemanticId,
        domain_id: SemanticId,
        coordinate_convention_id: SemanticId,
    },
}

/// Immutable fixed-size declaration bound to one existing FIEL/MID2 field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UncertaintyRecord {
    record_id: u64,
    field: PhysicalFieldSemantics,
    scope: UncertaintyScope,
    estimate: UncertaintyEstimate,
    method: Option<MethodVersion>,
    evidence: Option<EvidenceReference>,
}

impl UncertaintyRecord {
    pub fn new(
        record_id: u64,
        field: PhysicalFieldSemantics,
        estimate: UncertaintyEstimate,
        method: Option<MethodVersion>,
        evidence_citation_id: Option<u64>,
    ) -> Result<Self, UncertaintyError> {
        let evidence = match evidence_citation_id {
            Some(id) => Some(EvidenceReference::new(id)?),
            None => None,
        };
        Self::with_scope(
            record_id,
            field,
            UncertaintyScope::PhysicalField,
            estimate,
            method,
            evidence,
        )
    }

    /// Convert an already-propagated projection axis uncertainty. The caller
    /// must supply the axis unit because Tensor10D profile/domain/convention
    /// declarations do not define a unit conversion by themselves.
    pub fn from_projection_axis(
        record_id: u64,
        projection: &MappedProjection,
        axis_index: usize,
        axis_unit_id: SemanticId,
        method: Option<MethodVersion>,
        evidence_citation_id: Option<u64>,
    ) -> Result<Self, UncertaintyError> {
        if axis_index >= 10 {
            return Err(UncertaintyError::InvalidAxis);
        }
        require_id(axis_unit_id)?;
        let estimate = match projection.axis_uncertainty()[axis_index] {
            AxisUncertainty::Unknown => UncertaintyEstimate::Unknown,
            AxisUncertainty::Known(value) if value.is_finite() && value >= 0.0 => {
                UncertaintyEstimate::AbsoluteBound {
                    value: value as f64,
                    unit_id: axis_unit_id,
                }
            }
            AxisUncertainty::Known(_) => return Err(UncertaintyError::InvalidEstimate),
        };
        let declaration = projection.projection();
        let scope = UncertaintyScope::ProjectionAxis {
            mapping_id: projection.mapping_id(),
            axis_index: axis_index as u8,
            axis_unit_id,
            profile_id: declaration.profile_id(),
            domain_id: declaration.domain_id(),
            coordinate_convention_id: declaration.coordinate_convention_id(),
        };
        let evidence = match evidence_citation_id {
            Some(id) => Some(EvidenceReference::new(id)?),
            None => None,
        };
        Self::with_scope(
            record_id,
            *declaration.physical(),
            scope,
            estimate,
            method,
            evidence,
        )
    }

    fn with_scope(
        record_id: u64,
        field: PhysicalFieldSemantics,
        scope: UncertaintyScope,
        estimate: UncertaintyEstimate,
        method: Option<MethodVersion>,
        evidence: Option<EvidenceReference>,
    ) -> Result<Self, UncertaintyError> {
        if record_id == 0 || record_id > ID_MASK || field.field_id() > ID_MASK {
            return Err(UncertaintyError::InvalidIdentifier);
        }
        check_optional_id(field.physical_quantity())?;
        check_optional_id(field.typed_unit_id())?;
        if field.field_name_hash() > ID_MASK || field.legacy_unit_hash() > ID_MASK {
            return Err(UncertaintyError::InvalidIdentifier);
        }
        if let Some(method) = method {
            require_id(method.method_id)?;
        }
        if !matches!(estimate, UncertaintyEstimate::Unknown)
            && field.physical_quantity().value().is_none()
        {
            return Err(UncertaintyError::UnknownRequiredSemantics);
        }
        validate_scope(scope)?;
        validate_estimate(field, scope, estimate)?;
        Ok(Self {
            record_id,
            field,
            scope,
            estimate,
            method,
            evidence,
        })
    }

    pub const fn record_id(self) -> u64 {
        self.record_id
    }
    pub const fn field(self) -> PhysicalFieldSemantics {
        self.field
    }
    pub const fn scope(self) -> UncertaintyScope {
        self.scope
    }
    pub const fn estimate(self) -> UncertaintyEstimate {
        self.estimate
    }
    pub const fn method(self) -> Option<MethodVersion> {
        self.method
    }
    pub const fn evidence(self) -> Option<EvidenceReference> {
        self.evidence
    }

    /// Export this declaration as a bounded graph of Quins. Every row uses
    /// metadata=0 and the existing five-field `NQuin::calculate_parity` ABI.
    /// Predicates are proposed q_hash vocabulary terms, not yet registered
    /// universal terms. Unknown optional values get explicit state rows.
    pub fn write_quins(&self, out: &mut [NQuin]) -> Result<usize, UncertaintyError> {
        let count = self.required_quins();
        if out.len() < count {
            return Err(UncertaintyError::Capacity {
                required: count,
                available: out.len(),
            });
        }
        let encoded_estimate = self.encode_estimate()?;
        let mut cursor = 0;
        use proposed_predicates as p;
        self.push(out, &mut cursor, p::RDF_TYPE, p::UNCERTAINTY_CLASS);
        self.push(out, &mut cursor, p::HAS_FIELD_ID, self.field.field_id());
        self.push_integer(
            out,
            &mut cursor,
            p::HAS_FIELD_KIND,
            self.field.typed_field_kind() as u64,
        );
        self.push(
            out,
            &mut cursor,
            p::HAS_FIELD_NAME_HASH,
            self.field.field_name_hash(),
        );
        self.push_integer(
            out,
            &mut cursor,
            p::HAS_LEGACY_UNIT_STATE,
            bool_id(self.field.legacy_unit_hash() != 0),
        );
        if self.field.legacy_unit_hash() != 0 {
            self.push(
                out,
                &mut cursor,
                p::HAS_LEGACY_UNIT_HASH,
                self.field.legacy_unit_hash(),
            );
        }
        self.push_optional_id(
            out,
            &mut cursor,
            p::HAS_QUANTITY_STATE,
            p::HAS_QUANTITY_ID,
            self.field.physical_quantity(),
        );
        self.push_optional_id(
            out,
            &mut cursor,
            p::HAS_TYPED_UNIT_STATE,
            p::HAS_TYPED_UNIT_ID,
            self.field.typed_unit_id(),
        );
        match self.scope {
            UncertaintyScope::PhysicalField => {
                self.push_integer(out, &mut cursor, p::HAS_SCOPE_KIND, 0)
            }
            UncertaintyScope::ProjectionAxis {
                mapping_id,
                axis_index,
                axis_unit_id,
                profile_id,
                domain_id,
                coordinate_convention_id,
            } => {
                self.push_integer(out, &mut cursor, p::HAS_SCOPE_KIND, 1);
                self.push(out, &mut cursor, p::HAS_MAPPING_ID, mapping_id);
                self.push_integer(out, &mut cursor, p::HAS_AXIS_INDEX, axis_index as u64);
                self.push(
                    out,
                    &mut cursor,
                    p::HAS_PROFILE_ID,
                    profile_id.value().unwrap_or(0),
                );
                self.push(
                    out,
                    &mut cursor,
                    p::HAS_DOMAIN_ID,
                    domain_id.value().unwrap_or(0),
                );
                self.push(
                    out,
                    &mut cursor,
                    p::HAS_COORDINATE_CONVENTION_ID,
                    coordinate_convention_id.value().unwrap_or(0),
                );
                self.push(
                    out,
                    &mut cursor,
                    p::HAS_AXIS_UNIT_ID,
                    axis_unit_id.value().unwrap_or(0),
                );
            }
        }
        self.push_integer(
            out,
            &mut cursor,
            p::HAS_METHOD_STATE,
            bool_id(self.method.is_some()),
        );
        if let Some(method) = self.method {
            self.push(
                out,
                &mut cursor,
                p::HAS_METHOD_ID,
                method.method_id.value().unwrap_or(0),
            );
            self.push_integer(
                out,
                &mut cursor,
                p::HAS_METHOD_VERSION,
                method.version() as u64,
            );
        }
        self.push_integer(
            out,
            &mut cursor,
            p::HAS_EVIDENCE_STATE,
            bool_id(self.evidence.is_some()),
        );
        if let Some(evidence) = self.evidence {
            self.push(
                out,
                &mut cursor,
                p::HAS_EVIDENCE_CITATION,
                evidence.citation_id(),
            );
        }
        self.write_estimate(out, &mut cursor, encoded_estimate);
        Ok(cursor)
    }

    fn required_quins(self) -> usize {
        let base = 10
            + count_if(self.field.legacy_unit_hash() != 0)
            + count_if(self.field.physical_quantity().value().is_some())
            + count_if(self.field.typed_unit_id().value().is_some())
            + count_if(self.method.is_some()) * 2
            + count_if(self.evidence.is_some());
        let scope = match self.scope {
            UncertaintyScope::PhysicalField => 0,
            UncertaintyScope::ProjectionAxis { .. } => 6,
        };
        base + scope
            + match self.estimate {
                UncertaintyEstimate::Unknown => 1,
                UncertaintyEstimate::AbsoluteBound { .. } => 3,
                UncertaintyEstimate::RelativeBoundMillionths(_) => 2,
                UncertaintyEstimate::ConfidenceInterval { .. } => 5,
            }
    }

    fn push(&self, out: &mut [NQuin], cursor: &mut usize, predicate: u64, object: u64) {
        out[*cursor] = make_quin(self.record_id, predicate, object);
        *cursor += 1;
    }

    fn push_integer(&self, out: &mut [NQuin], cursor: &mut usize, predicate: u64, value: u64) {
        self.push(out, cursor, predicate, integer_object(value));
    }

    fn push_optional_id(
        &self,
        out: &mut [NQuin],
        cursor: &mut usize,
        state_predicate: u64,
        value_predicate: u64,
        value: SemanticId,
    ) {
        if let Some(value) = value.value() {
            self.push_integer(out, cursor, state_predicate, 1);
            self.push(out, cursor, value_predicate, value);
        } else {
            self.push_integer(out, cursor, state_predicate, 0);
        }
    }

    fn encode_estimate(&self) -> Result<EncodedEstimate, UncertaintyError> {
        Ok(match self.estimate {
            UncertaintyEstimate::Unknown => EncodedEstimate::Unknown,
            UncertaintyEstimate::AbsoluteBound { value, unit_id } => EncodedEstimate::Absolute {
                value: float_object(round_up(value).ok_or(UncertaintyError::NotRepresentable)?),
                unit_id: require_id(unit_id)?,
            },
            UncertaintyEstimate::RelativeBoundMillionths(value) => EncodedEstimate::Relative {
                millionths: value as u64,
            },
            UncertaintyEstimate::ConfidenceInterval {
                lower,
                upper,
                coverage_millionths,
                unit_id,
            } => EncodedEstimate::Interval {
                lower: float_object(round_down(lower).ok_or(UncertaintyError::NotRepresentable)?),
                upper: float_object(round_up(upper).ok_or(UncertaintyError::NotRepresentable)?),
                coverage_millionths: coverage_millionths as u64,
                unit_id: require_id(unit_id)?,
            },
        })
    }

    fn write_estimate(&self, out: &mut [NQuin], cursor: &mut usize, estimate: EncodedEstimate) {
        use proposed_predicates as p;
        match estimate {
            EncodedEstimate::Unknown => self.push_integer(out, cursor, p::HAS_ESTIMATE_KIND, 0),
            EncodedEstimate::Absolute { value, unit_id } => {
                self.push_integer(out, cursor, p::HAS_ESTIMATE_KIND, 1);
                self.push(out, cursor, p::HAS_ABSOLUTE_BOUND, value);
                self.push(out, cursor, p::HAS_ESTIMATE_UNIT_ID, unit_id);
            }
            EncodedEstimate::Relative { millionths } => {
                self.push_integer(out, cursor, p::HAS_ESTIMATE_KIND, 2);
                self.push_integer(out, cursor, p::HAS_RELATIVE_BOUND_MILLIONTHS, millionths);
            }
            EncodedEstimate::Interval {
                lower,
                upper,
                coverage_millionths,
                unit_id,
            } => {
                self.push_integer(out, cursor, p::HAS_ESTIMATE_KIND, 3);
                self.push(out, cursor, p::HAS_INTERVAL_LOWER, lower);
                self.push(out, cursor, p::HAS_INTERVAL_UPPER, upper);
                self.push_integer(out, cursor, p::HAS_COVERAGE_MILLIONTHS, coverage_millionths);
                self.push(out, cursor, p::HAS_ESTIMATE_UNIT_ID, unit_id);
            }
        }
    }
}

#[derive(Clone, Copy)]
enum EncodedEstimate {
    Unknown,
    Absolute {
        value: u64,
        unit_id: u64,
    },
    Relative {
        millionths: u64,
    },
    Interval {
        lower: u64,
        upper: u64,
        coverage_millionths: u64,
        unit_id: u64,
    },
}

fn validate_estimate(
    field: PhysicalFieldSemantics,
    scope: UncertaintyScope,
    estimate: UncertaintyEstimate,
) -> Result<(), UncertaintyError> {
    let unit = match scope {
        UncertaintyScope::PhysicalField => field.typed_unit_id(),
        UncertaintyScope::ProjectionAxis { axis_unit_id, .. } => axis_unit_id,
    };
    match estimate {
        UncertaintyEstimate::Unknown => Ok(()),
        UncertaintyEstimate::AbsoluteBound { value, unit_id } => {
            if !value.is_finite() || value < 0.0 {
                return Err(UncertaintyError::InvalidEstimate);
            }
            if unit_id != unit || unit_id.value().is_none() {
                return Err(UncertaintyError::UnitMismatch);
            }
            if round_up(value).is_none() {
                return Err(UncertaintyError::NotRepresentable);
            }
            Ok(())
        }
        UncertaintyEstimate::RelativeBoundMillionths(_) => Ok(()),
        UncertaintyEstimate::ConfidenceInterval {
            lower,
            upper,
            coverage_millionths,
            unit_id,
        } => {
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                return Err(UncertaintyError::InvalidInterval);
            }
            if coverage_millionths == 0 || coverage_millionths > UNCERTAINTY_SCALE {
                return Err(UncertaintyError::InvalidCoverage);
            }
            if unit_id != unit || unit_id.value().is_none() {
                return Err(UncertaintyError::UnitMismatch);
            }
            if round_down(lower).is_none() || round_up(upper).is_none() {
                return Err(UncertaintyError::NotRepresentable);
            }
            Ok(())
        }
    }
}

fn validate_scope(scope: UncertaintyScope) -> Result<(), UncertaintyError> {
    if let UncertaintyScope::ProjectionAxis {
        mapping_id,
        axis_index,
        axis_unit_id,
        profile_id,
        domain_id,
        coordinate_convention_id,
    } = scope
    {
        if mapping_id == 0 || mapping_id > ID_MASK || axis_index >= 10 {
            return Err(UncertaintyError::InvalidScope);
        }
        require_id(axis_unit_id)?;
        require_id(profile_id)?;
        require_id(domain_id)?;
        require_id(coordinate_convention_id)?;
    }
    Ok(())
}

fn require_id(id: SemanticId) -> Result<u64, UncertaintyError> {
    match id.value() {
        Some(value) if value <= ID_MASK => Ok(value),
        Some(_) => Err(UncertaintyError::InvalidIdentifier),
        None => Err(UncertaintyError::UnknownRequiredSemantics),
    }
}

fn check_optional_id(id: SemanticId) -> Result<(), UncertaintyError> {
    if let Some(value) = id.value() {
        if value > ID_MASK {
            return Err(UncertaintyError::InvalidIdentifier);
        }
    }
    Ok(())
}

fn bool_id(value: bool) -> u64 {
    if value {
        1
    } else {
        0
    }
}

fn count_if(value: bool) -> usize {
    if value {
        1
    } else {
        0
    }
}

fn make_quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    let metadata = 0;
    NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata,
        parity: NQuin::calculate_parity(subject, predicate, object, 0, metadata),
    }
}

fn float_object(value: f32) -> u64 {
    crate::frame_layout::pack_float_object(value)
}

fn integer_object(value: u64) -> u64 {
    // All call sites are type-bounded (`u16`, `u32`, or small enum/state values).
    crate::frame_layout::INLINE_TAG_INTEGER | value
}

fn round_up(value: f64) -> Option<f32> {
    if !value.is_finite() || value < -(f32::MAX as f64) || value > f32::MAX as f64 {
        return None;
    }
    let mut rounded = value as f32;
    if (rounded as f64) < value {
        let bits = rounded.to_bits();
        let next = if rounded >= 0.0 {
            bits.checked_add(1)?
        } else {
            bits.checked_sub(1)?
        };
        rounded = f32::from_bits(next);
    }
    if rounded.is_finite() {
        Some(rounded)
    } else {
        None
    }
}

fn round_down(value: f64) -> Option<f32> {
    if !value.is_finite() || value < -(f32::MAX as f64) || value > f32::MAX as f64 {
        return None;
    }
    let mut rounded = value as f32;
    if (rounded as f64) > value {
        let bits = rounded.to_bits();
        let next = if rounded > 0.0 {
            bits.checked_sub(1)?
        } else {
            bits.checked_add(1)?
        };
        rounded = f32::from_bits(next);
    }
    if rounded.is_finite() {
        Some(rounded)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UncertaintyError {
    InvalidIdentifier,
    UnknownRequiredSemantics,
    InvalidMethod,
    InvalidEstimate,
    InvalidInterval,
    InvalidCoverage,
    InvalidAxis,
    InvalidScope,
    UnitMismatch,
    NotRepresentable,
    Capacity { required: usize, available: usize },
}

#[cfg(test)]
#[path = "uncertainty_tests.rs"]
mod tests;
