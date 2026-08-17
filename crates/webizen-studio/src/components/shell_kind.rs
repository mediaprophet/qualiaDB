//! Classic (default) vs Poet chrome. Presentation only — Classic routes stay.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "webizen_shell_kind";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShellKind {
    #[default]
    Classic,
    Poet,
}

impl ShellKind {
    pub const fn storage_value(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Poet => "poet",
        }
    }

    pub const fn is_poet(self) -> bool {
        matches!(self, Self::Poet)
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "classic" => Some(Self::Classic),
            "poet" => Some(Self::Poet),
            _ => None,
        }
    }
}

pub fn initial_shell_kind() -> ShellKind {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(value)) = storage.get_item(STORAGE_KEY) {
                    if let Some(kind) = ShellKind::from_storage(&value) {
                        return kind;
                    }
                }
            }
        }
    }
    ShellKind::Classic
}

pub fn persist_shell_kind(kind: ShellKind) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(STORAGE_KEY, kind.storage_value());
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = kind;
}

pub fn use_shell_kind() -> Signal<ShellKind> {
    consume_context::<Signal<ShellKind>>()
}

#[component]
pub fn ShellKindSwitch() -> Element {
    let mut kind = use_shell_kind();
    let current = kind();
    let classic_style = mode_button_style(current == ShellKind::Classic);
    let poet_style = mode_button_style(current == ShellKind::Poet);
    rsx! {
        div {
            role: "group",
            aria_label: "Shell chrome",
            title: "Classic stays default. Poet is opt-in chrome; no Classic route is removed.",
            style: "display:inline-flex;align-items:center;gap:3px;padding:3px;border:1px solid var(--qualia-border);border-radius:11px;background:color-mix(in srgb,var(--qualia-bg) 72%,transparent);",
            button {
                r#type: "button",
                aria_pressed: current == ShellKind::Classic,
                style: "{classic_style}",
                onclick: move |_| {
                    kind.set(ShellKind::Classic);
                    persist_shell_kind(ShellKind::Classic);
                },
                "Classic"
            }
            button {
                r#type: "button",
                aria_pressed: current == ShellKind::Poet,
                style: "{poet_style}",
                onclick: move |_| {
                    kind.set(ShellKind::Poet);
                    persist_shell_kind(ShellKind::Poet);
                },
                "Poet"
            }
        }
    }
}

fn mode_button_style(active: bool) -> &'static str {
    if active {
        "border:1px solid color-mix(in srgb,#00d2ff 52%,transparent);border-radius:8px;padding:7px 11px;background:rgba(0,210,255,0.12);color:#00d2ff;font:inherit;font-size:.72rem;font-weight:750;cursor:pointer;"
    } else {
        "border:1px solid transparent;border-radius:8px;padding:7px 11px;background:transparent;color:var(--qualia-text-muted);font:inherit;font-size:.72rem;font-weight:650;cursor:pointer;"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_classic() {
        assert_eq!(ShellKind::default(), ShellKind::Classic);
        assert!(!ShellKind::Classic.is_poet());
        assert_eq!(ShellKind::from_storage("classic"), Some(ShellKind::Classic));
        assert_eq!(ShellKind::from_storage("poet"), Some(ShellKind::Poet));
    }
}
