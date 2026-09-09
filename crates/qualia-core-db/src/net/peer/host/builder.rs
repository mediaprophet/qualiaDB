//! Peer construction. The ledger is sized to the caller’s cell budget.

use super::identity::ControllerIdentity;
use super::session_table::SessionTable;
use super::NativePeer;
use crate::net::peer::cells::MAX_ORDINARY_CELL;
use crate::net::peer::runtime::{CancelEpoch, ReservationLedger, ResourceBudget};
use crate::net::qdnf::bearer::{ipc_pair, IpcEndpoint};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{LinkId, ScopeEpoch};

/// Default per-peer cell when the caller does not specify a budget.
pub const CELL_BYTES_DEFAULT: u64 = 64 * 1024 * 1024;

const LEDGER_WORK: u64 = 1024;
const LEDGER_IO: u64 = 256;

#[derive(Debug)]
pub struct PeerBuilder {
    identity: ControllerIdentity,
    local_link: LinkId,
    cell_bytes: u64,
}

impl PeerBuilder {
    pub fn new(controller: &[u8], local_link: LinkId, cell_bytes: u64) -> Result<Self, QdnfError> {
        if cell_bytes == 0 || cell_bytes > MAX_ORDINARY_CELL {
            return Err(QdnfError::Capacity);
        }
        Ok(Self {
            identity: ControllerIdentity::from_controller(controller)?,
            local_link,
            cell_bytes,
        })
    }

    pub fn with_bearer(self, bearer: IpcEndpoint) -> Result<NativePeer, QdnfError> {
        let cap = ResourceBudget {
            bytes: self.cell_bytes,
            work: LEDGER_WORK,
            io: LEDGER_IO,
        };
        Ok(NativePeer {
            identity: self.identity,
            local_link: self.local_link,
            neighbors: crate::net::qdnf::link::NeighborTable::new(),
            ledger: ReservationLedger::new(cap, cap, cap, cap, cap),
            sessions: SessionTable::new(),
            primary: None,
            bearer,
            cancel_epoch: CancelEpoch::ZERO,
            cancelled: false,
        })
    }
}

impl NativePeer {
    /// Two connected native peers over `local-ipc-v1`. Transport only; no session.
    pub fn pair_ipc(
        a_controller: &[u8],
        b_controller: &[u8],
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<(Self, Self), QdnfError> {
        let a_id = ControllerIdentity::from_controller(a_controller)?;
        let b_id = ControllerIdentity::from_controller(b_controller)?;
        if a_id == b_id {
            return Err(QdnfError::Conflict);
        }
        let (a_bearer, b_bearer) = ipc_pair(scope, mtu)?;
        let mut a_link = LinkId::ZERO;
        a_link.0[0] = 1;
        let mut b_link = LinkId::ZERO;
        b_link.0[0] = 2;
        Ok((
            PeerBuilder::new(a_controller, a_link, CELL_BYTES_DEFAULT)?.with_bearer(a_bearer)?,
            PeerBuilder::new(b_controller, b_link, CELL_BYTES_DEFAULT)?.with_bearer(b_bearer)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_rejects_zero_and_oversize_cell_bytes() {
        let link = LinkId::ZERO;
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", link, 0).unwrap_err(),
            QdnfError::Capacity
        );
        assert_eq!(
            PeerBuilder::new(b"did:q42:a", link, MAX_ORDINARY_CELL.saturating_add(1)).unwrap_err(),
            QdnfError::Capacity
        );
        assert!(PeerBuilder::new(b"did:q42:a", link, MAX_ORDINARY_CELL).is_ok());
        assert!(PeerBuilder::new(b"did:q42:a", link, CELL_BYTES_DEFAULT).is_ok());
    }
}
