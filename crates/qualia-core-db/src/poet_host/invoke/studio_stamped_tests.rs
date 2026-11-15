use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Barrier,
};

#[test]
fn identical_upsert_invalidates_old_stamp_and_cannot_restore_erased_journal() {
    let id = "stamp-identical-replacement";
    let (_, initial) =
        create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "Original")).unwrap();
    let (edited, old) = mutate_artifact_stamped(id, &initial, |a| {
        a.title = "Edited".into();
        Ok(())
    })
    .unwrap();
    upsert_artifact(edited.clone());
    let (replacement, current) = get_artifact_stamped(id).unwrap();
    assert_eq!(replacement, edited);
    assert_eq!(old.revision(), current.revision());
    assert_ne!(old, current);
    let mut called = false;
    assert!(mutate_artifact_stamped(id, &old, |_| {
        called = true;
        Ok(())
    })
    .is_err());
    assert!(!called);
    assert!(undo_artifact_stamped(id, &old).is_err());
    assert!(redo_artifact_stamped(id, &old).is_err());
    assert_eq!(
        get_artifact_stamped(id).unwrap(),
        (replacement, current.clone())
    );
    assert!(undo_artifact_stamped(id, &current).is_err());
}

#[test]
fn foreign_stamp_and_create_conflict_leave_existing_source_unchanged() {
    let id = "stamp-create-protected";
    let original = StudioArtifact::new(id, "Geometry", "Accepted");
    let (_, stamp) = create_artifact_if_absent(original.clone()).unwrap();
    let (_, foreign) =
        create_artifact_if_absent(StudioArtifact::new("stamp-other", "Geometry", "Other")).unwrap();
    assert!(create_artifact_if_absent(StudioArtifact::new(id, "Other", "Replacement")).is_err());
    assert!(
        mutate_artifact_stamped(id, &foreign, |_| panic!("foreign callback must not run")).is_err()
    );
    assert!(undo_artifact_stamped(id, &foreign).is_err());
    assert!(redo_artifact_stamped(id, &foreign).is_err());
    assert_eq!(get_artifact_stamped(id).unwrap(), (original, stamp));
}

#[test]
fn fallible_candidate_error_preserves_source_revision_and_both_stacks() {
    let id = "stamp-fallible-edit";
    let (_, first) =
        create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "First")).unwrap();
    let (_, second) = mutate_artifact_stamped(id, &first, |a| {
        a.title = "Second".into();
        Ok(())
    })
    .unwrap();
    let before = get_artifact_stamped(id).unwrap();
    let error = mutate_artifact_stamped(id, &second, |a| {
        a.title = "Rejected".into();
        Err("solver refused".into())
    })
    .unwrap_err();
    assert_eq!(error, "solver refused");
    assert_eq!(get_artifact_stamped(id).unwrap(), before);
    let (undone, third) = undo_artifact_stamped(id, &second).unwrap();
    assert_eq!(undone.title, "First");
    let before = get_artifact_stamped(id).unwrap();
    assert!(mutate_artifact_stamped(id, &third, |a| {
        a.properties.clear();
        Err("still refused".into())
    })
    .is_err());
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
    let handles: Vec<_> = (0..2)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let calls = Arc::clone(&calls);
            let stamp = base.clone();
            std::thread::spawn(move || {
                barrier.wait();
                mutate_artifact_stamped(id, &stamp, |a| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    a.title = index.to_string();
                    Ok(())
                })
            })
        })
        .collect();
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
    invalid_redo
        .redo_stack
        .push("unbacked source marker".into());
    assert!(create_artifact_if_absent(invalid_redo).is_err());
    assert!(get_artifact_stamped("stamp-invalid-redo").is_none());
    for (index, revision) in [0, 0x2000_0000].into_iter().enumerate() {
        let id = format!("stamp-invalid-clock-{index}");
        let mut source = StudioArtifact::new(&id, "Geometry", "Source");
        source.lamport_clock = revision;
        assert!(create_artifact_if_absent(source).is_err());
        assert!(get_artifact_stamped(&id).is_none());
    }
    for audit in [false, true] {
        let id = format!("stamp-invalid-budget-{audit}");
        let mut source = StudioArtifact::new(&id, "Geometry", "Source");
        if audit {
            source
                .history
                .push("x".repeat(journal::MAX_SOURCE_BYTES + 1));
        } else {
            source.title = "x".repeat(journal::MAX_SOURCE_BYTES + 1);
        }
        assert!(create_artifact_if_absent(source).is_err());
        assert!(get_artifact_stamped(&id).is_none());
    }
}

#[test]
fn unrelated_artifact_mutation_does_not_stale_current_stamp_but_same_artifact_is_held() {
    let (_, unrelated) =
        create_artifact_if_absent(StudioArtifact::new("stamp-unrelated-busy", "Geometry", "A"))
            .unwrap();
    let (_, target) =
        create_artifact_if_absent(StudioArtifact::new("stamp-target-stable", "Geometry", "B"))
            .unwrap();
    let (edited, current) = mutate_artifact_stamped("stamp-unrelated-busy", &unrelated, |source| {
        // This callback runs while the actual global store is locked and this
        // artifact's publication mutation is active. An unrelated stamp must
        // remain current rather than failing because a global try_lock is busy.
        assert!(artifact_stamp_is_current("stamp-target-stable", &target));
        assert!(!artifact_stamp_is_current(
            "stamp-unrelated-busy",
            &unrelated
        ));
        source.title = "A revised".into();
        Ok(())
    })
    .unwrap();
    assert_eq!(edited.title, "A revised");
    assert!(current.revision() > unrelated.revision());
    assert!(!artifact_stamp_is_current(
        "stamp-unrelated-busy",
        &unrelated
    ));
    assert!(artifact_stamp_is_current("stamp-unrelated-busy", &current));
    assert!(artifact_stamp_is_current("stamp-target-stable", &target));
}

#[test]
fn replacement_retires_stamp_and_poisoned_store_fails_closed() {
    let id = "stamp-retire-poison";
    let (_, old) = create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "Old")).unwrap();
    upsert_artifact(StudioArtifact::new(id, "Geometry", "Recreated"));
    let (_, current) = get_artifact_stamped(id).unwrap();
    assert!(!artifact_stamp_is_current(id, &old));
    assert!(artifact_stamp_is_current(id, &current));
    assert!(!stamped::stamp_is_current(id, &current, true));
}

#[test]
fn panicking_local_publication_mutation_remains_held_without_poisoning_global_store() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    let state = Arc::new(stamped::PublicationState::new("local-panic", 7));
    let artifact = StudioArtifact::new("local-panic", "Geometry", "Source");
    let stored = StoredArtifact {
        publication: Arc::clone(&state),
        artifact,
        journal: journal::SourceJournal::default(),
    };
    let stamp = StoreStamp::of(&stored);
    let result = catch_unwind(AssertUnwindSafe({
        let state = Arc::clone(&state);
        move || {
            let _mutation = state.begin(7).unwrap();
            panic!("exercise publication unwind guard");
        }
    }));
    assert!(result.is_err());
    assert!(!state.matches("local-panic", 7));
    assert!(state.begin(7).is_err());
    assert!(!stamped::stamp_is_current("local-panic", &stamp, false));
}

#[test]
fn dropping_canonical_stored_artifact_retires_retained_stamp() {
    let artifact = StudioArtifact::new("local-drop", "Geometry", "Source");
    let stored = StoredArtifact {
        publication: Arc::new(stamped::PublicationState::new(
            &artifact.id,
            artifact.lamport_clock,
        )),
        artifact,
        journal: journal::SourceJournal::default(),
    };
    let publication = Arc::clone(&stored.publication);
    let stamp = StoreStamp::of(&stored);
    assert!(publication.matches("local-drop", stamp.revision()));
    drop(stored);
    assert!(!publication.matches("local-drop", stamp.revision()));
}

#[test]
fn invalid_upsert_candidate_does_not_retire_accepted_source() {
    let id = "stamp-invalid-replacement-preserves-old";
    let (accepted, stamp) =
        create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "Accepted")).unwrap();
    let mut invalid = StudioArtifact::new(id, "Geometry", "Invalid");
    invalid.lamport_clock = 0;
    assert!(try_upsert_artifact(invalid).is_err());
    assert!(artifact_stamp_is_current(id, &stamp));
    assert_eq!(get_artifact_stamped(id).unwrap(), (accepted, stamp));
}

#[test]
fn legacy_infallible_upsert_keeps_its_unvalidated_compatibility_behavior() {
    let id = "stamp-legacy-infallible-upsert";
    let mut legacy = StudioArtifact::new(id, "Geometry", "Legacy");
    legacy.lamport_clock = 0;
    upsert_artifact(legacy.clone());
    assert_eq!(get_artifact(id), Some(legacy));
}
