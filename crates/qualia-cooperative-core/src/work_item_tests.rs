use super::*;

fn item(id: &str, project: &str, at: u32) -> WorkItem {
    let mut wi = WorkItem::new(project, WorkItemType::Task, format!("item-{id}"), at);
    wi.id = id.into();
    wi
}

fn event(id: &str, work_item: &str, status: WorkItemStatus, at: u32) -> WorkItemStatusEvent {
    WorkItemStatusEvent {
        id: id.into(),
        work_item_id: work_item.into(),
        status,
        occurred_at_unix: at,
    }
}

#[test]
fn envelope_kinds_and_class() {
    let wi = WorkItem::new("proj-1", WorkItemType::Issue, "Fix the bug", 1_700_000_000);
    let env = build_work_item_envelope(&wi, "did:q42:owner", "did:q42:owner", 10);
    assert!(env.id.contains(":work_item:"));
    assert_eq!(env.sensitivity, SensitivityClass::Restricted);

    let ev = WorkItemStatusEvent::new(&wi.id, WorkItemStatus::InProgress, 1_700_000_100);
    let senv = build_work_item_status_envelope(&ev, "did:q42:owner", "did:q42:owner", 20);
    assert!(senv.id.contains(":work_item_status:"));
}

#[test]
fn current_status_defaults_to_todo_without_events() {
    assert_eq!(current_status("wi-1", &[]), WorkItemStatus::Todo);
}

#[test]
fn current_status_is_latest_event() {
    let events = vec![
        event("e1", "wi-1", WorkItemStatus::Todo, 1),
        event("e2", "wi-1", WorkItemStatus::InProgress, 2),
        event("e3", "wi-1", WorkItemStatus::Done, 3),
    ];
    assert_eq!(current_status("wi-1", &events), WorkItemStatus::Done);
}

#[test]
fn board_is_invariant_under_duplicate_and_reordered_events() {
    let items = vec![item("wi-1", "p1", 1), item("wi-2", "p1", 2)];
    let base = vec![
        event("e1", "wi-1", WorkItemStatus::InProgress, 5),
        event("e2", "wi-2", WorkItemStatus::Done, 6),
    ];
    let board = derive_board(&items, &base);

    let shuffled = vec![
        event("e2", "wi-2", WorkItemStatus::Done, 6),
        event("e1", "wi-1", WorkItemStatus::InProgress, 5),
        event("e2", "wi-2", WorkItemStatus::Done, 6),
    ];
    assert_eq!(derive_board(&items, &shuffled), board);

    let in_progress = board
        .iter()
        .find(|c| c.status == WorkItemStatus::InProgress)
        .unwrap();
    assert_eq!(in_progress.cards.len(), 1);
    assert_eq!(in_progress.cards[0].work_item_id, "wi-1");
    let done = board
        .iter()
        .find(|c| c.status == WorkItemStatus::Done)
        .unwrap();
    assert_eq!(done.cards[0].work_item_id, "wi-2");
}

#[test]
fn board_columns_are_in_canonical_order() {
    let board = derive_board(&[], &[]);
    let statuses: Vec<_> = board.iter().map(|c| c.status).collect();
    assert_eq!(statuses, WorkItemStatus::ORDER.to_vec());
}

#[test]
fn summaries_round_trip_with_wbs_and_pdm_fields() {
    let wi = WorkItem::new("proj-1", WorkItemType::Phase, "Phase 1 Foundation", 1_700_000_000)
        .with_wbs(Some("root-0".into()), "1.1", 2)
        .with_constraint(TaskConstraint::Snet, Some(1_700_000_500))
        .with_three_point_estimate(ThreePointEstimate::new(60, 120, 240));
    let parsed = parse_work_item_summary(&work_item_summary(&wi)).unwrap();
    assert_eq!(parsed, wi);

    let ev = WorkItemStatusEvent::new(&wi.id, WorkItemStatus::Blocked, 42);
    let pev = parse_work_item_status_summary(&work_item_status_summary(&ev)).unwrap();
    assert_eq!(pev.work_item_id, wi.id);
    assert_eq!(pev.status, WorkItemStatus::Blocked);
}

#[test]
fn pdm_dependencies_time_entry_and_semantic_conflicts() {
    let d1 = WorkDependency::new_local("p1", "t1", "t2", DepKind::FS, Lag::WorkingMinutes(60), 10);
    let d2 = WorkDependency::new_local("p1", "t2", "t1", DepKind::SS, Lag::None, 20);
    let merged = merge_dependencies(&[d1.clone()], &[d1.clone(), d2]);
    assert_eq!(merged.len(), 2);

    let mut t1 = item("t1", "p1", 100).with_constraint(TaskConstraint::Mso, None);
    t1.assignee_did = Some("did:wf:alice".into());
    t1.estimate_minutes = Some(300);

    let mut t2 = item("t2", "p1", 100);
    t2.assignee_did = Some("did:wf:alice".into());
    t2.estimate_minutes = Some(240);

    let conflicts = detect_semantic_conflicts(&[t1, t2], &merged, 480);
    assert!(conflicts.iter().any(|c| matches!(c, SemanticConflict::DependencyCycle { .. })));
    assert!(conflicts.iter().any(|c| matches!(c, SemanticConflict::ConstraintViolation { .. })));
    assert!(conflicts.iter().any(|c| matches!(c, SemanticConflict::Overallocation { .. })));

    let te = TimeEntry {
        id: "te-1".into(),
        project_id: "p1".into(),
        work_item_id: "t1".into(),
        contributor_did: "did:wf:alice".into(),
        agreement_id: Some("ag-1".into()),
        minutes: 90,
        reimbursable_expense_cents: 1_500,
        note: "Implemented WBS parser".into(),
        logged_at_unix: 1_700_000_200,
    };
    let act = te.to_activity_record();
    assert_eq!(act.effort_minutes, 90);
    assert_eq!(act.reimbursable_expense_cents, 1_500);
    assert_eq!(act.work_item_id.as_deref(), Some("t1"));
}
