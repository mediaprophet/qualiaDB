//! Studio run panel (SI-10): progress, cancel, permission denial.
//! Shape-derived input uses si:inputShape. No Host IDs.

use dioxus::prelude::*;

pub const PERMISSION_DENIED: &str =
    "held / not yet — permission denial is usable; grant was refused";
pub const INPUT_SHAPE: &str = "si:inputShape";
const ERR_HOST: &str = "unknown entry point (not a Host ID)";
const HELD_INPUT: &str = "held / not yet — incomplete required input";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Idle,
    Running,
    Completed,
    Cancelled,
    Held,
    Denied,
}

impl RunPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Held => "held",
            Self::Denied => "denied",
        }
    }
}

fn entry_ok(entry: &str) -> bool {
    !entry.contains("Host.") && matches!(entry.trim(), "assess" | "recognise")
}

/// Pure run step. Host/bad entry and empty input are held; !permitted is Denied.
pub fn run_step(phase: RunPhase, entry: &str, input: &str, permitted: bool) -> RunPhase {
    if entry.contains("Host.") || !entry_ok(entry) {
        return RunPhase::Held;
    }
    if !permitted {
        return RunPhase::Denied;
    }
    if input.trim().is_empty() {
        return RunPhase::Held;
    }
    match phase {
        RunPhase::Idle => RunPhase::Running,
        RunPhase::Running => RunPhase::Completed,
        other => other,
    }
}

/// Cancel holds a new cancel except after Completed.
pub fn cancel(phase: RunPhase) -> RunPhase {
    if matches!(phase, RunPhase::Completed) {
        phase
    } else {
        RunPhase::Cancelled
    }
}

fn status_copy(phase: RunPhase, entry: &str) -> &'static str {
    if entry.contains("Host.") {
        return ERR_HOST;
    }
    match phase {
        RunPhase::Denied => PERMISSION_DENIED,
        RunPhase::Held if !entry_ok(entry) => ERR_HOST,
        RunPhase::Held => HELD_INPUT,
        other => other.as_str(),
    }
}

#[component]
pub fn RunPanel(entry: String) -> Element {
    let mut phase = use_signal(|| RunPhase::Idle);
    let mut input = use_signal(|| String::new());
    let mut permitted = use_signal(|| true);
    let status = status_copy(phase(), &entry);

    rsx! {
        div {
            class: "instrument-run-panel",
            "data-run-panel": "1",
            role: "region",
            "aria-label": "instrument run",

            div { class: "lexicon-bay-title", "Run" }
            p { class: "lexicon-bay-lede", "input shape {INPUT_SHAPE}" }

            label {
                "aria-label": "fixture input",
                input {
                    r#type: "text",
                    "data-run-input": "1",
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                }
            }

            button {
                r#type: "button",
                class: "lexicon-chip",
                "aria-label": if permitted() { "grant" } else { "deny" },
                "data-run-permit": "{permitted()}",
                onclick: move |_| permitted.set(!permitted()),
                if permitted() { "grant" } else { "deny" }
            }

            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Run",
                "data-run-go": "1",
                onclick: move |_| {
                    let next = run_step(phase(), &entry, &input(), permitted());
                    phase.set(next);
                },
                "Run"
            }
            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Cancel run",
                "data-run-cancel": "1",
                onclick: move |_| phase.set(cancel(phase())),
                "Cancel"
            }

            div {
                class: "lexicon-held-gate",
                role: "status",
                "data-run-phase": "{phase().as_str()}",
                "{status}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_and_empty_input_are_held() {
        assert_eq!(
            run_step(RunPhase::Idle, "Host.assess", "ok", true),
            RunPhase::Held
        );
        assert_eq!(
            run_step(RunPhase::Idle, "assess", "", true),
            RunPhase::Held
        );
        assert_eq!(
            run_step(RunPhase::Idle, "assess", "   ", true),
            RunPhase::Held
        );
    }

    #[test]
    fn deny_is_denied() {
        assert_eq!(
            run_step(RunPhase::Idle, "assess", "ok", false),
            RunPhase::Denied
        );
        assert_eq!(PERMISSION_DENIED.contains("permission denial"), true);
    }

    #[test]
    fn cancel_from_running() {
        assert_eq!(cancel(RunPhase::Running), RunPhase::Cancelled);
        assert_eq!(cancel(RunPhase::Idle), RunPhase::Cancelled);
        assert_eq!(cancel(RunPhase::Completed), RunPhase::Completed);
    }

    #[test]
    fn assess_ok_permitted_completes() {
        let running = run_step(RunPhase::Idle, "assess", "ok", true);
        assert_eq!(running, RunPhase::Running);
        assert_eq!(
            run_step(RunPhase::Running, "assess", "ok", true),
            RunPhase::Completed
        );
        assert_eq!(INPUT_SHAPE, "si:inputShape");
        assert!(!INPUT_SHAPE.contains("owl:Thing"));
        assert!(!INPUT_SHAPE.contains("Host."));
        assert!(!ERR_HOST.contains("Host."));
    }
}
