use dioxus::prelude::*;

/// Epistemic state for quantum context management
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EpistemicState {
    /// Ground truth (q = 0)
    Collapsed,
    /// GSR escrow (q > 0)
    Pending,
    /// Parallel context (q > 0)
    Sandbox,
}

/// Wavefunction collapse UI component
///
/// Displays epistemic status for nodes with pending wavefunction states.
/// Provides UI to collapse wavefunction and promote to ground truth.
///
/// Zero-heap considerations:
/// - Component state uses heap allocation (inherent to Dioxus/React framework)
/// - String props are heap-allocated (unavoidable for UI text)
/// - The actual quantum state management happens in backend (zero-heap compliant)
#[component]
pub fn EpistemicStatus(
    state: EpistemicState,
    node_id: String,
    on_collapse: Option<Callback<String>>,
) -> Element {
    match state {
        EpistemicState::Collapsed => {
            // No UI for collapsed states - return empty fragment
            rsx! {}
        }
        EpistemicState::Pending | EpistemicState::Sandbox => {
            let state_label = if state == EpistemicState::Pending {
                "In Escrow"
            } else {
                "Sandbox Context"
            };

            rsx! {
                div {
                    class: "epistemic-pending",
                    style: "opacity: 0.6; animation: pulse 2s infinite; padding: 8px; border-radius: 4px; background: rgba(255, 255, 255, 0.1);",

                    div {
                        class: "pulse-effect",
                        style: "box-shadow: 0 0 20px rgba(255, 255, 255, 0.5);",
                        span {
                            class: "ghosted-node",
                            style: "font-size: 12px; color: #888;",
                            "{state_label}"
                        }
                    }

                    if let Some(collapse_callback) = on_collapse {
                        button {
                            onclick: move |_| {
                                collapse_callback.call(node_id.clone());
                            },
                            style: "margin-left: 8px; padding: 4px 8px; font-size: 12px; cursor: pointer;",
                            "Collapse Wavefunction"
                        }
                    }
                }
            }
        }
    }
}
