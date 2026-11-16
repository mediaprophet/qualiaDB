//! Source fixtures for P08: translation, thin features, known distance, unmet
//! width, and sampled estimates that cannot pass the enclosing-bound profile.

use super::*;
use crate::specialized_libs::computational_geometry::bvh::BvhNode;
use crate::specialized_libs::computational_geometry::distance::Aabb;
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;
use crate::specialized_libs::computational_geometry::geometry_workspace::Cancellation;
use crate::specialized_libs::computational_geometry::primitives::Point3;

struct Buffers {
    aabbs: Vec<Aabb>,
    nodes: Vec<BvhNode>,
    prim: Vec<u32>,
    morton: Vec<u64>,
    sort: Vec<u32>,
    stack: Vec<u32>,
    overlap: Vec<u32>,
    patches: Vec<SurfacePatch>,
}

impl Buffers {
    fn new(triangles: usize, overlap: usize, patches: usize) -> Self {
        let n = triangles.max(1);
        let dummy = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0));
        Self {
            aabbs: vec![dummy; n],
            nodes: vec![BvhNode::default(); LedgerScratch::bvh_node_len(n).unwrap()],
            prim: vec![0; n],
            morton: vec![0; n],
            sort: vec![0; n],
            stack: vec![0; LedgerScratch::bvh_stack_len()],
            overlap: vec![0; overlap],
            patches: vec![SurfacePatch::empty(); patches.max(1)],
        }
    }

    fn view(&mut self) -> LedgerScratch<'_> {
        LedgerScratch {
            aabbs: &mut self.aabbs,
            nodes: &mut self.nodes,
            prim_indices: &mut self.prim,
            morton_codes: &mut self.morton,
            sort_indices: &mut self.sort,
            stack: &mut self.stack,
            overlap: &mut self.overlap,
            patches: &mut self.patches,
        }
    }
}

fn square(origin: Point3) -> (Vec<Point3>, Vec<[u32; 3]>) {
    (
        vec![
            origin,
            Point3::new(origin.x + 1.0, origin.y, origin.z),
            Point3::new(origin.x + 1.0, origin.y + 1.0, origin.z),
            Point3::new(origin.x, origin.y + 1.0, origin.z),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
    )
}

fn certify(
    rv: &[Point3],
    rt: &[[u32; 3]],
    dv: &[Point3],
    dt: &[[u32; 3]],
    options: BracketOptions,
    overlap: usize,
    work: u64,
    quantization: QuantizationModel,
    protected: Option<ProtectedCorrespondence<'_>>,
    attributes: Option<AttributePair<'_>>,
    admission: AdmissionEvidence,
) -> DecodedErrorLedger {
    let n = rt.len().max(dt.len());
    let mut buffers = Buffers::new(n, overlap, 16);
    let cancel = Cancellation::new();
    let plan = plan_ledger_capacity(rv.len(), rt.len(), dv.len(), dt.len(), overlap, 16, 0, 0).unwrap();
    let mut budget = ProductionBudget::admit(plan, usize::MAX, work, &cancel).unwrap();
    certify_decoded_output(
        DecodedMeshPair {
            reference: TriangleMesh { vertices: rv, triangles: rt },
            decoded: TriangleMesh { vertices: dv, triangles: dt },
        },
        attributes,
        protected,
        admission,
        quantization,
        GeometryProfile::g1(),
        options,
        &mut buffers.view(),
        &mut budget,
    )
    .unwrap()
}

fn enclosing() -> BracketOptions {
    BracketOptions { route: BoundRoute::EnclosingSubdivision, max_subdivision_depth: 8 }
}

#[test]
fn translated_large_coordinate_fixture_passes_g1() {
    let origin = Point3::new(1e8, -2e8, 3e8);
    let (v, t) = square(origin);
    let ledger = certify(&v, &t, &v, &t, enclosing(), t.len(), 1_000_000, QuantizationModel::Binary32, None, None, AdmissionEvidence::default());
    assert_eq!(ledger.status, LedgerStatus::Passed);
    assert_eq!(ledger.geometric.status, LedgerStatus::Passed);
    match ledger.geometric.enclosing {
        Metric::Bracket { lower, upper, class, .. } => {
            assert_eq!(class, MeasurementClass::EnclosingBound);
            let d = match ledger.geometric.diagonal {
                Metric::Scalar { value, .. } => value,
                other => panic!("diagonal present, got {other:?}"),
            };
            assert!(upper <= GeometryProfile::g1().upper_threshold(d));
            assert!(upper - lower <= GeometryProfile::g1().width_threshold(d));
        }
        other => panic!("expected enclosing bracket, got {other:?}"),
    }
    assert_eq!(ledger.physical.metric.to_wire(), MetricWire::Null);
    assert_eq!(ledger.geometric.forbidden_semantic_merges.to_wire(), MetricWire::Null);
    match ledger.quantization.bound {
        Metric::Scalar { value, class, .. } => {
            assert_eq!(class, MeasurementClass::EnclosingBound);
            assert!(value > 1.0, "f32 ulp at 1e8 is a separate record, not a geometry fail");
        }
        other => panic!("binary32 bound present, got {other:?}"),
    }
}

#[test]
fn thin_feature_fixture_preserves_enclosing_bracket() {
    let (mut v, mut t) = square(Point3::new(0.0, 0.0, 0.0));
    v.extend([
        Point3::new(0.5, 0.5, 0.0),
        Point3::new(0.5 + 1e-6, 0.5, 0.0),
        Point3::new(0.5, 0.5, 1e-3),
    ]);
    t.push([4, 5, 6]);
    let ledger = certify(&v, &t, &v, &t, enclosing(), t.len(), 1_000_000, QuantizationModel::NotSupplied, None, None, AdmissionEvidence::default());
    assert_eq!(ledger.status, LedgerStatus::Passed);
    assert_eq!(ledger.quantization.bound.to_wire(), MetricWire::Null);
    match ledger.geometric.enclosing {
        Metric::Bracket { class, .. } => assert_eq!(class, MeasurementClass::EnclosingBound),
        other => panic!("expected enclosing bracket, got {other:?}"),
    }
}

#[test]
fn known_distance_lower_bound_exceeds_threshold_fails() {
    let (rv, rt) = square(Point3::new(0.0, 0.0, 0.0));
    let (dv, dt) = square(Point3::new(0.0, 0.0, 0.05));
    let ledger = certify(&rv, &rt, &dv, &dt, enclosing(), rt.len().max(dt.len()), 1_000_000, QuantizationModel::NotSupplied, None, None, AdmissionEvidence::default());
    assert_eq!(ledger.status, LedgerStatus::Failed);
    assert_eq!(ledger.geometric.status, LedgerStatus::Failed);
    match ledger.geometric.enclosing {
        Metric::Bracket { lower, .. } => {
            let d = match ledger.geometric.diagonal {
                Metric::Scalar { value, .. } => value,
                other => panic!("diagonal present, got {other:?}"),
            };
            assert!(lower > GeometryProfile::g1().upper_threshold(d));
            assert!(lower > 0.04);
        }
        other => panic!("expected enclosing bracket, got {other:?}"),
    }
}

#[test]
fn unmet_width_is_inconclusive() {
    let (v, t) = square(Point3::new(0.0, 0.0, 0.0));
    let options = BracketOptions { route: BoundRoute::EnclosingSubdivision, max_subdivision_depth: 0 };
    let ledger = certify(&v, &t, &v, &t, options, 0, 1_000_000, QuantizationModel::NotSupplied, None, None, AdmissionEvidence::default());
    assert_eq!(ledger.status, LedgerStatus::Inconclusive);
    assert_eq!(ledger.geometric.status, LedgerStatus::Inconclusive);
    match ledger.geometric.enclosing {
        Metric::Bracket { lower, upper, class, .. } => {
            let d = match ledger.geometric.diagonal {
                Metric::Scalar { value, .. } => value,
                other => panic!("diagonal present, got {other:?}"),
            };
            assert_eq!(class, MeasurementClass::EnclosingBound);
            assert!(lower <= GeometryProfile::g1().upper_threshold(d));
            assert!(upper - lower > GeometryProfile::g1().width_threshold(d));
        }
        Metric::Absent { .. } => {}
        other => panic!("unexpected metric {other:?}"),
    }
}

#[test]
fn sampled_estimate_is_labelled_and_cannot_pass_enclosing_profile() {
    let (v, t) = square(Point3::new(0.0, 0.0, 0.0));
    let options = BracketOptions { route: BoundRoute::SampledVertices, max_subdivision_depth: 0 };
    let uv_ref = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let uv_dec = uv_ref;
    let ledger = certify(
        &v, &t, &v, &t, options, t.len(), 1_000_000, QuantizationModel::Binary32, None,
        Some(AttributePair { units: "uv", reference: &uv_ref, decoded: &uv_dec }),
        AdmissionEvidence { forbidden_semantic_merges: None },
    );
    assert_ne!(ledger.status, LedgerStatus::Passed);
    assert_eq!(ledger.status, LedgerStatus::Inconclusive);
    assert_eq!(ledger.geometric.sample.class(), Some(MeasurementClass::SampledEstimate));
    assert!(matches!(ledger.geometric.enclosing, Metric::Absent { .. }));
    assert_eq!(ledger.geometric.enclosing.to_wire(), MetricWire::Null);
    assert_eq!(ledger.physical.status, LedgerStatus::Unsupported);
    assert_eq!(ledger.physical.metric.to_wire(), MetricWire::Null);
    assert_eq!(ledger.geometric.forbidden_semantic_merges.to_wire(), MetricWire::Null);
    match ledger.attribute.metric {
        Metric::Scalar { class, value, .. } => {
            assert_eq!(class, MeasurementClass::SampledEstimate);
            assert_eq!(value, 0.0);
        }
        other => panic!("attribute measured, got {other:?}"),
    }
}

#[test]
fn protected_drift_and_forbidden_merges_are_separate_from_surface() {
    let (rv, t) = square(Point3::new(0.0, 0.0, 0.0));
    let mut dv = rv.clone();
    dv[0] = Point3::new(0.2, 0.0, 0.0);
    let pref = [0u32];
    let pdec = [0u32];
    let ledger = certify(
        &rv, &t, &dv, &t, enclosing(), t.len(), 1_000_000, QuantizationModel::NotSupplied,
        Some(ProtectedCorrespondence { reference_vertices: &pref, decoded_vertices: &pdec }),
        None,
        AdmissionEvidence { forbidden_semantic_merges: Some(0) },
    );
    assert_eq!(ledger.geometric.forbidden_semantic_merges.to_wire(), MetricWire::Number(0.0));
    match ledger.geometric.protected_drift {
        Metric::Scalar { value, class, .. } => {
            assert_eq!(class, MeasurementClass::Measured);
            assert!((value - 0.2).abs() < 1e-12);
        }
        other => panic!("expected protected drift, got {other:?}"),
    }
    assert_eq!(ledger.geometric.status, LedgerStatus::Failed);
}

#[test]
fn physical_channel_is_unsupported_not_zero_when_p10_evidence_is_absent() {
    let (v, t) = square(Point3::new(0.0, 0.0, 0.0));
    let ledger = certify(
        &v, &t, &v, &t, enclosing(), t.len(), 1_000_000, QuantizationModel::NotSupplied, None, None,
        AdmissionEvidence { forbidden_semantic_merges: None },
    );
    assert_eq!(ledger.physical.status, LedgerStatus::Unsupported);
    assert_eq!(ledger.physical.metric.to_wire(), MetricWire::Null);
    assert!(ledger.physical.note.contains("P10"));
    assert!(ledger.physical.note.contains("zero error"));
    let record = physical_record(None);
    assert_eq!(record.status, LedgerStatus::Unsupported);
    assert_eq!(record.metric.to_wire(), MetricWire::Null);
}

#[test]
fn physical_channel_records_supplied_p10_evidence() {
    use crate::specialized_libs::computational_geometry::physical_authority::{
        evidence_unsupported, ClaimId, EvidenceMethod, PhysicalEvidence,
    };
    use crate::specialized_libs::computational_geometry::production_contracts::ResultStatus;

    let unsupported = evidence_unsupported(ClaimId::ContactFreeTrajectory);
    let record = physical_record(Some(&unsupported));
    assert_eq!(record.status, LedgerStatus::Unsupported);
    assert_eq!(record.metric.to_wire(), MetricWire::Null);
    assert!(record.note.contains("collision"));

    let measured = PhysicalEvidence {
        status: ResultStatus::Failed,
        claim: ClaimId::PostStepPositiveVolumeSample,
        method: EvidenceMethod::ConnectedPbdVolumeProjection,
        value: Some(-1.0e-9),
        units: "m3",
        reason: "non-positive post-step volume sample",
    };
    let failed = physical_record(Some(&measured));
    assert_eq!(failed.status, LedgerStatus::Failed);
    match failed.metric {
        Metric::Scalar { value, class, .. } => {
            assert_eq!(class, MeasurementClass::Measured);
            assert!((value + 1.0e-9).abs() < 1e-18);
        }
        other => panic!("expected measured physical scalar, got {other:?}"),
    }
}
