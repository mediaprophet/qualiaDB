//! Focused checks import production code directly; no substitute implementation.
#[path = "../../../crates/qualia-core-db/src/net/qdnf/errors.rs"]
pub mod qdnf_errors;
#[path = "../../../crates/qualia-core-db/src/net/qdnf/types.rs"]
pub mod qdnf_types;
pub mod errors { pub use crate::qdnf_errors::*; pub use crate::qdnf_errors::QdnfError as CryptoError; }
pub mod net { pub mod qdnf { pub use crate::qdnf_types as types; pub use crate::qdnf_errors as errors; pub mod crypto { pub use crate::session_schedule as schedule; pub use crate::session_finished as finished; } } pub mod peer { pub use crate::parent_runtime as runtime; } }
#[path = "../../../crates/qualia-core-db/src/bundle/stream/proof/mod.rs"]
pub mod proof;
#[cfg(test)]
mod proof_tests;
#[cfg(test)]
mod allocation;

#[path = "../../../crates/qualia-core-db/src/bundle/format.rs"]
pub mod bundle_format;
#[path = "../../../crates/qualia-core-db/src/bundle/reader_limits.rs"]
pub mod bundle_limits;
#[path = "../../../crates/qualia-core-db/src/bundle/stream/index/mod.rs"]
pub mod bundle_index;
pub mod bundle {
    pub use crate::bundle_format as format;
    pub use crate::bundle_format::{BundleEntry, BundleError};
    pub use crate::bundle_limits::BundleReadLimits;
    pub mod stream { pub use crate::bundle_index as index; pub use crate::proof; pub use crate::descriptor; pub use crate::stream_storage as storage; pub use crate::stream_runtime as runtime; }
}
#[cfg(test)]
mod index_fixtures;

#[path = "../../../crates/qualia-core-db/src/crypto/network/types.rs"]
pub mod crypto_types;
pub use crypto_types as types;
#[path = "../../../crates/qualia-core-db/src/crypto/network/errors.rs"]
pub mod crypto_errors;
#[path = "../../../crates/qualia-core-db/src/crypto/network/mldsa.rs"]
pub mod crypto_mldsa;
#[path = "../../../crates/qualia-core-db/src/crypto/network/ed25519.rs"]
pub mod crypto_ed25519;
#[path = "../../../crates/qualia-core-db/src/crypto/network/dual_sign.rs"]
pub mod crypto_dual_sign;
#[path = "cose_codec/mod.rs"]
pub mod crypto_cose;
pub mod crypto { pub mod network {
    pub use crate::crypto_aead as aead;
    pub use crate::object_stream;
    pub use crate::crypto_types as types;
    pub use crate::crypto_kdf as kdf;
    pub use crate::crypto_transcript as transcript;
    pub use crate::crypto_secret_lease as secret_lease;
    pub use crate::crypto_network_digest as digest;
    pub use crate::crypto_errors as errors;
    pub use crate::crypto_mldsa as mldsa;
    pub use crate::crypto_ed25519 as ed25519;
    pub use crate::crypto_dual_sign as dual_sign;
    pub use crate::crypto_cose as cose;
} }

#[cfg(test)]
mod cose_fixtures;

#[path = "../../../crates/qualia-core-db/src/net/peer/runtime/mod.rs"]
pub mod parent_runtime;
#[path = "../../../crates/qualia-core-db/src/bundle/stream/runtime/mod.rs"]
pub mod stream_runtime;
#[cfg(test)]
mod runtime_allocations;

#[path = "../../../crates/qualia-core-db/src/specialized_libs/cryptographic_library/signing_profile.rs"]
pub mod library_signing_profile;
pub mod specialized_libs { pub mod cryptographic_library { pub use crate::library_signing_profile as signing_profile; } #[cfg(test)] pub mod computational_geometry { pub use crate::core_allocation_counter as allocation_counter; } }

#[path = "../../../crates/qualia-core-db/src/crypto/network/digest.rs"]
pub mod crypto_network_digest;
#[path = "../../../crates/qualia-core-db/src/bundle/stream/descriptor/mod.rs"]
pub mod descriptor;

#[cfg(test)]
mod descriptor_fixtures;

pub mod semantic_instruments;

#[path = "stream_storage/mod.rs"]
pub mod stream_storage;

#[cfg(test)]
mod closure_allocations;

#[cfg(test)]
mod storage_fixtures;

#[path = "../../../crates/qualia-core-db/src/crypto/network/kdf.rs"]
pub mod crypto_kdf;
#[path = "../../../crates/qualia-core-db/src/crypto/network/transcript.rs"]
pub mod crypto_transcript;
#[path = "../../../crates/qualia-core-db/src/crypto/network/secret_lease.rs"]
pub mod crypto_secret_lease;
#[path = "../../../crates/qualia-core-db/src/net/qdnf/crypto/schedule.rs"]
pub mod session_schedule;
#[path = "../../../crates/qualia-core-db/src/net/qdnf/crypto/finished.rs"]
pub mod session_finished;

#[cfg(test)]
mod endpoint_fixtures;

#[cfg(test)]
#[path = "../../../crates/qualia-core-db/src/specialized_libs/computational_geometry/allocation_counter.rs"]
pub mod core_allocation_counter;

#[cfg(test)]
mod span_fixtures;

#[path = "../../../crates/qualia-core-db/src/crypto/network/aead.rs"]
pub mod crypto_aead;
#[path = "../../../crates/qualia-core-db/src/crypto/network/object_stream/mod.rs"]
pub mod object_stream;

#[cfg(test)]
mod protected_fixtures;

pub mod sensing;
#[path = "../../../crates/qualia-core-db/src/storage_driver.rs"]
pub mod storage_driver;
#[path = "../../../crates/qualia-core-db/src/net/ebpf_filter.rs"]
pub mod ebpf_filter;
#[path = "../../../crates/qualia-core-db/src/zns_storage.rs"]
pub mod zns_storage;

#[path = "../../../crates/qualia-core-db/src/net/peer/replication/qbdl/wire/mod.rs"]
pub mod qbdl_service_wire;

#[cfg(test)]
mod service_wire_fixtures;

// Actual service carrier, with its canonical wire sibling.
pub use qbdl_service_wire as wire;
#[path = "../../../crates/qualia-core-db/src/net/peer/replication/qbdl/carrier/mod.rs"]
pub mod qbdl_service_carrier;
