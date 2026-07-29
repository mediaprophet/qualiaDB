//! Shared Naturalised / Advanced Technical presentation mode.
//!
//! The mode changes vocabulary and control density, never capability or access.
//! It is stored locally because it is a presentation preference, not identity.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "webizen_experience_mode";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExperienceMode {
    #[default]
    Naturalised,
    AdvancedTechnical,
}

impl ExperienceMode {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::Naturalised => "naturalised",
            Self::AdvancedTechnical => "advanced-technical",
        }
    }

    pub const fn is_advanced(self) -> bool {
        matches!(self, Self::AdvancedTechnical)
    }
}

pub fn initial_experience_mode() -> ExperienceMode {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(value)) = storage.get_item(STORAGE_KEY) {
                    if value == ExperienceMode::AdvancedTechnical.storage_value() {
                        return ExperienceMode::AdvancedTechnical;
                    }
                }
            }
        }
    }
    ExperienceMode::Naturalised
}

pub fn persist_experience_mode(mode: ExperienceMode) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(STORAGE_KEY, mode.storage_value());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = mode;
}

pub fn use_experience_mode() -> Signal<ExperienceMode> {
    consume_context::<Signal<ExperienceMode>>()
}

#[component]
pub fn ExperienceModeSwitch() -> Element {
    let mut mode = use_experience_mode();
    let current = mode();
    let natural_style = mode_button_style(current == ExperienceMode::Naturalised);
    let advanced_style = mode_button_style(current == ExperienceMode::AdvancedTechnical);

    rsx! {
        div {
            role: "group",
            aria_label: "Interface presentation mode",
            style: "display:inline-flex;align-items:center;gap:3px;padding:3px;border:1px solid var(--qualia-border);border-radius:11px;background:color-mix(in srgb,var(--qualia-bg) 72%,transparent);",
            button {
                r#type: "button",
                aria_pressed: current == ExperienceMode::Naturalised,
                style: "{natural_style}",
                onclick: move |_| {
                    mode.set(ExperienceMode::Naturalised);
                    persist_experience_mode(ExperienceMode::Naturalised);
                },
                "Naturalised"
            }
            button {
                r#type: "button",
                aria_pressed: current == ExperienceMode::AdvancedTechnical,
                style: "{advanced_style}",
                onclick: move |_| {
                    mode.set(ExperienceMode::AdvancedTechnical);
                    persist_experience_mode(ExperienceMode::AdvancedTechnical);
                },
                "Advanced Technical"
            }
        }
    }
}

fn mode_button_style(active: bool) -> &'static str {
    if active {
        "border:1px solid color-mix(in srgb,var(--qualia-accent) 52%,transparent);border-radius:8px;padding:7px 11px;background:var(--qualia-accent-glow);color:var(--qualia-accent);font:inherit;font-size:.72rem;font-weight:750;cursor:pointer;"
    } else {
        "border:1px solid transparent;border-radius:8px;padding:7px 11px;background:transparent;color:var(--qualia-text-muted);font:inherit;font-size:.72rem;font-weight:650;cursor:pointer;"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_have_stable_storage_values() {
        assert_eq!(ExperienceMode::Naturalised.storage_value(), "naturalised");
        assert_eq!(
            ExperienceMode::AdvancedTechnical.storage_value(),
            "advanced-technical"
        );
    }

    #[test]
    fn only_technical_mode_is_advanced() {
        assert!(!ExperienceMode::Naturalised.is_advanced());
        assert!(ExperienceMode::AdvancedTechnical.is_advanced());
    }
}
