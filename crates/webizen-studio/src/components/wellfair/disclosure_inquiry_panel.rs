//! **Disclosure & inquiry** panel (ADR 0011 D5 + D8).
//!
//! - **Disclosure traceability:** record a protective "I informed authority X" cc; record who accessed a
//!   payload (and, for a staff leak, *which delegate* acted); view the disclosure chain + the leak-suspect
//!   actor set; and **trace a leak** by its fingerprint to the accountable actor — so a betrayal is knowable.
//! - **Duty of inquiry:** classify conduct against a duty — the fair negligence test (was an *accessible*
//!   means left unchecked, and did a harmful act follow?), distinguishing no-fault / diligent / shortfall /
//!   negligent.

use super::host_client::{
    actors_with_access, assess_duty_of_inquiry, disclosure_chain, list_transparency_ccs,
    record_disclosure, record_transparency_cc, trace_leak,
};
use dioxus::prelude::*;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn csv(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}
fn opt(s: String) -> Option<String> {
    let s = s.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn verdict_label(v: &str) -> (&'static str, &'static str) {
    match v {
        "diligent" => ("Diligent — every accessible means was checked", "#2a9d8f"),
        "no_fault" => ("No fault — the means weren't accessible; couldn't have known", "#5c9a6f"),
        "unchecked_no_harm" => ("Shortfall — accessible means unchecked, but no harm followed", "#c9a227"),
        "negligent" => ("Negligent — unchecked accessible means, and harm followed", "#b4453a"),
        _ => ("—", "#999"),
    }
}

#[component]
pub fn WellfairDisclosureInquiryPanel() -> Element {
    let mut status = use_signal(String::new);
    let mut ccs = use_signal(Vec::<serde_json::Value>::new);
    let mut chain = use_signal(Vec::<serde_json::Value>::new);
    let mut actors = use_signal(Vec::<String>::new);

    // Disclosure forms.
    let mut cc_credential = use_signal(|| "cc-transparency".to_string());
    let mut cc_authority = use_signal(|| "did:wf:mp".to_string());
    let mut cc_purpose = use_signal(|| "protection from serious crime".to_string());
    let mut d_commitment = use_signal(|| "0000000000000000000000000000000000000000000000000000000000000000".to_string());
    let mut d_credential = use_signal(|| "cc-transparency".to_string());
    let mut d_recipient = use_signal(|| "did:wf:mp".to_string());
    let mut d_delegate = use_signal(String::new);
    let mut d_onward = use_signal(String::new);
    let mut leak_fp = use_signal(String::new);
    let mut leak_result = use_signal(String::new);

    // Duty of inquiry form.
    let mut q_act = use_signal(|| "record the person as unreliable / medicate".to_string());
    let mut q_actor = use_signal(|| "did:wf:facility-staff".to_string());
    let mut q_means = use_signal(|| "specialist-credential, prior-records".to_string());
    let mut q_accessible = use_signal(|| "specialist-credential".to_string());
    let mut q_checked = use_signal(String::new);
    let mut q_acted = use_signal(|| true);
    let mut q_injury = use_signal(|| true);
    let mut q_verdict = use_signal(String::new);

    let reload = move || {
        spawn(async move {
            if let Ok(serde_json::Value::Array(rows)) = list_transparency_ccs().await {
                ccs.set(rows);
            }
        });
    };
    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let do_cc = move |_| {
        let (c, a, p) = (cc_credential(), cc_authority(), cc_purpose());
        spawn(async move {
            match record_transparency_cc(&c, &a, &p).await {
                Ok(()) => { status.set(format!("Recorded: informed {a}.")); reload(); }
                Err(e) => status.set(format!("cc failed: {e}")),
            }
        });
    };
    let do_disclosure = move |_| {
        let (c, cr, r, dele, on) = (d_commitment(), d_credential(), d_recipient(), d_delegate(), d_onward());
        spawn(async move {
            match record_disclosure(&c, &cr, &r, opt(dele), opt(on)).await {
                Ok(ev) => {
                    let fp = str_field(&ev, "fingerprint");
                    status.set(format!("Disclosure recorded (id {}). fingerprint: {}", str_field(&ev, "id"),
                        ev.get("fingerprint").map(|x| x.to_string()).unwrap_or(fp)));
                }
                Err(e) => status.set(format!("disclosure failed: {e}")),
            }
        });
    };
    let do_view = move |_| {
        let c = d_commitment();
        spawn(async move {
            match disclosure_chain(&c).await {
                Ok(serde_json::Value::Array(rows)) => chain.set(rows),
                Ok(_) => chain.set(Vec::new()),
                Err(e) => status.set(format!("chain failed: {e}")),
            }
            match actors_with_access(&c).await {
                Ok(serde_json::Value::Array(rows)) => actors.set(rows.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()),
                Ok(_) => actors.set(Vec::new()),
                Err(e) => status.set(format!("actors failed: {e}")),
            }
        });
    };
    let do_trace = move |_| {
        let fp = leak_fp();
        spawn(async move {
            match trace_leak(&fp).await {
                Ok(v) => {
                    let ev = v.get("event").cloned().unwrap_or(serde_json::Value::Null);
                    if ev.is_null() {
                        leak_result.set("No disclosure matches that fingerprint.".into());
                    } else {
                        let actor = ev.get("acting_delegate_did").and_then(|x| x.as_str())
                            .unwrap_or_else(|| ev.get("recipient_did").and_then(|x| x.as_str()).unwrap_or("?"));
                        leak_result.set(format!("Leak traced → accountable actor: {actor}"));
                    }
                }
                Err(e) => leak_result.set(format!("trace failed: {e}")),
            }
        });
    };
    let do_assess = move |_| {
        let (act, actor, means, accessible, checked, acted, injury) =
            (q_act(), q_actor(), q_means(), q_accessible(), q_checked(), q_acted(), q_injury());
        spawn(async move {
            let means_ids = csv(&means);
            let accessible_ids = csv(&accessible);
            let expected_means: Vec<_> = means_ids
                .iter()
                .map(|id| serde_json::json!({ "id": id, "description": id, "accessible": accessible_ids.contains(id) }))
                .collect();
            let duty = serde_json::json!({ "act": act, "expected_means": expected_means }).to_string();
            let conduct = serde_json::json!({
                "actor_did": actor,
                "checked_means_ids": csv(&checked),
                "acted": acted,
                "caused_further_injury": injury,
            })
            .to_string();
            match assess_duty_of_inquiry(&duty, &conduct).await {
                Ok(v) => q_verdict.set(v),
                Err(e) => status.set(format!("assess failed: {e}")),
            }
        });
    };

    let field_style = "padding:0.3rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;background:var(--qualia-surface-2,#fff);";
    let label_style = "display:flex;flex-direction:column;gap:0.15rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);";
    let btn_primary = "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.78rem;cursor:pointer;";
    let btn_plain = "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;";
    let (vlabel, vcolor) = verdict_label(&q_verdict());

    rsx! {
        section {
            aria_label: "WellFair disclosure and inquiry",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.9rem;",
            super::shared::DomainChrome { domain: "Care", chip: "Rights · disclosure · inquiry duty", show_memory: true }
            h2 { style: "margin:0;font-size:1rem;", "Disclosure traceability & duty of inquiry" }
            if !status().is_empty() {
                p { style: "margin:0;font-size:0.74rem;color:var(--qualia-accent,#2b6);word-break:break-all;", "{status()}" }
            }

            // ── Disclosure traceability ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.55rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.85rem;font-weight:600;", "Disclosure traceability" }
                p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                    "Record who you informed and who accessed what (incl. which staffer acted), so a leak is traceable + attributable."
                }
                label { style: "{label_style}", "Transparency cc — credential id"
                    input { style: "{field_style}", value: "{cc_credential}", oninput: move |e| cc_credential.set(e.value()) } }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    label { style: "{label_style}", "Informed authority DID"
                        input { style: "{field_style}", value: "{cc_authority}", oninput: move |e| cc_authority.set(e.value()) } }
                    label { style: "{label_style}", "Purpose"
                        input { style: "{field_style}", value: "{cc_purpose}", oninput: move |e| cc_purpose.set(e.value()) } }
                }
                button { style: "{btn_plain}", onclick: do_cc, "Record cc" }

                label { style: "{label_style}", "Payload commitment (hex)"
                    input { style: "{field_style}", value: "{d_commitment}", oninput: move |e| d_commitment.set(e.value()) } }
                label { style: "{label_style}", "Under credential id"
                    input { style: "{field_style}", value: "{d_credential}", oninput: move |e| d_credential.set(e.value()) } }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    label { style: "{label_style}", "Recipient DID"
                        input { style: "{field_style}", value: "{d_recipient}", oninput: move |e| d_recipient.set(e.value()) } }
                    label { style: "{label_style}", "Acting delegate DID (optional)"
                        input { style: "{field_style}", value: "{d_delegate}", oninput: move |e| d_delegate.set(e.value()) } }
                    label { style: "{label_style}", "Onward-share to (optional)"
                        input { style: "{field_style}", value: "{d_onward}", oninput: move |e| d_onward.set(e.value()) } }
                }
                div { style: "display:flex;gap:0.4rem;flex-wrap:wrap;",
                    button { style: "{btn_primary}", onclick: do_disclosure, "Record disclosure" }
                    button { style: "{btn_plain}", onclick: do_view, "View chain + actors" }
                }
                if !actors.read().is_empty() {
                    div { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                        "Actors with access (leak must be within): "
                        strong { "{actors.read().join(\", \")}" }
                    }
                }
                if !chain.read().is_empty() {
                    ul { style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.2rem;",
                        for ev in chain.read().clone() {
                            li { style: "font-size:0.7rem;color:var(--qualia-text-muted,#666);font-family:monospace;",
                                "{str_field(&ev, \"recipient_did\")}"
                                if let Some(d) = ev.get("acting_delegate_did").and_then(|x| x.as_str()) { " via {d}" }
                                " · fp {ev.get(\"fingerprint\").map(|x| x.to_string()).unwrap_or_default()}"
                            }
                        }
                    }
                }
                div { style: "display:flex;gap:0.4rem;align-items:flex-end;flex-wrap:wrap;",
                    label { style: "{label_style}", "Trace a leak — fingerprint (hex)"
                        input { style: "{field_style}", value: "{leak_fp}", oninput: move |e| leak_fp.set(e.value()) } }
                    button { style: "{btn_plain}", onclick: do_trace, "Trace" }
                }
                if !leak_result().is_empty() {
                    p { style: "margin:0;font-size:0.74rem;color:var(--qualia-danger,#b44);", "{leak_result()}" }
                }
            }

            // ── Duty of inquiry ──
            div {
                style: "display:flex;flex-direction:column;gap:0.35rem;padding:0.55rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div { style: "font-size:0.85rem;font-weight:600;", "Duty of inquiry (fair negligence test)" }
                p { style: "margin:0;font-size:0.7rem;color:var(--qualia-text-muted,#777);",
                    "Negligence = failing to check an ACCESSIBLE means, then acting to cause harm. If the means weren't accessible, there's no fault — the actor couldn't have known."
                }
                label { style: "{label_style}", "Consequential act"
                    input { style: "{field_style}", value: "{q_act}", oninput: move |e| q_act.set(e.value()) } }
                label { style: "{label_style}", "Actor DID"
                    input { style: "{field_style}", value: "{q_actor}", oninput: move |e| q_actor.set(e.value()) } }
                label { style: "{label_style}", "Expected means (CSV of ids)"
                    input { style: "{field_style}", value: "{q_means}", oninput: move |e| q_means.set(e.value()) } }
                label { style: "{label_style}", "Which were accessible (CSV)"
                    input { style: "{field_style}", value: "{q_accessible}", oninput: move |e| q_accessible.set(e.value()) } }
                label { style: "{label_style}", "Which were actually checked (CSV)"
                    input { style: "{field_style}", value: "{q_checked}", oninput: move |e| q_checked.set(e.value()) } }
                div { style: "display:flex;gap:0.8rem;flex-wrap:wrap;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                    label { style: "display:flex;align-items:center;gap:0.3rem;",
                        input { r#type: "checkbox", checked: q_acted(), onchange: move |e| q_acted.set(e.checked()) }
                        "took the act"
                    }
                    label { style: "display:flex;align-items:center;gap:0.3rem;",
                        input { r#type: "checkbox", checked: q_injury(), onchange: move |e| q_injury.set(e.checked()) }
                        "caused further injury"
                    }
                }
                button { style: "{btn_primary}", onclick: do_assess, "Assess" }
                if !q_verdict().is_empty() {
                    div { style: "padding:0.4rem 0.55rem;border-radius:6px;color:#fff;font-size:0.76rem;background:{vcolor};", "{vlabel}" }
                }
                p { style: "margin:0;font-size:0.66rem;color:var(--qualia-text-muted,#999);",
                    "A proposal over evidence, never an automated verdict — a court / ombudsman decides."
                }
            }
        }
    }
}
