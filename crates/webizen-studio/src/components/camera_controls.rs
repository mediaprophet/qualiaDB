//! Manifold projection viewpoint adjustment UI component.
//!
//! Provides buttons for adjusting the projection viewpoint through the 10D epistemic manifold.
//! This is NOT traditional camera navigation - it's geometric examination of epistemic relationships.
//! All UI state uses stack-allocated primitives (Copy types) for zero-heap compliance.

use dioxus::prelude::*;

/// Manifold projection control state - zero-heap compliant (all Copy types)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraControlState {
    /// Projection viewpoint rotation speed multiplier
    pub rotation_speed: f64,
    /// Projection viewpoint zoom speed multiplier
    pub zoom_speed: f64,
    /// Projection viewpoint pan speed multiplier
    pub pan_speed: f64,
}

impl CameraControlState {
    pub fn new() -> Self {
        Self {
            rotation_speed: 0.1,
            zoom_speed: 1.0,
            pan_speed: 10.0,
        }
    }
}

/// Manifold projection control component with buttons for viewpoint adjustment.
/// These controls adjust the projection viewpoint for geometric examination of epistemic relationships
/// in the 10D manifold, not traditional 3D scene navigation.
///
/// # Zero-Heap Compliance
/// - All state uses Copy types (f64 primitives)
/// - No heap allocation in UI state management
/// - Callbacks use stack-allocated parameters
#[component]
pub fn CameraControls(
    /// Callback for orbit operation (yaw, pitch in radians)
    on_orbit: EventHandler<(f64, f64)>,
    /// Callback for zoom operation (delta zoom amount)
    on_zoom: EventHandler<f64>,
    /// Callback for pan operation (dx, dy screen deltas)
    on_pan: EventHandler<(f64, f64)>,
    /// Optional initial control state
    #[props(default)]
    initial_state: CameraControlState,
) -> Element {
    let state = use_signal(|| initial_state);

    rsx! {
        div {
            class: "camera-controls",
            style: "display: flex; flex-direction: column; gap: 8px; padding: 12px; background: rgba(11, 14, 20, 0.9); border-radius: 8px; border: 1px solid rgba(103, 232, 249, 0.2);",

            // Projection viewpoint rotation controls
            div {
                class: "control-group",
                style: "display: flex; flex-direction: column; gap: 4px;",
                div {
                    class: "control-label",
                    style: "font-size: 12px; color: #67e8f9; font-weight: 600; margin-bottom: 4px;",
                    "Projection Rotation"
                }
                div {
                    class: "button-row",
                    style: "display: flex; gap: 4px;",
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let yaw = -state.read().rotation_speed;
                            let pitch = 0.0;
                            on_orbit.call((yaw, pitch));
                        },
                        "← Left"
                    }
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let yaw = state.read().rotation_speed;
                            let pitch = 0.0;
                            on_orbit.call((yaw, pitch));
                        },
                        "Right →"
                    }
                }
                div {
                    class: "button-row",
                    style: "display: flex; gap: 4px;",
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let yaw = 0.0;
                            let pitch = state.read().rotation_speed;
                            on_orbit.call((yaw, pitch));
                        },
                        "↑ Up"
                    }
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let yaw = 0.0;
                            let pitch = -state.read().rotation_speed;
                            on_orbit.call((yaw, pitch));
                        },
                        "Down ↓"
                    }
                }
            }

            // Zoom controls
            div {
                class: "control-group",
                style: "display: flex; flex-direction: column; gap: 4px; margin-top: 8px;",
                div {
                    class: "control-label",
                    style: "font-size: 12px; color: #67e8f9; font-weight: 600; margin-bottom: 4px;",
                    "Zoom"
                }
                div {
                    class: "button-row",
                    style: "display: flex; gap: 4px;",
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let delta = state.read().zoom_speed;
                            on_zoom.call(delta);
                        },
                        "+ In"
                    }
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let delta = -state.read().zoom_speed;
                            on_zoom.call(delta);
                        },
                        "− Out"
                    }
                }
            }

            // Pan controls
            div {
                class: "control-group",
                style: "display: flex; flex-direction: column; gap: 4px; margin-top: 8px;",
                div {
                    class: "control-label",
                    style: "font-size: 12px; color: #67e8f9; font-weight: 600; margin-bottom: 4px;",
                    "Pan"
                }
                div {
                    class: "button-row",
                    style: "display: flex; gap: 4px;",
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let dx = -state.read().pan_speed;
                            let dy = 0.0;
                            on_pan.call((dx, dy));
                        },
                        "←"
                    }
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let dx = state.read().pan_speed;
                            let dy = 0.0;
                            on_pan.call((dx, dy));
                        },
                        "→"
                    }
                }
                div {
                    class: "button-row",
                    style: "display: flex; gap: 4px;",
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let dx = 0.0;
                            let dy = -state.read().pan_speed;
                            on_pan.call((dx, dy));
                        },
                        "↑"
                    }
                    button {
                        class: "control-btn",
                        style: "flex: 1; padding: 8px; background: rgba(103, 232, 249, 0.1); border: 1px solid rgba(103, 232, 249, 0.3); border-radius: 4px; color: #67e8f9; cursor: pointer; font-size: 14px; transition: all 0.2s;",
                        onmousedown: move |_| {
                            let dx = 0.0;
                            let dy = state.read().pan_speed;
                            on_pan.call((dx, dy));
                        },
                        "↓"
                    }
                }
            }

            // Reset button
            div {
                class: "control-group",
                style: "margin-top: 8px;",
                button {
                    class: "control-btn reset-btn",
                    style: "width: 100%; padding: 8px; background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 4px; color: #ef4444; cursor: pointer; font-size: 14px; font-weight: 600; transition: all 0.2s;",
                    onmousedown: move |_| {
                        // Reset to default camera position
                        on_orbit.call((0.0, 0.0));
                        on_zoom.call(0.0);
                        on_pan.call((0.0, 0.0));
                    },
                    "Reset Camera"
                }
            }
        }
    }
}
