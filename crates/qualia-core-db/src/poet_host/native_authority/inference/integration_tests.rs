use super::super::*;
use super::*;
use qualia_cooperative_core::projects::{Project, ProjectMembership, ProjectRole};
use std::cell::Cell;
#[path = "../session_test_fixture.rs"]
mod fixture;

fn selection(id: &IdentifierAssociationEvidence) -> IdentifierSelection<'_> {
    IdentifierSelection {
        value: &id.value,
        scheme: &id.scheme,
        issuer: &id.issuer,
        project_id: &id.project_id,
        evidence_ref: &id.evidence_ref,
    }
}
fn refused(owner: &NativeAuthority, association: &AuthenticatedInferenceAssociation) {
    let calls = Cell::new(0);
    assert!(owner
        .with_current_inference_association(association, |_| calls.set(calls.get() + 1))
        .is_err());
    assert_eq!(calls.get(), 0);
}

#[test]
fn every_principal_kind_supports_distinct_identifiers_without_changing_subject() {
    for kind in [
        PrincipalKind::Human,
        PrincipalKind::SoftwareAgent,
        PrincipalKind::Organization,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut snapshot = fixture::snapshot();
        snapshot.subjects[0].kind = kind;
        let owner = fixture::open(dir.path(), snapshot.clone());
        let session = fixture::login(&owner, "author", &fixture::AUTHOR);
        let mut observations = Vec::new();
        for id in &snapshot.subjects[0].identifiers {
            let association = owner
                .bind_inference_association(&session, selection(id))
                .unwrap();
            let evidence = owner
                .with_current_inference_association(&association, |v| v)
                .unwrap();
            assert_eq!(evidence, association.evidence());
            assert_eq!(evidence.kind, kind);
            assert_eq!(evidence.subject, crate::q_hash("author"));
            assert_eq!(
                evidence.controller,
                ed25519::public_from_seed(&fixture::AUTHOR)
            );
            observations.push(evidence);
        }
        assert_eq!(observations.len(), 2);
        assert_eq!(
            observations[0].admission_digest,
            observations[1].admission_digest
        );
        assert_ne!(
            observations[0].selected_identifier_digest,
            observations[1].selected_identifier_digest
        );
        assert_ne!(
            observations[0].association_digest,
            observations[1].association_digest
        );
        assert!(owner.publication_is_current(&session, fixture::generation(&owner, &session)));
    }
}

#[test]
fn exact_selector_subject_owner_and_signed_adoption_are_required() {
    let dir = tempfile::tempdir().unwrap();
    let snapshot = fixture::snapshot();
    let owner = fixture::open(dir.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    let id = &snapshot.subjects[0].identifiers[1];
    for field in 0..5 {
        let mut selected = selection(id);
        match field {
            0 => selected.value = "wrong",
            1 => selected.scheme = "wrong",
            2 => selected.issuer = "wrong",
            3 => selected.project_id = "wrong",
            _ => selected.evidence_ref = "wrong",
        }
        assert!(matches!(
            owner.bind_inference_association(&session, selected),
            Err(InferenceAssociationError::Scope)
        ));
    }
    assert!(matches!(
        owner.bind_inference_association(&session, selection(&snapshot.subjects[1].identifiers[0])),
        Err(InferenceAssociationError::Scope)
    ));
    let other_dir = tempfile::tempdir().unwrap();
    let other = fixture::open(other_dir.path(), snapshot.clone());
    assert!(matches!(
        other.bind_inference_association(&session, selection(id)),
        Err(InferenceAssociationError::Authentication)
    ));
    let association = owner
        .bind_inference_association(&session, selection(id))
        .unwrap();
    refused(&other, &association);
    let mut duplicate = snapshot.clone();
    let repeated = duplicate.subjects[0].identifiers[1].clone();
    duplicate.subjects[0].identifiers.push(repeated);
    let duplicate_dir = tempfile::tempdir().unwrap();
    assert!(NativeAuthority::open(
        duplicate_dir.path(),
        fixture::anchor(),
        fixture::sign(duplicate),
        fixture::document(),
        1024 * 1024,
        256
    )
    .is_err());
    let mut forged = fixture::sign(snapshot);
    forged.snapshot.subjects[0].kind = PrincipalKind::Organization;
    let forged_dir = tempfile::tempdir().unwrap();
    assert!(NativeAuthority::open(
        forged_dir.path(),
        fixture::anchor(),
        forged,
        fixture::document(),
        1024 * 1024,
        256
    )
    .is_err());
}

#[test]
fn actual_controller_proof_and_replay_refusal_precede_association() {
    let dir = tempfile::tempdir().unwrap();
    let snapshot = fixture::snapshot();
    let owner = fixture::open(dir.path(), snapshot.clone());
    let challenge = owner.begin_session("author", 300).unwrap();
    let mut wrong = [0; 64];
    ed25519::sign_with_context(
        &fixture::REVIEWER,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut wrong,
    )
    .unwrap();
    assert!(owner.complete_session(challenge.clone(), &wrong).is_err());
    let mut correct = [0; 64];
    ed25519::sign_with_context(
        &fixture::AUTHOR,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut correct,
    )
    .unwrap();
    assert!(owner.complete_session(challenge, &correct).is_err());
    let fresh = fixture::login(&owner, "author", &fixture::AUTHOR);
    let association = owner
        .bind_inference_association(&fresh, selection(&snapshot.subjects[0].identifiers[0]))
        .unwrap();
    assert_eq!(
        owner.with_current_inference_association(&association, |_| 42),
        Ok(42)
    );
}

#[test]
fn signed_rotation_revocation_and_disconnect_invalidate_old_handles() {
    for mode in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = fixture::snapshot();
        let owner = fixture::open(dir.path(), snapshot.clone());
        let session = fixture::login(&owner, "author", &fixture::AUTHOR);
        let association = owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
            .unwrap();
        if mode == 2 {
            owner.disconnect(&session).unwrap();
        } else {
            let mut successor = snapshot.clone();
            successor.sequence += 1;
            if mode == 0 {
                successor.subjects[0].controller_public_key = ed25519::public_from_seed(&[61; 32]);
            } else {
                successor.subjects[0].revoked = true;
            }
            let mut forged = fixture::sign(successor.clone());
            forged.signature[0] ^= 1;
            assert!(owner.replace_admission(forged).is_err());
            assert_eq!(
                owner.with_current_inference_association(&association, |_| 1),
                Ok(1)
            );
            owner
                .replace_admission(fixture::sign(successor.clone()))
                .unwrap();
            if mode == 0 {
                let fresh = fixture::login(&owner, "author", &[61; 32]);
                let current = owner
                    .bind_inference_association(
                        &fresh,
                        selection(&successor.subjects[0].identifiers[1]),
                    )
                    .unwrap();
                assert_eq!(current.evidence().subject, association.evidence().subject);
                assert_eq!(
                    current.evidence().selected_identifier_digest,
                    association.evidence().selected_identifier_digest
                );
                assert_ne!(
                    current.evidence().controller,
                    association.evidence().controller
                );
                assert_ne!(
                    current.evidence().admission_digest,
                    association.evidence().admission_digest
                );
            } else {
                assert!(owner.begin_session("author", 300).is_err());
            }
        }
        refused(&owner, &association);
    }
}

#[test]
fn session_deadline_and_each_live_authority_edge_is_rechecked_not_cached_evidence() {
    for mode in 0..6 {
        let dir = tempfile::tempdir().unwrap();
        let snapshot = fixture::snapshot();
        let owner = fixture::open(dir.path(), snapshot.clone());
        let session = fixture::login(&owner, "author", &fixture::AUTHOR);
        let association = owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
            .unwrap();
        let timestamp = now().unwrap();
        assert!(association.evidence().valid_until > timestamp);
        {
            // Deterministic state-edge injection, not a forged public admission route.
            let mut state = owner.state.lock().unwrap();
            match mode {
                0 => {
                    state
                        .sessions
                        .iter_mut()
                        .flatten()
                        .find(|s| s.handle == session)
                        .unwrap()
                        .expires = timestamp
                }
                1 => {
                    state.durable.admission.snapshot.subjects[0].identifiers[1].valid_until_unix =
                        timestamp
                }
                2 => state.durable.admission.snapshot.subjects[0].valid_until_unix = timestamp,
                3 => state.durable.admission.snapshot.valid_until_unix = timestamp,
                4 => state.durable.admission.snapshot.revoked = true,
                _ => state.reconciliation = true,
            }
        }
        refused(&owner, &association);
    }
}

#[test]
fn expired_or_unknown_identifier_does_not_remove_other_associations() {
    let dir = tempfile::tempdir().unwrap();
    let mut snapshot = fixture::snapshot();
    snapshot.subjects[0].identifiers[1].valid_until_unix = now().unwrap();
    let owner = fixture::open(dir.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    assert!(owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
        .is_err());
    let good = owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[0]))
        .unwrap();
    assert_eq!(
        owner.with_current_inference_association(&good, |_| 7),
        Ok(7)
    );
    assert_eq!(
        owner
            .state
            .lock()
            .unwrap()
            .durable
            .admission
            .snapshot
            .subjects[0]
            .identifiers,
        snapshot.subjects[0].identifiers
    );
}

#[test]
fn current_and_refused_associations_are_zero_alloc_with_positive_control() {
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        assert_zero_alloc, AllocGuard,
    };
    let control = AllocGuard::begin("association allocator positive control", true);
    let allocated = vec![9u8; 1024];
    std::hint::black_box(&allocated);
    assert!(control.check().is_err());
    drop(allocated);
    let dir = tempfile::tempdir().unwrap();
    let snapshot = fixture::snapshot();
    let owner = fixture::open(dir.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    let association = owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[0]))
        .unwrap();
    assert_zero_alloc("association valid and bad selector", || {
        let new = owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
            .unwrap();
        assert_eq!(owner.with_current_inference_association(&new, |_| 9), Ok(9));
        let mut missing = selection(&snapshot.subjects[0].identifiers[0]);
        missing.evidence_ref = "not admitted";
        assert!(owner.bind_inference_association(&session, missing).is_err());
    });
    owner.disconnect(&session).unwrap();
    assert_zero_alloc("association disconnected", || {
        assert!(owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[0]))
            .is_err());
        refused(&owner, &association);
    });
}
