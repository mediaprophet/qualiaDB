//! Contextual instrument panel — dynamic toolbar that changes based on the
//! selected container type. Appears above the canvas when a container is
//! selected, hides when the canvas is clicked.
//!
//! Ports the contextual instrument panel concept from the Canvas_Workbench mockup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod catalog;
mod chain;
mod commands;
mod dispatch;
mod panel;
mod ribbon;

pub use chain::{activate_chain, activate_chain_on_container, deactivate_chain};
pub use panel::{hide, show_for_container};
