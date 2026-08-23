//! HyperCanvas tokens — copied from `C:\Projects\NLP\Canvas_Workbench\styles`.

use dioxus::prelude::*;

const THEME: &str = include_str!("css/theme.css");
const LAYOUT: &str = include_str!("css/layout.css");
const CANVAS: &str = include_str!("css/canvas.css");
const TOOLBOXES: &str = include_str!("css/toolboxes.css");
const ICONS: &str = include_str!("css/icons.css");

#[component]
pub fn HyperCanvasStyles() -> Element {
    rsx! {
        style { "{THEME}\n{LAYOUT}\n{CANVAS}\n{TOOLBOXES}\n{ICONS}" }
    }
}
