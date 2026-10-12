//! Camera attachment to a native rigid body, expressed through the existing
//! orbit camera ABI. The current ABI preserves world-up rather than bank roll.
mod placement;
pub use placement::{mounted_camera,MountedCamera,MountError};
#[cfg(test)]
mod tests;
