//! Scoped object operations over the existing central manager; no second vault.
//! Crypto policy admission is separate from recipient/publisher/disclosure authority.
mod checks;
mod open;
mod publication;
mod recipient;
mod seal;
mod workspace;
pub(crate) use open::UnwrappedMetadata;
pub use publication::{ManagedObjectPublication, PublicationRequest};
pub use recipient::{ManagedRecipientUnwrap, RecipientUnwrapRequest};
pub use seal::ManagedObjectSealer;
pub use workspace::PublicationWorkspace;
#[cfg(test)]
mod tests;

mod effect;
mod publication_signing;
