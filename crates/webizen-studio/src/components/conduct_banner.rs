//! Conduct / deny banner — rights gate and shield denials must never be silent.
//!
//! Surfaces `ChatInferenceResult.block_reason` / `shield_alert` (and optional
//! `conduct-violation` events) as a dedicated red/amber strip that cannot be
//! missed. Dismissible; reappears on the next deny. U1-B.

#![allow(non_snake_case)]
use dioxus::prelude::*;

/// Visual severity for conduct / gate denials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConductKind {
    /// Intent/output gate deny or uncommitted inference with a reason.
    GateDeny,
    /// Sentinel / axiom shield alert (anachronism, bounds, etc.).
    ShieldAlert,
    /// Inference blocked for another recorded reason (no model, cancel, …).
    InferenceBlock,
}

impl ConductKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::GateDeny => "Gate deny",
            Self::ShieldAlert => "Shield alert",
            Self::InferenceBlock => "No reply",
        }
    }

    /// (background, border, text, accent pill bg)
    pub fn colours(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Self::GateDeny => ("#450a0a", "#ef4444", "#fecaca", "#7f1d1d"),
            Self::ShieldAlert => ("#78350f", "#f59e0b", "#fde68a", "#92400e"),
            Self::InferenceBlock => ("#7c2d12", "#fb923c", "#ffedd5", "#9a3412"),
        }
    }
}

/// One visible conduct notice (reason + kind).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConductNotice {
    pub kind: ConductKind,
    pub reason: String,
}

impl ConductNotice {
    pub fn gate_deny(reason: impl Into<String>) -> Self {
        Self {
            kind: ConductKind::GateDeny,
            reason: reason.into(),
        }
    }

    pub fn shield_alert(reason: impl Into<String>) -> Self {
        Self {
            kind: ConductKind::ShieldAlert,
            reason: reason.into(),
        }
    }

    pub fn inference_block(reason: impl Into<String>) -> Self {
        Self {
            kind: ConductKind::InferenceBlock,
            reason: reason.into(),
        }
    }
}

/// Build a notice from a `ChatInferenceResult` JSON object (invoke return or
/// `chat-done.result`). Returns `None` when the turn committed cleanly with no shield.
pub fn notice_from_chat_result(result: &serde_json::Value) -> Option<ConductNotice> {
    let shield = result
        .get("shield_alert")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let committed = result
        .get("committed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let reason = result
        .get("block_reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if shield {
        let text = reason.unwrap_or_else(|| {
            "Shield / Sentinel blocked this turn (axiom bounds or anomaly)."
                .to_string()
        });
        return Some(ConductNotice::shield_alert(text));
    }

    if let Some(text) = reason {
        // Heuristic: gate language → GateDeny; otherwise inference block.
        let lower = text.to_ascii_lowercase();
        let kind = if lower.contains("intent")
            || lower.contains("gate")
            || lower.contains("deny")
            || lower.contains("forbid")
            || lower.contains("deontic")
            || lower.contains("ungrounded")
            || lower.contains("provenance")
            || lower.contains("conduct")
            || lower.contains("rights")
        {
            ConductKind::GateDeny
        } else {
            ConductKind::InferenceBlock
        };
        return Some(ConductNotice {
            kind,
            reason: text,
        });
    }

    // Uncommitted with no reason still must not be silent.
    if !committed {
        return Some(ConductNotice::inference_block(
            "Inference did not commit — no block_reason from host.",
        ));
    }

    None
}

/// Parse `chat-done` event payload: `{ session_id, committed, result }`
/// (optionally nested under Tauri's `payload` key — caller unwraps).
pub fn notice_from_chat_done(payload: &serde_json::Value) -> Option<ConductNotice> {
    let result = payload.get("result").unwrap_or(payload);
    notice_from_chat_result(result)
}

/// Parse optional `conduct-violation` event: `{ reason }` or `{ reason, summary }`.
pub fn notice_from_conduct_violation(payload: &serde_json::Value) -> Option<ConductNotice> {
    let reason = payload
        .get("reason")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("summary").and_then(|v| v.as_str()))
        .or_else(|| payload.get("message").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "Conduct violation recorded (no reason text).".to_string());
    Some(ConductNotice::gate_deny(reason))
}

/// Full-width banner: red/amber strip with dismiss control.
///
/// Pass `None` to hide. Dismiss only clears the current notice; the next deny
/// re-sets the signal and the banner reappears.
#[component]
pub fn ConductBanner(
    notice: Option<ConductNotice>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let Some(n) = notice else {
        return rsx! {};
    };
    let (bg, border, fg, pill) = n.kind.colours();
    let title = n.kind.title();
    let reason = n.reason.clone();
    let style = format!(
        "display:flex; align-items:flex-start; justify-content:space-between; gap:12px; \
         background:{bg}; border-bottom:2px solid {border}; color:{fg}; \
         padding:10px 18px; font-size:13px; line-height:1.45; box-sizing:border-box;"
    );
    let pill_style = format!(
        "display:inline-block; font-size:10px; font-weight:700; letter-spacing:0.06em; \
         text-transform:uppercase; background:{pill}; color:{fg}; padding:2px 8px; \
         border-radius:4px; border:1px solid {border}; white-space:nowrap;"
    );
    let dismiss_style = format!(
        "flex-shrink:0; background:transparent; color:{fg}; border:1px solid {border}; \
         border-radius:6px; padding:4px 10px; font-size:11px; font-weight:600; cursor:pointer;"
    );

    rsx! {
        div {
            role: "alert",
            "aria-live": "assertive",
            style: "{style}",
            div { style: "display:flex; flex-direction:column; gap:4px; min-width:0; flex:1;",
                div { style: "display:flex; align-items:center; gap:8px; flex-wrap:wrap;",
                    span { style: "{pill_style}", "{title}" }
                    span {
                        style: "font-weight:700; font-size:12px; letter-spacing:0.02em;",
                        "Rights / governance — not silent"
                    }
                }
                p {
                    style: "margin:0; white-space:pre-wrap; word-break:break-word; font-size:13px;",
                    "{reason}"
                }
            }
            button {
                style: "{dismiss_style}",
                title: "Dismiss this notice (it will return on the next deny)",
                onclick: move |_| on_dismiss.call(()),
                "Dismiss"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shield_alert_wins_over_block_reason() {
        let v = json!({
            "committed": false,
            "shield_alert": true,
            "block_reason": "Shield — anachronism term"
        });
        let n = notice_from_chat_result(&v).expect("notice");
        assert_eq!(n.kind, ConductKind::ShieldAlert);
        assert!(n.reason.contains("Shield"));
    }

    #[test]
    fn gate_language_maps_to_gate_deny() {
        let v = json!({
            "committed": false,
            "shield_alert": false,
            "block_reason": "Intent gate deny: rights ontology Forbid"
        });
        let n = notice_from_chat_result(&v).expect("notice");
        assert_eq!(n.kind, ConductKind::GateDeny);
    }

    #[test]
    fn uncommitted_without_reason_is_not_silent() {
        let v = json!({ "committed": false, "shield_alert": false });
        let n = notice_from_chat_result(&v).expect("notice");
        assert_eq!(n.kind, ConductKind::InferenceBlock);
    }

    #[test]
    fn committed_clean_returns_none() {
        let v = json!({
            "committed": true,
            "shield_alert": false,
            "block_reason": null
        });
        assert!(notice_from_chat_result(&v).is_none());
    }

    #[test]
    fn chat_done_nested_result() {
        let v = json!({
            "session_id": "s1",
            "committed": false,
            "result": {
                "committed": false,
                "shield_alert": false,
                "block_reason": "No active model"
            }
        });
        let n = notice_from_chat_done(&v).expect("notice");
        assert_eq!(n.kind, ConductKind::InferenceBlock);
        assert!(n.reason.contains("No active model"));
    }

    #[test]
    fn conduct_violation_payload() {
        let v = json!({ "reason": "capability scope breach" });
        let n = notice_from_conduct_violation(&v).expect("notice");
        assert_eq!(n.kind, ConductKind::GateDeny);
        assert!(n.reason.contains("capability"));
    }
}
