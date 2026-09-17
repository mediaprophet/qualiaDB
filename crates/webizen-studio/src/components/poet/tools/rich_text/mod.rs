//! Customisable rich-text tool — we make this; TinyMCE is not a dependency.

mod spec;
mod toolbar;

#[allow(unused_imports)]
pub use spec::{RichCommand, ToolbarGroup, ToolbarSpec};
pub use toolbar::RichTextTool;
