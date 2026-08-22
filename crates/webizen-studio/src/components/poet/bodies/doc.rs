use crate::components::poet::tools::cml::AuraTray;
use crate::components::poet::tools::rich_text::RichTextTool;
use dioxus::prelude::*;

#[component]
pub fn DocBody() -> Element {
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:4px;flex:1;",
            RichTextTool {}
            AuraTray {}
        }
    }
}
