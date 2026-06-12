#![allow(non_snake_case)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::shoelace::*;

/// Exact field structure for the ModuleAttachmentArgs payload.
/// This will be serialized into binary (bincode) and sent as the `arguments_raw`
/// field within an `McpPayload` to the `validate_hardware_assembly` MCP tool.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModuleAttachmentArgs {
    pub baseplate_id: String,
    pub module_id: String,
    pub grid_x: u8,
    pub grid_y: u8,
    pub width: u8,
    pub height: u8,
    pub expected_power_draw_w: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigStatus {
    Idle,
    Validating,
    Approved,
    Violated(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HardwareConfigState {
    pub baseplate_id: String,
    pub pending_module: Option<ModuleAttachmentArgs>,
    pub attached_modules: Vec<ModuleAttachmentArgs>,
    pub status: ConfigStatus,
}

#[component]
pub fn HardwareConfigurator() -> Element {
    let mut state = use_signal(|| HardwareConfigState {
        baseplate_id: "baseplate_v1".to_string(),
        pending_module: None,
        attached_modules: Vec::new(),
        status: ConfigStatus::Idle,
    });

    // 2. Token Vaporization (Runs once on component mount to strip session tokens from URL)
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            let mut clean_url = location.pathname().unwrap_or_else(|_| "/".to_string());
            if let Ok(hash) = location.hash() { 
                clean_url.push_str(&hash); 
            }
            // Scrub qualia_token from the URL history
            let _ = window.history()
                .unwrap()
                .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&clean_url));
        }
    });

    let mut handle_snap_action = move |module: ModuleAttachmentArgs| {
        // 1. Enter "Validating" / Ghosting state
        let mut current = state.read().clone();
        current.pending_module = Some(module.clone());
        current.status = ConfigStatus::Validating;
        state.set(current);

        spawn(async move {
            let mut current = state.read().clone();
            // Simulate validation logic (e.g. power draw > 100W fails)
            if module.expected_power_draw_w > 100.0 {
                current.status = ConfigStatus::Violated("Exceeds maximum power draw for this routing lane.".to_string());
                current.pending_module = None;
            } else {
                current.status = ConfigStatus::Approved;
                current.attached_modules.push(module);
                current.pending_module = None;
            }
            state.set(current);
        });
    };

    rsx! {
        div { class: "hardware-config-pane flex flex-col h-full",
            SlCard {
                div { slot: "header", class: "flex justify-between items-center",
                    span { "Hardware Assembly Configurator" }
                    match state.read().status {
                        ConfigStatus::Idle => rsx! { SlBadge { variant: "neutral", "Idle" } },
                        ConfigStatus::Validating => rsx! { SlBadge { variant: "primary", SlSpinner {} " Validating" } },
                        ConfigStatus::Approved => rsx! { SlBadge { variant: "success", "Approved" } },
                        ConfigStatus::Violated(_) => rsx! { SlBadge { variant: "danger", "Violation Detected" } },
                    }
                }
                
                div { class: "flex-col flex gap-4",
                    p { class: "text-sm text-gray-400",
                        "Snap components onto the baseplate. The local daemon will validate structural and electrical compliance via Sanctuary Gates."
                    }
                    
                    // Render any violation messages from the daemon
                    if let ConfigStatus::Violated(err_msg) = &state.read().status {
                        SlAlert { variant: "danger", open: true,
                            span { "slot": "icon", SlIcon { name: "exclamation-triangle" } }
                            strong { "Sanctuary Gate Blocked Assembly: " }
                            "{err_msg}"
                        }
                    }

                    // Main Interactive SVG Grid Baseplate
                    div { class: "relative w-full h-64 bg-gray-900 border-2 border-gray-700 rounded overflow-hidden",
                        svg {
                            width: "100%", height: "100%",
                            
                            // Grid pattern
                            defs {
                                pattern { id: "grid", width: "40", height: "40", pattern_units: "userSpaceOnUse",
                                    path { d: "M 40 0 L 0 0 0 40", fill: "none", stroke: "#333", "stroke-width": "1" }
                                }
                            }
                            rect { width: "100%", height: "100%", fill: "url(#grid)" }
                            
                            // Render previously attached modules
                            for (i, m) in state.read().attached_modules.iter().enumerate() {
                                rect {
                                    key: "{i}",
                                    x: "{m.grid_x as u32 * 40}",
                                    y: "{m.grid_y as u32 * 40}",
                                    width: "{m.width as u32 * 40}",
                                    height: "{m.height as u32 * 40}",
                                    fill: "#2f9e44", // Approved color
                                    stroke: "#2b8a3e",
                                    "stroke-width": "2",
                                }
                            }

                            // Render pending/ghost module
                            if let Some(m) = &state.read().pending_module {
                                rect {
                                    x: "{m.grid_x as u32 * 40}",
                                    y: "{m.grid_y as u32 * 40}",
                                    width: "{m.width as u32 * 40}",
                                    height: "{m.height as u32 * 40}",
                                    fill: "#e67700", // Validating color
                                    stroke: "#f08c00",
                                    "stroke-width": "2",
                                    "stroke-dasharray": "4",
                                }
                            }
                        }
                    }

                    // Simulated "Toolbar" to trigger snaps
                    div { class: "flex gap-2",
                        div {
                            onclick: move |_| handle_snap_action(ModuleAttachmentArgs {
                                baseplate_id: state.read().baseplate_id.clone(),
                                module_id: "wifi_sensor_module".to_string(),
                                grid_x: 2, grid_y: 2, width: 2, height: 1,
                                expected_power_draw_w: 15.0, // Safe
                            }),
                            SlButton { 
                                variant: "default",
                                "Snap Wi-Fi Module (Safe)" 
                            }
                        }
                        div {
                            onclick: move |_| handle_snap_action(ModuleAttachmentArgs {
                                baseplate_id: state.read().baseplate_id.clone(),
                                module_id: "heavy_heating_element".to_string(),
                                grid_x: 4, grid_y: 2, width: 3, height: 2,
                                expected_power_draw_w: 150.0, // Should trigger violation
                            }),
                            SlButton { 
                                variant: "danger", outline: true,
                                "Snap Heater (Will Fail)" 
                            }
                        }
                    }
                }
            }
        }
    }
}
