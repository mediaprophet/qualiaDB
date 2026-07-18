//! Envelope / SFX temporal descriptors (caller-buffered, zero-heap hot path).
//!
//! Amplitude-envelope tracking plus a family of temporal / sound-effect (SFX)
//! shape descriptors (Essentia-style): attack time, temporal-centroid ratios,
//! decay characterisation and flatness. Each leaf module holds exactly one
//! public function.

pub mod centroid_time;
pub mod derivative_sfx;
pub mod flatness_sfx;
pub mod follower;
pub mod log_attack_time;
pub mod max_to_total;
pub mod min_to_total;
pub mod strong_decay;
pub mod tc_to_total;

pub use centroid_time::centroid_time;
pub use derivative_sfx::derivative_sfx;
pub use flatness_sfx::flatness_sfx;
pub use follower::envelope_follow;
pub use log_attack_time::log_attack_time;
pub use max_to_total::max_to_total;
pub use min_to_total::min_to_total;
pub use strong_decay::strong_decay;
pub use tc_to_total::tc_to_total;
