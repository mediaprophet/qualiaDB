//! Versioned cold contract adapters and caller-buffered collapse admission.
mod collapse;
mod identity;
mod records;
mod transfer;
mod validation;

pub use collapse::*;
pub use identity::*;
pub use records::*;
pub use transfer::*;
pub use validation::*;

#[cfg(test)]
mod tests;
