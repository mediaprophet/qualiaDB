//! Non-wrapping generation handles.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LeaseHandle {
    pub slot: u16,
    pub generation: Generation,
}

impl LeaseHandle {
    pub const INVALID: Self = Self {
        slot: 0,
        generation: Generation::ZERO,
    };

    pub fn matches(self, expected: Self) -> Result<(), QdnfError> {
        if self.slot != expected.slot || self.generation != expected.generation {
            Err(QdnfError::StaleGeneration)
        } else {
            Ok(())
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferLease {
    pub handle: LeaseHandle,
    pub capacity: u32,
    pub initialized: u32,
    pub exclusive: bool,
}
