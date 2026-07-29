//! Route-compatible entry point for the 0.0.28 configuration system.

use dioxus::prelude::*;

#[component]
pub fn SettingsPage() -> Element {
    rsx! { crate::components::settings::SettingsShell {} }
}
