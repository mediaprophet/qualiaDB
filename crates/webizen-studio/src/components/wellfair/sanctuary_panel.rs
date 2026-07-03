//! Sanctuary — PIN setup, lock/unlock, classified notes (SAF-01..20).

use super::host_client::{
    fetch_health_records, fetch_sanctuary_prefs, lock_sanctuary, sanctuary_vault_add_note,
    sanctuary_vault_configured, sanctuary_vault_is_keychain_wrapped, sanctuary_vault_list_notes,
    setup_sanctuary, setup_sanctuary_vault, setup_sanctuary_vault_wrapped, unlock_sanctuary,
    SanctuaryPrefsDto, SanctuaryVaultNoteDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct SanctuaryUi {
    status: String,
    real_pin: String,
    decoy_pin: String,
    unlock_pin: String,
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
                "Locking hides sanctuary-protected records (therapy notes, welfare cases) from ordinary views. Sensitive free-text notes live encrypted in the Encrypted vault below. Decoy PIN shows a harmless session."
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

#[derive(Clone, Debug, Default)]
struct VaultUi {
    status: String,
    configured: bool,
    setup_real: String,
    setup_decoy: String,
    pin: String,
    lane: Option<String>,
    note_body: String,
    notes: Vec<SanctuaryVaultNoteDto>,
    opened: bool,
    /// T1.2: bind the new vault to this device's OS keychain (experimental, recovery-gated).
    wrap_keychain: bool,
    /// Whether the on-disk vault is keychain-wrapped.
    keychain_wrapped: bool,
    /// One-time recovery code shown once after a wrapped setup — the user must record it.
    recovery_code: Option<String>,
}

/// Encrypted-at-rest Sanctuary notes (independent PBKDF2 key + AEAD; real vs decoy lane).
/// This is the genuine boundary: notes exist only as ciphertext until opened with a PIN.
#[component]
pub fn WellfairSanctuaryVaultPanel() -> Element {
    let mut ui = use_signal(VaultUi::default);

    use_effect(move || {
        spawn(async move {
            if let Ok(c) = sanctuary_vault_configured().await {
                ui.write().configured = c;
            }
            if let Ok(w) = sanctuary_vault_is_keychain_wrapped().await {
                ui.write().keychain_wrapped = w;
            }
        });
    });

    rsx! {
        section {
            aria_label: "WellFair encrypted sanctuary vault",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Encrypted vault" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Notes here are encrypted at rest with a key derived from your PIN — nothing is readable on disk without it. The decoy PIN opens a separate lane that never contains your real notes."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            if ui().keychain_wrapped {
                p {
                    style: "margin:0 0 0.5rem;font-size:0.72rem;color:#1d6a5f;",
                    "🔑 This vault is bound to this device's keychain."
                }
            }

            if let Some(code) = ui().recovery_code.clone() {
                div {
                    role: "alert",
                    style: "margin:0 0 0.6rem;padding:0.6rem;border:1px solid #e6394655;background:#e6394611;border-radius:8px;",
                    p { style: "margin:0 0 0.3rem;font-size:0.74rem;font-weight:600;color:#a52834;", "Save this recovery code now — it is shown only once" }
                    code {
                        style: "display:block;word-break:break-all;font-size:0.72rem;padding:0.4rem;background:var(--qualia-surface,#fff);border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        "{code}"
                    }
                    p {
                        style: "margin:0.3rem 0 0;font-size:0.68rem;color:var(--qualia-text-muted,#777);",
                        "If this device's keychain is ever lost, this code is the only way to reopen the vault. Store it somewhere safe and offline."
                    }
                }
            }

            if !ui().configured {
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Real PIN"
                        input {
                            r#type: "password",
                            value: "{ui().setup_real}",
                            oninput: move |e| ui.write().setup_real = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Decoy PIN"
                        input {
                            r#type: "password",
                            value: "{ui().setup_decoy}",
                            oninput: move |e| ui.write().setup_decoy = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                }
                label {
                    style: "display:flex;gap:0.4rem;align-items:flex-start;margin-bottom:0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                    input {
                        r#type: "checkbox",
                        checked: ui().wrap_keychain,
                        onchange: move |e| ui.write().wrap_keychain = e.value() == "true",
                        style: "margin-top:0.15rem;",
                    }
                    span {
                        strong { "Bind to this device's keychain (experimental). " }
                        "Adds a hardware-held secret so disk + PIN alone can't open the vault. "
                        "You'll get a one-time recovery code — if this device's keychain is lost and you don't have that code, the vault is unrecoverable."
                    }
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let real = ui().setup_real.clone();
                        let decoy = ui().setup_decoy.clone();
                        let wrap = ui().wrap_keychain;
                        spawn(async move {
                            ui.write().status = "Creating encrypted vault…".into();
                            if wrap {
                                match setup_sanctuary_vault_wrapped(&real, &decoy).await {
                                    Ok(code) => {
                                        ui.write().configured = true;
                                        ui.write().keychain_wrapped = true;
                                        ui.write().recovery_code = Some(code);
                                        ui.write().setup_real.clear();
                                        ui.write().setup_decoy.clear();
                                        ui.write().status = "Keychain-wrapped vault created. SAVE the recovery code below.".into();
                                    }
                                    Err(e) => ui.write().status = format!("Setup failed: {e}"),
                                }
                            } else {
                                match setup_sanctuary_vault(&real, &decoy).await {
                                    Ok(()) => {
                                        ui.write().configured = true;
                                        ui.write().setup_real.clear();
                                        ui.write().setup_decoy.clear();
                                        ui.write().status = "Encrypted vault created. Open it with your PIN.".into();
                                    }
                                    Err(e) => ui.write().status = format!("Setup failed: {e}"),
                                }
                            }
                        });
                    },
                    "Create encrypted vault"
                }
            } else if !ui().opened {
                div {
                    style: "display:flex;gap:0.5rem;align-items:flex-end;",
                    label {
                        style: "flex:1;display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "PIN"
                        input {
                            r#type: "password",
                            value: "{ui().pin}",
                            oninput: move |e| ui.write().pin = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    button {
                        style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                        onclick: move |_| {
                            let pin = ui().pin.clone();
                            if pin.is_empty() { return; }
                            spawn(async move {
                                ui.write().status = "Opening…".into();
                                match sanctuary_vault_list_notes(&pin).await {
                                    Ok((lane, notes)) => {
                                        ui.write().lane = Some(lane);
                                        ui.write().notes = notes;
                                        ui.write().opened = true;
                                        ui.write().status = "Opened. Close to clear decrypted notes.".into();
                                    }
                                    Err(e) => ui.write().status = format!("{e}"),
                                }
                            });
                        },
                        "Open vault"
                    }
                }
            } else {
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;margin-bottom:0.5rem;font-size:0.78rem;",
                    span {
                        style: if ui().lane.as_deref() == Some("decoy") {
                            "padding:0.2rem 0.5rem;border-radius:6px;background:#e9c46a33;color:#8a6d1d;"
                        } else {
                            "padding:0.2rem 0.5rem;border-radius:6px;background:#2a9d8f22;color:#1d6f63;"
                        },
                        if ui().lane.as_deref() == Some("decoy") { "Decoy lane" } else { "Real lane" }
                    }
                    button {
                        style: "padding:0.35rem 0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                        onclick: move |_| {
                            ui.write().pin.clear();
                            ui.write().notes.clear();
                            ui.write().lane = None;
                            ui.write().opened = false;
                            ui.write().status = "Closed — decrypted notes cleared.".into();
                        },
                        "Close"
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;margin-bottom:0.5rem;",
                    "New note"
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
                        let pin = ui().pin.clone();
                        let body = ui().note_body.trim().to_string();
                        if body.is_empty() { return; }
                        spawn(async move {
                            match sanctuary_vault_add_note(&pin, &body).await {
                                Ok(_) => {
                                    ui.write().note_body.clear();
                                    match sanctuary_vault_list_notes(&pin).await {
                                        Ok((lane, notes)) => {
                                            ui.write().lane = Some(lane);
                                            ui.write().notes = notes;
                                        }
                                        Err(e) => ui.write().status = format!("{e}"),
                                    }
                                    ui.write().status = "Encrypted note saved.".into();
                                }
                                Err(e) => ui.write().status = format!("Failed: {e}"),
                            }
                        });
                    },
                    "Save encrypted note"
                }

                if !ui().notes.is_empty() {
                    h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Notes ({ui().notes.len()})" }
                    ul {
                        style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                        for n in ui().notes.clone() {
                            li {
                                key: "{n.id}",
                                style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.76rem;white-space:pre-wrap;",
                                "{n.body}"
                            }
                        }
                    }
                }
            }
        }
    }
}