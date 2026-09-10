//! Socket-independent Internet connection manager contract.
//!
//! Policy, generations, candidates and durable delivery live here.
//! Native sockets, ICE/TURN/WSS adapters live in `p2p/connectivity`.

pub mod browser;
pub mod candidates;
pub mod durable;
pub mod evidence;
pub mod invitation;
pub mod matrix;
pub mod mtu;
pub mod path;
pub mod planner;
pub mod policy;
pub mod rendezvous;
pub mod state;

pub use browser::{BrowserBearer, BrowserProfile, IceTransportPolicy};
pub use candidates::{
    gather_permitted, pair_priority, AddrFamily, Candidate, CandidateKind, CandidateTables,
    MAX_ACTIVE_PATHS, MAX_ATTEMPTS, MAX_PAIRS,
};
pub use durable::{Delivery, DurableQueue};
pub use evidence::{
    address_dependent_is_not_universal_relay_law, browser_turn_interop_executed,
    connection_manager_implemented, internet_two_host_handshake_executed, public_relay_dialed,
};
pub use invitation::{
    phrase_must_not_mint_wg_keys, sign_invitation, verify_invitation, Invitation,
};
pub use path::{
    consent_fresh, may_activate_send_path, relay_ip_is_not_direct_candidate, resolve_role_conflict,
    ObservedSource, CONSENT_INTERVAL_MS, CONSENT_TIMEOUT_MS,
};
pub use planner::{ordinary_first_actions, tick, Schedule, Scheduled};
pub use policy::{Disclosure, PathPolicy};
pub use rendezvous::{common_relay, RendezvousStore};
pub use state::{ConnError, ConnSlot, ConnState};
