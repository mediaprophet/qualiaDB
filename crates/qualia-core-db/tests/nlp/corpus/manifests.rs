//! NLP-005 benchmark manifests: authored gold stays UNEVALUATED; external sets unavailable.

use super::super::gate::release_gate_succeeds;
use super::super::manifest::{load_manifest, GateStatus};

const CATCHMENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-005-catchment-notes-v0.json"
));
const ENGLISH: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-005-english-notes-v0.json"
));
const ONTONOTES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-005-ontonotes-unavailable.json"
));
const GUM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-005-gum-unavailable.json"
));
const FRAMENET: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-005-framenet-unavailable.json"
));

#[test]
fn authored_v0_manifests_are_unevaluated_not_quality_gates() {
    for (json, dataset) in [
        (CATCHMENT, "qualia-catchment-notes-v0"),
        (ENGLISH, "qualia-english-notes-v0"),
    ] {
        let m = load_manifest(json).expect(dataset);
        assert_eq!(m.dataset_id, dataset);
        assert_eq!(m.gate_status, GateStatus::Unevaluated);
        assert!(m.minimum_score.is_none());
        assert!(!m.release_required);
        assert_eq!(m.resource_envelope.network, "forbidden");
        assert!(!m.resource_envelope.download_datasets);
        assert!(!m.resource_envelope.download_models);
        assert!(!release_gate_succeeds(&m));
        assert!(m.split_hash.is_some());
    }
}

#[test]
fn external_nlp005_stubs_are_unavailable_and_do_not_pass() {
    for (json, dataset) in [
        (ONTONOTES, "ontonotes"),
        (GUM, "ud-english-gum"),
        (FRAMENET, "framenet"),
    ] {
        let m = load_manifest(json).expect(dataset);
        assert_eq!(m.dataset_id, dataset);
        assert_eq!(m.gate_status, GateStatus::Unavailable);
        assert!(m.release_required);
        assert!(!release_gate_succeeds(&m));
        let reason = m.unavailable_reason.expect("reason");
        assert!(reason.contains("not present") || reason.contains("Not present"));
        assert!(reason.to_ascii_lowercase().contains("not") && reason.contains("download"));
    }
}
