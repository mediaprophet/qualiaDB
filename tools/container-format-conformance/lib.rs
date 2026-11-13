//! Tests the actual production codec files without the unrelated inference/render dependency graph.
#[path = "../../crates/qualia-core-db/src/container_10d/crc32c.rs"]
pub mod production_crc32c;
pub mod container_10d { pub use crate::production_crc32c as crc32c; }
#[path = "../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs"]
pub mod production_chunk_integrity;
#[path = "../../crates/qualia-core-db/src/q42/m64_matter/mod.rs"]
pub mod production_m64;
#[path = "../../crates/qualia-core-db/src/q42/emf_container/mod.rs"]
pub mod production_emf;
// Real retained-field ownership tests use the production reader/dense unpack;
// no Portal, device, acoustic substitute, or inference graph is linked.
#[path = "../../crates/qualia-core-db/src/render/portal_emf_state.rs"]
pub mod production_emf_state;
pub mod q42 {
    pub use crate::production_chunk_integrity as chunk_integrity;
    pub use crate::production_m64 as m64_matter;
    pub use crate::production_emf as emf_container;
}
