//! Partition gate. Delivery is WouldBlock until healed.

use crate::net::qdnf::errors::QdnfError;

/// Independent partition flag for the harness pipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartitionGate {
    partitioned: bool,
}

impl PartitionGate {
    pub const fn open() -> Self {
        Self { partitioned: false }
    }

    pub const fn is_partitioned(self) -> bool {
        self.partitioned
    }

    pub fn partition(&mut self) {
        self.partitioned = true;
    }

    pub fn heal(&mut self) {
        self.partitioned = false;
    }

    pub fn admit(self) -> Result<(), QdnfError> {
        if self.partitioned {
            Err(QdnfError::WouldBlock)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_heal() {
        let mut gate = PartitionGate::open();
        assert_eq!(gate.admit(), Ok(()));
        gate.partition();
        assert_eq!(gate.admit(), Err(QdnfError::WouldBlock));
        gate.heal();
        assert_eq!(gate.admit(), Ok(()));
        assert!(!gate.is_partitioned());
    }
}
