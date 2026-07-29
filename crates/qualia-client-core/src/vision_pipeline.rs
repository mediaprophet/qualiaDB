//! End-to-end native vision pipeline (MVP + GSW W/G/S).
//!
//! Synthetic or raw RGB → detect (reference or production weights) → track →
//! epistemic quins → overlay BMP. Plus generate + image-to-3D. No Python.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use qualia_core_db::container_10d::provenance_section::ProvenanceSidecar;
use qualia_core_db::render::assets::{mesh_to_nquins_with_digests, Mesh};
use qualia_core_db::render::compile_10d::{
    compile_mesh_to_10d_vision, compile_mesh_to_10d_vision_with_provenance,
};
use qualia_core_db::sparql_library::vision_shacl::{
    validate_vision_observation_graph, VisionShaclReport,
};
use qualia_core_db::specialized_libs::computational_geometry::{
    decimate_qem, DecimateOptions, Point3,
};
use qualia_core_db::tensor::Tensor10D;
use qualia_core_db::NQuin;
use qualia_vision::detector::{
    GridMultiObjectDetector, CLASS_MOSTLY_BLUE, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_RED,
};
use qualia_vision::generator::{compile_generation_receipt_quins, NativeImageGenerator};
use qualia_vision::media_store::{MediaStore, RetentionClass};
use qualia_vision::metrics::evaluate_synthetic;
use qualia_vision::overlay::{box_css_percent, compose_rgb_overlay_rgba8, encode_bmp_rgba8};
use qualia_vision::query_instances_in_region;
use qualia_vision::semantic::{compile_observation_quins_full, media_digest, VisionQuin};
use qualia_vision::spatial::{
    cleanup_mesh_ir, detections_to_node_hints, image_to_heightfield_mesh, mesh_ir_to_export,
    mesh_ir_to_obj, pack_geometry_export_for_10d, MeshCleanupOptions, MeshIR, NodeHint,
};
use qualia_vision::synthetic::{generate_scene_rgb8, sample_id, DatasetSplit, SyntheticSampleId};
use qualia_vision::tracker::BoundedTracker;
use qualia_vision::types::{Detection, ImageView, PixelFormat, VisualModel, MAX_DETECTIONS};
use qualia_vision::weights::{ProductionVision, VisionBackendKind, VisionWeightBundle};
use serde::Serialize;

use crate::vision_ingest::{
    append_human_attestation, append_native_observation_quins, human_correct_quin,
    human_reject_quin, NativeDetection,
};

#[derive(Debug, Clone, Serialize)]
pub struct OverlayBoxDto {
    pub class_hash: String,
    pub instance_hash: String,
    pub score: f32,
    pub track_id: u32,
    pub frame_index: u32,
    /// CSS percent: left, top, width, height
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub rejected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VisionDemoResult {
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub split: String,
    pub model_hash: String,
    pub media_hash: String,
    pub detections: Vec<OverlayBoxDto>,
    pub n_gt: usize,
    pub n_pred: usize,
    pub quins_written: usize,
    pub shacl_ok: bool,
    pub shacl_observations: u32,
    pub shacl_human: u32,
    /// data:image/bmp;base64,... with boxes drawn
    pub overlay_data_url: String,
    pub note: String,
    /// `reference` | `production_weights`
    pub backend: String,
    pub is_reference_backend: bool,
    pub synthetic_match_acc: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateResult {
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub steps: u32,
    pub model_hash: String,
    pub prompt_hash: String,
    pub output_hash: String,
    pub is_reference_generator: bool,
    pub image_data_url: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageTo3dResult {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub mesh_hash: String,
    pub model_hash: String,
    pub validation_ok: bool,
    pub validation_status: String,
    pub is_reference_recon: bool,
    pub note: String,
}

/// Full G→S continuum: generate → media store → recon → validate → OBJ + .10d + quins.
#[derive(Debug, Clone, Serialize)]
pub struct GsContinuumResult {
    pub generate: GenerateResult,
    pub mesh: ImageTo3dResult,
    pub media_digest_hex: String,
    pub media_stored: bool,
    pub obj_bytes: usize,
    pub container_10d_bytes: usize,
    pub geometry_quins: usize,
    pub generation_quins: usize,
    pub obj_path: Option<String>,
    pub container_10d_path: Option<String>,
    pub note: String,
}

/// Convert vision MeshIR → core `Mesh` via public export + 10d pack (C1 handoff).
fn mesh_ir_to_core_mesh(ir: &MeshIR) -> Result<Mesh, String> {
    let export = mesh_ir_to_export(ir).map_err(|e| format!("mesh_ir_to_export: {e:?}"))?;
    let g = pack_geometry_export_for_10d(&export);
    Ok(Mesh {
        positions: g.positions,
        triangles: g.triangles,
        min: g.min,
        max: g.max,
    })
}

/// Map vision [`NodeHint`] → [`Tensor10D`] for spectral paint (D1/D2).
///
/// Epistemic parallel context (q=1): vision detections are proposals, not ground truth.
pub fn node_hint_to_tensor10d(h: &NodeHint) -> Tensor10D {
    Tensor10D::parallel_context(
        1.0, // q: proposal
        0.0, // v: Euclidean
        0.0, // w: medical-class slot (vision still uses 0 until domain wiring)
        h.x,
        h.y,
        h.z,
        h.t,
        1.0, // alpha full amplitude; score is in σ path already
        0.0,
        h.sigma.clamp(0.0, 1.0),
    )
}

/// Host C2: optional QEM decimate when triangle count exceeds `max_faces`.
fn maybe_decimate_mesh(mesh: &mut Mesh, max_faces: usize) -> Result<Option<String>, String> {
    if mesh.triangle_count() <= max_faces || max_faces == 0 {
        return Ok(None);
    }
    let verts: Vec<Point3> = mesh
        .positions
        .iter()
        .map(|p| Point3::new(p[0] as f64, p[1] as f64, p[2] as f64))
        .collect();
    let tris = mesh.triangles.clone();
    let mut out_v = vec![Point3::new(0.0, 0.0, 0.0); verts.len()];
    let mut out_t = vec![[0u32; 3]; tris.len()];
    let report = decimate_qem(
        &verts,
        &tris,
        DecimateOptions::to_faces(max_faces),
        &mut out_v,
        &mut out_t,
    )
    .map_err(|e| format!("decimate_qem: {e:?}"))?;
    mesh.positions = out_v[..report.vertices]
        .iter()
        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
        .collect();
    mesh.triangles = out_t[..report.faces].to_vec();
    // Recompute AABB.
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    mesh.min = min;
    mesh.max = max;
    Ok(Some(format!(
        "decimated faces {}→{} ({} collapses)",
        tris.len(),
        report.faces,
        report.collapses
    )))
}

fn det_to_dto(d: &Detection, rejected: bool) -> OverlayBoxDto {
    let (left, top, width, height) = box_css_percent(d);
    OverlayBoxDto {
        class_hash: format!("0x{:016x}", d.class_hash),
        instance_hash: format!("0x{:016x}", d.instance_hash),
        score: d.score_f32(),
        track_id: d.track_id,
        frame_index: d.frame_index,
        left,
        top,
        width,
        height,
        rejected,
    }
}

fn vision_quin_to_nquin(v: &VisionQuin) -> NQuin {
    NQuin {
        subject: v.subject,
        predicate: v.predicate,
        object: v.object,
        context: v.context,
        metadata: v.metadata,
        parity: v.parity,
    }
}

/// Run synthetic sample through detect → track → compile → SHACL → overlay BMP.
pub fn run_synthetic_demo(
    split: DatasetSplit,
    index: u32,
    width: u32,
    height: u32,
) -> Result<VisionDemoResult, String> {
    run_synthetic_demo_with_backend(split, index, width, height, "reference")
}

pub fn run_synthetic_demo_with_backend(
    split: DatasetSplit,
    index: u32,
    width: u32,
    height: u32,
    backend: &str,
) -> Result<VisionDemoResult, String> {
    let sample = sample_id(split, index, width, height);
    run_sample_demo_with_backend(&sample, backend)
}

pub fn run_sample_demo(sample: &SyntheticSampleId) -> Result<VisionDemoResult, String> {
    run_sample_demo_with_backend(sample, "reference")
}

pub fn run_sample_demo_with_backend(
    sample: &SyntheticSampleId,
    backend: &str,
) -> Result<VisionDemoResult, String> {
    let w = sample.width;
    let h = sample.height;
    let px = (w as usize) * (h as usize);
    let mut rgb = vec![0u8; px * 3];
    let mut gt = [Detection::empty(); MAX_DETECTIONS];
    let n_gt = generate_scene_rgb8(sample, &mut rgb, &mut gt).map_err(|e| format!("{e:?}"))?;

    let img = ImageView {
        bytes: &rgb,
        width: w,
        height: h,
        row_stride: w * 3,
        format: PixelFormat::Rgb8,
    };
    let mut preds = [Detection::empty(); MAX_DETECTIONS];
    let mut emb = [0.0f32; 32];
    let mut ws = [0u8; MAX_DETECTIONS];
    let use_prod = backend.eq_ignore_ascii_case("production")
        || backend.eq_ignore_ascii_case("production_weights")
        || backend.eq_ignore_ascii_case("weights");

    let (n_pred, model_hash, backend_kind, is_ref, synth_acc) = if use_prod {
        let classes = [CLASS_MOSTLY_RED, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_BLUE];
        let bundle = VisionWeightBundle::from_seed(0x01D1_FACE_u64, 16, &classes);
        let mh = bundle.model_hash();
        let mut prod = ProductionVision::new(bundle);
        let counts = prod
            .infer(img, &mut preds, &mut emb, &mut ws)
            .map_err(|e| format!("{e:?}"))?;
        let mut m2 =
            ProductionVision::new(VisionWeightBundle::from_seed(0x01D1_FACE_u64, 16, &classes));
        let metrics =
            evaluate_synthetic(&mut m2, VisionBackendKind::ProductionWeights, mh, 4, 32, 24);
        (
            counts.detections,
            mh,
            "production_weights",
            false,
            Some(metrics.mean_match_acc),
        )
    } else {
        let det = GridMultiObjectDetector::new(4, 3);
        let n_pred = det
            .detect(img, 0, &mut preds, &mut ws)
            .map_err(|e| format!("{e:?}"))?;
        let mh = det.model_hash();
        let mut det2 = GridMultiObjectDetector::new(4, 3);
        let metrics = evaluate_synthetic(&mut det2, VisionBackendKind::Reference, mh, 4, 32, 24);
        (n_pred, mh, "reference", true, Some(metrics.mean_match_acc))
    };

    let mut tracker = BoundedTracker::new();
    tracker.update(0, &mut preds, n_pred);

    let digest = media_digest(&rgb);
    let mut vquins = [VisionQuin::with_parity(0, 0, 0, 0, 0); 256];
    let n_q = compile_observation_quins_full(digest, &preds[..n_pred], model_hash, &mut vquins);

    let mut nquins = Vec::with_capacity(n_q);
    for q in vquins.iter().take(n_q) {
        nquins.push(vision_quin_to_nquin(q));
    }
    let report = validate_vision_observation_graph(&nquins);

    let mut rgba = vec![0u8; px * 4];
    compose_rgb_overlay_rgba8(w, h, &rgb, &preds, n_pred, [0, 255, 180, 255], 2, &mut rgba)
        .map_err(|e| format!("{e:?}"))?;
    let mut bmp = vec![0u8; 54 + px * 4];
    let bmp_n = encode_bmp_rgba8(w, h, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    let b64 = B64.encode(&bmp[..bmp_n]);

    let split_s = match sample.split {
        DatasetSplit::Train => "train",
        DatasetSplit::Test => "test",
    };

    let note = if is_ref {
        "Backend=reference (grid). Epistemic only — not ground truth. H1 real eval not run."
    } else {
        "Backend=production_weights (QVWT seed fixture). Synthetic metrics only until H1 corpus."
    };

    Ok(VisionDemoResult {
        width: w,
        height: h,
        seed: sample.seed,
        split: split_s.to_string(),
        model_hash: format!("0x{:016x}", model_hash),
        media_hash: format!("0x{:016x}", digest.hash),
        detections: preds[..n_pred]
            .iter()
            .map(|d| det_to_dto(d, false))
            .collect(),
        n_gt,
        n_pred,
        quins_written: n_q,
        shacl_ok: report.ok,
        shacl_observations: report.observation_count,
        shacl_human: report.human_attestation_count,
        overlay_data_url: format!("data:image/bmp;base64,{b64}"),
        note: note.into(),
        backend: backend_kind.into(),
        is_reference_backend: is_ref,
        synthetic_match_acc: synth_acc,
    })
}

/// Swarm G — native seeded image generation.
pub fn generate_image(
    prompt: &str,
    seed: u64,
    steps: u32,
    width: u32,
    height: u32,
) -> Result<GenerateResult, String> {
    let w = width.clamp(8, 256);
    let h = height.clamp(8, 256);
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    let g = NativeImageGenerator::new();
    let rec = g
        .generate_rgb8(prompt, seed, steps, w, h, &mut rgb)
        .map_err(|e| format!("{e:?}"))?;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        rgba[i * 4] = rgb[i * 3];
        rgba[i * 4 + 1] = rgb[i * 3 + 1];
        rgba[i * 4 + 2] = rgb[i * 3 + 2];
        rgba[i * 4 + 3] = 255;
    }
    let mut bmp = vec![0u8; 54 + rgba.len()];
    let n = encode_bmp_rgba8(w, h, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    Ok(GenerateResult {
        width: w,
        height: h,
        seed,
        steps: rec.steps,
        model_hash: format!("0x{:016x}", rec.model_hash),
        prompt_hash: format!("0x{:016x}", rec.prompt_hash),
        output_hash: format!("0x{:016x}", rec.output_digest.hash),
        is_reference_generator: rec.is_reference_generator,
        image_data_url: format!("data:image/bmp;base64,{}", B64.encode(&bmp[..n])),
        note: "Native reference generator (seeded). Not a foundation DiT; swap weights under G0 licence.".into(),
    })
}

/// Swarm S-V10 — heightfield recon + validation.
pub fn image_to_3d_from_rgb(
    width: u32,
    height: u32,
    rgb: &[u8],
    grid: u32,
) -> Result<ImageTo3dResult, String> {
    let img = ImageView {
        bytes: rgb,
        width,
        height,
        row_stride: width * 3,
        format: PixelFormat::Rgb8,
    };
    let (mesh, rec, rep) = image_to_heightfield_mesh(img, grid).map_err(|e| format!("{e:?}"))?;
    let status = format!("{:?}", rep.status);
    Ok(ImageTo3dResult {
        vertex_count: mesh.vertex_count() as u32,
        triangle_count: mesh.triangle_count() as u32,
        mesh_hash: format!("0x{:016x}", mesh.content_hash),
        model_hash: format!("0x{:016x}", rec.model_hash),
        validation_ok: rep.ok(),
        validation_status: status,
        is_reference_recon: rec.is_reference_recon,
        note: "Heightfield recon is epistemic proposal; validated before any Q42 geometry commit."
            .into(),
    })
}

/// Generate then reconstruct (G→S10 smoke path).
pub fn generate_and_reconstruct(
    prompt: &str,
    seed: u64,
) -> Result<(GenerateResult, ImageTo3dResult), String> {
    let gen = generate_image(prompt, seed, 4, 32, 32)?;
    let mut rgb = vec![0u8; 32 * 32 * 3];
    let g = NativeImageGenerator::new();
    g.generate_rgb8(prompt, seed, 4, 32, 32, &mut rgb)
        .map_err(|e| format!("{e:?}"))?;
    let mesh = image_to_3d_from_rgb(32, 32, &rgb, 8)?;
    Ok((gen, mesh))
}

/// Full continuum used before handing off to auditory work:
/// generate → content-addressed store → heightfield recon → validate →
/// OBJ + sealed `.10d` + generation receipt quins + geometry quins.
pub fn run_gs_continuum(
    storage_root: &std::path::Path,
    prompt: &str,
    seed: u64,
    steps: u32,
    width: u32,
    height: u32,
    recon_grid: u32,
    media_time_ms: u64,
) -> Result<GsContinuumResult, String> {
    let w = width.clamp(8, 128);
    let h = height.clamp(8, 128);
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    let gen = NativeImageGenerator::new();
    let rec = gen
        .generate_rgb8_cancellable(prompt, seed, steps, w, h, &mut rgb, None, media_time_ms)
        .map_err(|e| format!("{e:?}"))?;

    // Media store (no partial commit if later steps fail after store — store is deduped).
    let media_dir = storage_root.join("vision_media");
    let store = MediaStore::open(&media_dir).map_err(|e| e)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let record = store
        .import_bytes(
            &rgb,
            "application/octet-stream",
            w,
            h,
            RetentionClass::Restricted,
            now,
        )
        .map_err(|e| e)?;

    let mut gen_quins = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
    let n_gen_q = compile_generation_receipt_quins(&rec, &mut gen_quins);

    // BMP preview for API
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for i in 0..(w * h) as usize {
        rgba[i * 4] = rgb[i * 3];
        rgba[i * 4 + 1] = rgb[i * 3 + 1];
        rgba[i * 4 + 2] = rgb[i * 3 + 2];
        rgba[i * 4 + 3] = 255;
    }
    let mut bmp = vec![0u8; 54 + rgba.len()];
    let bmp_n = encode_bmp_rgba8(w, h, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    let generate = GenerateResult {
        width: w,
        height: h,
        seed,
        steps: rec.steps,
        model_hash: format!("0x{:016x}", rec.model_hash),
        prompt_hash: format!("0x{:016x}", rec.prompt_hash),
        output_hash: format!("0x{:016x}", rec.output_digest.hash),
        is_reference_generator: rec.is_reference_generator,
        image_data_url: format!("data:image/bmp;base64,{}", B64.encode(&bmp[..bmp_n])),
        note: format!(
            "Stored media digest {}; media_time_ms={media_time_ms} for cross-modal timeline.",
            record.digest_hex
        ),
    };

    let img = ImageView {
        bytes: &rgb,
        width: w,
        height: h,
        row_stride: w * 3,
        format: PixelFormat::Rgb8,
    };
    let (mut mesh_ir, recon_rec, rep) =
        image_to_heightfield_mesh(img, recon_grid).map_err(|e| format!("{e:?}"))?;
    if !rep.ok() {
        return Err(format!("mesh validation failed: {:?}", rep.status));
    }

    // C2 vision-side: drop degenerate faces before export.
    let quality = cleanup_mesh_ir(
        &mut mesh_ir,
        MeshCleanupOptions {
            weld_epsilon: 1e-6,
            min_area: 0.0,
        },
    )
    .map_err(|e| format!("mesh quality cleanup: {e:?}"))?;

    let mut obj_buf = vec![0u8; mesh_ir.positions.len() * 64 + mesh_ir.indices.len() * 24 + 256];
    let obj_n = mesh_ir_to_obj(&mesh_ir, &mut obj_buf).map_err(|e| format!("{e:?}"))?;
    obj_buf.truncate(obj_n);

    let mut core_mesh = mesh_ir_to_core_mesh(&mesh_ir)?;
    // C2 host: QEM when heightfield is dense (cap 2048 faces for edge browsers).
    let decimate_note = maybe_decimate_mesh(&mut core_mesh, 2048)?;

    // D1: seal with Tensor10DNodes from a single plane marker (heightfield centre).
    // Full detection→node path uses seal_vision_mesh_with_detections when callers have dets.
    let centre = Tensor10D::parallel_context(
        1.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.35, // mid-band σ for recon marker
    );
    // D4: provenance sidecar — media digest bytes + recon model hash in-envelope.
    let mut version_hash = [0u8; 32];
    let mh = recon_rec.model_hash.to_le_bytes();
    version_hash[..8].copy_from_slice(&mh);
    let dig = record.digest_u64.to_le_bytes();
    version_hash[8..16].copy_from_slice(&dig);
    // Source bytes: short media digest prefix (self-authenticating CRC of this payload).
    let source_tag = format!(
        "qualia-vision-recon;media={};model=0x{:016x}",
        record.digest_hex, recon_rec.model_hash
    );
    let provenance = ProvenanceSidecar::new(
        source_tag.into_bytes(),
        "application/x-qualia-vision-recon",
        "PermissiveReady-local", // synthetic continuum — not a third-party weight licence
    )
    .with_metadata(
        format!(
            r#"{{"media_digest":"{}","model_hash":"0x{:016x}"}}"#,
            record.digest_hex, recon_rec.model_hash
        )
        .into_bytes(),
        0,
        version_hash,
    );
    // C3+D4: topology/spatial when CG linked + provenance always.
    let container = compile_mesh_to_10d_vision_with_provenance(&core_mesh, &[centre], &provenance)
        .map_err(|e| e.to_string())?;
    // CRC of container for compiled digest (first 4 bytes of crc is enough for quin object)
    let compiled_digest = {
        let mut h: u32 = 0;
        for chunk in container.chunks(4) {
            let mut b = [0u8; 4];
            b[..chunk.len()].copy_from_slice(chunk);
            h ^= u32::from_le_bytes(b);
        }
        h
    };
    let source_digest = (record.digest_u64 & 0xFFFF_FFFF) as u32;
    let asset_uri = format!("urn:qualia:vision:recon:{}", record.digest_hex);
    let (geo_quins, _lex) = mesh_to_nquins_with_digests(
        &core_mesh,
        &asset_uri,
        "obj",
        source_digest,
        compiled_digest,
    );

    let out_dir = storage_root
        .join("vision_geometry")
        .join(&record.digest_hex);
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let obj_path = out_dir.join("recon.obj");
    let c10_path = out_dir.join("recon.10d");
    std::fs::write(&obj_path, &obj_buf).map_err(|e| e.to_string())?;
    std::fs::write(&c10_path, &container).map_err(|e| e.to_string())?;

    // Append generation + geometry quins to vision_native.wal when possible
    let mut nquin_buf = Vec::with_capacity(n_gen_q + geo_quins.len());
    for q in gen_quins.iter().take(n_gen_q) {
        nquin_buf.push(NQuin {
            subject: q.subject,
            predicate: q.predicate,
            object: q.object,
            context: q.context,
            metadata: q.metadata,
            parity: q.parity,
        });
    }
    nquin_buf.extend(geo_quins.iter().cloned());
    let wal_path = storage_root.join("models").join("vision_native.wal");
    if let Some(parent) = wal_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut wal) = qualia_core_db::wal::WriteAheadLog::open(&wal_path) {
        for q in &nquin_buf {
            let _ = wal.append_mutation(q);
        }
    }

    let mesh = ImageTo3dResult {
        vertex_count: mesh_ir.vertex_count() as u32,
        triangle_count: mesh_ir.triangle_count() as u32,
        mesh_hash: format!("0x{:016x}", mesh_ir.content_hash),
        model_hash: format!("0x{:016x}", recon_rec.model_hash),
        validation_ok: true,
        validation_status: format!("{:?}", rep.status),
        is_reference_recon: recon_rec.is_reference_recon,
        note: format!(
            "Validated MeshIR → cleanup(deg={},weld={}) → OBJ + sealed .10d with Tensor10DNodes; digests on quins.",
            quality.degenerates_removed, quality.vertices_welded
        ),
    };

    let mut note = String::from(
        "G→S continuum closed: store + validate + cleanup + compile(mesh+nodes). Ready for 10d browse.",
    );
    if let Some(d) = decimate_note {
        note.push(' ');
        note.push_str(&d);
    }

    Ok(GsContinuumResult {
        generate,
        mesh,
        media_digest_hex: record.digest_hex,
        media_stored: true,
        obj_bytes: obj_n,
        container_10d_bytes: container.len(),
        geometry_quins: nquin_buf.len().saturating_sub(n_gen_q),
        generation_quins: n_gen_q,
        obj_path: Some(obj_path.display().to_string()),
        container_10d_path: Some(c10_path.display().to_string()),
        note,
    })
}

/// Seal a MeshIR with detection-derived Tensor10D nodes (D1 product path).
pub fn seal_vision_mesh_with_detections(
    mesh_ir: &MeshIR,
    dets: &[Detection],
) -> Result<Vec<u8>, String> {
    let mut cleaned = mesh_ir.clone();
    let _ = cleanup_mesh_ir(
        &mut cleaned,
        MeshCleanupOptions {
            weld_epsilon: 1e-6,
            min_area: 0.0,
        },
    );
    let mut core = mesh_ir_to_core_mesh(&cleaned)?;
    let _ = maybe_decimate_mesh(&mut core, 4096)?;
    let mut hints = vec![
        NodeHint {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
            sigma: 0.0,
        };
        dets.len().min(256)
    ];
    let n = detections_to_node_hints(dets, &mut hints);
    let nodes: Vec<Tensor10D> = hints[..n].iter().map(node_hint_to_tensor10d).collect();
    compile_mesh_to_10d_vision(&core, &nodes).map_err(|e| e.to_string())
}

/// Persist native observations to WAL and return SHACL report.
pub fn ingest_demo_to_wal(
    storage_root: &std::path::Path,
    demo: &VisionDemoResult,
) -> Result<VisionShaclReport, String> {
    let media_hash = u64::from_str_radix(demo.media_hash.trim_start_matches("0x"), 16)
        .map_err(|e| e.to_string())?;
    let model_hash = u64::from_str_radix(demo.model_hash.trim_start_matches("0x"), 16)
        .map_err(|e| e.to_string())?;
    let mut natives = Vec::new();
    for d in &demo.detections {
        let instance_hash = u64::from_str_radix(d.instance_hash.trim_start_matches("0x"), 16)
            .map_err(|e| e.to_string())?;
        let class_hash = u64::from_str_radix(d.class_hash.trim_start_matches("0x"), 16)
            .map_err(|e| e.to_string())?;
        let (x0, y0, x1, y1) = css_to_u16(d.left, d.top, d.width, d.height);
        natives.push(NativeDetection {
            class_hash,
            instance_hash,
            score_u16: (d.score.clamp(0.0, 1.0) * 65535.0) as u16,
            x_min_u16: x0,
            y_min_u16: y0,
            x_max_u16: x1,
            y_max_u16: y1,
            frame_index: d.frame_index,
            track_id: d.track_id,
            flags: 0,
        });
    }
    let n = append_native_observation_quins(
        storage_root,
        media_hash,
        (demo.width as u64) * (demo.height as u64) * 3,
        model_hash,
        &natives,
    )
    .map_err(|e| e.to_string())?;
    // Rebuild minimal graph for SHACL
    let mut buf = [NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    }; 256];
    let written = crate::vision_ingest::compile_native_observation_quins(
        media_hash,
        (demo.width as u64) * (demo.height as u64) * 3,
        model_hash,
        &natives,
        &mut buf,
    )
    .map_err(|e| e.to_string())?;
    let _ = n;
    Ok(validate_vision_observation_graph(&buf[..written]))
}

fn css_to_u16(left: f32, top: f32, width: f32, height: f32) -> (u16, u16, u16, u16) {
    let x0 = ((left / 100.0) * 65535.0).clamp(0.0, 65535.0) as u16;
    let y0 = ((top / 100.0) * 65535.0).clamp(0.0, 65535.0) as u16;
    let x1 = (((left + width) / 100.0) * 65535.0).clamp(0.0, 65535.0) as u16;
    let y1 = (((top + height) / 100.0) * 65535.0).clamp(0.0, 65535.0) as u16;
    (x0, y0, x1, y1)
}

/// Human rejects instance; machine claims remain. Returns updated attestation note.
pub fn reject_instance(
    storage_root: &std::path::Path,
    human_did_hash: u64,
    instance_hash: u64,
    reason_hash: u64,
) -> Result<(), String> {
    let q = human_reject_quin(human_did_hash, instance_hash, reason_hash);
    append_human_attestation(storage_root, &q).map_err(|e| e.to_string())
}

pub fn correct_instance(
    storage_root: &std::path::Path,
    human_did_hash: u64,
    instance_hash: u64,
    new_class_hash: u64,
) -> Result<(), String> {
    let q = human_correct_quin(human_did_hash, instance_hash, new_class_hash);
    append_human_attestation(storage_root, &q).map_err(|e| e.to_string())
}

/// Public helpers for UI without storage.
pub fn demo_train(index: u32) -> Result<VisionDemoResult, String> {
    run_synthetic_demo(DatasetSplit::Train, index, 96, 64)
}

pub fn demo_test(index: u32) -> Result<VisionDemoResult, String> {
    run_synthetic_demo(DatasetSplit::Test, index, 96, 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_demo_produces_overlay_and_shacl() {
        let r = demo_test(0).expect("demo");
        assert!(r.n_pred >= 1 || r.n_gt >= 1);
        assert!(r.overlay_data_url.starts_with("data:image/bmp;base64,"));
        assert!(r.shacl_ok, "shacl should pass on full compile");
        assert!(r.quins_written >= 1);
        assert_eq!(r.backend, "reference");
    }

    #[test]
    fn production_backend_labelled() {
        let r = run_synthetic_demo_with_backend(DatasetSplit::Test, 0, 48, 32, "production")
            .expect("prod");
        assert_eq!(r.backend, "production_weights");
        assert!(!r.is_reference_backend);
        assert!(r.synthetic_match_acc.is_some());
    }

    #[test]
    fn generate_and_recon_smoke() {
        let (g, m) = generate_and_reconstruct("test field", 7).expect("g+s");
        assert!(g.is_reference_generator);
        assert!(m.validation_ok);
        assert!(m.triangle_count > 0);
    }

    #[test]
    fn gs_continuum_writes_obj_and_10d() {
        let dir = std::env::temp_dir().join(format!(
            "qv-gs-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let r = run_gs_continuum(&dir, "hills", 11, 3, 24, 24, 6, 1000).expect("continuum");
        assert!(r.media_stored);
        assert!(r.obj_bytes > 0);
        assert!(r.container_10d_bytes > 64);
        assert!(r.geometry_quins >= 1);
        assert!(r.generation_quins == 3);
        let obj = r.obj_path.as_ref().unwrap();
        assert!(std::path::Path::new(obj).is_file());
        let c10 = r.container_10d_path.as_ref().unwrap();
        assert!(std::path::Path::new(c10).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn section15_smoke_passes() {
        let s = section15_smoke().expect("§15");
        assert!(s.contains("OK"));
    }

    #[test]
    fn rgb_import_detects() {
        let dir = std::env::temp_dir().join(format!(
            "qv-imp-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let mut rgb = vec![0u8; 16 * 16 * 3];
        for p in rgb.chunks_mut(3) {
            p[0] = 220;
            p[1] = 20;
            p[2] = 20;
        }
        let r = detect_from_rgb8(&dir, &rgb, 16, 16, "reference", true).unwrap();
        assert!(r.n_pred >= 1);
        assert!(r.shacl_ok);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Detect from raw RGB8 buffer (file import path).
pub fn detect_from_rgb8(
    storage_root: &std::path::Path,
    rgb: &[u8],
    width: u32,
    height: u32,
    backend: &str,
    persist: bool,
) -> Result<VisionDemoResult, String> {
    if rgb.len() < (width as usize) * (height as usize) * 3 || width == 0 || height == 0 {
        return Err("bad rgb geometry".into());
    }
    let img = ImageView {
        bytes: rgb,
        width,
        height,
        row_stride: width * 3,
        format: PixelFormat::Rgb8,
    };
    let mut preds = [Detection::empty(); MAX_DETECTIONS];
    let mut emb = [0.0f32; 32];
    let mut ws = [0u8; MAX_DETECTIONS];
    let use_prod = backend.eq_ignore_ascii_case("production")
        || backend.eq_ignore_ascii_case("production_weights");
    let (n_pred, model_hash, backend_kind, is_ref) = if use_prod {
        let classes = [CLASS_MOSTLY_RED, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_BLUE];
        let bundle = VisionWeightBundle::from_seed(0x01D1_FACE_u64, 16, &classes);
        let mh = bundle.model_hash();
        let mut prod = ProductionVision::new(bundle);
        let counts = prod
            .infer(img, &mut preds, &mut emb, &mut ws)
            .map_err(|e| format!("{e:?}"))?;
        (counts.detections, mh, "production_weights", false)
    } else {
        let det = GridMultiObjectDetector::new(4, 3);
        let n = det
            .detect(img, 0, &mut preds, &mut ws)
            .map_err(|e| format!("{e:?}"))?;
        (n, det.model_hash(), "reference", true)
    };
    let mut tracker = BoundedTracker::new();
    tracker.update(0, &mut preds, n_pred);
    let digest = media_digest(rgb);
    if persist {
        let store = MediaStore::open(storage_root.join("vision_media")).map_err(|e| e)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = store.import_bytes(
            rgb,
            "image/x-rgb8",
            width,
            height,
            RetentionClass::Restricted,
            now,
        );
    }
    let mut vquins = [VisionQuin::with_parity(0, 0, 0, 0, 0); 256];
    let n_q = compile_observation_quins_full(digest, &preds[..n_pred], model_hash, &mut vquins);
    let mut nq = Vec::new();
    for q in vquins.iter().take(n_q) {
        nq.push(vision_quin_to_nquin(q));
    }
    let report = validate_vision_observation_graph(&nq);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    compose_rgb_overlay_rgba8(
        width,
        height,
        rgb,
        &preds,
        n_pred,
        [0, 255, 180, 255],
        2,
        &mut rgba,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut bmp = vec![0u8; 54 + rgba.len()];
    let bmp_n = encode_bmp_rgba8(width, height, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    Ok(VisionDemoResult {
        width,
        height,
        seed: 0,
        split: "import".into(),
        model_hash: format!("0x{:016x}", model_hash),
        media_hash: format!("0x{:016x}", digest.hash),
        detections: preds[..n_pred]
            .iter()
            .map(|d| det_to_dto(d, false))
            .collect(),
        n_gt: 0,
        n_pred,
        quins_written: n_q,
        shacl_ok: report.ok,
        shacl_observations: report.observation_count,
        shacl_human: report.human_attestation_count,
        overlay_data_url: format!("data:image/bmp;base64,{}", B64.encode(&bmp[..bmp_n])),
        note: format!("RGB import path. Backend={backend_kind}. Epistemic only."),
        backend: backend_kind.into(),
        is_reference_backend: is_ref,
        synthetic_match_acc: None,
    })
}

/// Decode PNG/JPEG and detect.
pub fn detect_from_image_file(
    storage_root: &std::path::Path,
    path: &std::path::Path,
    backend: &str,
) -> Result<VisionDemoResult, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    detect_from_rgb8(storage_root, rgb.as_raw(), w, h, backend, true)
}

/// Query observation quins by normalized region (u16).
pub fn query_vision_region(quins: &[VisionQuin], x0: u16, y0: u16, x1: u16, y1: u16) -> Vec<u64> {
    let mut out = [0u64; 64];
    let n = query_instances_in_region(quins, x0, y0, x1, y1, &mut out);
    out[..n].to_vec()
}

/// §15 automated smoke: synthetic detect + SHACL + overlay.
pub fn section15_smoke() -> Result<String, String> {
    let r = demo_test(0)?;
    if !r.shacl_ok {
        return Err("SHACL failed".into());
    }
    if r.n_pred == 0 && r.n_gt == 0 {
        return Err("no detections".into());
    }
    if !r.overlay_data_url.starts_with("data:image/bmp") {
        return Err("no overlay".into());
    }
    Ok(format!(
        "section15_smoke OK backend={} dets={} quins={}",
        r.backend, r.n_pred, r.quins_written
    ))
}

/// Phase 11–12: twin eligibility + closed-form elasticity preview (A1, not FEA).
pub fn twin_elasticity_demo() -> Result<serde_json::Value, String> {
    use qualia_vision::spatial::{
        closed_form_bar_stretch, promote_elasticity_preview, refuse_fea_unless_eligible,
        run_elasticity_preview_if_eligible, BarStretchInput, MeshIR,
    };
    let mut m = MeshIR::empty();
    m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    m.indices = vec![0, 1, 2];
    m.recompute_bounds_and_hash();
    let viz = qualia_vision::spatial::assess_twin_eligibility(&m);
    let promoted = promote_elasticity_preview(&m);
    let refuse_viz = refuse_fea_unless_eligible(viz).is_err();
    let r = run_elasticity_preview_if_eligible(
        promoted,
        BarStretchInput {
            force_n: 1000.0,
            length_m: 1.0,
            area_m2: 0.01,
            youngs_pa: 2.0e11,
        },
    )
    .map_err(|e| e.to_string())?;
    let _ = closed_form_bar_stretch(BarStretchInput {
        force_n: 1.0,
        length_m: 1.0,
        area_m2: 1.0,
        youngs_pa: 1.0e9,
    });
    Ok(serde_json::json!({
        "viz_only_refuses_fea": refuse_viz,
        "promoted_domain": format!("{:?}", promoted.domain),
        "displacement_m": r.displacement_m,
        "assurance": r.assurance_note,
        "note": "A1 closed-form bar stretch only — not mesh FEA / not A4."
    }))
}

/// Ensure seed QVWT on disk; load for production path.
pub fn ensure_vision_weights(storage_root: &std::path::Path) -> Result<String, String> {
    use qualia_vision::detector::{CLASS_MOSTLY_BLUE, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_RED};
    let path = storage_root.join("models").join("vision_seed.qvwt");
    let classes = [CLASS_MOSTLY_RED, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_BLUE];
    if path.is_file() {
        let b = VisionWeightBundle::load_path(&path, &classes)?;
        return Ok(format!(
            "loaded QVWT hash=0x{:016x} path={}",
            b.content_hash,
            path.display()
        ));
    }
    let b = VisionWeightBundle::from_seed(0x01D1_FACE_u64, 16, &classes);
    b.save_path(&path)?;
    Ok(format!(
        "wrote QVWT seed hash=0x{:016x} path={}",
        b.content_hash,
        path.display()
    ))
}

/// Detect with QVWT from disk if present.
pub fn detect_with_disk_weights(
    storage_root: &std::path::Path,
    rgb: &[u8],
    width: u32,
    height: u32,
) -> Result<VisionDemoResult, String> {
    use qualia_vision::detector::{CLASS_MOSTLY_BLUE, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_RED};
    let path = storage_root.join("models").join("vision_seed.qvwt");
    let classes = [CLASS_MOSTLY_RED, CLASS_MOSTLY_GREEN, CLASS_MOSTLY_BLUE];
    let bundle = if path.is_file() {
        VisionWeightBundle::load_path(&path, &classes)?
    } else {
        let b = VisionWeightBundle::from_seed(0x01D1_FACE_u64, 16, &classes);
        b.save_path(&path)?;
        b
    };
    let img = ImageView {
        bytes: rgb,
        width,
        height,
        row_stride: width * 3,
        format: PixelFormat::Rgb8,
    };
    let model_hash = bundle.content_hash;
    let mut prod = ProductionVision::new(bundle);
    let mut preds = [Detection::empty(); MAX_DETECTIONS];
    let mut emb = [0.0f32; 32];
    let mut ws = [0u8; MAX_DETECTIONS];
    let counts = prod
        .infer(img, &mut preds, &mut emb, &mut ws)
        .map_err(|e| format!("{e:?}"))?;
    let digest = media_digest(rgb);
    let mut tracker = BoundedTracker::new();
    tracker.update(0, &mut preds, counts.detections);
    let mut vquins = [VisionQuin::with_parity(0, 0, 0, 0, 0); 256];
    let n_q = compile_observation_quins_full(
        digest,
        &preds[..counts.detections],
        model_hash,
        &mut vquins,
    );
    let mut nq = Vec::new();
    for q in vquins.iter().take(n_q) {
        nq.push(vision_quin_to_nquin(q));
    }
    let report = validate_vision_observation_graph(&nq);
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    compose_rgb_overlay_rgba8(
        width,
        height,
        rgb,
        &preds,
        counts.detections,
        [0, 255, 180, 255],
        2,
        &mut rgba,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut bmp = vec![0u8; 54 + rgba.len()];
    let bmp_n = encode_bmp_rgba8(width, height, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    Ok(VisionDemoResult {
        width,
        height,
        seed: 0,
        split: "disk-qvwt".into(),
        model_hash: format!("0x{:016x}", model_hash),
        media_hash: format!("0x{:016x}", digest.hash),
        detections: preds[..counts.detections]
            .iter()
            .map(|d| det_to_dto(d, false))
            .collect(),
        n_gt: 0,
        n_pred: counts.detections,
        quins_written: n_q,
        shacl_ok: report.ok,
        shacl_observations: report.observation_count,
        shacl_human: report.human_attestation_count,
        overlay_data_url: format!("data:image/bmp;base64,{}", B64.encode(&bmp[..bmp_n])),
        note: "QVWT loaded from models/vision_seed.qvwt (seed weights, not foundation).".into(),
        backend: "production_weights".into(),
        is_reference_backend: false,
        synthetic_match_acc: None,
    })
}

/// Biosense excellence demos (consent-bound recipes; no training).
#[derive(Debug, Clone, Serialize)]
pub struct BiosensePulseDemo {
    pub bpm: f32,
    pub confidence: f32,
    pub snr: f32,
    pub abstained: bool,
    pub used_evm: bool,
    pub reason: String,
}

/// Synthetic pulse + optional EVM → rPPG (security consent template for demo).
pub fn biosense_self_monitor_pulse_demo(use_evm: bool) -> Result<BiosensePulseDemo, String> {
    use qualia_vision::{self_monitor_pulse_evm, synthetic_pulse_sequence, BiosenseConsent};
    let seq = synthetic_pulse_sequence(32, 32, 90, 30.0, 72.0).map_err(|e| format!("{e}"))?;
    let consent = BiosenseConsent::grant_security_template(1);
    let r = self_monitor_pulse_evm(
        consent,
        seq.as_packed_rgb(),
        seq.n_frames,
        seq.width,
        seq.height,
        seq.fps,
        use_evm,
        0.15,
    );
    Ok(BiosensePulseDemo {
        bpm: r.bpm,
        confidence: r.confidence,
        snr: r.snr,
        abstained: r.abstained,
        used_evm: r.used_evm,
        reason: r
            .reason
            .map(|a| format!("{a:?}"))
            .unwrap_or_else(|| "ok".into()),
    })
}

/// Super-resolution ("Enhance") result: before/after PNG data URLs + honesty.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SrResultDto {
    pub before_data_url: String,
    pub after_data_url: String,
    pub backend_id: String,
    pub device: String,
    pub generative: bool,
    pub out_width: u32,
    pub out_height: u32,
    pub degraded: bool,
}

/// Decode arbitrary image bytes, classically super-resolve (Enhance), and return
/// before/after PNG data URLs plus device/honesty metadata.
///
/// Mirrors the Listen/audio_capabilities composition: decode → engine op →
/// PNG-encode both frames → DTO. Classical SR is non-generative (`Sharpen`).
pub fn super_resolve_image(
    bytes: &[u8],
    scale: u8,
    kernel: &str,
    prefer_gpu: bool,
) -> Result<SrResultDto, String> {
    use qualia_core_db::specialized_libs::computer_vision::cv::buffer::RgbView;
    use qualia_core_db::specialized_libs::computer_vision::cv::codecs::encode_png;
    use qualia_core_db::specialized_libs::computer_vision::gpu::dispatch::VisionComputeDevice;
    use qualia_core_db::specialized_libs::computer_vision::gpu::policy::{
        ThermalHint, VisionVramBudget,
    };
    use qualia_core_db::specialized_libs::computer_vision::sr::device_policy::super_resolve_with_policy;
    use qualia_core_db::specialized_libs::computer_vision::sr::super_resolve::{
        ClassicalKernel, EnhancementMode, SrBackend, SrRequest,
    };

    if !(2..=4).contains(&scale) {
        return Err("scale must be 2..=4".into());
    }

    // 1. Decode → RGB8.
    let img = image::load_from_memory(bytes).map_err(|e| format!("decode image: {e}"))?;
    let rgb_img = img.to_rgb8();
    let (w, h) = rgb_img.dimensions();
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    let rgb: Vec<u8> = rgb_img.into_raw();

    // 2. Map kernel string (same mapping as mcp vision.rs op).
    let ck = match kernel {
        "nearest" => ClassicalKernel::Nearest,
        "bilinear" => ClassicalKernel::Bilinear,
        "lanczos" | "lanczos3" => ClassicalKernel::Lanczos3,
        _ => ClassicalKernel::Bicubic,
    };
    let req = SrRequest {
        rgb: &rgb,
        width: w,
        height: h,
        scale,
        backend: SrBackend::Classical(ck),
        mode: EnhancementMode::Sharpen,
    };

    // 3. Allocate output and run under device policy.
    let ow = w.checked_mul(scale as u32).ok_or("output width overflow")?;
    let oh = h
        .checked_mul(scale as u32)
        .ok_or("output height overflow")?;
    let out_len = (ow as usize)
        .checked_mul(oh as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or("output size overflow")?;
    let mut out = vec![0u8; out_len];
    let (report, compute) = super_resolve_with_policy(
        &req,
        prefer_gpu,
        ThermalHint::Cool,
        VisionVramBudget::default(),
        &mut out,
    )
    .map_err(|e| format!("super_resolve: {e:?}"))?;

    // 4. PNG-encode both input and output.
    let before_view = RgbView::new(w, h, w.saturating_mul(3), &rgb).ok_or("bad input rgb view")?;
    let before_png = encode_png(before_view).map_err(|e| format!("encode before png: {e:?}"))?;
    let after_view = RgbView::new(
        report.out_width,
        report.out_height,
        ow.saturating_mul(3),
        &out,
    )
    .ok_or("bad output rgb view")?;
    let after_png = encode_png(after_view).map_err(|e| format!("encode after png: {e:?}"))?;

    let device = match compute.device {
        VisionComputeDevice::Cpu => "cpu",
        VisionComputeDevice::SharedGpu => "shared_gpu",
        VisionComputeDevice::Unavailable => "unavailable",
    };

    Ok(SrResultDto {
        before_data_url: format!("data:image/png;base64,{}", B64.encode(&before_png)),
        after_data_url: format!("data:image/png;base64,{}", B64.encode(&after_png)),
        backend_id: report.backend_id.to_string(),
        device: device.to_string(),
        generative: report.generative,
        out_width: report.out_width,
        out_height: report.out_height,
        degraded: compute.degraded,
    })
}

/// Local CBIR proxy hashes for an RGB buffer (not CLIP).
pub fn vision_local_embed_demo(
    rgb: &[u8],
    width: u32,
    height: u32,
) -> Result<serde_json::Value, String> {
    use qualia_vision::{
        ahash_u64, color_hist_embed_rgb, dhash_u64, GrayView, RgbView, COLOR_HIST_EMBED_DIM,
    };
    let n = (width * height) as usize;
    if rgb.len() < n * 3 {
        return Err("buffer too small".into());
    }
    let mut gray = vec![0u8; n];
    for i in 0..n {
        let o = i * 3;
        gray[i] = ((rgb[o] as u16 + rgb[o + 1] as u16 + rgb[o + 2] as u16) / 3) as u8;
    }
    let g = GrayView::new(width, height, width, &gray).ok_or("bad gray view")?;
    let ah = ahash_u64(g).map_err(|e| format!("{e:?}"))?;
    let dh = dhash_u64(g).map_err(|e| format!("{e:?}"))?;
    let mut hist = [0.0f32; COLOR_HIST_EMBED_DIM];
    let rv = RgbView::new(width, height, width * 3, rgb).ok_or("bad rgb view")?;
    color_hist_embed_rgb(rv, &mut hist).map_err(|e| format!("{e:?}"))?;
    Ok(serde_json::json!({
        "ahash": format!("{:016x}", ah),
        "dhash": format!("{:016x}", dh),
        "hist_dim": COLOR_HIST_EMBED_DIM,
        "hist0": hist[0],
        "note": "local CBIR proxy; not foundation CLIP"
    }))
}
