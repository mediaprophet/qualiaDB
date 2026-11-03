//! Bind checked container index rows to existing custody operations.
//!
//! Structural selection does not authenticate a container, its semantic
//! declarations, or current policy. Store/read operations recheck their own
//! authorities and retain all existing uncertain-publication outcomes.
mod publish;
mod read;
mod recovery;
mod selection;

pub use publish::prepare_and_publish_container_store_with_quota;
pub use read::{read_container_observation, ContainerReadError};
pub use recovery::{
    recover_container_observation, ContainerRecoveryError, RecoveredContainerObservation,
};
pub use selection::{
    bind_container_observation, ContainerBindingAuthority, ContainerCustodySelection,
    ContainerSelectionError,
};
