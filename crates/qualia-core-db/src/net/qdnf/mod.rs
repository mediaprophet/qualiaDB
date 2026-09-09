//! Qualia Decentralized Network Fabric — native protocol libraries.
//!
//! This module tree does not import `libp2p`, DNS, or IP sockets. Native
//! Independent tests use `local-ipc-v1`. Raw Ethernet remains an explicit
//! platform gate until a privileged AF_PACKET backend is admitted.

pub mod authority;
pub mod bearer;
pub mod biometrics;
pub mod cells;
pub mod clinical;
pub mod contracts;
pub mod crypto;
pub mod economics;
pub mod errors;
pub mod evidence;
pub mod fabric;
pub mod fixtures;
pub mod frame;
pub mod harness;
pub mod link;
pub mod policy_labels;
pub mod profiles;
pub mod registries;
pub mod resolve;
pub mod route;
pub mod session;
pub mod types;
pub mod vertical;

pub use errors::QdnfError;
pub use types::{Generation, LinkId, StrongDigest};
