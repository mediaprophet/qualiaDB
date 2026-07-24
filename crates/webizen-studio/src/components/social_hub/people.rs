//! People tab — profile, invites, accept, mesh, peers, contacts, group chat.

#![allow(non_snake_case)]

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;

use super::helpers::*;
use super::types::*;

/// All signals needed by the People tab.
pub struct PeopleSignals {
    pub status: Signal<String>,
    pub contacts: Signal<Vec<serde_json::Value>>,
    pub peers: Signal<Vec<serde_json::Value>>,
    pub invite_code: Signal<String>,
    pub invite_out: Signal<String>,
    pub invite_mailto: Signal<String>,
    pub invite_in: Signal<String>,
    pub display_name: Signal<String>,
    pub group_title: Signal<String>,
    pub group_dids: Signal<String>,
    pub magic_link: Signal<String>,
    pub magic_accept: Signal<String>,
    pub relation: Signal<String>,
    pub domain_name: Signal<String>,
    pub peer_endpoint_did: Signal<String>,
    pub peer_endpoint_edit: Signal<String>,
    pub mesh_status_text: Signal<String>,
    pub active_project: Signal<String>,
    pub active_project_id: Signal<String>,
    pub collab_list: Signal<Vec<serde_json::Value>>,
    pub tab: Signal<HubTab>,
}

pub fn render_people(sig: PeopleSignals) -> Element {
    let PeopleSignals {
        mut status,
        contacts,
        peers,
        invite_code,
        invite_out,
        invite_mailto,
        mut invite_in,
        mut display_name,
        mut group_title,
        mut group_dids,
        magic_link,
        mut magic_accept,
        mut relation,
        mut domain_name,
        mut peer_endpoint_did,
        mut peer_endpoint_edit,
        mesh_status_text,
        mut active_project,
        mut active_project_id,
        mut collab_list,
        mut tab,
    } = sig;

    rsx! {
        div { style: "{PANEL}",
            div {
                style: "margin:0 0 1rem;padding:0.85rem 1rem;border-radius:12px;border:1px solid #334155;background:linear-gradient(135deg,rgba(139,92,246,0.12),rgba(15,23,42,0.9));max-width:720px;",
                p { style: "margin:0 0 0.25rem;font-size:0.62rem;font-weight:800;letter-spacing:0.05em;text-transform:uppercase;color:#a5b4fc;",
                    "People · natural persons & peer bonds"
                }
                p { style: "margin:0;font-size:0.8rem;color:#94a3b8;line-height:1.45;",
                    "Invite humans and optional agent/service relations. Relation type (peer / agent / service) is explicit — instruments stay instruments. After bonds exist, open Chat or Projects; remember material in Lived Memory."
                }
            }
            // ── Profile card ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "You" }
                p { style: "{MUTED}",
                    "Set a display name and turn on invites so other people can connect to you. Invites stay private until you share them."
                }
                input {
                    style: "{INPUT}", placeholder: "Display name", value: "{display_name}",
                    oninput: move |e| display_name.set(e.value()),
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (display_name, mut status) = (display_name, status);
                            spawn(async move {
                                // Partial patch is intentional: backend merges onto the loaded
                                // profile (display_name + enable connect invites). Do not send a
                                // bare SharingPolicy object — that used to fail deserialize with
                                // `missing field share_display_name`.
                                let body = json!({
                                    "display_name": display_name(),
                                    "sharing": {
                                        "allow_group_chat_invites": true,
                                        "share_display_name": true,
                                    }
                                });
                                let body = serde_json::to_string(&body).unwrap_or_default();
                                match invoke_json::<serde_json::Value>("save_user_profile", json!({ "profileJson": body })).await {
                                    Ok(_) => status.set("Profile saved — invites enabled.".into()),
                                    Err(e) => status.set(format!("Save failed: {e}")),
                                }
                            });
                        }
                    },
                    "Save + enable invites"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (contacts, peers, status) = (contacts, peers, status);
                            spawn(async move {
                                let (c, p) = load_people_lists().await;
                                apply_people_lists(c, p, contacts, peers, status, "Refreshed · ");
                            });
                        }
                    },
                    "Refresh people"
                }
            }

            // ── Invite card ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Invite someone" }
                p { style: "{MUTED}",
                    "Generate a signed invite. Copy the short code or the full invite JSON and send it out-of-band (email, message). They paste it under Accept."
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (mut invite_out, mut invite_code, mut invite_mailto, mut status) =
                                (invite_out, invite_code, invite_mailto, status);
                            spawn(async move {
                                match invoke_json::<serde_json::Value>(
                                    "generate_connect_invite",
                                    json!({ "frontDoorId": serde_json::Value::Null }),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        let code = s(&v, "code");
                                        let payload = s(&v, "invite_json");
                                        invite_out.set(payload);
                                        invite_code.set(code);
                                        invite_mailto.set(s(&v, "mailto_url"));
                                        if invite_code().is_empty() && invite_out().is_empty() {
                                            status.set("Invite returned empty — check profile invites are enabled.".into());
                                        } else {
                                            status.set("Invite ready — use Copy code or Copy full invite.".into());
                                        }
                                    }
                                    Err(e) => status.set(format!(
                                        "Generate invite failed: {e}. Save + enable invites first if sharing is off."
                                    )),
                                }
                            });
                        }
                    },
                    "Generate invite"
                }
                if !invite_code().is_empty() || !invite_out().is_empty() {
                    if !invite_code().is_empty() {
                        p { style: "font-size:1.25rem;letter-spacing:0.12em;color:#a7f3d0;font-family:monospace;margin:8px 0;", "{invite_code}" }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| copy_to_clipboard(&invite_code(), status, "Invite code copied."),
                            "Copy code"
                        }
                    }
                    if !invite_out().is_empty() {
                        button {
                            style: "{BTN2}",
                            onclick: move |_| copy_to_clipboard(&invite_out(), status, "Full invite payload copied."),
                            "Copy full invite"
                        }
                        div { style: "{CODE}", "{invite_out}" }
                    }
                    if !invite_mailto().is_empty() {
                        p { style: "margin-top:8px;",
                            a { href: "{invite_mailto}", style: "color:#93c5fd;", "Share via email" }
                        }
                    }
                }
            }

            // ── Accept invite / coop package ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Accept invite / coop package" }
                p { style: "{MUTED}",
                    "Paste a full coop share package (preferred) or bare invite JSON. Package connects you, scopes the project, and admits host+you on the local roster. Short codes alone are not enough."
                }
                textarea {
                    style: "{INPUT} min-height:90px;font-family:monospace;font-size:11px;",
                    placeholder: "Paste coop share package or invite JSON",
                    value: "{invite_in}",
                    oninput: move |e| invite_in.set(e.value()),
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (mut invite_in, mut contacts, mut peers, mut status, mut active_project, mut active_project_id, mut collab_list, mut tab) =
                                (invite_in, contacts, peers, status, active_project, active_project_id, collab_list, tab);
                            spawn(async move {
                                let input = invite_in().trim().to_string();
                                if input.is_empty() {
                                    status.set("Paste a package or invite JSON first.".into());
                                    return;
                                }
                                let result = if input.contains("qualia_coop_share") {
                                    invoke_json::<serde_json::Value>(
                                        "accept_coop_share_package",
                                        json!({ "packageOrInvite": input }),
                                    )
                                    .await
                                } else {
                                    invoke_json::<serde_json::Value>(
                                        "accept_connect_invite",
                                        json!({ "input": input }),
                                    )
                                    .await
                                    .map(|c| serde_json::json!({ "connected": true, "contact": c, "message": "Connected." }))
                                };
                                match result {
                                    Ok(v) => {
                                        let contact = v.get("contact").cloned().unwrap_or(v.clone());
                                        let name = {
                                            let n = s(&contact, "display_name");
                                            if n.is_empty() { s(&contact, "did") } else { n }
                                        };
                                        let pid = s(&v, "project_id");
                                        let pname = s(&v, "project_name");
                                        if !pid.is_empty() {
                                            active_project_id.set(pid.clone());
                                            store_active_project(&pid, if pname.is_empty() { &pid } else { &pname });
                                        }
                                        if !pname.is_empty() {
                                            active_project.set(pname.clone());
                                        }
                                        if let Some(gc) = v.get("group_chat") {
                                            let sid = s(gc, "session_id");
                                            let title = s(gc, "title");
                                            if let Some(win) = web_sys::window() {
                                                if let Ok(Some(storage)) = win.session_storage() {
                                                    if !sid.is_empty() {
                                                        let _ = storage.set_item("webizen_open_session_id", &sid);
                                                    }
                                                    if !title.is_empty() {
                                                        let _ = storage.set_item("webizen_chat_peer_title", &title);
                                                    }
                                                    if !pname.is_empty() {
                                                        let tok = pname.replace(' ', "_");
                                                        let _ = storage.set_item(
                                                            "webizen_talk_draft",
                                                            &format!("#project:{tok} Joined via package."),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        invite_in.set(String::new());
                                        let (c, p) = load_people_lists().await;
                                        apply_people_lists(c, p, contacts, peers, status, &format!("Connected with {name}. "));
                                        if !pid.is_empty() {
                                            if let Ok(list) = invoke_json::<serde_json::Value>(
                                                "list_project_collaborators",
                                                json!({ "projectId": pid }),
                                            )
                                            .await
                                            {
                                                collab_list.set(json_list(list, &["collaborators", "items"]));
                                            }
                                            let _ = invoke_json::<serde_json::Value>("mesh_start", json!({})).await;
                                            let has_group = v.get("group_chat").map(|g| !g.is_null()).unwrap_or(false);
                                            if has_group {
                                                tab.set(HubTab::Chat);
                                                status.set(format!(
                                                    "Joined with {name}. Project scoped{}. Group chat opened. Mesh started when possible.",
                                                    if pname.is_empty() { String::new() } else { format!(" to {pname}") }
                                                ));
                                            } else {
                                                tab.set(HubTab::Projects);
                                                status.set(format!(
                                                    "Joined with {name}. Project scoped{}. Mesh started when possible.",
                                                    if pname.is_empty() { String::new() } else { format!(" to {pname}") }
                                                ));
                                            }
                                        } else {
                                            status.set(format!("Connected with {name}."));
                                        }
                                    }
                                    Err(e) => status.set(format!("Accept failed: {e}")),
                                }
                            });
                        }
                    },
                    "Accept package / invite"
                }
                p { style: "margin:12px 0 6px;font-size:12px;color:#94a3b8;",
                    "Or accept a magic / deep link (registers a social mesh peer)"
                }
                textarea {
                    style: "{INPUT} min-height:48px;font-family:monospace;font-size:11px;",
                    placeholder: "Paste magic link or deep link (webizen://… or https://…)",
                    value: "{magic_accept}",
                    oninput: move |e| magic_accept.set(e.value()),
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (mut magic_accept, contacts, peers, mut status, active_project_id, active_project) =
                                (magic_accept, contacts, peers, status, active_project_id, active_project);
                            spawn(async move {
                                let link = magic_accept().trim().to_string();
                                if link.is_empty() {
                                    status.set("Paste a magic link first.".into());
                                    return;
                                }
                                match invoke_json::<serde_json::Value>(
                                    "accept_connection",
                                    json!({ "link": link }),
                                )
                                .await
                                {
                                    Ok(peer) => {
                                        let name = {
                                            let n = s(&peer, "display_name");
                                            if n.is_empty() { s(&peer, "did") } else { n }
                                        };
                                        magic_accept.set(String::new());
                                        let _ = invoke_json::<serde_json::Value>("mesh_start", json!({})).await;
                                        let pid = active_project_id();
                                        let pname = active_project();
                                        let did = s(&peer, "did");
                                        if !pid.is_empty() && !did.is_empty() {
                                            let rel = s(&peer, "relation_type");
                                            let role = if rel.eq_ignore_ascii_case("agent")
                                                || rel.eq_ignore_ascii_case("service")
                                            {
                                                "agent"
                                            } else {
                                                "contributor"
                                            };
                                            let _ = invoke_json::<serde_json::Value>(
                                                "add_project_collaborator",
                                                json!({
                                                    "projectId": pid,
                                                    "projectName": pname,
                                                    "memberDid": did,
                                                    "displayName": name,
                                                    "role": role,
                                                }),
                                            )
                                            .await;
                                        }
                                        let (c, p) = load_people_lists().await;
                                        apply_people_lists(
                                            c,
                                            p,
                                            contacts,
                                            peers,
                                            status,
                                            &format!("Connected with {name} · "),
                                        );
                                    }
                                    Err(e) => status.set(format!("Accept magic link failed: {e}")),
                                }
                            });
                        }
                    },
                    "Accept magic link"
                }
            }

            // ── Mesh card ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Mesh (reach people & bots)" }
                p { style: "{MUTED}",
                    "SocialWebNet carries chat to accepted peers. Start the mesh so collaborators (and agent/service peers) can connect. Peers without a known endpoint connect when they reach you (roaming)."
                }
                div { style: "color:#a7f3d0;font-size:12px;margin-bottom:8px;white-space:pre-wrap;",
                    if mesh_status_text().is_empty() { "Mesh status not loaded yet." } else { "{mesh_status_text}" }
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let mut mesh_status_text = mesh_status_text;
                            let mut status = status;
                            spawn(async move {
                                match invoke_json::<serde_json::Value>("mesh_start", json!({})).await {
                                    Ok(v) => {
                                        let running = v.get("running").and_then(|x| x.as_bool()).unwrap_or(true);
                                        let n = v.get("peers").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
                                        mesh_status_text.set(format!(
                                            "Mesh running={running} · {n} peer tunnel(s) configured.\n{}",
                                            serde_json::to_string_pretty(&v).unwrap_or_default()
                                        ));
                                        status.set("Mesh started — dialable peers will handshake.".into());
                                    }
                                    Err(e) => status.set(format!("Mesh start failed: {e}")),
                                }
                            });
                        }
                    },
                    "Start mesh"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let mut mesh_status_text = mesh_status_text;
                            let mut status = status;
                            spawn(async move {
                                let st = invoke_json::<serde_json::Value>("mesh_status", json!({})).await;
                                let dial = invoke_json::<serde_json::Value>("mesh_dialability", json!({})).await;
                                match (st, dial) {
                                    (Ok(s), Ok(d)) => {
                                        mesh_status_text.set(format!(
                                            "status:\n{}\n\ndialability:\n{}",
                                            serde_json::to_string_pretty(&s).unwrap_or_default(),
                                            serde_json::to_string_pretty(&d).unwrap_or_default()
                                        ));
                                    }
                                    (Err(e), _) | (_, Err(e)) => status.set(format!("Mesh status failed: {e}")),
                                }
                            });
                        }
                    },
                    "Refresh mesh status"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let mut status = status;
                            spawn(async move {
                                match invoke_json::<serde_json::Value>("mesh_stop", json!({})).await {
                                    Ok(_) => status.set("Mesh stopped.".into()),
                                    Err(e) => status.set(format!("Mesh stop failed: {e}")),
                                }
                            });
                        }
                    },
                    "Stop mesh"
                }
                div { style: "border-top:1px solid #1f2937;margin-top:12px;padding-top:10px;",
                    div { style: "color:#cbd5e1;font-size:12px;font-weight:600;margin-bottom:6px;",
                        "Set peer endpoint (so you can dial them)"
                    }
                    input {
                        style: "{INPUT}",
                        placeholder: "Peer DID",
                        value: "{peer_endpoint_did}",
                        oninput: move |e| peer_endpoint_did.set(e.value()),
                    }
                    input {
                        style: "{INPUT}",
                        placeholder: "host:port (from their mesh listen addr)",
                        value: "{peer_endpoint_edit}",
                        oninput: move |e| peer_endpoint_edit.set(e.value()),
                    }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            if peer_endpoint_did().trim().is_empty() {
                                if let Some(p) = peers().first() {
                                    peer_endpoint_did.set(s(p, "did"));
                                }
                            }
                        },
                        "Fill first peer DID"
                    }
                    button {
                        style: "{BTN}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (peer_endpoint_did, peer_endpoint_edit, mut status, mut mesh_status_text) =
                                    (peer_endpoint_did, peer_endpoint_edit, status, mesh_status_text);
                                spawn(async move {
                                    let did = peer_endpoint_did().trim().to_string();
                                    let ep = peer_endpoint_edit().trim().to_string();
                                    if did.is_empty() || ep.is_empty() {
                                        status.set("DID and host:port required.".into());
                                        return;
                                    }
                                    match invoke_json::<serde_json::Value>(
                                        "set_social_peer_endpoint",
                                        json!({ "did": did, "endpoint": ep }),
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            status.set("Endpoint saved — Start mesh again to dial.".into());
                                            let _ = invoke_json::<serde_json::Value>("mesh_start", json!({})).await;
                                            if let Ok(d) = invoke_json::<serde_json::Value>(
                                                "mesh_dialability",
                                                json!({}),
                                            )
                                            .await
                                            {
                                                mesh_status_text.set(
                                                    serde_json::to_string_pretty(&d).unwrap_or_default(),
                                                );
                                            }
                                        }
                                        Err(e) => status.set(format!("Set endpoint failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Save endpoint + restart mesh"
                    }
                }
            }

            // ── Social peers ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Social peers" }
                p { style: "{MUTED}",
                    "People (or agents) you accepted via a magic link — mesh/social peers on this machine."
                }
                if peers().is_empty() {
                    p { style: "{MUTED}",
                        "No social peers yet. Accept a magic link above, or generate one and have them accept yours."
                    }
                }
                for p in peers() {
                    {
                        let did = s(&p, "did");
                        let name = {
                            let n = s(&p, "display_name");
                            if n.is_empty() {
                                did.clone()
                            } else {
                                n
                            }
                        };
                        let rel = s(&p, "relation_type");
                        let active = p.get("active").and_then(|x| x.as_bool()).unwrap_or(true);
                        let meta = {
                            let mut parts: Vec<&str> = Vec::new();
                            if !rel.is_empty() {
                                parts.push(rel.as_str());
                            }
                            if !active {
                                parts.push("inactive");
                            }
                            parts.join(" · ")
                        };
                        let name_t = name.clone();
                        #[allow(unused_variables)]
                        let did_chat = did.clone();
                        let is_agent = rel.eq_ignore_ascii_case("agent")
                            || rel.eq_ignore_ascii_case("service");
                        rsx! {
                            div {
                                style: "padding:8px 10px;background:#0b1220;border-radius:8px;margin-bottom:6px;font-size:12px;",
                                div { style: "font-weight:600;", "{name}" }
                                if is_agent {
                                    span {
                                        style: "display:inline-block;font-size:10px;padding:2px 8px;border-radius:999px;background:#1e3a5f;color:#93c5fd;margin:2px 0;",
                                        "agent / bot peer"
                                    }
                                }
                                div { style: "font-family:monospace;color:#64748b;word-break:break-all;font-size:10px;", "{did}" }
                                if !meta.is_empty() {
                                    div { style: "margin-top:4px;font-size:11px;color:#94a3b8;", "{meta}" }
                                }
                                button {
                                    style: "{BTN2} margin-top:6px;",
                                    onclick: move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            open_talk_with(&name_t, &did_chat, &format!("Hi {name_t} — "));
                                        }
                                        let mut t = tab;
                                        t.set(HubTab::Chat);
                                        status.set(format!("Opening Chat with {name_t}."));
                                    },
                                    "Open Chat"
                                }
                            }
                        }
                    }
                }
            }

            // ── Magic link ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Magic link (domain-scoped peer)" }
                p { style: "{MUTED}",
                    "Mint a deep link for someone to peer with you. Optional domain enables an HTTPS form; relation describes the relationship."
                }
                select {
                    style: "{INPUT}",
                    value: "{relation}",
                    onchange: move |e| relation.set(e.value()),
                    option { value: "peer", "Peer (person)" }
                    option { value: "collaborator", "Collaborator" }
                    option { value: "guardian", "Guardian" }
                    option { value: "service", "Service" }
                    option { value: "agent", "Agent / bot (their local agent)" }
                }
                input {
                    style: "{INPUT}",
                    placeholder: "Domain (optional, e.g. example.org)",
                    value: "{domain_name}",
                    oninput: move |e| domain_name.set(e.value()),
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (relation, domain_name, mut magic_link, mut status) =
                                (relation, domain_name, magic_link, status);
                            spawn(async move {
                                let mut fd = String::new();
                                if let Ok(list) =
                                    invoke_json::<Vec<serde_json::Value>>("get_front_doors", json!({})).await
                                {
                                    if let Some(first) = list.first() {
                                        fd = s(first, "did_uri");
                                        if fd.is_empty() {
                                            fd = s(first, "did");
                                        }
                                    }
                                }
                                let dom = domain_name();
                                match invoke_json::<serde_json::Value>(
                                    "generate_magic_link",
                                    json!({
                                        "frontDoorDid": fd,
                                        "relationType": relation(),
                                        "domain": dom
                                    }),
                                )
                                .await
                                {
                                    Ok(v) => {
                                        let link = s(&v, "https_link");
                                        let link = if link.is_empty() { s(&v, "deep_link") } else { link };
                                        let link = if link.is_empty() {
                                            v.to_string()
                                        } else {
                                            link
                                        };
                                        magic_link.set(link);
                                        status.set("Magic link ready — copy and share it.".into());
                                    }
                                    Err(e) => status.set(format!("Magic link failed: {e}")),
                                }
                            });
                        }
                    },
                    "Generate magic link"
                }
                if !magic_link().is_empty() {
                    div { style: "{CODE}", "{magic_link}" }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| copy_to_clipboard(&magic_link(), status, "Magic link copied."),
                        "Copy magic link"
                    }
                }
            }

            // ── Contacts ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Contacts" }
                p { style: "{MUTED}",
                    "Chat contacts from accepted invites. Used for group chat and directory."
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (contacts, peers, status) = (contacts, peers, status);
                            spawn(async move {
                                let (c, p) = load_people_lists().await;
                                apply_people_lists(c, p, contacts, peers, status, "Refreshed · ");
                            });
                        }
                    },
                    "Refresh contacts & peers"
                }
                if contacts().is_empty() {
                    p { style: "{MUTED}",
                        "No contacts yet. Generate an invite and have them accept it, or accept an invite someone sent you."
                    }
                }
                for c in contacts() {
                    {
                        let did = s(&c, "did");
                        let name = {
                            let n = s(&c, "display_name");
                            if n.is_empty() { "Unnamed contact".into() } else { n }
                        };
                        let name_t = name.clone();
                        #[allow(unused_variables)]
                        let did_chat = did.clone();
                        let did_group = did.clone();
                        rsx! {
                            div {
                                style: "padding:8px 10px;background:#0b1220;border-radius:8px;margin-bottom:6px;",
                                div { style: "font-weight:600;color:#f3f4f6;", "{name}" }
                                div { style: "font-size:11px;color:#64748b;font-family:monospace;word-break:break-all;", "{did}" }
                                button {
                                    style: "{BTN2} margin-top:6px;",
                                    onclick: move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            open_talk_with(&name_t, &did_chat, &format!("Hi {name_t} — "));
                                        }
                                        let mut t = tab;
                                        t.set(HubTab::Chat);
                                        status.set(format!("Opening Chat with {name_t}."));
                                    },
                                    "Open Chat"
                                }
                                button {
                                    style: "{BTN2} margin-top:6px;",
                                    onclick: move |_| {
                                        let d = did_group.clone();
                                        if !d.is_empty() {
                                            let cur = group_dids();
                                            if cur.is_empty() {
                                                group_dids.set(d);
                                            } else if !cur.contains(&d) {
                                                group_dids.set(format!("{cur}, {d}"));
                                            }
                                            status.set("Added to group DID list below.".into());
                                        }
                                    },
                                    "Add to group"
                                }
                            }
                        }
                    }
                }
            }

            // ── Group chat ──
            div { style: "{CARD}",
                h2 { style: "{H2}", "Group chat" }
                p { style: "{MUTED}",
                    "Start a multi-party conversation from contact DIDs. Open it afterwards under Chat → Conversations."
                }
                input {
                    style: "{INPUT}", placeholder: "Group title", value: "{group_title}",
                    oninput: move |e| group_title.set(e.value()),
                }
                textarea {
                    style: "{INPUT} min-height:56px;font-family:monospace;font-size:11px;",
                    placeholder: "Participant DIDs (comma-separated)",
                    value: "{group_dids}",
                    oninput: move |e| group_dids.set(e.value()),
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        let dids: Vec<String> = contacts()
                            .iter()
                            .map(|c| s(c, "did"))
                            .filter(|d| !d.is_empty())
                            .collect();
                        group_dids.set(dids.join(", "));
                    },
                    "Fill from contacts"
                }
                button {
                    style: "{BTN}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (group_title, group_dids, mut status) = (group_title, group_dids, status);
                            spawn(async move {
                                let dids: Vec<String> = group_dids()
                                    .split(',')
                                    .map(|x| x.trim().to_string())
                                    .filter(|x| !x.is_empty())
                                    .collect();
                                if dids.is_empty() {
                                    status.set("Add at least one participant DID.".into());
                                    return;
                                }
                                let title = group_title();
                                let title_arg = if title.trim().is_empty() {
                                    serde_json::Value::Null
                                } else {
                                    json!(title)
                                };
                                match invoke_json::<String>(
                                    "create_group_chat_session",
                                    json!({ "title": title_arg, "participantDids": dids }),
                                )
                                .await
                                {
                                    Ok(id) => status.set(format!("Group created ({id}). Open it under Chat → Conversations.")),
                                    Err(e) => status.set(format!("Create group failed: {e}")),
                                }
                            });
                        }
                    },
                    "Create group"
                }
            }
        }
    }
}
