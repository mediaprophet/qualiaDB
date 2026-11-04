use super::*;
use etalon_inference_lab::auto_improve::neighbors;
use std::path::PathBuf;

#[test]
fn search_space_nonempty_and_dedup_labels() {
    let space = default_search_space();
    assert!(
        space.len() >= 8,
        "expected a real search grid, got {}",
        space.len()
    );
    let mut labels: Vec<&str> = space.iter().map(|c| c.label.as_str()).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(
        labels.len(),
        space.len(),
        "duplicate labels in search space"
    );
}

#[test]
fn neighbors_flip_bits() {
    let c = LabConfig {
        label: "base".into(),
        resident: true,
        coop: true,
        ffn_fusion: true,
        kv_int8: true,
        mode: InferenceMode::Portable,
        backend: None,
    };
    let n = neighbors(&c);
    assert!(n.len() >= 3);
    assert!(n.iter().any(|x| x.coop != c.coop));
    assert!(n.iter().any(|x| x.kv_int8 != c.kv_int8));
    assert!(n.iter().any(|x| x.mode == InferenceMode::FastVerify));
}

#[test]
fn toggles_json_is_compact() {
    let c = LabConfig {
        label: "t".into(),
        resident: true,
        coop: false,
        ffn_fusion: true,
        kv_int8: true,
        mode: InferenceMode::FastVerify,
        backend: Some("vulkan"),
    };
    let j = c.toggles_json();
    assert!(j.contains("\"resident\":true"));
    assert!(j.contains("\"coop\":false"));
    assert!(j.contains("fast-verify"));
    assert!(j.contains("vulkan"));
}

#[test]
fn lockin_summary_handles_empty_best() {
    let pkg = LockInPackage {
        best: None,
        ollama_tok_s: None,
        a_gap: None,
        trials: 0,
        generations: 0,
        elapsed_secs: 0.0,
        out_dir: PathBuf::from("experiments/inference-lab/lockin"),
        methodology: String::new(),
    };
    let s = format_lockin_summary(&pkg);
    assert!(s.contains("none"));
    assert!(s.contains("LOCK-IN SUMMARY"));
}
