//! In-process cell scaling (E10.5) and fail-closed AF_XDP (E10.6).
//!
//! [`pass_budget`] charges the 42 MiB Sentinel ceiling as **accounting**.
//! [`scaling`] admits 1..=16 cells against one host budget with equal-share
//! fairness. [`hardware`] samples process RSS and NUMA; idle joules stay
//! unmeasured. [`af_xdp`] never opens a zero-copy NIC on this Linux userspace.

pub mod af_xdp;
pub mod hardware;
pub mod pass_budget;
pub mod scaling;

pub use af_xdp::{open_af_xdp, silent_zero_copy_fallback, umem_registered, AfXdp};
pub use hardware::{
    idle_energy_measured, idle_joules, numa_measured, numa_node, rapl_energy_uj, rss_bytes,
    rss_measured,
};
pub use pass_budget::{
    charge_pass, pass_maps_process_rss, reclaim_pass, PassCharge, PassGuard, PassOutcome,
    SENTINEL_PASS_TOTAL,
};
pub use scaling::{fair_grant, HardwareObservation, ScaleHost, MAX_SCALE_CELLS};
