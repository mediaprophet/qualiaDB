//! Projects tab — cooperative projects, vault state, members, group chat, share packages.

#![allow(non_snake_case)]

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;

use super::helpers::*;
use super::types::*;

/// All signals needed by the Projects tab.
pub struct ProjectsSignals {
    pub status: Signal<String>,
    pub project_name: Signal<String>,
    pub active_project: Signal<String>,
    pub active_project_id: Signal<String>,
    pub project_list: Signal<Vec<(String, String)>>,
    pub last_project_json: Signal<String>,
    pub vault_lifecycle: Signal<crate::components::wellfair::host_dto::VaultLifecycle>,
    pub collab_list: Signal<Vec<serde_json::Value>>,
    pub collab_did: Signal<String>,
    pub collab_name: Signal<String>,
    pub collab_role: Signal<String>,
    pub coop_package_text: Signal<String>,
    pub contacts: Signal<Vec<serde_json::Value>>,
    pub peers: Signal<Vec<serde_json::Value>>,
    pub tab: Signal<HubTab>,
}

pub fn render_projects(sig: ProjectsSignals) -> Element {
    let ProjectsSignals {
        mut status,
        mut project_name,
        mut active_project,
        mut active_project_id,
        project_list,
        last_project_json,
        vault_lifecycle,
        collab_list,
        mut collab_did,
        mut collab_name,
        mut collab_role,
        coop_package_text,
        contacts,
        peers,
        tab,
    } = sig;

    let vault = vault_lifecycle();
    let vault_label = vault_state_label(vault);
    let vault_detail = vault_state_detail(vault);
    let vault_attention = vault_needs_attention(vault);

    rsx! {
        div { style: "{PANEL}",
            div { style: "{CARD}",
                h2 { style: "{H2}", "How cooperative work works here" }
                p { style: "{MUTED}",
                    "1. Create or seed a project (works even if the vault is locked — local-first).\n2. Copy full join package → send to people or their bots.\n3. They paste under People → Accept package.\n4. Admit members / Start mesh / group chat.\n5. Optional: unlock Sanctuary for vault-backed Wellfair journal records."
                }
            }

            // Vault state
            div {
                style: if vault_attention {
                    "background:#1c1917;border:1px solid #f59e0b;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;"
                } else {
                    "background:#052e1c;border:1px solid #10b981;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;"
                },
                h2 { style: "margin:0 0 0.35rem;font-size:1.05rem;color:#fde68a;font-weight:700;",
                    "Vault (optional depth): {vault_label}"
                }
                p { style: "margin:0 0 0.75rem;color:#cbd5e1;font-size:0.88rem;line-height:1.5;",
                    "{vault_detail} Local coop projects and join packages work without unlocking. Unlock for journal/ledger depth."
                }
                if vault_attention {
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;align-items:center;",
                        Link {
                            to: crate::Route::SanctuaryRoute {},
                            style: "display:inline-block;background:#f59e0b;color:#1c1917;padding:10px 16px;border-radius:8px;font-weight:700;font-size:13px;text-decoration:none;",
                            "Open Sanctuary (unlock / set up vault)"
                        }
                        Link {
                            to: crate::Route::WellfairRoute {},
                            style: "display:inline-block;background:#334155;color:#e5e7eb;padding:10px 14px;border-radius:8px;font-weight:600;font-size:13px;text-decoration:none;",
                            "Open Wellfair"
                        }
                    }
                }
            }

            // ── Project list ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Cooperative projects" }
                p { style: "{MUTED}",
                    "Shared work with people and their agents: roster, join package, group chat, #project: tags. QualiaDB Development Cooperative can be seeded in one click."
                }
                if !active_project().is_empty() {
                    p { style: "color:#a7f3d0;font-size:13px;margin:0 0 4px;",
                        "● Active: {active_project}"
                    }
                    if !active_project_id().is_empty() {
                        p { style: "color:#64748b;font-size:11px;font-family:monospace;margin:0 0 8px;word-break:break-all;",
                            "board id: {active_project_id}"
                        }
                    }
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (mut project_list, mut status, mut vault_lifecycle) =
                                (project_list, status, vault_lifecycle);
                            spawn(async move {
                                let snap = crate::components::wellfair::host_client::fetch_host_snapshot()
                                    .await;
                                vault_lifecycle.set(snap.vault);
                                match list_project_records().await {
                                    Ok(plist) => {
                                        status.set(format!(
                                            "{} project(s). Vault: {}.",
                                            plist.len(),
                                            vault_state_label(snap.vault)
                                        ));
                                        project_list.set(plist);
                                    }
                                    Err(e) => status.set(vault_hint(&e)),
                                }
                            });
                        }
                    },
                    "Refresh list"
                }
                if project_list().is_empty() {
                    p { style: "{MUTED}", "No projects yet — create one or seed QualiaDB Development Cooperative." }
                }
                for (pid, plabel) in project_list() {
                    {
                        let pid_c = pid.clone();
                        let plabel_c = plabel.clone();
                        let is_on = active_project_id() == pid;
                        rsx! {
                            button {
                                style: if is_on {
                                    "display:block;width:100%;text-align:left;margin-bottom:6px;padding:10px 12px;border-radius:8px;border:1px solid #8b5cf6;background:rgba(139,92,246,0.12);color:#e9d5ff;cursor:pointer;font-size:13px;"
                                } else {
                                    "display:block;width:100%;text-align:left;margin-bottom:6px;padding:10px 12px;border-radius:8px;border:1px solid #1f2937;background:#0b1220;color:#e5e7eb;cursor:pointer;font-size:13px;"
                                },
                                onclick: move |_| {
                                    active_project_id.set(pid_c.clone());
                                    active_project.set(plabel_c.clone());
                                    store_active_project(&pid_c, &plabel_c);
                                    status.set(format!("Scoped to {plabel_c}. Work board will pick up this id."));
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let pid = pid_c.clone();
                                        let mut collab_list = collab_list;
                                        spawn(async move {
                                            if let Ok(v) = invoke_json::<serde_json::Value>(
                                                "list_project_collaborators",
                                                json!({ "projectId": pid }),
                                            )
                                            .await
                                            {
                                                collab_list.set(json_list(v, &["collaborators", "items"]));
                                            }
                                        });
                                    }
                                },
                                strong { "{plabel}" }
                                span { style: "display:block;font-size:10px;color:#64748b;font-family:monospace;margin-top:3px;", "{pid}" }
                            }
                        }
                    }
                }
            }

            // ── Create project ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Create a project" }
                input {
                    style: "{INPUT}",
                    placeholder: "Project name",
                    value: "{project_name}",
                    oninput: move |e| project_name.set(e.value()),
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (project_name, mut active_project, mut active_project_id, mut project_list, mut last_project_json, mut status, mut vault_lifecycle) =
                                (project_name, active_project, active_project_id, project_list, last_project_json, status, vault_lifecycle);
                            spawn(async move {
                                let name = project_name().trim().to_string();
                                if name.is_empty() {
                                    status.set("Name the project.".into());
                                    return;
                                }
                                match create_project_record(&name, "", vec![]).await {
                                    Ok((board_id, label, obj)) => {
                                        active_project.set(label.clone());
                                        active_project_id.set(board_id.clone());
                                        store_active_project(&board_id, &label);
                                        last_project_json.set(obj.to_string());
                                        if let Ok(plist) = list_project_records().await {
                                            project_list.set(plist);
                                        }
                                        status.set(format!("Project '{label}' ready · board id stored for Work."));
                                    }
                                    Err(e) => {
                                        let snap = crate::components::wellfair::host_client::fetch_host_snapshot()
                                            .await;
                                        vault_lifecycle.set(snap.vault);
                                        status.set(vault_hint(&e));
                                    }
                                }
                            });
                        }
                    },
                    "Create project"
                }
                if status().to_lowercase().contains("unlock")
                    || status().to_lowercase().contains("vault")
                    || vault_needs_attention(vault_lifecycle())
                {
                    p { style: "margin:8px 0 0;display:flex;flex-wrap:wrap;gap:12px;",
                        Link {
                            to: crate::Route::SanctuaryRoute {},
                            style: "color:#fde68a;font-size:13px;font-weight:600;",
                            "Open Sanctuary to unlock vault →"
                        }
                        Link {
                            to: crate::Route::WellfairRoute {},
                            style: "color:#93c5fd;font-size:13px;",
                            "Open Wellfair →"
                        }
                    }
                }
            }

            // ── Seed QualiaDB Development Cooperative ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "QualiaDB Development Cooperative" }
                p { style: "{MUTED}",
                    "One click seeds the project intended to host this system's development among connected peers — backlog, review, releases — using the same cooperative stack."
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (mut active_project, mut active_project_id, mut project_name, mut project_list, mut last_project_json, mut status, mut vault_lifecycle) =
                                (active_project, active_project_id, project_name, project_list, last_project_json, status, vault_lifecycle);
                            spawn(async move {
                                let name = "QualiaDB Development Cooperative";
                                let description = "Self-hosted cooperative workspace for QualiaDB / Webizen development: tasks, reviews, releases, and contributor evidence among front-door peers. Not a cloud SaaS — local-first project records.";
                                match create_project_record(name, description, vec!["rights", "agency"]).await {
                                    Ok((board_id, label, obj)) => {
                                        active_project.set(label.clone());
                                        active_project_id.set(board_id.clone());
                                        project_name.set(label.clone());
                                        store_active_project(&board_id, &label);
                                        last_project_json.set(obj.to_string());
                                        if let Ok(plist) = list_project_records().await {
                                            project_list.set(plist);
                                        }
                                        status.set("QualiaDB Development Cooperative ready. People → invite · Chat → tag #project · Keep → Work board.".into());
                                    }
                                    Err(e) => {
                                        let snap = crate::components::wellfair::host_client::fetch_host_snapshot()
                                            .await;
                                        vault_lifecycle.set(snap.vault);
                                        status.set(vault_hint(&e));
                                    }
                                }
                            });
                        }
                    },
                    "Seed QualiaDB Development Cooperative"
                }
            }

            // ── Project members ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Project members (people & agents)" }
                p { style: "{MUTED}",
                    "Admit contacts or peers to the active project so cooperative work has a roster. Roles: contributor, steward, observer, or agent (their bot)."
                }
                if active_project_id().is_empty() {
                    p { style: "{MUTED}", "Select or create a project first." }
                } else {
                    p { style: "color:#a7f3d0;font-size:12px;margin:0 0 8px;",
                        "Active: {active_project} · {active_project_id}"
                    }
                    input {
                        style: "{INPUT}",
                        placeholder: "Member DID",
                        value: "{collab_did}",
                        oninput: move |e| collab_did.set(e.value()),
                    }
                    input {
                        style: "{INPUT}",
                        placeholder: "Display name (optional)",
                        value: "{collab_name}",
                        oninput: move |e| collab_name.set(e.value()),
                    }
                    select {
                        style: "{INPUT}",
                        value: "{collab_role}",
                        onchange: move |e| collab_role.set(e.value()),
                        option { value: "contributor", "Contributor" }
                        option { value: "steward", "Steward" }
                        option { value: "observer", "Observer" }
                        option { value: "agent", "Agent / bot" }
                    }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            if collab_did().trim().is_empty() {
                                if let Some(c) = contacts().first() {
                                    collab_did.set(s(c, "did"));
                                    let n = s(c, "display_name");
                                    if !n.is_empty() {
                                        collab_name.set(n);
                                    }
                                } else if let Some(p) = peers().first() {
                                    collab_did.set(s(p, "did"));
                                    let n = s(p, "display_name");
                                    if !n.is_empty() {
                                        collab_name.set(n);
                                    }
                                }
                            }
                        },
                        "Fill from first contact/peer"
                    }
                    button {
                        style: "{BTN}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (
                                    collab_did,
                                    collab_name,
                                    collab_role,
                                    active_project_id,
                                    active_project,
                                    mut collab_list,
                                    mut status,
                                ) = (
                                    collab_did,
                                    collab_name,
                                    collab_role,
                                    active_project_id,
                                    active_project,
                                    collab_list,
                                    status,
                                );
                                spawn(async move {
                                    let pid = active_project_id();
                                    let did = collab_did().trim().to_string();
                                    if pid.is_empty() || did.is_empty() {
                                        status.set("Need active project and member DID.".into());
                                        return;
                                    }
                                    let role = collab_role();
                                    match invoke_json::<serde_json::Value>(
                                        "add_project_collaborator",
                                        json!({
                                            "projectId": pid.clone(),
                                            "projectName": active_project(),
                                            "memberDid": did.clone(),
                                            "displayName": collab_name(),
                                            "role": role.clone(),
                                        }),
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            let wf_role = if role == "agent" {
                                                "contributor".to_string()
                                            } else {
                                                role.clone()
                                            };
                                            let _ = invoke_json::<String>(
                                                "wellfair_add_project_membership",
                                                json!({
                                                    "projectId": pid.clone(),
                                                    "memberDid": did,
                                                    "role": wf_role,
                                                }),
                                            )
                                            .await;
                                            if let Ok(v) = invoke_json::<serde_json::Value>(
                                                "list_project_collaborators",
                                                json!({ "projectId": pid }),
                                            )
                                            .await
                                            {
                                                collab_list.set(json_list(v, &["collaborators", "items"]));
                                            }
                                            status.set(
                                                "Member admitted to project roster (and vault membership if unlocked)."
                                                    .into(),
                                            );
                                        }
                                        Err(e) => status.set(format!("Admit failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Admit to project"
                    }
                    if collab_list().is_empty() {
                        p { style: "{MUTED}", "No members on this project yet." }
                    }
                    for m in collab_list() {
                        {
                            let did = s(&m, "member_did");
                            let name = {
                                let n = s(&m, "display_name");
                                if n.is_empty() { did.clone() } else { n }
                            };
                            let role = s(&m, "role");
                            rsx! {
                                div {
                                    style: "padding:8px 10px;background:#0b1220;border-radius:8px;margin-bottom:6px;font-size:12px;",
                                    div { style: "font-weight:600;", "{name}" }
                                    div { style: "color:#94a3b8;font-size:11px;", "role: {role}" }
                                    div { style: "font-family:monospace;color:#64748b;font-size:10px;word-break:break-all;", "{did}" }
                                }
                            }
                        }
                    }
                }
            }

            // ── Project group chat & share package ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Project group chat & share package" }
                p { style: "{MUTED}",
                    "Spin a multi-party chat from the project roster, and copy a coop share package (no private keys) so others know how to join."
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (active_project_id, active_project, mut status, mut tab) =
                                (active_project_id, active_project, status, tab);
                            spawn(async move {
                                let pid = active_project_id();
                                if pid.is_empty() {
                                    status.set("Select a project first.".into());
                                    return;
                                }
                                match invoke_json::<serde_json::Value>(
                                    "create_project_group_chat",
                                    json!({
                                        "projectId": pid,
                                        "projectName": active_project(),
                                        "extraDids": serde_json::Value::Null,
                                    }),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        let sid = s(&v, "session_id");
                                        let title = s(&v, "title");
                                        if let Some(win) = web_sys::window() {
                                            if let Ok(Some(storage)) = win.session_storage() {
                                                if !sid.is_empty() {
                                                    let _ = storage.set_item("webizen_open_session_id", &sid);
                                                }
                                                if !title.is_empty() {
                                                    let _ = storage.set_item("webizen_chat_peer_title", &title);
                                                }
                                                let tok = active_project().replace(' ', "_");
                                                if !tok.is_empty() {
                                                    let _ = storage.set_item(
                                                        "webizen_talk_draft",
                                                        &format!("#project:{tok} "),
                                                    );
                                                }
                                            }
                                        }
                                        tab.set(HubTab::Chat);
                                        status.set(format!(
                                            "Group chat ready ({title}). Opened Chat — pick the conversation if needed."
                                        ));
                                    }
                                    Err(e) => status.set(format!("Group chat failed: {e}")),
                                }
                            });
                        }
                    },
                    "Create project group chat"
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (active_project_id, active_project, mut coop_package_text, mut status) =
                                (active_project_id, active_project, coop_package_text, status);
                            spawn(async move {
                                match invoke_json::<serde_json::Value>(
                                    "coop_share_package",
                                    json!({
                                        "projectId": active_project_id(),
                                        "projectName": active_project(),
                                    }),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        let text = serde_json::to_string_pretty(&v)
                                            .unwrap_or_else(|_| v.to_string());
                                        coop_package_text.set(text.clone());
                                        copy_to_clipboard(
                                            &text,
                                            status,
                                            "Join package copied — send that one blob to your collaborator (or their bot). They paste it under Talk → People → Accept package / invite.",
                                        );
                                    }
                                    Err(e) => status.set(format!("Join package failed: {e}")),
                                }
                            });
                        }
                    },
                    "Copy full join package (one paste for them)"
                }
                if !coop_package_text().is_empty() {
                    div { style: "{CODE}", "{coop_package_text}" }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| copy_to_clipboard(&coop_package_text(), status, "Join package copied again."),
                        "Copy again"
                    }
                }
            }

            // ── Engage others ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Engage others (and their bots)" }
                p { style: "{MUTED}",
                    "One path: Copy full join package (above) with a project selected → send to them → they paste under People → Accept package. That connects, scopes the project, and starts a group chat when possible. Mesh still needs Start mesh for live peer traffic."
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (active_project_id, active_project, mut coop_package_text, mut status, mut tab) =
                                (active_project_id, active_project, coop_package_text, status, tab);
                            spawn(async move {
                                match invoke_json::<serde_json::Value>(
                                    "coop_share_package",
                                    json!({
                                        "projectId": active_project_id(),
                                        "projectName": active_project(),
                                    }),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        let text = serde_json::to_string_pretty(&v)
                                            .unwrap_or_else(|_| v.to_string());
                                        coop_package_text.set(text.clone());
                                        copy_to_clipboard(
                                            &text,
                                            status,
                                            "Join package on clipboard. Send it as one message; they paste under Talk → People → Accept package / invite.",
                                        );
                                        tab.set(HubTab::People);
                                    }
                                    Err(e) => status.set(format!(
                                        "Join package failed: {e}. Set a display name under People first."
                                    )),
                                }
                            });
                        }
                    },
                    "Invite collaborator (copy join package)"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        let mut t = tab;
                        t.set(HubTab::People);
                        status.set(
                            "People: Generate invite or magic link. Use relation Agent/bot for their local agent."
                                .into(),
                        );
                    },
                    "Go to People"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        let mut t = tab;
                        t.set(HubTab::Chat);
                    },
                    "Go to Chat"
                }
                Link {
                    to: crate::Route::WorkRoute {},
                    style: "display:inline-block;{BTN2} text-decoration:none;",
                    "Open Work board"
                }
                Link {
                    to: crate::Route::WellfairRoute {},
                    style: "display:inline-block;{BTN2} text-decoration:none;",
                    "Open Wellfair"
                }
                if !active_project().is_empty() {
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            let tok = active_project().replace(' ', "_");
                            #[cfg(target_arch = "wasm32")]
                            {
                                if let Some(win) = web_sys::window() {
                                    if let Ok(Some(storage)) = win.session_storage() {
                                        let _ = storage.set_item(
                                            "webizen_talk_draft",
                                            &format!("#project:{tok} "),
                                        );
                                    }
                                }
                            }
                            let mut t = tab;
                            t.set(HubTab::Chat);
                            let mut status = status;
                            status.set(format!("Draft tagged #project:{tok} — open a chat and send."));
                        },
                        "Tag next chat message"
                    }
                }
                if !last_project_json().is_empty() {
                    p { style: "margin:12px 0 4px;font-size:11px;color:#64748b;", "Last project record (for work board id)" }
                    div { style: "{CODE}", "{last_project_json}" }
                }
            }
        }
    }
}
