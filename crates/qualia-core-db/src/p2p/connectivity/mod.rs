//! Native Internet adapters: demux, ICE, TURN codec, WSS relay, NAT64.
//!
//! No second authority engine. Policy lives in `net/peer/connectivity`.

#![cfg(not(target_arch = "wasm32"))]

pub mod demux;
pub mod establish;
pub mod ice;
pub mod nat64;
pub mod turn;
pub mod two_process;
pub mod wss;

pub use demux::{classify_overlay, classify_udp, OverlayPort, UdpClass};
pub use ice::{IceAgent, IceRole};
pub use nat64::{discover_pref64, numeric_ipv4_hint_insufficient_on_v6only, Pref64};
pub use turn::wss_is_not_turn;
pub use wss::local_authenticated_wss_executed;
