//! Document container — customisable rich-text tool + gazetteer.

use crate::components::poet::tools::rich_text::RichTextTool;
use dioxus::prelude::*;

#[component]
pub fn DocBody() -> Element {
    rsx! { RichTextTool {} }
}
