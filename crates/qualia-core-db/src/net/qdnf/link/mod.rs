//! QLink discovery, adjacency and neighbor table.

pub mod adjacency;
pub mod budget;
pub mod challenge;
pub mod cookies;
pub mod discovery;
pub mod neighbor;

pub use adjacency::{link_id_from_key, Adjacency, AdjacencyState};
pub use budget::{admit_unauthenticated, PreAuthBudget, MAX_UNAUTH_BYTES, MAX_UNAUTH_WORK};
pub use challenge::{
    accept_beacon, insert_after_challenge, ChallengeTable, DiscoveryChallenge,
    MAX_PENDING_CHALLENGES,
};
pub use cookies::{CookieJar, ReachabilityCookie};
pub use discovery::{rotating_tag, Beacon, DiscoveryMode};
pub use neighbor::NeighborTable;
