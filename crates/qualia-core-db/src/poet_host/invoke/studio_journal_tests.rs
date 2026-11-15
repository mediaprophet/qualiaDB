//! Source safety tests; the loft/map helper is test-only kernel regeneration,
//! not an installed organic editor, preview renderer, codec or model package.
use super::*;

fn source(id: &str) -> StudioArtifact {
    let mut art = StudioArtifact::new(id, "Geometry", "Editable branch surface fixture");
    art.properties.insert("subject_identifier".into(), "urn:test:organism".into());
    art.properties.insert("part_identifier".into(), "urn:test:branch".into());
    art.properties.insert("source_author".into(), "urn:test:contributor".into());
    art.properties.insert("roughness".into(), "0.8".into());
    for (section, z) in [0.0, 0.8, 1.6].into_iter().enumerate() {
        for (point, [x, y]) in [[-0.08, -0.04], [0.06, -0.05], [0.09, 0.02], [0.01, 0.08], [-0.07, 0.04]].into_iter().enumerate() {
            let taper = 1.0 - section as f32 * 0.18;
            art.nodes.insert(format!("section-{section}-point-{point}"), BTreeMap::from([
                ("x".into(), (x * taper).to_string()),
                ("y".into(), (y * taper).to_string()),
                ("z".into(), z.to_string()),
            ]));
        }
    }
    art
}

// Uses existing public native kernels; no files or package exports are produced.
fn regenerate(art: &StudioArtifact) -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<u8>) {
    use crate::render::surface_authoring::{generate_surface_maps_into, SurfaceRecipe};
    use crate::specialized_libs::computational_geometry::authoring::loft_convex_sections_into;
    let sections: Vec<Vec<[f32; 3]>> = (0..3).map(|section| {
        (0..5).map(|point| {
            let node = &art.nodes[&format!("section-{section}-point-{point}")];
            [node["x"].parse().unwrap(), node["y"].parse().unwrap(), node["z"].parse().unwrap()]
        }).collect()
    }).collect();
    let mut positions = vec![[0.0; 3]; 64];
    let mut triangles = vec![[0; 3]; 64];
    let receipt = loft_convex_sections_into(&sections, &mut positions, &mut triangles).unwrap();
    positions.truncate(receipt.vertex_count);
    triangles.truncate(receipt.triangle_count);
    let (mut colour, mut normal, mut orm) = (vec![0; 256], vec![0; 256], vec![0; 256]);
    generate_surface_maps_into(SurfaceRecipe {
        seed: 13, tint: [0.5, 0.32, 0.16], contrast: 0.2, relief: 0.002,
        bands: [12, 1], band_strength: 0.7,
        roughness: art.properties["roughness"].parse().unwrap(), metallic: 0.0,
    }, 8, &mut colour, &mut normal, &mut orm).unwrap();
    (positions, triangles, orm)
}

#[test]
fn source_undo_redo_regenerates_identical_loft_and_material() {
    let original = source("journal-loft-material");
    let before = regenerate(&original);
    upsert_artifact(original.clone());
    let edited = mutate_artifact_at(&original.id, Some(1), |art| {
        art.nodes.get_mut("section-1-point-0").unwrap().insert("x".into(), "-0.09".into());
        art.properties.insert("roughness".into(), "0.35".into());
        art.title = "Edited surface fixture".into();
    }).unwrap();
    let after = regenerate(&edited);
    assert_ne!(before.0, after.0);
    assert_ne!(before.2, after.2);
    let undone = undo_artifact_at(&original.id, Some(2)).unwrap();
    assert_eq!(regenerate(&undone), before);
    assert_eq!(undone.nodes, original.nodes);
    assert_eq!(undone.properties, original.properties);
    assert_eq!(undone.title, original.title);
    assert_eq!((undone.did_q42, &undone.domain), (original.did_q42, &original.domain));
    assert_eq!(undone.lamport_clock, 3);
    assert!(undone.history.starts_with(&edited.history));
    let redone = redo_artifact_at(&original.id, Some(3)).unwrap();
    assert_eq!(regenerate(&redone), after);
    assert_eq!(redone.lamport_clock, 4);
    assert!(redone.history.starts_with(&undone.history));
    assert!(!redone.properties.contains_key("_undo_last"));
    assert!(!redone.properties.contains_key("_redo_last"));
}

#[test]
fn stale_edits_and_undo_redo_are_atomic() {
    let id = "journal-stale";
    upsert_artifact(source(id));
    mutate_artifact_at(id, Some(1), |art| { art.title = "Second".into(); }).unwrap();
    let before = get_artifact(id).unwrap();
    let mut called = false;
    assert!(mutate_artifact_at(id, Some(1), |_| { called = true; }).is_err());
    assert!(!called);
    assert!(undo_artifact_at(id, Some(1)).is_err());
    assert!(redo_artifact_at(id, Some(1)).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
    undo_artifact_at(id, Some(2)).unwrap();
    let before = get_artifact(id).unwrap();
    assert!(redo_artifact_at(id, Some(2)).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
    mutate_artifact_at(id, Some(3), |art| { art.title = "New branch".into(); }).unwrap();
    assert!(redo_artifact(id).is_err());
}

#[test]
fn concurrent_same_revision_accepts_one_edit_without_running_stale_callback() {
    use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, Barrier};
    let id = "journal-concurrent-revision";
    let original = source(id);
    upsert_artifact(original.clone());
    let barrier = Arc::new(Barrier::new(3));
    let callbacks = Arc::new([AtomicUsize::new(0), AtomicUsize::new(0)]);
    let handles: Vec<_> = (0..2).map(|index| {
        let barrier = Arc::clone(&barrier);
        let callbacks = Arc::clone(&callbacks);
        std::thread::spawn(move || {
            barrier.wait();
            let result = mutate_artifact_at(id, Some(1), |art| {
                callbacks[index].fetch_add(1, Ordering::SeqCst);
                art.title = format!("Contributor edit {index}");
                art.properties.insert("roughness".into(), ["0.3", "0.6"][index].into());
            });
            (index, result)
        })
    }).collect();
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|(_, result)| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|(_, result)| result.is_err()).count(), 1);
    let (winner, accepted) = results.iter().find(|(_, result)| result.is_ok()).unwrap();
    let (loser, rejected) = results.iter().find(|(_, result)| result.is_err()).unwrap();
    assert!(rejected.as_ref().unwrap_err().contains("Stale Studio revision"));
    assert_eq!(callbacks[*winner].load(Ordering::SeqCst), 1);
    assert_eq!(callbacks[*loser].load(Ordering::SeqCst), 0);
    let stored = get_artifact(id).unwrap();
    assert_eq!(&stored, accepted.as_ref().unwrap());
    assert_eq!(stored.title, format!("Contributor edit {winner}"));
    assert_eq!(stored.properties["roughness"], ["0.3", "0.6"][*winner]);
    assert_eq!(stored.nodes, original.nodes);
    assert_eq!(stored.properties["source_author"], original.properties["source_author"]);
    assert_eq!(stored.lamport_clock, 2);
    assert_eq!(stored.history, vec!["edit:1->2"]);
    let undone = undo_artifact_at(id, Some(2)).unwrap();
    assert_eq!((undone.title, undone.nodes, undone.properties),
        (original.title, original.nodes, original.properties));
    assert_eq!(undone.history, vec!["edit:1->2", "undo:2->3"]);
}

#[test]
fn rejected_source_or_policy_edits_leave_all_state_unchanged() {
    let id = "journal-policy-budget";
    upsert_artifact(source(id));
    let before = get_artifact(id).unwrap();
    for field in ["id", "domain", "policy", "clock", "audit"] {
        assert!(mutate_artifact_at(id, Some(1), |art| match field {
            "id" => art.id = "replacement".into(),
            "domain" => art.domain = "replacement".into(),
            "policy" => art.sensitivity = 2,
            "clock" => art.lamport_clock = 9,
            _ => art.history.push("forged audit".into()),
        }).is_err());
        assert_eq!(get_artifact(id).unwrap(), before);
    }
    let before = get_artifact(id).unwrap();
    assert!(mutate_artifact(id, |art| art.history.push("forged audit".into())).is_err());
    assert!(mutate_artifact(id, |art| art.title = "x".repeat(journal::MAX_SOURCE_BYTES + 1)).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
}

#[test]
fn clock_and_history_limits_refuse_without_wrapping_or_losing_source() {
    let mut art = source("journal-clock-limit");
    art.lamport_clock = journal::MAX_REVISION - 1;
    upsert_artifact(art);
    let final_state = mutate_artifact("journal-clock-limit", |art| art.title = "Last valid".into()).unwrap();
    assert_eq!(final_state.lamport_clock, journal::MAX_REVISION);
    assert!(undo_artifact("journal-clock-limit").is_err());
    assert!(mutate_artifact("journal-clock-limit", |_| {}).is_err());
    assert_eq!(get_artifact("journal-clock-limit").unwrap(), final_state);
    let id = "journal-step-limit";
    upsert_artifact(source(id));
    for step in 0..journal::MAX_JOURNAL_STEPS {
        mutate_artifact(id, |art| art.title = step.to_string()).unwrap();
    }
    let before = get_artifact(id).unwrap();
    assert!(mutate_artifact(id, |_| {}).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
    undo_artifact(id).unwrap();
    redo_artifact(id).unwrap();
}

#[test]
fn legacy_replacement_and_import_cannot_reuse_old_snapshots() {
    let id = "journal-replacement";
    upsert_artifact(source(id));
    mutate_artifact(id, |art| art.title = "Edited".into()).unwrap();
    let mut replacement = get_artifact(id).unwrap();
    replacement.title = "Replacement through legacy API".into();
    upsert_artifact(replacement.clone());
    assert!(undo_artifact(id).is_err());
    assert_eq!(get_artifact(id).unwrap().title, replacement.title);
    import_into_store("journal-import", "Geometry", "md", "# Imported source").unwrap();
    assert!(undo_artifact("journal-import").is_err());
}

#[test]
fn explicit_host_limits_admit_large_sources_and_refuse_lossy_reduction() {
    let id = "journal-host-limits";
    upsert_artifact(source(id));
    let limits = SourceJournalLimits { source_bytes: 2 * journal::MAX_SOURCE_BYTES,
        journal_bytes: 32 * journal::MAX_SOURCE_BYTES, ..SourceJournalLimits::default() };
    configure_artifact_journal(id, Some(1), limits).unwrap();
    let large = "detailed editable source ".repeat(50_000);
    assert!(large.len() > journal::MAX_SOURCE_BYTES);
    let edited = mutate_artifact_at(id, Some(2), |art| art.title = large.clone()).unwrap();
    assert!(configure_artifact_journal(id, Some(3), SourceJournalLimits::default()).is_err());
    assert_eq!(get_artifact(id).unwrap(), edited);
    let undone = undo_artifact_at(id, Some(3)).unwrap();
    assert_eq!(undone.title, "Editable branch surface fixture");
    assert_eq!(redo_artifact_at(id, Some(4)).unwrap().title, large);
}

#[test]
fn retained_byte_and_audit_budgets_refuse_atomically() {
    let id = "journal-byte-budget";
    upsert_artifact(source(id));
    let limits = SourceJournalLimits { journal_bytes: 80_000, ..SourceJournalLimits::default() };
    configure_artifact_journal(id, Some(1), limits).unwrap();
    for _ in 0..4 { mutate_artifact(id, |_| {}).unwrap(); }
    let before = get_artifact(id).unwrap();
    assert!(mutate_artifact(id, |_| {}).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);

    let id = "journal-audit-budget";
    upsert_artifact(source(id));
    configure_artifact_journal(id, Some(1), SourceJournalLimits {
        audit_entries: 4, ..SourceJournalLimits::default()
    }).unwrap();
    mutate_artifact(id, |_| {}).unwrap();
    undo_artifact(id).unwrap();
    redo_artifact(id).unwrap();
    let before = get_artifact(id).unwrap();
    assert_eq!(before.history.len(), 4);
    assert!(undo_artifact(id).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
}

#[test]
fn prospective_audit_bytes_are_admitted_before_every_activation() {
    let id = "journal-audit-byte-boundary";
    upsert_artifact(StudioArtifact::new(id, "Geometry", ""));
    let limits = SourceJournalLimits { source_bytes: 64, ..SourceJournalLimits::default() };
    configure_artifact_journal(id, Some(1), limits).unwrap();
    mutate_artifact(id, |_| {}).unwrap();
    undo_artifact(id).unwrap();
    redo_artifact(id).unwrap();
    mutate_artifact(id, |_| {}).unwrap();
    undo_artifact(id).unwrap();
    let before = get_artifact(id).unwrap();
    assert_eq!(before.history.iter().map(String::len).sum::<usize>(), 64);
    assert!(mutate_artifact(id, |_| {}).is_err());
    assert!(undo_artifact(id).is_err());
    assert!(redo_artifact(id).is_err());
    // A configuration change must budget its own append-only audit event too.
    assert!(configure_artifact_journal(id, Some(7), limits).is_err());
    assert_eq!(get_artifact(id).unwrap(), before);
    configure_artifact_journal(id, Some(7), SourceJournalLimits { source_bytes: 128, ..limits }).unwrap();
    // Refused restore did not consume the pending redo snapshot.
    assert_eq!(redo_artifact_at(id, Some(8)).unwrap().lamport_clock, 9);
}

#[test]
fn existing_studio_handlers_accept_checked_and_legacy_calls() {
    use super::super::studio_domains::{studio_edit, studio_redo, studio_undo};
    use vibe::{Span, Value};
    let id = "journal-invoke";
    upsert_artifact(source(id));
    let span = Span { start: 0, end: 0 };
    let mut input = BTreeMap::from([
        ("id".into(), Value::String(id.into())),
        ("prop_key".into(), Value::String("roughness".into())),
        ("prop_val".into(), Value::String("0.4".into())),
        ("expected_revision".into(), Value::U64(1)),
    ]);
    studio_edit(&Value::Record(input.clone()), span).unwrap();
    assert!(studio_undo(&Value::Record(input.clone()), span).is_err());
    input.insert("expected_revision".into(), Value::U64(2));
    studio_undo(&Value::Record(input.clone()), span).unwrap();
    input.insert("expected_revision".into(), Value::U64(3));
    studio_redo(&Value::Record(input.clone()), span).unwrap();
    for invalid in [Value::String("4".into()), Value::U64(0), Value::U64(u64::MAX), Value::F64(4.5)] {
        input.insert("expected_revision".into(), invalid);
        let before = get_artifact(id).unwrap();
        assert!(studio_edit(&Value::Record(input.clone()), span).is_err());
        assert_eq!(get_artifact(id).unwrap(), before);
    }
    input.remove("expected_revision");
    studio_edit(&Value::Record(input), span).unwrap();
    assert_eq!(get_artifact(id).unwrap().lamport_clock, 5);
}
