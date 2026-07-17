//! **Talk hub** — the human path for social + cooperative work (people, their agents/bots, projects).
//!
//! Tabs: **Chat** · **People** · **Reception** · **Mail** · **Projects**.
//! First-run follows `talk_setup_status` (domain → mailboxes/receiver → people → chat/projects).
//! Cooperative help depends on People + Projects working: invite peers (and agent/service relations),
//! open chat, seed a project, tag messages, share work board id.

#![allow(non_snake_case)]

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HubTab {
    Chat,
    People,
    Reception,
    /// Domains & mail (purpose inboxes, relationship addresses, transport).
    Mail,
    Projects,
}

const ROOT: &str = "display:flex;flex-direction:column;height:100%;background:#0b1220;color:#e5e7eb;box-sizing:border-box;font-family:inherit;min-height:0;";
const TABS: &str = "display:flex;gap:4px;padding:8px 14px;border-bottom:1px solid #1f2937;background:#0f172a;flex-shrink:0;overflow-x:auto;";
const TAB: &str = "padding:8px 14px;border-radius:8px;border:1px solid transparent;background:transparent;color:#94a3b8;font-weight:600;font-size:13px;cursor:pointer;white-space:nowrap;";
const TAB_ON: &str = "padding:8px 14px;border-radius:8px;border:1px solid #8b5cf6;background:rgba(139,92,246,0.15);color:#e9d5ff;font-weight:600;font-size:13px;cursor:pointer;white-space:nowrap;";
const PANEL: &str = "flex:1;overflow-y:auto;padding:1.25rem 1.5rem;min-height:0;";
const CARD: &str = "background:#111827;border:1px solid #1f2937;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;";
const H2: &str = "margin:0 0 0.35rem;font-size:1.15rem;color:#e9d5ff;font-weight:700;";
const MUTED: &str = "margin:0 0 0.85rem;color:#94a3b8;font-size:0.88rem;line-height:1.5;";
const INPUT: &str = "width:100%;box-sizing:border-box;padding:9px 11px;margin-bottom:8px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;font-family:inherit;font-size:13px;";
const BTN: &str = "background:#8b5cf6;color:white;padding:9px 14px;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-size:13px;margin-right:8px;margin-bottom:6px;";
const BTN2: &str = "background:#334155;color:#e5e7eb;padding:8px 12px;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-size:12px;margin-right:8px;margin-bottom:6px;";
const STATUS: &str = "padding:8px 14px;background:#0b3b2e;border-bottom:1px solid #10b981;color:#a7f3d0;font-size:12px;white-space:pre-wrap;flex-shrink:0;";
const CODE: &str = "font-family:ui-monospace,Consolas,monospace;font-size:12px;background:#0b1220;border:1px solid #334155;border-radius:8px;padding:10px;white-space:pre-wrap;word-break:break-all;color:#a7f3d0;max-height:220px;overflow:auto;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Tauri often returns Wellfair payloads as a JSON *string*; normalize to object.
fn as_object(v: serde_json::Value) -> serde_json::Value {
    if let Some(s) = v.as_str() {
        serde_json::from_str(s).unwrap_or(v)
    } else {
        v
    }
}

/// Normalize list-ish JSON (array, `{peers|contacts|…}`, or stringified JSON) to a vec of objects.
fn json_list(v: serde_json::Value, wrapper_keys: &[&str]) -> Vec<serde_json::Value> {
    let v = as_object(v);
    if let Some(arr) = v.as_array() {
        return arr.clone();
    }
    for key in wrapper_keys {
        if let Some(arr) = v.get(*key).and_then(|p| p.as_array()) {
            return arr.clone();
        }
    }
    vec![]
}

/// Hand off to Chat: draft text + optional conversation title for a peer/contact/agent.
#[cfg(target_arch = "wasm32")]
fn open_talk_with(name: &str, did: &str, draft_prefix: &str) {
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.session_storage() {
            let draft = if draft_prefix.is_empty() {
                format!("Hi {name} — ")
            } else {
                draft_prefix.to_string()
            };
            let _ = storage.set_item("webizen_talk_draft", &draft);
            if !name.is_empty() {
                let _ = storage.set_item("webizen_chat_peer_title", name);
            }
            if !did.is_empty() {
                let _ = storage.set_item("webizen_chat_peer_did", did);
            }
        }
    }
}

/// Load chat contacts + social peers together (People tab / boot / after accept).
#[cfg(target_arch = "wasm32")]
async fn load_people_lists() -> (Result<Vec<serde_json::Value>, String>, Result<Vec<serde_json::Value>, String>) {
    let contacts = match invoke_json::<serde_json::Value>("list_chat_contacts", json!({})).await {
        Ok(v) => Ok(json_list(v, &["contacts", "items"])),
        Err(e) => Err(e),
    };
    let peers = match invoke_json::<serde_json::Value>("list_social_peers", json!({})).await {
        Ok(v) => Ok(json_list(v, &["peers", "items"])),
        Err(e) => Err(e),
    };
    (contacts, peers)
}

/// Apply list results + a short status line for the People tab.
#[cfg(target_arch = "wasm32")]
fn apply_people_lists(
    contacts_res: Result<Vec<serde_json::Value>, String>,
    peers_res: Result<Vec<serde_json::Value>, String>,
    mut contacts: Signal<Vec<serde_json::Value>>,
    mut peers: Signal<Vec<serde_json::Value>>,
    mut status: Signal<String>,
    prefix: &str,
) {
    let mut errs: Vec<String> = Vec::new();
    let n_c = match contacts_res {
        Ok(list) => {
            let n = list.len();
            contacts.set(list);
            n
        }
        Err(e) => {
            errs.push(format!("contacts: {e}"));
            contacts().len()
        }
    };
    let n_p = match peers_res {
        Ok(list) => {
            let n = list.len();
            peers.set(list);
            n
        }
        Err(e) => {
            errs.push(format!("peers: {e}"));
            peers().len()
        }
    };
    if errs.is_empty() {
        status.set(format!("{prefix}{n_c} contact(s), {n_p} peer(s)."));
    } else {
        status.set(format!(
            "{prefix}{n_c} contact(s), {n_p} peer(s). Load issue: {}",
            errs.join("; ")
        ));
    }
}

/// Project board id is the uuid suffix of `urn:wellfair:project:…`.
fn project_board_id(record_id: &str) -> String {
    record_id
        .rsplit(':')
        .next()
        .unwrap_or(record_id)
        .to_string()
}

/// Journal `summary` for projects is a JSON object string
/// (`{"name","description","created_at_unix"}` from wellfare-core), not a plain title.
fn project_display_name(summary: Option<&str>, fallback: &str) -> String {
    let Some(raw) = summary.map(str::trim).filter(|s| !s.is_empty()) else {
        return fallback.to_string();
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(name) = v.get("name").and_then(|x| x.as_str()).map(str::trim) {
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    // Plain-string summary (or non-object JSON) — use as-is when short enough to be a title.
    if !raw.starts_with('{') && raw.len() < 200 {
        return raw.to_string();
    }
    fallback.to_string()
}

fn store_active_project(id: &str, name: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(win) = web_sys::window() {
        if let Ok(Some(storage)) = win.session_storage() {
            if !id.is_empty() {
                let _ = storage.set_item("webizen_active_project_id", id);
            }
            if !name.is_empty() {
                let _ = storage.set_item("webizen_active_project_name", name);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (id, name);
    }
}

fn vault_hint(err: &str) -> String {
    let e = err.to_lowercase();
    if e.contains("unlock")
        || e.contains("vault")
        || e.contains("host api not initialized")
        || e.contains("not initialized")
    {
        format!(
            "{err}\n\nFirst-run for cooperative work: Open Sanctuary → create/unlock vault → return here → Create or Seed project → People (invite) → Admit members on the project → Start mesh so peers and their bots can reach you."
        )
    } else {
        err.to_string()
    }
}

/// Plain-language vault state for the Projects tab (from `wellfair_host_snapshot`).
fn vault_state_label(v: crate::components::wellfair::host_dto::VaultLifecycle) -> &'static str {
    use crate::components::wellfair::host_dto::VaultLifecycle;
    match v {
        VaultLifecycle::Unlocked => "Unlocked",
        VaultLifecycle::Locked => "Locked",
        VaultLifecycle::Unconfigured => "Not set up",
    }
}

fn vault_state_detail(v: crate::components::wellfair::host_dto::VaultLifecycle) -> &'static str {
    use crate::components::wellfair::host_dto::VaultLifecycle;
    match v {
        VaultLifecycle::Unlocked => {
            "Project records can be listed and created from this machine."
        }
        VaultLifecycle::Locked => {
            "Unlock the vault in Sanctuary before listing or creating projects."
        }
        VaultLifecycle::Unconfigured => {
            "No Sanctuary vault yet — create one, then return here for cooperative projects."
        }
    }
}

fn vault_needs_attention(v: crate::components::wellfair::host_dto::VaultLifecycle) -> bool {
    use crate::components::wellfair::host_dto::VaultLifecycle;
    !matches!(v, VaultLifecycle::Unlocked)
}

#[cfg(target_arch = "wasm32")]
async fn list_project_records() -> Result<Vec<(String, String)>, String> {
    // Host returns Result<String, String> → JSON array of journal/health records.
    let raw = invoke_json::<serde_json::Value>(
        "wellfair_list_health_records",
        json!({ "limit": 96 }),
    )
    .await?;
    let arr = json_list(raw, &["records", "items"]);
    Ok(arr
        .into_iter()
        .filter(|r| s(r, "kind") == "project")
        .map(|r| {
            let id = project_board_id(&s(&r, "id"));
            let summary = r.get("summary").and_then(|x| x.as_str());
            let label = project_display_name(summary, &id);
            (id, label)
        })
        .collect())
}

#[cfg(target_arch = "wasm32")]
async fn create_project_record(
    name: &str,
    description: &str,
    licensing: Vec<&str>,
) -> Result<(String, String, serde_json::Value), String> {
    let onts: Vec<String> = licensing.into_iter().map(|s| s.to_string()).collect();
    let raw = invoke_json::<serde_json::Value>(
        "wellfair_add_project",
        json!({
            "name": name,
            "description": description,
            "licensingOntologies": onts
        }),
    )
    .await?;
    // Host returns Result<String, String> — often a JSON *string* of the JournalEntry.
    let obj = as_object(raw);
    let full_id = s(&obj, "id");
    if full_id.is_empty() {
        return Err("Project response missing id (expected urn:wellfair:project:…).".into());
    }
    let board_id = project_board_id(&full_id);
    let summary = obj.get("summary").and_then(|x| x.as_str());
    let label = project_display_name(summary, name);
    Ok((board_id, label, obj))
}

/// Best-effort clipboard write (browser / webview).
#[cfg(target_arch = "wasm32")]
fn copy_to_clipboard(text: &str, mut status: Signal<String>, ok_msg: &str) {
    let text = text.to_string();
    let ok_msg = ok_msg.to_string();
    spawn(async move {
        if let Some(win) = web_sys::window() {
            let nav = win.navigator();
            let clipboard = nav.clipboard();
            let prom = clipboard.write_text(&text);
            match wasm_bindgen_futures::JsFuture::from(prom).await {
                Ok(_) => status.set(ok_msg),
                Err(_) => status.set("Could not copy — select the text and copy manually.".into()),
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn copy_to_clipboard(_text: &str, mut status: Signal<String>, _ok_msg: &str) {
    status.set("Copy is available in the desktop webview.".into());
}

/// Apply `front_door_forms` JSON into the Reception DNS fields.
#[cfg(target_arch = "wasm32")]
fn apply_front_door_forms(
    v: &serde_json::Value,
    mut dns_name: Signal<String>,
    mut dns_txt: Signal<String>,
    mut turtle: Signal<String>,
) {
    dns_name.set(s(v, "dns_name"));
    dns_txt.set(s(v, "dns_txt"));
    turtle.set(s(v, "turtle"));
}

/// Fetch DNS copy-paste values for one domain into the Reception fields.
#[cfg(target_arch = "wasm32")]
async fn load_dns_forms_for(
    domain: &str,
    dns_name: Signal<String>,
    dns_txt: Signal<String>,
    turtle: Signal<String>,
) -> Result<(), String> {
    let v = invoke_json::<serde_json::Value>("front_door_forms", json!({ "domain": domain })).await?;
    apply_front_door_forms(&v, dns_name, dns_txt, turtle);
    Ok(())
}

/// Fetch DNS values for every listed domain; join with clear separators for multi-domain paste.
#[cfg(target_arch = "wasm32")]
async fn load_dns_forms_for_all(
    domain_names: &[String],
    mut dns_name: Signal<String>,
    mut dns_txt: Signal<String>,
    mut turtle: Signal<String>,
) -> Result<usize, String> {
    let mut names = String::new();
    let mut txts = String::new();
    let mut turtles = String::new();
    let mut ok = 0usize;
    let mut last_err = String::new();
    for domain in domain_names {
        match invoke_json::<serde_json::Value>("front_door_forms", json!({ "domain": domain })).await
        {
            Ok(v) => {
                if ok > 0 {
                    names.push_str("\n\n");
                    txts.push_str("\n\n");
                    turtles.push_str("\n\n");
                }
                names.push_str(&format!("# {domain}\n{}", s(&v, "dns_name")));
                txts.push_str(&format!("# {domain}\n{}", s(&v, "dns_txt")));
                let t = s(&v, "turtle");
                if !t.is_empty() {
                    turtles.push_str(&format!("# {domain}\n{t}"));
                }
                ok += 1;
            }
            Err(e) => last_err = format!("{domain}: {e}"),
        }
    }
    if ok == 0 {
        return Err(if last_err.is_empty() {
            "No DNS values could be built.".into()
        } else {
            last_err
        });
    }
    dns_name.set(names);
    dns_txt.set(txts);
    turtle.set(turtles);
    Ok(ok)
}

/// Primary Talk surface: Chat · People · Reception · Projects.
#[component]
pub fn SocialHub() -> Element {
    let mut tab = use_signal(|| {
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
    let mut status = use_signal(String::new);

    // Shared people state
    let mut contacts = use_signal(Vec::<serde_json::Value>::new);
    let mut invite_code = use_signal(String::new);
    let mut invite_out = use_signal(String::new);
    let mut invite_mailto = use_signal(String::new);
    let mut invite_in = use_signal(String::new);
    let mut display_name = use_signal(String::new);
    let mut group_title = use_signal(String::new);
    let mut group_dids = use_signal(String::new);
    let mut magic_link = use_signal(String::new);
    let mut relation = use_signal(|| "peer".to_string());

    // Reception state
    let mut domain_name = use_signal(String::new);
    let mut domain_label = use_signal(String::new);
    let mut domains = use_signal(Vec::<serde_json::Value>::new);
    let mut front_doors = use_signal(Vec::<serde_json::Value>::new);
    let mut dns_name = use_signal(String::new);
    let mut dns_txt = use_signal(String::new);
    let mut turtle = use_signal(String::new);

    // Projects state
    let mut project_name = use_signal(String::new);
    let mut active_project = use_signal(String::new);
    let mut active_project_id = use_signal(String::new);
    let mut project_list = use_signal(Vec::<(String, String)>::new);
    let mut last_project_json = use_signal(String::new);
    let mut peers = use_signal(Vec::<serde_json::Value>::new);
    let mut magic_accept = use_signal(String::new);
    let mut active_model_chip = use_signal(String::new);
    let mut vault_lifecycle = use_signal(|| {
        crate::components::wellfair::host_dto::VaultLifecycle::Unconfigured
    });
    let mut setup_banner = use_signal(String::new);
    let mut mesh_status_text = use_signal(String::new);
    let mut collab_list = use_signal(Vec::<serde_json::Value>::new);
    let mut collab_did = use_signal(String::new);
    let mut collab_name = use_signal(String::new);
    let mut collab_role = use_signal(|| "contributor".to_string());
    let mut peer_endpoint_edit = use_signal(String::new);
    let mut peer_endpoint_did = use_signal(String::new);
    let mut coop_package_text = use_signal(String::new);

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
                        let (mut project_list, mut status, mut vault_lifecycle) =
                            (project_list, status, vault_lifecycle);
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
                                        "{n} project(s). Vault: {}. Select one or create.",
                                        vault_state_label(snap.vault)
                                    ));
                                }
                                Err(e) => status.set(vault_hint(&e)),
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
                div { style: "{PANEL}",
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
                                        let body = json!({
                                            "display_name": display_name(),
                                            "sharing": { "allow_group_chat_invites": true }
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
                                        // Prefer full package accept; fall back to bare invite.
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
                                                // Group chat from accept_coop_share — hand off to Chat.
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
                                    let (mut magic_accept, contacts, peers, status, active_project_id, active_project) =
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
                                                // Best-effort: bring mesh up so the new peer can reach us.
                                                let _ = invoke_json::<serde_json::Value>("mesh_start", json!({})).await;
                                                // Auto-admit peer to active project when scoped.
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
                                let did_t = did.clone();
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
                                                    open_talk_with(&name_t, &did_t, &format!("Hi {name_t} — "));
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
                                        // Prefer first front-door DID if any.
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
                                let did_t = did.clone();
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
                                                    open_talk_with(&name_t, &did_t, &format!("Hi {name_t} — "));
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
                                                let d = did_t.clone();
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

            // ── Reception (domain front door) ─────────────────────────────
            if tab() == HubTab::Reception {
                div { style: "{PANEL}",
                    div { style: "{CARD}",
                        h2 { style: "{H2}", "Reception — be findable by domain" }
                        p { style: "{MUTED}",
                            "Three steps: (1) make a public-facing identity, (2) link a domain you own, (3) paste one TXT record at your domain registrar. Your private vault is never published."
                        }
                    }

                    div { style: "{CARD}",
                        h2 { style: "{H2}", "1. Your public identity" }
                        p { style: "{MUTED}",
                            "Create the front-door identity others will use to find you. Skip if you already have one listed below — step 2 can create one for you if needed."
                        }
                        button {
                            style: "{BTN}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut front_doors, mut status) = (front_doors, status);
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>(
                                            "generate_front_door",
                                            json!({ "label": "Primary reception" }),
                                        )
                                        .await
                                        {
                                            Ok(_) => {
                                                status.set("Public identity ready.".into());
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("get_front_doors", json!({})).await {
                                                    front_doors.set(list);
                                                }
                                            }
                                            Err(e) => status.set(format!("Could not create identity: {e}")),
                                        }
                                    });
                                }
                            },
                            "Create public identity"
                        }
                        if front_doors().is_empty() {
                            p { style: "{MUTED}", "None yet — create one, or register a domain and we will make one." }
                        }
                        for d in front_doors() {
                            div {
                                style: "padding:8px 10px;background:#0b1220;border-radius:8px;margin-bottom:6px;font-size:12px;",
                                div { style: "font-weight:600;", "{s(&d, \"label\")}" }
                                div { style: "font-family:monospace;color:#94a3b8;word-break:break-all;",
                                    {
                                        let did = s(&d, "did_uri");
                                        if did.is_empty() { s(&d, "did") } else { did }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "{CARD}",
                        h2 { style: "{H2}", "2. Register your domain" }
                        p { style: "{MUTED}",
                            "Type a domain you control (for example example.org). We link it to your public identity, then prepare the DNS values automatically."
                        }
                        input {
                            style: "{INPUT}", placeholder: "Domain name (example.org)", value: "{domain_name}",
                            oninput: move |e| domain_name.set(e.value()),
                        }
                        input {
                            style: "{INPUT}", placeholder: "Friendly label (optional)", value: "{domain_label}",
                            oninput: move |e| domain_label.set(e.value()),
                        }
                        button {
                            style: "{BTN}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (
                                        domain_name,
                                        domain_label,
                                        front_doors,
                                        mut domains,
                                        mut front_doors_sig,
                                        dns_name,
                                        dns_txt,
                                        turtle,
                                        mut status,
                                    ) = (
                                        domain_name,
                                        domain_label,
                                        front_doors,
                                        domains,
                                        front_doors,
                                        dns_name,
                                        dns_txt,
                                        turtle,
                                        status,
                                    );
                                    spawn(async move {
                                        let name = domain_name().trim().to_string();
                                        if name.is_empty() {
                                            status.set("Enter a domain name.".into());
                                            return;
                                        }
                                        let mut fd = String::new();
                                        if let Some(first) = front_doors().first() {
                                            fd = s(first, "did_uri");
                                            if fd.is_empty() {
                                                fd = s(first, "did");
                                            }
                                        }
                                        if fd.is_empty() {
                                            // Create a door if none.
                                            if let Ok(door) = invoke_json::<serde_json::Value>(
                                                "generate_front_door",
                                                json!({ "label": format!("Door for {name}") }),
                                            )
                                            .await
                                            {
                                                fd = s(&door, "did_uri");
                                                if fd.is_empty() {
                                                    fd = s(&door, "did");
                                                }
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>(
                                                    "get_front_doors",
                                                    json!({}),
                                                )
                                                .await
                                                {
                                                    front_doors_sig.set(list);
                                                }
                                            }
                                        }
                                        if fd.is_empty() {
                                            status.set("Could not create a public identity — try step 1 first.".into());
                                            return;
                                        }
                                        let label = domain_label();
                                        match invoke_json::<serde_json::Value>(
                                            "add_mail_domain",
                                            json!({
                                                "name": name,
                                                "agentType": "person",
                                                "frontDoorDid": fd,
                                                "label": label,
                                                "parent": serde_json::Value::Null
                                            }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                if let Some(arr) = v.as_array() {
                                                    domains.set(arr.clone());
                                                } else if let Ok(v2) = invoke_json::<serde_json::Value>("list_mail_domains", json!({})).await {
                                                    if let Some(arr) = v2.as_array() {
                                                        domains.set(arr.clone());
                                                    } else if let Some(arr) = v2.get("domains").and_then(|d| d.as_array()) {
                                                        domains.set(arr.clone());
                                                    }
                                                }
                                                // Auto-build DNS so the user does not need a second click.
                                                // Onboard purpose inboxes + catchall (semantic mail surface).
                                                let mail_msg = match invoke_json::<serde_json::Value>(
                                                    "onboard_mail_domain",
                                                    json!({ "domain": name }),
                                                )
                                                .await
                                                {
                                                    Ok(v) => v
                                                        .get("message")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("Mail onboarded.")
                                                        .to_string(),
                                                    Err(e) => format!("Mail onboard skipped: {e}"),
                                                };
                                                match load_dns_forms_for(
                                                    &name,
                                                    dns_name,
                                                    dns_txt,
                                                    turtle,
                                                )
                                                .await
                                                {
                                                    Ok(()) => status.set(format!(
                                                        "Domain {name} registered. {mail_msg} DNS ready below — then open Mail tab."
                                                    )),
                                                    Err(e) => status.set(format!(
                                                        "Domain {name} registered. {mail_msg} DNS failed: {e}."
                                                    )),
                                                }
                                            }
                                            Err(e) => status.set(format!("Could not register domain: {e}")),
                                        }
                                    });
                                }
                            },
                            "Register domain"
                        }

                        // One-click DNS when domains already exist (no extra hunting).
                        if !domains().is_empty() {
                            div {
                                style: "margin-top:10px;padding-top:10px;border-top:1px solid #1f2937;",
                                p { style: "{MUTED}",
                                    "Already registered? Build the DNS values in one click, then copy them in step 3."
                                }
                                {
                                    let first_name = domains()
                                        .first()
                                        .map(|d| s(d, "name"))
                                        .unwrap_or_default();
                                    let all_names: Vec<String> =
                                        domains().iter().map(|d| s(d, "name")).collect();
                                    let multi = all_names.len() > 1;
                                    let first_btn_label = if multi {
                                        "Build DNS for first domain"
                                    } else {
                                        "Build DNS record"
                                    };
                                    rsx! {
                                        if !first_name.is_empty() {
                                            button {
                                                style: "{BTN}",
                                                onclick: move |_| {
                                                    let first = first_name.clone();
                                                    domain_name.set(first.clone());
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let (dns_name, dns_txt, turtle, mut status) =
                                                            (dns_name, dns_txt, turtle, status);
                                                        spawn(async move {
                                                            match load_dns_forms_for(
                                                                &first,
                                                                dns_name,
                                                                dns_txt,
                                                                turtle,
                                                            )
                                                            .await
                                                            {
                                                                Ok(()) => status.set(format!(
                                                                    "DNS ready for {first} — copy name + TXT in step 3."
                                                                )),
                                                                Err(e) => status.set(format!(
                                                                    "Could not build DNS: {e}"
                                                                )),
                                                            }
                                                        });
                                                    }
                                                },
                                                "{first_btn_label}"
                                            }
                                        }
                                        if multi {
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| {
                                                    let names = all_names.clone();
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let (dns_name, dns_txt, turtle, mut status) =
                                                            (dns_name, dns_txt, turtle, status);
                                                        spawn(async move {
                                                            match load_dns_forms_for_all(
                                                                &names,
                                                                dns_name,
                                                                dns_txt,
                                                                turtle,
                                                            )
                                                            .await
                                                            {
                                                                Ok(n) => status.set(format!(
                                                                    "DNS ready for {n} domain(s) — copy name + TXT in step 3."
                                                                )),
                                                                Err(e) => status.set(format!(
                                                                    "Could not build DNS: {e}"
                                                                )),
                                                            }
                                                        });
                                                    }
                                                },
                                                "Build DNS for all domains"
                                            }
                                        }
                                    }
                                }
                                // Per-domain shortcuts (same action, named).
                                p { style: "margin:10px 0 6px;font-size:12px;color:#64748b;", "Or pick one domain:" }
                                for d in domains() {
                                    {
                                        let name = s(&d, "name");
                                        rsx! {
                                            button {
                                                style: "{BTN2}",
                                                onclick: move |_| {
                                                    let name = name.clone();
                                                    domain_name.set(name.clone());
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let (dns_name, dns_txt, turtle, mut status) =
                                                            (dns_name, dns_txt, turtle, status);
                                                        spawn(async move {
                                                            match load_dns_forms_for(
                                                                &name,
                                                                dns_name,
                                                                dns_txt,
                                                                turtle,
                                                            )
                                                            .await
                                                            {
                                                                Ok(()) => status.set(format!(
                                                                    "DNS ready for {name} — copy into your registrar."
                                                                )),
                                                                Err(e) => status.set(format!(
                                                                    "Could not build DNS for {name}: {e}"
                                                                )),
                                                            }
                                                        });
                                                    }
                                                },
                                                "DNS for {name}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "{CARD}",
                        h2 { style: "{H2}", "3. Copy DNS into your registrar" }
                        p { style: "{MUTED}",
                            "At the place you manage the domain, add a TXT record. Paste the name, then the value. Your private keys are never included."
                        }
                        if dns_name().is_empty() {
                            p { style: "{MUTED}",
                                "Nothing to copy yet — register a domain (step 2) or use “Build DNS” if you already have one."
                            }
                        } else {
                            p { style: "margin:0 0 6px;font-size:12px;color:#94a3b8;", "Record name" }
                            div { style: "{CODE}", "{dns_name}" }
                            button {
                                style: "{BTN2}",
                                onclick: move |_| copy_to_clipboard(&dns_name(), status, "DNS name copied."),
                                "Copy name"
                            }
                            p { style: "margin:12px 0 6px;font-size:12px;color:#94a3b8;", "TXT value" }
                            div { style: "{CODE}", "{dns_txt}" }
                            button {
                                style: "{BTN2}",
                                onclick: move |_| copy_to_clipboard(&dns_txt(), status, "TXT value copied — paste at your registrar."),
                                "Copy TXT"
                            }
                            if !turtle().is_empty() {
                                p { style: "margin:12px 0 6px;font-size:12px;color:#94a3b8;",
                                    "Optional profile text (advanced) — only if you also host a small web page for this domain"
                                }
                                div { style: "{CODE}", "{turtle}" }
                                button {
                                    style: "{BTN2}",
                                    onclick: move |_| copy_to_clipboard(&turtle(), status, "Profile text copied."),
                                    "Copy Turtle"
                                }
                            }
                        }
                    }
                }
            }

            // ── Projects (cooperative) ────────────────────────────────────
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

            if tab() == HubTab::Projects {
                {
                    let vault = vault_lifecycle();
                    let vault_label = vault_state_label(vault);
                    let vault_detail = vault_state_detail(vault);
                    let vault_attention = vault_needs_attention(vault);
                    rsx! {
                div { style: "{PANEL}",
                    // Vault state (from wellfair_host_snapshot)
                    div {
                        style: if vault_attention {
                            "background:#1c1917;border:1px solid #f59e0b;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;"
                        } else {
                            "background:#052e1c;border:1px solid #10b981;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;"
                        },
                        h2 { style: "margin:0 0 0.35rem;font-size:1.05rem;color:#fde68a;font-weight:700;",
                            "Vault: {vault_label}"
                        }
                        p { style: "margin:0 0 0.75rem;color:#cbd5e1;font-size:0.88rem;line-height:1.5;",
                            "{vault_detail}"
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

                    div { style: "{CARD}",
                        h2 { style: "{H2}", "Cooperative projects" }
                        p { style: "{MUTED}",
                            "Projects are where relationships become work: tasks, chat scoped with #project:, contributions. This is also how QualiaDB's own development is meant to be hosted among peers."
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
                                                // Refresh vault state so the banner reflects lock failures.
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
                                    // Prefer first contact DID if field empty.
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
                                                    // Best-effort vault-backed membership when unlocked.
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
                    } // rsx! vault-scoped Projects panel
                } // let vault_label block
            }
        }
    }
}
