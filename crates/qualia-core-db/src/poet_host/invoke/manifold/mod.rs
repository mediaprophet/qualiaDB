//! Future seam: `qualia-manifold` (`tensor/`, `container_10d/`, entity-view projection).
//! 3D/4D UI scripts call these; HTML/GPU canvas stays canvas (D13).

mod axes;
mod distance;
mod project;

pub use axes::taxonomy as axes;
pub use distance::distance;
pub use project::project;
