//! Poet HyperCanvas — structure from `C:\Projects\NLP\Canvas_Workbench` and `POET-SPEC-001..023`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use dioxus::prelude::*;

mod bodies;
mod chest;
mod chrome;
mod containers;
pub(crate) mod engine;
mod gpu_frame;
mod host;
pub mod icons;
mod inspector;
pub mod instrument_bay;
mod kinds;
pub mod lexicon_bay;
mod manifolds;
mod radial_menu;
mod stage;
mod store;
mod styles;
mod tools;
mod vibe_console;
mod workbench;

pub use icons::IconBadge;
pub use kinds::{ContainerKind, ManifoldId, ToolboxId};
pub use radial_menu::{RadialActionRing, RadialState};
pub use workbench::PoetWorkbench;

#[component]
pub fn PoetHarness() -> Element {
    rsx! { PoetWorkbench {} }
}
