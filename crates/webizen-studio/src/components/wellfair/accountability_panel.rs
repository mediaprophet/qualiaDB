//! **Accountability fabric** panel (ADR 0011) — the human-facing surface for the tamper-evident ledger and
//! revocable consent credentials.
//!
//! The loop, made reachable: grant a worker (an agent) scoped, purpose-bound access to a committed payload;
//! record *how and why* they acted (attributable, court-auditable); revoke when done (crypto-enforced — the
//! wrapped key is destroyed, access ends) while the **conduct trail survives**; and verify the whole record
//! is intact — a betrayer cannot quietly drop the inconvenient act without it being detected and named.
//!
//! Honesty note surfaced in the UI: the payload **commitment** and the **wrapped key** are supplied by the
//! vault's envelope-encryption layer in the real flow (that wiring is deferred/coordinate). Here they are
//! entered directly so the accountability loop can be exercised end-to-end.

use super::host_client::{
    conduct_audit_trail, grant_consent_credential, ledger_entries, ledger_verify,
    list_consent_credentials, open_owner_payload, owner_envelope_public, record_conduct,
    revoke_consent_credential, seal_and_grant_credential,
};
use dioxus::prelude::*;

/// A 32-byte all-zero commitment (64 hex chars) — a usable default until the vault supplies a real one.
const DEFAULT_COMMITMENT_HEX: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
/// A placeholder wrapped key (opaque bytes); the vault's envelope encryption supplies the real one.
const DEFAULT_WRAPPED_KEY_HEX: &str = "deadbeefcafef00d";

fn short(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[component]
pub fn WellfairAccountabilityPanel() -> Element {
    // Ledger state.
    let mut ledger = use_signal(Vec::<serde_json::Value>::new);
    let mut integrity = use_signal(String::new);
    // Credential state.
    let mut creds = use_signal(Vec::<serde_json::Value>::new);
    // Audit trail state (for the selected credential).
    let mut audit = use_signal(Vec::<serde_json::Value>::new);
    let mut audit_for = use_signal(String::new);
    let mut status = use_signal(String::new);

    // Grant form.
    let mut g_agent = use_signal(|| "did:wf:social-worker".to_string());
    let mut g_scope = use_signal(|| "housing-support-case".to_string());
    let mut g_purpose = use_signal(|| "assess and arrange support".to_string());
    let mut g_commitment = use_signal(|| DEFAULT_COMMITMENT_HEX.to_string());
    let mut g_wrapped = use_signal(|| DEFAULT_WRAPPED_KEY_HEX.to_string());

    // Conduct form.
    let mut c_cred = use_signal(String::new);
    let mut c_action = use_signal(|| "accessed the housing record".to_string());
    let mut c_reason = use_signal(|| "acting under the granted consent".to_string());

    // Real envelope-encryption: seal a plaintext payload + open it back.
    let mut seal_payload_text = use_signal(|| "Emergency housing placement requested; unsafe living situation.".to_string());
    let mut owner_pubkey = use_signal(String::new);
    let mut opened_plaintext = use_signal(String::new);

    let reload = move || {
        spawn(async move {
            match ledger_entries(32).await {
                Ok(serde_json::Value::Array(rows)) => ledger.set(rows),
                Ok(_) => ledger.set(Vec::new()),
                Err(e) => status.set(format!("Ledger unavailable: {e}")),
            }
            match ledger_verify().await {
                Ok(v) => {
                    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
                    integrity.set(if ok {
                        "✓ Ledger intact — every entry chains and its signature verifies.".into()
                    } else {
                        format!("⚠ Tamper detected: {}", v.get("tamper").cloned().unwrap_or_default())
                    });
                }
                Err(e) => integrity.set(format!("Verify unavailable: {e}")),
            }
            match list_consent_credentials().await {
                Ok(serde_json::Value::Array(rows)) => creds.set(rows),
                Ok(_) => creds.set(Vec::new()),
                Err(e) => status.set(format!("Credentials unavailable: {e}")),
            }
            if let Ok(pk) = owner_envelope_public().await {
                owner_pubkey.set(pk);
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let do_grant = move |_| {
        let (agent, scope, purpose, commitment, wrapped) =
            (g_agent(), g_scope(), g_purpose(), g_commitment(), g_wrapped());
        spawn(async move {
            match grant_consent_credential(&agent, &scope, &purpose, &commitment, &wrapped, None).await {
                Ok(v) => {
                    status.set(format!("Granted credential {}", str_field(&v, "id")));
                    reload();
                }
                Err(e) => status.set(format!("Grant failed: {e}")),
            }
        });
    };

    let do_seal_grant = move |_| {
        let (agent, scope, purpose, payload) = (g_agent(), g_scope(), g_purpose(), seal_payload_text());
        spawn(async move {
            // Empty agent public key ⇒ sealed to the owner (self-custody), so it can be opened here.
            match seal_and_grant_credential(&agent, "", &scope, &purpose, &payload, None).await {
                Ok(v) => {
                    status.set(format!("Sealed + granted credential {} (payload encrypted)", str_field(&v, "id")));
                    reload();
                }
                Err(e) => status.set(format!("Seal + grant failed: {e}")),
            }
        });
    };

    let do_conduct = move |_| {
        let (agent, cred_id, action, reason, commitment) =
            (g_agent(), c_cred(), c_action(), c_reason(), g_commitment());
        if cred_id.trim().is_empty() {
            status.set("Enter a credential id to record conduct under.".into());
            return;
        }
        spawn(async move {
            match record_conduct(&agent, &cred_id, &action, &reason, &commitment).await {
                Ok(v) => {
                    status.set(format!("Recorded conduct {}", str_field(&v, "id")));
                    reload();
                }
                Err(e) => status.set(format!("Record failed: {e}")),
            }
        });
    };

    rsx! {
        section {
            aria_label: "WellFair accountability fabric",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.85rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Accountability — consent credentials & tamper-evident ledger" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            p {
                style: "margin:0;font-size:0.72rem;color:var(--qualia-text-muted,#777);",
                "Grant a worker scoped access, record how/why they acted, revoke when done (the data goes; the conduct trail stays). The commitment + wrapped key come from the vault's envelope encryption in the real flow."
            }
            if !status().is_empty() {
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-accent,#2b6);", "{status()}" }
            }

            // ── Ledger integrity ──
            div {
                style: "padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.8rem;font-weight:600;margin-bottom:0.25rem;", "Ledger integrity" }
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-text-muted,#555);", "{integrity()}" }
            }

            // ── Grant a consent credential ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;",
                div { style: "font-size:0.8rem;font-weight:600;", "Grant a consent credential" }
                LabeledInput { label: "Agent DID", value: g_agent, oninput: move |v| g_agent.set(v) }
                LabeledInput { label: "Scope", value: g_scope, oninput: move |v| g_scope.set(v) }
                LabeledInput { label: "Purpose", value: g_purpose, oninput: move |v| g_purpose.set(v) }
                LabeledInput { label: "Payload commitment (hex, 32 bytes — vault-supplied)", value: g_commitment, oninput: move |v| g_commitment.set(v) }
                LabeledInput { label: "Wrapped key (hex — vault-supplied)", value: g_wrapped, oninput: move |v| g_wrapped.set(v) }
                button {
                    style: "align-self:flex-start;padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;",
                    onclick: do_grant,
                    "Grant (raw wrapped key)"
                }
            }

            // ── Seal & grant with REAL envelope encryption ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.8rem;font-weight:600;", "Seal & grant — real envelope encryption" }
                p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                    "Encrypts the text below under a random key, seals that key to you (self-custody), and grants a credential over it. Revoking destroys the sealed key — the ciphertext then can't be opened. Uses the Agent DID / Scope / Purpose above."
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                    "Payload (encrypted at rest)"
                    textarea {
                        style: "padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;min-height:3rem;font-family:inherit;",
                        value: "{seal_payload_text}",
                        oninput: move |ev| seal_payload_text.set(ev.value()),
                    }
                }
                button {
                    style: "align-self:flex-start;padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;",
                    onclick: do_seal_grant,
                    "Seal & grant (encrypted)"
                }
                if !owner_pubkey().is_empty() {
                    div { style: "font-size:0.66rem;color:var(--qualia-text-muted,#999);font-family:monospace;word-break:break-all;",
                        "your envelope public key: {owner_pubkey()}"
                    }
                }
                if !opened_plaintext().is_empty() {
                    div { style: "font-size:0.74rem;color:var(--qualia-accent,#2b6);padding:0.35rem 0.45rem;border-radius:6px;border:1px dashed var(--qualia-accent,#2b6);",
                        "decrypted: {opened_plaintext()}"
                    }
                }
            }

            // ── Record conduct ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;",
                div { style: "font-size:0.8rem;font-weight:600;", "Record conduct (under a credential)" }
                LabeledInput { label: "Credential id", value: c_cred, oninput: move |v| c_cred.set(v) }
                LabeledInput { label: "Action", value: c_action, oninput: move |v| c_action.set(v) }
                LabeledInput { label: "Reason", value: c_reason, oninput: move |v| c_reason.set(v) }
                button {
                    style: "align-self:flex-start;padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                    onclick: do_conduct,
                    "Record conduct"
                }
            }

            // ── Consent credentials list ──
            if !creds.read().is_empty() {
                div {
                    style: "display:flex;flex-direction:column;gap:0.4rem;",
                    div { style: "font-size:0.8rem;font-weight:600;", "Consent credentials" }
                    for c in creds.read().clone() {
                        {
                            let id = str_field(&c, "id");
                            let revoked = c.get("revoked_unix").map(|x| !x.is_null()).unwrap_or(false);
                            let id_for_revoke = id.clone();
                            let id_for_audit = id.clone();
                            let id_for_open = id.clone();
                            rsx! {
                                div {
                                    key: "{id}",
                                    style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.74rem;display:flex;flex-direction:column;gap:0.2rem;",
                                    div {
                                        style: "display:flex;justify-content:space-between;gap:0.5rem;align-items:center;",
                                        strong { "{id}" }
                                        if revoked {
                                            span { style: "color:var(--qualia-danger,#b44);font-size:0.7rem;", "revoked" }
                                        } else {
                                            span { style: "color:var(--qualia-accent,#2b6);font-size:0.7rem;", "active" }
                                        }
                                    }
                                    div { style: "color:var(--qualia-text-muted,#666);",
                                        "{str_field(&c, \"agent_did\")} · {str_field(&c, \"scope\")}"
                                    }
                                    div {
                                        style: "display:flex;gap:0.4rem;margin-top:0.15rem;",
                                        button {
                                            style: "padding:0.2rem 0.5rem;border-radius:5px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.7rem;cursor:pointer;",
                                            onclick: move |_| {
                                                let cid = id_for_audit.clone();
                                                spawn(async move {
                                                    match conduct_audit_trail(&cid).await {
                                                        Ok(serde_json::Value::Array(rows)) => { audit.set(rows); audit_for.set(cid); }
                                                        Ok(_) => { audit.set(Vec::new()); audit_for.set(cid); }
                                                        Err(e) => status.set(format!("Audit failed: {e}")),
                                                    }
                                                });
                                            },
                                            "Audit trail"
                                        }
                                        if !revoked {
                                            button {
                                                style: "padding:0.2rem 0.5rem;border-radius:5px;border:1px solid var(--qualia-accent,#2b6);background:transparent;color:var(--qualia-accent,#2b6);font-size:0.7rem;cursor:pointer;",
                                                onclick: move |_| {
                                                    let cid = id_for_open.clone();
                                                    spawn(async move {
                                                        match open_owner_payload(&cid).await {
                                                            Ok(pt) => { opened_plaintext.set(pt); status.set(format!("Opened {cid} — decrypted via the live credential.")); }
                                                            Err(e) => status.set(format!("Open failed (expected once revoked): {e}")),
                                                        }
                                                    });
                                                },
                                                "Open"
                                            }
                                            button {
                                                style: "padding:0.2rem 0.5rem;border-radius:5px;border:1px solid var(--qualia-danger,#c66);background:transparent;color:var(--qualia-danger,#b44);font-size:0.7rem;cursor:pointer;",
                                                onclick: move |_| {
                                                    let cid = id_for_revoke.clone();
                                                    spawn(async move {
                                                        match revoke_consent_credential(&cid).await {
                                                            Ok(true) => { status.set(format!("Revoked {cid} — access ended, conduct trail kept.")); reload(); }
                                                            Ok(false) => status.set(format!("{cid} was already inactive.")),
                                                            Err(e) => status.set(format!("Revoke failed: {e}")),
                                                        }
                                                    });
                                                },
                                                "Revoke"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Audit trail for the selected credential ──
            if !audit_for().is_empty() {
                div {
                    style: "display:flex;flex-direction:column;gap:0.3rem;",
                    div { style: "font-size:0.8rem;font-weight:600;", "Audit trail — {audit_for()}" }
                    if audit.read().is_empty() {
                        p { style: "margin:0;font-size:0.74rem;color:var(--qualia-text-muted,#888);", "No conduct recorded under this credential yet." }
                    } else {
                        for r in audit.read().clone() {
                            div {
                                key: "{str_field(&r, \"id\")}",
                                style: "padding:0.4rem 0.5rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.73rem;",
                                div { strong { "{str_field(&r, \"action\")}" } }
                                div { style: "color:var(--qualia-text-muted,#666);", "{str_field(&r, \"reason\")}" }
                                div { style: "color:var(--qualia-text-muted,#888);font-size:0.68rem;", "by {str_field(&r, \"agent_did\")}" }
                            }
                        }
                    }
                }
            }

            // ── Ledger entries ──
            if !ledger.read().is_empty() {
                div {
                    style: "display:flex;flex-direction:column;gap:0.3rem;",
                    div { style: "font-size:0.8rem;font-weight:600;", "Ledger (newest first)" }
                    for e in ledger.read().clone() {
                        div {
                            key: "{e.get(\"seq\").and_then(|x| x.as_u64()).unwrap_or(0)}",
                            style: "padding:0.35rem 0.5rem;border-radius:6px;border:1px solid var(--qualia-border,#f0f0f0);font-size:0.71rem;display:flex;justify-content:space-between;gap:0.5rem;",
                            span {
                                span { style: "font-family:monospace;color:var(--qualia-text-muted,#999);", "#{e.get(\"seq\").and_then(|x| x.as_u64()).unwrap_or(0)} " }
                                strong { "{str_field(&e, \"kind\")}" }
                            }
                            span { style: "font-family:monospace;color:var(--qualia-text-muted,#aaa);", "{short(&str_field(&e, \"entry_hash_hex\"), 12)}" }
                        }
                    }
                }
            }
        }
    }
}

/// A small labelled text input row (kept local to this panel).
#[component]
fn LabeledInput(label: String, value: Signal<String>, oninput: EventHandler<String>) -> Element {
    rsx! {
        label {
            style: "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
            "{label}"
            input {
                style: "padding:0.3rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;background:var(--qualia-surface-2,#fff);",
                value: "{value}",
                oninput: move |ev| oninput.call(ev.value()),
            }
        }
    }
}
