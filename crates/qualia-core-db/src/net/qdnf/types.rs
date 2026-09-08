//! Shared QDNF identifiers. Short hashes are lookup indexes, never proofs.

use super::errors::QdnfError;

/// SHA-384 digest. Full security identifier; never truncated into a Quin field.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct StrongDigest(pub [u8; 48]);

impl StrongDigest {
    pub const ZERO: Self = Self([0u8; 48]);

    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 48] {
        &self.0
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, QdnfError> {
        let arr: [u8; 48] = bytes.try_into().map_err(|_| QdnfError::Range)?;
        Ok(Self(arr))
    }
}

impl core::fmt::Debug for StrongDigest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "StrongDigest({:02x}{:02x}…)", self.0[0], self.0[1])
    }
}

/// Compact Q42 / FNV lookup index. Must not authorize delivery.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct QHashIndex(pub u64);

/// Epoch-scoped 128-bit QLink selector.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkId(pub [u8; 16]);

impl LinkId {
    pub const ZERO: Self = Self([0u8; 16]);

    #[inline]
    pub const fn is_zero(self) -> bool {
        let mut i = 0;
        while i < 16 {
            if self.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Per-session/route flow selector.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FlowId(pub [u8; 8]);

impl FlowId {
    pub const ZERO: Self = Self([0u8; 8]);
}

/// Adapter-observed bearer locator. Never taken from a claimed payload field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObservedLocator {
    pub bytes: [u8; 32],
    pub len: u8,
}

impl ObservedLocator {
    pub const EMPTY: Self = Self {
        bytes: [0u8; 32],
        len: 0,
    };

    pub fn from_slice(src: &[u8]) -> Result<Self, QdnfError> {
        if src.len() > 32 {
            return Err(QdnfError::Capacity);
        }
        let mut bytes = [0u8; 32];
        bytes[..src.len()].copy_from_slice(src);
        Ok(Self {
            bytes,
            len: src.len() as u8,
        })
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

/// Self-certifying DNI coordinate. Compact routing aid, not a DID.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DniCoordinate {
    pub network_id: u64,
    pub realm_id: u64,
    pub node_id: u64,
    pub service_id: u64,
}

impl DniCoordinate {
    pub const ZERO: Self = Self {
        network_id: 0,
        realm_id: 0,
        node_id: 0,
        service_id: 0,
    };
}

/// Non-wrapping generation. Exhaustion retires the slot.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Generation(pub u64);

impl Generation {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn next(self) -> Result<Self, QdnfError> {
        match self.0.checked_add(1) {
            Some(n) => Ok(Self(n)),
            None => Err(QdnfError::StaleGeneration),
        }
    }
}

/// Local operation identity. Survives path change; never recycled while live.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperationId(pub [u8; 16]);

impl OperationId {
    pub const ZERO: Self = Self([0u8; 16]);
}

/// Scope / cell epoch binding for leases and caches.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeEpoch {
    pub scope: u64,
    pub epoch: u64,
}

/// Profile / suite identifier. Unknown values fail closed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProfileId(pub u16);

impl ProfileId {
    pub const QPR_PQ_1: Self = Self(0x0101);
    pub const QDNF_CRYPTO_1: Self = Self(0x0001);
}

/// Sequence / packet number in a protocol context.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sequence(pub u64);

impl Sequence {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub const fn next(self) -> Result<Self, QdnfError> {
        match self.0.checked_add(1) {
            Some(n) => Ok(Self(n)),
            None => Err(QdnfError::Capacity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_digest_is_48_bytes() {
        assert_eq!(core::mem::size_of::<StrongDigest>(), 48);
        assert_ne!(
            core::mem::size_of::<StrongDigest>(),
            core::mem::size_of::<QHashIndex>()
        );
    }

    #[test]
    fn generation_does_not_wrap() {
        assert_eq!(Generation(u64::MAX).next(), Err(QdnfError::StaleGeneration));
        assert_eq!(Generation(1).next(), Ok(Generation(2)));
    }

    #[test]
    fn locator_rejects_oversized() {
        assert_eq!(
            ObservedLocator::from_slice(&[0u8; 33]),
            Err(QdnfError::Capacity)
        );
    }
}
