use super::*;
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, Barrier};

#[test]
fn identical_upsert_invalidates_old_stamp_and_cannot_restore_erased_journal() {
    let id = "stamp-identical-replacement";
    let (_, initial) = create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "Original")).unwrap();
    let (edited, old) = mutate_artifact_stamped(id, &initial, |a| { a.title = "Edited".into(); Ok(()) }).unwrap();
    upsert_artifact(edited.clone());
    let (replacement, current) = get_artifact_stamped(id).unwrap();
    assert_eq!(replacement, edited);
    assert_eq!(old.revision(), current.revision());
    assert_ne!(old, current);
    let mut called = false;
    assert!(mutate_artifact_stamped(id, &old, |_| { called = true; Ok(()) }).is_err());
    assert!(!called);
    assert!(undo_artifact_stamped(id, &old).is_err());
    assert!(redo_artifact_stamped(id, &old).is_err());
    assert_eq!(get_artifact_stamped(id).unwrap(), (replacement, current.clone()));
    assert!(undo_artifact_stamped(id, &current).is_err());
}

#[test]
fn foreign_stamp_and_create_conflict_leave_existing_source_unchanged() {
    let id = "stamp-create-protected";
    let original = StudioArtifact::new(id, "Geometry", "Accepted");
    let (_, stamp) = create_artifact_if_absent(original.clone()).unwrap();
    let (_, foreign) = create_artifact_if_absent(StudioArtifact::new("stamp-other", "Geometry", "Other")).unwrap();
    assert!(create_artifact_if_absent(StudioArtifact::new(id, "Other", "Replacement")).is_err());
    assert!(mutate_artifact_stamped(id, &foreign, |_| panic!("foreign callback must not run")).is_err());
    assert!(undo_artifact_stamped(id, &foreign).is_err());
    assert!(redo_artifact_stamped(id, &foreign).is_err());
    assert_eq!(get_artifact_stamped(id).unwrap(), (original, stamp));
}

#[test]
fn fallible_candidate_error_preserves_source_revision_and_both_stacks() {
    let id = "stamp-fallible-edit";
    let (_, first) = create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "First")).unwrap();
    let (_, second) = mutate_artifact_stamped(id, &first, |a| { a.title = "Second".into(); Ok(()) }).unwrap();
    let before = get_artifact_stamped(id).unwrap();
    let error = mutate_artifact_stamped(id, &second, |a| { a.title = "Rejected".into(); Err("solver refused".into()) }).unwrap_err();
    assert_eq!(error, "solver refused");
    assert_eq!(get_artifact_stamped(id).unwrap(), before);
    let (undone, third) = undo_artifact_stamped(id, &second).unwrap();
    assert_eq!(undone.title, "First");
    let before = get_artifact_stamped(id).unwrap();
    assert!(mutate_artifact_stamped(id, &third, |a| { a.properties.clear(); Err("still refused".into()) }).is_err());
    assert_eq!(get_artifact_stamped(id).unwrap(), before);
    let (redone, fourth) = redo_artifact_stamped(id, &third).unwrap();
    assert_eq!(redone.title, "Second");
    assert_eq!(fourth.revision(), 4);
}

#[test]
fn concurrent_stamped_edit_activates_one_callback_and_returns_current_stamp() {
    let id = "stamp-concurrent";
    let (_, base) = create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "Base")).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let calls = Arc::new(AtomicUsize::new(0));
    let handles: Vec<_> = (0..2).map(|index| {
        let barrier = Arc::clone(&barrier); let calls = Arc::clone(&calls); let stamp = base.clone();
        std::thread::spawn(move || {
            barrier.wait();
            mutate_artifact_stamped(id, &stamp, |a| { calls.fetch_add(1, Ordering::SeqCst); a.title = index.to_string(); Ok(()) })
        })
    }).collect();
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(get_artifact_stamped(id).unwrap(), winner);
    assert_eq!(winner.1.revision(), 2);
}

#[test]
fn initial_admission_rejects_invalid_revision_and_retained_budgets() {
    let mut invalid_redo = StudioArtifact::new("stamp-invalid-redo", "Geometry", "Source");
    invalid_redo.redo_stack.push("unbacked source marker".into());
    assert!(create_artifact_if_absent(invalid_redo).is_err());
    assert!(get_artifact_stamped("stamp-invalid-redo").is_none());
    for (index, revision) in [0, 0x2000_0000].into_iter().enumerate() {
        let id = format!("stamp-invalid-clock-{index}");
        let mut source = StudioArtifact::new(&id, "Geometry", "Source"); source.lamport_clock = revision;
        assert!(create_artifact_if_absent(source).is_err()); assert!(get_artifact_stamped(&id).is_none());
    }
    for audit in [false, true] {
        let id = format!("stamp-invalid-budget-{audit}");
        let mut source = StudioArtifact::new(&id, "Geometry", "Source");
        if audit { source.history.push("x".repeat(journal::MAX_SOURCE_BYTES + 1)); }
        else { source.title = "x".repeat(journal::MAX_SOURCE_BYTES + 1); }
        assert!(create_artifact_if_absent(source).is_err()); assert!(get_artifact_stamped(&id).is_none());
    }
}
