//! **Domains & Mail** pane — manage the mail domains a person owns and the addresses under each, with a
//! **BYO-networking, no-hosting** posture: a domain is bound to a front-door DID, and the "front-door forms"
//! it produces are a DNS `TXT` record you paste at your registrar/Cloudflare — no server to run.
//!
//! Two kinds of addresses live under a domain: **purpose inboxes** (minted from a preset whose rules encode
//! the inbox's intent — e.g. what may reach it) and **relationship addresses** (bound to a specific
//! relationship DID so a correspondent gets their own pairwise address, not a shared firehose). Each address
//! can be enabled/disabled without deletion, so a leaked or retired address is silenced, not lost.
//!
//! Backend Tauri commands (all args camelCase): `list_mail_domains`, `add_mail_domain`,
//! `purpose_inbox_presets`, `list_mail_addresses`, `mint_purpose_inbox`, `mint_relationship_address`,
//! `set_mail_address_enabled`, `front_door_forms`.

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

const PANEL: &str = "background: #1f2937; padding: 14px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);";
const INPUT: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; background: #111827; color: #f3f4f6; border: 1px solid #374151; border-radius: 8px; font-family: inherit;";
const BTN: &str = "background: #8b5cf6; color: white; padding: 7px 14px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer;";
const BTN_MUTED: &str = "background: #374151; color: #e5e7eb; padding: 5px 10px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 12px;";
const CHIP: &str = "display: inline-block; font-size: 11px; padding: 2px 8px; border-radius: 999px; background: #0f172a; color: #a5b4fc; margin: 2px 4px 2px 0; border: 1px solid #334155;";
const TEXTAREA: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; background: #0b1220; color: #d1fae5; border: 1px solid #374151; border-radius: 8px; font-family: monospace; font-size: 12px; resize: vertical;";

// The agent-type tokens the backend understands for `add_mail_domain`.
const AGENT_TYPES: &[(&str, &str)] = &[
    ("person", "Person"),
    ("org", "Organisation"),
    ("ai", "AI agent"),
    ("service", "Service"),
    ("content", "Content"),
    ("group", "Group"),
];

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

#[component]
pub fn DomainsPane() -> Element {
    // Raw JSON responses held as Values; string fields read defensively via s(), and the top-level
    // list responses read directly with `.as_array()`.
    let domains = use_signal(|| serde_json::Value::Array(vec![]));
    let presets = use_signal(|| serde_json::Value::Array(vec![]));
    let addresses = use_signal(|| serde_json::Value::Array(vec![]));
    let forms = use_signal(|| serde_json::Value::Null);
    let selected = use_signal(String::new); // selected domain name
    let status = use_signal(String::new);

    // Add-domain form fields.
    let new_name = use_signal(String::new);
    let new_agent = use_signal(|| "person".to_string());
    let new_did = use_signal(String::new);
    let new_label = use_signal(String::new);
    let new_parent = use_signal(String::new);

    // Mint controls.
    let preset_local = use_signal(String::new); // chosen preset's local part
    let rel_local = use_signal(String::new);
    let rel_did = use_signal(String::new);

    // Collapsible front-door forms.
    let show_turtle = use_signal(|| false);
    let show_jsonld = use_signal(|| false);

    // Cloudflare easy-install + self-hosting serve.
    let cf_token = use_signal(String::new);
    let cf_account_id = use_signal(String::new);
    let github_token = use_signal(String::new);
    let github_repo = use_signal(String::new);
    let cf_status = use_signal(String::new);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &domains, &presets, &addresses, &forms, &selected, &status, &new_name, &new_agent,
            &new_did, &new_label, &new_parent, &preset_local, &rel_local, &rel_did, &show_turtle,
            &show_jsonld, &cf_token, &cf_account_id, &github_token, &github_repo, &cf_status,
        );
    }

    // Load domains + purpose-inbox presets on mount.
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let (mut domains, mut presets, mut status) = (domains, presets, status);
            spawn(async move {
                match invoke_json::<serde_json::Value>("list_mail_domains", json!({})).await {
                    Ok(v) => domains.set(v),
                    Err(e) => status.set(format!("Load domains failed: {e}")),
                }
                match invoke_json::<serde_json::Value>("purpose_inbox_presets", json!({})).await {
                    Ok(v) => presets.set(v),
                    Err(e) => status.set(format!("Load presets failed: {e}")),
                }
            });
        }
    });

    let domain_list = domains().as_array().cloned().unwrap_or_default();
    let preset_list = presets().as_array().cloned().unwrap_or_default();
    let address_list = addresses().as_array().cloned().unwrap_or_default();
    let sel = selected();
    let f = forms();

    rsx! {
        div { style: "padding: 18px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 1100px; margin: 0 auto;",
                h2 { style: "color: #a78bfa; margin: 0 0 4px; font-size: 24px;", "Domains & Mail" }
                p { style: "color: #9ca3af; margin: 0 0 12px; font-size: 13px;",
                    "Your mail domains and the addresses under them. A domain points at a front-door DID via a single DNS TXT record — paste it at your registrar and you're reachable, with no server to host. Purpose inboxes carry their own rules; relationship addresses are pairwise, so each correspondent gets their own, and any address can be silenced without being deleted."
                }

                if !status().is_empty() {
                    div { style: "background: #3b0b0b; border: 1px solid #ef4444; color: #fecaca; padding: 8px 12px; border-radius: 8px; margin-bottom: 12px; font-size: 13px;", "{status}" }
                }

                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 16px; align-items: start;",

                    // ── LEFT: domains + selected domain's addresses ─────────────
                    div {

                        // Domain list.
                        div { style: "{PANEL} margin-bottom: 12px;",
                            div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;", "Domains" }
                            if domain_list.is_empty() {
                                div { style: "color: #6b7280; font-size: 13px; margin-bottom: 8px;", "No domains yet — add one below." }
                            }
                            for d in domain_list.clone() {
                                {
                                    let dname = s(&d, "name");
                                    let dlabel = s(&d, "label");
                                    let dtype = s(&d, "agent_type");
                                    let dparent = s(&d, "parent");
                                    let is_sel = dname == sel;
                                    let dname_click = dname.clone();
                                    rsx! {
                                        div {
                                            style: if is_sel { "display: flex; justify-content: space-between; align-items: center; padding: 8px 10px; margin-bottom: 4px; border-radius: 8px; cursor: pointer; background: rgba(139,92,246,0.18); border: 1px solid #6d28d9;" } else { "display: flex; justify-content: space-between; align-items: center; padding: 8px 10px; margin-bottom: 4px; border-radius: 8px; cursor: pointer; background: #0f172a; border: 1px solid #1f2937;" },
                                            onclick: move |_| {
                                                let mut selr = selected; selr.set(dname_click.clone());
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let dpick = dname_click.clone();
                                                    let (mut addresses, mut forms, mut status) = (addresses, forms, status);
                                                    spawn(async move {
                                                        match invoke_json::<serde_json::Value>("list_mail_addresses", json!({ "domain": dpick })).await {
                                                            Ok(v) => addresses.set(v),
                                                            Err(e) => status.set(format!("Load addresses failed: {e}")),
                                                        }
                                                        match invoke_json::<serde_json::Value>("front_door_forms", json!({ "domain": dpick })).await {
                                                            Ok(v) => forms.set(v),
                                                            Err(e) => status.set(format!("Load front-door forms failed: {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            div {
                                                span { style: "font-weight: 700; color: #f3f4f6; font-size: 14px; font-family: monospace;", "{dname}" }
                                                if !dlabel.is_empty() {
                                                    span { style: "color: #9ca3af; font-size: 12px; margin-left: 8px;", "{dlabel}" }
                                                }
                                                if !dparent.is_empty() {
                                                    div { style: "color: #6b7280; font-size: 11px; margin-top: 2px;", "↳ under {dparent}" }
                                                }
                                            }
                                            if !dtype.is_empty() {
                                                span { style: "{CHIP} color: #7dd3fc;", "{dtype}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Add-domain form.
                        div { style: "{PANEL} margin-bottom: 12px;",
                            div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;", "Add domain" }
                            input {
                                style: "{INPUT} margin-bottom: 6px; font-size: 13px;",
                                placeholder: "name — e.g. example.com", value: "{new_name}",
                                oninput: move |e| { let mut n = new_name; n.set(e.value()); }
                            }
                            select {
                                style: "{INPUT} margin-bottom: 6px; font-size: 13px;",
                                value: "{new_agent}",
                                onchange: move |e| { let mut a = new_agent; a.set(e.value()); },
                                for (tok, label) in AGENT_TYPES.iter() {
                                    option { value: "{tok}", "{label}" }
                                }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 6px; font-size: 13px; font-family: monospace;",
                                placeholder: "front-door DID — did:…", value: "{new_did}",
                                oninput: move |e| { let mut n = new_did; n.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 6px; font-size: 13px;",
                                placeholder: "label (optional)", value: "{new_label}",
                                oninput: move |e| { let mut n = new_label; n.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 8px; font-size: 13px;",
                                placeholder: "parent domain (optional)", value: "{new_parent}",
                                oninput: move |e| { let mut n = new_parent; n.set(e.value()); }
                            }
                            button {
                                style: "{BTN} width: 100%; font-size: 13px;",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (new_name, new_agent, new_did, new_label, new_parent, mut domains, mut status) =
                                            (new_name, new_agent, new_did, new_label, new_parent, domains, status);
                                        spawn(async move {
                                            let name = new_name().trim().to_string();
                                            if name.is_empty() { status.set("Domain name is required.".into()); return; }
                                            let args = json!({
                                                "name": name,
                                                "agentType": new_agent(),
                                                "frontDoorDid": new_did().trim(),
                                                "label": new_label().trim(),
                                                "parent": new_parent().trim(),
                                            });
                                            match invoke_json::<serde_json::Value>("add_mail_domain", args).await {
                                                Ok(v) => {
                                                    domains.set(v);
                                                    let mut n = new_name; n.set(String::new());
                                                    let mut d = new_did; d.set(String::new());
                                                    let mut l = new_label; l.set(String::new());
                                                    let mut p = new_parent; p.set(String::new());
                                                }
                                                Err(e) => status.set(format!("Add domain failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "＋ Add domain"
                            }
                        }

                        // Selected domain's addresses + mint controls.
                        if !sel.is_empty() {
                            div { style: "{PANEL}",
                                div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                                    "Addresses · {sel}"
                                }
                                if address_list.is_empty() {
                                    div { style: "color: #6b7280; font-size: 13px; margin-bottom: 8px;", "No addresses yet — mint one below." }
                                }
                                for a in address_list.clone() {
                                    {
                                        let addr = s(&a, "address");
                                        let local_part = s(&a, "local_part");
                                        let kind = s(&a, "kind");
                                        let rel = s(&a, "relationship_did");
                                        let enabled = a.get("enabled").and_then(|x| x.as_bool()).unwrap_or(false);
                                        #[cfg(target_arch = "wasm32")]
                                        let addr_toggle = addr.clone();
                                        rsx! {
                                            div { style: "display: flex; justify-content: space-between; align-items: center; padding: 8px 10px; margin-bottom: 4px; border-radius: 8px; background: #0f172a; border: 1px solid #1f2937;",
                                                div {
                                                    div {
                                                        span { style: "font-weight: 700; color: #f3f4f6; font-size: 13px; font-family: monospace;", "{addr}" }
                                                        if !kind.is_empty() {
                                                            span { style: "{CHIP} color: #86efac; margin-left: 6px;", "{kind}" }
                                                        }
                                                    }
                                                    if !local_part.is_empty() {
                                                        div { style: "color: #6b7280; font-size: 11px; margin-top: 2px;", "local: {local_part}" }
                                                    }
                                                    if !rel.is_empty() {
                                                        div { style: "color: #6b7280; font-size: 11px; font-family: monospace; margin-top: 2px; word-break: break-all;", "↔ {rel}" }
                                                    }
                                                }
                                                button {
                                                    style: if enabled { format!("{BTN_MUTED} background: #065f46; color: #d1fae5;") } else { format!("{BTN_MUTED} background: #7f1d1d; color: #fecaca;") },
                                                    onclick: move |_| {
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            let want = !enabled;
                                                            let (addr_toggle, mut addresses, mut status) = (addr_toggle.clone(), addresses, status);
                                                            spawn(async move {
                                                                match invoke_json::<serde_json::Value>("set_mail_address_enabled", json!({ "address": addr_toggle, "enabled": want })).await {
                                                                    Ok(v) => addresses.set(v),
                                                                    Err(e) => status.set(format!("Toggle address failed: {e}")),
                                                                }
                                                            });
                                                        }
                                                    },
                                                    if enabled { "Enabled" } else { "Disabled" }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Mint purpose inbox.
                                div { style: "border-top: 1px solid #374151; padding-top: 10px; margin-top: 8px;",
                                    div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;", "Mint purpose inbox" }
                                    div { style: "display: flex; gap: 6px; align-items: center;",
                                        select {
                                            style: "{INPUT} font-size: 12px;",
                                            value: "{preset_local}",
                                            onchange: move |e| { let mut p = preset_local; p.set(e.value()); },
                                            option { value: "", "Choose a preset…" }
                                            for p in preset_list.clone() {
                                                {
                                                    let plocal = s(&p, "local");
                                                    let plabel = s(&p, "label");
                                                    rsx! { option { value: "{plocal}", "{plabel} ({plocal})" } }
                                                }
                                            }
                                        }
                                        button {
                                            style: "{BTN} font-size: 12px; white-space: nowrap;",
                                            onclick: move |_| {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let chosen = preset_local();
                                                    if chosen.is_empty() { return; }
                                                    // Find the chosen preset to serialize its rules.
                                                    let rules_json = preset_list.iter()
                                                        .find(|p| s(p, "local") == chosen)
                                                        .and_then(|p| p.get("rules"))
                                                        .map(|r| serde_json::to_string(r).unwrap_or_default())
                                                        .unwrap_or_default();
                                                    let dom = selected();
                                                    let (mut addresses, mut status) = (addresses, status);
                                                    spawn(async move {
                                                        match invoke_json::<serde_json::Value>("mint_purpose_inbox", json!({ "domain": dom.clone(), "local": chosen, "rulesJson": rules_json })).await {
                                                            Ok(_) => {
                                                                if let Ok(v) = invoke_json::<serde_json::Value>("list_mail_addresses", json!({ "domain": dom })).await { addresses.set(v); }
                                                            }
                                                            Err(e) => status.set(format!("Mint purpose inbox failed: {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            "Mint"
                                        }
                                    }
                                }

                                // Mint relationship address.
                                div { style: "border-top: 1px solid #374151; padding-top: 10px; margin-top: 10px;",
                                    div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;", "Mint relationship address" }
                                    input {
                                        style: "{INPUT} margin-bottom: 6px; font-size: 12px;",
                                        placeholder: "local part — e.g. alice", value: "{rel_local}",
                                        oninput: move |e| { let mut n = rel_local; n.set(e.value()); }
                                    }
                                    input {
                                        style: "{INPUT} margin-bottom: 6px; font-size: 12px; font-family: monospace;",
                                        placeholder: "relationship DID — did:…", value: "{rel_did}",
                                        oninput: move |e| { let mut n = rel_did; n.set(e.value()); }
                                    }
                                    button {
                                        style: "{BTN} width: 100%; font-size: 12px;",
                                        onclick: move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let dom = selected();
                                                let (rel_local, rel_did, mut addresses, mut status) = (rel_local, rel_did, addresses, status);
                                                spawn(async move {
                                                    let local = rel_local().trim().to_string();
                                                    let did = rel_did().trim().to_string();
                                                    if local.is_empty() || did.is_empty() { status.set("Local part and relationship DID are required.".into()); return; }
                                                    match invoke_json::<serde_json::Value>("mint_relationship_address", json!({ "domain": dom.clone(), "local": local, "relationshipDid": did })).await {
                                                        Ok(_) => {
                                                            let mut l = rel_local; l.set(String::new());
                                                            let mut d = rel_did; d.set(String::new());
                                                            if let Ok(v) = invoke_json::<serde_json::Value>("list_mail_addresses", json!({ "domain": dom })).await { addresses.set(v); }
                                                        }
                                                        Err(e) => status.set(format!("Mint relationship address failed: {e}")),
                                                    }
                                                });
                                            }
                                        },
                                        "Mint relationship address"
                                    }
                                }
                            }
                        }
                    }

                    // ── RIGHT: front-door forms for the selected domain ─────────
                    div { style: "{PANEL}",
                        div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;", "Front-door forms" }
                        if sel.is_empty() {
                            div { style: "color: #6b7280; font-size: 13px;", "Select a domain to see the DNS record and provenance forms that make it reachable." }
                        } else if f.is_null() {
                            div { style: "color: #6b7280; font-size: 13px;", "No front-door forms for {sel} yet." }
                        } else {
                            {
                                let dns_name = s(&f, "dns_name");
                                let dns_txt = s(&f, "dns_txt");
                                let turtle = s(&f, "turtle");
                                let jsonld = s(&f, "jsonld");
                                rsx! {
                                    div { style: "color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "DNS name" }
                                    div { style: "font-family: monospace; font-size: 13px; color: #f3f4f6; background: #0f172a; border: 1px solid #1f2937; border-radius: 8px; padding: 8px 10px; margin-bottom: 12px; word-break: break-all;", "{dns_name}" }

                                    div { style: "color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "TXT record" }
                                    textarea {
                                        style: "{TEXTAREA} height: 90px; margin-bottom: 4px;",
                                        readonly: true,
                                        value: "{dns_txt}"
                                    }
                                    div { style: "color: #86efac; font-size: 12px; margin-bottom: 14px;",
                                        "Add this TXT record at your registrar/Cloudflare — no hosting needed."
                                    }

                                    // Collapsible Turtle.
                                    button {
                                        style: "{BTN_MUTED} width: 100%; margin-bottom: 6px; text-align: left;",
                                        onclick: move |_| { let mut t = show_turtle; let now = t(); t.set(!now); },
                                        if show_turtle() { "▾ Turtle" } else { "▸ Turtle" }
                                    }
                                    if show_turtle() {
                                        textarea {
                                            style: "{TEXTAREA} height: 160px; margin-bottom: 12px;",
                                            readonly: true,
                                            value: "{turtle}"
                                        }
                                    }

                                    // Collapsible JSON-LD.
                                    button {
                                        style: "{BTN_MUTED} width: 100%; margin-bottom: 6px; text-align: left;",
                                        onclick: move |_| { let mut j = show_jsonld; let now = j(); j.set(!now); },
                                        if show_jsonld() { "▾ JSON-LD" } else { "▸ JSON-LD" }
                                    }
                                    if show_jsonld() {
                                        textarea {
                                            style: "{TEXTAREA} height: 160px;",
                                            readonly: true,
                                            value: "{jsonld}"
                                        }
                                    }

                                    // Cloudflare easy-install (just paste an API token) + self-host serve.
                                    div { style: "border-top: 1px solid #374151; margin-top: 14px; padding-top: 12px;",
                                        div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;", "Publish via Cloudflare (optional — just an API token)" }
                                        input {
                                            style: "{INPUT} font-size: 12px; font-family: monospace; margin-bottom: 4px;",
                                            placeholder: "Cloudflare API token", value: "{cf_token}",
                                            oninput: move |e| { let mut t = cf_token; t.set(e.value()); }
                                        }
                                        input {
                                            style: "{INPUT} font-size: 12px; font-family: monospace; margin-bottom: 4px;",
                                            placeholder: "Cloudflare Account ID", value: "{cf_account_id}",
                                            oninput: move |e| { let mut t = cf_account_id; t.set(e.value()); }
                                        }
                                        button {
                                            style: "{BTN} width: 100%; font-size: 12px; margin-top: 4px;",
                                            onclick: move |_| {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let (cf_token, mut cf_status) = (cf_token, cf_status);
                                                    let dom = selected();
                                                    spawn(async move {
                                                        let token = cf_token();
                                                        if token.trim().is_empty() { cf_status.set("Paste a Cloudflare API token first.".into()); return; }
                                                        cf_status.set("Verifying token…".into());
                                                        if let Err(e) = invoke_json::<serde_json::Value>("cf_verify_token", json!({ "token": token })).await { cf_status.set(format!("Token invalid: {e}")); return; }
                                                        let zones = match invoke_json::<serde_json::Value>("cf_list_zones", json!({ "token": token })).await { Ok(z) => z, Err(e) => { cf_status.set(format!("List zones failed: {e}")); return; } };
                                                        let zone_id = zones.as_array().and_then(|zs| zs.iter().find(|z| { let n = s(z, "name"); !n.is_empty() && dom.ends_with(&n) }).map(|z| s(z, "id")));
                                                        let Some(zone_id) = zone_id else { cf_status.set("No matching Cloudflare zone for this domain.".into()); return; };
                                                        match invoke_json::<serde_json::Value>("cf_publish_front_door", json!({ "token": token, "zoneId": zone_id, "domain": dom })).await {
                                                            Ok(_) => cf_status.set("Published the _qdp front-door record to Cloudflare ✓".into()),
                                                            Err(e) => cf_status.set(format!("Publish failed: {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            "Publish _qdp record to Cloudflare"
                                        }
                                        button {
                                            style: "{BTN} width: 100%; font-size: 12px; margin-top: 6px; background: #0ea5e9;",
                                            onclick: move |_| {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let (cf_token, cf_account_id, mut cf_status) = (cf_token, cf_account_id, cf_status);
                                                    let dom = selected();
                                                    spawn(async move {
                                                        let token = cf_token();
                                                        let account = cf_account_id();
                                                        if token.trim().is_empty() || account.trim().is_empty() { 
                                                            cf_status.set("Paste Cloudflare API token and Account ID first.".into()); return; 
                                                        }
                                                        cf_status.set("Verifying token and fetching zones…".into());
                                                        if let Err(e) = invoke_json::<serde_json::Value>("cf_verify_token", json!({ "token": token })).await { cf_status.set(format!("Token invalid: {e}")); return; }
                                                        let zones = match invoke_json::<serde_json::Value>("cf_list_zones", json!({ "token": token })).await { Ok(z) => z, Err(e) => { cf_status.set(format!("List zones failed: {e}")); return; } };
                                                        let zone_id = zones.as_array().and_then(|zs| zs.iter().find(|z| { let n = s(z, "name"); !n.is_empty() && dom.ends_with(&n) }).map(|z| s(z, "id")));
                                                        let Some(zone_id) = zone_id else { cf_status.set("No matching Cloudflare zone for this domain.".into()); return; };
                                                        
                                                        cf_status.set("Deploying full node infrastructure (R2 + Worker + Tunnel)…".into());
                                                        match invoke_json::<serde_json::Value>("cf_deploy_infrastructure", json!({ "token": token, "accountId": account, "zoneId": zone_id, "domain": dom })).await {
                                                            Ok(_) => cf_status.set("Provisioned full node infrastructure successfully! ✓".into()),
                                                            Err(e) => cf_status.set(format!("Deployment failed: {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            "Provision Full Node (Worker + R2 + Tunnel)"
                                        }
                                        div { style: "border-top: 1px solid #374151; margin-top: 14px; padding-top: 12px;",
                                            div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;", "Publish Static Site (GitHub + CF Pages)" }
                                            input {
                                                style: "{INPUT} font-size: 12px; font-family: monospace; margin-bottom: 4px;",
                                                placeholder: "GitHub Personal Access Token", value: "{github_token}",
                                                oninput: move |e| { let mut t = github_token; t.set(e.value()); }
                                            }
                                            input {
                                                style: "{INPUT} font-size: 12px; font-family: monospace; margin-bottom: 4px;",
                                                placeholder: "GitHub Repository Name (e.g. my-site)", value: "{github_repo}",
                                                oninput: move |e| { let mut t = github_repo; t.set(e.value()); }
                                            }
                                            button {
                                                style: "{BTN} width: 100%; font-size: 12px; margin-top: 4px; background: #8b5cf6;",
                                                onclick: move |_| {
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        let (gh_token, gh_repo, cf_token, cf_account, mut cf_status) = (github_token, github_repo, cf_token, cf_account_id, cf_status);
                                                        spawn(async move {
                                                            let gh_t = gh_token();
                                                            let gh_r = gh_repo();
                                                            let cf_t = cf_token();
                                                            let cf_a = cf_account();
                                                            if gh_t.trim().is_empty() || gh_r.trim().is_empty() || cf_t.trim().is_empty() || cf_a.trim().is_empty() { 
                                                                cf_status.set("Fill GitHub Token, Repo Name, Cloudflare Token, and Account ID first.".into()); return; 
                                                            }
                                                            
                                                            cf_status.set("Deploying static site to GitHub and CF Pages...".into());
                                                            match invoke_json::<serde_json::Value>("deploy_static_site_cf_pages", json!({ "githubToken": gh_t, "githubRepo": gh_r, "cfToken": cf_t, "cfAccount": cf_a })).await {
                                                                Ok(res) => cf_status.set(format!("Deployed successfully to {} ✓", res["cf_project"].as_str().unwrap_or(""))),
                                                                Err(e) => cf_status.set(format!("Deployment failed: {e}")),
                                                            }
                                                        });
                                                    }
                                                },
                                                "Publish Static Site to CF Pages"
                                            }
                                        }
                                        button {
                                            style: "{BTN_MUTED} width: 100%; font-size: 12px; margin-top: 6px;",
                                            onclick: move |_| {
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let mut cf_status = cf_status;
                                                    let dom = selected();
                                                    spawn(async move {
                                                        match invoke_json::<serde_json::Value>("start_qdp_server", json!({ "domain": dom, "bindAddr": "127.0.0.1:8765" })).await {
                                                            Ok(_) => cf_status.set("Serving /.well-known/QDP on 127.0.0.1:8765 (bind to your overlay for peers).".into()),
                                                            Err(e) => cf_status.set(format!("Serve failed: {e}")),
                                                        }
                                                    });
                                                }
                                            },
                                            "Serve /.well-known/QDP locally"
                                        }
                                        if !cf_status().is_empty() {
                                            div { style: "color: #9ca3af; font-size: 12px; margin-top: 6px;", "{cf_status}" }
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
