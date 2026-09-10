//! Independent test harness. Product APIs are not used as the sole oracle.

pub mod allocation;
pub mod ci_classes;
pub mod claims;
pub mod clock;
pub mod entropy;
pub mod faults;
#[cfg(test)]
pub mod intercept;
pub mod limitations;
pub mod multihop;
pub mod negative;
pub mod oracle;
pub mod oracles;
pub mod partition;
pub mod qualification;
pub mod scenarios;

pub use allocation::{record_alloc, take_alloc, HarnessLease, ReservationTracker};
pub use clock::FakeClock;
pub use entropy::{fill_os, EntropySource, ProductionEntropy, SeededEntropy};
pub use faults::{Fault, FaultPipe, FaultSchedule};
pub use oracle::{independent_decode_header, independent_hop_limit, independent_magic_ok};
pub use oracles::{independent_finished, plaintext_on_wire, ProtectedView};
pub use partition::PartitionGate;
pub use qualification::{
    qualify, EvidenceClass, Instrument, InstrumentResult, InstrumentVerdict, BASELINE_COMMIT,
    LINUX_NATIVE_RUNNER, MSVC_RUNNER,
};
