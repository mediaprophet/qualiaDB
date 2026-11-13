use crate::bundle::stream::{
    descriptor::DescriptorError, policy::PolicyError, proof::ProofError, storage::StorageError,
};
use crate::net::qdnf::errors::QdnfError;
use crate::storage_driver::ranges::RangeError;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreError {
    Unsupported,
    Empty,
    Closed,
    Poisoned,
    Bounds,
    Overflow,
    ScratchCapacity,
    IdentityMismatch,
    PageMismatch,
    CheckpointConflict,
    Descriptor(DescriptorError),
    Proof(ProofError),
    Storage(StorageError),
    Backend(RangeError),
    Recovery(crate::storage_driver::ranges::recovery::RecoveryError),
    Runtime(QdnfError),
    Policy(PolicyError),
}
macro_rules! convert {
    ($t:ty,$v:ident) => {
        impl From<$t> for StoreError {
            fn from(e: $t) -> Self {
                Self::$v(e)
            }
        }
    };
}
convert!(DescriptorError, Descriptor);
convert!(ProofError, Proof);
convert!(StorageError, Storage);
convert!(RangeError, Backend);
convert!(
    crate::storage_driver::ranges::recovery::RecoveryError,
    Recovery
);
convert!(QdnfError, Runtime);
convert!(PolicyError, Policy);
