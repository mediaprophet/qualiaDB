//! NLP-004 evaluation scaffold — integration-test entry.
//!
//! Cargo auto-discovers `tests/*.rs` only. Nested modules live under
//! `tests/nlp/` and are declared here.
//!
//! This suite is a **plumbing / schema / authored-gold harness**, not a quality claim:
//! no F1, no accuracy, no dataset download. Authored gold may honestly freeze
//! known splitter defects (`Dr.`, `e.g.`) until NLP-102.

mod nlp;

use nlp::gate::release_gate_succeeds;
use nlp::manifest::{load_manifest, GateStatus, UNAVAILABLE_MANIFEST_JSON};

#[test]
fn unavailable_external_stub_does_not_succeed_the_gate() {
    let manifest = load_manifest(UNAVAILABLE_MANIFEST_JSON).expect("unavailable stub must parse");
    assert_eq!(manifest.gate_status, GateStatus::Unavailable);
    assert_eq!(manifest.gate_status.as_str(), "unavailable");
    assert!(manifest.release_required);
    assert!(manifest
        .unavailable_reason
        .as_ref()
        .expect("reason required")
        .contains("licensed"));
    assert!(!release_gate_succeeds(&manifest));
}
