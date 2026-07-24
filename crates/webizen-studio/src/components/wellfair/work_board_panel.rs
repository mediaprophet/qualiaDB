//! Cooperative work board — tasks/issues/milestones on a replay-safe Kanban (plan §11.2 "Work").
//!
//! The current status of each card is derived by the host (latest status event), never mutated
//! in place — so moving a card appends an immutable transition.

use super::host_client::{
    add_work_item, add_work_item_status, fetch_work_item_board, BoardColumnDto,
};
#[cfg(target_arch = "wasm32")]
use super::host_client::{ingest_document, view_select_uri, IngestFacets};
use crate::Route;
use dioxus::prelude::*;

const STATUSES: &[(&str, &str)] = &[
    ("proposed", "Proposed"),
    ("todo", "To do"),
    ("in_progress", "In progress"),
    ("blocked", "Blocked"),
    ("in_review", "In review"),
    ("done", "Done"),
    ("cancelled", "Cancelled"),
];

#[derive(Clone, Debug)]
struct BoardUi {
    status: String,
    project_id: String,
    new_title: String,
    new_type: String,
    columns: Vec<BoardColumnDto>,
}

impl Default for BoardUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            project_id: String::new(),
            new_title: String::new(),
            new_type: "task".into(),
            columns: Vec::new(),
        }
    }
}

#[component]
pub fn WellfairWorkBoardPanel() -> Element {
    let mut ui = use_signal(|| {
        #[allow(unused_mut)]
        let mut b = BoardUi::default();
        // Prefer project id handed off from Relations → Projects (sessionStorage).
        #[cfg(target_arch = "wasm32")]
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.session_storage() {
                if let Ok(Some(id)) = storage.get_item("webizen_active_project_id") {
                    if !id.trim().is_empty() {
                        b.project_id = id;
                    }
                }
            }
        }
        b
    });

    let load = move || {
        spawn(async move {
            let project = ui().project_id.trim().to_string();
            if project.is_empty() {
                let mut w = ui.write();
                w.columns = Vec::new();
                w.status = String::new();
                return;
            }
            match fetch_work_item_board(&project).await {
                Ok(cols) => {
                    let mut w = ui.write();
                    w.columns = cols;
                    w.status = String::new();
                }
                Err(e) => ui.write().status = format!("Board unavailable: {e}"),
            }
        });
    };

    // Auto-fetch board once when we already have a project id from Talk.
    let mut auto_loaded = use_signal(|| false);
    use_effect(move || {
        if auto_loaded() {
            return;
        }
        if ui().project_id.trim().is_empty() {
            return;
        }
        auto_loaded.set(true);
        load();
    });

    let has_cards = ui().columns.iter().any(|c| !c.cards.is_empty());
    let status_text = ui().status.clone();
    let project_empty = ui().project_id.trim().is_empty();
    let empty_hint = if project_empty {
        "Select a project (Relations → Projects), then add work items."
    } else {
        "No work items for this project yet. Add one above, or refresh after seeding."
    };

    rsx! {
        section {
            aria_label: "WellFair cooperative work board",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            super::shared::DomainChrome { domain: "Practice", chip: "Labour · work board", show_memory: true }
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Work board" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Tasks, issues, and milestones. Card status is derived from immutable transitions — moving a card records a new event, never rewriting history. Project id is filled from Relations → Projects when you select or create a project."
            }
            if !status_text.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{status_text}" }
            }

            if project_empty {
                div {
                    style: "margin:0 0 0.75rem;padding:0.65rem 0.75rem;border:1px solid var(--qualia-accent,#2a6f97);border-radius:8px;background:var(--qualia-surface-2,#f0f7fb);font-size:0.8rem;",
                    p { style: "margin:0 0 0.4rem;",
                        "No project selected. Choose one under Relations → Projects so the board id is filled automatically."
                    }
                    Link {
                        to: Route::TalkRoute {},
                        style: "color:var(--qualia-accent,#2a6f97);font-weight:600;text-decoration:none;",
                        "Open Relations → Projects"
                    }
                }
            } else {
                div {
                    style: "margin:0 0 0.75rem;display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;",
                    button {
                        r#type: "button",
                        style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #6d28d9;background:rgba(139,92,246,0.15);color:#e9d5ff;font-size:0.78rem;font-weight:700;cursor:pointer;",
                        title: "Ingest a board snapshot into Lived Memory Work lane",
                        onclick: move |_| {
                            let project = ui().project_id.trim().to_string();
                            if project.is_empty() {
                                ui.write().status = "Enter a project id first.".into();
                                return;
                            }
                            spawn(async move {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let n_cards: usize =
                                        ui().columns.iter().map(|c| c.cards.len()).sum();
                                    ui.write().status = "Saving board note to Lived Memory…".into();
                                    let uri = format!("webizen:memory/work/board/{project}");
                                    let text = format!(
                                        "# Work board · {project}\n\n\
                                         Practice → Lived Memory snapshot.\n\n\
                                         - **Project / board id:** `{project}`\n\
                                         - **Cards on board:** {n_cards}\n\
                                         - **Lane:** Work\n\n\
                                         Open **Memory** to spatialize or continue from session selection.\n"
                                    );
                                    let facets = IngestFacets {
                                        project: Some(project.clone()),
                                        purpose: Some("work-board".into()),
                                        section: Some("work".into()),
                                        sensitivity: Some("restricted".into()),
                                        ..Default::default()
                                    };
                                    match ingest_document(
                                        &uri,
                                        "text/markdown",
                                        &text,
                                        None,
                                        &facets,
                                        "restricted",
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            let _ = view_select_uri(&uri).await;
                                            ui.write().status = "Saved to Lived Memory · Work lane · open Memory to spatialize.".into();
                                        }
                                        Err(e) => {
                                            ui.write().status = format!(
                                                "Could not save to Memory (vault locked or host unavailable): {e}"
                                            );
                                        }
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    ui.write().status =
                                        "Remember in Lived Memory requires the desktop host.".into();
                                }
                            });
                        },
                        "Remember in Lived Memory"
                    }
                    Link {
                        to: Route::LibraryRoute {},
                        style: "font-size:0.76rem;font-weight:700;padding:0.35rem 0.65rem;border-radius:999px;border:1px solid #6d28d9;background:rgba(139,92,246,0.12);color:#e9d5ff;text-decoration:none;",
                        "→ Memory"
                    }
                }
            }

            div {
                style: "display:flex;gap:0.5rem;align-items:flex-end;margin-bottom:0.5rem;",
                label {
                    style: "flex:1;display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Project id"
                    input {
                        r#type: "text",
                        placeholder: "auto from Relations → Projects, or paste uuid",
                        value: "{ui().project_id}",
                        oninput: move |e| ui.write().project_id = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                button {
                    style: "padding:0.4rem 0.7rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                    onclick: move |_| load(),
                    "Refresh board"
                }
            }

            div {
                style: "display:grid;grid-template-columns:2fr 1fr auto;gap:0.5rem;margin-bottom:0.75rem;",
                input {
                    r#type: "text",
                    placeholder: "New work item title",
                    value: "{ui().new_title}",
                    oninput: move |e| ui.write().new_title = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                select {
                    value: "{ui().new_type}",
                    onchange: move |e| ui.write().new_type = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    option { value: "task", "Task" }
                    option { value: "issue", "Issue" }
                    option { value: "milestone", "Milestone" }
                }
                button {
                    style: "padding:0.4rem 0.7rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let project = ui().project_id.trim().to_string();
                        let title = ui().new_title.trim().to_string();
                        if project.is_empty() || title.is_empty() {
                            ui.write().status = "Enter a project id and a title.".into();
                            return;
                        }
                        let item_type = ui().new_type.clone();
                        spawn(async move {
                            ui.write().status = "Adding work item…".into();
                            match add_work_item(&project, &item_type, &title).await {
                                Ok(_) => {
                                    ui.write().status = String::new();
                                    ui.write().new_title = String::new();
                                    load();
                                }
                                Err(e) => ui.write().status = format!("Failed: {e}"),
                            }
                        });
                    },
                    "Add"
                }
            }

            if !has_cards {
                p {
                    style: "margin:0;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "{empty_hint}"
                }
            } else {
                div {
                    style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(160px,1fr));gap:0.6rem;",
                    for col in ui().columns.clone() {
                        if !col.cards.is_empty() {
                            div {
                                key: "{col.status}",
                                style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.5rem;background:var(--qualia-surface,#fff);",
                                h4 {
                                    style: "margin:0 0 0.4rem;font-size:0.76rem;text-transform:uppercase;letter-spacing:0.04em;color:var(--qualia-text-muted,#666);",
                                    "{col.status} ({col.cards.len()})"
                                }
                                for card in col.cards.clone() {
                                    div {
                                        key: "{card.work_item_id}",
                                        style: "border:1px solid var(--qualia-border,#eee);border-radius:6px;padding:0.4rem;margin-bottom:0.4rem;font-size:0.74rem;",
                                        div { style: "font-weight:600;margin-bottom:0.25rem;", "{card.title}" }
                                        div { style: "color:var(--qualia-text-muted,#888);margin-bottom:0.3rem;", "{card.item_type} · {card.priority}" }
                                        select {
                                            value: "{card.status}",
                                            onchange: {
                                                let id = card.work_item_id.clone();
                                                move |e| {
                                                    let id = id.clone();
                                                    let next = e.value();
                                                    spawn(async move {
                                                        match add_work_item_status(&id, &next).await {
                                                            Ok(_) => {
                                                                ui.write().status = String::new();
                                                                load();
                                                            }
                                                            Err(err) => ui.write().status = format!("Failed: {err}"),
                                                        }
                                                    });
                                                }
                                            },
                                            style: "width:100%;padding:0.25rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.72rem;",
                                            for (val, label) in STATUSES.iter() {
                                                option { value: "{val}", "{label}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
