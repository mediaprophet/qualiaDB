//! Social graph — LWW + peer hash, presented on the renderer contract.

use crate::components::poet::engine::{self, PoetEvalResult};
use crate::components::poet::gpu_frame::PoetGpuFrame;
use crate::Route;
use dioxus::prelude::*;

const LWW: &str = r#"requires [ capability("capability.invoke") ];
effect fn merge() {
    return capability.invoke("Social.lww", {
        local: { s: 1, p: 2, o: 10, clock: 1 },
        remote: { s: 1, p: 2, o: 20, clock: 5 }
    });
}
"#;

const PEER: &str = r#"requires [ capability("capability.invoke") ];
effect fn peer() {
    return capability.invoke("Net.peer_hash", "did:q42:preview");
}
"#;

fn get_poet_project_handoff() -> (String, String) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.session_storage() {
            if let Ok(Some(val)) = storage.get_item("webizen_poet_project") {
                if let Some(id_start) = val.find("\"id\":\"") {
                    let rest = &val[id_start + 6..];
                    if let Some(id_end) = rest.find('"') {
                        let id = &rest[..id_end];
                        let name = if let Some(n_start) = val.find("\"name\":\"") {
                            let n_rest = &val[n_start + 8..];
                            n_rest.find('"').map(|e| &n_rest[..e]).unwrap_or("Cooperative Project")
                        } else {
                            "Cooperative Project"
                        };
                        return (id.to_string(), name.to_string());
                    }
                }
            }
        }
    }
    ("proj-qualia-shed".to_string(), "Qualia Engine Shed".to_string())
}

#[component]
/// Dioxus `SocialBody` component.
pub fn SocialBody() -> Element {
    let nav = navigator();
    let mut out = use_signal(|| String::new());
    let mut busy = use_signal(|| false);

    let (handoff_id, handoff_name) = get_poet_project_handoff();
    let mut project_id = use_signal(move || handoff_id);
    let mut effort_minutes = use_signal(|| 60u32);
    let mut capital_cents = use_signal(|| 0u64);
    let mut roi_multiplier = use_signal(|| "1.5".to_string());
    let mut artifact_did = use_signal(|| "did:q42:artifact:poet-eval-42".to_string());
    let mut status_msg = use_signal(|| String::new());

    rsx! {
        div { style: "display:grid;gap:12px;padding:8px;",
            div { style: "display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px;",
                p { style: muted(), "Peers are hashes, not a fake chat stream. LWW is the sync kernel; the ring is Render.scene(social)." }
                div { style: "display:flex;gap:6px;",
                    button {
                        r#type: "button",
                        style: "background:#1e293b;color:#38bdf8;border:1px solid #334155;border-radius:4px;padding:3px 8px;font-size:.70rem;cursor:pointer;",
                        onclick: move |_| { nav.push(Route::TalkRoute {}); },
                        "💬 Open Social Graph (/talk)"
                    }
                    button {
                        r#type: "button",
                        style: "background:#3b82f6;color:#ffffff;border:none;border-radius:4px;padding:3px 8px;font-size:.70rem;font-weight:600;cursor:pointer;",
                        onclick: move |_| { nav.push(Route::WorkRoute {}); },
                        "📂 Open Projects in OS-Shell (/work)"
                    }
                }
            }

            PoetGpuFrame { kind: "social", width: 720, height: 260 }

            div { style: "display:flex;gap:8px;flex-wrap:wrap;",
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(LWW, "merge", &mut busy, &mut out),
                    "Social.lww"
                }
                button {
                    disabled: busy(),
                    style: crate::components::poet::vibe_console::secondary(),
                    onclick: move |_| run(PEER, "peer", &mut busy, &mut out),
                    "Net.peer_hash"
                }
            }

            // Interactive POET -> Cooperative Project Log Contribution Form
            div { style: "background:#0f172a;border:1px solid #1e293b;border-radius:6px;padding:12px;display:grid;gap:8px;",
                div { style: "display:flex;justify-content:space-between;align-items:center;",
                    strong { style: "color:#38bdf8;font-size:.82rem;", "📊 Log POET Execution to Cooperative Project" }
                    span { style: "color:#94a3b8;font-size:.68rem;", "Active: {handoff_name}" }
                }
                div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:8px;",
                    div {
                        label { style: "font-size:.65rem;color:#64748b;display:block;", "Project ID" }
                        input {
                            style: "width:100%;background:#090d16;color:#e2e8f0;border:1px solid #334155;border-radius:4px;padding:4px 6px;font-size:.72rem;",
                            value: "{project_id}",
                            oninput: move |e| project_id.set(e.value()),
                        }
                    }
                    div {
                        label { style: "font-size:.65rem;color:#64748b;display:block;", "Effort (Minutes)" }
                        input {
                            r#type: "number",
                            style: "width:100%;background:#090d16;color:#e2e8f0;border:1px solid #334155;border-radius:4px;padding:4px 6px;font-size:.72rem;",
                            value: "{effort_minutes}",
                            oninput: move |e| effort_minutes.set(e.value().parse().unwrap_or(0)),
                        }
                    }
                    div {
                        label { style: "font-size:.65rem;color:#64748b;display:block;", "Capital (Cents)" }
                        input {
                            r#type: "number",
                            style: "width:100%;background:#090d16;color:#e2e8f0;border:1px solid #334155;border-radius:4px;padding:4px 6px;font-size:.72rem;",
                            value: "{capital_cents}",
                            oninput: move |e| capital_cents.set(e.value().parse().unwrap_or(0)),
                        }
                    }
                    div {
                        label { style: "font-size:.65rem;color:#64748b;display:block;", "ROI Multiplier" }
                        input {
                            style: "width:100%;background:#090d16;color:#e2e8f0;border:1px solid #334155;border-radius:4px;padding:4px 6px;font-size:.72rem;",
                            value: "{roi_multiplier}",
                            oninput: move |e| roi_multiplier.set(e.value()),
                        }
                    }
                    div { style: "grid-column:1/-1;",
                        label { style: "font-size:.65rem;color:#64748b;display:block;", "POET Artifact DID (did:q42)" }
                        input {
                            style: "width:100%;background:#090d16;color:#e2e8f0;border:1px solid #334155;border-radius:4px;padding:4px 6px;font-size:.72rem;font-family:monospace;",
                            value: "{artifact_did}",
                            oninput: move |e| artifact_did.set(e.value()),
                        }
                    }
                }

                div { style: "display:flex;justify-content:flex-end;gap:8px;margin-top:4px;",
                    button {
                        disabled: busy(),
                        style: "background:#10b981;color:#ffffff;border:none;border-radius:4px;padding:5px 12px;font-size:.75rem;font-weight:600;cursor:pointer;",
                        onclick: move |_| {
                            busy.set(true);
                            let pid = project_id().clone();
                            let mins = effort_minutes();
                            let cents = capital_cents();
                            let roi_str = roi_multiplier().clone();
                            let art_did = artifact_did().clone();

                            let script = format!(
                                "requires [ capability(\"capability.invoke\") ];\neffect fn seal_poet_contrib() {{\n    return capability.invoke(\"coop_contribution_log\", {{\n        project_id: \"{}\",\n        effort_minutes: {},\n        capital_cents: {},\n        roi_multiplier: {},\n        artifact_did: \"{}\"\n    }});\n}}\n",
                                pid, mins, cents, roi_str, art_did
                            );

                            spawn(async move {
                                let roi_f32 = roi_str.parse::<f32>().unwrap_or(1.0);
                                let desc = format!("POET Toolchain Execution ({} min, x{} ROI)", mins, roi_str);
                                let art_opt = if art_did.trim().is_empty() { None } else { Some(art_did.as_str()) };
                                let _ = crate::components::wellfair::host_client::add_contribution(
                                    &pid,
                                    "did:q42:agent:poet-runtime",
                                    &desc,
                                    mins,
                                    cents,
                                    roi_f32,
                                    "Public",
                                    art_opt,
                                ).await;

                                match engine::eval(script, false, Some("seal_poet_contrib".into())).await {
                                    Ok(PoetEvalResult { ok: true, value, .. }) => {
                                        out.set(value);
                                        status_msg.set("✓ Sealed contribution to POET store and host client.".into());
                                    }
                                    Ok(v) => {
                                        out.set(v.diagnostic.unwrap_or_else(|| "rejected".into()));
                                        status_msg.set("Recorded locally. POET diagnostic returned.".into());
                                    }
                                    Err(e) => {
                                        out.set(e);
                                        status_msg.set("Recorded locally. POET eval error.".into());
                                    }
                                }
                                busy.set(false);
                            });
                        },
                        "Log Contribution via POET Toolchain"
                    }
                }
                if !status_msg().is_empty() {
                    span { style: "color:#34d399;font-size:.70rem;", "{status_msg}" }
                }
            }

            if !out().is_empty() {
                pre { style: "font-size:.72rem;white-space:pre-wrap;margin:0;background:#090d16;padding:8px;border-radius:4px;border:1px solid #1e293b;color:#38bdf8;", "{out}" }
            }
        }
    }
}

fn run(src: &'static str, name: &'static str, busy: &mut Signal<bool>, out: &mut Signal<String>) {
    busy.set(true);
    let mut busy = *busy;
    let mut out = *out;
    spawn(async move {
        match engine::eval(src.into(), false, Some(name.into())).await {
            Ok(PoetEvalResult {
                ok: true, value, ..
            }) => out.set(value),
            Ok(v) => out.set(v.diagnostic.unwrap_or_else(|| "rejected".into())),
            Err(e) => out.set(e),
        }
        busy.set(false);
    });
}

fn muted() -> &'static str {
    "margin:0;color:#94a3b8;font-size:.74rem;line-height:1.45;"
}
