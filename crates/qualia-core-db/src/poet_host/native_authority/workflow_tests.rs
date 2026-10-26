//! Software fixtures exercise exact source records; no human review is claimed.
use super::*;

#[test]
fn requested_review_is_revision_bound_distinct_and_durable() {
    let temp = tempfile::tempdir().unwrap();
    let adopted = snapshot();
    let owner = open(temp.path(), adopted.clone());
    let author = login(&owner, "author", &AUTHOR);
    owner
        .record_contribution_at(
            &author,
            &document(),
            "effort/1",
            "Authored source",
            20,
            WorkStatus::Unpaid {
                evidence_ref: "fixture/unpaid".into(),
            },
        )
        .unwrap();
    owner
        .request_source_review(&author, &document(), "request/1", "fixture/request")
        .unwrap();
    assert!(owner
        .record_requested_source_review(
            &author,
            &document(),
            "request/1",
            "review/self",
            ReviewDecision::Accepted,
            "self",
            "fixture/self"
        )
        .is_err());
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    assert_eq!(
        owner.workflow_evidence(&author).unwrap().principal_subject,
        "author"
    );
    let mut stale = document();
    stale.revision += 1;
    assert!(owner
        .record_requested_source_review(
            &reviewer,
            &stale,
            "request/1",
            "review/stale",
            ReviewDecision::Accepted,
            "stale",
            "fixture/stale"
        )
        .is_err());
    owner
        .record_requested_source_review(
            &reviewer,
            &document(),
            "request/1",
            "review/1",
            ReviewDecision::ChangesRequested,
            "Keep editable master",
            "fixture/inspection",
        )
        .unwrap();
    let evidence = owner.workflow_evidence(&reviewer).unwrap();
    assert_eq!(evidence.requests.len(), 1);
    assert_eq!(
        evidence.requests[0].decision_ids,
        vec!["review/1".to_string()]
    );
    assert_eq!(evidence.reviews[0].target, document());
    assert_eq!(
        evidence.reviews[0].principal_kind,
        PrincipalKind::SoftwareAgent
    );
    owner.disconnect(&reviewer).unwrap();
    assert!(owner
        .record_requested_source_review(
            &reviewer,
            &document(),
            "request/1",
            "review/denied",
            ReviewDecision::Accepted,
            "revoked",
            "fixture/revoked"
        )
        .is_err());
    drop(owner);
    let restored = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(adopted),
        document(),
        1024 * 1024,
        256,
    )
    .unwrap();
    assert!(restored.workflow_evidence(&reviewer).is_err());
    let reviewer = login(&restored, "reviewer", &REVIEWER);
    let evidence = restored.workflow_evidence(&reviewer).unwrap();
    assert_eq!(evidence.contributions.len(), 1);
    assert_eq!(evidence.requests.len(), 1);
    assert_eq!(evidence.reviews.len(), 1);
}
