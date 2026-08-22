//! Bounded vocab-chunk parser. Not a full N3 engine.

use std::collections::BTreeMap;

pub const MAX_CHUNK_BYTES: usize = 256 * 1024;
pub const MAX_TERMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabTerm {
    pub iri: String,
    pub label: Option<String>,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabChunk {
    pub prefixes: BTreeMap<String, String>,
    pub terms: Vec<VocabTerm>,
    pub content_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabError {
    TooLarge { bytes: usize },
    TooManyTerms { count: usize },
    Empty,
}

impl std::fmt::Display for VocabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { bytes } => {
                write!(f, "vocab chunk {bytes} bytes exceeds {MAX_CHUNK_BYTES}")
            }
            Self::TooManyTerms { count } => {
                write!(f, "vocab chunk {count} terms exceeds {MAX_TERMS}")
            }
            Self::Empty => write!(f, "vocab chunk has no terms"),
        }
    }
}

impl VocabChunk {
    pub fn prefix_iri(&self, prefix: &str) -> Option<&str> {
        self.prefixes.get(prefix).map(String::as_str)
    }

    pub fn expand(&self, prefixed: &str) -> Option<String> {
        let (p, local) = prefixed.split_once(':')?;
        let base = self.prefix_iri(p)?;
        Some(format!("{base}{local}"))
    }

    pub fn has_local(&self, prefix: &str, local: &str) -> bool {
        let Some(iri) = self.expand(&format!("{prefix}:{local}")) else {
            return false;
        };
        self.terms.iter().any(|t| t.iri == iri)
    }

    pub fn hash_hex(&self) -> String {
        self.content_hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// P17.5 lock check. Does not fetch latest.
    pub fn lock_matches(&self, expected_hex: &str) -> bool {
        expected_hex.eq_ignore_ascii_case(&self.hash_hex())
    }
}

pub fn parse_chunk(bytes: &[u8]) -> Result<VocabChunk, VocabError> {
    if bytes.len() > MAX_CHUNK_BYTES {
        return Err(VocabError::TooLarge { bytes: bytes.len() });
    }
    let src = std::str::from_utf8(bytes).unwrap_or("");
    let hash = content_hash(bytes);
    let mut prefixes = BTreeMap::new();
    let mut by_iri: BTreeMap<String, VocabTerm> = BTreeMap::new();

    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("@prefix ")
            .or_else(|| line.strip_prefix("prefix "))
        {
            if let Some((name, iri)) = parse_prefix(rest) {
                prefixes.insert(name, iri);
            }
            continue;
        }
        if let Some((s, p, o)) = parse_triple(line) {
            let s_iri = expand(&prefixes, &s);
            let p_iri = expand(&prefixes, &p);
            let entry = by_iri.entry(s_iri.clone()).or_insert_with(|| VocabTerm {
                iri: s_iri,
                label: None,
                parents: Vec::new(),
            });
            if p_iri.ends_with("label") || p == "rdfs:label" {
                entry.label = Some(unquote(&o));
            } else if p_iri.ends_with("subClassOf") || p == "rdfs:subClassOf" {
                entry.parents.push(expand(&prefixes, &o));
            }
        }
    }

    if by_iri.len() > MAX_TERMS {
        return Err(VocabError::TooManyTerms { count: by_iri.len() });
    }
    let terms: Vec<VocabTerm> = by_iri.into_values().collect();
    if terms.is_empty() {
        return Err(VocabError::Empty);
    }
    Ok(VocabChunk {
        prefixes,
        terms,
        content_hash: hash,
    })
}

fn parse_prefix(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim().trim_end_matches('.').trim();
    let (name, iri_part) = rest.split_once(':')?;
    let iri = iri_part
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_string();
    if name.trim().is_empty() || iri.is_empty() {
        return None;
    }
    Some((name.trim().to_string(), iri))
}

fn parse_triple(line: &str) -> Option<(String, String, String)> {
    let line = line.trim().trim_end_matches('.').trim();
    let mut parts = Vec::new();
    if line.starts_with('"') {
        return None;
    }
    let mut rest = line;
    for _ in 0..3 {
        rest = rest.trim();
        if rest.is_empty() {
            return None;
        }
        if rest.starts_with('"') {
            let end = rest[1..].find('"')? + 1;
            parts.push(rest[..=end].to_string());
            rest = &rest[end + 1..];
        } else if rest.starts_with('<') {
            let end = rest.find('>')?;
            parts.push(rest[..=end].to_string());
            rest = &rest[end + 1..];
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            parts.push(rest[..end].to_string());
            rest = &rest[end..];
        }
    }
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0].clone(), parts[1].clone(), parts[2].clone()))
}

fn expand(prefixes: &BTreeMap<String, String>, tok: &str) -> String {
    if let Some(inner) = tok.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        return inner.to_string();
    }
    if tok.starts_with('"') {
        return unquote(tok);
    }
    if let Some((p, local)) = tok.split_once(':') {
        if let Some(base) = prefixes.get(p) {
            return format!("{base}{local}");
        }
    }
    tok.to_string()
}

fn unquote(tok: &str) -> String {
    tok.trim()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string()
}

/// Deterministic 32-byte identity (four FNV-1a 64 streams). Not SHA-256.
pub fn content_hash(bytes: &[u8]) -> [u8; 32] {
    const OFFSETS: [u64; 4] = [
        0xcbf29ce484222325,
        0x6c62272e07bb0142,
        0x9e3779b97f4a7c15,
        0x243f6a8885a308d3,
    ];
    let mut out = [0u8; 32];
    for (lane, seed) in OFFSETS.iter().enumerate() {
        let mut h = *seed;
        for (i, b) in bytes.iter().enumerate() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
            h ^= (i as u64).rotate_left((lane as u32 * 7) % 63);
        }
        out[lane * 8..(lane + 1) * 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}
