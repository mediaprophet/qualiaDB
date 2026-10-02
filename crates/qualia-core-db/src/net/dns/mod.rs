//! QualiaDB Native Zero-Heap DNS & SDN Engine.
//!
//! Provides an end-to-end, pure-Rust implementation of DNS wire parsing,
//! serialisation, Q42 Super-Quin integration, zero-allocation ring-buffer
//! caching, Socially Defined Network (SDN) Front-Door discovery, native
//! UDP socket resolution, comprehensive record ontology registry,
//! lossless arbitrary-length record chunking, and persistent binary archives
//! without any 3rd-party DNS crates.
//!
//! Conforms to QualiaDB Rule 0-A (zero-heap in hot paths) and Rule 0-B.

pub mod archive;
pub mod cache_ring;
pub mod lossless;
pub mod ontology;
pub mod quin_records;
pub mod resolver;
pub mod sdn;
pub mod wire;

pub use archive::*;
pub use cache_ring::DnsCacheRing;
pub use lossless::*;
pub use ontology::*;
pub use quin_records::*;
pub use resolver::{QualiaDnsResolver, ResolverConfig};
pub use sdn::{parse_front_door_txt, parse_ns_encoded_did, SdnFrontDoorView};
pub use wire::{
    build_query_packet, decode_domain_name, encode_domain_name, parse_records, DnsHeader,
    DnsQuestion, DnsRecordView, DnsType,
};
