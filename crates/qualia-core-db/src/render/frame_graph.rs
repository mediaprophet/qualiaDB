//! Allocation-free frame graph resource and pass dependency model.
pub use etalon_render::frame_quality::frame_graph::*;

#[cfg(test)]
#[path = "frame_graph_tests.rs"]
mod tests;
