//! Qualia ↔ W3C Solid bridge (personal pod server + consumer agent library).
//!
//! # Roles
//! - **Personal pod server** — LDP BasicContainer over a filesystem root (`solid serve`)
//! - **Consumer agent** — fetch/put remote Solid resources (`solid fetch` / `solid put`)
//! - **Demo OIDC** — local Solid-OIDC-shaped discovery for hackathon smoke only
//!   (`QUALIA_SOLID_DEMO_OIDC=1` or `BridgeConfig.demo_oidc`); see `NON_GOALS.md`
//!
//! # Allocation firewall
//! HTTP string payloads are hashed into `NQuin`s in this crate before crossing into
//! core hot paths (ADR 006).

pub mod consumer;
pub mod ldp_translator;
pub mod oidc_micro_idp;
pub mod pod_store;
pub mod solid_proxy;
pub mod vocab;

pub use consumer::{
    fetch_resource, fetch_to_quin_buffer, post_to_container, put_resource, turtle_to_quins,
    ConsumerError, FetchResult,
};
pub use pod_store::PodStore;
pub use solid_proxy::{run_bridge, start_proxy_daemon, start_proxy_daemon_with, BridgeConfig};
