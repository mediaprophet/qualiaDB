//! SI-04 attestation profiles. Adapters over existing native and W3C VC
//! runtimes. Do not replace those types.

pub mod award;
pub mod kinds;
pub mod native;
pub mod open_badge;
pub mod status;
pub mod w3c;

pub use award::{issue_capability_award, CapabilityAward};
pub use kinds::{AttestationKind, InstrumentAttestation};
pub use native::{issue_native, verify_native, NativeIssued};
pub use open_badge::{badge_export, BadgeImage, InstrumentBadgeExport};
pub use status::{AttestationStatus, StatusRegistry};
pub use w3c::{issue_w3c, verify_w3c, W3cIssued};
