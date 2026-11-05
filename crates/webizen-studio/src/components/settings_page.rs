//! Route-compatible entry point for the 0.0.28 configuration system.

use aether::prelude::*;

#[component]
/// Aether `SettingsPage` component.
pub fn SettingsPage() -> Element {
    rsx! { crate::components::settings::SettingsShell {} }
}
