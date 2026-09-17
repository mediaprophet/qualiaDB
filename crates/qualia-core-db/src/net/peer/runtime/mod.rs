//! Peer runtime: leases, admission ledger, events/effects, scheduler, cancel.

pub mod cancel;
pub mod events;
pub mod generations;
pub mod handles;
pub mod leases;
pub mod ledger;
pub mod resources;
pub mod scheduler;

pub use cancel::{CancelEpoch, OperationHandle, OperationTable};
pub use events::{EffectKind, EffectQueue, EventKind, KernelEffect, KernelEvent};
pub use generations::bump_generation;
pub use handles::{BufferLease, LeaseHandle};
pub use leases::{LeaseTable, LEASE_SLOTS};
pub use ledger::{AdmissionScopes, ReservationHandle, ReservationLedger, ResourceBudget};
pub use resources::ResourceGovernor;
pub use scheduler::{
    FairScheduler, PollOutcome, ScheduledWork, WorkClass, BACKGROUND_CAP, CONTROL_CAP,
    INTERACTIVE_CAP, MAX_SCOPES,
};
