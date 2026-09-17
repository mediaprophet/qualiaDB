//! FND-03 versioned interface golden fixtures (PARTIAL).
//!
//! Identifier size/role table plus a live empty DiscoveryBeacon vector.
//! Not an independent oracle — that remains QA-01.03.

pub mod golden;
pub mod ids;

pub use golden::{
    allow_classical_retry, capture_empty_discovery_beacon, require_known_profile,
    EMPTY_BEACON_PREFIX, GOLDEN_BASE_HEADER_LEN, GOLDEN_MAGIC, GOLDEN_VERSION,
};
pub use ids::{
    digest_sizes_match_types, short_hash_is_authority, QHASH_INDEX_LEN, STRONG_DIGEST_LEN,
};

#[cfg(not(target_arch = "wasm32"))]
pub use golden::NetworkCursor;
