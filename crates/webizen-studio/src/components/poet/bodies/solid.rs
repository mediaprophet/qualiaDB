//! Solid body — W3C Solid Pod LDP explorer & zero-lock-in migration wizard.

use dioxus::prelude::*;

#[component]
pub fn SolidBody() -> Element {
    let mut pod_url = use_signal(|| "https://pod.example.org/inbox/".to_string());
    let generated_ttl = use_signal(|| {
        r#"@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix ldp: <http://www.w3.org/ns/ldp#> .
@prefix solid: <http://www.w3.org/ns/solid/terms#> .
@prefix qualia: <urn:qualia:> .

<#me>
    a foaf:Person ;
    foaf:name "Timothy Charles Holborn" ;
    solid:oidcIssuer <https://solidcommunity.net> ;
    ldp:inbox <https://pod.example.org/inbox/> ;
    solid:publicTypeIndex <https://pod.example.org/settings/publicTypeIndex.ttl> ;
    solid:privateTypeIndex <https://pod.example.org/settings/privateTypeIndex.ttl> .
"#
        .to_string()
    });
    let mut active_tab = use_signal(|| "profile".to_string());

    rsx! {
        div { style: "display:grid;gap:8px;padding:8px;",
            div { style: "display:flex;gap:4px;border-bottom:1px solid rgba(255,255,255,0.08);padding-bottom:6px;",
                button {
                    class: if active_tab() == "profile" { "btn-tab active" } else { "btn-tab" },
                    style: format!(
                        "background:{};border:none;color:{};padding:4px 8px;border-radius:4px;font-size:11px;cursor:pointer;",
                        if active_tab() == "profile" { "rgba(56,189,248,0.2)" } else { "transparent" },
                        if active_tab() == "profile" { "var(--accent-cyan)" } else { "var(--text-muted)" }
                    ),
                    onclick: move |_| active_tab.set("profile".into()),
                    "🪪 WebID Profile"
                }
                button {
                    class: if active_tab() == "degrade" { "btn-tab active" } else { "btn-tab" },
                    style: format!(
                        "background:{};border:none;color:{};padding:4px 8px;border-radius:4px;font-size:11px;cursor:pointer;",
                        if active_tab() == "degrade" { "rgba(56,189,248,0.2)" } else { "transparent" },
                        if active_tab() == "degrade" { "var(--accent-cyan)" } else { "var(--text-muted)" }
                    ),
                    onclick: move |_| active_tab.set("degrade".into()),
                    "📉 4-Tier Degradation"
                }
                button {
                    class: if active_tab() == "export" { "btn-tab active" } else { "btn-tab" },
                    style: format!(
                        "background:{};border:none;color:{};padding:4px 8px;border-radius:4px;font-size:11px;cursor:pointer;",
                        if active_tab() == "export" { "rgba(56,189,248,0.2)" } else { "transparent" },
                        if active_tab() == "export" { "var(--accent-cyan)" } else { "var(--text-muted)" }
                    ),
                    onclick: move |_| active_tab.set("export".into()),
                    "📦 Export Manifold"
                }
            }

            if active_tab() == "profile" {
                div { style: "display:grid;gap:6px;",
                    div { style: "display:flex;gap:6px;align-items:center;",
                        input {
                            style: "flex:1;background:#141a23;border:1px solid rgba(255,255,255,0.1);color:var(--text-primary);padding:4px 8px;border-radius:4px;font-size:11px;",
                            value: "{pod_url}",
                            oninput: move |e| pod_url.set(e.value()),
                        }
                        button {
                            style: "background:rgba(56,189,248,0.15);border:1px solid rgba(56,189,248,0.3);color:var(--accent-cyan);padding:4px 8px;border-radius:4px;font-size:11px;cursor:pointer;",
                            "Fetch Inbox"
                        }
                    }
                    pre {
                        style: "background:#0d1117;border:1px solid rgba(255,255,255,0.06);border-radius:4px;padding:8px;font-family:var(--font-mono);font-size:11px;color:#38bdf8;white-space:pre-wrap;margin:0;max-height:160px;overflow-y:auto;",
                        "{generated_ttl}"
                    }
                }
            } else if active_tab() == "degrade" {
                div { style: "display:grid;gap:6px;font-size:11px;",
                    div { style: "padding:6px;background:rgba(0,230,118,0.08);border-left:3px solid #00E676;border-radius:4px;",
                        strong { style: "color:#00E676;", "Tier 1: 10D Manifold State Vector (.10d / .q42)" }
                        p { style: "margin:2px 0 0;color:var(--text-secondary);", "Full tensor state with quantum chemistry, audio latents, and neural weights." }
                    }
                    div { style: "padding:6px;background:rgba(56,189,248,0.08);border-left:3px solid #38bdf8;border-radius:4px;",
                        strong { style: "color:#38bdf8;", "Tier 2: Unicode PUA Interactive Icons (U+E000–U+E1FF)" }
                        p { style: "margin:2px 0 0;color:var(--text-secondary);", "Custom glyph font bundle with dynamic visual state mappings." }
                    }
                    div { style: "padding:6px;background:rgba(251,191,36,0.08);border-left:3px solid #fbbf24;border-radius:4px;",
                        strong { style: "color:#fbbf24;", "Tier 3: W3C Solid Pod RDFa / Turtle Semantics" }
                        p { style: "margin:2px 0 0;color:var(--text-secondary);", "Zero-lock-in export to W3C Linked Data Platform pods." }
                    }
                    div { style: "padding:6px;background:rgba(148,163,184,0.08);border-left:3px solid #94a3b8;border-radius:4px;",
                        strong { style: "color:#94a3b8;", "Tier 4: Plaintext & Markdown UTF-8 Fallback" }
                        p { style: "margin:2px 0 0;color:var(--text-secondary);", "Readable on standard terminals and screen readers." }
                    }
                }
            } else {
                div { style: "display:grid;gap:8px;",
                    p { style: "margin:0;font-size:11px;color:var(--text-secondary);", "Export all current manifold containers to a self-describing W3C Solid LDP archive with .meta.ttl sidecars." }
                    button {
                        style: "background:var(--accent-cyan);color:#0f172a;font-weight:600;border:none;padding:6px 12px;border-radius:4px;font-size:11px;cursor:pointer;",
                        "⬇️ Download .solid.zip Archive"
                    }
                }
            }
        }
    }
}
