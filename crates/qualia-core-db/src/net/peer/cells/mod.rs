//! Supervised cell admission (RT-02, **partial**).
//!
//! Reserves a bounded number of cell slots against the existing RT-01
//! [`LeaseTable`](crate::net::peer::runtime::LeaseTable) and
//! [`ReservationLedger`](crate::net::peer::runtime::ReservationLedger).
//! This library does **not** own a scheduler, construct [`crate::governance::webizen::SlgArena`],
//! or allocate backing storage.

pub mod admit;
pub mod host_owner;

pub use admit::{
    llm_shares_ordinary_network_reserve, CellProfile, CellSlot, CellTable, MAX_CELLS,
    MAX_ORDINARY_CELL,
};
pub use host_owner::{extra_identity_multiplies_host_budget, HostAdmission};
