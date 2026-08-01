//! Person / apparatus fleet management in Settings.

#![allow(non_snake_case)]

use super::{PRIMARY_BUTTON, SECONDARY_BUTTON, PANEL, WARNING_CARD, SUCCESS_CARD, FIELD};
use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PersonPublicView {
    person_id: String,
    verifying_key_hex: String,
    #[serde(default)]
    created_at_unix: u64,
    #[serde(default)]
    display_hint: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct DeviceView {
    device_id: String,
    person_id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    hostname: String,
    #[serde(default)]
    control_base_url: String,
    is_local: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct IdentityPlaneView {
    person: PersonPublicView,
    local_device_id: String,
    #[serde(default)]
    devices: Vec<DeviceView>,
    #[serde(default)]
    os_account_is_not_principal: bool,
    #[serde(default)]
    notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersonTransferBundleView {
    format: String,
    person: serde_json::Value,
}

#[component]
pub fn IdentityPlanePanel() -> Element {
    let mut plane = use_signal(|| Option::<IdentityPlaneView>::None);
    let mut status = use_signal(String::new);
    let mut error = use_signal(String::new);
    let mut control_url = use_signal(String::new);
    let mut import_json = use_signal(String::new);
    let mut peer_json = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let mut refresh = move || {
        busy.set(true);
        error.set(String::new());
        spawn(async move {
            match invoke_json::<IdentityPlaneView>("get_identity_plane", serde_json::json!({})).await
            {
                Ok(value) => {
                    if let Some(local) = value.devices.iter().find(|d| d.is_local) {
                        control_url.set(local.control_base_url.clone());
                    }
                    plane.set(Some(value));
                }
                Err(e) => error.set(e),
            }
            busy.set(false);
        });
    };

    use_hook(move || refresh());

    rsx! {
        section { style: "display:grid;gap:16px;",
            div { style: "{PANEL}",
                h2 { style: "margin:0 0 8px;font-size:1.05rem;", "Person and devices" }
                p { style: "margin:0;color:var(--qualia-text-muted);font-size:.76rem;line-height:1.55;",
                    "You are not the machine, and you are not the OS login. A "
                    strong { "person" }
                    " principal can own many "
                    strong { "apparatus" }
                    " installs. Jobs can target a device ID; remote delivery uses that device's control URL."
                }
            }

            if !error().is_empty() {
                div { style: "{WARNING_CARD}", "{error}" }
            }
            if !status().is_empty() {
                div { style: "{SUCCESS_CARD}", "{status}" }
            }

            if let Some(p) = plane() {
                div { style: "{PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.92rem;", "Person principal" }
                    dl { style: "margin:0;display:grid;grid-template-columns:auto 1fr;gap:8px 12px;font-size:.74rem;",
                        dt { style: "color:var(--qualia-text-muted);", "Person ID" }
                        dd { style: "margin:0;overflow-wrap:anywhere;font-family:ui-monospace,monospace;font-size:.68rem;", "{p.person.person_id}" }
                        dt { style: "color:var(--qualia-text-muted);", "Verifying key" }
                        dd { style: "margin:0;overflow-wrap:anywhere;font-family:ui-monospace,monospace;font-size:.68rem;", "{p.person.verifying_key_hex}" }
                        dt { style: "color:var(--qualia-text-muted);", "OS account" }
                        dd { style: "margin:0;", "Not used as identity" }
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;margin-top:14px;",
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<PersonTransferBundleView>(
                                        "export_person_transfer_bundle",
                                        serde_json::json!({}),
                                    ).await {
                                        Ok(bundle) => {
                                            let text = serde_json::to_string_pretty(&bundle).unwrap_or_default();
                                            import_json.set(text);
                                            status.set("Transfer bundle loaded into the box below — save it privately, then import on another machine.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Export person transfer bundle"
                        }
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>(
                                        "mint_person_webid_tls_cert",
                                        serde_json::json!({}),
                                    ).await {
                                        Ok(v) => {
                                            status.set(format!(
                                                "WebID-TLS cert written: {}",
                                                v.get("cert_path").and_then(|x| x.as_str()).unwrap_or("ok")
                                            ));
                                        }
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Mint WebID-TLS cert for person"
                        }
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| refresh(),
                            "Refresh"
                        }
                    }
                }

                div { style: "{PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.92rem;", "This apparatus (local device)" }
                    p { style: "margin:0 0 10px;font-size:.72rem;color:var(--qualia-text-muted);line-height:1.5;",
                        "Device ID: "
                        span { style: "font-family:ui-monospace,monospace;font-size:.68rem;", "{p.local_device_id}" }
                    }
                    label { style: "display:block;font-size:.72rem;font-weight:750;margin-bottom:6px;",
                        "Control base URL (LAN address other machines use to send jobs here)"
                    }
                    input {
                        value: "{control_url}",
                        placeholder: "http://192.168.1.20:8080",
                        style: "{FIELD}",
                        oninput: move |e| control_url.set(e.value()),
                    }
                    p { style: "margin:8px 0 0;font-size:.68rem;color:var(--qualia-text-muted);line-height:1.45;",
                        "Use a reachable LAN or VPN URL, not only 127.0.0.1, if another computer will deliver jobs."
                    }
                    button {
                        r#type: "button",
                        style: format!("{PRIMARY_BUTTON} margin-top:12px;"),
                        disabled: busy(),
                        onclick: move |_| {
                            let url = control_url();
                            busy.set(true);
                            spawn(async move {
                                match invoke_json::<IdentityPlaneView>(
                                    "set_local_control_base_url",
                                    serde_json::json!({ "url": url }),
                                ).await {
                                    Ok(v) => {
                                        plane.set(Some(v));
                                        status.set("Control URL saved.".into());
                                    }
                                    Err(e) => error.set(e),
                                }
                                busy.set(false);
                            });
                        },
                        "Save control URL"
                    }
                }

                div { style: "{PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.92rem;", "Fleet devices" }
                    if p.devices.is_empty() {
                        p { style: "margin:0;color:var(--qualia-text-muted);font-size:.74rem;", "No devices yet." }
                    } else {
                        div { style: "display:grid;gap:8px;",
                            for d in p.devices {
                                div { style: "padding:12px;border:1px solid var(--qualia-border);border-radius:11px;",
                                    div { style: "display:flex;justify-content:space-between;gap:8px;flex-wrap:wrap;",
                                        strong { style: "font-size:.78rem;",
                                            if d.label.is_empty() {
                                                "Apparatus"
                                            } else {
                                                "{d.label}"
                                            }
                                        }
                                        span { style: "font-size:.64rem;font-weight:800;text-transform:uppercase;",
                                            if d.is_local { "Local" } else { "Peer" }
                                        }
                                    }
                                    div { style: "margin-top:6px;font-family:ui-monospace,monospace;font-size:.66rem;overflow-wrap:anywhere;", "{d.device_id}" }
                                    if !d.control_base_url.is_empty() {
                                        div { style: "margin-top:4px;font-size:.7rem;color:var(--qualia-text-muted);",
                                            "URL: "
                                            "{d.control_base_url}"
                                        }
                                    }
                                    if !d.hostname.is_empty() {
                                        div { style: "margin-top:2px;font-size:.7rem;color:var(--qualia-text-muted);",
                                            "Host: "
                                            "{d.hostname}"
                                            " (informational only)"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;",
                        button {
                            r#type: "button",
                            style: "{SECONDARY_BUTTON}",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    match invoke_json::<usize>("retry_remote_job_outbox", serde_json::json!({})).await {
                                        Ok(n) => status.set(format!(
                                            "Retried remote outbox; delivered {}.",
                                            n
                                        )),
                                        Err(e) => error.set(e),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Retry remote job outbox"
                        }
                    }
                }
            }

            div { style: "{PANEL}",
                h3 { style: "margin:0 0 8px;font-size:.92rem;", "Import person transfer bundle" }
                p { style: "margin:0 0 10px;font-size:.72rem;color:var(--qualia-text-muted);line-height:1.5;",
                    "Paste a bundle exported from another install of "
                    em { "your" }
                    " person principal. This does not copy OS accounts."
                }
                textarea {
                    value: "{import_json}",
                    rows: "6",
                    placeholder: "Paste person transfer bundle JSON (format qualia.person.transfer.v1)",
                    style: format!("{FIELD} min-height:120px;font-family:ui-monospace,monospace;font-size:.68rem;"),
                    oninput: move |e| import_json.set(e.value()),
                }
                button {
                    r#type: "button",
                    style: format!("{PRIMARY_BUTTON} margin-top:10px;"),
                    disabled: busy() || import_json().trim().is_empty(),
                    onclick: move |_| {
                        let raw = import_json();
                        busy.set(true);
                        spawn(async move {
                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                Ok(bundle) => {
                                    let payload = serde_json::json!({ "bundle": bundle });
                                    match invoke_json::<IdentityPlaneView>(
                                        "import_person_transfer_bundle",
                                        payload,
                                    )
                                    .await
                                    {
                                        Ok(v) => {
                                            plane.set(Some(v));
                                            status.set("Person principal imported; local apparatus re-bound.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                }
                                Err(e) => error.set(format!("Invalid JSON: {}", e)),
                            }
                            busy.set(false);
                        });
                    },
                    "Import person bundle"
                }
            }

            div { style: "{PANEL}",
                h3 { style: "margin:0 0 8px;font-size:.92rem;", "Register peer apparatus" }
                p { style: "margin:0 0 10px;font-size:.72rem;color:var(--qualia-text-muted);line-height:1.5;",
                    "Paste a device record JSON from the peer's "
                    code { "get_identity_plane" }
                    " local device (or full devices entry), including control_base_url."
                }
                textarea {
                    value: "{peer_json}",
                    rows: "5",
                    placeholder: "Peer device JSON: device_id, person_id, control_base_url",
                    style: format!("{FIELD} min-height:100px;font-family:ui-monospace,monospace;font-size:.68rem;"),
                    oninput: move |e| peer_json.set(e.value()),
                }
                button {
                    r#type: "button",
                    style: format!("{PRIMARY_BUTTON} margin-top:10px;"),
                    disabled: busy() || peer_json().trim().is_empty(),
                    onclick: move |_| {
                        let raw = peer_json();
                        busy.set(true);
                        spawn(async move {
                            match serde_json::from_str::<serde_json::Value>(&raw) {
                                Ok(device) => {
                                    let payload = serde_json::json!({ "device": device });
                                    match invoke_json::<IdentityPlaneView>(
                                        "register_remote_apparatus_device",
                                        payload,
                                    )
                                    .await
                                    {
                                        Ok(v) => {
                                            plane.set(Some(v));
                                            status.set("Peer apparatus registered.".into());
                                        }
                                        Err(e) => error.set(e),
                                    }
                                }
                                Err(e) => error.set(format!("Invalid JSON: {}", e)),
                            }
                            busy.set(false);
                        });
                    },
                    "Register peer device"
                }
            }
        }
    }
}
