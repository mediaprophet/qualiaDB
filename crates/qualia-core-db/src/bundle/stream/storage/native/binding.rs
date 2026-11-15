//! Storage partition identity composed from actual shared signed selection.
use super::StoreError as E;
use crate::bundle::stream::{
    descriptor::CandidateIdentity,
    proof::Representation,
    selection::SelectedPlainEntry,
    storage::{IdentityFields, StorageClass, StorageIdentity},
};
use crate::net::qdnf::types::{Generation, StrongDigest};
#[derive(Clone, Copy, Debug)]
pub struct StorePartition {
    /// Host-selected public partition, never a disclosure grant.
    pub privacy: StrongDigest,
    pub policy_generation: Generation,
    pub storage_generation: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct SelectedStoreBinding {
    pub(super) identity: StorageIdentity,
    pub(super) candidate: CandidateIdentity,
    pub(super) ordinal: u32,
}
impl SelectedStoreBinding {
    pub fn from_selected(
        entry: &SelectedPlainEntry<'_>,
        partition: StorePartition,
    ) -> Result<Self, E> {
        if partition.policy_generation == Generation::ZERO {
            return Err(E::IdentityMismatch);
        }
        let candidate = entry.candidate_identity();
        let identity = StorageIdentity::new(IdentityFields {
            descriptor: candidate.descriptor_id(),
            context: candidate.context_id(),
            entry: entry.entry_id(),
            generation: candidate.generation(),
            representation: Representation::Plaintext,
            storage_class: StorageClass::PublicPlaintextCandidate,
            key_epoch: 0,
            logical_length: entry.logical_length(),
            root: entry.root(),
            privacy_partition: partition.privacy,
            policy_generation: partition.policy_generation.0,
            storage_generation: partition.storage_generation,
        })?;
        Ok(Self {
            identity,
            candidate,
            ordinal: entry.ordinal(),
        })
    }
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }
}
