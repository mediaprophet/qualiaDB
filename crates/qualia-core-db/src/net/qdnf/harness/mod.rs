//! Independent test harness. Product APIs are not used as the sole oracle.

pub mod allocation;
pub mod clock;
pub mod entropy;
pub mod faults;
pub mod oracle;
pub mod partition;

pub use allocation::{record_alloc, take_alloc, HarnessLease, ReservationTracker};
pub use clock::FakeClock;
pub use entropy::{fill_os, EntropySource, ProductionEntropy, SeededEntropy};
pub use faults::{Fault, FaultPipe, FaultSchedule};
pub use oracle::{independent_decode_header, independent_hop_limit, independent_magic_ok};
pub use partition::PartitionGate;
