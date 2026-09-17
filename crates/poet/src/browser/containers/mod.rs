//! Container rendering: glassmorphism containers with type tags, badges, ports.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod attrs;
mod body;
mod body_core;
mod body_health;
mod body_ontology;
mod body_project;
mod body_studio;
mod shell;

pub use shell::build_container;
