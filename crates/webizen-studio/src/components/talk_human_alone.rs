//! Human-alone Talk policy — people first; instruments stay tools.
//!
//! ConnectChat must work like a basic messaging peer (Messages / Signal bar):
//! a person can send and receive without an agent remote-driving the thread.
//! A missing model is a **held / not yet** instrument path — teachable, never
//! red-broken, never a peer-person chatbot. No Host invent: callers use
//! existing cmds (`append_chat_message`, `stream_chat_inference`, `mesh_*`,
//! `ChatGraph.*` where already bound).

/// Talk chrome hold — mapped onto `HonestyChip` in ConnectChat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkHoldLevel {
    /// Wait-honest: missing model / mesh not started.
    Held,
    /// Live with caveats (instrument under principal, mesh on).
    Partial,
}

impl TalkHoldLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Held => "held / not yet",
            Self::Partial => "Partial",
        }
    }

    pub fn bg_is_panic_red(self) -> bool {
        false
    }
}

/// Session title when the human starts a thread without naming a peer.
pub const DEFAULT_CONVERSATION_TITLE: &str = "Conversation";

/// Composer placeholder — always a person-to-person write box.
pub const COMPOSER_PLACEHOLDER: &str =
    "Write a message… (Enter to send, Shift+Enter for a line)";

/// Brief status after a human message is persisted.
pub const SENT_SAYABLE: &str = "Sent.";

/// Header / empty-state copy when Talk is ready for people.
pub const TALK_PEOPLE_BLURB: &str =
    "Messages with people. An instrument is a tool you can ask — not the other party.";

/// Teachable held path when no local model is active.
pub const INSTRUMENT_HELD_SAYABLE: &str =
    "Instrument · held / not yet — choose and test a local model in Settings → AI instruments. Messages still send.";

/// Compact chip when no model is active.
pub const INSTRUMENT_HELD_CHIP: &str = "Instrument · held / not yet";

/// Status while a gated local instrument is actually running.
pub const INSTRUMENT_WORKING_SAYABLE: &str = "Instrument working…";

/// Empty-thread invite — works with or without a model.
pub const EMPTY_THREAD_INVITE: &str =
    "Type below and press Send — a conversation starts if needed. Ask an instrument only when you want a tool.";

/// Mesh not running: wait-honest, not broken.
pub const MESH_HELD_SAYABLE: &str =
    "Mesh · held / not yet — start in People when you need someone on another machine.";

/// New-conversation control — never “chat with your agent”.
pub const NEW_CONVERSATION_LABEL: &str = "＋ New conversation";

/// Answering / instrument picker empty option — people only.
pub const PEOPLE_ONLY_LABEL: &str = "People in this thread";

/// How Send should treat this draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalkSendKind {
    /// Persist the human message. Do not call inference.
    HumanOnly,
    /// Persist, then optionally invoke `stream_chat_inference` for these slugs
    /// (`None` = default local instrument).
    AskInstrument { slugs: Vec<Option<String>> },
}

/// Classify a send. `@slug` that matches the roster is an explicit tool ask.
/// Unknown `@tokens` stay in the human text. `ask_instrument` is the Ask-tool
/// control. A selected `active_agent` never auto-drives plain Send.
pub fn classify_talk_send(
    body: &str,
    ask_instrument: bool,
    active_agent: &str,
    mentioned_slugs: &[String],
) -> TalkSendKind {
    if !mentioned_slugs.is_empty() {
        return TalkSendKind::AskInstrument {
            slugs: mentioned_slugs.iter().cloned().map(Some).collect(),
        };
    }
    if ask_instrument {
        let slug = if active_agent.is_empty() {
            None
        } else {
            Some(active_agent.to_string())
        };
        return TalkSendKind::AskInstrument { slugs: vec![slug] };
    }
    let _ = body;
    TalkSendKind::HumanOnly
}

/// Roster `@slug` mentions in first-mention order. Unknown handles are ignored
/// so a person can address people without blocking send.
pub fn mentioned_instrument_slugs(body: &str, roster: &[serde_json::Value]) -> Vec<String> {
    let mut requested = Vec::new();
    for token in body.split_whitespace() {
        let Some(rest) = token.strip_prefix('@') else {
            continue;
        };
        let slug = rest
            .trim_matches(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-')
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let known = roster.iter().any(|agent| {
            agent
                .get("slug")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s == slug)
        });
        if !known {
            continue;
        }
        if !requested.iter().any(|known| known == &slug) {
            requested.push(slug);
        }
        if requested.len() >= 4 {
            break;
        }
    }
    requested
}

/// Instrument honesty for Talk chrome. Missing model → held, never “Needs model”
/// / “Instrument none”, never panic red.
pub fn instrument_honesty(active_model: &str) -> (TalkHoldLevel, String, String) {
    if active_model.trim().is_empty() {
        (
            TalkHoldLevel::Held,
            INSTRUMENT_HELD_SAYABLE.to_string(),
            INSTRUMENT_HELD_CHIP.to_string(),
        )
    } else {
        (
            TalkHoldLevel::Partial,
            "Instrument under principal — a tool, not a peer person".to_string(),
            format!("Instrument · {active_model}"),
        )
    }
}

/// Mesh honesty from existing `mesh_status` fields.
pub fn mesh_honesty(running: bool, peer_count: usize) -> (TalkHoldLevel, String) {
    if running {
        (
            TalkHoldLevel::Partial,
            format!("Mesh · on · {peer_count} peer(s)"),
        )
    } else {
        (TalkHoldLevel::Held, MESH_HELD_SAYABLE.to_string())
    }
}

/// True when Talk should call `stream_chat_inference` after persist.
pub fn should_invoke_instrument(kind: &TalkSendKind, has_model: bool) -> bool {
    matches!(kind, TalkSendKind::AskInstrument { .. }) && has_model
}

/// Held sayable when the human asked a tool but no model is active.
pub fn instrument_held_instead_of_broken() -> &'static str {
    INSTRUMENT_HELD_SAYABLE
}

/// Copy must never use these banned Talk voices.
pub fn talk_copy_is_human_alone(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    !lower.contains("needs model")
        && !lower.contains("instrument · none")
        && !lower.contains("instrument none")
        && !lower.contains("unavailable")
        && !lower.contains("chat with your agent")
        && !lower.contains("message your agent")
        && !lower.contains("your agent is thinking")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(slug: &str) -> serde_json::Value {
        serde_json::json!({ "slug": slug, "display_name": slug })
    }

    #[test]
    fn plain_send_is_human_only_even_with_active_agent() {
        let kind = classify_talk_send("hello", false, "local-helper", &[]);
        assert_eq!(kind, TalkSendKind::HumanOnly);
        assert!(!should_invoke_instrument(&kind, true));
        assert!(!should_invoke_instrument(&kind, false));
    }

    #[test]
    fn ask_instrument_without_mention_uses_active_or_default() {
        let kind = classify_talk_send("summarise this", true, "local-helper", &[]);
        assert_eq!(
            kind,
            TalkSendKind::AskInstrument {
                slugs: vec![Some("local-helper".into())]
            }
        );
        assert!(should_invoke_instrument(&kind, true));
        assert!(
            !should_invoke_instrument(&kind, false),
            "missing model stays held — no inference"
        );
        let defaulted = classify_talk_send("summarise this", true, "", &[]);
        assert_eq!(
            defaulted,
            TalkSendKind::AskInstrument {
                slugs: vec![None]
            }
        );
    }

    #[test]
    fn known_mention_invokes_instrument_unknown_stays_human() {
        let roster = vec![agent("helper")];
        let mentioned = mentioned_instrument_slugs("hi @alice and @helper please", &roster);
        assert_eq!(mentioned, vec!["helper".to_string()]);
        let kind = classify_talk_send("hi @alice and @helper please", false, "", &mentioned);
        assert!(matches!(kind, TalkSendKind::AskInstrument { slugs } if slugs == vec![Some("helper".into())]));
        let none = mentioned_instrument_slugs("hi @alice", &roster);
        assert!(none.is_empty());
        assert_eq!(
            classify_talk_send("hi @alice", false, "helper", &none),
            TalkSendKind::HumanOnly
        );
    }

    #[test]
    fn missing_model_chip_is_held_not_needs_model() {
        let (level, detail, chip) = instrument_honesty("");
        assert_eq!(level, TalkHoldLevel::Held);
        assert_eq!(level.label(), "held / not yet");
        assert_eq!(chip, INSTRUMENT_HELD_CHIP);
        assert!(talk_copy_is_human_alone(&detail));
        assert!(talk_copy_is_human_alone(&chip));
        assert!(!level.bg_is_panic_red());
    }

    #[test]
    fn active_model_chip_names_tool_not_peer() {
        let (level, detail, chip) = instrument_honesty("smollm2");
        assert_eq!(level, TalkHoldLevel::Partial);
        assert!(chip.contains("smollm2"));
        assert!(detail.contains("not a peer"));
        assert!(talk_copy_is_human_alone(&detail));
    }

    #[test]
    fn mesh_stopped_is_held_not_unavailable_word() {
        let (level, text) = mesh_honesty(false, 0);
        assert_eq!(level.label(), "held / not yet");
        assert!(talk_copy_is_human_alone(&text));
        let (on, live) = mesh_honesty(true, 2);
        assert_eq!(on, TalkHoldLevel::Partial);
        assert!(live.contains("2 peer"));
    }

    #[test]
    fn session_and_composer_copy_is_human_alone() {
        assert_eq!(DEFAULT_CONVERSATION_TITLE, "Conversation");
        for s in [
            DEFAULT_CONVERSATION_TITLE,
            COMPOSER_PLACEHOLDER,
            SENT_SAYABLE,
            TALK_PEOPLE_BLURB,
            INSTRUMENT_HELD_SAYABLE,
            INSTRUMENT_HELD_CHIP,
            INSTRUMENT_WORKING_SAYABLE,
            EMPTY_THREAD_INVITE,
            MESH_HELD_SAYABLE,
            NEW_CONVERSATION_LABEL,
            PEOPLE_ONLY_LABEL,
            instrument_held_instead_of_broken(),
        ] {
            assert!(talk_copy_is_human_alone(s), "banned voice in: {s}");
        }
    }
}
