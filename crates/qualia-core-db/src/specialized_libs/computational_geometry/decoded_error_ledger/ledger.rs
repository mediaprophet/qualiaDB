//! Typed error ledger: separate geometric, attribute, quantization, physical,
//! and numerical records. QEM costs and sampled estimates are never renamed
//! into an enclosing surface certificate.

use super::surface_bracket::{certify_surface, numerical_slack};
use super::{
    AbsenceReason, AdmissionEvidence, AttributePair, BoundRoute, BracketOptions, DecodedMeshPair,
    GeometryProfile, LedgerError, LedgerScratch, LedgerStatus, MeasurementClass, Metric,
    MetricWire, ProtectedCorrespondence, QuantizationModel, Units,
};
use crate::specialized_libs::computational_geometry::distance::distance_3d;
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    BudgetError, ProductionBudget,
};
use crate::specialized_libs::computational_geometry::physical_authority::PhysicalEvidence;
use crate::specialized_libs::computational_geometry::primitives::Point3;
use crate::specialized_libs::computational_geometry::production_contracts::ResultStatus;

const PHYSICAL_NOTE: &str =
    "P10 physical_authority is present; certify_decoded_output was not given PhysicalEvidence, so the physical channel is unsupported rather than a zero error";
const NUMERICAL_NOTE: &str =
    "IEEE-754 f64 round-to-nearest serial arithmetic; slack is a first-order ulp estimate, not outward rounding";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometricRecord {
    pub status: LedgerStatus,
    pub enclosing: Metric,
    pub sample: Metric,
    pub protected_drift: Metric,
    pub forbidden_semantic_merges: Metric,
    pub diagonal: Metric,
    pub domain_min: Point3,
    pub domain_max: Point3,
    pub method: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttributeRecord {
    pub status: LedgerStatus,
    pub metric: Metric,
    pub units_name: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuantizationRecord {
    pub status: LedgerStatus,
    pub bound: Metric,
    pub model: QuantizationModel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalRecord {
    pub status: LedgerStatus,
    pub metric: Metric,
    pub note: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericalRecord {
    pub status: LedgerStatus,
    pub slack: Metric,
    pub outward_rounding: bool,
    pub note: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecodedErrorLedger {
    pub status: LedgerStatus,
    pub geometric: GeometricRecord,
    pub attribute: AttributeRecord,
    pub quantization: QuantizationRecord,
    pub physical: PhysicalRecord,
    pub numerical: NumericalRecord,
}

impl DecodedErrorLedger {
    pub fn metric_wire(self) -> [MetricWire; 5] {
        [
            self.geometric.enclosing.to_wire(),
            self.attribute.metric.to_wire(),
            self.quantization.bound.to_wire(),
            self.physical.metric.to_wire(),
            self.numerical.slack.to_wire(),
        ]
    }
}

pub fn certify_decoded_output(
    pair: DecodedMeshPair<'_>,
    attributes: Option<AttributePair<'_>>,
    protected: Option<ProtectedCorrespondence<'_>>,
    admission: AdmissionEvidence,
    quantization: QuantizationModel,
    profile: GeometryProfile,
    options: BracketOptions,
    scratch: &mut LedgerScratch<'_>,
    budget: &mut ProductionBudget<'_>,
) -> Result<DecodedErrorLedger, LedgerError> {
    certify_decoded_output_with_physical(
        pair,
        attributes,
        protected,
        admission,
        quantization,
        profile,
        options,
        scratch,
        budget,
        None,
    )
}

/// Map a supplied P10 evidence record into the P08 physical channel.
/// `None` stays unsupported with `NotSupplied`; it is never a silent 0.0.
pub fn physical_record(evidence: Option<&PhysicalEvidence>) -> PhysicalRecord {
    match evidence {
        None => PhysicalRecord {
            status: LedgerStatus::Unsupported,
            metric: Metric::absent(AbsenceReason::NotSupplied),
            note: PHYSICAL_NOTE,
        },
        Some(ev) => PhysicalRecord {
            status: ledger_status_from_result(ev.status),
            metric: match ev.value {
                Some(value) if value.is_finite() => Metric::Scalar {
                    value,
                    units: Units::Dimensionless,
                    class: MeasurementClass::Measured,
                },
                _ => Metric::absent(absence_from_result(ev.status)),
            },
            note: ev.reason,
        },
    }
}

fn ledger_status_from_result(status: ResultStatus) -> LedgerStatus {
    match status {
        ResultStatus::Passed => LedgerStatus::Passed,
        ResultStatus::Failed => LedgerStatus::Failed,
        ResultStatus::Inconclusive => LedgerStatus::Inconclusive,
        ResultStatus::Skipped | ResultStatus::Unsupported => LedgerStatus::Unsupported,
    }
}

fn absence_from_result(status: ResultStatus) -> AbsenceReason {
    match status {
        ResultStatus::Unsupported | ResultStatus::Skipped => AbsenceReason::UnsupportedCapability,
        ResultStatus::Inconclusive => AbsenceReason::IncompleteEnclosure,
        ResultStatus::Failed | ResultStatus::Passed => AbsenceReason::NotSupplied,
    }
}

pub fn certify_decoded_output_with_physical(
    pair: DecodedMeshPair<'_>,
    attributes: Option<AttributePair<'_>>,
    protected: Option<ProtectedCorrespondence<'_>>,
    admission: AdmissionEvidence,
    quantization: QuantizationModel,
    profile: GeometryProfile,
    options: BracketOptions,
    scratch: &mut LedgerScratch<'_>,
    budget: &mut ProductionBudget<'_>,
    physical_evidence: Option<&PhysicalEvidence>,
) -> Result<DecodedErrorLedger, LedgerError> {
    validate_pair(pair)?;
    match budget.checkpoint(0) {
        Ok(()) => {}
        Err(BudgetError::Cancelled) => {
            return Ok(cancelled_ledger());
        }
        Err(err) => return Err(LedgerError::Budget(err)),
    }
    let physical = physical_record(physical_evidence);
    let attribute = attribute_record(attributes)?;
    let quantization = quantization_record(quantization, pair.decoded.vertices)?;
    let (protected_metric, protected_status) = protected_drift(pair, protected, profile, budget)?;
    let merges = merge_metric(admission.forbidden_semantic_merges);
    let surface = certify_surface(pair, profile, options, scratch, budget)?;
    let mut geometric_status = surface.status;
    if matches!(protected_status, LedgerStatus::Failed) {
        geometric_status = LedgerStatus::Failed;
    }
    if matches!(protected_status, LedgerStatus::Cancelled) {
        geometric_status = LedgerStatus::Cancelled;
    }
    if let Metric::Scalar { value, .. } = merges {
        if value > 0.0 && geometric_status != LedgerStatus::Cancelled {
            geometric_status = LedgerStatus::Failed;
        }
    }
    let geometric = GeometricRecord {
        status: geometric_status,
        enclosing: surface.enclosing,
        sample: surface.sample,
        protected_drift: protected_metric,
        forbidden_semantic_merges: merges,
        diagonal: Metric::Scalar {
            value: surface.diagonal,
            units: Units::AssetLength,
            class: MeasurementClass::Measured,
        },
        domain_min: surface.domain_min,
        domain_max: surface.domain_max,
        method: surface.method,
    };
    let numerical = NumericalRecord {
        status: if surface.slack.is_finite() {
            LedgerStatus::Passed
        } else {
            LedgerStatus::Inconclusive
        },
        slack: Metric::Scalar {
            value: surface.slack,
            units: Units::AssetLength,
            class: MeasurementClass::Measured,
        },
        outward_rounding: false,
        note: NUMERICAL_NOTE,
    };
    let status = overall(
        geometric_status,
        attribute.status,
        quantization.status,
        options.route,
    );
    Ok(DecodedErrorLedger {
        status,
        geometric,
        attribute,
        quantization,
        physical,
        numerical,
    })
}

fn overall(
    geometric: LedgerStatus,
    _attribute: LedgerStatus,
    _quantization: LedgerStatus,
    route: BoundRoute,
) -> LedgerStatus {
    if geometric == LedgerStatus::Cancelled {
        return LedgerStatus::Cancelled;
    }
    if geometric == LedgerStatus::Failed {
        return LedgerStatus::Failed;
    }
    if route == BoundRoute::SampledVertices {
        return LedgerStatus::Inconclusive;
    }
    geometric
}

fn validate_pair(pair: DecodedMeshPair<'_>) -> Result<(), LedgerError> {
    if pair.reference.vertices.is_empty()
        || pair.reference.triangles.is_empty()
        || pair.decoded.vertices.is_empty()
        || pair.decoded.triangles.is_empty()
    {
        return Err(LedgerError::InvalidInput("empty_mesh"));
    }
    Ok(())
}

fn attribute_record(attributes: Option<AttributePair<'_>>) -> Result<AttributeRecord, LedgerError> {
    let Some(pair) = attributes else {
        return Ok(AttributeRecord {
            status: LedgerStatus::Unsupported,
            metric: Metric::absent(AbsenceReason::NotSupplied),
            units_name: "",
        });
    };
    if pair.reference.len() != pair.decoded.len() {
        return Ok(AttributeRecord {
            status: LedgerStatus::Unsupported,
            metric: Metric::absent(AbsenceReason::UnsupportedCapability),
            units_name: pair.units,
        });
    }
    let mut max_abs = 0.0_f64;
    for (a, b) in pair.reference.iter().zip(pair.decoded.iter()) {
        if !(a.is_finite() && b.is_finite()) {
            return Err(LedgerError::InvalidInput("non_finite_attribute"));
        }
        max_abs = max_abs.max((a - b).abs());
    }
    Ok(AttributeRecord {
        status: LedgerStatus::Passed,
        metric: Metric::Scalar {
            value: max_abs,
            units: Units::Dimensionless,
            class: MeasurementClass::SampledEstimate,
        },
        units_name: pair.units,
    })
}

fn quantization_record(
    model: QuantizationModel,
    decoded: &[Point3],
) -> Result<QuantizationRecord, LedgerError> {
    match model {
        QuantizationModel::NotSupplied => Ok(QuantizationRecord {
            status: LedgerStatus::Unsupported,
            bound: Metric::absent(AbsenceReason::NotSupplied),
            model,
        }),
        QuantizationModel::UniformGrid { delta } => {
            if delta.iter().any(|d| !d.is_finite() || *d < 0.0) {
                return Err(LedgerError::InvalidInput("quantization_delta"));
            }
            let bound = 0.5 * (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
            Ok(QuantizationRecord {
                status: LedgerStatus::Passed,
                bound: Metric::Scalar {
                    value: bound,
                    units: Units::AssetLength,
                    class: MeasurementClass::EnclosingBound,
                },
                model,
            })
        }
        QuantizationModel::Binary32 => {
            let mut bound = 0.0_f64;
            for p in decoded {
                let ux = 0.5 * f32_ulp(p.x);
                let uy = 0.5 * f32_ulp(p.y);
                let uz = 0.5 * f32_ulp(p.z);
                bound = bound.max((ux * ux + uy * uy + uz * uz).sqrt());
            }
            if !bound.is_finite() {
                return Err(LedgerError::InvalidInput("quantization_non_finite"));
            }
            Ok(QuantizationRecord {
                status: LedgerStatus::Passed,
                bound: Metric::Scalar {
                    value: bound,
                    units: Units::AssetLength,
                    class: MeasurementClass::EnclosingBound,
                },
                model,
            })
        }
    }
}

fn f32_ulp(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    let y = x as f32;
    if !y.is_finite() {
        return f64::INFINITY;
    }
    let bits = y.to_bits();
    let step = |b: u32| {
        let z = f32::from_bits(b);
        if z.is_finite() { (z as f64 - y as f64).abs() } else { 0.0 }
    };
    step(bits.wrapping_add(1)).max(step(bits.wrapping_sub(1)))
}

fn protected_drift(
    pair: DecodedMeshPair<'_>,
    protected: Option<ProtectedCorrespondence<'_>>,
    profile: GeometryProfile,
    budget: &mut ProductionBudget<'_>,
) -> Result<(Metric, LedgerStatus), LedgerError> {
    let Some(corr) = protected else {
        return Ok((
            Metric::absent(AbsenceReason::NotSupplied),
            LedgerStatus::Unsupported,
        ));
    };
    if corr.reference_vertices.len() != corr.decoded_vertices.len() {
        return Err(LedgerError::InvalidInput("protected_length"));
    }
    match budget.checkpoint(corr.reference_vertices.len() as u64) {
        Ok(()) => {}
        Err(BudgetError::Cancelled) => {
            return Ok((
                Metric::absent(AbsenceReason::Cancelled),
                LedgerStatus::Cancelled,
            ));
        }
        Err(BudgetError::WorkLimitExceeded { .. }) => {
            return Ok((
                Metric::absent(AbsenceReason::BudgetExhausted),
                LedgerStatus::Inconclusive,
            ));
        }
        Err(err) => return Err(LedgerError::Budget(err)),
    }
    let mut max_drift = 0.0_f64;
    for (&ri, &di) in corr.reference_vertices.iter().zip(corr.decoded_vertices.iter()) {
        let r = *pair
            .reference
            .vertices
            .get(ri as usize)
            .ok_or(LedgerError::InvalidInput("protected_index"))?;
        let d = *pair
            .decoded
            .vertices
            .get(di as usize)
            .ok_or(LedgerError::InvalidInput("protected_index"))?;
        max_drift = max_drift.max(distance_3d(r, d));
    }
    let diagonal = match pair.reference.vertices {
        [] => 0.0,
        verts => {
            let mut min = verts[0];
            let mut max = verts[0];
            for p in verts {
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                min.z = min.z.min(p.z);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
                max.z = max.z.max(p.z);
            }
            distance_3d(min, max)
        }
    };
    let tau = if diagonal > 8.0 * f64::EPSILON * (1.0 + max_drift) {
        profile.protected_threshold(diagonal)
    } else {
        profile
            .absolute_scale
            .map(|s| profile.protected_drift_over_diagonal * s)
            .unwrap_or(0.0)
    };
    let status = if max_drift > tau {
        LedgerStatus::Failed
    } else {
        LedgerStatus::Passed
    };
    Ok((
        Metric::Scalar {
            value: max_drift,
            units: Units::AssetLength,
            class: MeasurementClass::Measured,
        },
        status,
    ))
}

fn merge_metric(count: Option<u64>) -> Metric {
    match count {
        None => Metric::absent(AbsenceReason::NotSupplied),
        Some(n) => Metric::Scalar {
            value: n as f64,
            units: Units::Dimensionless,
            class: MeasurementClass::Measured,
        },
    }
}

fn cancelled_ledger() -> DecodedErrorLedger {
    let absent = Metric::absent(AbsenceReason::Cancelled);
    DecodedErrorLedger {
        status: LedgerStatus::Cancelled,
        geometric: GeometricRecord {
            status: LedgerStatus::Cancelled,
            enclosing: absent,
            sample: absent,
            protected_drift: absent,
            forbidden_semantic_merges: absent,
            diagonal: absent,
            domain_min: Point3::new(0.0, 0.0, 0.0),
            domain_max: Point3::new(0.0, 0.0, 0.0),
            method: "cancelled",
        },
        attribute: AttributeRecord {
            status: LedgerStatus::Cancelled,
            metric: absent,
            units_name: "",
        },
        quantization: QuantizationRecord {
            status: LedgerStatus::Cancelled,
            bound: absent,
            model: QuantizationModel::NotSupplied,
        },
        physical: PhysicalRecord {
            status: LedgerStatus::Unsupported,
            metric: Metric::absent(AbsenceReason::Cancelled),
            note: PHYSICAL_NOTE,
        },
        numerical: NumericalRecord {
            status: LedgerStatus::Cancelled,
            slack: Metric::Scalar {
                value: numerical_slack(0.0),
                units: Units::AssetLength,
                class: MeasurementClass::Measured,
            },
            outward_rounding: false,
            note: NUMERICAL_NOTE,
        },
    }
}
