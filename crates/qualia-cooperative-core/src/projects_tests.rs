use super::*;

fn contribution(
    id: &str,
    project: &str,
    contributor: &str,
    minutes: u32,
    at: u32,
) -> Contribution {
    Contribution {
        id: id.into(),
        project_id: project.into(),
        contributor_did: contributor.into(),
        description: format!("work-{id}"),
        effort_minutes: minutes,
        capital_cents: 0,
        roi_multiplier: 1.0,
        privacy_level: ContributionPrivacy::default(),
        occurred_at_unix: at,
        occurred_at_instant: None,
        predecessor_id: None,
    }
}

#[test]
fn project_envelope_is_restricted_and_kind_correct() {
    let p = Project::new("Community Garden", "Shared beds", vec![], 1_700_000_000);
    let env = build_project_envelope(&p, "did:wf:coop", "did:wf:steward", 10, None);
    assert!(env.id.contains(":project:"));
    assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    assert_eq!(env.valid_time_start_unix, Some(1_700_000_000));
}

#[test]
fn membership_envelope_kind_and_class() {
    let m = ProjectMembership::new("proj-1", "did:wf:alice", ProjectRole::Steward, 42);
    let env = build_membership_envelope(&m, "did:wf:coop", "did:wf:coop", 50, None);
    assert!(env.id.contains(":project_membership:"));
    assert_eq!(env.sensitivity, SensitivityClass::Restricted);
}

#[test]
fn contribution_envelope_kind_and_chain() {
    let first = Contribution::new(
        "proj-1",
        "did:wf:bob",
        "dig beds",
        60,
        0,
        1.0,
        ContributionPrivacy::default(),
        100,
    );
    let second = Contribution::new(
        "proj-1",
        "did:wf:bob",
        "plant seeds",
        30,
        0,
        1.0,
        ContributionPrivacy::default(),
        200,
    )
    .following(&first);
    let env = build_contribution_envelope(&second, "did:wf:coop", "did:wf:bob", 250, None);
    assert!(env.id.contains(":contribution:"));
    assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    assert_eq!(env.predecessor_id, Some(contribution_record_id(&first.id)));
    assert!(env.predecessor_id.unwrap().contains(":contribution:"));
}

#[test]
fn merge_dedupes_by_id_add_wins() {
    let a = vec![
        contribution("c1", "p1", "did:wf:a", 60, 1),
        contribution("c2", "p1", "did:wf:a", 30, 2),
    ];
    let dup = vec![contribution("c1", "p1", "did:wf:a", 60, 1)];
    let merged = merge_contributions(&a, &dup);
    assert_eq!(
        merged.len(),
        2,
        "duplicate id must not create a second entry"
    );
}

#[test]
fn obligations_invariant_under_duplication_and_reorder() {
    let base = vec![
        contribution("c1", "p1", "did:wf:a", 60, 3),
        contribution("c2", "p1", "did:wf:a", 30, 1),
        contribution("c3", "p1", "did:wf:b", 45, 2),
        contribution("c4", "p2", "did:wf:a", 15, 4),
    ];
    let incoming = vec![
        contribution("c3", "p1", "did:wf:b", 45, 2),
        contribution("c1", "p1", "did:wf:a", 60, 3),
        contribution("c4", "p2", "did:wf:a", 15, 4),
        contribution("c2", "p1", "did:wf:a", 30, 1),
        contribution("c2", "p1", "did:wf:a", 30, 1),
    ];
    let merged = merge_contributions(&base, &incoming);
    let obligations = derive_obligations(&merged);

    let p1a = obligations
        .iter()
        .find(|o| o.project_id == "p1" && o.contributor_did == "did:wf:a")
        .unwrap();
    assert_eq!(p1a.total_effort_minutes, 90);
    assert_eq!(p1a.contribution_count, 2);

    let p1b = obligations
        .iter()
        .find(|o| o.project_id == "p1" && o.contributor_did == "did:wf:b")
        .unwrap();
    assert_eq!(p1b.total_effort_minutes, 45);

    let p2a = obligations
        .iter()
        .find(|o| o.project_id == "p2" && o.contributor_did == "did:wf:a")
        .unwrap();
    assert_eq!(p2a.total_effort_minutes, 15);

    let obligations2 = derive_obligations(&merge_contributions(&incoming, &base));
    assert_eq!(obligations, obligations2);
}

#[test]
fn replaying_incoming_twice_does_not_double_effort() {
    let base = vec![contribution("c1", "p1", "did:wf:a", 120, 1)];
    let incoming = vec![contribution("c2", "p1", "did:wf:a", 60, 2)];
    let once = derive_obligations(&merge_contributions(&base, &incoming));
    let twice = derive_obligations(&merge_contributions(
        &merge_contributions(&base, &incoming),
        &incoming,
    ));
    assert_eq!(once, twice);
    assert_eq!(once[0].total_effort_minutes, 180);
}

#[test]
fn derive_obligations_dedupes_raw_unmerged_input() {
    let raw = vec![
        contribution("c1", "p1", "did:wf:a", 60, 1),
        contribution("c1", "p1", "did:wf:a", 60, 1),
        contribution("c1", "p1", "did:wf:a", 60, 1),
    ];
    let obligations = derive_obligations(&raw);
    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].total_effort_minutes, 60);
    assert_eq!(obligations[0].contribution_count, 1);
}

#[test]
fn summaries_include_primary_fields_and_round_trip_losslessly() {
    let p = Project::new("Repair Cafe", "Fix things together", vec![], 1_700_000_000);
    let ps = project_summary(&p);
    assert!(ps.contains("Repair Cafe"));
    assert!(ps.contains("Fix things together"));

    let m = ProjectMembership::new("proj-1", "did:wf:carol", ProjectRole::Observer, 99);
    let ms = membership_summary(&m);
    assert!(ms.contains("did:wf:carol"));
    assert!(ms.contains("observer"));

    let pred = Contribution::new(
        "p1",
        "did:wf:dave",
        "initial setup",
        45,
        1_000,
        1.25,
        ContributionPrivacy::Permissive,
        4,
    );
    let mut c = Contribution::new(
        "p1",
        "did:wf:dave",
        "follow-up work",
        90,
        25_000,
        1.5,
        ContributionPrivacy::Private,
        5,
    )
    .following(&pred);
    c.occurred_at_instant = Some(InstantBridge::unix(5, 750_000_000));
    let cs = contribution_summary(&c);
    let round_tripped = parse_contribution_summary(&c.id, &cs, 0).unwrap();
    assert_eq!(round_tripped, c);
}
