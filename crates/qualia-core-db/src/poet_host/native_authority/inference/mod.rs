//! Current authenticated subject/identifier associations for inference adapters.
//! Authentication is not permission to install policy or execute inference.
//! The currently supported Human, SoftwareAgent and Organization kinds remain
//! distinct (not an exhaustive identity ontology), and a
//! subject may select any currently admitted identifier. No canonical DID is
//! imposed. Authoring delegation inspection is not an inference-purpose grant.
mod binding;
pub use binding::{
    AuthenticatedInferenceAssociation, IdentifierSelection, InferenceAssociationError,
    InferenceAssociationEvidence,
};
#[cfg(test)]
mod tests;

#[cfg(test)]
mod integration_tests;
