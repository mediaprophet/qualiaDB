//! Zero-Allocation In-Memory DNS Record Types Ontology Registry.
//!
//! Exposes a compile-time static ontology defining the characteristics, RFCs,
//! wire schemas, categories, and lossless QualiaDB storage strategies for all
//! standard DNS record types (matching `ontologies/dns_record_types.ttl`).
//!
//! Conforms to QualiaDB Rule 0-A (zero-heap in hot paths) and Rule 0-B (< 500 lines).

#![allow(dead_code)]

use super::wire::DnsType;

/// Semantic category of the DNS Resource Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsCategory {
    Address,
    Routing,
    Delegation,
    Security,
    Service,
    Information,
    Pseudo,
    Experimental,
}

/// Standards-track status according to IETF / IANA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsStatus {
    Standard,
    Proposed,
    Experimental,
    Historic,
    Obsolete,
}

/// Zero-heap QualiaDB storage strategy for this record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualiaStorageStrategy {
    /// Fits losslessly in a single 48-byte Super-Quin (e.g. A, AAAA, MX, SRV, CNAME, NS, PTR).
    InlineSuperQuin,
    /// Arbitrary length payload chunked into contiguous 16-byte Super-Quins with ECC parity.
    LosslessQuinChunkChain,
}

/// Comprehensive metadata descriptor for a single DNS record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsRecordTypeDescriptor {
    pub type_code: u16,
    pub type_name: &'static str,
    pub defining_rfc: &'static str,
    pub description: &'static str,
    pub schema_definition: &'static str,
    pub category: DnsCategory,
    pub status: DnsStatus,
    pub storage_strategy: QualiaStorageStrategy,
}

impl DnsRecordTypeDescriptor {
    pub const fn is_lossless_inline(&self) -> bool {
        matches!(self.storage_strategy, QualiaStorageStrategy::InlineSuperQuin)
    }
}

/// Authoritative static table of all standard DNS record types.
pub static DNS_ONTOLOGY_REGISTRY: &[DnsRecordTypeDescriptor] = &[
    DnsRecordTypeDescriptor {
        type_code: 1,
        type_name: "A",
        defining_rfc: "RFC 1035",
        description: "32-bit IPv4 address",
        schema_definition: "ipv4: [u8; 4]",
        category: DnsCategory::Address,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 2,
        type_name: "NS",
        defining_rfc: "RFC 1035",
        description: "Authoritative name server for the zone",
        schema_definition: "nsdname: domain name",
        category: DnsCategory::Delegation,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 5,
        type_name: "CNAME",
        defining_rfc: "RFC 1035",
        description: "Canonical name for an alias",
        schema_definition: "cname: domain name",
        category: DnsCategory::Delegation,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 6,
        type_name: "SOA",
        defining_rfc: "RFC 1035",
        description: "Start of Authority zone parameters",
        schema_definition: "mname: domain, rname: domain, serial: u32, refresh: u32, retry: u32, expire: u32, minimum: u32",
        category: DnsCategory::Information,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 12,
        type_name: "PTR",
        defining_rfc: "RFC 1035",
        description: "Pointer to a canonical name (reverse DNS)",
        schema_definition: "ptrdname: domain name",
        category: DnsCategory::Delegation,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 13,
        type_name: "HINFO",
        defining_rfc: "RFC 8482",
        description: "Host information (CPU and Operating System)",
        schema_definition: "cpu: string, os: string",
        category: DnsCategory::Information,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 15,
        type_name: "MX",
        defining_rfc: "RFC 1035, RFC 7505",
        description: "Mail exchange server preference and host",
        schema_definition: "preference: u16, exchange: domain name",
        category: DnsCategory::Routing,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 16,
        type_name: "TXT",
        defining_rfc: "RFC 1035",
        description: "Text strings up to 64KB for machine/human metadata (SPF, DKIM, QDP)",
        schema_definition: "txt_data: [length-prefixed strings]",
        category: DnsCategory::Information,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 28,
        type_name: "AAAA",
        defining_rfc: "RFC 3596",
        description: "128-bit IPv6 address",
        schema_definition: "ipv6: [u8; 16] (lossless object[0..8] + context[8..16])",
        category: DnsCategory::Address,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 29,
        type_name: "LOC",
        defining_rfc: "RFC 1876",
        description: "Geographic location information (lat, long, altitude)",
        schema_definition: "version: u8, size: u8, hp: u8, vp: u8, lat: u32, lon: u32, alt: u32",
        category: DnsCategory::Information,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 33,
        type_name: "SRV",
        defining_rfc: "RFC 2782",
        description: "Generalized service location record",
        schema_definition: "priority: u16, weight: u16, port: u16, target: domain name",
        category: DnsCategory::Routing,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 35,
        type_name: "NAPTR",
        defining_rfc: "RFC 3403",
        description: "Naming Authority Pointer for ENUM/SIP regex rewrite",
        schema_definition: "order: u16, pref: u16, flags: str, services: str, regexp: str, replacement: domain",
        category: DnsCategory::Routing,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 37,
        type_name: "CERT",
        defining_rfc: "RFC 4398",
        description: "PKIX, SPKI, PGP certificate container",
        schema_definition: "type: u16, key_tag: u16, algorithm: u8, cert_data: bytes",
        category: DnsCategory::Information,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 39,
        type_name: "DNAME",
        defining_rfc: "RFC 6672",
        description: "Delegation Name redirect for an entire subtree",
        schema_definition: "target: domain name",
        category: DnsCategory::Delegation,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 41,
        type_name: "OPT",
        defining_rfc: "RFC 6891",
        description: "EDNS0 option pseudo-record for extended flags and payload",
        schema_definition: "udp_payload_size: u16, ext_rcode: u8, version: u8, flags: u16, options: bytes",
        category: DnsCategory::Pseudo,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 43,
        type_name: "DS",
        defining_rfc: "RFC 4034",
        description: "Delegation Signer pointing to child DNSKEY",
        schema_definition: "key_tag: u16, algorithm: u8, digest_type: u8, digest: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 44,
        type_name: "SSHFP",
        defining_rfc: "RFC 4255",
        description: "SSH Public Key Fingerprint",
        schema_definition: "algorithm: u8, fp_type: u8, fingerprint: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 45,
        type_name: "IPSECKEY",
        defining_rfc: "RFC 4025",
        description: "IPsec keying material",
        schema_definition: "precedence: u8, gw_type: u8, algorithm: u8, gateway: bytes, key: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 46,
        type_name: "RRSIG",
        defining_rfc: "RFC 4034",
        description: "DNSSEC cryptographic signature for an RRSet",
        schema_definition: "type_covered: u16, alg: u8, labels: u8, orig_ttl: u32, sig_exp: u32, sig_inc: u32, key_tag: u16, signer: domain, sig: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 47,
        type_name: "NSEC",
        defining_rfc: "RFC 4034",
        description: "Next Secure denial of existence record",
        schema_definition: "next_domain: domain name, type_bit_maps: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 48,
        type_name: "DNSKEY",
        defining_rfc: "RFC 4034",
        description: "DNSSEC public signing key",
        schema_definition: "flags: u16, protocol: u8, algorithm: u8, public_key: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 50,
        type_name: "NSEC3",
        defining_rfc: "RFC 5155",
        description: "Hashed authenticated denial of existence",
        schema_definition: "hash_alg: u8, flags: u8, iterations: u16, salt: bytes, next_hashed: bytes, type_bit_maps: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 51,
        type_name: "NSEC3PARAM",
        defining_rfc: "RFC 5155",
        description: "Parameters for NSEC3 records",
        schema_definition: "hash_alg: u8, flags: u8, iterations: u16, salt: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 52,
        type_name: "TLSA",
        defining_rfc: "RFC 6698",
        description: "DANE TLS certificate association",
        schema_definition: "usage: u8, selector: u8, matching_type: u8, cert_association_data: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 53,
        type_name: "SMIMEA",
        defining_rfc: "RFC 8162",
        description: "S/MIME certificate association",
        schema_definition: "usage: u8, selector: u8, matching_type: u8, cert_association_data: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 59,
        type_name: "CDS",
        defining_rfc: "RFC 7344",
        description: "Child DS for automated delegation signer updates",
        schema_definition: "key_tag: u16, algorithm: u8, digest_type: u8, digest: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 60,
        type_name: "CDNSKEY",
        defining_rfc: "RFC 7344",
        description: "Child DNSKEY for automated DNSSEC updates",
        schema_definition: "flags: u16, protocol: u8, algorithm: u8, public_key: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 61,
        type_name: "OPENPGPKEY",
        defining_rfc: "RFC 7929",
        description: "OpenPGP public key keyring",
        schema_definition: "public_keyring: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 62,
        type_name: "CSYNC",
        defining_rfc: "RFC 7477",
        description: "Child-to-Parent synchronization of records",
        schema_definition: "serial: u32, flags: u16, type_bit_maps: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 63,
        type_name: "ZONEMD",
        defining_rfc: "RFC 8976",
        description: "Zone digest for complete zone content verification",
        schema_definition: "serial: u32, scheme: u8, algorithm: u8, digest: bytes",
        category: DnsCategory::Security,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 64,
        type_name: "SVCB",
        defining_rfc: "RFC 9460",
        description: "General service binding for transport protocols",
        schema_definition: "priority: u16, target_name: domain name, svc_params: bytes",
        category: DnsCategory::Service,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 65,
        type_name: "HTTPS",
        defining_rfc: "RFC 9460",
        description: "Service binding for HTTPS with HTTP/2, HTTP/3, and ECH",
        schema_definition: "priority: u16, target_name: domain name, svc_params: bytes",
        category: DnsCategory::Service,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 255,
        type_name: "ANY",
        defining_rfc: "RFC 1035, RFC 8482",
        description: "Meta-query type requesting all available records",
        schema_definition: "none (query only)",
        category: DnsCategory::Pseudo,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::InlineSuperQuin,
    },
    DnsRecordTypeDescriptor {
        type_code: 256,
        type_name: "URI",
        defining_rfc: "RFC 7553",
        description: "Publishing mappings from hostnames to URIs",
        schema_definition: "priority: u16, weight: u16, target: string",
        category: DnsCategory::Service,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
    DnsRecordTypeDescriptor {
        type_code: 257,
        type_name: "CAA",
        defining_rfc: "RFC 8659",
        description: "Certification Authority Authorization constraint",
        schema_definition: "flags: u8, tag: string, value: string",
        category: DnsCategory::Service,
        status: DnsStatus::Standard,
        storage_strategy: QualiaStorageStrategy::LosslessQuinChunkChain,
    },
];

/// Look up a DNS record descriptor by its numeric RR type code.
pub fn lookup_by_code(code: u16) -> Option<&'static DnsRecordTypeDescriptor> {
    let mut i = 0;
    while i < DNS_ONTOLOGY_REGISTRY.len() {
        if DNS_ONTOLOGY_REGISTRY[i].type_code == code {
            return Some(&DNS_ONTOLOGY_REGISTRY[i]);
        }
        i += 1;
    }
    None
}

/// Look up a DNS record descriptor by its mnemonic name (case-insensitive).
pub fn lookup_by_name(name: &str) -> Option<&'static DnsRecordTypeDescriptor> {
    let mut i = 0;
    while i < DNS_ONTOLOGY_REGISTRY.len() {
        if DNS_ONTOLOGY_REGISTRY[i].type_name.eq_ignore_ascii_case(name) {
            return Some(&DNS_ONTOLOGY_REGISTRY[i]);
        }
        i += 1;
    }
    None
}

/// Look up descriptor from a typed `DnsType`.
pub fn lookup_by_dns_type(dtype: DnsType) -> Option<&'static DnsRecordTypeDescriptor> {
    lookup_by_code(u16::from(dtype))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ontology_registry_lookups() {
        let a_desc = lookup_by_code(1).unwrap();
        assert_eq!(a_desc.type_name, "A");
        assert_eq!(a_desc.category, DnsCategory::Address);
        assert!(a_desc.is_lossless_inline());

        let aaaa_desc = lookup_by_name("aaaa").unwrap();
        assert_eq!(aaaa_desc.type_code, 28);
        assert!(aaaa_desc.is_lossless_inline());

        let https_desc = lookup_by_dns_type(DnsType::HTTPS).unwrap();
        assert_eq!(https_desc.type_code, 65);
        assert_eq!(https_desc.storage_strategy, QualiaStorageStrategy::LosslessQuinChunkChain);

        assert!(lookup_by_code(9999).is_none());
        assert!(lookup_by_name("NONEXISTENT").is_none());
    }
}
