use crate::net::qdnf::types::StrongDigest;

pub const LEAF_BYTES: u64 = 16_384;
pub const MAX_LOGICAL_BYTES: u64 = 1 << 40;
pub const MAX_SIBLINGS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Representation {
    Plaintext = 0,
    Ciphertext = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofError {
    Limit,
    InvalidSpec,
    Index,
    Length,
    ProofLength,
    RootMismatch,
    Incomplete,
    Complete,
    Capacity,
    Sink,
    Poisoned,
}

/// Structurally checked tree parameters, not authenticated publisher metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeSpec {
    pub(crate) context: StrongDigest,
    pub(crate) entry: u32,
    pub(crate) generation: u64,
    pub(crate) representation: Representation,
    pub(crate) epoch: u32,
    pub(crate) length: u64,
}
impl TreeSpec {
    pub fn new(
        context: StrongDigest,
        entry: u32,
        generation: u64,
        representation: Representation,
        epoch: u32,
        length: u64,
    ) -> Result<Self, ProofError> {
        if length > MAX_LOGICAL_BYTES {
            return Err(ProofError::Limit);
        }
        if representation == Representation::Plaintext && epoch != 0 {
            return Err(ProofError::InvalidSpec);
        }
        Ok(Self {
            context,
            entry,
            generation,
            representation,
            epoch,
            length,
        })
    }
    pub const fn context(&self) -> StrongDigest {
        self.context
    }
    pub const fn entry_id(&self) -> u32 {
        self.entry
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn representation(&self) -> Representation {
        self.representation
    }
    pub const fn key_epoch(&self) -> u32 {
        self.epoch
    }
    pub const fn logical_length(&self) -> u64 {
        self.length
    }
    pub const fn leaf_count(&self) -> u64 {
        self.length / LEAF_BYTES + (self.length % LEAF_BYTES != 0) as u64
    }
    pub fn logical_leaf_length(&self, index: u64) -> Result<usize, ProofError> {
        if index >= self.leaf_count() {
            return Err(ProofError::Index);
        }
        Ok((self.length - index * LEAF_BYTES).min(LEAF_BYTES) as usize)
    }
    pub fn encoded_leaf_length(&self, index: u64) -> Result<usize, ProofError> {
        Ok(self.logical_leaf_length(index)?
            + if self.representation == Representation::Ciphertext {
                16
            } else {
                0
            })
    }
}
