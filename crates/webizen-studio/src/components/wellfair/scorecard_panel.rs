//! **Well-being score-card** panel (§G) — the accumulative, *traceable* reading a person can act on.
//!
//! Renders [`WellbeingScorecardReport`](qualia_client_core-side): one card per health-relevant **aspect**
//! (systemic load / stress / resilience / converging factors / combined-effect / physiological demand), each
//! with its coarse **band**, a score bar, and — crucially — the **contribution linkages** that produced it
//! (the opposite of a black box), plus the investigable **hypotheses** as starting points.
//!
//! Framing is load-bearing and shown in the UI: this is the person's **own inward reading**
//! (forum-internum, Sanctuary-class) — a set of *Hypotheses to explore*, **not a diagnosis and not a
//! rating**. `Resilience` reads "higher is better"; every other aspect reads "higher = more to discuss".

use super::host_client::{compute_scorecard, get_weight_model, reset_weight_model, set_weight_model};
use dioxus::prelude::*;

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
fn u64_field(v: &serde_json::Value, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
}

/// Human label for an aspect's snake_case tag (mirrors `Aspect::label`).
fn aspect_label(tag: &str) -> &'static str {
    match tag {
        "systemic_load" => "Systemic load",
        "stress" => "Stress load",
        "resilience" => "Resilience & support",
        "convergence" => "Converging factors",
        "interaction_load" => "Combined-effect load",
        "physiological_demand" => "Physiological demand",
        _ => "Aspect",
    }
}

/// Plain-wording subtitle (mirrors `Aspect::plain_label`).
fn aspect_plain(tag: &str) -> &'static str {
    match tag {
        "systemic_load" => "how much is adding up across your body",
        "stress" => "load on the stress-response systems",
        "resilience" => "what's helping and supporting recovery",
        "convergence" => "several things pointing at the same place",
        "interaction_load" => "things that may combine",
        "physiological_demand" => "the extra work of your current life stage",
        _ => "",
    }
}

fn band_label(band: &str) -> &'static str {
    match band {
        "settled" => "Settled",
        "building" => "Building",
        "heightened" => "Heightened",
        "marked" => "Marked",
        _ => "—",
    }
}

/// Colour for a band. `resilience` inverts the connotation — a high band there is reassuring, not a warning.
fn band_color(band: &str, is_resilience: bool) -> &'static str {
    if is_resilience {
        // supportive scale: more is better (green regardless of height)
        return "#2a9d8f";
    }
    match band {
        "settled" => "#5c9a6f",
        "building" => "#c9a227",
        "heightened" => "#d1791f",
        "marked" => "#b4453a",
        _ => "#999",
    }
}

#[component]
pub fn WellfairScorecardPanel() -> Element {
    let mut report = use_signal(|| serde_json::Value::Null);
    let mut threshold = use_signal(|| 2usize);
    let mut status = use_signal(String::new);
    // The person's own weight model — (system_id, aspect_tag, weight_pct) rows, editable.
    let mut weights = use_signal(Vec::<(String, String, u32)>::new);
    let mut authored = use_signal(|| false);
    let mut show_model = use_signal(|| false);

    let reload = move || {
        let t = threshold();
        spawn(async move {
            match compute_scorecard(t).await {
                Ok(v) => {
                    let mapped = u64_field(&v, "mapped_count");
                    let total = u64_field(&v, "total_records");
                    status.set(if total == 0 {
                        "No records yet — add conditions / medications / diet to build your score-card.".into()
                    } else {
                        format!("{mapped} of {total} record(s) mapped to a factor.")
                    });
                    report.set(v);
                }
                Err(e) => status.set(format!("Score-card unavailable: {e}")),
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let load_model = move || {
        spawn(async move {
            if let Ok(v) = get_weight_model().await {
                authored.set(v.get("authored").and_then(|x| x.as_bool()).unwrap_or(false));
                let rows = v
                    .get("model")
                    .and_then(|m| m.get("system_weights"))
                    .and_then(|w| w.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|w| {
                                (
                                    str_field(w, "system_id"),
                                    str_field(w, "aspect"),
                                    w.get("weight_pct").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                weights.set(rows);
            }
        });
    };
    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        load_model();
    });

    let save_model = move |_| {
        let rows = weights();
        spawn(async move {
            let system_weights: Vec<serde_json::Value> = rows
                .iter()
                .map(|(sys, asp, w)| serde_json::json!({ "system_id": sys, "aspect": asp, "weight_pct": w }))
                .collect();
            let model_json = serde_json::json!({ "system_weights": system_weights }).to_string();
            match set_weight_model(&model_json).await {
                Ok(()) => { status.set("Saved — the card now reads you through your own model.".into()); authored.set(true); reload(); }
                Err(e) => status.set(format!("Save failed: {e}")),
            }
        });
    };
    let reset_model = move |_| {
        spawn(async move {
            match reset_weight_model().await {
                Ok(()) => { status.set("Reset to the starting suggestion.".into()); load_model(); reload(); }
                Err(e) => status.set(format!("Reset failed: {e}")),
            }
        });
    };

    let r = report();
    let disclosure = str_field(&r, "disclosure");
    let empty = Vec::new();
    let aspects = r
        .get("scorecard")
        .and_then(|s| s.get("aspects"))
        .and_then(|a| a.as_array())
        .unwrap_or(&empty)
        .clone();
    let hypotheses = r
        .get("hypotheses")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();

    rsx! {
        section {
            aria_label: "WellFair well-being score-card",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);display:flex;flex-direction:column;gap:0.7rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Well-being score-card" }
                div {
                    style: "display:flex;align-items:center;gap:0.4rem;",
                    label { style: "font-size:0.7rem;color:var(--qualia-text-muted,#777);", "convergence" }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "6",
                        value: "{threshold()}",
                        style: "width:3rem;padding:0.2rem 0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.75rem;",
                        oninput: move |ev| {
                            if let Ok(n) = ev.value().parse::<usize>() { threshold.set(n.clamp(1, 6)); }
                        },
                    }
                    button {
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                        onclick: move |_| reload(),
                        "Refresh"
                    }
                }
            }

            // Forum-internum framing — always shown.
            if !disclosure.is_empty() {
                p {
                    style: "margin:0;font-size:0.72rem;color:var(--qualia-text-muted,#666);padding:0.4rem 0.5rem;border-left:3px solid var(--qualia-accent,#2b6);background:var(--qualia-surface-2,#fff);border-radius:0 6px 6px 0;",
                    "{disclosure}"
                }
            }
            if !status().is_empty() {
                p { style: "margin:0;font-size:0.74rem;color:var(--qualia-text-muted,#777);", "{status()}" }
            }

            // ── The person authors their own interpretive lens ──
            div {
                style: "padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);",
                div {
                    style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;cursor:pointer;",
                    onclick: move |_| { let s = show_model(); show_model.set(!s); },
                    div {
                        strong { style: "font-size:0.82rem;", "How your body is read — your weight model" }
                        div { style: "font-size:0.68rem;color:var(--qualia-text-muted,#999);",
                            if authored() { "Using your own model." } else { "Using the starting suggestion — edit it to make it yours." }
                        }
                    }
                    span { style: "font-size:0.9rem;color:var(--qualia-text-muted,#888);", if show_model() { "▾" } else { "▸" } }
                }
                if show_model() {
                    div {
                        style: "margin-top:0.5rem;display:flex;flex-direction:column;gap:0.3rem;",
                        p { style: "margin:0;font-size:0.68rem;color:var(--qualia-text-muted,#888);",
                            "This is the lens the card reads you through — which body systems weigh into each aspect. It's yours to set. The starting values are only a suggestion, never a verdict; the software shouldn't define you."
                        }
                        for (i, (sys, asp, w)) in weights().into_iter().enumerate() {
                            div {
                                key: "{i}",
                                style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;font-size:0.72rem;",
                                span { "{sys} · {aspect_label(&asp)}" }
                                input {
                                    r#type: "number", min: "0", max: "100",
                                    style: "width:4rem;padding:0.2rem 0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.74rem;",
                                    value: "{w}",
                                    oninput: move |ev| {
                                        if let Ok(n) = ev.value().parse::<u32>() {
                                            let mut ws = weights.write();
                                            if i < ws.len() { ws[i].2 = n.min(100); }
                                        }
                                    },
                                }
                            }
                        }
                        div {
                            style: "display:flex;gap:0.4rem;margin-top:0.3rem;",
                            button {
                                style: "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.76rem;cursor:pointer;",
                                onclick: save_model,
                                "Save my model"
                            }
                            button {
                                style: "padding:0.3rem 0.7rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.76rem;cursor:pointer;",
                                onclick: reset_model,
                                "Reset to suggestion"
                            }
                        }
                    }
                }
            }

            // One card per aspect.
            for a in aspects.clone() {
                {
                    let tag = str_field(&a, "aspect");
                    let band = str_field(&a, "band");
                    let is_resilience = tag == "resilience";
                    let score = u64_field(&a, "score_milli");
                    let pct = (score / 10).min(100);
                    let color = band_color(&band, is_resilience);
                    let contributions = a.get("contributions").and_then(|c| c.as_array()).cloned().unwrap_or_default();
                    rsx! {
                        div {
                            key: "{tag}",
                            style: "padding:0.5rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);background:var(--qualia-surface-2,#fff);display:flex;flex-direction:column;gap:0.3rem;",
                            div {
                                style: "display:flex;justify-content:space-between;align-items:baseline;gap:0.5rem;",
                                div {
                                    strong { style: "font-size:0.85rem;", "{aspect_label(&tag)}" }
                                    div { style: "font-size:0.68rem;color:var(--qualia-text-muted,#999);", "{aspect_plain(&tag)}" }
                                }
                                span {
                                    style: "padding:0.1rem 0.5rem;border-radius:999px;font-size:0.68rem;color:#fff;background:{color};",
                                    "{band_label(&band)}"
                                }
                            }
                            // Score bar.
                            div {
                                style: "height:0.4rem;border-radius:999px;background:var(--qualia-border,#eee);overflow:hidden;",
                                div { style: "height:100%;width:{pct}%;background:{color};" }
                            }
                            // Contribution linkages — the traceability.
                            if contributions.is_empty() {
                                div { style: "font-size:0.68rem;color:var(--qualia-text-muted,#aaa);", "nothing accumulating here" }
                            } else {
                                ul {
                                    style: "margin:0.1rem 0 0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.15rem;",
                                    for c in contributions {
                                        li {
                                            style: "display:flex;justify-content:space-between;gap:0.5rem;font-size:0.7rem;color:var(--qualia-text-muted,#666);",
                                            span { "{str_field(&c, \"label\")}" }
                                            span { style: "font-family:monospace;color:var(--qualia-text-muted,#999);", "{str_field(&c, \"evidence\")}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Investigable hypotheses — the pathway-starts.
            if !hypotheses.is_empty() {
                div {
                    style: "display:flex;flex-direction:column;gap:0.3rem;",
                    div { style: "font-size:0.8rem;font-weight:600;", "Starting points to explore" }
                    p { style: "margin:0;font-size:0.68rem;color:var(--qualia-text-muted,#999);",
                        "Questions to ask, things to track or test, levers you control — not answers, not a diagnosis."
                    }
                    for h in hypotheses {
                        div {
                            key: "{str_field(&h, \"id\")}",
                            style: "padding:0.4rem 0.5rem;border-radius:6px;border:1px dashed var(--qualia-border,#ddd);font-size:0.74rem;",
                            "{str_field(&h, \"label\")}"
                        }
                    }
                }
            }
        }
    }
}
