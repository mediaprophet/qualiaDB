//! Key provider request/grant surface.

pub mod grant;
pub mod software;

pub use grant::ProviderGrant;
pub use software::SoftwareBackend;
