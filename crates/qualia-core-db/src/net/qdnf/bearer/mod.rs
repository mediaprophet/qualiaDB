//! Bearer adapters. Each backend is a focused file.

pub mod contract;
pub mod fragment;
pub mod ipc;
pub mod leased_ipc;
pub mod lifecycle;
pub mod metadata;
pub mod mtu;
pub mod raw_ethernet;

pub use contract::{Bearer, BearerCapabilities, RecvMeta};
pub use fragment::{
    AdmitTicket, FRAG_HDR_LEN, FragmentHeader, HandshakeReassembler, MAX_HANDSHAKE_BYTES,
    admit_and_buffer, decode_fragment, encode_fragment, fragment_payload_mtu, split_handshake,
};
pub use ipc::{IpcEndpoint, ipc_pair};
pub use leased_ipc::{LeasedIpc, leased_ipc_pair};
pub use lifecycle::{BearerLifecycle, BearerPhase};
pub use mtu::{
    DEFAULT_QDNF_MTU, MAX_QDNF_MTU, MIN_QDNF_MTU, interface_loss_error, negotiate_mtu,
    reconnect_requires_new_admission, unfragmented_fit,
};
pub use raw_ethernet::{
    DEV_ETHERTYPE, EthernetEvidence, EthernetLoop, RawEthernet, TwoHostProbe, TwoHostProbeReason,
    decapsulate_ethernet, encapsulate_ethernet, ethernet_loop_pair, physical_two_host_qualified,
    probe_two_host, probe_veth_namespace, silent_ip_fallback, veth_namespace_qualified,
};
