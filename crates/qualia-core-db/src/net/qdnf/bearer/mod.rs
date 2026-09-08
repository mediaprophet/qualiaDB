//! Bearer adapters. Each backend is a focused file.

pub mod contract;
pub mod ipc;
pub mod leased_ipc;
pub mod lifecycle;
pub mod metadata;
pub mod raw_ethernet;

pub use contract::{Bearer, BearerCapabilities, RecvMeta};
pub use ipc::{ipc_pair, IpcEndpoint};
pub use leased_ipc::{leased_ipc_pair, LeasedIpc};
pub use lifecycle::{BearerLifecycle, BearerPhase};
pub use raw_ethernet::RawEthernet;
