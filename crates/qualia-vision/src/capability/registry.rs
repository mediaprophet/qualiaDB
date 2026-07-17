//! Machine-readable vision excellence capability registry (D1–D9 spine).

use super::entry::CapabilityEntry;
use super::status::CapabilityStatus;

/// Full registry snapshot for ledger / Studio / swarm progress.
pub fn all_capabilities() -> &'static [CapabilityEntry] {
    REGISTRY
}

pub fn by_id(id: &str) -> Option<&'static CapabilityEntry> {
    REGISTRY.iter().find(|e| e.id == id)
}

pub fn count_by_status(status: CapabilityStatus) -> usize {
    REGISTRY.iter().filter(|e| e.status == status).count()
}

macro_rules! cap {
    ($id:expr, $dom:expr, $name:expr, $st:ident, $hon:expr) => {
        CapabilityEntry::new($id, $dom, $name, CapabilityStatus::$st, $hon)
    };
}

static REGISTRY: &[CapabilityEntry] = &[
    // D1 classical
    cap!("D1.01", "D1", "buffer_roi_channels", Present, "cv/buffer"),
    cap!("D1.02", "D1", "colour_hist", Present, "cv/color + cv/hist"),
    cap!("D1.03", "D1", "filters", Present, "cv/filter gaussian/median/box"),
    cap!("D1.04", "D1", "morphology", Present, "cv/morph erode/dilate"),
    cap!("D1.05", "D1", "edges", Present, "cv/edges sobel/canny"),
    cap!("D1.06", "D1", "contours", Present, "cv/contours find_external"),
    cap!("D1.07", "D1", "warps", Present, "cv/transform affine/perspective"),
    cap!("D1.08", "D1", "features_orb_match", Present, "cv/features ORB-class + match"),
    cap!("D1.09", "D1", "optical_flow", Present, "cv/flow lucas_kanade_sparse"),
    cap!("D1.10", "D1", "codecs_png_jpeg", Partial, "feature codecs + bmp path"),
    cap!("D1.11", "D1", "video_file_io", Missing, "not yet"),
    cap!("D1.12", "D1", "camera_capture", Partial, "desktop intent pattern; vision hooks"),
    cap!("D1.13", "D1", "drawing_overlay", Present, "cv/draw + overlay"),
    cap!("D1.14", "D1", "photo_denoise", Present, "cv/photo bilateral_denoise"),
    cap!("D1.15", "D1", "stitch", Missing, "optional vertical"),
    // D2 learned
    cap!("D2.01", "D2", "detector_tracker", CompleteWithGate, "YuNet face (MIT) + YOLO-NAS (Apache-2.0) pack; grid detector until ONNX wired"),
    cap!("D2.02", "D2", "semantic_quins", Beyond, "epistemic observations"),
    cap!("D2.03", "D2", "qvwt_load", CompleteWithGate, "seed QVWT + YOLO-NAS/YuNet commercial pack"),
    cap!("D2.04", "D2", "segmentation", Missing, "gated"),
    cap!("D2.05", "D2", "depth_mono", Missing, "gated"),
    cap!("D2.06", "D2", "body_pose", Missing, "gated"),
    cap!("D2.07", "D2", "ocr", Missing, "product demand"),
    cap!("D2.08", "D2", "generative_image", Partial, "reference generator"),
    // D3 biosense
    cap!("D3.01", "D3", "face_landmarks", CompleteWithGate, "MediaPipe Face Mesh Apache-2.0 pack — see vision-excellence-commercial-model-pack.md"),
    cap!("D3.02", "D3", "frame_quality", Present, "biosense/quality"),
    cap!("D3.03", "D3", "rppg_hr", Present, "POS+CHROM ensemble + SNR"),
    cap!("D3.04", "D3", "hrv_proxy", Partial, "window/SNR gated RMSSD-class"),
    cap!("D3.05", "D3", "respiration_video", Present, "motion energy band"),
    cap!("D3.06", "D3", "eulerian_color_mag", Present, "biosense/magnification"),
    cap!("D3.07", "D3", "eulerian_motion_mag", Present, "biosense/magnification"),
    cap!("D3.08", "D3", "lagrangian_mag", Partial, "track-amplify lite"),
    cap!("D3.09", "D3", "liveness_pad", Present, "pure-landmark PAD: TTS/TTC + PnP + PAR(2D x, no model Z) + jitter"),
    cap!("D3.10", "D3", "face_template_vault", CompleteWithGate, "SFace ONNX Apache-2.0 pack — sanctuary store; ROI hash proxy until loaded"),
    cap!("D3.11", "D3", "voice_biometric", Partial, "qualia-audio speech path + policy"),
    cap!("D3.12", "D3", "multimodal_bio_fusion", Partial, "recipes fuse when consented"),
    cap!("D3.13", "D3", "affect_proposals", Present, "blendshape heuristic Path A; optional OMZ emotion Path B gated"),
    cap!("D3.14", "D3", "au_lite", Partial, "blendshape temporal events when mesh wired"),
    cap!("D3.15", "D3", "biosignal_graph", Present, "compile helpers hashes+confidence"),
    cap!("D3.16", "D3", "contact_ppg_harness", CompleteWithGate, "principal device corpus"),
    // D4 policy
    cap!("D4.01", "D4", "biosense_consent", Present, "purpose-bound consent"),
    cap!("D4.02", "D4", "deontic_biometric", Present, "evaluate_processing_act permit/deny"),
    cap!("D4.03", "D4", "sparql_mm_obs", Partial, "existing sparql_mm + vision query"),
    cap!("D4.04", "D4", "sparql_fed_policy", Partial, "local policy_ask; FED wire next"),
    cap!("D4.05", "D4", "cctv_compliance_mode", Present, "stage filter by policy"),
    cap!("D4.06", "D4", "multi_camera_graph", Missing, "federation depth"),
    cap!("D4.07", "D4", "jurisdiction_tags", Partial, "policy context field"),
    cap!("D4.08", "D4", "duress_sanctuary", Partial, "platform sanctuary"),
    cap!("D4.09", "D4", "mindware_biometric_bind", Partial, "consent co-gate"),
    cap!("D4.10", "D4", "rights_audit", Present, "biosense audit log lines"),
    // D5 3D
    cap!("D5.01", "D5", "mesh_ir_validate", Present, "spatial/validate"),
    cap!("D5.02", "D5", "obj_export", Present, "spatial/export_obj"),
    cap!("D5.03", "D5", "stl_export", Present, "spatial/export_stl"),
    cap!("D5.04", "D5", "3mf_export", Partial, "minimal 3mf package"),
    cap!("D5.05", "D5", "gltf_handoff", Partial, "core render path"),
    cap!("D5.06", "D5", "image_to_3d_heightfield", Present, "spatial/image_to_3d"),
    cap!("D5.07", "D5", "image_to_3d_multiview", Missing, "recon depth"),
    cap!("D5.08", "D5", "photogrammetry", Missing, "recon depth"),
    cap!("D5.09", "D5", "print_readiness", Present, "spatial/print_readiness"),
    cap!("D5.10", "D5", "printer_envelope", Present, "print_readiness envelope"),
    cap!("D5.11", "D5", "synthetic_train_scenes", Present, "synthetic module"),
    cap!("D5.12", "D5", "synthetic_3d_corpora", Partial, "heightfield meshes"),
    cap!("D5.13", "D5", "10d_handoff", Present, "geometry quins path"),
    cap!("D5.14", "D5", "twin_a1_preview", Present, "twin_bridge A1"),
    // D6 engineering wire
    cap!("D6.01", "D6", "comp_geom_ingest", Beyond, "computational_geometry crate"),
    cap!("D6.02", "D6", "fem_preview_link", Partial, "A1 bar stretch only"),
    cap!("D6.03", "D6", "cfd_optional", Partial, "engineering_analysis"),
    cap!("D6.04", "D6", "physics_sim", Beyond, "physics_simulation"),
    cap!("D6.05", "D6", "linalg_numerics", Beyond, "linear_algebra"),
    cap!("D6.06", "D6", "symbolic_math", Beyond, "symbolic_*"),
    cap!("D6.07", "D6", "assurance_a0_a4", Present, "honesty labels twin/FEA"),
    // D7 biology
    cap!("D7.01", "D7", "medical_imaging_path", Partial, "medical_computing + sensitivity"),
    cap!("D7.02", "D7", "anatomy_mesh_graph", Partial, "Anatomy QApp"),
    cap!("D7.03", "D7", "microscopy", Missing, "vertical"),
    cap!("D7.04", "D7", "cell_track", Missing, "vertical"),
    cap!("D7.05", "D7", "biomarker_graph", Partial, "Anatomy knowledge"),
    cap!("D7.06", "D7", "cheminformatics", Beyond, "medical_computing"),
    cap!("D7.07", "D7", "clinical_formulas", Beyond, "non-vision"),
    cap!("D7.08", "D7", "hipaa_process_notes", Partial, "compliance module"),
    cap!("D7.09", "D7", "bio_twin_viz", Present, "viz-only default"),
    // D8 multimodal
    cap!("D8.01", "D8", "shared_media_clock", Present, "qualia-audio"),
    cap!("D8.02", "D8", "av_correlation", Present, "non-causal"),
    cap!("D8.03", "D8", "joint_biosense", Partial, "recipe-level"),
    cap!("D8.04", "D8", "cross_modal_train_export", Missing, "later"),
    // D9 surfaces
    cap!("D9.01", "D9", "studio_vision_workbench", Partial, "existing workbench"),
    cap!("D9.02", "D9", "wellfair_handoff", Partial, "optional export"),
    cap!("D9.03", "D9", "library_catalogue", Present, "perception_catalog"),
    cap!("D9.04", "D9", "wasm_edge_profile", Partial, "capability subset"),
    cap!("D9.05", "D9", "desktop_camera_ux", Partial, "consent UX"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_nonempty_unique_ids() {
        assert!(REGISTRY.len() >= 40);
        for (i, a) in REGISTRY.iter().enumerate() {
            for (j, b) in REGISTRY.iter().enumerate() {
                if i != j {
                    assert_ne!(a.id, b.id);
                }
            }
        }
    }

    #[test]
    fn present_count_positive() {
        assert!(count_by_status(CapabilityStatus::Present) >= 15);
    }
}
