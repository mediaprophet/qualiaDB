//! Bearer adapters. Each backend is a focused file.

pub mod contract;
pub mod fragment;
pub mod ipc;
pub mod leased_ipc;
pub mod lifecycle;
pub mod metadata;
pub mod mtu;
pub mod nym;
pub mod raw_ethernet;

pub use contract::{Bearer, BearerCapabilities, RecvMeta};
pub use fragment::{
    admit_and_buffer, decode_fragment, encode_fragment, fragment_payload_mtu, split_handshake,
    AdmitTicket, FragmentHeader, HandshakeReassembler, FRAG_HDR_LEN, MAX_HANDSHAKE_BYTES,
};
pub use ipc::{ipc_pair, IpcEndpoint};
pub use leased_ipc::{leased_ipc_pair, LeasedIpc};
pub use lifecycle::{BearerLifecycle, BearerPhase};
pub use mtu::{
    interface_loss_error, negotiate_mtu, reconnect_requires_new_admission, unfragmented_fit,
    DEFAULT_QDNF_MTU, MAX_QDNF_MTU, MIN_QDNF_MTU,
};
pub use nym::{
    decapsulate_nym, encapsulate_nym, NymSimulatedBearer, NYM_ENVELOPE_VERSION, NYM_HEADER_LEN,
    NYM_MAGIC, NYM_SPHINX_MTU,
};
pub use raw_ethernet::{
    decapsulate_ethernet, encapsulate_ethernet, ethernet_loop_pair, physical_two_host_qualified,
    probe_two_host, probe_veth_namespace, silent_ip_fallback, veth_namespace_qualified,
    EthernetEvidence, EthernetLoop, RawEthernet, TwoHostProbe, TwoHostProbeReason, DEV_ETHERTYPE,
};
