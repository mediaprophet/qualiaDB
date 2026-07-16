//! **Domains & Mail** — domain identity, purpose/relationship mailboxes, **local inbox**, and a
//! **local SMTP receiver** so mail actually lands here (not a paid half-stack).
//!
//! Product path: register domain → onboard purpose inboxes + catchall → start local receiver →
//! paste MX/SPF (and QDP TXT) at the registrar / tunnel public host → messages appear in the inbox
//! with semantic rules applied. Optional classic SMTP/IMAP remains for send/import only.
//!
//! Backend Tauri commands (camelCase): domain CRUD + `onboard_mail_domain`, `mail_list` /
//! `mail_accept` / `mail_receiver_*` / `mail_dns_forms`, plus optional transport.

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

    // Delivery probe + classic SMTP/IMAP transport (optional — BYO provider after domain setup).
    let probe_to = use_signal(String::new);
    let probe_result = use_signal(String::new);
    let smtp_host = use_signal(String::new);
    let smtp_port = use_signal(|| "587".to_string());
    let smtp_user = use_signal(String::new);
    let smtp_pass = use_signal(String::new);
    let imap_host = use_signal(String::new);
    let imap_port = use_signal(|| "993".to_string());
    let imap_user = use_signal(String::new);
    let imap_pass = use_signal(String::new);
    let compose_from = use_signal(String::new);
    let compose_to = use_signal(String::new);
    let compose_subject = use_signal(String::new);
    let compose_body = use_signal(String::new);
    let fetch_mailbox = use_signal(|| "INBOX".to_string());
    let fetch_out = use_signal(String::new);
    let transport_status = use_signal(String::new);

    // Local product inbox + SMTP receiver.
    let inbox = use_signal(|| serde_json::Value::Array(vec![]));
    let inbox_counts = use_signal(String::new);
    let selected_mail = use_signal(String::new);
    let mail_body = use_signal(String::new);
    let receiver_status = use_signal(String::new);
    let receiver_bind = use_signal(|| "127.0.0.1:2525".to_string());
    let mail_dns_block = use_signal(String::new);
    let test_from = use_signal(|| "friend@elsewhere.example".to_string());
    let test_to = use_signal(String::new);
    let test_subject = use_signal(|| "Hello from a real inbox path".to_string());
    let test_body = use_signal(|| "If you can read this in Talk → Mail, domain mail works.".to_string());
    let show_quarantine = use_signal(|| true);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &domains, &presets, &addresses, &forms, &selected, &status, &new_name, &new_agent,
            &new_did, &new_label, &new_parent, &preset_local, &rel_local, &rel_did, &show_turtle,
            &show_jsonld, &cf_token, &cf_account_id, &github_token, &github_repo, &cf_status,
            &probe_to, &probe_result, &smtp_host, &smtp_port, &smtp_user, &smtp_pass,
            &imap_host, &imap_port, &imap_user, &imap_pass, &compose_from, &compose_to,
            &compose_subject, &compose_body, &fetch_mailbox, &fetch_out, &transport_status,
            &inbox, &inbox_counts, &selected_mail, &mail_body, &receiver_status, &receiver_bind,
            &mail_dns_block, &test_from, &test_to, &test_subject, &test_body, &show_quarantine,
        );
    }

    // Load domains + presets + transport + inbox + receiver on mount.
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let (mut domains, mut presets, mut status) = (domains, presets, status);
            let (
                mut smtp_host,
                mut smtp_port,
                mut smtp_user,
                mut smtp_pass,
                mut imap_host,
                mut imap_port,
                mut imap_user,
                mut imap_pass,
                mut inbox,
                mut inbox_counts,
                mut receiver_status,
                mut receiver_bind,
            ) = (
                smtp_host, smtp_port, smtp_user, smtp_pass, imap_host, imap_port, imap_user,
                imap_pass, inbox, inbox_counts, receiver_status, receiver_bind,
            );
            spawn(async move {
                match invoke_json::<serde_json::Value>("list_mail_domains", json!({})).await {
                    Ok(v) => domains.set(v),
                    Err(e) => status.set(format!("Load domains failed: {e}")),
                }
                match invoke_json::<serde_json::Value>("purpose_inbox_presets", json!({})).await {
                    Ok(v) => presets.set(v),
                    Err(e) => status.set(format!("Load presets failed: {e}")),
                }
                if let Ok(cfg) =
                    invoke_json::<serde_json::Value>("load_mail_transport_config", json!({})).await
                {
                    if let Some(smtp) = cfg.get("smtp") {
                        if !smtp.is_null() {
                            smtp_host.set(s(smtp, "host"));
                            if let Some(p) = smtp.get("port").and_then(|x| x.as_u64()) {
                                smtp_port.set(p.to_string());
                            }
                            smtp_user.set(s(smtp, "username"));
                            smtp_pass.set(s(smtp, "password"));
                        }
                    }
                    if let Some(imap) = cfg.get("imap") {
                        if !imap.is_null() {
                            imap_host.set(s(imap, "host"));
                            if let Some(p) = imap.get("port").and_then(|x| x.as_u64()) {
                                imap_port.set(p.to_string());
                            }
                            imap_user.set(s(imap, "username"));
                            imap_pass.set(s(imap, "password"));
                        }
                    }
                }
                if let Ok(v) = invoke_json::<serde_json::Value>(
                    "mail_list",
                    json!({ "includeQuarantine": true }),
                )
                .await
                {
                    if let Some(arr) = v.get("messages") {
                        inbox.set(arr.clone());
                    }
                    if let Some(c) = v.get("counts") {
                        inbox_counts.set(format!(
                            "{} total · {} unread · {} quarantine",
                            c.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
                            c.get("unread").and_then(|x| x.as_u64()).unwrap_or(0),
                            c.get("quarantine").and_then(|x| x.as_u64()).unwrap_or(0),
                        ));
                    }
                }
                if let Ok(st) =
                    invoke_json::<serde_json::Value>("mail_receiver_status", json!({})).await
                {
                    let running = st.get("running").and_then(|x| x.as_bool()).unwrap_or(false);
                    let bind = s(&st, "bind");
                    let port = st.get("port").and_then(|x| x.as_u64()).unwrap_or(0);
                    if !bind.is_empty() {
                        receiver_bind.set(bind.clone());
                    }
                    receiver_status.set(if running {
                        format!("Receiver RUNNING on {bind} (port {port})")
                    } else {
                        "Receiver stopped — start it so mail can land here.".into()
                    });
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
                p { style: "color: #9ca3af; margin: 0 0 12px; font-size: 13px; line-height: 1.5;",
                    "This is your mail product: register a domain, mint purpose inboxes (and catchall), start the local SMTP receiver, paste MX/SPF at your registrar (tunnel/public host if you want the internet to reach you). Messages land in the inbox below with semantic rules — you are not paying a half-stack to host nothing."
                }

                // ── Local inbox (the product) ─────────────────────────────
                div { style: "{PANEL} margin-bottom: 16px; border: 1px solid #6d28d9;",
                    div { style: "display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 10px;",
                        div {
                            div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;",
                                "Inbox"
                            }
                            div { style: "color: #94a3b8; font-size: 12px; margin-top: 2px;",
                                if inbox_counts().is_empty() { "No messages yet." } else { "{inbox_counts}" }
                            }
                        }
                        div { style: "display: flex; gap: 6px; flex-wrap: wrap;",
                            button {
                                style: "{BTN_MUTED}",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (mut inbox, mut inbox_counts, mut status, show_quarantine) =
                                            (inbox, inbox_counts, status, show_quarantine);
                                        spawn(async move {
                                            match invoke_json::<serde_json::Value>(
                                                "mail_list",
                                                json!({ "includeQuarantine": show_quarantine() }),
                                            )
                                            .await
                                            {
                                                Ok(v) => {
                                                    if let Some(arr) = v.get("messages") {
                                                        inbox.set(arr.clone());
                                                    }
                                                    if let Some(c) = v.get("counts") {
                                                        inbox_counts.set(format!(
                                                            "{} total · {} unread · {} quarantine",
                                                            c.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
                                                            c.get("unread").and_then(|x| x.as_u64()).unwrap_or(0),
                                                            c.get("quarantine").and_then(|x| x.as_u64()).unwrap_or(0),
                                                        ));
                                                    }
                                                }
                                                Err(e) => status.set(format!("Refresh inbox failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "Refresh"
                            }
                            button {
                                style: "{BTN_MUTED}",
                                onclick: move |_| {
                                    let mut q = show_quarantine;
                                    let next = !q();
                                    q.set(next);
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (mut inbox, mut inbox_counts) = (inbox, inbox_counts);
                                        spawn(async move {
                                            if let Ok(v) = invoke_json::<serde_json::Value>(
                                                "mail_list",
                                                json!({ "includeQuarantine": next }),
                                            )
                                            .await
                                            {
                                                if let Some(arr) = v.get("messages") {
                                                    inbox.set(arr.clone());
                                                }
                                                if let Some(c) = v.get("counts") {
                                                    inbox_counts.set(format!(
                                                        "{} total · {} unread · {} quarantine",
                                                        c.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
                                                        c.get("unread").and_then(|x| x.as_u64()).unwrap_or(0),
                                                        c.get("quarantine").and_then(|x| x.as_u64()).unwrap_or(0),
                                                    ));
                                                }
                                            }
                                        });
                                    }
                                },
                                if show_quarantine() { "Hide quarantine" } else { "Show quarantine" }
                            }
                        }
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div { style: "max-height: 280px; overflow-y: auto;",
                            {
                                let msgs = inbox().as_array().cloned().unwrap_or_default();
                                if msgs.is_empty() {
                                    rsx! {
                                        div { style: "color: #6b7280; font-size: 13px; padding: 12px; background: #0f172a; border-radius: 8px;",
                                            "Empty. Start the receiver, send a test below, or wait for real SMTP delivery."
                                        }
                                    }
                                } else {
                                    rsx! {
                                        for m in msgs {
                                            {
                                                let id = s(&m, "id");
                                                let subj = s(&m, "subject");
                                                let from = s(&m, "from_address");
                                                let to = s(&m, "to_address");
                                                let via = s(&m, "via");
                                                let read = m.get("read").and_then(|x| x.as_bool()).unwrap_or(false);
                                                let q = m.get("quarantined").and_then(|x| x.as_bool()).unwrap_or(false);
                                                let id_click = id.clone();
                                                let is_sel = selected_mail() == id;
                                                rsx! {
                                                    div {
                                                        style: if is_sel {
                                                            "padding: 8px 10px; margin-bottom: 4px; border-radius: 8px; cursor: pointer; background: rgba(139,92,246,0.2); border: 1px solid #6d28d9;"
                                                        } else {
                                                            "padding: 8px 10px; margin-bottom: 4px; border-radius: 8px; cursor: pointer; background: #0f172a; border: 1px solid #1f2937;"
                                                        },
                                                        onclick: move |_| {
                                                            let id = id_click.clone();
                                                            let mut selected_mail = selected_mail;
                                                            let mut mail_body = mail_body;
                                                            selected_mail.set(id.clone());
                                                            #[cfg(target_arch = "wasm32")]
                                                            {
                                                                spawn(async move {
                                                                    if let Ok(v) = invoke_json::<serde_json::Value>(
                                                                        "mail_get",
                                                                        json!({ "id": id.clone() }),
                                                                    )
                                                                    .await
                                                                    {
                                                                        let body = s(&v, "body");
                                                                        let from = s(&v, "from_address");
                                                                        let to = s(&v, "to_address");
                                                                        let subj = s(&v, "subject");
                                                                        let reasons = v.get("reasons").cloned().unwrap_or(serde_json::json!([]));
                                                                        mail_body.set(format!(
                                                                            "From: {from}\nTo: {to}\nSubject: {subj}\n\n{body}\n\n— reasons —\n{}",
                                                                            serde_json::to_string_pretty(&reasons).unwrap_or_default()
                                                                        ));
                                                                        let _ = invoke_json::<serde_json::Value>(
                                                                            "mail_set_read",
                                                                            json!({ "id": id, "read": true }),
                                                                        )
                                                                        .await;
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        div { style: if read { "font-weight: 500; color: #cbd5e1; font-size: 13px;" } else { "font-weight: 700; color: #f3f4f6; font-size: 13px;" },
                                                            if subj.is_empty() { "(no subject)" } else { "{subj}" }
                                                        }
                                                        div { style: "color: #6b7280; font-size: 11px; margin-top: 2px; font-family: monospace;",
                                                            "{from} → {to}"
                                                        }
                                                        div {
                                                            span { style: "{CHIP}", "{via}" }
                                                            if q {
                                                                span { style: "{CHIP} color: #fca5a5;", "quarantine" }
                                                            }
                                                            if !read {
                                                                span { style: "{CHIP} color: #86efac;", "unread" }
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
                        div {
                            if mail_body().is_empty() {
                                div { style: "color: #6b7280; font-size: 13px;", "Select a message to read." }
                            } else {
                                textarea {
                                    style: "{TEXTAREA} height: 240px;",
                                    readonly: true,
                                    value: "{mail_body}"
                                }
                                if !selected_mail().is_empty() {
                                    button {
                                        style: "{BTN_MUTED} margin-top: 6px; background: #7f1d1d; color: #fecaca;",
                                        onclick: move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let id = selected_mail();
                                                let (mut selected_mail, mut mail_body, mut inbox, mut inbox_counts) =
                                                    (selected_mail, mail_body, inbox, inbox_counts);
                                                spawn(async move {
                                                    let _ = invoke_json::<serde_json::Value>(
                                                        "mail_delete",
                                                        json!({ "id": id }),
                                                    )
                                                    .await;
                                                    selected_mail.set(String::new());
                                                    mail_body.set(String::new());
                                                    if let Ok(v) = invoke_json::<serde_json::Value>(
                                                        "mail_list",
                                                        json!({ "includeQuarantine": true }),
                                                    )
                                                    .await
                                                    {
                                                        if let Some(arr) = v.get("messages") {
                                                            inbox.set(arr.clone());
                                                        }
                                                        if let Some(c) = v.get("counts") {
                                                            inbox_counts.set(format!(
                                                                "{} total · {} unread · {} quarantine",
                                                                c.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
                                                                c.get("unread").and_then(|x| x.as_u64()).unwrap_or(0),
                                                                c.get("quarantine").and_then(|x| x.as_u64()).unwrap_or(0),
                                                            ));
                                                        }
                                                    }
                                                });
                                            }
                                        },
                                        "Delete message"
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Local SMTP receiver + MX DNS ──────────────────────────
                div { style: "{PANEL} margin-bottom: 16px;",
                    div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                        "Local SMTP receiver (this is the mail host)"
                    }
                    p { style: "color: #9ca3af; font-size: 12px; margin: 0 0 8px; line-height: 1.45;",
                        "Starts an SMTP edge on this machine. Mail to your minted addresses is accepted, ruled, and stored above. For the public internet: set MX to a hostname that reaches this port (router forward or Cloudflare Tunnel). Port 25 is often blocked on residential links — tunnel 25→2525 or host the edge on a small VPS that forwards here."
                    }
                    div { style: "color: #86efac; font-size: 12px; margin-bottom: 8px;", "{receiver_status}" }
                    div { style: "display: flex; gap: 8px; flex-wrap: wrap; align-items: center; margin-bottom: 8px;",
                        input {
                            style: "{INPUT} flex: 1; min-width: 180px; font-size: 12px; font-family: monospace;",
                            placeholder: "bind — 127.0.0.1:2525 or 0.0.0.0:2525",
                            value: "{receiver_bind}",
                            oninput: move |e| { let mut b = receiver_bind; b.set(e.value()); }
                        }
                        button {
                            style: "{BTN} font-size: 12px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let bind = receiver_bind();
                                    let mut receiver_status = receiver_status;
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>(
                                            "mail_receiver_start",
                                            json!({ "bind": bind }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                let msg = v.get("message").and_then(|m| m.as_str())
                                                    .or_else(|| v.get("bind").and_then(|m| m.as_str()))
                                                    .unwrap_or("started");
                                                receiver_status.set(format!("Receiver RUNNING — {msg}"));
                                            }
                                            Err(e) => receiver_status.set(format!("Start failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Start receiver"
                        }
                        button {
                            style: "{BTN_MUTED} font-size: 12px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let mut receiver_status = receiver_status;
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>("mail_receiver_stop", json!({})).await {
                                            Ok(_) => receiver_status.set("Receiver stopped.".into()),
                                            Err(e) => receiver_status.set(format!("Stop failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Stop"
                        }
                    }
                    if !sel.is_empty() {
                        button {
                            style: "{BTN_MUTED} font-size: 12px; margin-bottom: 8px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let dom = selected();
                                    let mut mail_dns_block = mail_dns_block;
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>(
                                            "mail_dns_forms",
                                            json!({ "domain": dom }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                let block = v.get("plaintext_block")
                                                    .and_then(|b| b.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                mail_dns_block.set(block);
                                            }
                                            Err(e) => mail_dns_block.set(format!("DNS forms failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Show MX / SPF paste block for {sel}"
                        }
                    }
                    if !mail_dns_block().is_empty() {
                        textarea {
                            style: "{TEXTAREA} height: 110px; margin-bottom: 8px;",
                            readonly: true,
                            value: "{mail_dns_block}"
                        }
                    }
                    div { style: "border-top: 1px solid #374151; padding-top: 10px; margin-top: 4px;",
                        div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;",
                            "Send test into the product path (no external provider)"
                        }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                            placeholder: "from", value: "{test_from}",
                            oninput: move |e| { let mut t = test_from; t.set(e.value()); }
                        }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                            placeholder: if sel.is_empty() { "to — e.g. frontdoor@your.domain".to_string() } else { format!("to — e.g. frontdoor@{sel}") },
                            value: "{test_to}",
                            oninput: move |e| { let mut t = test_to; t.set(e.value()); }
                        }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                            placeholder: "subject", value: "{test_subject}",
                            oninput: move |e| { let mut t = test_subject; t.set(e.value()); }
                        }
                        textarea {
                            style: "{TEXTAREA} height: 60px; margin-bottom: 6px;",
                            value: "{test_body}",
                            oninput: move |e| { let mut t = test_body; t.set(e.value()); }
                        }
                        button {
                            style: "{BTN} font-size: 12px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (test_from, test_to, test_subject, test_body, mut status, mut inbox, mut inbox_counts) =
                                        (test_from, test_to, test_subject, test_body, status, inbox, inbox_counts);
                                    let sel_dom = selected();
                                    spawn(async move {
                                        let mut to = test_to().trim().to_string();
                                        if to.is_empty() && !sel_dom.is_empty() {
                                            to = format!("frontdoor@{sel_dom}");
                                        }
                                        if to.is_empty() {
                                            status.set("Set a to-address (mint mailboxes first).".into());
                                            return;
                                        }
                                        match invoke_json::<serde_json::Value>(
                                            "mail_accept",
                                            json!({
                                                "from": test_from().trim(),
                                                "to": to,
                                                "subject": test_subject(),
                                                "body": test_body(),
                                                "senderVerified": false,
                                            }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                let ok = v.get("accepted").and_then(|x| x.as_bool()).unwrap_or(false);
                                                if ok {
                                                    status.set("Accepted into local inbox.".into());
                                                    if let Ok(list) = invoke_json::<serde_json::Value>(
                                                        "mail_list",
                                                        json!({ "includeQuarantine": true }),
                                                    )
                                                    .await
                                                    {
                                                        if let Some(arr) = list.get("messages") {
                                                            inbox.set(arr.clone());
                                                        }
                                                        if let Some(c) = list.get("counts") {
                                                            inbox_counts.set(format!(
                                                                "{} total · {} unread · {} quarantine",
                                                                c.get("total").and_then(|x| x.as_u64()).unwrap_or(0),
                                                                c.get("unread").and_then(|x| x.as_u64()).unwrap_or(0),
                                                                c.get("quarantine").and_then(|x| x.as_u64()).unwrap_or(0),
                                                            ));
                                                        }
                                                    }
                                                } else {
                                                    let rej = v.get("rejected").and_then(|r| r.as_str()).unwrap_or("rejected");
                                                    status.set(format!("Not accepted: {rej}"));
                                                }
                                            }
                                            Err(e) => status.set(format!("Accept failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Deliver test message"
                        }
                    }
                }

                // Quick onboard: mint all purpose presets + catchall for selected domain.
                if !sel.is_empty() {
                    button {
                        style: "{BTN} margin-bottom: 12px;",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let dom = selected();
                                let (mut addresses, mut status) = (addresses, status);
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>(
                                        "onboard_mail_domain",
                                        json!({ "domain": dom.clone() }),
                                    )
                                    .await
                                    {
                                        Ok(v) => {
                                            let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("Onboarded.");
                                            status.set(msg.to_string());
                                            if let Ok(list) = invoke_json::<serde_json::Value>(
                                                "list_mail_addresses",
                                                json!({ "domain": dom }),
                                            )
                                            .await
                                            {
                                                addresses.set(list);
                                            }
                                        }
                                        Err(e) => status.set(format!("Onboard mail failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Onboard mail for {sel} (purpose inboxes + catchall)"
                    }
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
                                        let (
                                            new_name,
                                            new_agent,
                                            new_did,
                                            new_label,
                                            new_parent,
                                            mut domains,
                                            mut status,
                                            mut addresses,
                                            mut selected,
                                        ) = (
                                            new_name,
                                            new_agent,
                                            new_did,
                                            new_label,
                                            new_parent,
                                            domains,
                                            status,
                                            addresses,
                                            selected,
                                        );
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
                                                    // Auto-onboard purpose inboxes + catchall (same path as Reception).
                                                    let mail_msg = match invoke_json::<serde_json::Value>(
                                                        "onboard_mail_domain",
                                                        json!({ "domain": name.clone() }),
                                                    )
                                                    .await
                                                    {
                                                        Ok(r) => r
                                                            .get("message")
                                                            .and_then(|m| m.as_str())
                                                            .unwrap_or("Mail onboarded.")
                                                            .to_string(),
                                                        Err(e) => format!("Mail onboard skipped: {e}"),
                                                    };
                                                    selected.set(name.clone());
                                                    if let Ok(list) = invoke_json::<serde_json::Value>(
                                                        "list_mail_addresses",
                                                        json!({ "domain": name }),
                                                    )
                                                    .await
                                                    {
                                                        addresses.set(list);
                                                    }
                                                    status.set(format!("Domain added. {mail_msg}"));
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

                // ── Delivery probe (semantic routing / fail-closed wildcards) ──
                div { style: "{PANEL} margin-top: 16px;",
                    div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                        "Delivery resolve (semantic routing)"
                    }
                    p { style: "color: #9ca3af; font-size: 12px; margin: 0 0 8px;",
                        "Exact minted addresses deliver under their rules. Unknown locals reject unless catchall@ is onboarded (quarantine intake — not an open relay)."
                    }
                    div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap;",
                        input {
                            style: "{INPUT} flex: 1; min-width: 200px; font-size: 13px; font-family: monospace;",
                            placeholder: "to address — e.g. stranger@example.com",
                            value: "{probe_to}",
                            oninput: move |e| { let mut p = probe_to; p.set(e.value()); }
                        }
                        button {
                            style: "{BTN} font-size: 12px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let to = probe_to();
                                    let (mut probe_result, mut status) = (probe_result, status);
                                    spawn(async move {
                                        if to.trim().is_empty() {
                                            probe_result.set("Enter a to-address.".into());
                                            return;
                                        }
                                        match invoke_json::<serde_json::Value>(
                                            "resolve_mail_delivery",
                                            json!({ "toAddress": to.trim() }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                probe_result.set(
                                                    serde_json::to_string_pretty(&v)
                                                        .unwrap_or_else(|_| v.to_string()),
                                                );
                                            }
                                            Err(e) => {
                                                probe_result.set(String::new());
                                                status.set(format!("Resolve failed: {e}"));
                                            }
                                        }
                                    });
                                }
                            },
                            "Resolve delivery"
                        }
                    }
                    if !probe_result().is_empty() {
                        textarea {
                            style: "{TEXTAREA} height: 120px; margin-top: 8px;",
                            readonly: true,
                            value: "{probe_result}"
                        }
                    }
                }

                // ── Optional: external send / IMAP import ──────────────────
                div { style: "{PANEL} margin-top: 16px;",
                    div { style: "color: #e5e7eb; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 8px;",
                        "Optional: external SMTP send / IMAP import"
                    }
                    p { style: "color: #9ca3af; font-size: 12px; margin: 0 0 10px;",
                        "Not required for receiving. Use only if you want to send via a submission host or import old mail from another provider into the local inbox above."
                    }
                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                        div {
                            div { style: "color: #a5b4fc; font-size: 11px; font-weight: 700; margin-bottom: 6px;", "SMTP (send)" }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                placeholder: "host", value: "{smtp_host}",
                                oninput: move |e| { let mut h = smtp_host; h.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                placeholder: "port (587)", value: "{smtp_port}",
                                oninput: move |e| { let mut p = smtp_port; p.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                                placeholder: "username", value: "{smtp_user}",
                                oninput: move |e| { let mut u = smtp_user; u.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                r#type: "password",
                                placeholder: "password / app password", value: "{smtp_pass}",
                                oninput: move |e| { let mut p = smtp_pass; p.set(e.value()); }
                            }
                        }
                        div {
                            div { style: "color: #a5b4fc; font-size: 11px; font-weight: 700; margin-bottom: 6px;", "IMAP (fetch)" }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                placeholder: "host", value: "{imap_host}",
                                oninput: move |e| { let mut h = imap_host; h.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                placeholder: "port (993)", value: "{imap_port}",
                                oninput: move |e| { let mut p = imap_port; p.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                                placeholder: "username", value: "{imap_user}",
                                oninput: move |e| { let mut u = imap_user; u.set(e.value()); }
                            }
                            input {
                                style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                                r#type: "password",
                                placeholder: "password / app password", value: "{imap_pass}",
                                oninput: move |e| { let mut p = imap_pass; p.set(e.value()); }
                            }
                        }
                    }
                    button {
                        style: "{BTN} margin-top: 8px; font-size: 12px;",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (
                                    smtp_host,
                                    smtp_port,
                                    smtp_user,
                                    smtp_pass,
                                    imap_host,
                                    imap_port,
                                    imap_user,
                                    imap_pass,
                                    mut transport_status,
                                ) = (
                                    smtp_host,
                                    smtp_port,
                                    smtp_user,
                                    smtp_pass,
                                    imap_host,
                                    imap_port,
                                    imap_user,
                                    imap_pass,
                                    transport_status,
                                );
                                spawn(async move {
                                    let sport: u16 = smtp_port().parse().unwrap_or(587);
                                    let iport: u16 = imap_port().parse().unwrap_or(993);
                                    let smtp = if smtp_host().trim().is_empty() {
                                        String::new()
                                    } else {
                                        serde_json::json!({
                                            "host": smtp_host().trim(),
                                            "port": sport,
                                            "username": smtp_user().trim(),
                                            "password": smtp_pass(),
                                        })
                                        .to_string()
                                    };
                                    let imap = if imap_host().trim().is_empty() {
                                        String::new()
                                    } else {
                                        serde_json::json!({
                                            "host": imap_host().trim(),
                                            "port": iport,
                                            "username": imap_user().trim(),
                                            "password": imap_pass(),
                                        })
                                        .to_string()
                                    };
                                    match invoke_json::<serde_json::Value>(
                                        "save_mail_transport_config",
                                        json!({ "smtpJson": smtp, "imapJson": imap }),
                                    )
                                    .await
                                    {
                                        Ok(_) => transport_status.set("Transport prefs saved.".into()),
                                        Err(e) => transport_status.set(format!("Save failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Save transport prefs"
                    }
                    if !transport_status().is_empty() {
                        div { style: "color: #9ca3af; font-size: 12px; margin-top: 6px;", "{transport_status}" }
                    }

                    div { style: "border-top: 1px solid #374151; margin-top: 14px; padding-top: 12px;",
                        div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;", "Compose & send" }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                            placeholder: "from", value: "{compose_from}",
                            oninput: move |e| { let mut c = compose_from; c.set(e.value()); }
                        }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px; font-family: monospace;",
                            placeholder: "to", value: "{compose_to}",
                            oninput: move |e| { let mut c = compose_to; c.set(e.value()); }
                        }
                        input {
                            style: "{INPUT} margin-bottom: 4px; font-size: 12px;",
                            placeholder: "subject", value: "{compose_subject}",
                            oninput: move |e| { let mut c = compose_subject; c.set(e.value()); }
                        }
                        textarea {
                            style: "{TEXTAREA} height: 80px; margin-bottom: 6px;",
                            placeholder: "body",
                            value: "{compose_body}",
                            oninput: move |e| { let mut c = compose_body; c.set(e.value()); }
                        }
                        button {
                            style: "{BTN} font-size: 12px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (
                                        smtp_host,
                                        smtp_port,
                                        smtp_user,
                                        smtp_pass,
                                        compose_from,
                                        compose_to,
                                        compose_subject,
                                        compose_body,
                                        mut transport_status,
                                    ) = (
                                        smtp_host,
                                        smtp_port,
                                        smtp_user,
                                        smtp_pass,
                                        compose_from,
                                        compose_to,
                                        compose_subject,
                                        compose_body,
                                        transport_status,
                                    );
                                    spawn(async move {
                                        if smtp_host().trim().is_empty() {
                                            transport_status.set("Set SMTP host first.".into());
                                            return;
                                        }
                                        let sport: u16 = smtp_port().parse().unwrap_or(587);
                                        let smtp = serde_json::json!({
                                            "host": smtp_host().trim(),
                                            "port": sport,
                                            "username": smtp_user().trim(),
                                            "password": smtp_pass(),
                                        })
                                        .to_string();
                                        let mail = serde_json::json!({
                                            "from": compose_from().trim(),
                                            "to": compose_to().trim(),
                                            "subject": compose_subject(),
                                            "body": compose_body(),
                                        })
                                        .to_string();
                                        match invoke_json::<serde_json::Value>(
                                            "mail_send",
                                            json!({ "smtpJson": smtp, "mailJson": mail }),
                                        )
                                        .await
                                        {
                                            Ok(_) => transport_status.set("Sent.".into()),
                                            Err(e) => transport_status.set(format!("Send failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Send"
                        }
                    }

                    div { style: "border-top: 1px solid #374151; margin-top: 14px; padding-top: 12px;",
                        div { style: "color: #cbd5e1; font-size: 12px; font-weight: 600; margin-bottom: 6px;",
                            "Fetch unseen (with semantic verdicts)"
                        }
                        div { style: "display: flex; gap: 8px; align-items: center; margin-bottom: 6px;",
                            input {
                                style: "{INPUT} flex: 1; font-size: 12px;",
                                placeholder: "mailbox (INBOX)", value: "{fetch_mailbox}",
                                oninput: move |e| { let mut m = fetch_mailbox; m.set(e.value()); }
                            }
                            button {
                                style: "{BTN} font-size: 12px;",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (
                                            imap_host,
                                            imap_port,
                                            imap_user,
                                            imap_pass,
                                            fetch_mailbox,
                                            mut fetch_out,
                                            mut transport_status,
                                        ) = (
                                            imap_host,
                                            imap_port,
                                            imap_user,
                                            imap_pass,
                                            fetch_mailbox,
                                            fetch_out,
                                            transport_status,
                                        );
                                        spawn(async move {
                                            if imap_host().trim().is_empty() {
                                                transport_status.set("Set IMAP host first.".into());
                                                return;
                                            }
                                            let iport: u16 = imap_port().parse().unwrap_or(993);
                                            let imap = serde_json::json!({
                                                "host": imap_host().trim(),
                                                "port": iport,
                                                "username": imap_user().trim(),
                                                "password": imap_pass(),
                                            })
                                            .to_string();
                                            let mb = {
                                                let m = fetch_mailbox();
                                                if m.trim().is_empty() { "INBOX".into() } else { m }
                                            };
                                            match invoke_json::<serde_json::Value>(
                                                "mail_fetch",
                                                json!({ "imapJson": imap, "mailbox": mb }),
                                            )
                                            .await
                                            {
                                                Ok(v) => {
                                                    fetch_out.set(
                                                        serde_json::to_string_pretty(&v)
                                                            .unwrap_or_else(|_| v.to_string()),
                                                    );
                                                    transport_status.set("Fetch complete (verdicts applied).".into());
                                                }
                                                Err(e) => {
                                                    fetch_out.set(String::new());
                                                    transport_status.set(format!("Fetch failed: {e}"));
                                                }
                                            }
                                        });
                                    }
                                },
                                "Fetch unseen"
                            }
                        }
                        if !fetch_out().is_empty() {
                            textarea {
                                style: "{TEXTAREA} height: 180px;",
                                readonly: true,
                                value: "{fetch_out}"
                            }
                        }
                    }
                }
            }
        }
    }
}
