//! Cooperative work items — phases, epics, stories, tasks, issues, bugs, and milestones —
//! with WBS hierarchy, PDM dependencies, and a replay-safe Kanban board.
//!
//! A `WorkItem` is created once (immutable core). Status transitions are separate immutable
//! `WorkItemStatusEvent` records; the *current* status is a **derived projection** — the latest
//! event per work item — never a mutated field. Deduping events by stable id before deriving
//! makes the board invariant under duplicated, reordered, or replayed sync frames (plan §8.3/§8.4).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};
pub use crate::work_graph::{
    detect_semantic_conflicts, merge_dependencies, DepKind, Lag, SemanticConflict, TaskConstraint,
    TaskRef, ThreePointEstimate, TimeEntry, WorkDependency,
};

/// The kind of work item across WBS and Agile/Kanban views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    #[default]
    Task,
    Issue,
    Milestone,
    Phase,
    Epic,
    Story,
    Bug,
}

/// Kanban lifecycle status. `ORDER` gives the canonical column ordering for board rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    Proposed,
    #[default]
    Todo,
    InProgress,
    Blocked,
    InReview,
    Done,
    Cancelled,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
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
    /// Optional parent work item ID in the WBS hierarchy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Dotted WBS code (e.g. `"1.2.3"`).
    #[serde(default)]
    pub wbs: String,
    /// WBS outline depth (`0` = root, `1` = phase/summary, etc.).
    #[serde(default)]
    pub outline_level: u16,
    /// Effort estimate in whole minutes (integer discipline, plan §8.4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_minutes: Option<u32>,
    /// Optional three-point PERT duration/effort estimate (`optimistic_m`, `most_likely_m`, `pessimistic_m`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub three_point_estimate: Option<ThreePointEstimate>,
    /// Scheduling constraint (`Asap`, `Alap`, `Snet`, `Snlt`, `Fnet`, `Fnlt`, `Mso`, `Mfo`).
    #[serde(default)]
    pub constraint: TaskConstraint,
    /// Optional Unix timestamp anchor for date-constrained tasks (`Snet`, `Snlt`, `Fnet`, `Fnlt`, `Mso`, `Mfo`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint_date_unix: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee_did: Option<String>,
    pub created_at_unix: u32,
}

impl WorkItem {
    /// Creates a new instance.
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
            parent_id: None,
            wbs: String::new(),
            outline_level: 0,
            estimate_minutes: None,
            three_point_estimate: None,
            constraint: TaskConstraint::Asap,
            constraint_date_unix: None,
            assignee_did: None,
            created_at_unix,
        }
    }

    /// Builder helper to set WBS hierarchy coordinates (`parent_id`, `wbs`, `outline_level`).
    pub fn with_wbs(
        mut self,
        parent_id: Option<String>,
        wbs: impl Into<String>,
        outline_level: u16,
    ) -> Self {
        self.parent_id = parent_id;
        self.wbs = wbs.into();
        self.outline_level = outline_level;
        self
    }

    /// Builder helper to attach a `TaskConstraint` and optional anchor timestamp.
    pub fn with_constraint(
        mut self,
        constraint: TaskConstraint,
        constraint_date_unix: Option<u32>,
    ) -> Self {
        self.constraint = constraint;
        self.constraint_date_unix = constraint_date_unix;
        self
    }

    /// Builder helper to attach a `ThreePointEstimate`.
    pub fn with_three_point_estimate(mut self, estimate: ThreePointEstimate) -> Self {
        self.three_point_estimate = Some(estimate);
        self
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
    /// Creates a new instance.
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

/// `work-item-record` ID.
pub fn work_item_record_id(uuid: &str) -> String {
    format!("urn:qualia:work_item:{uuid}")
}

/// `work-item-status-record` ID.
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

/// Builds work-item-envelope.
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

/// Builds work-item-status-envelope.
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

/// Work-item summary (lossless across WBS and PDM fields).
pub fn work_item_summary(item: &WorkItem) -> String {
    serde_json::json!({
        "id": item.id,
        "project_id": item.project_id,
        "item_type": item.item_type,
        "title": item.title,
        "description": item.description,
        "priority": item.priority,
        "parent_id": item.parent_id,
        "wbs": item.wbs,
        "outline_level": item.outline_level,
        "estimate_minutes": item.estimate_minutes,
        "three_point_estimate": item.three_point_estimate,
        "constraint": item.constraint,
        "constraint_date_unix": item.constraint_date_unix,
        "assignee_did": item.assignee_did,
        "created_at_unix": item.created_at_unix,
    })
    .to_string()
}

/// Work-item status summary.
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

/// Parses work-item-status-summary.
pub fn parse_work_item_status_summary(summary: &str) -> Option<WorkItemStatusEvent> {
    serde_json::from_str(summary).ok()
}

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

/// Derive the Kanban board for a set of work items and their status events.
pub fn derive_board(items: &[WorkItem], events: &[WorkItemStatusEvent]) -> Vec<BoardColumn> {
    let merged = merge_status_events(events, &[]);
    let mut columns: Vec<BoardColumn> = WorkItemStatus::ORDER
        .iter()
        .map(|status| BoardColumn {
            status: *status,
            cards: Vec::new(),
        })
        .collect();
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
#[path = "work_item_tests.rs"]
mod tests;
