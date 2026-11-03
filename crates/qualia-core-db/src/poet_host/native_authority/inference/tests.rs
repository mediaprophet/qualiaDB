use super::super::*;
use super::*;
use qualia_cooperative_core::{
    authoring::*,
    projects::{Project, ProjectMembership, ProjectRole},
};
#[path = "../session_test_fixture.rs"]
mod fixture;

fn selection<'a>(id: &'a IdentifierAssociationEvidence) -> IdentifierSelection<'a> {
    IdentifierSelection {
        value: &id.value,
        scheme: &id.scheme,
        issuer: &id.issuer,
        project_id: &id.project_id,
        evidence_ref: &id.evidence_ref,
    }
}

#[test]
fn all_three_capacities_keep_subject_and_multiple_identifiers_separate() {
    for kind in [
        PrincipalKind::Human,
        PrincipalKind::SoftwareAgent,
        PrincipalKind::Organization,
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut snapshot = fixture::snapshot();
        snapshot.subjects[0].kind = kind;
        let owner = fixture::open(temp.path(), snapshot.clone());
        let session = fixture::login(&owner, "author", &fixture::AUTHOR);
        let primary = owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[0]))
            .unwrap();
        let alternate = owner
            .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
            .unwrap();
        assert_eq!(primary.evidence().subject, alternate.evidence().subject);
        assert_eq!(primary.evidence().kind, kind);
        assert_ne!(
            primary.evidence().selected_identifier_digest,
            alternate.evidence().selected_identifier_digest
        );
        assert_eq!(
            primary.evidence().controller,
            alternate.evidence().controller
        );
        assert_eq!(
            owner.with_current_inference_association(&alternate, |e| e.kind),
            Ok(kind)
        );
        // The original authoring route still accepts its canonical read permit.
        assert!(owner.publication_is_current(&session, fixture::generation(&owner, &session)));
    }
}

#[test]
fn selection_requires_full_current_signed_association_and_exact_owner() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = fixture::snapshot();
    let owner = fixture::open(temp.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    let mut selected = selection(&snapshot.subjects[0].identifiers[1]);
    selected.issuer = "unadmitted-issuer";
    assert!(matches!(
        owner.bind_inference_association(&session, selected),
        Err(InferenceAssociationError::Scope)
    ));
    let association = owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
        .unwrap();
    let other_dir = tempfile::tempdir().unwrap();
    let other = fixture::open(other_dir.path(), snapshot);
    assert_eq!(
        other.with_current_inference_association(&association, |_| ()),
        Err(InferenceAssociationError::Stale)
    );
    owner.disconnect(&session).unwrap();
    let called = std::cell::Cell::new(false);
    assert!(owner
        .with_current_inference_association(&association, |_| called.set(true))
        .is_err());
    assert!(!called.get());
}

#[test]
fn rotation_invalidates_old_binding_without_rewriting_subject_or_aliases() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot = fixture::snapshot();
    let owner = fixture::open(temp.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    let old = owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
        .unwrap();
    let mut successor = snapshot.clone();
    successor.sequence += 1;
    let rotated = [39; 32];
    successor.subjects[0].controller_public_key = ed25519::public_from_seed(&rotated);
    owner
        .replace_admission(fixture::sign(successor.clone()))
        .unwrap();
    assert_eq!(
        owner.with_current_inference_association(&old, |_| ()),
        Err(InferenceAssociationError::Stale)
    );
    let new_session = fixture::login(&owner, "author", &rotated);
    let current = owner
        .bind_inference_association(
            &new_session,
            selection(&successor.subjects[0].identifiers[1]),
        )
        .unwrap();
    assert_eq!(old.evidence().subject, current.evidence().subject);
    assert_eq!(
        old.evidence().selected_identifier_digest,
        current.evidence().selected_identifier_digest
    );
    assert_ne!(old.evidence().controller, current.evidence().controller);
    assert_ne!(
        old.evidence().admission_digest,
        current.evidence().admission_digest
    );
}

#[test]
fn expired_alternate_does_not_erase_other_current_identifiers() {
    let temp = tempfile::tempdir().unwrap();
    let mut snapshot = fixture::snapshot();
    snapshot.subjects[0].identifiers[1].valid_until_unix = now().unwrap();
    let owner = fixture::open(temp.path(), snapshot.clone());
    let session = fixture::login(&owner, "author", &fixture::AUTHOR);
    assert!(owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[1]))
        .is_err());
    assert!(owner
        .bind_inference_association(&session, selection(&snapshot.subjects[0].identifiers[0]))
        .is_ok());
}
