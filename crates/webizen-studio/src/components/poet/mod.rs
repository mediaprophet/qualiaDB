//! Poet HyperCanvas — structure from `C:\Projects\NLP\Canvas_Workbench`.

use dioxus::prelude::*;

mod bodies;
mod chest;
mod chrome;
mod containers;
mod engine;
mod gpu_frame;
mod kinds;
mod manifolds;
mod stage;
mod store;
mod styles;
mod tools;
mod vibe_console;
mod workbench;

pub use kinds::{ContainerKind, ManifoldId, ToolboxId};
pub use workbench::PoetWorkbench;

#[component]
pub fn PoetHarness() -> Element {
    rsx! { PoetWorkbench {} }
}
