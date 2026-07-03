//! Decoy-retention toggle — real-session-only setting (vault v2 slice B).
//!
//! This panel controls what happens to activity that someone causes while they have made
//! the owner open the *cover* (decoy) space. Two choices:
//!   - `auto_archive`  — a quiet record is kept automatically (default, recommended).
//!   - `manual_triage` — nothing is kept until the owner reviews it next time in their real space.
//!
//! **It must NEVER render in the decoy session.** If the current session is the decoy session,
//! the component returns an empty fragment — a person who was forced to open the cover space
//! must not learn that a record is being kept. The setting lives only in the real/normal session.
//!
//! The language is deliberately plain, second-person, and jargon-free — it must be legible to
//! someone under duress. No "audit log", no "manifold", no "provenance".

use super::host_client::{
    fetch_sanctuary_prefs, get_decoy_retention_mode, set_decoy_retention_mode,
};
use dioxus::prelude::*;

const MODE_AUTO: &str = "auto_archive";
const MODE_MANUAL: &str = "manual_triage";

#[derive(Clone, Debug, Default)]
struct DecoyRetentionUi {
    /// Current mode ("auto_archive" | "manual_triage"). Defaults to auto-archive.
    mode: String,
    /// Small status line under the intro.
    status: String,
    /// Whether the current session is the decoy session — if so we render nothing.
    decoy_session: bool,
    /// Whether the initial load has completed (avoids a flash of the wrong choice).
    loaded: bool,
}

#[component]
pub fn WellfairDecoyRetentionPanel() -> Element {
    let mut ui = use_signal(DecoyRetentionUi::default);

    use_effect(move || {
        spawn(async move {
            // Learn the session kind first — never render this in a decoy session.
            if let Ok(prefs) = fetch_sanctuary_prefs().await {
                ui.write().decoy_session = prefs.decoy_session;
            }
            match get_decoy_retention_mode().await {
                Ok(mode) => {
                    let mode = if mode == MODE_MANUAL {
                        MODE_MANUAL.to_string()
                    } else {
                        MODE_AUTO.to_string()
                    };
                    ui.write().mode = mode;
                }
                Err(e) => {
                    ui.write().mode = MODE_AUTO.to_string();
                    ui.write().status = format!("Couldn't load this setting: {e}");
                }
            }
            ui.write().loaded = true;
        });
    });

    // Hard gate: this setting must never be visible in the cover space.
    if ui().decoy_session {
        return rsx! {};
    }

    let choose = move |mode: &'static str| {
        spawn(async move {
            ui.write().mode = mode.to_string();
            match set_decoy_retention_mode(mode).await {
                Ok(()) => {
                    let msg = if mode == MODE_AUTO {
                        "Saved. A record will be kept for you automatically."
                    } else {
                        "Saved. You'll be asked to review before anything is kept."
                    };
                    ui.write().status = msg.into();
                }
                Err(e) => ui.write().status = format!("Couldn't save this setting: {e}"),
            }
        });
    };

    let auto_selected = ui().mode != MODE_MANUAL;
    let manual_selected = ui().mode == MODE_MANUAL;

    // A selected option gets an accent border/tint; the other stays quiet.
    let option_style = |selected: bool| {
        if selected {
            "display:flex;gap:0.55rem;align-items:flex-start;padding:0.65rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:10px;background:var(--qualia-surface,#fff);cursor:pointer;margin-bottom:0.5rem;"
        } else {
            "display:flex;gap:0.55rem;align-items:flex-start;padding:0.65rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fff);cursor:pointer;margin-bottom:0.5rem;"
        }
    };

    rsx! {
        section {
            aria_label: "What to keep from your cover space",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "What to keep from your cover space" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Your cover space can quietly keep a record of anything someone changes when they make you open it — so you stay in control of your real space."
            }
            if !ui().status.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }
            }

            fieldset {
                style: "border:none;margin:0;padding:0;",
                // Option 1 — auto-archive (default, recommended).
                label {
                    style: option_style(auto_selected),
                    input {
                        r#type: "radio",
                        name: "decoy-retention",
                        checked: auto_selected,
                        onchange: move |_| choose(MODE_AUTO),
                        style: "margin-top:0.2rem;",
                    }
                    span {
                        style: "display:flex;flex-direction:column;gap:0.2rem;",
                        span {
                            style: "font-size:0.85rem;font-weight:600;",
                            "Save a record for me automatically "
                            span {
                                style: "font-weight:500;color:#1d6f63;font-size:0.74rem;",
                                "(recommended)"
                            }
                        }
                        span {
                            style: "font-size:0.76rem;color:var(--qualia-text-muted,#555);",
                            "You don't have to do anything in the moment. Next time you open your real space, you can look back at what happened."
                        }
                    }
                }

                // Option 2 — manual triage.
                label {
                    style: option_style(manual_selected),
                    input {
                        r#type: "radio",
                        name: "decoy-retention",
                        checked: manual_selected,
                        onchange: move |_| choose(MODE_MANUAL),
                        style: "margin-top:0.2rem;",
                    }
                    span {
                        style: "display:flex;flex-direction:column;gap:0.2rem;",
                        span {
                            style: "font-size:0.85rem;font-weight:600;",
                            "Let me choose each time"
                        }
                        span {
                            style: "font-size:0.76rem;color:var(--qualia-text-muted,#555);",
                            "Next time you open your real space, you'll see what changed and decide what to keep or delete. Nothing is saved until you do."
                        }
                    }
                }
            }

            // Quiet, honest limit — no false promises about what this can protect against.
            p {
                style: "margin:0.4rem 0 0;font-size:0.72rem;color:var(--qualia-text-muted,#888);",
                "This protects you from someone using the app. It can't hide your real space from an expert examining the files on your device."
            }
        }
    }
}
