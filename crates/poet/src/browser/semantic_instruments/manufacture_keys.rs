//! Keyboard-only manufacture path (SI-09).
//!
//! Tab/Arrow moves focus across named ACTIONS; Enter activates the focused
//! step. Publish activates only when [`super::manufacture::can_publish`] is Ok.
//! No Host IDs.

use super::a11y::action_name_ok;
use super::manufacture::{can_publish, next_action, Draft, ACTIONS, NEXT_PUBLISH};

/// Classified manufacture key chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManufactureKey {
    Next,
    Prev,
    Activate,
    Ignore,
}

/// Focus index into [`ACTIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManufactureFocus {
    pub index: usize,
}

impl ManufactureFocus {
    pub fn start() -> Self {
        Self { index: 0 }
    }

    pub fn action_id(self) -> &'static str {
        ACTIONS[self.index.min(ACTIONS.len().saturating_sub(1))].0
    }

    pub fn action_name(self) -> &'static str {
        ACTIONS[self.index.min(ACTIONS.len().saturating_sub(1))].1
    }
}

pub fn classify(key: &str, ctrl: bool) -> ManufactureKey {
    if ctrl {
        return ManufactureKey::Ignore;
    }
    match key {
        "Tab" | "ArrowRight" | "Right" | "ArrowDown" | "Down" => ManufactureKey::Next,
        "ArrowLeft" | "Left" | "ArrowUp" | "Up" => ManufactureKey::Prev,
        "Enter" | " " | "Spacebar" => ManufactureKey::Activate,
        _ => ManufactureKey::Ignore,
    }
}

fn advance(focus: &mut ManufactureFocus, forward: bool) {
    let n = ACTIONS.len();
    if n == 0 {
        return;
    }
    if forward {
        focus.index = (focus.index + 1) % n;
    } else {
        focus.index = focus.index.checked_sub(1).unwrap_or(n - 1);
    }
}

/// Apply a classified key. Returns the next-action / publish status string.
pub fn apply_key(
    focus: &mut ManufactureFocus,
    key: ManufactureKey,
    draft: &mut Draft,
) -> Result<&'static str, &'static str> {
    match key {
        ManufactureKey::Ignore => Ok(next_action(draft)),
        ManufactureKey::Next => {
            advance(focus, true);
            Ok(next_action(draft))
        }
        ManufactureKey::Prev => {
            advance(focus, false);
            Ok(next_action(draft))
        }
        ManufactureKey::Activate => activate_focused(focus, draft),
    }
}

fn activate_focused(focus: &ManufactureFocus, draft: &mut Draft) -> Result<&'static str, &'static str> {
    match focus.action_id() {
        "add-definition-n3" => draft.source_ok = true,
        "compile-graph" => {
            if !draft.source_ok {
                return Err(next_action(draft));
            }
            draft.graph_ok = true;
            draft.round_trip_loss = false;
        }
        "build-executable" => {
            if !draft.graph_ok {
                return Err(next_action(draft));
            }
            draft.executable_ok = true;
        }
        "label-category" => draft.labelled = true,
        "show-round-trip-loss" => {
            draft.round_trip_loss = draft.source_ok && !draft.graph_ok;
        }
        "validate" | "sign" => {}
        "publish" => {
            can_publish(draft)?;
            return Ok(NEXT_PUBLISH);
        }
        other if other.contains("Host.") => return Err("unknown entry point (not a Host ID)"),
        _ => return Err(next_action(draft)),
    }
    Ok(next_action(draft))
}

/// Full keyboard-only author path: Tab through steps, Enter to complete each.
pub fn keyboard_only_author_to_publish(draft: &mut Draft) -> Result<(), &'static str> {
    let mut focus = ManufactureFocus::start();
    // Ensure every action name is a real a11y handle before the path runs.
    for (_, name) in ACTIONS {
        if !action_name_ok(name) {
            return Err("held / not yet — manufacture action missing accessible name");
        }
    }
    let order = [
        "add-definition-n3",
        "compile-graph",
        "build-executable",
        "label-category",
        "validate",
        "sign",
        "publish",
    ];
    for want in order {
        while focus.action_id() != want {
            apply_key(&mut focus, ManufactureKey::Next, draft)?;
        }
        apply_key(&mut focus, ManufactureKey::Activate, draft)?;
    }
    can_publish(draft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_tab_enter_arrows() {
        assert_eq!(classify("Tab", false), ManufactureKey::Next);
        assert_eq!(classify("Enter", false), ManufactureKey::Activate);
        assert_eq!(classify("ArrowLeft", false), ManufactureKey::Prev);
        assert_eq!(classify("z", true), ManufactureKey::Ignore);
    }

    #[test]
    fn publish_enter_blocked_until_ready() {
        let mut focus = ManufactureFocus {
            index: ACTIONS
                .iter()
                .position(|(id, _)| *id == "publish")
                .expect("publish"),
        };
        let mut draft = Draft::default();
        assert!(apply_key(&mut focus, ManufactureKey::Activate, &mut draft).is_err());
        draft.source_ok = true;
        draft.graph_ok = true;
        draft.executable_ok = true;
        draft.labelled = true;
        assert_eq!(
            apply_key(&mut focus, ManufactureKey::Activate, &mut draft),
            Ok(NEXT_PUBLISH)
        );
    }

    #[test]
    fn keyboard_only_path_reaches_publish() {
        let mut draft = Draft::default();
        assert!(keyboard_only_author_to_publish(&mut draft).is_ok());
        assert!(can_publish(&draft).is_ok());
        assert!(!draft.round_trip_loss);
    }

    #[test]
    fn host_key_token_never_in_actions() {
        for (id, name) in ACTIONS {
            assert!(!id.contains("Host."));
            assert!(!name.contains("Host."));
            assert!(action_name_ok(name));
        }
    }
}
