//! End-to-end native vision pipeline for first-release product path.
//!
//! Synthetic or raw RGB → grid detect → optional track → epistemic quins →
//! overlay BMP. No Python. No Ollama.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use qualia_core_db::sparql_library::vision_shacl::{
    validate_vision_observation_graph, VisionShaclReport,
};
use qualia_core_db::NQuin;
use qualia_vision::detector::GridMultiObjectDetector;
use qualia_vision::overlay::{
    box_css_percent, compose_rgb_overlay_rgba8, encode_bmp_rgba8,
};
use qualia_vision::semantic::{compile_observation_quins_full, media_digest, VisionQuin};
use qualia_vision::synthetic::{
    generate_scene_rgb8, sample_id, DatasetSplit, SyntheticSampleId,
};
use qualia_vision::tracker::BoundedTracker;
use qualia_vision::types::{Detection, ImageView, PixelFormat, MAX_DETECTIONS};
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
    let sample = sample_id(split, index, width, height);
    run_sample_demo(&sample)
}

pub fn run_sample_demo(sample: &SyntheticSampleId) -> Result<VisionDemoResult, String> {
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
    let det = GridMultiObjectDetector::new(4, 3);
    let mut preds = [Detection::empty(); MAX_DETECTIONS];
    let mut ws = [0u8; MAX_DETECTIONS];
    let n_pred = det
        .detect(img, 0, &mut preds, &mut ws)
        .map_err(|e| format!("{e:?}"))?;

    let mut tracker = BoundedTracker::new();
    tracker.update(0, &mut preds, n_pred);

    let digest = media_digest(&rgb);
    let model_hash = det.model_hash();
    let mut vquins = [VisionQuin::with_parity(0, 0, 0, 0, 0); 256];
    let n_q = compile_observation_quins_full(digest, &preds[..n_pred], model_hash, &mut vquins);

    let mut nquins = Vec::with_capacity(n_q);
    for q in vquins.iter().take(n_q) {
        nquins.push(vision_quin_to_nquin(q));
    }
    let report = validate_vision_observation_graph(&nquins);

    let mut rgba = vec![0u8; px * 4];
    compose_rgb_overlay_rgba8(
        w,
        h,
        &rgb,
        &preds,
        n_pred,
        [0, 255, 180, 255],
        2,
        &mut rgba,
    )
    .map_err(|e| format!("{e:?}"))?;
    let mut bmp = vec![0u8; 54 + px * 4];
    let bmp_n = encode_bmp_rgba8(w, h, &rgba, &mut bmp).map_err(|e| format!("{e:?}"))?;
    let b64 = B64.encode(&bmp[..bmp_n]);

    let split_s = match sample.split {
        DatasetSplit::Train => "train",
        DatasetSplit::Test => "test",
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
        note: "Epistemic proposals only — not ground truth. Reject/correct preserves machine claims."
            .into(),
    })
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
        let instance_hash =
            u64::from_str_radix(d.instance_hash.trim_start_matches("0x"), 16)
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
    }
}
