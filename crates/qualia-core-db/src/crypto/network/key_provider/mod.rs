//! Key provider request/grant surface.

pub mod grant;
pub mod hardware;
pub mod software;

pub use grant::ProviderGrant;
pub use hardware::HardwareBackend;
pub use software::SoftwareBackend;
