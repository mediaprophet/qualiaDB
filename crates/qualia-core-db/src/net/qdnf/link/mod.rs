//! QLink discovery, adjacency and neighbor table.

pub mod adjacency;
pub mod budget;
pub mod challenge;
pub mod cookies;
pub mod discovery;
pub mod neighbor;

pub use adjacency::{Adjacency, AdjacencyState, link_id_from_key};
pub use budget::{MAX_UNAUTH_BYTES, MAX_UNAUTH_WORK, PreAuthBudget, admit_unauthenticated};
pub use challenge::{
    ChallengeTable, DiscoveryChallenge, MAX_PENDING_CHALLENGES, accept_beacon,
    insert_after_challenge,
};
pub use cookies::{CookieJar, ReachabilityCookie};
pub use discovery::{Beacon, DiscoveryMode, rotating_tag};
pub use neighbor::NeighborTable;
