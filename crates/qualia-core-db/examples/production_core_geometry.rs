//! Bounded P04 integration tracer. This is not production-core qualification.
//! Run with `--out <new-directory> [--target-faces N] [--empty] [--corpus-json PATH] [--driver admitted|geometric]`.
use qualia_core_db::container_10d::{attach_texture_coordinate_section, read_texture_coordinate_section_into};
use qualia_core_db::render::assets::Mesh;
use qualia_core_db::render::compile_10d::{compile_mesh_to_10d, decode_10d_mesh};
use qualia_core_db::specialized_libs::computational_geometry::attribute_transfer::{
    encode_lineage_snapshot, encode_lineage_snapshot_json, lineage_snapshot_len, CollapseTransfer,
    ProtectedDomain, TransferError, TransferRow,
};
use qualia_core_db::specialized_libs::computational_geometry::bvh::BvhNode;
use qualia_core_db::specialized_libs::computational_geometry::decoded_error_ledger::{
    AbsenceReason, AdmissionEvidence, AttributePair, BoundRoute, BracketOptions, DecodedErrorLedger,
    DecodedMeshPair, GeometryProfile, LedgerScratch, LedgerStatus, MeasurementClass, Metric,
    ProtectedCorrespondence, QuantizationModel, SurfacePatch, TriangleMesh, Units,
    certify_decoded_output, plan_ledger_capacity,
};
use qualia_core_db::specialized_libs::computational_geometry::Aabb;
use qualia_core_db::specialized_libs::computational_geometry::decimate_3_semantic::admission::{
    decimate_semantic_admitted_3, AdmissionError, AdmissionIdentity, AdmissionInput, AdmissionLimits, AdmissionOutput,
    AdmissionPolicy, AdmissionReport, AdmissionScratch, AdmissionSlot, DomainLayout, SemanticDomains, Termination,
    TopologyMode,
};
use qualia_core_db::specialized_libs::computational_geometry::decimate_3_semantic::normalized_cost::NormalizedCostProfile;
use qualia_core_db::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    BudgetError, CapacityPlan, ProductionBudget,
};
use qualia_core_db::specialized_libs::computational_geometry::geometry_workspace::Cancellation;
use qualia_core_db::specialized_libs::computational_geometry::production_contracts::{
    CollapseEvent, ContentId, ContractError, CornerDelta, CornerId, FaceDelta, ReasonCode, RemapBuffers,
};
use qualia_core_db::specialized_libs::computational_geometry::ddg_production::{
    algebraic_residual, assemble_cotangent_admitted, BoundaryConditionKind, NullspacePolicy,
    OperatorConventions, OperatorScratch, ResidualRequest, SolverLimits,
};
use qualia_core_db::specialized_libs::computational_geometry::{
    compute_extended_collapse_cost, decimate_qem, DecimateOptions, Point3, SemanticDecimateWeights,
    SemanticVertexAttributes, SparseEntry,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_OUTPUT_BUDGET: usize = 1024 * 1024;
const MAX_OUTPUT_BUDGET: usize = 16 * 1024 * 1024;
const MAX_SCALARS: usize = 65_536;

type ProbeResult<T> = Result<T, (&'static str, String)>;
type Fixture = (Vec<Point3>, Vec<[u32; 3]>, Vec<SemanticVertexAttributes>);

struct LoadedCorpus {
    id: String,
    vertices: Vec<Point3>,
    triangles: Vec<[u32; 3]>,
    attributes: Vec<SemanticVertexAttributes>,
    extent_diagonal: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Driver {
    Admitted,
    GeometricQem,
}

struct LineageBundle {
    binary: Vec<u8>,
    json: Vec<u8>,
    sha256: String,
    live: u64,
    committed_collapses: u64,
}

struct SimplifyResult {
    vertices: Vec<Point3>,
    attributes: Vec<SemanticVertexAttributes>,
    triangles: Vec<[u32; 3]>,
    faces: usize,
    verts: usize,
    collapses: u64,
    termination: String,
    largest_cost: f64,
    admitted_bytes: u64,
    work_used: u64,
    driver: &'static str,
    lineage: Option<LineageBundle>,
    status: &'static str,
}

struct RunSpec {
    target_faces: usize,
    empty: bool,
    corpus: Option<LoadedCorpus>,
    driver: Driver,
}

fn invalid(message: impl Into<String>) -> (&'static str, String) {
    ("invalid_input", message.into())
}

fn validate_input(vertices: &[Point3], triangles: &[[u32; 3]], target: usize) -> ProbeResult<()> {
    if vertices.is_empty() || triangles.is_empty() {
        return Err(invalid("empty fixture"));
    }
    if target > triangles.len() {
        return Err(invalid("target_faces exceeds input faces"));
    }
    Ok(())
}

fn checked(condition: bool, message: &str) -> ProbeResult<()> {
    if condition {
        Ok(())
    } else {
        Err(("oracle_mismatch", message.into()))
    }
}

fn reason_status(code: ReasonCode) -> &'static str {
    match code {
        ReasonCode::CapacityExhausted => "capacity_exhausted",
        ReasonCode::UnsatisfiedConstraint => "unsatisfied_constraint",
        ReasonCode::Cancelled => "cancelled",
        ReasonCode::UnsupportedCapability => "unsupported_capability",
        _ => "invalid_input",
    }
}
fn transfer_status(e: TransferError) -> (&'static str, String) {
    (reason_status(ContractError::from(e).code), format!("{e:?}"))
}
fn admission_status(e: AdmissionError) -> (&'static str, String) {
    match e {
        AdmissionError::Contract(c) => (reason_status(c.code), c.field.into()),
        AdmissionError::Budget(BudgetError::Cancelled) => ("cancelled", "admission cancelled".into()),
        AdmissionError::Budget(b) => ("capacity_exhausted", b.to_string()),
    }
}
fn receipt_status(report: &AdmissionReport, target_faces: usize) -> &'static str {
    match report.termination {
        Termination::ReachedTarget if report.faces <= target_faces => "passed",
        Termination::WorkExhausted | Termination::EventCapacityExhausted => "capacity_exhausted",
        Termination::Cancelled => "cancelled",
        Termination::TransferFailure(code) => reason_status(code),
        _ => "unsatisfied_constraint",
    }
}

fn mesh_content_id(vertices: &[Point3], triangles: &[[u32; 3]]) -> ContentId {
    let mut bytes = Vec::with_capacity(vertices.len() * 24 + triangles.len() * 12);
    for p in vertices {
        bytes.extend_from_slice(&p.x.to_le_bytes());
        bytes.extend_from_slice(&p.y.to_le_bytes());
        bytes.extend_from_slice(&p.z.to_le_bytes());
    }
    for tri in triangles {
        bytes.extend_from_slice(&tri[0].to_le_bytes());
        bytes.extend_from_slice(&tri[1].to_le_bytes());
        bytes.extend_from_slice(&tri[2].to_le_bytes());
    }
    ContentId::sha256(&bytes)
}

fn snapshot_lineage(transfer: &CollapseTransfer<'_>, target: ContentId) -> ProbeResult<LineageBundle> {
    let n = lineage_snapshot_len(transfer).map_err(transfer_status)?;
    let cancel = Cancellation::new();
    let mut budget = ProductionBudget::admit(CapacityPlan::default(), usize::MAX, u64::MAX, &cancel)
        .map_err(|e| ("capacity_exhausted", e.to_string()))?;
    let mut binary = vec![0_u8; n];
    let resource = encode_lineage_snapshot(transfer, target, &mut binary, &mut budget).map_err(transfer_status)?;
    binary.truncate(resource.byte_len as usize);
    let json_cap = n.saturating_mul(16).saturating_add(4096).max(65_536);
    let mut json = vec![0_u8; json_cap];
    let written = encode_lineage_snapshot_json(transfer, target, &mut json, &mut budget).map_err(transfer_status)?;
    json.truncate(written);
    Ok(LineageBundle {
        sha256: sha256(&binary),
        live: resource.element_count.unwrap_or(0),
        committed_collapses: transfer.committed_collapses(),
        binary,
        json,
    })
}

fn simplify(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    attributes: &[SemanticVertexAttributes],
    target_faces: usize,
    driver: Driver,
) -> ProbeResult<SimplifyResult> {
    match driver {
        Driver::Admitted => simplify_admitted(vertices, triangles, attributes, target_faces),
        Driver::GeometricQem => simplify_geometric(vertices, triangles, attributes, target_faces),
    }
}

fn simplify_geometric(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    attributes: &[SemanticVertexAttributes],
    target_faces: usize,
) -> ProbeResult<SimplifyResult> {
    let mut out_vertices = vec![Point3::default(); vertices.len()];
    let mut out_triangles = vec![[0_u32; 3]; triangles.len()];
    let report = decimate_qem(
        vertices,
        triangles,
        DecimateOptions::to_faces(target_faces),
        &mut out_vertices,
        &mut out_triangles,
    )
    .map_err(|e| ("invalid_input", format!("{e:?}")))?;
    out_vertices.truncate(report.vertices);
    out_triangles.truncate(report.faces);
    let out_attributes = if report.collapses == 0 && report.vertices == attributes.len() {
        attributes.to_vec()
    } else {
        vec![SemanticVertexAttributes::default(); report.vertices]
    };
    let termination = if report.stopped_on_error {
        "StoppedOnError".into()
    } else if report.faces <= target_faces {
        "ReachedTarget".into()
    } else {
        "NoLegalCollapses".into()
    };
    Ok(SimplifyResult {
        vertices: out_vertices,
        attributes: out_attributes,
        triangles: out_triangles,
        faces: report.faces,
        verts: report.vertices,
        collapses: report.collapses as u64,
        termination,
        largest_cost: report.max_error,
        admitted_bytes: 0,
        work_used: 0,
        driver: "geometric",
        lineage: None,
        status: if report.faces <= target_faces { "passed" } else { "unsatisfied_constraint" },
    })
}

fn operator_residual_json(vertices: &[Point3], triangles: &[[u32; 3]]) -> Value {
    let n = vertices.len();
    let limits = SolverLimits::default();
    if n == 0 || triangles.is_empty() || n > limits.effective_vertex_cap() || triangles.len() > limits.effective_triangle_cap() {
        return json!({
            "status": "not_measured",
            "reason": "decoded_mesh_exceeds_dense_operator_cap",
            "kind": "algebraic_residual",
            "wire": Value::Null
        });
    }
    let nnz_cap = triangles.len().saturating_mul(9).max(1);
    let mut laplacian = vec![SparseEntry { row: 0, col: 0, value: 0.0 }; nnz_cap];
    let mut mass = vec![0.0; n];
    let mut residual = vec![0.0; n];
    let cancel = Cancellation::new();
    let mut budget = match ProductionBudget::admit(CapacityPlan::default(), usize::MAX, u64::MAX, &cancel) {
        Ok(budget) => budget,
        Err(e) => {
            return json!({
                "status": "not_measured",
                "reason": e.to_string(),
                "kind": "algebraic_residual",
                "wire": Value::Null
            })
        }
    };
    let conventions = OperatorConventions::cotangent(
        BoundaryConditionKind::NeumannNatural,
        NullspacePolicy::ConstantOnConnectedNeumann { pin: None },
    );
    let assembled = match assemble_cotangent_admitted(
        vertices,
        triangles,
        OperatorScratch { laplacian: &mut laplacian, mass: &mut mass, residual: &mut residual },
        conventions,
        limits,
        &mut budget,
    ) {
        Ok(assembled) => assembled,
        Err(e) => {
            return json!({
                "status": "not_measured",
                "reason": e.to_string(),
                "kind": "algebraic_residual",
                "wire": Value::Null
            })
        }
    };
    let values = vec![1.0; n];
    let rhs = vec![0.0; n];
    let fixed = vec![false; n];
    match algebraic_residual(
        n,
        &laplacian[..assembled.nnz],
        &rhs,
        &values,
        &fixed,
        &mut residual,
        ResidualRequest::two_norm(),
        &mut budget,
    ) {
        Ok(report) => json!({
            "status": "measured",
            "kind": "algebraic_residual",
            "norm": "euclidean_two",
            "value": report.value,
            "relative": report.relative,
            "rhs_norm": report.rhs_norm,
            "solution_norm": report.solution_norm,
            "free_count": report.free_count,
            "field": "constant_neumann",
            "conventions": "cotangent_neumann_natural",
            "condition_estimate": assembled.condition_estimate,
            "note": "DdgReport.residual is iterate change; this field is ||b-Ax|| on free DOFs"
        }),
        Err(e) => json!({
            "status": "not_measured",
            "reason": e.to_string(),
            "kind": "algebraic_residual",
            "wire": Value::Null
        }),
    }
}

fn simplify_admitted(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    attributes: &[SemanticVertexAttributes],
    target_faces: usize,
) -> ProbeResult<SimplifyResult> {
    let (n, m) = (vertices.len(), triangles.len());
    let mut semantic_domains = vec![SemanticDomains::default(); n];
    let mut protected = vec![ProtectedDomain::default(); n];
    let mut stable_ids = vec![0_u64; n];
    for i in 0..n {
        let chart = if attributes[i].subpart_tag == 101 { 1 } else { 2 };
        let locked = u64::from(attributes[i].cluster_border_locked);
        semantic_domains[i] = SemanticDomains {
            subpart: attributes[i].subpart_tag as u64,
            uv_chart: chart,
            protected_region: locked,
            ..SemanticDomains::default()
        };
        protected[i] = ProtectedDomain {
            subpart: attributes[i].subpart_tag as u64,
            chart,
            protected: locked,
            ..ProtectedDomain::default()
        };
        stable_ids[i] = i as u64 + 1;
    }
    let identity = AdmissionIdentity {
        run: ContentId::sha256(b"production-core-geometry/run"),
        source: ContentId::sha256(b"production-core-geometry/source"),
        policy: ContentId::sha256(b"production-core-geometry/policy"),
    };
    let cancel = Cancellation::new();
    let mut construct = ProductionBudget::admit(CapacityPlan::default(), usize::MAX, u64::MAX, &cancel)
        .map_err(|e| ("capacity_exhausted", e.to_string()))?;
    let mut rows = vec![TransferRow::default(); n];
    let mut entries = vec![SparseEntry { row: 0, col: 0, value: 0.0 }; n.saturating_mul(n.max(1))];
    let mut transfer = CollapseTransfer::identity(
        identity.run, identity.source, identity.policy, vertices, &protected, &stable_ids, m, n.max(1),
        &mut rows, &mut entries, 1e-9, &mut construct,
    )
    .map_err(transfer_status)?;
    let mut slots = vec![AdmissionSlot::empty(); n.max(m)];
    let mut face_scratch = vec![FaceDelta { original_face: 0, before: [0; 3], after: None }; m];
    let mut corner_scratch = vec![CornerDelta { source: CornerId { face: 0, slot: 0 }, target: None }; m * 3];
    let mut transfer_scratch = vec![0_u8; n.max(1) * 16];
    let mut out_vertices = vec![Point3::default(); n];
    let mut out_attributes = vec![SemanticVertexAttributes::default(); n];
    let mut out_domains = vec![SemanticDomains::default(); n];
    let mut out_triangles = vec![[0_u32; 3]; m];
    let mut lineage = vec![0_u32; n];
    let mut vertex_compact = vec![0_u32; n];
    let mut face_compact = vec![0_u32; m];
    let mut corner_compact = vec![None; m * 3];
    let mut events = vec![None::<CollapseEvent>; n.saturating_sub(1)];
    let report = decimate_semantic_admitted_3(
        AdmissionInput { vertices, attributes, triangles, domains: &semantic_domains, stable_vertex_ids: &stable_ids },
        AdmissionPolicy {
            topology: TopologyMode::StrictManifold,
            domain_layout: DomainLayout::VertexAlignedSplitCorners,
            target_faces,
            preserve_boundary_vertices: true,
            cost: NormalizedCostProfile::default(),
            maximum_normalized_cost: None,
        },
        identity,
        AdmissionLimits { bytes: 16 * 1024 * 1024, work: 1_000_000_000 },
        &cancel,
        AdmissionScratch { slots: &mut slots, faces: &mut face_scratch, corners: &mut corner_scratch, transfer: &mut transfer_scratch },
        AdmissionOutput {
            vertices: &mut out_vertices,
            attributes: &mut out_attributes,
            domains: &mut out_domains,
            triangles: &mut out_triangles,
            remaps: RemapBuffers {
                vertex_lineage: &mut lineage,
                surviving_vertex_compaction: &mut vertex_compact,
                face_compaction: &mut face_compact,
                corner_compaction: &mut corner_compact,
            },
            events: &mut events,
        },
        &mut transfer,
    )
    .map_err(admission_status)?;
    out_vertices.truncate(report.vertices);
    out_attributes.truncate(report.vertices);
    out_triangles.truncate(report.faces);
    let target_id = mesh_content_id(&out_vertices, &out_triangles);
    let lineage = snapshot_lineage(&transfer, target_id)?;
    Ok(SimplifyResult {
        vertices: out_vertices,
        attributes: out_attributes,
        triangles: out_triangles,
        faces: report.faces,
        verts: report.vertices,
        collapses: report.committed_collapses as u64,
        termination: format!("{:?}", report.termination),
        largest_cost: report.largest_cost.total,
        admitted_bytes: report.admitted_bytes as u64,
        work_used: report.work_used,
        driver: "admitted",
        lineage: Some(lineage),
        status: receipt_status(&report, target_faces),
    })
}

fn fixture(empty: bool) -> Fixture {
    if empty {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    // Extent diagonal is exactly 1 before conversion to f32. The tilted plane gives
    // an analytic independent residual and a visible nonzero side-view extent.
    let diagonal = (2.0_f64 + 0.375_f64.powi(2)).sqrt();
    let normal_length = (1.0_f64 + 0.25_f64.powi(2) + 0.125_f64.powi(2)).sqrt();
    // Two independent 3x5 charts share physical seam positions at x=0.5.
    // Left seam UV.u=1, right seam UV.u=0: the UV discontinuity is real.
    let mut vertices = Vec::with_capacity(30);
    let mut attributes = Vec::with_capacity(30);
    for side in 0..2 {
        for j in 0..=4 {
            for i in 0..=2 {
                let x = (side * 2 + i) as f64 / 4.0;
                let y = j as f64 / 4.0;
                vertices.push(Point3::new(x / diagonal, y / diagonal, (0.25 * x + 0.125 * y) / diagonal));
                attributes.push(SemanticVertexAttributes {
                    normal: [-0.25 / normal_length, -0.125 / normal_length, 1.0 / normal_length],
                    uv: [i as f64 / 2.0, y],
                    subpart_tag: if side == 0 { 101 } else { 202 },
                    cluster_border_locked: i == 0 || i == 2 || j == 0 || j == 4,
                    ..Default::default()
                });
            }
        }
    }
    let mut triangles = Vec::with_capacity(32);
    for side in 0..2_u32 {
        for j in 0..4_u32 {
            for i in 0..2_u32 {
                let a = side * 15 + j * 3 + i;
                triangles.extend([[a, a + 1, a + 4], [a, a + 4, a + 3]]);
            }
        }
    }
    (vertices, triangles, attributes)
}

fn policy_probe() -> ProbeResult<Value> {
    let a = SemanticVertexAttributes { subpart_tag: 101, ..Default::default() };
    let b = SemanticVertexAttributes { subpart_tag: 202, ..a };
    let p = Point3::new(0.0, 0.0, 0.0);
    let q = Point3::new(1.0, 0.0, 0.0);
    let finite = SemanticDecimateWeights::default();
    let hard = SemanticDecimateWeights { semantic_boundary_penalty: f64::INFINITY, ..finite };
    checked(
        compute_extended_collapse_cost(p, q, p, 0.0, &a, &b, &finite).is_some(),
        "finite semantic penalty unexpectedly became hard rejection",
    )?;
    checked(
        compute_extended_collapse_cost(p, q, p, 0.0, &a, &b, &hard).is_none(),
        "hard policy admitted mixed labels",
    )?;
    let locked = SemanticVertexAttributes { cluster_border_locked: true, ..a };
    checked(
        compute_extended_collapse_cost(p, q, p, 0.0, &locked, &a, &finite).is_none(),
        "locked endpoint admitted a collapse",
    )?;
    // Retain the existing infinity*zero defect as evidence, without freezing it as desired behavior.
    let compatible_admitted = compute_extended_collapse_cost(p, q, p, 0.0, &a, &a, &hard).is_some();
    Ok(json!({"finite_cross_label_pair_ranked": true, "infinite_cross_label_pair_rejected": true,
        "locked_endpoint_rejected": true, "infinite_compatible_label_pair_admitted": compatible_admitted,
        "infinite_compatible_admission_status": if compatible_admitted { "passed" } else { "failed_existing_api" }}))
}

fn mesh(vertices: &[Point3], triangles: &[[u32; 3]]) -> Mesh {
    let positions: Vec<[f32; 3]> = vertices.iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]).collect();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for point in &positions {
        for k in 0..3 {
            min[k] = min[k].min(point[k]);
            max[k] = max[k].max(point[k]);
        }
    }
    Mesh { positions, triangles: triangles.to_vec(), min, max }
}

fn seal(mesh: &Mesh, attributes: &[SemanticVertexAttributes]) -> ProbeResult<Vec<u8>> {
    let uv: Vec<[f32; 2]> = attributes.iter().map(|a| [a.uv[0] as f32, a.uv[1] as f32]).collect();
    let bytes = compile_mesh_to_10d(mesh).map_err(|e| ("invalid_input", e.to_string()))?;
    attach_texture_coordinate_section(&bytes, &uv).map_err(|e| ("invalid_input", format!("UV01 seal: {e:?}")))
}

fn sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in Sha256::digest(bytes).iter() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 15) as usize] as char);
    }
    encoded
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    (0..3).map(|k| (f64::from(a[k]) - f64::from(b[k])).powi(2)).sum::<f64>().sqrt()
}

fn decoded_points(positions: &[[f32; 3]]) -> Vec<Point3> {
    positions
        .iter()
        .map(|p| Point3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])))
        .collect()
}

fn ledger_status_name(status: LedgerStatus) -> &'static str {
    match status {
        LedgerStatus::Passed => "passed",
        LedgerStatus::Failed => "failed",
        LedgerStatus::Inconclusive => "inconclusive",
        LedgerStatus::Unsupported => "unsupported",
        LedgerStatus::Cancelled => "cancelled",
    }
}

fn class_name(class: MeasurementClass) -> &'static str {
    match class {
        MeasurementClass::SampledEstimate => "sampled_estimate",
        MeasurementClass::Probabilistic => "probabilistic",
        MeasurementClass::EnclosingBound => "enclosing_bound",
        MeasurementClass::ProofUnderAssumptions => "proof_under_assumptions",
        MeasurementClass::Measured => "measured",
    }
}

fn absence_name(reason: AbsenceReason) -> &'static str {
    match reason {
        AbsenceReason::UnsupportedCapability => "unsupported_capability",
        AbsenceReason::NotSupplied => "not_supplied",
        AbsenceReason::Cancelled => "cancelled",
        AbsenceReason::BudgetExhausted => "budget_exhausted",
        AbsenceReason::DomainEmpty => "domain_empty",
        AbsenceReason::IncompleteEnclosure => "incomplete_enclosure",
    }
}

fn units_name(units: Units) -> &'static str {
    match units {
        Units::AssetLength => "asset_length",
        Units::Dimensionless => "dimensionless",
    }
}

fn metric_json(metric: Metric) -> Value {
    match metric {
        Metric::Absent { reason } => json!({
            "status": "absent",
            "reason": absence_name(reason),
            "wire": Value::Null
        }),
        Metric::Scalar { value, units, class } => json!({
            "value": value,
            "units": units_name(units),
            "class": class_name(class)
        }),
        Metric::Bracket { lower, upper, units, class } => json!({
            "lower": lower,
            "upper": upper,
            "units": units_name(units),
            "class": class_name(class)
        }),
    }
}

fn finite_scalar(metric: Metric) -> Option<f64> {
    match metric {
        Metric::Scalar { value, .. } if value.is_finite() => Some(value),
        _ => None,
    }
}

fn has_enclosing_bound(ledger: &DecodedErrorLedger) -> bool {
    matches!(ledger.geometric.enclosing, Metric::Bracket { class: MeasurementClass::EnclosingBound, .. })
}

fn symmetric_surface_json(ledger: &DecodedErrorLedger) -> Value {
    let profile = GeometryProfile::g1();
    let diagonal = finite_scalar(ledger.geometric.diagonal);
    let slack = finite_scalar(ledger.numerical.slack);
    let mut value = json!({
        "status": ledger_status_name(ledger.geometric.status),
        "enclosing": metric_json(ledger.geometric.enclosing),
        "sample": metric_json(ledger.geometric.sample),
        "protected_drift": metric_json(ledger.geometric.protected_drift),
        "forbidden_semantic_merges": metric_json(ledger.geometric.forbidden_semantic_merges),
        "method": ledger.geometric.method,
        "g1": {
            "profile": "g1",
            "decision": ledger_status_name(ledger.geometric.status),
            "upper_over_diagonal": profile.upper_over_diagonal,
            "width_over_diagonal": profile.width_over_diagonal,
            "protected_drift_over_diagonal": profile.protected_drift_over_diagonal,
            "upper_threshold": diagonal.map(|d| profile.upper_threshold(d)),
            "width_threshold": diagonal.map(|d| profile.width_threshold(d)),
            "outward_rounding": ledger.numerical.outward_rounding,
            "slack": metric_json(ledger.numerical.slack),
            "numerical_note": ledger.numerical.note
        },
        "reference_domain": "authored_frozen_triangles",
        "compared_to": "decoded_final_10d_buffers"
    });
    if let Some(d) = diagonal {
        value["diagonal"] = json!(d);
    } else {
        value["diagonal"] = metric_json(ledger.geometric.diagonal);
    }
    if let Some(s) = slack {
        value["slack"] = json!(s);
    }
    match ledger.geometric.enclosing {
        Metric::Bracket { lower, upper, class, units } => {
            value["lower"] = json!(lower);
            value["upper"] = json!(upper);
            value["class"] = json!(class_name(class));
            value["units"] = json!(units_name(units));
        }
        Metric::Absent { reason } => {
            value["class"] = Value::Null;
            value["reason"] = json!(absence_name(reason));
            value["wire"] = Value::Null;
            value["note"] = json!("sampled estimates cannot pass G1 enclosing profile");
        }
        Metric::Scalar { class, .. } => {
            value["class"] = json!(class_name(class));
            value["note"] = json!("sampled estimates cannot pass G1 enclosing profile");
        }
    }
    value
}

fn inconclusive_surface(message: &str) -> Value {
    json!({
        "status": "inconclusive",
        "class": Value::Null,
        "reason": "incomplete_enclosure",
        "wire": Value::Null,
        "message": message,
        "note": "sampled estimates cannot pass G1 enclosing profile",
        "g1": {
            "profile": "g1",
            "decision": "inconclusive",
            "outward_rounding": false
        },
        "reference_domain": "authored_frozen_triangles",
        "compared_to": "decoded_final_10d_buffers"
    })
}

fn certify_decoded_final(
    reference_vertices: &[Point3],
    reference_triangles: &[[u32; 3]],
    decoded_vertices: &[Point3],
    decoded_triangles: &[[u32; 3]],
    attributes: Option<AttributePair<'_>>,
    protected: Option<ProtectedCorrespondence<'_>>,
    forbidden_semantic_merges: Option<u64>,
) -> Result<(DecodedErrorLedger, u64), String> {
    let overlap = reference_triangles.len().max(decoded_triangles.len()).max(1);
    let patches = 64usize;
    let n = overlap;
    let dummy = Aabb::new(Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0));
    let node_len = LedgerScratch::bvh_node_len(n).map_err(|e| e.to_string())?;
    let mut aabbs = vec![dummy; n];
    let mut nodes = vec![BvhNode::default(); node_len];
    let mut prim = vec![0u32; n];
    let mut morton = vec![0u64; n];
    let mut sort = vec![0u32; n];
    let mut stack = vec![0u32; LedgerScratch::bvh_stack_len()];
    let mut overlap_buf = vec![0u32; overlap];
    let mut patch_buf = vec![SurfacePatch::empty(); patches];
    let attribute_scalars = attributes
        .map(|pair| pair.reference.len().saturating_add(pair.decoded.len()))
        .unwrap_or(0);
    let protected_pairs = protected.map(|pair| pair.reference_vertices.len()).unwrap_or(0);
    let plan = plan_ledger_capacity(
        reference_vertices.len(),
        reference_triangles.len(),
        decoded_vertices.len(),
        decoded_triangles.len(),
        overlap,
        patches,
        attribute_scalars,
        protected_pairs,
    )
    .map_err(|e| e.to_string())?;
    let cancel = Cancellation::new();
    let mut budget = ProductionBudget::admit(plan, usize::MAX, 1_000_000_000, &cancel).map_err(|e| e.to_string())?;
    let mut scratch = LedgerScratch {
        aabbs: &mut aabbs,
        nodes: &mut nodes,
        prim_indices: &mut prim,
        morton_codes: &mut morton,
        sort_indices: &mut sort,
        stack: &mut stack,
        overlap: &mut overlap_buf,
        patches: &mut patch_buf,
    };
    let ledger = certify_decoded_output(
        DecodedMeshPair {
            reference: TriangleMesh { vertices: reference_vertices, triangles: reference_triangles },
            decoded: TriangleMesh { vertices: decoded_vertices, triangles: decoded_triangles },
        },
        attributes,
        protected,
        AdmissionEvidence { forbidden_semantic_merges },
        QuantizationModel::Binary32,
        GeometryProfile::g1(),
        BracketOptions { route: BoundRoute::EnclosingSubdivision, max_subdivision_depth: 12 },
        &mut scratch,
        &mut budget,
    )
    .map_err(|e| e.to_string())?;
    Ok((ledger, budget.work_used()))
}

struct Artifact {
    name: &'static str,
    bytes: Vec<u8>,
}

fn json_artifact(name: &'static str, value: &impl serde::Serialize) -> ProbeResult<Artifact> {
    Ok(Artifact { name, bytes: serde_json::to_vec_pretty(value).map_err(|e| invalid(e.to_string()))? })
}

fn json_f64(value: &Value, field: &str) -> ProbeResult<f64> {
    value
        .as_f64()
        .or_else(|| value.as_u64().map(|n| n as f64))
        .or_else(|| value.as_i64().map(|n| n as f64))
        .ok_or_else(|| invalid(format!("non-numeric {field}")))
}

fn mesh_payload<'a>(root: &'a Value) -> ProbeResult<&'a Value> {
    let data = root.get("data").ok_or_else(|| invalid("corpus JSON missing data"))?;
    if data.get("kind").and_then(Value::as_str) == Some("representation_candidate") {
        data.get("input")
            .ok_or_else(|| invalid("representation_candidate missing input mesh"))
    } else {
        Ok(data)
    }
}

fn load_corpus_from_value(root: &Value) -> ProbeResult<LoadedCorpus> {
    let id = root
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unnamed-corpus-fixture")
        .to_string();
    let payload = mesh_payload(root)?;
    let vertex_list = payload
        .get("vertices")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("corpus JSON missing vertices"))?;
    let triangle_list = payload
        .get("triangles")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("corpus JSON missing triangles"))?;
    if vertex_list.is_empty() || triangle_list.is_empty() {
        return Err(invalid("corpus JSON empty mesh"));
    }
    let mut vertices = Vec::with_capacity(vertex_list.len());
    for (i, item) in vertex_list.iter().enumerate() {
        let xyz = item
            .as_array()
            .ok_or_else(|| invalid(format!("vertex {i} is not an array")))?;
        if xyz.len() != 3 {
            return Err(invalid(format!("vertex {i} does not have 3 coordinates")));
        }
        let x = json_f64(&xyz[0], "x")?;
        let y = json_f64(&xyz[1], "y")?;
        let z = json_f64(&xyz[2], "z")?;
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            return Err(invalid(format!("vertex {i} is non-finite")));
        }
        vertices.push(Point3::new(x, y, z));
    }
    let n = vertices.len();
    let mut triangles = Vec::with_capacity(triangle_list.len());
    for (i, item) in triangle_list.iter().enumerate() {
        let idx = item
            .as_array()
            .ok_or_else(|| invalid(format!("triangle {i} is not an array")))?;
        if idx.len() != 3 {
            return Err(invalid(format!("triangle {i} does not have 3 indices")));
        }
        let mut tri = [0u32; 3];
        for k in 0..3 {
            let v = idx[k]
                .as_u64()
                .or_else(|| idx[k].as_i64().and_then(|n| u64::try_from(n).ok()))
                .ok_or_else(|| invalid(format!("triangle {i} index")))?;
            if v >= n as u64 {
                return Err(invalid(format!("triangle {i} index {v} out of range")));
            }
            tri[k] = v as u32;
        }
        triangles.push(tri);
    }
    let uvs = payload.get("uvs").and_then(Value::as_array);
    let subparts = payload
        .get("subpart_tags")
        .or_else(|| payload.get("domains"))
        .and_then(Value::as_array);
    let mut locked = vec![false; n];
    if let Some(list) = payload.get("locked_vertices").and_then(Value::as_array) {
        for item in list {
            let idx = item
                .as_u64()
                .or_else(|| item.as_i64().and_then(|n| u64::try_from(n).ok()))
                .ok_or_else(|| invalid("locked vertex index"))?;
            if idx >= n as u64 {
                return Err(invalid("locked vertex outside vertex count"));
            }
            locked[idx as usize] = true;
        }
    }
    let mut attributes = Vec::with_capacity(n);
    for i in 0..n {
        let uv = if let Some(uvs) = uvs {
            if uvs.len() != n {
                return Err(invalid("uvs length does not match vertex count"));
            }
            let pair = uvs[i]
                .as_array()
                .ok_or_else(|| invalid("uv is not an array"))?;
            if pair.len() != 2 {
                return Err(invalid("uv does not have 2 coordinates"));
            }
            [json_f64(&pair[0], "u")?, json_f64(&pair[1], "v")?]
        } else {
            [0.0, 0.0]
        };
        let subpart_tag = if let Some(tags) = subparts {
            if tags.len() != n {
                return Err(invalid("subpart_tags length does not match vertex count"));
            }
            tags[i]
                .as_u64()
                .or_else(|| tags[i].as_i64().and_then(|n| u64::try_from(n).ok()))
                .ok_or_else(|| invalid("subpart tag"))? as u32
        } else {
            0
        };
        attributes.push(SemanticVertexAttributes {
            uv,
            subpart_tag,
            cluster_border_locked: locked[i],
            ..SemanticVertexAttributes::default()
        });
    }
    let extent_diagonal = root.pointer("/oracle/scales/extent_diagonal").and_then(Value::as_f64);
    Ok(LoadedCorpus {
        id,
        vertices,
        triangles,
        attributes,
        extent_diagonal,
    })
}

fn load_corpus_from_str(text: &str) -> ProbeResult<LoadedCorpus> {
    let root: Value = serde_json::from_str(text).map_err(|e| invalid(format!("corpus JSON parse: {e}")))?;
    load_corpus_from_value(&root)
}

fn load_corpus_from_path(path: &Path) -> ProbeResult<LoadedCorpus> {
    let text = fs::read_to_string(path).map_err(|e| invalid(format!("read corpus JSON {}: {e}", path.display())))?;
    load_corpus_from_str(&text)
}

fn attribute_records(attributes: &[SemanticVertexAttributes]) -> Vec<Value> {
    attributes
        .iter()
        .map(|a| {
            json!({
                "normal": a.normal, "uv": a.uv, "subpart_tag": a.subpart_tag,
                "skeleton_joint_id": a.skeleton_joint_id, "affordance_boundary": a.affordance_boundary,
                "cluster_border_locked": a.cluster_border_locked, "bifurcation_sensitivity": a.bifurcation_sensitivity,
            })
        })
        .collect()
}

fn run(target_faces: usize, empty: bool) -> ProbeResult<(Vec<Artifact>, Value)> {
    execute_run(RunSpec { target_faces, empty, corpus: None, driver: Driver::Admitted })
}

fn execute_run(spec: RunSpec) -> ProbeResult<(Vec<Artifact>, Value)> {
    let total_start = Instant::now();
    let policy_probes = policy_probe()?;
    let builtin = spec.corpus.is_none() && !spec.empty;
    let (fixture_name, consumes_corpus_json, extent_diagonal, vertices, triangles, attributes) =
        if let Some(corpus) = spec.corpus {
            (
                corpus.id,
                true,
                corpus.extent_diagonal.unwrap_or(0.0),
                corpus.vertices,
                corpus.triangles,
                corpus.attributes,
            )
        } else {
            let (vertices, triangles, attributes) = fixture(spec.empty);
            (
                "normalized-tilted-two-chart-grid".to_string(),
                false,
                1.0,
                vertices,
                triangles,
                attributes,
            )
        };
    let target_faces = spec.target_faces;
    validate_input(&vertices, &triangles, target_faces)?;
    if builtin {
        for j in 0..=4 {
            let (left, right) = (j * 3 + 2, 15 + j * 3);
            checked(
                vertices[left] == vertices[right]
                    && attributes[left].uv[0] == 1.0
                    && attributes[right].uv[0] == 0.0
                    && attributes[left].subpart_tag != attributes[right].subpart_tag,
                "fixture is missing its duplicated UV seam",
            )?;
        }
    }
    let simplify_start = Instant::now();
    let simplified = simplify(&vertices, &triangles, &attributes, target_faces, spec.driver)?;
    let simplify_ms = simplify_start.elapsed().as_secs_f64() * 1000.0;
    let out_vertices = simplified.vertices;
    let out_attributes = simplified.attributes;
    let out_triangles = simplified.triangles;
    let admitted = spec.driver == Driver::Admitted;
    checked(!out_vertices.is_empty() && !out_triangles.is_empty(), "empty simplification output")?;
    // Independent checks use the authored domain and exact locked input locations.
    let seam_x = vertices.get(2).map(|p| p.x).unwrap_or(0.0);
    let forbidden_label_vertices = if builtin {
        out_vertices
            .iter()
            .zip(&out_attributes)
            .filter(|(p, a)| match a.subpart_tag {
                101 => p.x > seam_x,
                202 => p.x < seam_x,
                _ => true,
            })
            .count()
    } else {
        0
    };
    if builtin && admitted {
        checked(forbidden_label_vertices == 0, "output label escaped its authored half-plane")?;
    }
    let mixed_domain_triangles = out_triangles
        .iter()
        .filter(|tri| {
            let labels = tri.map(|i| out_attributes[i as usize].subpart_tag);
            labels[0] != labels[1] || labels[1] != labels[2]
        })
        .count();
    if admitted {
        checked(mixed_domain_triangles == 0, "output triangle crosses semantic domains")?;
    }
    let mut protected_count = 0_usize;
    for (p, a) in vertices.iter().zip(&attributes).filter(|(_, a)| a.cluster_border_locked) {
        protected_count += 1;
        if admitted {
            checked(
                out_vertices.iter().zip(&out_attributes).any(|(q, b)| p == q && a == b),
                "locked vertex or its attributes changed",
            )?;
        }
    }
    let source_mesh = mesh(&vertices, &triangles);
    let simplified_mesh = mesh(&out_vertices, &out_triangles);
    let driver_name = simplified.driver;
    let result_faces = simplified.faces;
    let result_verts = simplified.verts;
    let result_collapses = simplified.collapses;
    let result_termination = simplified.termination.clone();
    let result_largest_cost = simplified.largest_cost;
    let result_admitted_bytes = simplified.admitted_bytes;
    let result_work_used = simplified.work_used;
    let result_status = simplified.status;
    let lineage = simplified.lineage;
    let seal_start = Instant::now();
    let source = seal(&source_mesh, &attributes)?;
    let simplified = seal(&simplified_mesh, &out_attributes)?;
    let decoded = decode_10d_mesh(&simplified).map_err(|e| invalid(e.to_string()))?;
    let decoded_source = decode_10d_mesh(&source).map_err(|e| invalid(e.to_string()))?;
    checked(decoded.triangles == simplified_mesh.triangles, "triangle codec roundtrip changed topology")?;
    checked(decoded_source.triangles == source_mesh.triangles, "source codec roundtrip changed topology")?;
    checked(
        decoded.positions.len() == simplified_mesh.positions.len()
            && decoded_source.positions.len() == source_mesh.positions.len(),
        "codec changed vertex count",
    )?;
    checked(
        decoded.positions.len() * 3 <= MAX_SCALARS && decoded.positions.iter().flatten().all(|v| v.is_finite()),
        "decoded vertices exceed scalar cap or contain nonfinite values",
    )?;
    let mut uv = vec![[0.0_f32; 2]; decoded.positions.len()];
    read_texture_coordinate_section_into(&simplified, decoded.positions.len(), &mut uv)
        .map_err(|e| invalid(format!("UV01 read: {e:?}")))?;
    let expected_uv: Vec<[f32; 2]> = out_attributes.iter().map(|a| [a.uv[0] as f32, a.uv[1] as f32]).collect();
    checked(uv == expected_uv, "UV01 roundtrip changed authored coordinates")?;
    let seal_roundtrip_ms = seal_start.elapsed().as_secs_f64() * 1000.0;
    let max_quantization =
        simplified_mesh.positions.iter().zip(&decoded.positions).map(|(&a, &b)| distance(a, b)).fold(0.0_f64, f64::max);
    let max_source_quantization = source_mesh
        .positions
        .iter()
        .zip(&decoded_source.positions)
        .map(|(&a, &b)| distance(a, b))
        .fold(0.0_f64, f64::max);
    let max_plane_residual = if builtin {
        out_vertices
            .iter()
            .map(|p| (p.z - 0.25 * p.x - 0.125 * p.y).abs() / (1.0_f64 + 0.25_f64.powi(2) + 0.125_f64.powi(2)).sqrt())
            .fold(0.0_f64, f64::max)
    } else {
        f64::NAN
    };
    let max_decoded_locked_displacement = out_attributes
        .iter()
        .enumerate()
        .filter(|(_, a)| a.cluster_border_locked)
        .map(|(i, _)| distance(simplified_mesh.positions[i], decoded.positions[i]))
        .fold(0.0_f64, f64::max);
    let decoded_vertices = decoded_points(&decoded.positions);
    let reference_uv: Vec<f64> = attributes.iter().flat_map(|a| a.uv).collect();
    let decoded_uv: Vec<f64> = uv.iter().flat_map(|p| [f64::from(p[0]), f64::from(p[1])]).collect();
    let attribute_pair = if reference_uv.len() == decoded_uv.len() {
        Some(AttributePair { units: "uv", reference: &reference_uv, decoded: &decoded_uv })
    } else {
        None
    };
    let mut protected_ref = Vec::new();
    let mut protected_dec = Vec::new();
    for (i, (p, a)) in vertices.iter().zip(&attributes).enumerate() {
        if !a.cluster_border_locked {
            continue;
        }
        if let Some(j) = out_vertices.iter().position(|q| q == p) {
            protected_ref.push(i as u32);
            protected_dec.push(j as u32);
        }
    }
    let protected = if protected_ref.is_empty() {
        None
    } else {
        Some(ProtectedCorrespondence {
            reference_vertices: &protected_ref,
            decoded_vertices: &protected_dec,
        })
    };
    let enclosing_start = Instant::now();
    let enclosing_result = certify_decoded_final(
        &vertices,
        &triangles,
        &decoded_vertices,
        &decoded.triangles,
        attribute_pair,
        protected,
        Some((forbidden_label_vertices + mixed_domain_triangles) as u64),
    );
    let enclosing_ms = enclosing_start.elapsed().as_secs_f64() * 1000.0;
    let (symmetric_surface_deviation, enclosing_complete, enclosing_work_used) = match enclosing_result {
        Ok((ledger, work_used)) => {
            (symmetric_surface_json(&ledger), has_enclosing_bound(&ledger), json!(work_used))
        }
        Err(message) => (inconclusive_surface(&message), false, Value::Null),
    };
    let operator_residual = operator_residual_json(&decoded_vertices, &decoded.triangles);
    let collapse_provenance = match &lineage {
        Some(bundle) => json!({
            "status": "present",
            "schema": "qualia.collapse-lineage/QCLN0001",
            "sha256": bundle.sha256,
            "live": bundle.live,
            "committed_collapses": bundle.committed_collapses,
            "files": ["collapse-provenance.bin", "collapse-provenance.json"],
            "note": "hashed QCLN0001 snapshot; hashing is not collapse-provenance qualification"
        }),
        None => json!({
            "status": "not_supplied",
            "reason": "geometric_qem_has_no_collapse_transfer",
            "wire": Value::Null
        }),
    };
    let mut artifacts = vec![
        Artifact { name: "source.10d", bytes: source },
        Artifact { name: "simplified.10d", bytes: simplified },
        json_artifact("source-attributes.json", &attribute_records(&attributes))?,
        json_artifact("decoded-vertices.json", &decoded.positions)?,
        json_artifact("decoded-triangles.json", &decoded.triangles)?,
        json_artifact("attributes.json", &attribute_records(&out_attributes))?,
    ];
    if let Some(bundle) = lineage {
        artifacts.push(Artifact { name: "collapse-provenance.bin", bytes: bundle.binary });
        artifacts.push(Artifact { name: "collapse-provenance.json", bytes: bundle.json });
    }
    let hashes: Vec<Value> =
        artifacts.iter().map(|a| json!({"file": a.name, "sha256": sha256(&a.bytes), "bytes": a.bytes.len()})).collect();
    let status = result_status;
    let mut limitations = vec![
        "integration tracer only".to_string(),
        "labels retained in attributes.json rather than a new .10d section".to_string(),
        "non-ReachedTarget reports are a committed prefix, not an accepted asset".to_string(),
    ];
    if !enclosing_complete {
        limitations.push("whole-surface enclosure incomplete; sampled estimates cannot pass G1".into());
    }
    limitations.push("no GPU stage in this executable".into());
    limitations.push("physical_response not measured; P10 PhysicalEvidence was not supplied".into());
    limitations.push(
        "existing infinite-penalty path may reject compatible labels via infinity times zero; see policy probe".into(),
    );
    if admitted {
        limitations.push("collapse-provenance is a hashed QCLN0001 snapshot; hashing is not qualification".into());
    } else {
        limitations.push("geometric QEM ablation; semantic domain gates were not applied".into());
        limitations.push("collapse-provenance not_supplied; geometric driver has no CollapseTransfer".into());
    }
    limitations.push("operator residual is constant-Neumann ||b-Ax|| on decoded mesh; not qualification".into());
    let receipt = json!({
        "schema": "qualia.production-core.geometry-tracer.v1", "status": status,
        "fixture": {"name": fixture_name, "input_vertices": vertices.len(),
            "input_faces": triangles.len(), "extent_diagonal": extent_diagonal,
            "consumes_corpus_json": consumes_corpus_json,
            "source": if consumes_corpus_json { "corpus_json" } else { "builtin" },
            "subpart_tags": [101, 202],
            "uv_seam": if builtin {
                Value::String("5 coincident vertex pairs; left u=1 and right u=0; disconnected chart topology".into())
            } else {
                Value::Null
            }},
        "policy": {"driver": driver_name,
            "semantic_transition": if admitted { "hard_reject_admitted_domains" } else { "not_applied_geometric_ablation" },
            "protected": "perimeter_and_center_seam",
            "transfer_gate": if admitted { "p07_collapse_transfer" } else { "not_supplied" },
            "cost_helper_probes": policy_probes},
        "result": {"target_faces": target_faces, "vertices": result_verts, "faces": result_faces,
            "collapses": result_collapses, "termination": result_termination, "driver": driver_name,
            "protected_vertices_preserved": protected_count,
            "forbidden_label_vertices": forbidden_label_vertices, "mixed_domain_triangles": mixed_domain_triangles,
            "locked_precompile_displacement": 0.0},
        "metrics": {
            "largest_normalized_collapse_cost": {"value": result_largest_cost, "kind": if admitted { "ranking_cost_not_distance" } else { "qem_max_error_not_distance" }},
            "analytic_plane_residual": if builtin {
                json!({"value": max_plane_residual, "units": "normalized_world", "kind": "vertex_sample"})
            } else {
                json!({"status": "not_measured", "reason": "builtin tilted-plane residual applies only to the two-chart grid"})
            },
            "source_codec_position_error": {"value": max_source_quantization, "units": "normalized_world", "kind": "paired_vertex_max"},
            "simplified_codec_position_error": {"value": max_quantization, "units": "normalized_world", "kind": "paired_vertex_max"},
            "decoded_locked_position_error": {"value": max_decoded_locked_displacement, "units": "normalized_world", "kind": "paired_vertex_max"},
            "uv01_exact_f32_roundtrip": true,
            "symmetric_surface_deviation": symmetric_surface_deviation,
            "collapse_provenance": collapse_provenance,
            "operator_residual": operator_residual,
            "physical_response": {
                "status": "not_measured",
                "reason": "p10_physical_evidence_not_supplied",
                "wire": Value::Null
            }},
        "timings_ms": {"admitted_decimate": simplify_ms, "seal_and_roundtrip": seal_roundtrip_ms,
            "decoded_enclosing_bound": enclosing_ms,
            "geometry_probe_total": total_start.elapsed().as_secs_f64()*1000.0},
        "resources": {"fixture_vertices_cap": vertices.len(), "fixture_faces_cap": triangles.len(),
            "decoded_scalar_cap": MAX_SCALARS, "admitted_bytes": result_admitted_bytes, "work_used": result_work_used,
            "enclosing_work_used": enclosing_work_used},
        "artifacts": hashes,
        "limitations": limitations
    });
    artifacts.push(json_artifact("geometry-receipt.json", &receipt)?);
    Ok((artifacts, receipt))
}

// Only paths created by this invocation are removed on error/unwind. No recursive cleanup.
struct OutputGuard {
    directory: PathBuf,
    files: Vec<PathBuf>,
    retained: bool,
}
impl Drop for OutputGuard {
    fn drop(&mut self) {
        if !self.retained {
            for path in &self.files {
                let _ = fs::remove_file(path);
            }
            let _ = fs::remove_dir(&self.directory);
        }
    }
}

fn publish(out: &Path, artifacts: &[Artifact], budget: usize) -> ProbeResult<()> {
    let total = artifacts
        .iter()
        .try_fold(0_usize, |n, a| n.checked_add(a.bytes.len()))
        .ok_or(("capacity_exhausted", "output size overflow".into()))?;
    if total > budget {
        return Err(("capacity_exhausted", format!("output bytes {total} exceed budget {budget}")));
    }
    // create_dir is the atomic create-only reservation: never reuse an output directory.
    fs::create_dir(out).map_err(|e| invalid(format!("create-only output {}: {e}", out.display())))?;
    let mut owner = OutputGuard { directory: out.to_owned(), files: Vec::new(), retained: false };
    for artifact in artifacts {
        let path = out.join(artifact.name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| invalid(format!("create {}: {e}", artifact.name)))?;
        owner.files.push(path);
        file.write_all(&artifact.bytes).map_err(|e| invalid(format!("write {}: {e}", artifact.name)))?;
    }
    owner.retained = true;
    Ok(())
}

fn execute() -> ProbeResult<Value> {
    let mut out: Option<PathBuf> = None;
    let mut target = 24_usize;
    let mut target_specified = false;
    let mut empty = false;
    let mut budget = DEFAULT_OUTPUT_BUDGET;
    let mut corpus_path: Option<PathBuf> = None;
    let mut driver = Driver::Admitted;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = Some(PathBuf::from(args.next().ok_or_else(|| invalid("missing --out value"))?)),
            "--target-faces" => {
                target = args
                    .next()
                    .ok_or_else(|| invalid("missing target"))?
                    .parse()
                    .map_err(|_| invalid("invalid target"))?;
                target_specified = true;
            }
            "--empty" => empty = true,
            "--driver" => {
                let value = args.next().ok_or_else(|| invalid("missing --driver value"))?;
                driver = match value.as_str() {
                    "admitted" | "semantic" => Driver::Admitted,
                    "geometric" | "qem" => Driver::GeometricQem,
                    _ => return Err(invalid("driver must be admitted or geometric")),
                };
            }
            "--corpus-json" => {
                corpus_path = Some(PathBuf::from(
                    args.next().ok_or_else(|| invalid("missing --corpus-json value"))?,
                ))
            }
            "--output-byte-budget" => {
                budget = args
                    .next()
                    .ok_or_else(|| invalid("missing budget"))?
                    .parse()
                    .map_err(|_| invalid("invalid budget"))?
            }
            _ => return Err(invalid(format!("unknown argument: {arg}"))),
        }
    }
    let out = out.ok_or_else(|| invalid("usage: production_core_geometry --out <new-directory> [--target-faces N] [--empty] [--corpus-json PATH] [--driver admitted|geometric] [--output-byte-budget N]"))?;
    if budget == 0 || budget > MAX_OUTPUT_BUDGET {
        return Err(invalid("output-byte-budget outside 1..=16777216"));
    }
    if empty && corpus_path.is_some() {
        return Err(invalid("--empty cannot be combined with --corpus-json"));
    }
    let corpus = match corpus_path {
        Some(path) => Some(load_corpus_from_path(&path)?),
        None => None,
    };
    if let Some(loaded) = &corpus {
        if !target_specified {
            target = loaded.triangles.len();
        }
    }
    let (artifacts, receipt) = execute_run(RunSpec { target_faces: target, empty, corpus, driver })?;
    publish(&out, &artifacts, budget)?;
    Ok(json!({"status": receipt["status"], "output_directory": out, "receipt": "geometry-receipt.json"}))
}

fn main() {
    match execute() {
        Ok(value) => {
            println!("{value}");
            if value["status"] != "passed" {
                std::process::exit(2);
            }
        }
        Err((status, message)) => {
            eprintln!("{}", json!({"status": status, "message": message}));
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_impossible_target_are_not_success() {
        assert_eq!(run(24, true).err().unwrap().0, "invalid_input");
        assert_eq!(run(33, false).err().unwrap().0, "invalid_input");
        assert_eq!(run(0, false).unwrap().1["status"], "unsatisfied_constraint");
    }

    #[test]
    fn corpus_json_loads_triangle_mesh_and_identity_candidate() {
        let plane = r#"{"id":"analytic-plane-scale-1","data":{"kind":"triangle_mesh","vertices":[[0,0,0],[1,0,0],[1,1,0],[0,1,0]],"triangles":[[0,1,2],[0,2,3]]},"oracle":{"scales":{"extent_diagonal":1.4142135623730951}}}"#;
        let loaded = load_corpus_from_str(plane).unwrap();
        assert_eq!(loaded.id, "analytic-plane-scale-1");
        assert_eq!(loaded.vertices.len(), 4);
        assert_eq!(loaded.triangles.len(), 2);
        assert!((loaded.extent_diagonal.unwrap() - 1.4142135623730951).abs() < 1e-15);

        let identity = r#"{"id":"identity-two-subpart-seam","data":{"kind":"representation_candidate","input":{"vertices":[[0,0,0],[0.5,0,0],[0.5,1,0],[0.5,0,0],[1,0,0],[0.5,1,0]],"triangles":[[0,1,2],[3,4,5]],"uvs":[[0,0],[1,0],[1,1],[0,0],[1,0],[1,1]],"subpart_tags":[101,101,101,202,202,202],"locked_vertices":[0]}}}"#;
        let seam = load_corpus_from_str(identity).unwrap();
        assert_eq!(seam.vertices.len(), 6);
        assert_eq!(seam.attributes[0].subpart_tag, 101);
        assert_eq!(seam.attributes[3].subpart_tag, 202);
        assert!(seam.attributes[0].cluster_border_locked);
        assert!(!seam.attributes[1].cluster_border_locked);
        assert_eq!(seam.attributes[2].uv[0], 1.0);

        let (_, receipt) = execute_run(RunSpec {
            target_faces: 2,
            empty: false,
            corpus: Some(loaded),
            driver: Driver::Admitted,
        })
        .unwrap();
        assert_eq!(receipt["fixture"]["name"], "analytic-plane-scale-1");
        assert_eq!(receipt["fixture"]["consumes_corpus_json"], true);
        assert_eq!(receipt["fixture"]["source"], "corpus_json");
        assert_eq!(receipt["metrics"]["analytic_plane_residual"]["status"], "not_measured");
        assert_ne!(receipt["metrics"]["symmetric_surface_deviation"]["status"], "not_measured");
        assert_eq!(receipt["metrics"]["physical_response"]["status"], "not_measured");
        assert_eq!(receipt["metrics"]["physical_response"]["wire"], Value::Null);
    }

    #[test]
    fn identity_and_reduction_keep_locks_labels_and_uv_roundtrip() {
        let (_, identity) = run(32, false).unwrap();
        assert_eq!(identity["status"], "passed");
        assert_eq!(identity["result"]["collapses"], 0);
        let (_, reduced) = run(24, false).unwrap();
        assert_eq!(reduced["status"], "passed");
        assert!(reduced["result"]["collapses"].as_u64().unwrap() > 0);
    }

    #[test]
    fn identity_decoded_output_reports_enclosing_g1() {
        let (_, identity) = run(32, false).unwrap();
        let surface = &identity["metrics"]["symmetric_surface_deviation"];
        assert_eq!(surface["compared_to"], "decoded_final_10d_buffers");
        assert_eq!(surface["reference_domain"], "authored_frozen_triangles");
        assert_eq!(surface["status"], "passed");
        assert_eq!(surface["class"], "enclosing_bound");
        assert_eq!(surface["g1"]["decision"], "passed");
        assert_eq!(surface["g1"]["outward_rounding"], false);
        assert!(surface["lower"].as_f64().is_some());
        assert!(surface["upper"].as_f64().is_some());
        assert!(surface["diagonal"].as_f64().unwrap() > 0.0);
        assert!(surface["slack"].as_f64().unwrap() > 0.0);
        assert_eq!(surface["sample"]["class"], "sampled_estimate");
        assert_eq!(identity["metrics"]["physical_response"]["status"], "not_measured");
        assert_eq!(identity["metrics"]["physical_response"]["wire"], Value::Null);
        let limitations = identity["limitations"].as_array().unwrap();
        assert!(!limitations.iter().any(|v| {
            v.as_str() == Some("no independent whole-surface distance certificate")
        }));
        let (_, reduced) = run(24, false).unwrap();
        let reduced_surface = &reduced["metrics"]["symmetric_surface_deviation"];
        assert_ne!(reduced_surface["status"], "not_measured");
        assert_ne!(reduced_surface["class"], "sampled_estimate");
        assert_eq!(reduced["metrics"]["physical_response"]["status"], "not_measured");
    }

    #[test]
    fn retention_is_create_only_and_budgeted() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("retained");
        let artifacts = [Artifact { name: "probe.json", bytes: b"[]".to_vec() }];
        assert_eq!(publish(&out, &artifacts, 1).unwrap_err().0, "capacity_exhausted");
        assert!(!out.exists());
        publish(&out, &artifacts, 2).unwrap();
        assert!(publish(&out, &artifacts, 2).is_err());
        assert_eq!(fs::read(out.join("probe.json")).unwrap(), b"[]");
    }

    #[test]
    fn output_owner_cleans_error_and_unwind() {
        let parent = tempfile::tempdir().unwrap();
        let out = parent.path().join("failed");
        let artifacts = [Artifact { name: "same", bytes: vec![1] }, Artifact { name: "same", bytes: vec![2] }];
        assert!(publish(&out, &artifacts, 2).is_err());
        assert!(!out.exists());
        let out = parent.path().join("unwind");
        fs::create_dir(&out).unwrap();
        let path = out.join("owned");
        fs::write(&path, b"owned").unwrap();
        let result = std::panic::catch_unwind(|| {
            let _owner = OutputGuard { directory: out.clone(), files: vec![path], retained: false };
            panic!("controlled cleanup probe");
        });
        assert!(result.is_err());
        assert!(!out.exists());
    }

    #[test]
    fn admitted_identity_emits_qcln0001_lineage() {
        let (artifacts, identity) = run(32, false).unwrap();
        assert_eq!(identity["status"], "passed");
        assert_eq!(identity["policy"]["driver"], "admitted");
        assert_eq!(identity["metrics"]["collapse_provenance"]["status"], "present");
        assert_eq!(identity["metrics"]["collapse_provenance"]["schema"], "qualia.collapse-lineage/QCLN0001");
        assert!(identity["metrics"]["collapse_provenance"]["live"].as_u64().unwrap() > 0);
        let bin = artifacts.iter().find(|a| a.name == "collapse-provenance.bin").unwrap();
        assert_eq!(&bin.bytes[..8], b"QCLN0001");
        let json = artifacts.iter().find(|a| a.name == "collapse-provenance.json").unwrap();
        let parsed: Value = serde_json::from_slice(&json.bytes).unwrap();
        assert_eq!(parsed["schema"], "qualia.collapse-lineage/QCLN0001");
        assert_eq!(identity["metrics"]["operator_residual"]["status"], "measured");
        assert_eq!(identity["metrics"]["operator_residual"]["kind"], "algebraic_residual");
        assert_eq!(identity["metrics"]["operator_residual"]["field"], "constant_neumann");
        assert!(identity["metrics"]["operator_residual"]["value"].as_f64().unwrap() < 1e-8);
    }

    #[test]
    fn geometric_driver_is_qem_ablation_without_lineage() {
        let (artifacts, receipt) = execute_run(RunSpec {
            target_faces: 24,
            empty: false,
            corpus: None,
            driver: Driver::GeometricQem,
        })
        .unwrap();
        assert_eq!(receipt["policy"]["driver"], "geometric");
        assert_eq!(receipt["result"]["driver"], "geometric");
        assert_eq!(receipt["metrics"]["collapse_provenance"]["status"], "not_supplied");
        assert_eq!(receipt["metrics"]["collapse_provenance"]["wire"], Value::Null);
        assert!(artifacts.iter().all(|a| a.name != "collapse-provenance.bin"));
        assert_eq!(receipt["metrics"]["physical_response"]["status"], "not_measured");
        assert_eq!(receipt["metrics"]["operator_residual"]["kind"], "algebraic_residual");
        assert_ne!(receipt["metrics"]["operator_residual"]["status"], "not_measured");
    }
}
