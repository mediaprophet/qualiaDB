//! Sanctuary — PIN setup, lock/unlock, classified notes (SAF-01..20).

use super::host_client::{
    add_sanctuary_note, fetch_health_records, fetch_sanctuary_prefs, lock_sanctuary,
    setup_sanctuary, unlock_sanctuary, SanctuaryPrefsDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct SanctuaryUi {
    status: String,
    real_pin: String,
    decoy_pin: String,
    unlock_pin: String,
    note_body: String,
    prefs: SanctuaryPrefsDto,
    records: Vec<HealthRecordDto>,
}

#[component]
pub fn WellfairSanctuaryPanel() -> Element {
    let mut ui = use_signal(SanctuaryUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(p) = fetch_sanctuary_prefs().await {
                ui.write().prefs = p;
            }
            if let Ok(list) = fetch_health_records(48).await {
                let notes: Vec<_> = list
                    .into_iter()
                    .filter(|r| matches!(r.kind.as_str(), "sanctuary_note" | "therapy_note" | "welfare_case"))
                    .collect();
                ui.write().records = notes;
            }
        });
    };

    use_effect(move || {
        reload();
    });

    let locked = ui().prefs.locked;
    let enabled = ui().prefs.enabled;
    let decoy = ui().prefs.decoy_session;

    rsx! {
        section {
            aria_label: "WellFair sanctuary",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Sanctuary" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Isolated domain for classified notes. Decoy PIN shows a harmless session while keeping protected records hidden."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            if !enabled {
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Real unlock PIN"
                        input {
                            r#type: "password",
                            value: "{ui().real_pin}",
                            oninput: move |e| ui.write().real_pin = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Decoy PIN"
                        input {
                            r#type: "password",
                            value: "{ui().decoy_pin}",
                            oninput: move |e| ui.write().decoy_pin = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let real = ui().real_pin.clone();
                        let decoy = ui().decoy_pin.clone();
                        spawn(async move {
                            ui.write().status = "Setting up Sanctuary…".into();
                            match setup_sanctuary(&real, &decoy).await {
                                Ok(p) => {
                                    ui.write().prefs = p;
                                    ui.write().status = "Sanctuary armed. Lock when you need privacy.".into();
                                    reload();
                                }
                                Err(e) => ui.write().status = format!("Setup failed: {e}"),
                            }
                        });
                    },
                    "Arm Sanctuary"
                }
            } else {
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;margin-bottom:0.75rem;font-size:0.78rem;",
                    span {
                        style: if locked { "padding:0.2rem 0.5rem;border-radius:6px;background:#e76f5122;color:#9c3d2e;" } else { "padding:0.2rem 0.5rem;border-radius:6px;background:#2a9d8f22;color:#1d6f63;" },
                        if locked {
                            if decoy { "Locked (decoy session)" } else { "Locked" }
                        } else {
                            "Unlocked"
                        }
                    }
                    if !locked {
                        button {
                            style: "padding:0.35rem 0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                            onclick: move |_| {
                                spawn(async move {
                                    match lock_sanctuary().await {
                                        Ok(p) => {
                                            ui.write().prefs = p;
                                            ui.write().status = "Sanctuary locked — protected kinds hidden from lists.".into();
                                            reload();
                                        }
                                        Err(e) => ui.write().status = format!("Lock failed: {e}"),
                                    }
                                });
                            },
                            "Lock now"
                        }
                    }
                }
                if locked {
                    div {
                        style: "display:flex;gap:0.5rem;align-items:flex-end;margin-bottom:0.75rem;",
                        label {
                            style: "flex:1;display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                            "PIN"
                            input {
                                r#type: "password",
                                value: "{ui().unlock_pin}",
                                oninput: move |e| ui.write().unlock_pin = e.value(),
                                style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                            }
                        }
                        button {
                            style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                            onclick: move |_| {
                                let pin = ui().unlock_pin.clone();
                                spawn(async move {
                                    match unlock_sanctuary(&pin).await {
                                        Ok(p) => {
                                            let status = if p.decoy_session {
                                                "Decoy session — protected records remain hidden.".into()
                                            } else if !p.locked {
                                                "Unlocked.".into()
                                            } else {
                                                "Still locked.".into()
                                            };
                                            ui.write().prefs = p;
                                            ui.write().status = status;
                                            reload();
                                        }
                                        Err(e) => ui.write().status = format!("Unlock failed: {e}"),
                                    }
                                });
                            },
                            "Unlock"
                        }
                    }
                }
                if !locked && !decoy {
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;margin-bottom:0.5rem;",
                        "Sanctuary note (Classified)"
                        textarea {
                            rows: "3",
                            value: "{ui().note_body}",
                            oninput: move |e| ui.write().note_body = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        }
                    }
                    button {
                        style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #457b9d;background:#457b9d18;color:#457b9d;font-size:0.8rem;cursor:pointer;",
                        onclick: move |_| {
                            let body = ui().note_body.trim().to_string();
                            if body.is_empty() { return; }
                            spawn(async move {
                                match add_sanctuary_note(&body).await {
                                    Ok(_) => {
                                        ui.write().status = "Sanctuary note saved.".into();
                                        ui.write().note_body.clear();
                                        reload();
                                    }
                                    Err(e) => ui.write().status = format!("Failed: {e}"),
                                }
                            });
                        },
                        "Save sanctuary note"
                    }
                }
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Visible protected records ({ui().records.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;font-size:0.74rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.35rem 0;border-bottom:1px solid var(--qualia-border,#eee);",
                            "{r.kind}"
                        }
                    }
                }
            }
        }
    }
}