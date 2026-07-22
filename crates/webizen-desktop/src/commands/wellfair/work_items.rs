#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager};

fn parse_work_item_type(s: &str) -> qualia_cooperative_core::work_item::WorkItemType {
    use qualia_cooperative_core::work_item::WorkItemType::*;
    match s.to_ascii_lowercase().as_str() {
        "issue" => Issue,
        "milestone" => Milestone,
        _ => Task,
    }
}

fn parse_work_item_status(s: &str) -> qualia_cooperative_core::work_item::WorkItemStatus {
    use qualia_cooperative_core::work_item::WorkItemStatus::*;
    match s.to_ascii_lowercase().as_str() {
        "proposed" => Proposed,
        "in_progress" => InProgress,
        "blocked" => Blocked,
        "in_review" => InReview,
        "done" => Done,
        "cancelled" => Cancelled,
        _ => Todo,
    }
}

#[command]
pub fn wellfair_add_work_item(
    app: AppHandle,
    project_id: String,
    item_type: String,
    title: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let item = qualia_cooperative_core::work_item::WorkItem::new(
            project_id,
            parse_work_item_type(&item_type),
            title,
            wellfair_now_unix(),
        );
        let entry = host.add_work_item(&item)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_add_work_item_status(
    app: AppHandle,
    work_item_id: String,
    status: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_mut()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let event = qualia_cooperative_core::work_item::WorkItemStatusEvent::new(
            work_item_id,
            parse_work_item_status(&status),
            wellfair_now_unix(),
        );
        let entry = host.add_work_item_status(&event)?;
        serde_json::to_string(&entry).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_work_item_board(
    app: AppHandle,
    project_id: String,
    limit: usize,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized â€” unlock vault first".to_string())?;
        let board = host.work_item_board(&project_id, limit)?;
        serde_json::to_string(&board).map_err(|e| e.to_string())
    })?
}

// --- Agency layer: supported-agency delegations (ADR Â§7â€“Â§10) ---------------------------------
