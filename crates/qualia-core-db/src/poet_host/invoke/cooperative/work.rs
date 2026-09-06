//! `CooperativeWork.board_project` — Kanban projection over work items + status events.

use qualia_cooperative_core::work_item::{derive_board, WorkItem, WorkItemStatusEvent};
use vibe::{Span, Value};

use super::super::args;
use super::codec::{decode_field, encode_json};

/// Derive a deterministic Kanban board for a project-scoped item set.
///
/// Args (record):
/// - `items` — list of [`WorkItem`] records
/// - `events` — list of [`WorkItemStatusEvent`] records (may be empty)
///
/// Result: `{ columns: [BoardColumn, ...] }`
pub fn board_project(args_v: &Value, span: Span) -> Result<Value, vibe::Diagnostic> {
    let items: Vec<WorkItem> =
        decode_field(args_v, "items", span, "CooperativeWork.board_project")?;
    let events: Vec<WorkItemStatusEvent> = match args::rec(args_v, "events") {
        Some(_) => decode_field(args_v, "events", span, "CooperativeWork.board_project")?,
        None => Vec::new(),
    };
    let columns = derive_board(&items, &events);
    let columns_v = encode_json(&columns, span, "CooperativeWork.board_project")?;
    Ok(args::record([("columns", columns_v)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_cooperative_core::work_item::{WorkItemPriority, WorkItemStatus, WorkItemType};
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn board_projects_items_into_columns() {
        let item = WorkItem {
            id: "wi-1".into(),
            project_id: "proj".into(),
            item_type: WorkItemType::Task,
            title: "Ship board".into(),
            description: String::new(),
            priority: WorkItemPriority::Normal,
            estimate_minutes: None,
            assignee_did: None,
            created_at_unix: 10,
        };
        let event = WorkItemStatusEvent {
            id: "ev-1".into(),
            work_item_id: "wi-1".into(),
            status: WorkItemStatus::InProgress,
            occurred_at_unix: 20,
        };
        let args_v = rec(&[
            (
                "items",
                encode_json(&[item], Span { start: 0, end: 0 }, "test").unwrap(),
            ),
            (
                "events",
                encode_json(&[event], Span { start: 0, end: 0 }, "test").unwrap(),
            ),
        ]);
        let out = board_project(&args_v, Span { start: 0, end: 0 }).expect("ok");
        let Value::Record(m) = out else {
            panic!("expected record");
        };
        let Some(Value::List(cols)) = m.get("columns") else {
            panic!("expected columns list");
        };
        assert!(!cols.is_empty());
        let in_progress = cols.iter().find(|c| {
            matches!(
                c,
                Value::Record(r)
                    if r.get("status") == Some(&Value::String("in_progress".into()))
            )
        });
        assert!(in_progress.is_some());
    }

    #[test]
    fn missing_items_fails_closed() {
        let err = board_project(&rec(&[]), Span { start: 0, end: 0 }).expect_err("needs items");
        assert!(err.message.contains("items"));
    }
}
