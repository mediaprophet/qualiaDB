//! **Talk hub** — the human path for social + cooperative work (people, their agents/bots, projects).
//!
//! Tabs: **Chat** · **People** · **Reception** · **Mail** · **Projects**.
//! First-run follows `talk_setup_status` (domain → mailboxes/receiver → people → chat/projects).
//! Cooperative help depends on People + Projects working: invite peers (and agent/service relations),
//! open chat, seed a project, tag messages, share work board id.

#![allow(non_snake_case)]

pub mod types;
pub mod helpers;
pub mod people;
pub mod reception;
pub mod projects;

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;

use types::*;
#[allow(unused_imports)]
use helpers::*;
use people::{PeopleSignals, render_people};
use reception::{ReceptionSignals, render_reception};
use projects::{ProjectsSignals, render_projects};

/// Primary Talk surface: Chat · People · Reception · Projects.
#[component]
pub fn SocialHub() -> Element {
    let tab = use_signal(|| {
        // Omnibox / deep-link: webizen_talk_tab = chat|people|reception|projects
        #[cfg(target_arch = "wasm32")]
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.session_storage() {
                if let Ok(Some(t)) = storage.get_item("webizen_talk_tab") {
                    let _ = storage.remove_item("webizen_talk_tab");
                    return match t.as_str() {
                        "people" => HubTab::People,
                        "reception" => HubTab::Reception,
                        "mail" | "email" | "inbox" => HubTab::Mail,
                        "projects" => HubTab::Projects,
                        _ => HubTab::Chat,
                    };
                }
            }
        }
        HubTab::Chat
    });
    let status = use_signal(String::new);

    // Shared people state
    let contacts = use_signal(Vec::<serde_json::Value>::new);
    let invite_code = use_signal(String::new);
    let invite_out = use_signal(String::new);
    let invite_mailto = use_signal(String::new);
    let invite_in = use_signal(String::new);
    let display_name = use_signal(String::new);
    let group_title = use_signal(String::new);
    let group_dids = use_signal(String::new);
    let magic_link = use_signal(String::new);
    let relation = use_signal(|| "peer".to_string());

    // Reception state
    let domain_name = use_signal(String::new);
    let domain_label = use_signal(String::new);
    let domains = use_signal(Vec::<serde_json::Value>::new);
    let front_doors = use_signal(Vec::<serde_json::Value>::new);
    let dns_name = use_signal(String::new);
    let dns_txt = use_signal(String::new);
    let turtle = use_signal(String::new);

    // Projects state
    let project_name = use_signal(String::new);
    let active_project = use_signal(String::new);
    let active_project_id = use_signal(String::new);
    let project_list = use_signal(Vec::<(String, String)>::new);
    let last_project_json = use_signal(String::new);
    let peers = use_signal(Vec::<serde_json::Value>::new);
    let magic_accept = use_signal(String::new);
    let active_model_chip = use_signal(String::new);
    let vault_lifecycle = use_signal(|| {
        crate::components::wellfair::host_dto::VaultLifecycle::Unconfigured
    });
    let setup_banner = use_signal(String::new);
    let mesh_status_text = use_signal(String::new);
    let collab_list = use_signal(Vec::<serde_json::Value>::new);
    let collab_did = use_signal(String::new);
    let collab_name = use_signal(String::new);
    let collab_role = use_signal(|| "contributor".to_string());
    let peer_endpoint_edit = use_signal(String::new);
    let peer_endpoint_did = use_signal(String::new);
    let coop_package_text = use_signal(String::new);

    // Boot: profile, contacts + social peers, model chip, vault, projects, first-run route.
    #[cfg(target_arch = "wasm32")]
    {
        let mut display_name = display_name;
        let mut contacts = contacts;
        let mut peers = peers;
        let mut status = status;
        let mut active_model_chip = active_model_chip;
        let mut project_list = project_list;
        let mut active_project = active_project;
        let mut active_project_id = active_project_id;
        let mut vault_lifecycle = vault_lifecycle;
        let mut setup_banner = setup_banner;
        let mut tab = tab;
        use_effect(move || {
            spawn(async move {
                if let Ok(prof) = invoke_json::<serde_json::Value>("get_user_profile", json!({})).await {
                    let n = s(&prof, "display_name");
                    if !n.is_empty() {
                        display_name.set(n);
                    }
                }
                if let Ok(Some(m)) =
                    invoke_json::<Option<String>>("get_active_model", json!({})).await
                {
                    if !m.is_empty() {
                        active_model_chip.set(m);
                    }
                }
                // If domain mail exists but receiver is down, start it (finish the product path).
                if let Ok(st0) =
                    invoke_json::<serde_json::Value>("talk_setup_status", json!({})).await
                {
                    let has_mb = st0
                        .get("has_mailboxes")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    let recv = st0
                        .get("receiver_running")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    if has_mb && !recv {
                        let _ = invoke_json::<serde_json::Value>(
                            "mail_receiver_start",
                            json!({ "bind": serde_json::Value::Null }),
                        )
                        .await;
                    }
                }
                // First-run / readiness — route to the next human step (not ops chrome).
                if let Ok(st) =
                    invoke_json::<serde_json::Value>("talk_setup_status", json!({})).await
                {
                    let next = s(&st, "next_step");
                    let domains_n = st.get("domains").and_then(|x| x.as_u64()).unwrap_or(0);
                    let mailboxes = st.get("mailboxes").and_then(|x| x.as_u64()).unwrap_or(0);
                    let recv = st
                        .get("receiver_running")
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    let people = st.get("has_people").and_then(|x| x.as_bool()).unwrap_or(false);
                    let banner = format!(
                        "Setup: {domains_n} domain(s) · {mailboxes} mailbox(es) · receiver {} · people {}.",
                        if recv { "on" } else { "off" },
                        if people { "yes" } else { "none yet" }
                    );
                    setup_banner.set(banner);
                    // Only auto-route when the default Chat tab would leave a beginner stranded.
                    match next.as_str() {
                        "reception" => {
                            tab.set(HubTab::Reception);
                            status.set(
                                "First: register a domain under Reception (identity → domain → DNS). That is how others and their bots find you."
                                    .into(),
                            );
                        }
                        "mail_onboard" | "mail_receiver" => {
                            tab.set(HubTab::Mail);
                            status.set(
                                "Domain is set — open Mail, onboard mailboxes if needed, start the local receiver so mail lands here."
                                    .into(),
                            );
                        }
                        "people" => {
                            tab.set(HubTab::People);
                            status.set(
                                "Domain ready. Invite people (or agent/service peers) under People — that is how cooperative help starts."
                                    .into(),
                            );
                        }
                        _ => {}
                    }
                }
                // People data: contacts + mesh peers (covers deep-link open on People).
                let (contacts_res, peers_res) = load_people_lists().await;
                let n_contacts = contacts_res.as_ref().map(|l| l.len()).unwrap_or(0);
                let n_peers = peers_res.as_ref().map(|l| l.len()).unwrap_or(0);
                if let Ok(list) = contacts_res {
                    contacts.set(list);
                }
                if let Ok(list) = peers_res {
                    peers.set(list);
                }
                // Vault lifecycle for Projects tab (best-effort host snapshot).
                {
                    let snap =
                        crate::components::wellfair::host_client::fetch_host_snapshot().await;
                    vault_lifecycle.set(snap.vault);
                }
                // Restore session scope first so auto-pick does not clobber a user choice.
                if let Some(win) = web_sys::window() {
                    if let Ok(Some(storage)) = win.session_storage() {
                        if let Ok(Some(id)) = storage.get_item("webizen_active_project_id") {
                            if !id.is_empty() {
                                active_project_id.set(id);
                            }
                        }
                        if let Ok(Some(name)) = storage.get_item("webizen_active_project_name") {
                            if !name.is_empty() {
                                active_project.set(name);
                            }
                        }
                    }
                }
                let project_note = match list_project_records().await {
                    Ok(plist) => {
                        let n = plist.len();
                        if active_project_id().is_empty() {
                            if let Some((id, label)) = plist.first() {
                                active_project_id.set(id.clone());
                                active_project.set(label.clone());
                                store_active_project(id, label);
                            }
                        }
                        project_list.set(plist);
                        if n > 0 {
                            format!(" · {n} project(s)")
                        } else {
                            String::new()
                        }
                    }
                    Err(e) => {
                        // Keep vault banner accurate; only append vault-ish notes to boot status.
                        let lower = e.to_lowercase();
                        if lower.contains("unlock")
                            || lower.contains("vault")
                            || lower.contains("host api not initialized")
                        {
                            " · vault locked for projects".to_string()
                        } else {
                            String::new()
                        }
                    }
                };
                let model_bit = if active_model_chip().is_empty() {
                    "no model"
                } else {
                    "model on"
                };
                status.set(format!(
                    "Talk ready · {n_contacts} contact(s) · {n_peers} peer(s) · {model_bit}{project_note}. Private by default."
                ));
            });
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &contacts, &invite_code, &invite_out, &invite_mailto, &invite_in, &display_name,
            &group_title, &group_dids, &magic_link, &relation, &domain_name, &domain_label,
            &domains, &front_doors, &dns_name, &dns_txt, &turtle, &project_name, &active_project,
            &active_project_id, &project_list, &last_project_json, &peers, &magic_accept,
            &active_model_chip, &vault_lifecycle, &status, &tab, &setup_banner,
            &mesh_status_text, &collab_list, &collab_did, &collab_name, &collab_role,
            &peer_endpoint_edit, &peer_endpoint_did, &coop_package_text,
        );
    }

    let tab_btn = move |id: HubTab, label: &'static str| {
        let on = tab() == id;
        rsx! {
            button {
                r#type: "button",
                style: if on { TAB_ON } else { TAB },
                onclick: move |_| {
                    let mut t = tab;
                    t.set(id);
                    #[cfg(target_arch = "wasm32")]
                    if id == HubTab::People {
                        let (contacts, peers, status) = (contacts, peers, status);
                        spawn(async move {
                            let (c, p) = load_people_lists().await;
                            apply_people_lists(c, p, contacts, peers, status, "People · ");
                        });
                    }
                    #[cfg(target_arch = "wasm32")]
                    if id == HubTab::Reception {
                        let (mut domains, mut front_doors, mut status) =
                            (domains, front_doors, status);
                        spawn(async move {
                            if let Ok(v) = invoke_json::<serde_json::Value>("list_mail_domains", json!({})).await {
                                if let Some(arr) = v.as_array() {
                                    domains.set(arr.clone());
                                } else if let Some(arr) = v.get("domains").and_then(|d| d.as_array()) {
                                    domains.set(arr.clone());
                                }
                            }
                            if let Ok(list) =
                                invoke_json::<Vec<serde_json::Value>>("get_front_doors", json!({})).await
                            {
                                front_doors.set(list);
                            }
                            status.set(
                                "Reception: create identity → register domain → copy DNS. Private vault stays private."
                                    .into(),
                            );
                        });
                    }
                    #[cfg(target_arch = "wasm32")]
                    if id == HubTab::Projects {
                        let (mut project_list, mut status, mut vault_lifecycle, mut collab_list, active_project_id) =
                            (project_list, status, vault_lifecycle, collab_list, active_project_id);
                        spawn(async move {
                            let snap =
                                crate::components::wellfair::host_client::fetch_host_snapshot()
                                    .await;
                            vault_lifecycle.set(snap.vault);
                            match list_project_records().await {
                                Ok(plist) => {
                                    let n = plist.len();
                                    project_list.set(plist);
                                    status.set(format!(
                                        "{n} project(s). Vault: {} (optional). Select one, invite via join package.",
                                        vault_state_label(snap.vault)
                                    ));
                                }
                                Err(e) => status.set(vault_hint(&e)),
                            }
                            let pid = active_project_id();
                            if !pid.is_empty() {
                                if let Ok(v) = invoke_json::<serde_json::Value>(
                                    "list_project_collaborators",
                                    json!({ "projectId": pid }),
                                )
                                .await
                                {
                                    collab_list.set(json_list(v, &["collaborators", "items"]));
                                }
                            }
                        });
                    }
                    #[cfg(target_arch = "wasm32")]
                    if id == HubTab::Chat {
                        let mut active_model_chip = active_model_chip;
                        spawn(async move {
                            if let Ok(Some(m)) =
                                invoke_json::<Option<String>>("get_active_model", json!({})).await
                            {
                                active_model_chip.set(m);
                            }
                        });
                    }
                },
                "{label}"
            }
        }
    };

    rsx! {
        div { style: "{ROOT}",
            div {
                style: "padding:12px 16px 8px;flex-shrink:0;display:flex;align-items:flex-start;justify-content:space-between;gap:12px;",
                div {
                    h1 { style: "margin:0;font-size:1.2rem;color:#e9d5ff;", "Talk" }
                    p { style: "margin:4px 0 0;color:#64748b;font-size:12px;line-height:1.4;max-width:36rem;",
                        "Chat · people · domain · mail · cooperative projects — engage people and their bots without a SaaS middleman."
                    }
                }
                if active_model_chip().is_empty() {
                    span {
                        style: "font-size:11px;color:#fde68a;background:#78350f;padding:5px 11px;border-radius:999px;white-space:nowrap;flex-shrink:0;",
                        "○ No model"
                    }
                } else {
                    span {
                        style: "font-size:11px;color:#a7f3d0;background:#064e3b;padding:5px 11px;border-radius:999px;max-width:14rem;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex-shrink:0;",
                        title: "{active_model_chip}",
                        "● {active_model_chip}"
                    }
                }
            }
            div { style: "{TABS}",
                {tab_btn(HubTab::Chat, "Chat")}
                {tab_btn(HubTab::People, "People")}
                {tab_btn(HubTab::Reception, "Reception")}
                {tab_btn(HubTab::Mail, "Mail")}
                {tab_btn(HubTab::Projects, "Projects")}
            }
            if !setup_banner().is_empty() {
                div {
                    style: "padding:8px 14px;background:#111827;border-bottom:1px solid #1f2937;color:#94a3b8;font-size:12px;flex-shrink:0;display:flex;flex-wrap:wrap;gap:8px;align-items:center;",
                    span { "{setup_banner}" }
                    button {
                        r#type: "button",
                        style: "{BTN2} margin:0;",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (mut setup_banner, mut tab, mut status) =
                                    (setup_banner, tab, status);
                                spawn(async move {
                                    if let Ok(st) = invoke_json::<serde_json::Value>(
                                        "talk_setup_status",
                                        json!({}),
                                    )
                                    .await
                                    {
                                        let next = s(&st, "next_step");
                                        let domains_n =
                                            st.get("domains").and_then(|x| x.as_u64()).unwrap_or(0);
                                        let mailboxes = st
                                            .get("mailboxes")
                                            .and_then(|x| x.as_u64())
                                            .unwrap_or(0);
                                        let recv = st
                                            .get("receiver_running")
                                            .and_then(|x| x.as_bool())
                                            .unwrap_or(false);
                                        let people = st
                                            .get("has_people")
                                            .and_then(|x| x.as_bool())
                                            .unwrap_or(false);
                                        setup_banner.set(format!(
                                            "Setup: {domains_n} domain(s) · {mailboxes} mailbox(es) · receiver {} · people {}.",
                                            if recv { "on" } else { "off" },
                                            if people { "yes" } else { "none yet" }
                                        ));
                                        match next.as_str() {
                                            "reception" => {
                                                tab.set(HubTab::Reception);
                                                status.set("Next: Reception — register your domain.".into());
                                            }
                                            "mail_onboard" | "mail_receiver" => {
                                                tab.set(HubTab::Mail);
                                                status.set("Next: Mail — onboard + receiver.".into());
                                            }
                                            "people" => {
                                                tab.set(HubTab::People);
                                                status.set(
                                                    "Next: People — invite collaborators / agents.".into(),
                                                );
                                            }
                                            "chat_or_projects" => {
                                                tab.set(HubTab::Projects);
                                                status.set(
                                                    "Ready to collaborate — Projects for shared work, Chat for conversation."
                                                        .into(),
                                                );
                                            }
                                            _ => status.set("Setup refreshed.".into()),
                                        }
                                    }
                                });
                            }
                        },
                        "Refresh setup · go next"
                    }
                }
            }
            if !status().is_empty() {
                div { style: "{STATUS}", "{status}" }
            }

            // ── Chat ──────────────────────────────────────────────────────
            if tab() == HubTab::Chat {
                div { style: "flex:1;min-height:0;overflow:hidden;display:flex;flex-direction:column;",
                    crate::components::connect_chat::ConnectChat {}
                }
            }

            // ── People ────────────────────────────────────────────────────
            if tab() == HubTab::People {
                {render_people(PeopleSignals {
                    status,
                    contacts,
                    peers,
                    invite_code,
                    invite_out,
                    invite_mailto,
                    invite_in,
                    display_name,
                    group_title,
                    group_dids,
                    magic_link,
                    magic_accept,
                    relation,
                    domain_name,
                    peer_endpoint_did,
                    peer_endpoint_edit,
                    mesh_status_text,
                    active_project,
                    active_project_id,
                    collab_list,
                    tab,
                })}
            }

            // ── Reception (domain front door) ─────────────────────────────
            if tab() == HubTab::Reception {
                {render_reception(ReceptionSignals {
                    status,
                    domain_name,
                    domain_label,
                    domains,
                    front_doors,
                    dns_name,
                    dns_txt,
                    turtle,
                })}
            }

            // ── Mail ──────────────────────────────────────────────────────
            if tab() == HubTab::Mail {
                div { style: "flex:1;min-height:0;overflow:hidden;display:flex;flex-direction:column;",
                    div { style: "padding:10px 16px;border-bottom:1px solid #1f2937;background:#0f172a;flex-shrink:0;",
                        p { style: "margin:0;color:#94a3b8;font-size:12px;line-height:1.45;",
                            "Register a domain (Reception) → purpose inboxes mint automatically → start the local SMTP receiver in this pane → paste MX/SPF so the internet can reach you. Mail lands in the local inbox with semantic rules. External SMTP/IMAP is optional import/send only."
                        }
                    }
                    div { style: "flex:1;min-height:0;overflow:hidden;",
                        crate::components::domains_pane::DomainsPane {}
                    }
                }
            }

            // ── Projects (cooperative) ────────────────────────────────────
            if tab() == HubTab::Projects {
                {render_projects(ProjectsSignals {
                    status,
                    project_name,
                    active_project,
                    active_project_id,
                    project_list,
                    last_project_json,
                    vault_lifecycle,
                    collab_list,
                    collab_did,
                    collab_name,
                    collab_role,
                    coop_package_text,
                    contacts,
                    peers,
                    tab,
                })}
            }
        }
    }
}
