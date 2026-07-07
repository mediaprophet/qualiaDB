//! **Safeguards** panel (ADR 0011 D6/D7) — the person's dead-man and incapacity switches, made reachable.
//!
//! - **Dead-man switch:** post-death disposition of a payload, **gamified + reversible** — it fires only when
//!   the liveness heartbeat lapses AND a quorum of distinct parties attest. "I'm alive" resets it.
//! - **Incapacity switch:** activates a pre-designated advocate on a **corroborated** trigger (party quorum +
//!   optionally an official instrument); **reversible** on recovery.
//!
//! Every action is owner-signed into the tamper-evident accountability ledger. Actual post-event enactment
//! (key-release / friend-side attestation) is the distributed-custody layer; here the owner arms, keeps the
//! heartbeat alive, and can test the trigger.

use super::host_client::{
    activate_incapacity, arm_dead_mans_switch, arm_incapacity_switch, attest_dead_mans,
    dead_mans_alive, enact_dead_mans, enact_dead_mans_release, enact_dead_mans_release_via_peers,
    list_dead_mans_switches, list_incapacity_switches, reconstruct_and_release, regain_capacity,
    split_dek_recovery,
};
use dioxus::prelude::*;

/// Parse `"did=pubkeyhex, did2=pubkeyhex2"` into `(did, pubkey_hex)` pairs.
fn party_keys(s: &str) -> Vec<(String, String)> {
    s.split(',')
        .filter_map(|pair| {
            let (did, pk) = pair.split_once('=')?;
            let (did, pk) = (did.trim(), pk.trim());
            if did.is_empty() || pk.is_empty() { None } else { Some((did.to_string(), pk.to_string())) }
        })
        .collect()
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// Parse a comma-separated list into trimmed, non-empty entries.
fn csv(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

/// Render a JSON byte-array (`[u8;N]`) as a short hex prefix for display.
fn arr_hex_short(v: &serde_json::Value, key: &str) -> String {
    let Some(arr) = v.get(key).and_then(|x| x.as_array()) else { return "—".into() };
    let hex: String = arr
        .iter()
        .take(6)
        .map(|b| format!("{:02x}", b.as_u64().unwrap_or(0) as u8))
        .collect();
    format!("{hex}…")
}

#[component]
pub fn WellfairSafeguardsPanel() -> Element {
    let mut dm_list = use_signal(Vec::<serde_json::Value>::new);
    let mut ic_list = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);

    // Dead-man form (the commitment field is shared across arm / alive / attest / enact).
    let mut dm_commitment = use_signal(|| "0000000000000000000000000000000000000000000000000000000000000000".to_string());
    let mut dm_grace = use_signal(|| "86400".to_string());
    let mut dm_parties = use_signal(|| "did:wf:friend-a, did:wf:friend-b".to_string());
    let mut dm_threshold = use_signal(|| "2".to_string());
    let mut dm_disposition = use_signal(|| "release_to".to_string());
    let mut dm_disp_parties = use_signal(|| "did:wf:trustee".to_string());
    let mut dm_attest_party = use_signal(|| "did:wf:friend-a".to_string());
    let mut dm_attest_kind = use_signal(|| "believed_dead".to_string());
    let mut dm_release_keys = use_signal(String::new);
    // Social recovery (Shamir).
    let mut sr_threshold = use_signal(|| "2".to_string());
    let mut sr_parties = use_signal(|| "did:wf:friend-a, did:wf:friend-b, did:wf:friend-c".to_string());
    let mut sr_shares_out = use_signal(String::new);
    let mut sr_shares_in = use_signal(String::new);

    // Incapacity form.
    let mut ic_principal = use_signal(|| "did:wf:me".to_string());
    let mut ic_kind = use_signal(|| "involuntary_psychiatric".to_string());
    let mut ic_advocate = use_signal(|| "did:wf:advocate".to_string());
    let mut ic_parties = use_signal(|| "did:wf:advocate, did:wf:friend".to_string());
    let mut ic_threshold = use_signal(|| "2".to_string());
    let mut ic_require_official = use_signal(|| false);
    let mut ic_official = use_signal(String::new);

    let reload = move || {
        spawn(async move {
            match list_dead_mans_switches().await {
                Ok(serde_json::Value::Array(rows)) => dm_list.set(rows),
                Ok(_) => dm_list.set(Vec::new()),
                Err(e) => status.set(format!("Dead-man list unavailable: {e}")),
            }
            match list_incapacity_switches().await {
                Ok(serde_json::Value::Array(rows)) => ic_list.set(rows),
                Ok(_) => ic_list.set(Vec::new()),
                Err(e) => status.set(format!("Incapacity list unavailable: {e}")),
            }
        });
    };
    use_effect(move || reload());

    let do_arm_dm = move |_| {
        let (c, grace, parties, thr, disp, dparties) = (
            dm_commitment(), dm_grace(), dm_parties(), dm_threshold(), dm_disposition(), dm_disp_parties(),
        );
        spawn(async move {
            let grace = grace.parse::<u64>().unwrap_or(86400);
            let thr = thr.parse::<usize>().unwrap_or(2);
            match arm_dead_mans_switch(&c, grace, csv(&parties), thr, &disp, csv(&dparties)).await {
                Ok(()) => { status.set("Dead-man switch armed.".into()); reload(); }
                Err(e) => status.set(format!("Arm failed: {e}")),
            }
        });
    };
    let do_alive = move |_| {
        let c = dm_commitment();
        spawn(async move {
            match dead_mans_alive(&c).await {
                Ok(true) => { status.set("Heartbeat touched — you're alive, switch reset.".into()); reload(); }
                Ok(false) => status.set("No dead-man switch for that commitment.".into()),
                Err(e) => status.set(format!("Alive failed: {e}")),
            }
        });
    };
    let do_attest = move |_| {
        let (c, party, kind) = (dm_commitment(), dm_attest_party(), dm_attest_kind());
        spawn(async move {
            match attest_dead_mans(&c, &party, &kind).await {
                Ok(true) => { status.set(format!("Attestation recorded from {party}.")); reload(); }
                Ok(false) => status.set("No dead-man switch for that commitment.".into()),
                Err(e) => status.set(format!("Attest failed: {e}")),
            }
        });
    };
    let do_enact = move |_| {
        let c = dm_commitment();
        spawn(async move {
            match enact_dead_mans(&c).await {
                Ok(v) => {
                    let d = v.get("disposition").cloned().unwrap_or(serde_json::Value::Null);
                    if d.is_null() {
                        status.set("Not triggerable yet (heartbeat not lapsed, or quorum not met).".into());
                    } else {
                        status.set(format!("Enacted. Disposition: {d}"));
                    }
                    reload();
                }
                Err(e) => status.set(format!("Enact failed: {e}")),
            }
        });
    };

    let do_release = move |_| {
        let (c, keys) = (dm_commitment(), dm_release_keys());
        spawn(async move {
            let pk = party_keys(&keys);
            if pk.is_empty() {
                status.set("Enter release keys as did=pubkeyhex pairs.".into());
                return;
            }
            match enact_dead_mans_release(&c, pk).await {
                Ok(v) => {
                    if v.get("enacted").and_then(|x| x.as_bool()).unwrap_or(false) {
                        status.set("Enacted + released — disposition parties granted access to the payload.".into());
                        reload();
                    } else {
                        status.set("Not triggerable yet (heartbeat not lapsed, or quorum not met).".into());
                    }
                }
                Err(e) => status.set(format!("Release failed: {e}")),
            }
        });
    };

    let do_release_via_peers = move |_| {
        let c = dm_commitment();
        spawn(async move {
            match enact_dead_mans_release_via_peers(&c).await {
                Ok(v) => {
                    let enacted = v.get("result").and_then(|r| r.get("enacted")).and_then(|x| x.as_bool()).unwrap_or(false);
                    let missing = v.get("missing_keys_for").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
                    if enacted {
                        status.set(if missing > 0 {
                            format!("Released to peers with known keys — {missing} party(ies) still missing a published key.")
                        } else {
                            "Released to all disposition parties (keys resolved from peers).".into()
                        });
                        reload();
                    } else {
                        status.set("Not triggerable yet (heartbeat/quorum).".into());
                    }
                }
                Err(e) => status.set(format!("Release-via-peers failed: {e}")),
            }
        });
    };

    let do_split = move |_| {
        let (c, thr, parties) = (dm_commitment(), sr_threshold(), sr_parties());
        spawn(async move {
            let thr = thr.parse::<usize>().unwrap_or(2);
            match split_dek_recovery(&c, thr, csv(&parties)).await {
                Ok(v) => {
                    sr_shares_out.set(serde_json::to_string(&v.get("shares").cloned().unwrap_or(v)).unwrap_or_default());
                    status.set("Split — hand each share to its friend (off-device).".into());
                }
                Err(e) => status.set(format!("Split failed: {e}")),
            }
        });
    };
    let do_reconstruct = move |_| {
        let (c, shares_in, keys) = (dm_commitment(), sr_shares_in(), dm_release_keys());
        spawn(async move {
            let shares: serde_json::Value = match serde_json::from_str(&shares_in) {
                Ok(v) => v,
                Err(e) => { status.set(format!("Shares JSON invalid: {e}")); return; }
            };
            match reconstruct_and_release(&c, shares, party_keys(&keys)).await {
                Ok(v) => {
                    if v.get("enacted").and_then(|x| x.as_bool()).unwrap_or(false) {
                        status.set("Reconstructed from friends' shares (no owner key) + released.".into());
                        reload();
                    } else {
                        status.set("Not triggerable yet, or shares insufficient.".into());
                    }
                }
                Err(e) => status.set(format!("Reconstruct failed: {e}")),
            }
        });
    };

    let do_arm_ic = move |_| {
        let (p, k, adv, parties, thr, req) = (
            ic_principal(), ic_kind(), ic_advocate(), ic_parties(), ic_threshold(), ic_require_official(),
        );
        spawn(async move {
            let thr = thr.parse::<usize>().unwrap_or(2);
            match arm_incapacity_switch(&p, &k, &adv, csv(&parties), thr, req).await {
                Ok(()) => { status.set("Incapacity switch armed.".into()); reload(); }
                Err(e) => status.set(format!("Arm failed: {e}")),
            }
        });
    };
    let do_activate = move |_| {
        let (p, parties, official, req) = (ic_principal(), ic_parties(), ic_official(), ic_require_official());
        spawn(async move {
            let official = if req && !official.trim().is_empty() { Some(official) } else { None };
            match activate_incapacity(&p, csv(&parties), official).await {
                Ok(true) => { status.set("Advocate activated (trigger satisfied).".into()); reload(); }
                Ok(false) => status.set("Trigger not satisfied (quorum / official instrument).".into()),
                Err(e) => status.set(format!("Activate failed: {e}")),
            }
        });
    };
    let do_regain = move |_| {
        let p = ic_principal();
        spawn(async move {
            match regain_capacity(&p).await {
                Ok(true) => { status.set("Capacity regained — advocate stood down.".into()); reload(); }
                Ok(false) => status.set("No incapacity switch for that principal.".into()),
                Err(e) => status.set(format!("Regain failed: {e}")),
            }
        });
    };

    rsx! {
        section {
            aria_label: "WellFair safeguards",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.9rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Safeguards — dead-man & incapacity switches" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            if !status().is_empty() {
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-accent,#2b6);", "{status()}" }
            }

            // ── Dead-man switch ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.55rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.85rem;font-weight:600;", "Dead-man switch" }
                p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                    "Fires only when your liveness heartbeat lapses AND a quorum of your chosen parties attest. \"I'm alive\" resets it. Governs a payload by its commitment."
                }
                Field { label: "Payload commitment (hex)", value: dm_commitment, oninput: move |v| dm_commitment.set(v) }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    Field { label: "Grace (secs)", value: dm_grace, oninput: move |v| dm_grace.set(v) }
                    Field { label: "Quorum threshold", value: dm_threshold, oninput: move |v| dm_threshold.set(v) }
                }
                Field { label: "Trigger parties (CSV)", value: dm_parties, oninput: move |v| dm_parties.set(v) }
                label { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);display:flex;flex-direction:column;gap:0.15rem;",
                    "Disposition"
                    select {
                        style: "padding:0.3rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        value: "{dm_disposition}",
                        oninput: move |ev| dm_disposition.set(ev.value()),
                        option { value: "release_to", "Release to trustees (reversible)" }
                        option { value: "make_public", "Make public (irreversible)" }
                    }
                }
                if dm_disposition() == "release_to" {
                    Field { label: "Release-to parties (CSV)", value: dm_disp_parties, oninput: move |v| dm_disp_parties.set(v) }
                }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    button { style: btn_primary(), onclick: do_arm_dm, "Arm" }
                    button { style: btn_plain(), onclick: do_alive, "I'm alive" }
                    button { style: btn_plain(), onclick: do_enact, "Enact (test)" }
                }
                div { style: "display:flex;gap:0.4rem;align-items:flex-end;flex-wrap:wrap;",
                    Field { label: "Attest as party", value: dm_attest_party, oninput: move |v| dm_attest_party.set(v) }
                    label { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);display:flex;flex-direction:column;gap:0.15rem;",
                        "Kind"
                        select {
                            style: "padding:0.3rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                            value: "{dm_attest_kind}",
                            oninput: move |ev| dm_attest_kind.set(ev.value()),
                            option { value: "believed_dead", "Believed dead" }
                            option { value: "no_contact", "No contact" }
                            option { value: "abandon", "Abandon (let go)" }
                        }
                    }
                    button { style: btn_plain(), onclick: do_attest, "Attest" }
                }
                div { style: "display:flex;gap:0.4rem;align-items:flex-end;flex-wrap:wrap;",
                    Field { label: "Release keys (did=pubkeyhex, …)", value: dm_release_keys, oninput: move |v| dm_release_keys.set(v) }
                    button { style: btn_primary(), onclick: do_release, "Enact & release keys" }
                    button { style: btn_plain(), onclick: do_release_via_peers, "Enact & release (via peers)" }
                }
                // ── Social recovery (Shamir): reconstruct the key WITHOUT the owner ──
                div {
                    style: "display:flex;flex-direction:column;gap:0.3rem;margin-top:0.3rem;padding:0.45rem 0.5rem;border-radius:6px;border:1px dashed var(--qualia-accent,#2b6);",
                    div { style: "font-size:0.76rem;font-weight:600;", "Social recovery (Shamir)" }
                    p { style: "margin:0;font-size:0.68rem;color:var(--qualia-text-muted,#888);",
                        "Split the key into shares for your friends — a quorum of them can reconstruct it later WITHOUT you (the true post-death path)."
                    }
                    div { style: "display:flex;gap:0.4rem;align-items:flex-end;flex-wrap:wrap;",
                        Field { label: "Threshold (k)", value: sr_threshold, oninput: move |v| sr_threshold.set(v) }
                        Field { label: "Share-holder parties (CSV)", value: sr_parties, oninput: move |v| sr_parties.set(v) }
                        button { style: btn_plain(), onclick: do_split, "Split into shares" }
                    }
                    if !sr_shares_out().is_empty() {
                        textarea {
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.66rem;font-family:monospace;min-height:3rem;",
                            readonly: true,
                            value: "{sr_shares_out()}",
                        }
                    }
                    label { style: "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                        "Reconstruct — paste a quorum of shares (JSON array)"
                        textarea {
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.66rem;font-family:monospace;min-height:3rem;",
                            value: "{sr_shares_in()}",
                            oninput: move |ev| sr_shares_in.set(ev.value()),
                        }
                    }
                    button { style: btn_primary(), onclick: do_reconstruct, "Reconstruct & release (no owner key)" }
                }
                if !dm_list.read().is_empty() {
                    ul { style: "margin:0.2rem 0 0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.3rem;",
                        for rec in dm_list.read().clone() {
                            {
                                let sw = rec.get("switch").cloned().unwrap_or(serde_json::Value::Null);
                                let fired = sw.get("fired_unix").map(|x| !x.is_null()).unwrap_or(false);
                                let attesters = rec.get("attestations").and_then(|a| a.as_array()).map(|a| a.len()).unwrap_or(0);
                                let thr = sw.get("trigger").and_then(|t| t.get("attestation_threshold")).and_then(|x| x.as_u64()).unwrap_or(0);
                                rsx! {
                                    li { style: "padding:0.35rem 0.5rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.72rem;display:flex;justify-content:space-between;gap:0.5rem;",
                                        span { style: "font-family:monospace;color:var(--qualia-text-muted,#888);", "{arr_hex_short(&sw, \"payload_commitment\")}" }
                                        span { "{attesters}/{thr} attested" }
                                        if fired {
                                            span { style: "color:var(--qualia-danger,#b44);", "fired" }
                                        } else {
                                            span { style: "color:var(--qualia-accent,#2b6);", "armed" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Incapacity switch ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.55rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.85rem;font-weight:600;", "Incapacity switch" }
                p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                    "Activates a pre-chosen advocate on a corroborated trigger (party quorum + optionally an official instrument). Reversible on recovery — the advocate stands down."
                }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    Field { label: "Principal DID", value: ic_principal, oninput: move |v| ic_principal.set(v) }
                    Field { label: "Advocate DID", value: ic_advocate, oninput: move |v| ic_advocate.set(v) }
                }
                label { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);display:flex;flex-direction:column;gap:0.15rem;",
                    "Kind"
                    select {
                        style: "padding:0.3rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        value: "{ic_kind}",
                        oninput: move |ev| ic_kind.set(ev.value()),
                        option { value: "involuntary_psychiatric", "Involuntary psychiatric admission" }
                        option { value: "serious_injury", "Serious injury" }
                    }
                }
                Field { label: "Trigger parties (CSV)", value: ic_parties, oninput: move |v| ic_parties.set(v) }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;align-items:flex-end;",
                    Field { label: "Quorum threshold", value: ic_threshold, oninput: move |v| ic_threshold.set(v) }
                    label { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);display:flex;align-items:center;gap:0.3rem;",
                        input { r#type: "checkbox", checked: ic_require_official(), onchange: move |ev| ic_require_official.set(ev.checked()) }
                        "require official instrument"
                    }
                }
                if ic_require_official() {
                    Field { label: "Official instrument ref", value: ic_official, oninput: move |v| ic_official.set(v) }
                }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    button { style: btn_primary(), onclick: do_arm_ic, "Arm" }
                    button { style: btn_plain(), onclick: do_activate, "Activate advocate" }
                    button { style: btn_plain(), onclick: do_regain, "Regain capacity" }
                }
                if !ic_list.read().is_empty() {
                    ul { style: "margin:0.2rem 0 0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.3rem;",
                        for sw in ic_list.read().clone() {
                            {
                                let active = sw.get("active_since_unix").map(|x| !x.is_null()).unwrap_or(false);
                                rsx! {
                                    li { style: "padding:0.35rem 0.5rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.72rem;display:flex;justify-content:space-between;gap:0.5rem;",
                                        span { "{str_field(&sw, \"principal_did\")} → {str_field(&sw, \"advocate_did\")}" }
                                        if active {
                                            span { style: "color:var(--qualia-danger,#b44);", "advocate active" }
                                        } else {
                                            span { style: "color:var(--qualia-accent,#2b6);", "dormant" }
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
}

fn btn_primary() -> &'static str {
    "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;"
}
fn btn_plain() -> &'static str {
    "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;"
}

#[component]
fn Field(label: String, value: Signal<String>, oninput: EventHandler<String>) -> Element {
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
