//! Reception tab — public identity, domain registration, DNS copy-paste.

#![allow(non_snake_case)]

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;

use super::helpers::*;
use super::types::*;

/// All signals needed by the Reception tab.
pub struct ReceptionSignals {
    pub status: Signal<String>,
    pub domain_name: Signal<String>,
    pub domain_label: Signal<String>,
    pub domains: Signal<Vec<serde_json::Value>>,
    pub front_doors: Signal<Vec<serde_json::Value>>,
    pub dns_name: Signal<String>,
    pub dns_txt: Signal<String>,
    pub turtle: Signal<String>,
}

pub fn render_reception(sig: ReceptionSignals) -> Element {
    let ReceptionSignals {
        status,
        mut domain_name,
        mut domain_label,
        domains,
        front_doors,
        dns_name,
        dns_txt,
        turtle,
    } = sig;

    rsx! {
        div { style: "{PANEL}",
            div { style: "{CARD}",
                h2 { style: "{H2}", "Reception — be findable by domain" }
                p { style: "{MUTED}",
                    "Three steps: (1) make a public-facing identity, (2) link a domain you own, (3) paste one TXT record at your domain registrar. Your private vault is never published."
                }
            }

            // ── Step 1: Public identity ──
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

            // ── Step 2: Register domain ──
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
                                            #[allow(unused_variables)]
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

            // ── Step 3: Copy DNS ──
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
}
