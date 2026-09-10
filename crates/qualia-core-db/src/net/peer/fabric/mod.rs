//! Capability-scoped connection fabric (CSCP, `draft-webcivics-cscp-00`).
//!
//! Applications call [`connect`]. The fabric admits intent, resolves a private
//! contact, excludes prohibited paths, then scores the remainder. CSCP is a
//! control protocol. This crate does not implement QUIC, MASQUE, or a new cipher.

pub mod carrier;
pub mod connect;
pub mod contact;
pub mod evidence;
pub mod experiments;
pub mod intent;
pub mod kernel;
pub mod lease;
pub mod receipt;
pub mod select;
pub mod session;
pub mod wire;

#[cfg(not(target_arch = "wasm32"))]
pub mod relay;

pub use carrier::{CarrierKind, PathClass};
pub use connect::{connect, ConnectHandle, DriveOutcome, Fabric};
pub use contact::{ContactDescriptor, LocatorKind};
pub use evidence::{ObserverKind, PathEvidence, TransportWitness};
pub use intent::{ConnectionIntent, PeerId, ProtectionPolicy, Purpose, PurposeClass};
pub use kernel::{FabricError, FabricState, Kernel, KernelEffect, KernelEvent};
pub use lease::{CustodyLease, RelayLease};
pub use receipt::{OpReceipt, ReceiptStatus};
pub use select::{exclude_then_rank, RankedPath};
pub use session::SessionReady;
pub use wire::{connect_from_wire, encode_connect_request, CscpError};
