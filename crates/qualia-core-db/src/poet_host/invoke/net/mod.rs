//! Future seam: `qualia-net` (`net/`, `p2p/`). Handshake/dial is desktop/client-core.

mod peer;
pub mod pulse;

pub use peer::{peer_hash, sonic_pack};
