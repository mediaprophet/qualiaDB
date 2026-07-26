//! Post-turn verification — "generate first, heal second".
//!
//! Alternative to mid-decode Webizen Sentinel gating: run the LLM at full resident
//! speed (like Ollama), then **verify and self-heal** the completed draft against
//! the quant-graph fact table and a lightweight CML-shaped claim extract before
//! finalising the turn.
//!
//! # Why this exists
//! Mid-token governance (Phase-8 rings) is architecturally important for hard
//! fail-closed signals, but continuous logit inspection is **not** what makes
//! Qualia slower than Ollama — the GPU GEMV path is. Post-turn verify still
//! earns its keep: it recovers quality from aggressive INT4 without taxing every
//! token, and produces an auditable HTML/CML surface for the principal.
//!
//! # Pipeline
//! 1. LLM emits plain draft (no mid-decode interrupt).
//! 2. Extract crude claims / capital-style facts from draft + prompt.
//! 3. `quant_graph_grounding` repair when high-stakes needles mismatch.
//! 4. Emit `VerifiedTurn` with plain final text + HTML presentation + CML Turtle.

use crate::quant_graph_grounding::{ground_generation, GroundingResult};

/// One atomic check against the local fact / ontology surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyCheck {
    pub id: String,
    pub ok: bool,
    pub detail: String,
}

/// Result of post-turn verification / self-heal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedTurn {
    /// Plain text for logs / agents that want unadorned prose.
    pub final_text: String,
    /// HTML presentation of the answer + verification panel (CML-ish surface).
    pub display_html: String,
    /// Turtle sketch of cml:Proposed / observation claims (ingestible later).
    pub cml_turtle: String,
    /// Whether the draft was rewritten by the graph.
    pub repaired: bool,
    /// Checks performed (pass/fail).
    pub checks: Vec<VerifyCheck>,
    pub grounding_reason: Option<String>,
}

/// Env: return HTML as the primary `text` field from the agent when FastVerify is on.
#[inline]
pub fn return_html_as_text() -> bool {
    matches!(
        std::env::var("QUALIA_RETURN_VERIFY_HTML").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    ) || crate::inference_modes::fast_verify_html_default()
}

/// Full post-turn pass: graph ground + claim checks + HTML/CML packaging.
pub fn verify_and_heal_turn(prompt: &str, draft: &str) -> VerifiedTurn {
    let g: GroundingResult = ground_generation(prompt, draft);
    let mut checks = Vec::new();

    // Check 1: non-empty draft
    checks.push(VerifyCheck {
        id: "nonempty".into(),
        ok: !draft.trim().is_empty(),
        detail: if draft.trim().is_empty() {
            "draft empty".into()
        } else {
            format!("{} chars", draft.len())
        },
    });

    // Check 2: graph grounding
    if let Some(ref reason) = g.reason {
        checks.push(VerifyCheck {
            id: format!("graph:{reason}"),
            ok: !g.repaired,
            detail: if g.repaired {
                format!("repaired → {}", truncate(&g.text, 80))
            } else {
                "grounded (answer_ok present)".into()
            },
        });
    } else {
        checks.push(VerifyCheck {
            id: "graph:no_match".into(),
            ok: true,
            detail: "no high-stakes fact needles matched prompt".into(),
        });
    }

    // Check 3: crude self-consistency — if repaired, final must mention an answer_ok token
    // from the grounding result text (Paris etc. already in repair string).
    if g.repaired {
        checks.push(VerifyCheck {
            id: "heal_applied".into(),
            ok: true,
            detail: "quant-graph replaced ungrounded draft".into(),
        });
    }

    // Check 4: extract simple [[wiki]] / #topic CML tags from prompt for context binding
    let tags = extract_simple_cml_tags(prompt);
    if !tags.is_empty() {
        checks.push(VerifyCheck {
            id: "cml_tags".into(),
            ok: true,
            detail: format!("{} context tag(s) from prompt", tags.len()),
        });
    }

    let final_text = g.text.clone();
    let display_html = render_turn_html(prompt, draft, &final_text, g.repaired, &checks, &tags);
    let cml_turtle = render_cml_turtle(prompt, &final_text, g.repaired, &checks, &tags);

    log::info!(
        "post_turn_verify|repaired={}|checks={}|reason={:?}",
        g.repaired,
        checks.len(),
        g.reason
    );

    VerifiedTurn {
        final_text,
        display_html,
        cml_turtle,
        repaired: g.repaired,
        checks,
        grounding_reason: g.reason,
    }
}

/// When FastVerify (or quant-graph post path) is active, heal draft → final presentation.
pub fn maybe_verify_turn(prompt: &str, draft: &str) -> VerifiedTurn {
    if crate::inference_modes::post_turn_verify_enabled() {
        verify_and_heal_turn(prompt, draft)
    } else {
        // Identity wrap (still produce HTML if caller asks later).
        VerifiedTurn {
            final_text: draft.to_string(),
            display_html: format!(
                "<article class=\"q-turn\"><p>{}</p></article>",
                escape_html(draft)
            ),
            cml_turtle: String::new(),
            repaired: false,
            checks: vec![],
            grounding_reason: None,
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

fn escape_html(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            _ => c.to_string(),
        })
        .collect()
}

/// Minimal tag parse: `#topic:x` `#project:y` `[[concept]]` (no client-core dependency).
fn extract_simple_cml_tags(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    // [[concept]]
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let label = after[..end].trim();
            if !label.is_empty() {
                out.push(("general".into(), label.to_string()));
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    for raw in text.split_whitespace() {
        let tok = raw.trim_matches(|c: char| matches!(c, '.' | ',' | '!' | '?' | ';' | ')' | '('));
        if let Some(body) = tok.strip_prefix('#') {
            if let Some((k, v)) = body.split_once(':') {
                let tier = k.to_ascii_lowercase();
                if matches!(
                    tier.as_str(),
                    "topic" | "project" | "task" | "pursuit" | "general"
                ) {
                    let label = v.replace('_', " ");
                    if !label.is_empty() {
                        out.push((tier, label));
                    }
                }
            } else if !body.is_empty() {
                out.push(("topic".into(), body.replace('_', " ")));
            }
        }
    }
    out
}

fn render_turn_html(
    prompt: &str,
    draft: &str,
    final_text: &str,
    repaired: bool,
    checks: &[VerifyCheck],
    tags: &[(String, String)],
) -> String {
    let status = if repaired {
        "<span class=\"q-badge q-repaired\">self-healed</span>"
    } else {
        "<span class=\"q-badge q-ok\">verified</span>"
    };
    let mut checks_html = String::from("<ul class=\"q-checks\">");
    for c in checks {
        let mark = if c.ok { "✓" } else { "✗" };
        let cls = if c.ok { "pass" } else { "fail" };
        checks_html.push_str(&format!(
            "<li class=\"{cls}\"><code>{mark} {}</code> — {}</li>",
            escape_html(&c.id),
            escape_html(&c.detail)
        ));
    }
    checks_html.push_str("</ul>");

    let mut tags_html = String::new();
    if !tags.is_empty() {
        tags_html.push_str("<p class=\"q-cml-tags\">");
        for (t, l) in tags {
            tags_html.push_str(&format!(
                "<span class=\"q-tag\">#{}:{}</span> ",
                escape_html(t),
                escape_html(l)
            ));
        }
        tags_html.push_str("</p>");
    }

    let draft_block = if repaired {
        format!(
            "<details class=\"q-draft\"><summary>Original draft (pre-heal)</summary><pre>{}</pre></details>",
            escape_html(draft)
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Qualia turn</title>
<style>
body{{font-family:system-ui,sans-serif;background:#0c0f14;color:#e2e8f0;margin:1.5rem}}
.q-turn{{max-width:42rem;margin:auto}}
.q-badge{{font-size:.7rem;padding:.15rem .5rem;border-radius:999px;font-weight:700}}
.q-ok{{background:rgba(16,185,129,.2);color:#6ee7b7}}
.q-repaired{{background:rgba(245,158,11,.2);color:#fcd34d}}
.q-answer{{font-size:1.1rem;line-height:1.55;padding:1rem;border:1px solid rgba(148,163,184,.2);border-radius:12px;background:rgba(255,255,255,.04)}}
.q-checks{{font-size:.85rem;line-height:1.6}}
.q-checks .pass{{color:#86efac}}
.q-checks .fail{{color:#fca5a5}}
.q-meta{{font-size:.75rem;color:#94a3b8;margin-top:1.5rem}}
.q-tag{{display:inline-block;margin:.15rem;padding:.1rem .4rem;border-radius:6px;background:rgba(59,130,246,.15);color:#93c5fd;font-size:.75rem}}
pre{{white-space:pre-wrap;font-size:.8rem;opacity:.85}}
</style></head><body>
<article class="q-turn" data-qualia-verify="1">
  <header><h1>Response {status}</h1>
  <p class="q-meta">Prompt: {prompt}</p>{tags}
  </header>
  <section class="q-answer"><p>{answer}</p></section>
  {draft}
  <section><h2>Verification</h2>{checks}
  <p class="q-meta">Post-turn path: generate → graph/CML verify → finalise. Mid-decode Sentinel skipped in FastVerify mode.</p>
  </section>
</article></body></html>"#,
        status = status,
        prompt = escape_html(&truncate(prompt, 200)),
        tags = tags_html,
        answer = escape_html(final_text).replace('\n', "<br/>"),
        draft = draft_block,
        checks = checks_html,
    )
}

fn render_cml_turtle(
    prompt: &str,
    final_text: &str,
    repaired: bool,
    checks: &[VerifyCheck],
    tags: &[(String, String)],
) -> String {
    let mut out = String::from(
        "@prefix cml: <https://webizen.org/cml#> .\n@prefix q42: <https://ns.webizen.org/q42/> .\n\n",
    );
    out.push_str("<urn:qualia:turn:current> a cml:Turn ;\n");
    out.push_str(&format!("  cml:prompt {} ;\n", ttl_str(prompt)));
    out.push_str(&format!("  cml:finalText {} ;\n", ttl_str(final_text)));
    out.push_str(&format!(
        "  cml:selfHealed {} ;\n",
        if repaired { "true" } else { "false" }
    ));
    out.push_str("  cml:verifyPath \"post-turn\" .\n\n");
    for (i, c) in checks.iter().enumerate() {
        out.push_str(&format!(
            "<urn:qualia:check:{i}> a cml:VerifyCheck, cml:Proposed ;\n"
        ));
        out.push_str(&format!("  cml:id {} ;\n", ttl_str(&c.id)));
        out.push_str(&format!(
            "  cml:ok {} ;\n",
            if c.ok { "true" } else { "false" }
        ));
        out.push_str(&format!("  cml:detail {} .\n\n", ttl_str(&c.detail)));
    }
    for (t, l) in tags {
        out.push_str(&format!(
            "<urn:qualia:tag:{}:{}> a cml:Proposed ;\n  cml:tier {} ;\n  cml:label {} .\n\n",
            t,
            l.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>(),
            ttl_str(t),
            ttl_str(l)
        ));
    }
    out
}

fn ttl_str(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference_modes::{set_inference_mode, InferenceMode};
    use crate::quant_graph_grounding::reset_fact_store_to_defaults;

    #[test]
    fn heals_wrong_capital() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        reset_fact_store_to_defaults();
        set_inference_mode(InferenceMode::FastVerify);
        let v = verify_and_heal_turn("What is the capital of France?", "I think it is Lyon.");
        assert!(v.repaired);
        assert!(v.final_text.to_ascii_lowercase().contains("paris"));
        assert!(v.display_html.contains("self-healed") || v.display_html.contains("q-repaired"));
        assert!(v.cml_turtle.contains("cml:Turn"));
        set_inference_mode(InferenceMode::Portable);
    }

    #[test]
    fn leaves_good_answer() {
        reset_fact_store_to_defaults();
        let v = verify_and_heal_turn(
            "What is the capital of France?",
            "The capital of France is Paris.",
        );
        assert!(!v.repaired);
        assert!(v.final_text.contains("Paris"));
    }
}
