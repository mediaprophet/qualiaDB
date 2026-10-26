//! W3C CBOR-LD Tag Constants & Static/Dynamic Ontology Term Compression.

pub const TAG_CBOR_LD_UNCOMPRESSED: u64 = 0x0500;
pub const TAG_CBOR_LD_COMPRESSED: u64 = 0x0501;
pub const TAG_VIBE_QUANTITY: u64 = 4201;
pub const TAG_VIBE_INSTANT: u64 = 4202;
pub const TAG_VIBE_DURATION: u64 = 4203;
pub const TAG_VIBE_QUIN_REF: u64 = 4204;
pub const TAG_VIBE_DISCLOSURE_DENIED: u64 = 4205;
pub const TAG_VIBE_IRI: u64 = 32;

pub(crate) const TAG_VIBE_U64: u64 = 4206;
pub(crate) const TAG_VIBE_PREFIXED: u64 = 4207;
pub(crate) const TAG_VIBE_RECEIPT: u64 = 4208;
pub(crate) const TAG_VIBE_OK: u64 = 4209;
pub(crate) const TAG_VIBE_ERR: u64 = 4210;
pub(crate) const TAG_VIBE_BLANK: u64 = 4211;
pub(crate) const TAG_VIBE_VAR: u64 = 4212;

pub const DEFAULT_CONTEXT_IRI: &str = "https://qualia.world/context/ui/v1";
pub const DYNAMIC_TERM_BIT: u64 = 1u64 << 59;
pub const TOKEN_60BIT_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

/// Built-in static term table for Qualia/Vibe/UI/Form/QUDT/SHACL/RDF terms (`1..=1023`).
pub const STATIC_TERMS: &[(u64, &str)] = &[
    (1, "@context"),
    (2, "@id"),
    (3, "@type"),
    (4, "@value"),
    (5, "@graph"),
    (6, "@provider_host"),
    (7, "@provider_tld"),
    (10, "https://qualia.world/context/ui/v1"),
    (11, "https://qualia.world/context/clinical/v1"),
    (12, "https://qualia.world/context/vibe/v1"),
    (20, "ui:patch"),
    (21, "ui:target_id"),
    (22, "ui:property"),
    (23, "ui:value"),
    (24, "ui:items"),
    (25, "ui:kind"),
    (26, "ui:text"),
    (27, "ui:disabled"),
    (28, "ui:icon"),
    (29, "ui:color"),
    (30, "form:id"),
    (31, "form:fields"),
    (32, "form:valid"),
    (33, "form:violations"),
    (40, "qudt:KiloPascal"),
    (41, "qudt:Pascal"),
    (42, "qudt:Bar"),
    (43, "qudt:Meter"),
    (44, "qudt:Second"),
    (45, "qudt:Kilogram"),
    (46, "qudt:DimensionlessUnit"),
    (47, "qudt:Kelvin"),
    (48, "qudt:Mole"),
    (50, "sh:NodeShape"),
    (51, "sh:PropertyShape"),
    (52, "sh:path"),
    (53, "sh:minCount"),
    (54, "sh:maxCount"),
    (55, "sh:datatype"),
    (60, "rdf:type"),
    (61, "rdfs:label"),
    (62, "xsd:string"),
    (63, "xsd:integer"),
    (64, "xsd:double"),
    (65, "xsd:boolean"),
];

/// Looks up a compact static term ID (`1..=1023`) for a known ontology term or IRI.
pub fn lookup_static_term_id(iri: &str) -> Option<u64> {
    for &(id, term) in STATIC_TERMS {
        if term == iri {
            return Some(id);
        }
    }
    None
}

/// Looks up the static IRI or term string for a compact static term ID (`1..=1023`).
pub fn lookup_static_term_iri(term_id: u64) -> Option<&'static str> {
    for &(id, term) in STATIC_TERMS {
        if id == term_id {
            return Some(term);
        }
    }
    None
}

/// Compresses an IRI or property term into either a compact static table ID (`1..=1023`)
/// or a deterministic 60-bit FNV-1a token with bit 59 set (`DYNAMIC_TERM_BIT`).
pub fn compress_iri(iri: &str, context_iri: &str) -> u64 {
    if let Some(id) = lookup_static_term_id(iri) {
        return id;
    }
    if let Some(hex_part) = iri.strip_prefix("did:q42:term:") {
        if let Ok(parsed) = u64::from_str_radix(hex_part, 16) {
            if (parsed & DYNAMIC_TERM_BIT) != 0 && (parsed & !TOKEN_60BIT_MASK) == 0 {
                return parsed;
            }
        }
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in context_iri.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash ^= b'|' as u64;
    hash = hash.wrapping_mul(0x100000001b3);
    for &b in iri.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash & TOKEN_60BIT_MASK) | DYNAMIC_TERM_BIT
}

/// Expands a compressed term ID back to a static IRI/term or a deterministic
/// `did:q42:term:<hex>` topological identifier for dynamic 60-bit tokens.
pub fn expand_term(term_id: u64, _context_iri: &str) -> Option<String> {
    if let Some(iri) = lookup_static_term_iri(term_id) {
        return Some(iri.to_string());
    }
    if (term_id & DYNAMIC_TERM_BIT) != 0 && (term_id & !TOKEN_60BIT_MASK) == 0 {
        return Some(format!("did:q42:term:{term_id:015x}"));
    }
    None
}
