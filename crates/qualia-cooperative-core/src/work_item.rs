//! Cooperative work items — tasks, issues, and milestones — with a replay-safe Kanban board.
//!
//! A `WorkItem` is created once (immutable core). Status transitions are separate immutable
//! `WorkItemStatusEvent` records; the *current* status is a **derived projection** — the latest
//! event per work item — never a mutated field. Deduping events by stable id before deriving
//! makes the board invariant under duplicated, reordered, or replayed sync frames (plan §8.3/§8.4).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wellfare_core::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

/// The kind of work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    Task,
    Issue,
    Milestone,
}

impl Default for WorkItemType {
    fn default() -> Self {
        Self::Task
    }
}

/// Kanban lifecycle status. `ORDER` gives the canonical column ordering for board rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    Proposed,
    Todo,
    InProgress,
    Blocked,
    InReview,
    Done,
    Cancelled,
}

impl Default for WorkItemStatus {
    fn default() -> Self {
        Self::Todo
    }
}

impl WorkItemStatus {
    /// Canonical Kanban column order.
    pub const ORDER: [WorkItemStatus; 7] = [
        WorkItemStatus::Proposed,
        WorkItemStatus::Todo,
        WorkItemStatus::InProgress,
        WorkItemStatus::Blocked,
        WorkItemStatus::InReview,
        WorkItemStatus::Done,
        WorkItemStatus::Cancelled,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Default for WorkItemPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// The immutable core of a work item. Its status is derived from status events, not stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub item_type: WorkItemType,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub priority: WorkItemPriority,
    /// Effort estimate in whole minutes (integer discipline, plan §8.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee_did: Option<String>,
    pub created_at_unix: u32,
}

impl WorkItem {
    pub fn new(
        project_id: impl Into<String>,
        item_type: WorkItemType,
        title: impl Into<String>,
        created_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.into(),
            item_type,
            title: title.into(),
            description: String::new(),
            priority: WorkItemPriority::Normal,
            estimate_minutes: None,
            assignee_did: None,
            created_at_unix,
        }
    }
}

/// An immutable status transition for a work item. The board derives the current status as the
/// latest event per work item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItemStatusEvent {
    pub id: String,
    pub work_item_id: String,
    pub status: WorkItemStatus,
    pub occurred_at_unix: u32,
}

impl WorkItemStatusEvent {
    pub fn new(
        work_item_id: impl Into<String>,
        status: WorkItemStatus,
        occurred_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            work_item_id: work_item_id.into(),
            status,
            occurred_at_unix,
        }
    }
}

// ---------------------------------------------------------------------------
// Record ids + envelopes
// ---------------------------------------------------------------------------

pub fn work_item_record_id(uuid: &str) -> String {
    format!("urn:qualia:work_item:{uuid}")
}

pub fn work_item_status_record_id(uuid: &str) -> String {
    format!("urn:qualia:work_item_status:{uuid}")
}

fn cooperative_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    valid_start_unix: u32,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(valid_start_unix),
        valid_time_start_instant: None,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: None,
        tombstone: false,
    }
}

pub fn build_work_item_envelope(
    item: &WorkItem,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    cooperative_envelope(
        &work_item_record_id(&item.id),
        owner_did,
        author_did,
        asserted_unix,
        item.created_at_unix,
    )
}

pub fn build_work_item_status_envelope(
    event: &WorkItemStatusEvent,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    cooperative_envelope(
        &work_item_status_record_id(&event.id),
        owner_did,
        author_did,
        asserted_unix,
        event.occurred_at_unix,
    )
}

pub fn work_item_summary(item: &WorkItem) -> String {
    serde_json::json!({
        "id": item.id,
        "project_id": item.project_id,
        "item_type": item.item_type,
        "title": item.title,
        "priority": item.priority,
        "estimate_minutes": item.estimate_minutes,
        "assignee_did": item.assignee_did,
        "created_at_unix": item.created_at_unix,
    })
    .to_string()
}

pub fn work_item_status_summary(event: &WorkItemStatusEvent) -> String {
    serde_json::json!({
        "id": event.id,
        "work_item_id": event.work_item_id,
        "status": event.status,
        "occurred_at_unix": event.occurred_at_unix,
    })
    .to_string()
}

/// Reconstruct a `WorkItem` from a stored/transmitted summary JSON (fields not in the summary
/// take their defaults). Returns `None` on malformed input.
pub fn parse_work_item_summary(summary: &str) -> Option<WorkItem> {
    serde_json::from_str(summary).ok()
}

pub fn parse_work_item_status_summary(summary: &str) -> Option<WorkItemStatusEvent> {
    serde_json::from_str(summary).ok()
}

// ---------------------------------------------------------------------------
// Replay-safe derivations
// ---------------------------------------------------------------------------

/// Merge status-event sets add-wins by stable id (never re-apply), deterministically ordered
/// by (time, id). Idempotent and order-independent — the basis for replay-safe board derivation.
pub fn merge_status_events(
    existing: &[WorkItemStatusEvent],
    incoming: &[WorkItemStatusEvent],
) -> Vec<WorkItemStatusEvent> {
    let mut merged: Vec<WorkItemStatusEvent> = Vec::with_capacity(existing.len() + incoming.len());
    for ev in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.id == ev.id) {
            merged.push(ev.clone());
        }
    }
    merged.sort_by(|a, b| {
        a.occurred_at_unix
            .cmp(&b.occurred_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

/// The current status of a work item = the latest event (by time, then id) for it, or `Todo`
/// if it has no status events yet. Pure over the unique-event-id set (duplicates collapse first).
pub fn current_status(work_item_id: &str, events: &[WorkItemStatusEvent]) -> WorkItemStatus {
    merge_status_events(events, &[])
        .into_iter()
        .filter(|e| e.work_item_id == work_item_id)
        .last()
        .map(|e| e.status)
        .unwrap_or(WorkItemStatus::Todo)
}

/// A card as rendered on the board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardCard {
    pub work_item_id: String,
    pub title: String,
    pub item_type: WorkItemType,
    pub priority: WorkItemPriority,
    pub status: WorkItemStatus,
}

/// A Kanban column: a status and the cards currently in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardColumn {
    pub status: WorkItemStatus,
    pub cards: Vec<BoardCard>,
}

/// Derive the Kanban board for a set of work items and their status events. Columns are in
/// canonical order; each item's column is its current (latest-event) status. Invariant under
/// duplicated/reordered events because the current status is a pure derivation over unique ids.
pub fn derive_board(items: &[WorkItem], events: &[WorkItemStatusEvent]) -> Vec<BoardColumn> {
    let merged = merge_status_events(events, &[]);
    let mut columns: Vec<BoardColumn> = WorkItemStatus::ORDER
        .iter()
        .map(|status| BoardColumn {
            status: *status,
            cards: Vec::new(),
        })
        .collect();
    // Stable card order: by item creation time then id, so the board is deterministic.
    let mut ordered = items.to_vec();
    ordered.sort_by(|a, b| {
        a.created_at_unix
            .cmp(&b.created_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    for item in &ordered {
        let status = current_status(&item.id, &merged);
        if let Some(col) = columns.iter_mut().find(|c| c.status == status) {
            col.cards.push(BoardCard {
                work_item_id: item.id.clone(),
                title: item.title.clone(),
                item_type: item.item_type,
                priority: item.priority,
                status,
            });
        }
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, project: &str, at: u32) -> WorkItem {
        WorkItem {
            id: id.into(),
            project_id: project.into(),
            item_type: WorkItemType::Task,
            title: format!("item-{id}"),
            description: String::new(),
            priority: WorkItemPriority::Normal,
            estimate_minutes: None,
            assignee_did: None,
            created_at_unix: at,
        }
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

        // Reordered + replayed events yield the same board.
        let shuffled = vec![
            event("e2", "wi-2", WorkItemStatus::Done, 6),
            event("e1", "wi-1", WorkItemStatus::InProgress, 5),
            event("e2", "wi-2", WorkItemStatus::Done, 6), // duplicate
        ];
        assert_eq!(derive_board(&items, &shuffled), board);

        // wi-1 is InProgress, wi-2 is Done.
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
    fn summaries_round_trip() {
        let wi = WorkItem::new("proj-1", WorkItemType::Milestone, "v1.0", 1_700_000_000);
        let parsed = parse_work_item_summary(&work_item_summary(&wi)).unwrap();
        assert_eq!(parsed.id, wi.id);
        assert_eq!(parsed.item_type, WorkItemType::Milestone);
        assert_eq!(parsed.title, "v1.0");

        let ev = WorkItemStatusEvent::new(&wi.id, WorkItemStatus::Blocked, 42);
        let pev = parse_work_item_status_summary(&work_item_status_summary(&ev)).unwrap();
        assert_eq!(pev.work_item_id, wi.id);
        assert_eq!(pev.status, WorkItemStatus::Blocked);
    }
}
