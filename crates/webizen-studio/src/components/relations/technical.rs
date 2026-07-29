use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;

#[component]
pub fn RelationshipTechnicalInspector() -> Element {
    let mut mesh = use_signal(|| serde_json::json!({}));
    let mut dialability = use_signal(|| serde_json::json!([]));
    let mut setup = use_signal(|| serde_json::json!({}));
    let mut status = use_signal(String::new);

    let mut refresh = move || {
        status.set("Inspecting relationship transports…".to_string());
        spawn(async move {
            if let Ok(value) =
                invoke_json::<serde_json::Value>("mesh_status", serde_json::json!({})).await
            {
                mesh.set(value);
            }
            if let Ok(value) =
                invoke_json::<serde_json::Value>("mesh_dialability", serde_json::json!({})).await
            {
                dialability.set(value);
            }
            if let Ok(value) =
                invoke_json::<serde_json::Value>("talk_setup_status", serde_json::json!({})).await
            {
                setup.set(value);
            }
            status.set("Technical state is scoped to the Relations apparatus.".to_string());
        });
    };
    use_hook(move || refresh());

    let running = mesh()
        .get("running")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let peers = dialability()
        .as_array()
        .cloned()
        .or_else(|| {
            dialability()
                .get("peers")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
        .unwrap_or_default();
    let receiver = setup()
        .get("receiver_running")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let raw = serde_json::to_string_pretty(
        &serde_json::json!({"mesh":mesh(),"dialability":dialability(),"setup":setup()}),
    )
    .unwrap_or_default();

    rsx! {
        section { style: "height:100%;overflow-y:auto;padding:22px;display:grid;gap:14px;",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:14px;",
                div {
                    h2 { style: "margin:0;font-size:1.15rem;", "Relationship transport inspector" }
                    p { style: "margin:5px 0 0;color:var(--qualia-text-muted);font-size:.75rem;line-height:1.5;", "Configured, reachable, authenticated and synchronized are separate states. This view does not collapse them into one “connected” badge." }
                }
                button { style: "{crate::components::settings::SECONDARY_BUTTON}", onclick: move |_| refresh(), "Run scoped diagnostic" }
            }
            div { role: "status", style: "{crate::components::settings::PANEL}", "{status}" }
            div { style: "display:grid;grid-template-columns:repeat(4,minmax(140px,1fr));gap:10px;align-items:stretch;",
                TopologyNode { title: "This device", configured: true, reachable: true, authenticated: true, synchronized: running }
                TopologyNode { title: "SocialWebNet", configured: true, reachable: running, authenticated: running, synchronized: running }
                TopologyNode { title: "Relay fallback", configured: true, reachable: running, authenticated: running, synchronized: running }
                TopologyNode { title: "Peer devices", configured: !peers.is_empty(), reachable: peers.iter().any(|p| p.get("dialable").and_then(serde_json::Value::as_bool).unwrap_or(false)), authenticated: false, synchronized: false }
            }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:12px;",
                div { style: "{crate::components::settings::PANEL}",
                    h3 { style: "margin:0 0 9px;font-size:.88rem;", "Alternate channels" }
                    StateLine { label: "Email receiver", value: if receiver { "Running" } else { "Unused / off" } }
                    StateLine { label: "Solid POD", value: "Configured per relationship" }
                    StateLine { label: "QDP / DNS", value: "See Reception records" }
                }
                div { style: "{crate::components::settings::PANEL}",
                    h3 { style: "margin:0 0 9px;font-size:.88rem;", "Peer routes" }
                    if peers.is_empty() {
                        div { style: "{crate::components::settings::EMPTY_CARD}", "No peer dialability records." }
                    }
                    for peer in peers {
                        pre { style: "white-space:pre-wrap;font-size:.64rem;color:var(--qualia-text-muted);", "{serde_json::to_string_pretty(&peer).unwrap_or_default()}" }
                    }
                }
            }
            details { style: "{crate::components::settings::PANEL}",
                summary { style: "cursor:pointer;font-weight:750;font-size:.78rem;", "Raw scoped records" }
                pre { style: "margin:12px 0 0;white-space:pre-wrap;overflow:auto;max-height:360px;background:#050a12;padding:12px;border-radius:9px;color:#cbd5e1;font-size:.64rem;", "{raw}" }
            }
        }
    }
}

#[component]
fn TopologyNode(
    title: String,
    configured: bool,
    reachable: bool,
    authenticated: bool,
    synchronized: bool,
) -> Element {
    rsx! {
        div { style: "{crate::components::settings::PANEL}",
            strong { "{title}" }
            div { style: "display:grid;gap:6px;margin-top:11px;",
                StateLine { label: "Configured", value: if configured { "Yes" } else { "No" } }
                StateLine { label: "Reachable", value: if reachable { "Yes" } else { "No" } }
                StateLine { label: "Authenticated", value: if authenticated { "Yes" } else { "Unknown" } }
                StateLine { label: "Synchronized", value: if synchronized { "Yes" } else { "Unknown" } }
            }
        }
    }
}

#[component]
fn StateLine(label: String, value: String) -> Element {
    let positive = value == "Yes" || value == "Running";
    rsx! {
        div { style: "display:flex;justify-content:space-between;gap:8px;font-size:.66rem;",
            span { style: "color:var(--qualia-text-muted);", "{label}" }
            span { style: if positive { "color:#6ee7b7;" } else { "color:#fcd34d;" }, "{value}" }
        }
    }
}
