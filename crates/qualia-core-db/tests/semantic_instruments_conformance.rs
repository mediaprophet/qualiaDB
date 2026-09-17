//! SI-12 public-API conformance for semantic instruments.
//!
//! Catalogue / RPL / reference siblings are not imported unless re-exported
//! from `semantic_instruments`. Dispatch is by entry point, never a Host ID.

use qualia_core_db::semantic_instruments::attestation::CapabilityAward;
use qualia_core_db::semantic_instruments::resolve::DependencyKind;
use qualia_core_db::semantic_instruments::{
    assert_can_publish, build_demo, build_reference, demo_catalog, inspect_for_publish,
    issue_capability_award, open_collectable, plan_resolution, preflight, reference_catalog,
    run_entry, sha256_prefixed, CatalogRecord, ContentCategory, DependencyLock, DependencyReq,
    InstalledRelease, InstrumentError, LocalCatalogue, LocalRegistry, RunOutcome, RunRequest,
    WASM_Q42_HELD,
};

const UNIT_CONVERT: &str = "https://ns.webizen.org/demo/unit-convert/releases/1.0.0";
const AWARD_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn empty_lock(release_id: &str) -> DependencyLock {
    DependencyLock {
        release_id: release_id.into(),
        entries: Vec::new(),
    }
}

fn closed_unit_convert() -> (LocalRegistry, String, String) {
    let opened = open_collectable(&build_demo("unit-convert").expect("demo")).expect("open");
    let mut registry = LocalRegistry::new();
    registry
        .install(InstalledRelease {
            release_id: opened.manifest.release_id.clone(),
            content_digest: opened.manifest.content_digest.clone(),
            lock: empty_lock(&opened.manifest.release_id),
            closed: true,
        })
        .unwrap();
    (
        registry,
        opened.manifest.release_id,
        opened.manifest.content_digest,
    )
}

fn request<'a>(release_id: &'a str, entry_point: &'a str, input: &'a [u8]) -> RunRequest<'a> {
    RunRequest {
        release_id,
        entry_point,
        input,
        now: 1,
        cancelled: false,
        max_input_bytes: 64,
        max_steps: 8,
        parent_receipt: None,
    }
}

fn completed(outcome: RunOutcome) -> qualia_core_db::semantic_instruments::ExecutionReceipt {
    match outcome {
        RunOutcome::Completed { receipt } => receipt,
        RunOutcome::Held { .. } => panic!("expected Completed, got Held"),
        RunOutcome::Refused { error } => panic!("expected Completed, got Refused {error}"),
    }
}

fn held(outcome: RunOutcome) -> qualia_core_db::semantic_instruments::ExecutionReceipt {
    match outcome {
        RunOutcome::Held { receipt } => receipt,
        RunOutcome::Completed { .. } => panic!("expected Held, got Completed"),
        RunOutcome::Refused { error } => panic!("expected Held, got Refused {error}"),
    }
}

fn refused(outcome: RunOutcome) -> InstrumentError {
    match outcome {
        RunOutcome::Refused { error } => error,
        RunOutcome::Completed { .. } => panic!("expected Refused, got Completed"),
        RunOutcome::Held { .. } => panic!("expected Refused, got Held"),
    }
}

fn dep(id: &str, required: bool) -> DependencyReq {
    DependencyReq {
        id: id.into(),
        kind: DependencyKind::Ontology,
        version_constraint: "1.0.0".into(),
        expected_digest: String::new(),
        required,
        purpose: "conformance".into(),
    }
}

#[test]
fn demo_unit_convert_hmc_opens_as_demo() {
    let opened = open_collectable(&build_demo("unit-convert").unwrap()).unwrap();
    assert_eq!(opened.manifest.content_category, ContentCategory::Demo);
    assert_eq!(opened.manifest.release_id, UNIT_CONVERT);
    assert_eq!(opened.manifest.entry_point, "assess");
    assert!(opened.manifest.honesty_notice.to_ascii_lowercase().contains("demo"));
}

#[test]
fn closed_install_assess_demo_ok_matches_output_commitment() {
    let (registry, release_id, content_digest) = closed_unit_convert();
    let req = request(&release_id, "assess", b"demo-ok");
    let installed = preflight(&registry, &req).unwrap();
    assert!(installed.closed);
    let receipt = completed(run_entry(&registry, installed, &req));
    assert_eq!(receipt.release_id, release_id);
    assert_eq!(receipt.content_digest, content_digest);
    assert_eq!(receipt.lock_digest, installed.lock.digest());
    assert_eq!(receipt.entry_point, "assess");
    assert_eq!(receipt.outcome, "completed");
    assert_eq!(receipt.input_commitment, sha256_prefixed(b"demo-ok"));
    assert_eq!(receipt.output_commitment, sha256_prefixed(b"demo-ok"));
}

#[test]
fn empty_input_is_held() {
    let (registry, release_id, _) = closed_unit_convert();
    let req = request(&release_id, "assess", b"");
    let installed = preflight(&registry, &req).unwrap();
    let receipt = held(run_entry(&registry, installed, &req));
    assert_eq!(receipt.outcome, "held");
    assert_eq!(receipt.input_commitment, sha256_prefixed(b""));
    assert_eq!(receipt.output_commitment, sha256_prefixed(b""));
}

#[test]
fn framingham_entry_is_unknown_entry_point() {
    let (registry, release_id, _) = closed_unit_convert();
    let req = request(&release_id, "Host.ClinicalRisk.framingham", b"demo-ok");
    let installed = preflight(&registry, &req).unwrap();
    assert_eq!(
        refused(run_entry(&registry, installed, &req)),
        InstrumentError::UnknownEntryPoint
    );
}

#[test]
fn plan_resolution_optional_missing_ok_required_missing_errors() {
    let lock = plan_resolution(UNIT_CONVERT, &[dep("ont", false)], &[]).unwrap();
    assert!(lock.entries.is_empty());
    let err = plan_resolution(UNIT_CONVERT, &[dep("ont", true)], &[]).unwrap_err();
    assert_eq!(err, InstrumentError::RequiredMissing);
    let catalog = [CatalogRecord {
        id: "ont".into(),
        version: "1.0.0".into(),
        digest: "sha256:aa".into(),
        locator: "local:ont".into(),
        revoked: false,
        available_offline: true,
    }];
    let pinned = plan_resolution(UNIT_CONVERT, &[dep("ont", true)], &catalog).unwrap();
    assert_eq!(pinned.entries.len(), 1);
}

#[test]
fn capability_award_origin_is_not_truth_and_learner_is_not_instrument() {
    let award = CapabilityAward {
        learner: "did:example:learner".into(),
        assessment_release_id: UNIT_CONVERT.into(),
        content_digest: AWARD_DIGEST.into(),
        issuer: "did:example:issuer".into(),
        capability: "https://ns.webizen.org/demo/concepts/units".into(),
        issued_at: 1_700_000_000,
        valid_until: 1_800_000_000,
    };
    let issued = issue_capability_award(award.clone()).unwrap();
    assert!(issued.origin_is_not_truth);
    assert_ne!(issued.award_subject, issued.assessment_release_id);
    let mut collide = award;
    collide.learner = collide.assessment_release_id.clone();
    assert_eq!(
        issue_capability_award(collide).unwrap_err(),
        InstrumentError::AwardIsInstrument
    );
}

#[test]
fn reference_simple_units_opens_as_reference() {
    let opened = open_collectable(&build_reference("simple-units").unwrap()).unwrap();
    assert_eq!(opened.manifest.content_category, ContentCategory::Reference);
    assert!(opened
        .manifest
        .honesty_notice
        .to_ascii_lowercase()
        .contains("reference pack"));
}

#[test]
fn local_catalogue_publish_demo_not_endorsed_empty_incomplete() {
    let mut cat = LocalCatalogue::new();
    let listing = cat.publish(build_demo("unit-convert").unwrap()).unwrap();
    assert!(listing.listed);
    let discovered = cat.discover();
    assert_eq!(discovered.len(), 1);
    assert!(!discovered[0].endorsed);
    assert_eq!(
        cat.publish(Vec::new()).unwrap_err(),
        InstrumentError::IncompleteDownload
    );
    assert_eq!(cat.discover().len(), 1);
}

#[test]
fn inspect_for_publish_unit_convert_ok_empty_fails() {
    let insp = inspect_for_publish(&build_demo("unit-convert").unwrap()).unwrap();
    assert_eq!(insp.category, ContentCategory::Demo);
    assert!(insp.has_source_n3);
    assert_can_publish(&insp).unwrap();
    assert!(inspect_for_publish(&[]).is_err());
}

#[test]
fn wasm_q42_held_notice_names_held_and_q42() {
    assert!(WASM_Q42_HELD.contains("held"));
    assert!(WASM_Q42_HELD.contains("Q42"));
}

#[test]
fn malformed_sixteen_bytes_is_not_a_collectable() {
    assert!(open_collectable(&[0u8; 16]).is_err());
}

#[test]
fn wellbeing_training_is_not_diagnosis_as_fact() {
    let opened = open_collectable(&build_reference("wellbeing-training").unwrap()).unwrap();
    assert_eq!(opened.manifest.content_category, ContentCategory::Reference);
    let blob = format!(
        "{} {}",
        opened.manifest.accessible_text, opened.manifest.honesty_notice
    );
    let low = blob.to_ascii_lowercase();
    assert!(low.contains("not diagnosis") || blob.contains("Training"));
    assert!(!low.contains("diagnosis as fact"));
}

#[test]
fn demo_catalog_every_seed_accessible_text_non_empty() {
    assert!(!demo_catalog().is_empty());
    for seed in demo_catalog() {
        let opened = open_collectable(&build_demo(seed.slug).unwrap()).unwrap();
        assert!(
            !opened.manifest.accessible_text.is_empty(),
            "{}",
            seed.slug
        );
    }
}

#[test]
fn reference_catalog_every_seed_honesty_names_reference_pack() {
    assert!(!reference_catalog().is_empty());
    for seed in reference_catalog() {
        assert!(
            seed.honesty_notice
                .to_ascii_lowercase()
                .contains("reference pack"),
            "{}",
            seed.slug
        );
    }
}

#[test]
fn open_collectable_demo_has_a11y_handle() {
    let opened = open_collectable(&build_demo("unit-convert").unwrap()).unwrap();
    assert!(!opened.manifest.accessible_text.is_empty());
    assert!(!opened.visual_10d.is_empty());
}

#[test]
fn si09_author_to_publish_via_catalogue() {
    let bytes = build_demo("unit-convert").unwrap();
    let insp = inspect_for_publish(&bytes).unwrap();
    assert!(insp.has_source_n3);
    assert!(insp.has_visual_10d);
    assert!(insp.labelled);
    // Native embeds q42; WASM demos may declare round-trip loss — either is inspectable.
    let _ = insp.round_trip_loss;
    assert_can_publish(&insp).unwrap();
    let mut cat = LocalCatalogue::new();
    let listing = cat.publish(bytes).unwrap();
    assert!(!listing.endorsed);
    assert_eq!(listing.release_id, UNIT_CONVERT);
    assert!(!listing.name.contains("Host."));
}

#[test]
fn si10_offline_resolved_required_dep_runs_closed_install() {
    let dep_id = "https://ns.webizen.org/demo/concepts/units";
    let catalog = [CatalogRecord {
        id: dep_id.into(),
        version: "1.0.0".into(),
        digest: "sha256:offline-units".into(),
        locator: "local:units".into(),
        revoked: false,
        available_offline: true,
    }];
    let reqs = [DependencyReq {
        id: dep_id.into(),
        kind: DependencyKind::Ontology,
        version_constraint: "1.0.0".into(),
        expected_digest: String::new(),
        required: true,
        purpose: "offline-run".into(),
    }];
    let lock = plan_resolution(UNIT_CONVERT, &reqs, &catalog).unwrap();
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].locator, "local:units");

    let (registry, release_id, _) = closed_unit_convert();
    let req = request(&release_id, "assess", b"demo-ok");
    let installed = preflight(&registry, &req).unwrap();
    let receipt = completed(run_entry(&registry, installed, &req));
    assert_eq!(receipt.outcome, "completed");
}

#[test]
fn fixture_parity_demo_seeds_entry_points_never_host() {
    for seed in demo_catalog() {
        assert!(
            seed.entry_point == "assess" || seed.entry_point == "recognise",
            "{}",
            seed.slug
        );
        assert!(!seed.entry_point.contains("Host."));
        assert!(!seed.slug.contains("Host."));
        let opened = open_collectable(&build_demo(seed.slug).unwrap()).unwrap();
        assert_eq!(opened.manifest.entry_point, seed.entry_point);
        assert_eq!(opened.manifest.content_category, ContentCategory::Demo);
    }
    assert!(WASM_Q42_HELD.contains("held"));
    assert!(!WASM_Q42_HELD.contains("Host."));
}
