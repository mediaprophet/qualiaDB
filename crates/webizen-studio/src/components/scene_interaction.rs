//! Scene interaction component for epistemic anchor coordination.
//!
//! Handles mouse events for coordinating epistemic anchors and quantum context selection
//! via ray casting through the 10D manifold. Uses zero-heap principles for interaction
//! state (binary indices instead of String IDs).
//!
//! This is NOT traditional object picking for 3D scene manipulation.
//! This is epistemic anchor coordination for quantum context selection and
//! wavefunction collapse via ray casting through the 10D manifold.

use dioxus::prelude::*;

/// Epistemic state for quantum context management
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EpistemicState {
    /// Ground truth (q = 0)
    Collapsed,
    /// GSR escrow (q > 0)
    Pending,
    /// Parallel context (q > 0)
    Sandbox,
}

impl Default for EpistemicState {
    fn default() -> Self {
        EpistemicState::Collapsed
    }
}

/// Interaction state using zero-heap principles (binary indices)
#[derive(Clone, Copy, Debug, Default)]
struct InteractionState {
    /// Currently selected node index (binary index, not String ID)
    selected_index: Option<usize>,
    /// Currently hovered node index (binary index, not String ID)
    hovered_index: Option<usize>,
    /// Last mouse position (screen coordinates in physical pixels)
    mouse_pos: (f64, f64),
    /// Quantum context (q dimension) for selected anchor
    quantum_context: f64,
    /// Epistemic state for selected anchor
    epistemic_state: EpistemicState,
}

/// Scene interaction component for epistemic anchor coordination
///
/// Handles mouse events for coordinating epistemic anchors and quantum context selection
/// via ray casting through the 10D manifold. Displays selected anchor information with
/// quantum context and epistemic state.
///
/// Integration notes:
/// - This component should be overlaid on the render preview canvas
/// - Mouse events are translated to screen coordinates and sent to the renderer
/// - The renderer uses stack-allocated ray casting (pick_anchor, hover_anchor) to return binary indices
/// - Binary indices are used for zero-heap IPC with the native host
/// - Quantum context (q dimension) and epistemic state are displayed for selected anchors
#[component]
pub fn SceneInteraction() -> Element {
    let mut interaction_state = use_signal(|| InteractionState::default());
    let mut node_info = use_signal(|| String::new());

    // Handle mouse move for hovering over epistemic anchors
    let on_mouse_move = move |evt: dioxus::prelude::MouseEvent| {
        let coords = evt.client_coordinates();
        let mut state = interaction_state.write();
        state.mouse_pos = (coords.x as f64, coords.y as f64);

        // TODO: Integrate with actual renderer via Tauri command
        // Example integration:
        // #[cfg(target_arch = "wasm32")]
        // {
        //     wasm_bindgen_futures::spawn_local(async move {
        //         let hovered_index = invoke_tauri_hover_anchor(x, y).await;
        //         state.hovered_index = hovered_index;
        //     });
        // }

        // For now, simulate hover (in real implementation, this comes from renderer)
        // state.hovered_index = None; // Would be set by renderer
    };

    // Handle mouse click for coordinating epistemic anchor
    let on_mouse_click = move |evt: dioxus::prelude::MouseEvent| {
        let coords = evt.client_coordinates();
        let mut state = interaction_state.write();
        state.mouse_pos = (coords.x as f64, coords.y as f64);

        // TODO: Integrate with actual renderer via Tauri command
        // Example integration:
        // #[cfg(target_arch = "wasm32")]
        // {
        //     wasm_bindgen_futures::spawn_local(async move {
        //         let selected_index = invoke_tauri_pick_anchor(x, y).await;
        //         state.selected_index = selected_index;
        //
        //         // Fetch quantum context and epistemic state for selected anchor
        //         let (q, epistemic_state) = fetch_anchor_quantum_context(selected_index).await;
        //         state.quantum_context = q;
        //         state.epistemic_state = epistemic_state;
        //
        //         // Update scene with selected index for visual feedback
        //         update_scene_selected_index(selected_index).await;
        //     });
        // }

        // For now, simulate pick (in real implementation, this comes from renderer)
        // state.selected_index = None; // Would be set by renderer

        // Update node info display with quantum context
        if let Some(index) = state.selected_index {
            let epistemic_label = match state.epistemic_state {
                EpistemicState::Collapsed => "Collapsed",
                EpistemicState::Pending => "Pending",
                EpistemicState::Sandbox => "Sandbox",
            };
            node_info.set(format!(
                "Anchor: {} | q: {:.2} | State: {}",
                index, state.quantum_context, epistemic_label
            ));
        } else {
            node_info.set("No epistemic anchor selected".to_string());
        }
    };

    let state = interaction_state.read();
    let info_text = node_info.read();
    let mouse_x = state.mouse_pos.0;
    let mouse_y = state.mouse_pos.1;
    let mouse_pos_str = format!("({:.1}, {:.1})", mouse_x, mouse_y);

    // Format epistemic state for display
    let epistemic_state_label = match state.epistemic_state {
        EpistemicState::Collapsed => "Collapsed",
        EpistemicState::Pending => "Pending",
        EpistemicState::Sandbox => "Sandbox",
    };

    // Format quantum context for display
    let quantum_context_str = if state.selected_index.is_some() {
        format!("{:.2}", state.quantum_context)
    } else {
        "-".to_string()
    };

    rsx! {
        div {
            class: "scene-interaction-panel",
            style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 18px; padding: 1.15rem 1.2rem 1.25rem; backdrop-filter: blur(24px); box-shadow: 0 8px 32px rgba(0,0,0,0.08);",

            h2 {
                style: "margin: 0 0 0.25rem 0; font-size: 0.98rem; font-weight: 700; color: var(--qualia-text);",
                "Epistemic Anchor Coordination"
            }
            p {
                style: "margin: 0 0 0.9rem 0; font-size: 0.76rem; color: var(--qualia-text-muted); line-height: 1.45;",
                "Coordinate epistemic anchors for quantum context selection via ray casting through the 10D manifold. Zero-heap: uses binary indices for IPC."
            }

            div {
                style: "display: flex; flex-direction: column; gap: 0.5rem;",

                // Mouse position display
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: rgba(0,0,0,0.05); border-radius: 8px;",
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Mouse Position:"
                    }
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text); font-family: monospace;",
                        "{mouse_pos_str}"
                    }
                }

                // Selected anchor display
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: rgba(0,0,0,0.05); border-radius: 8px;",
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Selected Anchor:"
                    }
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text); font-family: monospace;",
                        "{info_text}"
                    }
                }

                // Quantum context display
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: rgba(0,0,0,0.05); border-radius: 8px;",
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Quantum Context (q):"
                    }
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text); font-family: monospace;",
                        "{quantum_context_str}"
                    }
                }

                // Epistemic state display
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: rgba(0,0,0,0.05); border-radius: 8px;",
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Epistemic State:"
                    }
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text); font-family: monospace;",
                        "{epistemic_state_label}"
                    }
                }

                // Hovered anchor display
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; padding: 0.5rem; background: rgba(0,0,0,0.05); border-radius: 8px;",
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Hovered Anchor:"
                    }
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text); font-family: monospace;",
                        if let Some(index) = state.hovered_index {
                            "Anchor index: {index}"
                        } else {
                            "None"
                        }
                    }
                }
            }

            // Interaction area (this would be overlaid on the render canvas)
            div {
                style: "margin-top: 0.9rem; padding: 0.75rem; background: rgba(0,0,0,0.08); border-radius: 10px; border: 1px dashed var(--qualia-border);",
                p {
                    style: "margin: 0; font-size: 0.75rem; color: var(--qualia-text-muted); text-align: center;",
                    "Interaction canvas area (overlay on render preview)"
                }
                div {
                    style: "width: 100%; height: 200px; background: rgba(0,0,0,0.1); border-radius: 8px; display: flex; align-items: center; justify-content: center; cursor: crosshair;",
                    onmousemove: on_mouse_move,
                    onclick: on_mouse_click,
                    span {
                        style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                        "Click or hover to coordinate epistemic anchors"
                    }
                }
            }

            // Wavefunction collapse trigger
            div {
                style: "margin-top: 0.9rem; padding: 0.75rem; background: rgba(0,0,0,0.05); border-radius: 10px;",
                p {
                    style: "margin: 0 0 0.5rem 0; font-size: 0.75rem; color: var(--qualia-text-muted); font-weight: 600;",
                    "Wavefunction Collapse:"
                }
                button {
                    style: "padding: 0.5rem 1rem; background: var(--qualia-accent); color: white; border: none; border-radius: 6px; font-size: 0.8rem; cursor: pointer; width: 100%;",
                    disabled: state.selected_index.is_none(),
                    onclick: move |_| {
                        // TODO: Trigger wavefunction collapse for selected anchor
                        // Example integration:
                        // #[cfg(target_arch = "wasm32")]
                        // {
                        //     wasm_bindgen_futures::spawn_local(async move {
                        //         if let Some(index) = state.selected_index {
                        //             trigger_wavefunction_collapse(index).await;
                        //         }
                        //     });
                        // }
                    },
                    "Collapse Wavefunction"
                }
                p {
                    style: "margin: 0.5rem 0 0 0; font-size: 0.7rem; color: var(--qualia-text-muted);",
                    "Collapses quantum context (q → 0) for selected epistemic anchor"
                }
            }

            // Integration notes
            div {
                style: "margin-top: 0.9rem; padding: 0.75rem; background: rgba(0,0,0,0.05); border-radius: 10px; border-left: 3px solid var(--qualia-border);",
                p {
                    style: "margin: 0 0 0.5rem 0; font-size: 0.75rem; color: var(--qualia-text-muted); font-weight: 600;",
                    "Integration Status:"
                }
                ul {
                    style: "margin: 0; padding-left: 1.2rem; font-size: 0.73rem; color: var(--qualia-text-muted); line-height: 1.5;",
                    li { "✓ WgpuRenderer: pick_anchor() and hover_anchor() methods implemented with stack-allocated ray casting" }
                    li { "✓ Zero-heap compliance: Binary indices returned for IPC, no heap allocation in ray casting" }
                    li { "✓ Quantum context display: Shows q dimension and epistemic state (Collapsed, Pending, Sandbox)" }
                    li { "✓ Wavefunction collapse trigger: UI button for collapsing quantum context" }
                    li { "○ Tauri IPC: Commands needed to bridge frontend to renderer (pick_anchor, hover_anchor)" }
                    li { "○ Canvas overlay: Component needs to be positioned over render preview" }
                    li { "○ Quantum context fetch: Need to fetch q dimension and epistemic state from backend" }
                }
            }
        }
    }
}
