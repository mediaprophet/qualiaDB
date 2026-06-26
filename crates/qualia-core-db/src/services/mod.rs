//! `services` category (reorg).

#[cfg(not(target_arch = "wasm32"))]
pub mod daemon;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon_graph;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon_query;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon_swarm;
#[cfg(not(target_arch = "wasm32"))]
pub mod daemon_tensor;
#[cfg(not(target_arch = "wasm32"))]
pub mod chat_relay_daemon;
#[cfg(not(target_arch = "wasm32"))]
pub mod webizen_server;
#[cfg(not(target_arch = "wasm32"))]
pub mod rpc;
#[cfg(not(target_arch = "wasm32"))]
pub mod webtorrent_routes;
#[cfg(not(target_arch = "wasm32"))]
pub mod webtorrent_seeder;
pub mod solid_ldp;
#[cfg(not(target_arch = "wasm32"))]
pub mod ilp_dispatcher;
