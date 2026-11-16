//! Bounded P04 integration tracer. This is not production-core qualification.
//! Run with `--out <new-directory> [--target-faces N] [--empty] [--corpus-json PATH]`.
use qualia_core_db::container_10d::{attach_texture_coordinate_section, read_texture_coordinate_section_into};
use qualia_core_db::render::assets::Mesh;
use qualia_core_db::render::compile_10d::{compile_mesh_to_10d, decode_10d_mesh};
use qualia_core_db::specialized_libs::computational_geometry::attribute_transfer::{
    CollapseTransfer, ProtectedDomain, TransferError, TransferRow,
};
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
use qualia_core_db::specialized_libs::computational_geometry::{
    compute_extended_collapse_cost, Point3, SemanticDecimateWeights, SemanticVertexAttributes, SparseEntry,
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

struct RunSpec {
    target_faces: usize,
    empty: bool,
    corpus: Option<LoadedCorpus>,
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

fn simplify(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    attributes: &[SemanticVertexAttributes],
    target_faces: usize,
) -> ProbeResult<(Vec<Point3>, Vec<SemanticVertexAttributes>, Vec<[u32; 3]>, AdmissionReport)> {
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
    Ok((out_vertices, out_attributes, out_triangles, report))
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
    execute_run(RunSpec { target_faces, empty, corpus: None })
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
    let (out_vertices, out_attributes, out_triangles, report) =
        simplify(&vertices, &triangles, &attributes, target_faces)?;
    let simplify_ms = simplify_start.elapsed().as_secs_f64() * 1000.0;
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
    if builtin {
        checked(forbidden_label_vertices == 0, "output label escaped its authored half-plane")?;
    }
    let mixed_domain_triangles = out_triangles
        .iter()
        .filter(|tri| {
            let labels = tri.map(|i| out_attributes[i as usize].subpart_tag);
            labels[0] != labels[1] || labels[1] != labels[2]
        })
        .count();
    checked(mixed_domain_triangles == 0, "output triangle crosses semantic domains")?;
    let mut protected_count = 0_usize;
    for (p, a) in vertices.iter().zip(&attributes).filter(|(_, a)| a.cluster_border_locked) {
        protected_count += 1;
        checked(
            out_vertices.iter().zip(&out_attributes).any(|(q, b)| p == q && a == b),
            "locked vertex or its attributes changed",
        )?;
    }
    let source_mesh = mesh(&vertices, &triangles);
    let simplified_mesh = mesh(&out_vertices, &out_triangles);
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
    let mut artifacts = vec![
        Artifact { name: "source.10d", bytes: source },
        Artifact { name: "simplified.10d", bytes: simplified },
        json_artifact("source-attributes.json", &attribute_records(&attributes))?,
        json_artifact("decoded-vertices.json", &decoded.positions)?,
        json_artifact("decoded-triangles.json", &decoded.triangles)?,
        json_artifact("attributes.json", &attribute_records(&out_attributes))?,
    ];
    let hashes: Vec<Value> =
        artifacts.iter().map(|a| json!({"file": a.name, "sha256": sha256(&a.bytes), "bytes": a.bytes.len()})).collect();
    let status = receipt_status(&report, target_faces);
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
        "policy": {"semantic_transition": "hard_reject_admitted_domains", "protected": "perimeter_and_center_seam",
            "transfer_gate": "p07_collapse_transfer", "cost_helper_probes": policy_probes},
        "result": {"target_faces": target_faces, "vertices": report.vertices, "faces": report.faces,
            "collapses": report.committed_collapses, "termination": format!("{:?}", report.termination),
            "protected_vertices_preserved": protected_count,
            "forbidden_label_vertices": forbidden_label_vertices, "mixed_domain_triangles": mixed_domain_triangles,
            "locked_precompile_displacement": 0.0},
        "metrics": {
            "largest_normalized_collapse_cost": {"value": report.largest_cost.total, "kind": "ranking_cost_not_distance"},
            "analytic_plane_residual": if builtin {
                json!({"value": max_plane_residual, "units": "normalized_world", "kind": "vertex_sample"})
            } else {
                json!({"status": "not_measured", "reason": "builtin tilted-plane residual applies only to the two-chart grid"})
            },
            "source_codec_position_error": {"value": max_source_quantization, "units": "normalized_world", "kind": "paired_vertex_max"},
            "simplified_codec_position_error": {"value": max_quantization, "units": "normalized_world", "kind": "paired_vertex_max"},
            "decoded_locked_position_error": {"value": max_decoded_locked_displacement, "units": "normalized_world", "kind": "paired_vertex_max"},
            "uv01_exact_f32_roundtrip": true,
            "symmetric_surface_deviation": {"status": "not_measured"},
            "operator_residual": {"status": "not_measured"},
            "physical_response": {"status": "not_measured"}},
        "timings_ms": {"admitted_decimate": simplify_ms, "seal_and_roundtrip": seal_roundtrip_ms,
            "geometry_probe_total": total_start.elapsed().as_secs_f64()*1000.0},
        "resources": {"fixture_vertices_cap": vertices.len(), "fixture_faces_cap": triangles.len(),
            "decoded_scalar_cap": MAX_SCALARS, "admitted_bytes": report.admitted_bytes, "work_used": report.work_used},
        "artifacts": hashes,
        "limitations": ["integration tracer only", "labels retained in attributes.json rather than a new .10d section",
            "non-ReachedTarget reports are a committed prefix, not an accepted asset",
            "no independent whole-surface distance certificate", "no GPU stage in this executable",
            "existing infinite-penalty path may reject compatible labels via infinity times zero; see policy probe"]
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
    let out = out.ok_or_else(|| invalid("usage: production_core_geometry --out <new-directory> [--target-faces N] [--empty] [--corpus-json PATH] [--output-byte-budget N]"))?;
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
    let (artifacts, receipt) = execute_run(RunSpec { target_faces: target, empty, corpus })?;
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
        })
        .unwrap();
        assert_eq!(receipt["fixture"]["name"], "analytic-plane-scale-1");
        assert_eq!(receipt["fixture"]["consumes_corpus_json"], true);
        assert_eq!(receipt["fixture"]["source"], "corpus_json");
        assert_eq!(receipt["metrics"]["analytic_plane_residual"]["status"], "not_measured");
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
}
