//! Peer construction. The ledger is sized to the caller’s cell budget.

use super::identity::ControllerIdentity;
use super::session_table::SessionTable;
use super::NativePeer;
use crate::net::peer::cells::host_owner::HostAdmission;
use crate::net::peer::cells::{CellSlot, MAX_ORDINARY_CELL};
use crate::net::peer::runtime::{CancelEpoch, ReservationLedger, ResourceBudget};
use crate::net::qdnf::bearer::IpcEndpoint;
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
        self.with_bearer_cell(bearer, None)
    }

    pub fn with_bearer_cell(
        self,
        bearer: IpcEndpoint,
        cell_slot: Option<CellSlot>,
    ) -> Result<NativePeer, QdnfError> {
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
            cell_slot,
        })
    }
}

impl NativePeer {
    /// Two connected native peers over `local-ipc-v1`. Transport only; no session.
    /// Both cells are admitted from one [`HostAdmission`] aggregate (`2 *` cell).
    pub fn pair_ipc(
        a_controller: &[u8],
        b_controller: &[u8],
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<(Self, Self), QdnfError> {
        let host_bytes = CELL_BYTES_DEFAULT
            .checked_mul(2)
            .ok_or(QdnfError::Capacity)?;
        let mut host = HostAdmission::new(host_bytes)?;
        super::exchange::pair_ipc_cells(
            &mut host,
            a_controller,
            b_controller,
            scope,
            mtu,
            CELL_BYTES_DEFAULT,
        )
    }

    /// Return this peer’s host cell to `host`. No-op when constructed without one.
    pub fn release_into_host(&mut self, host: &mut HostAdmission) -> Result<(), QdnfError> {
        match self.cell_slot.take() {
            Some(slot) => host.release_cell(slot),
            None => Ok(()),
        }
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
