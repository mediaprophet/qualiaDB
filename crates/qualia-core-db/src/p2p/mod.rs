#[cfg(not(target_arch = "wasm32"))]
pub mod mesh_datagram;
#[cfg(not(target_arch = "wasm32"))]
pub mod mesh_service;
#[cfg(feature = "libp2p-compat")]
pub mod protocol;
pub mod routing;
#[cfg(not(target_arch = "wasm32"))]
pub mod social_qdnf;
#[cfg(not(target_arch = "wasm32"))]
pub mod social_webnet;
#[cfg(feature = "libp2p-compat")]
pub mod swarm;
#[cfg(all(not(target_arch = "wasm32"), feature = "libp2p-compat"))]
pub mod sync_node;
#[cfg(all(not(target_arch = "wasm32"), feature = "libp2p-compat"))]
pub mod sync_ops;
#[cfg(not(target_arch = "wasm32"))]
pub mod wireguard_runtime;
#[cfg(not(target_arch = "wasm32"))]
pub mod wireguard_userspace;
