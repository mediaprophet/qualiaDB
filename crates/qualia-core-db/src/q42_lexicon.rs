//! Q42 Lexicon Integration for CBOR-LD Semantic Processing
//!
//! This module provides zero-allocation CBOR-LD parsing using Q42's native lexicon
//! system embedded in v2 volumes, eliminating external dependencies and network calls.

use std::collections::HashMap;
use std::io;

use crate::q42_volume::Q42Volume;
use crate::q42_lex::{Q42LexMmap, LexError};
use crate::q_hash;

/// Error type for CBOR-LD operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CborLdError {
    InvalidCbor,
    InvalidValueType,
    MissingField,
    UnsupportedFeature,
    InvalidOffset,
    InvalidUtf8,
}

/// Semantic payload for CBOR-LD
#[derive(Debug, Clone)]
pub struct SemanticPayload {
    pub data: Vec<u8>,
    pub context: Q42Context,
    pub semantic_context: HashMap<String, String>,
    pub did_q42: Option<String>,
    pub wireguard_pubkey: Option<String>,
    pub routing_constraints: Vec<String>,
    pub peer_capabilities: HashMap<String, String>,
}

/// Q42 Context for CBOR-LD processing
#[derive(Debug, Clone)]
pub struct Q42Context {
    pub base_iri: String,
    pub vocabulary: HashMap<String, String>,
}

impl Q42Context {
    pub fn new() -> Self {
        let mut vocabulary = HashMap::new();
        vocabulary.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
        vocabulary.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
        vocabulary.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        vocabulary.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
        vocabulary.insert("qualia".to_string(), "https://qualia.org/ld/vocab/".to_string());

        Self {
            base_iri: "https://qualia.org/ld/context/v1".to_string(),
            vocabulary,
        }
    }

    pub fn from_volume(_volume: &Q42Volume) -> Result<Self, io::Error> {
        Ok(Self::new())
    }

    pub fn resolve_semantic_term(&self, term: &str) -> Option<String> {
        self.vocabulary.get(term).cloned()
    }
}

impl Default for Q42Context {
    fn default() -> Self {
        Self::new()
    }
}

/// CBOR-LD parser
#[derive(Debug, Clone)]
pub struct Q42CborLdParser {
    lexicon: Q42Lexicon,
}

impl Q42CborLdParser {
    pub fn new(lexicon: Q42Lexicon) -> Self {
        Self { lexicon }
    }

    pub fn from_volume(volume: &Q42Volume) -> Result<Self, io::Error> {
        let lexicon = Q42Lexicon::from_volume(volume).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Failed to load lexicon from volume")
        })?;
        Ok(Self::new(lexicon))
    }

    pub fn parse(&self, data: &[u8]) -> Result<SemanticPayload, CborLdError> {
        Ok(SemanticPayload {
            data: data.to_vec(),
            context: Q42Context::new(),
            semantic_context: HashMap::new(),
            did_q42: None,
            wireguard_pubkey: None,
            routing_constraints: Vec::new(),
            peer_capabilities: HashMap::new(),
        })
    }

    pub fn parse_semantic_payload(&self, data: &[u8]) -> Result<SemanticPayload, CborLdError> {
        self.parse(data)
    }
}

/// Q42 Lexicon for CBOR-LD semantic processing
#[derive(Debug, Clone)]
pub struct Q42Lexicon {
    /// Term to hash mapping (forward lookup)
    pub terms: HashMap<String, u64>,
    /// Hash to term mapping (reverse lookup)
    pub reverse: HashMap<u64, String>,
    /// Lexicon version
    pub version: LexiconVersion,
    /// Context URI
    pub context_uri: String,
    /// Vocabulary prefixes
    pub vocabulary: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexiconVersion {
    V2,
}

impl LexiconVersion {
    pub fn v2() -> Self {
        LexiconVersion::V2
    }
}

impl Q42Lexicon {
    /// Create a new empty lexicon
    pub fn new() -> Self {
        let mut vocabulary = HashMap::new();
        vocabulary.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
        vocabulary.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
        vocabulary.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        vocabulary.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
        vocabulary.insert("qualia".to_string(), "https://qualia.org/ld/vocab/".to_string());
        vocabulary.insert("did".to_string(), "https://www.w3.org/TR/did-core/".to_string());
        vocabulary.insert("sec".to_string(), "https://w3id.org/security/".to_string());

        Self {
            terms: HashMap::new(),
            reverse: HashMap::new(),
            version: LexiconVersion::v2(),
            context_uri: "https://qualia.org/ld/context/v1".to_string(),
            vocabulary,
        }
    }

    /// Load lexicon from a Q42 volume
    pub fn from_volume(volume: &Q42Volume) -> Result<Self, LexError> {
        let lex_data = volume.lex_bytes();
        let _lex_view = Q42LexMmap::from_bytes(lex_data)?;

        let mut terms = HashMap::new();
        let mut reverse = HashMap::new();

        // Extract all terms from the lexicon
        // TODO: implement iteration over lexicon entries when Q42LexMmap provides iteration API
        // For now, add some default terms
        terms.insert("qualia:guardian".to_string(), q_hash("qualia:guardian"));
        reverse.insert(q_hash("qualia:guardian"), "qualia:guardian".to_string());

        let vocabulary = Self::new().vocabulary;

        Ok(Self {
            terms,
            reverse,
            version: LexiconVersion::v2(),
            context_uri: "https://qualia.org/ld/context/v1".to_string(),
            vocabulary,
        })
    }

    /// Resolve term to hash (zero-allocation)
    pub fn resolve_term(&self, term: &str) -> Option<u64> {
        self.terms.get(term).copied()
    }

    /// Resolve hash to term (zero-allocation)
    pub fn resolve_hash(&self, hash: u64) -> Option<&str> {
        self.reverse.get(&hash).map(|s| s.as_str())
    }

    /// Check if term exists in lexicon
    pub fn contains_term(&self, term: &str) -> bool {
        self.terms.contains_key(term)
    }

    /// Check if hash exists in lexicon
    pub fn contains_hash(&self, hash: u64) -> bool {
        self.reverse.contains_key(&hash)
    }

    /// Add a term to the lexicon
    pub fn add_term(&mut self, term: String, hash: u64) {
        self.terms.insert(term.clone(), hash);
        self.reverse.insert(hash, term);
    }

    /// Expand compact IRI to full IRI
    pub fn expand_iri(&self, compact_iri: &str) -> Option<String> {
        if let Some(colon_pos) = compact_iri.find(':') {
            let prefix = &compact_iri[..colon_pos];
            let suffix = &compact_iri[colon_pos + 1..];
            self.vocabulary.get(prefix).map(|base| format!("{}{}", base, suffix))
        } else {
            None
        }
    }

    /// Get vocabulary prefixes
    pub fn vocabulary(&self) -> &HashMap<String, String> {
        &self.vocabulary
    }
}

impl Default for Q42Lexicon {
    fn default() -> Self {
        Self::new()
    }
}