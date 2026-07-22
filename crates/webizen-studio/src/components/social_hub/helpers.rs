//! Utility functions for the Talk hub — JSON normalization, clipboard,
//! vault helpers, project record helpers, DNS form loading, Tauri invoke.

#![allow(non_snake_case)]

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;

// ── Tauri invoke bridge ───────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
pub async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

// ── JSON normalization ────────────────────────────────────────────────

/// Tauri often returns Wellfair payloads as a JSON *string*; normalize to object.
#[cfg(target_arch = "wasm32")]
pub fn as_object(v: serde_json::Value) -> serde_json::Value {
    if let Some(s) = v.as_str() {
        serde_json::from_str(s).unwrap_or(v)
    } else {
        v
    }
}

/// Normalize list-ish JSON (array, `{peers|contacts|…}`, or stringified JSON) to a vec of objects.
#[cfg(target_arch = "wasm32")]
pub fn json_list(v: serde_json::Value, wrapper_keys: &[&str]) -> Vec<serde_json::Value> {
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

// ── Chat hand-off ─────────────────────────────────────────────────────

/// Hand off to Chat: draft text + optional conversation title for a peer/contact/agent.
#[cfg(target_arch = "wasm32")]
pub fn open_talk_with(name: &str, did: &str, draft_prefix: &str) {
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

// ── People list loading ───────────────────────────────────────────────

/// Load chat contacts + social peers together (People tab / boot / after accept).
#[cfg(target_arch = "wasm32")]
pub async fn load_people_lists() -> (
    Result<Vec<serde_json::Value>, String>,
    Result<Vec<serde_json::Value>, String>,
) {
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
pub fn apply_people_lists(
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

// ── Project helpers ───────────────────────────────────────────────────

/// Project board id is the uuid suffix of `urn:wellfair:project:…`.
#[cfg(target_arch = "wasm32")]
pub fn project_board_id(record_id: &str) -> String {
    record_id
        .rsplit(':')
        .next()
        .unwrap_or(record_id)
        .to_string()
}

/// Journal `summary` for projects is a JSON object string
/// (`{"name","description","created_at_unix"}` from wellfare-core), not a plain title.
#[cfg(target_arch = "wasm32")]
pub fn project_display_name(summary: Option<&str>, fallback: &str) -> String {
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

pub fn store_active_project(id: &str, name: &str) {
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

#[cfg(target_arch = "wasm32")]
pub async fn list_project_records() -> Result<Vec<(String, String)>, String> {
    // Prefer local-first coop registry (works without vault).
    let mut out: Vec<(String, String)> = Vec::new();
    if let Ok(v) = invoke_json::<serde_json::Value>("list_coop_projects", json!({})).await {
        for row in json_list(v, &["projects", "items"]) {
            let id = s(&row, "id");
            let name = s(&row, "name");
            let n = row
                .get("member_count")
                .and_then(|x| x.as_u64())
                .unwrap_or(0);
            if !id.is_empty() {
                let label = if n > 0 {
                    format!("{name} · {n} member(s)")
                } else if name.is_empty() {
                    id.clone()
                } else {
                    name
                };
                out.push((id, label));
            }
        }
    }
    // Merge vault/wellfair projects when host is unlocked.
    if let Ok(raw) = invoke_json::<serde_json::Value>(
        "wellfair_list_health_records",
        json!({ "limit": 96 }),
    )
    .await
    {
        let arr = json_list(raw, &["records", "items"]);
        for r in arr.into_iter().filter(|r| s(r, "kind") == "project") {
            let id = project_board_id(&s(&r, "id"));
            let summary = r.get("summary").and_then(|x| x.as_str());
            let label = project_display_name(summary, &id);
            if !out.iter().any(|(i, _)| i == &id) {
                out.push((id, label));
            }
        }
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub async fn create_project_record(
    name: &str,
    description: &str,
    licensing: Vec<&str>,
) -> Result<(String, String, serde_json::Value), String> {
    // Try vault-backed create first; always fall back to local coop project so collab is not blocked.
    let onts: Vec<String> = licensing.into_iter().map(|s| s.to_string()).collect();
    match invoke_json::<serde_json::Value>(
        "wellfair_add_project",
        json!({
            "name": name,
            "description": description,
            "licensingOntologies": onts
        }),
    )
    .await
    {
        Ok(raw) => {
            let obj = as_object(raw);
            let full_id = s(&obj, "id");
            if full_id.is_empty() {
                // Fall through to local.
            } else {
                let board_id = project_board_id(&full_id);
                let summary = obj.get("summary").and_then(|x| x.as_str());
                let label = project_display_name(summary, name);
                // Mirror into local registry for join packages / roster.
                let _ = invoke_json::<serde_json::Value>(
                    "create_coop_project",
                    json!({ "name": name, "description": description }),
                )
                .await;
                return Ok((board_id, label, obj));
            }
        }
        Err(_) => {}
    }
    let local = invoke_json::<serde_json::Value>(
        "create_coop_project",
        json!({ "name": name, "description": description }),
    )
    .await
    .map_err(|e| {
        format!(
            "{e} — could not create local project either. Check Talk → People display name."
        )
    })?;
    let id = s(&local, "id");
    let label = s(&local, "name");
    if id.is_empty() {
        return Err("local project create returned no id".into());
    }
    Ok((id, if label.is_empty() { name.to_string() } else { label }, local))
}

// ── Vault helpers ─────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub fn vault_hint(err: &str) -> String {
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
pub fn vault_state_label(v: crate::components::wellfair::host_dto::VaultLifecycle) -> &'static str {
    use crate::components::wellfair::host_dto::VaultLifecycle;
    match v {
        VaultLifecycle::Unlocked => "Unlocked",
        VaultLifecycle::Locked => "Locked",
        VaultLifecycle::Unconfigured => "Not set up",
    }
}

pub fn vault_state_detail(
    v: crate::components::wellfair::host_dto::VaultLifecycle,
) -> &'static str {
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

pub fn vault_needs_attention(v: crate::components::wellfair::host_dto::VaultLifecycle) -> bool {
    use crate::components::wellfair::host_dto::VaultLifecycle;
    !matches!(v, VaultLifecycle::Unlocked)
}

// ── Clipboard ─────────────────────────────────────────────────────────

/// Best-effort clipboard write (browser / webview).
#[cfg(target_arch = "wasm32")]
pub fn copy_to_clipboard(text: &str, mut status: Signal<String>, ok_msg: &str) {
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
pub fn copy_to_clipboard(_text: &str, mut status: Signal<String>, _ok_msg: &str) {
    status.set("Copy is available in the desktop webview.".into());
}

// ── DNS form helpers ──────────────────────────────────────────────────

/// Apply `front_door_forms` JSON into the Reception DNS fields.
#[cfg(target_arch = "wasm32")]
pub fn apply_front_door_forms(
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
pub async fn load_dns_forms_for(
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
pub async fn load_dns_forms_for_all(
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
