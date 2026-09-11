//! Supervised cell admission (RT-02 / E10, **partial**).
//!
//! Reserves a bounded number of cell slots against the existing RT-01
//! [`LeaseTable`](crate::net::peer::runtime::LeaseTable) and
//! [`ReservationLedger`](crate::net::peer::runtime::ReservationLedger).
//! This library does **not** own a scheduler, construct [`crate::governance::webizen::SlgArena`],
//! or allocate backing storage.
//!
//! E10.1 production table is 16 caller-backed descriptors / [`admit::MAX_CELLS`]
//! slots under one [`HostAdmission`]. E10.5 scaling measurement and E10.6
//! AF_XDP are not implemented here (`PlatformUnsupported` remains correct).

pub mod admit;
pub mod descriptors;
pub mod host_owner;
pub mod isolation;
pub mod pass_budget;
pub mod spsc;
pub mod workers;

pub use admit::{
    llm_shares_ordinary_network_reserve, CellProfile, CellSlot, CellTable, MAX_CELLS,
    MAX_ORDINARY_CELL,
};
pub use descriptors::{admit_descriptors, CellDescriptor, MAX_DESCRIPTOR_CELLS};
pub use host_owner::{extra_identity_multiplies_host_budget, HostAdmission, MAX_HOST_BYTES};
pub use isolation::{admit_llm_cannot_starve_network, llm_backing_profile_allowed};
pub use pass_budget::{
    charge_pass, reclaim_pass, PassCharge, PassOutcome, SENTINEL_PASS_TOTAL,
};
pub use spsc::{OwnershipReceipt, SpscQueue, SPSC_CAP};
pub use workers::{
    cancel_worker, drain_and_handover, fail_worker, release_worker, start_worker, WorkerFence,
};
