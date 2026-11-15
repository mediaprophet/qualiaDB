//! Native controller enrollment. Requests prove a key, never a legal identity.
mod owner;
mod package_review_admin;
mod requests;
mod signer;
pub use owner::{EnrollmentApproval, EnrollmentPermission};
pub use requests::{EnrollmentRecord, EnrollmentRequest, EnrollmentStatus, ENROLLMENT_CONTEXT};
pub use signer::NativeSigner;
#[cfg(all(test, feature = "sanctuary-crypto"))]
mod tests;
