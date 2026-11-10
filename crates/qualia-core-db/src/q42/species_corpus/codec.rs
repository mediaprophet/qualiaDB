//! Native `.q42` encoding of a [`SpeciesCorpus`] and its CBOR-LD interchange projection.
//!
//! On disk the corpus is a unified Q42 volume: NQuins plus an embedded Q42LEX lexicon, the same
//! representation as [`crate::q42::model_helper`]. The root corpus node carries header fields;
//! every source, assertion and gap is its own node linked from the root (metadata = position).
//! Record fields become edges whose predicate IRI is the field path under the corpus context;
//! strings and exact numeric lexical forms resolve through the lexicon so decoding is lossless.
//! Non-negative numbers that fit also get an inline `xsd:decimal` mirror edge (`#decimal`) for
//! native numeric queries.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Number, Value};

use super::model::{CorpusConflict, SpeciesCorpus, SPECIES_CORPUS_CONTEXT};

/// A corpus written both as a native `.q42` volume and as its CBOR-LD projection.
#[cfg(not(target_arch = "wasm32"))]
pub struct SealedSpeciesCorpus {
    pub q42: Vec<u8>,
    pub cbor_ld: Vec<u8>,
    pub q42_sha256: String,
    pub conflicts: Vec<CorpusConflict>,
}
use crate::query::resolver::{INLINE_TAG_BOOLEAN, INLINE_TAG_DECIMAL, INLINE_TAG_MASK, INLINE_VALUE_MASK};
use crate::{q_hash, NQuin};

pub const MAX_CORPUS_Q42_BYTES: usize = 256 * 1024 * 1024;
const RECORD_KINDS: [&str; 3] = ["sources", "assertions", "gaps"];
const RDF_TYPE: &str = "rdf:type";
const TYPE_IRI: &str = "q42:QualiaSpeciesCorpus";

const LEAF_STRING: u64 = 0;
const LEAF_NUMBER: u64 = 1;
const LEAF_BOOL: u64 = 2;
const LEAF_NULL: u64 = 3;
const LEAF_DECIMAL_MIRROR: u64 = 4;
const LEAF_LINK: u64 = 5;
const LEAF_TYPE: u64 = 6;
const LEAF_EMPTY_OBJECT: u64 = 7;
const LEAF_EMPTY_ARRAY: u64 = 8;
/// Leaf type occupies metadata bits [0..3]; position index occupies bits [8..32].
const LEAF_MASK: u64 = 0xF;
const INDEX_SHIFT: u32 = 8;
const MAX_INDEX: usize = 0x00FF_FFFF;

fn node_id(iri: &str) -> u64 {
    q_hash(iri) & INLINE_VALUE_MASK
}

fn context_id() -> u64 {
    q_hash(SPECIES_CORPUS_CONTEXT) & 0x00FF_FFFF_FFFF_FFFF
}

fn predicate_iri(kind: &str, path: &str) -> String {
    format!("{SPECIES_CORPUS_CONTEXT}#{kind}/{path}")
}

struct Graph {
    lex: HashMap<u64, String>,
    quins: Vec<NQuin>,
    ctx: u64,
}

impl Graph {
    fn intern(&mut self, hash: u64, term: &str) -> Result<u64, String> {
        match self.lex.get(&hash) {
            Some(existing) if existing != term => Err(format!(
                "lexicon hash collision between '{existing}' and '{term}'"
            )),
            Some(_) => Ok(hash),
            None => {
                self.lex.insert(hash, term.to_string());
                Ok(hash)
            }
        }
    }

    fn edge(&mut self, s: u64, p_iri: &str, o: u64, meta: u64) -> Result<(), String> {
        let p = self.intern(q_hash(p_iri), p_iri)?;
        self.quins.push(NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: self.ctx,
            metadata: meta,
            parity: NQuin::calculate_parity(s, p, o, self.ctx, meta),
        });
        Ok(())
    }

    fn leaf(&mut self, s: u64, kind: &str, path: &str, v: &Value) -> Result<(), String> {
        let p = predicate_iri(kind, path);
        match v {
            Value::String(t) => {
                let o = self.intern(node_id(t), t)?;
                self.edge(s, &p, o, LEAF_STRING)
            }
            Value::Number(n) => {
                let text = n.to_string();
                let o = self.intern(node_id(&text), &text)?;
                self.edge(s, &p, o, LEAF_NUMBER)?;
                if let Some(f) = n.as_f64() {
                    let scaled = f * 1_000_000.0;
                    if f.is_finite() && f >= 0.0 && scaled <= INLINE_VALUE_MASK as f64 {
                        let o = INLINE_TAG_DECIMAL | (scaled.round() as u64 & INLINE_VALUE_MASK);
                        self.edge(s, &format!("{p}#decimal"), o, LEAF_DECIMAL_MIRROR)?;
                    }
                }
                Ok(())
            }
            Value::Bool(b) => self.edge(s, &p, INLINE_TAG_BOOLEAN | u64::from(*b), LEAF_BOOL),
            Value::Null => self.edge(s, &p, 0, LEAF_NULL),
            _ => Err(format!("non-leaf value at '{path}'")),
        }
    }

    fn marker(&mut self, s: u64, kind: &str, path: &str, leaf_type: u64) -> Result<(), String> {
        if path.is_empty() {
            return Err("empty container has no field path".into());
        }
        self.edge(s, &predicate_iri(kind, path), 0, leaf_type)
    }

    fn walk(&mut self, s: u64, kind: &str, path: &str, v: &Value) -> Result<(), String> {
        match v {
            Value::Object(m) if m.is_empty() => self.marker(s, kind, path, LEAF_EMPTY_OBJECT),
            Value::Array(items) if items.is_empty() => self.marker(s, kind, path, LEAF_EMPTY_ARRAY),
            Value::Object(m) => {
                for (k, child) in m {
                    if k.contains(['/', '[', ']', '#']) {
                        return Err(format!("field name '{k}' contains a reserved character"));
                    }
                    let p = if path.is_empty() { k.clone() } else { format!("{path}/{k}") };
                    self.walk(s, kind, &p, child)?;
                }
                Ok(())
            }
            Value::Array(items) => {
                for (i, child) in items.iter().enumerate() {
                    self.walk(s, kind, &format!("{path}[{i}]"), child)?;
                }
                Ok(())
            }
            leaf => self.leaf(s, kind, path, leaf),
        }
    }
}

/// Lowercase SHA-256 hex of volume bytes. This is the digest a species spec cites.
pub fn volume_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

impl SpeciesCorpus {
    /// Lexicon and quins for the native `.q42` representation, quins sorted by object.
    pub fn to_q42_graph(&self) -> Result<(HashMap<u64, String>, Vec<NQuin>), String> {
        self.validate()?;
        let doc = serde_json::to_value(self).map_err(|e| format!("corpus serialization: {e}"))?;
        let Value::Object(mut root) = doc else {
            return Err("corpus did not serialize to a record".into());
        };
        let mut g = Graph {
            lex: HashMap::new(),
            quins: Vec::new(),
            ctx: context_id(),
        };
        g.intern(context_id(), SPECIES_CORPUS_CONTEXT)?;
        let root_id = g.intern(node_id(&self.corpus_iri), &self.corpus_iri)?;
        let type_obj = g.intern(node_id(TYPE_IRI), TYPE_IRI)?;
        g.edge(root_id, RDF_TYPE, type_obj, LEAF_TYPE)?;

        for kind in RECORD_KINDS {
            let Some(Value::Array(items)) = root.remove(kind) else {
                continue;
            };
            if items.len() > MAX_INDEX {
                return Err(format!("too many {kind} records"));
            }
            for (i, item) in items.iter().enumerate() {
                let iri = format!("{}#{kind}/{i}", self.corpus_iri);
                let node = g.intern(node_id(&iri), &iri)?;
                g.edge(
                    root_id,
                    &predicate_iri("corpus", kind),
                    node,
                    LEAF_LINK | ((i as u64) << INDEX_SHIFT),
                )?;
                g.walk(node, kind, "", item)?;
            }
        }
        g.walk(root_id, "corpus", "", &Value::Object(root))?;
        g.quins.sort_unstable_by_key(|q| (q.object, q.subject, q.predicate));
        Ok((g.lex, g.quins))
    }

    /// Encodes the corpus as a unified `.q42` volume.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn to_q42_bytes(&self) -> Result<Vec<u8>, String> {
        use crate::q42_volume::UnifiedVolumeBuilder;
        use crate::QUINS_PER_BLOCK;
        let (lex, quins) = self.to_q42_graph()?;
        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex)
            .map_err(|e| format!("species corpus Q42LEX: {e:?}"))?;
        for (seq, chunk) in quins.chunks(QUINS_PER_BLOCK).enumerate() {
            builder
                .push_block(seq as u64, chunk)
                .map_err(|e| format!("species corpus block {seq}: {e}"))?;
        }
        let bytes = builder.finish_to_bytes();
        if bytes.len() > MAX_CORPUS_Q42_BYTES {
            return Err(format!("species corpus exceeds {MAX_CORPUS_Q42_BYTES} bytes"));
        }
        Ok(bytes)
    }

    /// Decodes and validates a corpus from unified `.q42` volume bytes.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_q42_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_CORPUS_Q42_BYTES {
            return Err(format!("species corpus exceeds {MAX_CORPUS_Q42_BYTES} bytes"));
        }
        let dir = tempfile::TempDir::new().map_err(|e| format!("species corpus scratch: {e}"))?;
        let path = dir.path().join("corpus.q42");
        std::fs::write(&path, bytes).map_err(|e| format!("species corpus scratch write: {e}"))?;
        Self::from_q42_path(&path)
    }

    /// Opens, verifies and decodes a corpus `.q42` volume.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_q42_path(path: &std::path::Path) -> Result<Self, String> {
        use crate::q42_volume::Q42Volume;
        let volume = Q42Volume::open(path).map_err(|e| format!("open species corpus: {e}"))?;
        volume
            .header()
            .verify_version()
            .map_err(|e| format!("invalid species corpus volume: {e}"))?;
        volume
            .verify_all_blocks()
            .map_err(|e| format!("species corpus block verification: {e}"))?;
        let lex = volume
            .lex_view()
            .map_err(|e| format!("invalid species corpus Q42LEX: {e:?}"))?;
        let quins = volume
            .read_all_quins()
            .map_err(|e| format!("read species corpus: {e}"))?;
        let resolve = |h: u64| -> Result<String, String> {
            let mut t = String::new();
            if lex.resolve_term_into(h, &mut t) {
                Ok(t)
            } else {
                Err(format!("species corpus has unresolved lexicon hash {h:#018x}"))
            }
        };
        decode_graph(&quins, resolve)
    }

    /// Native `.q42` bytes, the CBOR-LD projection, the volume digest, and derived conflicts.
    /// Conflicts stay in the volume as the disagreeing assertions; this also lists them.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn seal(&self) -> Result<SealedSpeciesCorpus, String> {
        self.validate()?;
        let q42 = self.to_q42_bytes()?;
        let cbor_ld = self.to_cbor_ld()?;
        Ok(SealedSpeciesCorpus {
            q42_sha256: volume_sha256(&q42),
            q42,
            cbor_ld,
            conflicts: self.conflicts(),
        })
    }

    /// CBOR-LD interchange projection (self-describe tag 55799).
    pub fn to_cbor_ld(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let mut out = vec![0xd9, 0xd9, 0xf7];
        ciborium::into_writer(self, &mut out).map_err(|e| format!("cbor encode: {e}"))?;
        Ok(out)
    }

    /// Decodes and validates the CBOR-LD interchange projection.
    pub fn from_cbor_ld(bytes: &[u8]) -> Result<Self, String> {
        let body = bytes.strip_prefix(&[0xd9, 0xd9, 0xf7]).unwrap_or(bytes);
        let corpus: Self = ciborium::from_reader(body).map_err(|e| format!("cbor decode: {e}"))?;
        corpus.validate()?;
        Ok(corpus)
    }
}

enum Seg {
    Key(String),
    Index(usize),
}

fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
    let mut out = Vec::new();
    for part in path.split('/') {
        let (key, rest) = part.split_at(part.find('[').unwrap_or(part.len()));
        if key.is_empty() {
            return Err(format!("malformed field path '{path}'"));
        }
        out.push(Seg::Key(key.to_string()));
        let mut rest = rest;
        while let Some(r) = rest.strip_prefix('[') {
            let end = r.find(']').ok_or_else(|| format!("malformed field path '{path}'"))?;
            let i: usize = r[..end]
                .parse()
                .map_err(|_| format!("malformed index in '{path}'"))?;
            if i > MAX_INDEX {
                return Err(format!("index too large in '{path}'"));
            }
            out.push(Seg::Index(i));
            rest = &r[end + 1..];
        }
        if !rest.is_empty() {
            return Err(format!("malformed field path '{path}'"));
        }
    }
    Ok(out)
}

/// Sparse arrays are filled with `Null` during insertion and checked afterwards.
fn insert(root: &mut Value, segs: &[Seg], leaf: Value) -> Result<(), String> {
    let mut cur = root;
    for (n, seg) in segs.iter().enumerate() {
        let last = n + 1 == segs.len();
        let next_is_index = matches!(segs.get(n + 1), Some(Seg::Index(_)));
        let fresh = || if next_is_index { Value::Array(Vec::new()) } else { Value::Object(Map::new()) };
        cur = match seg {
            Seg::Key(k) => {
                let m = cur.as_object_mut().ok_or("field path conflicts with a non-record value")?;
                if last {
                    if m.insert(k.clone(), leaf).is_some() {
                        return Err(format!("duplicate field '{k}'"));
                    }
                    return Ok(());
                }
                m.entry(k.clone()).or_insert_with(fresh)
            }
            Seg::Index(i) => {
                let a = cur.as_array_mut().ok_or("field path conflicts with a non-list value")?;
                if a.len() <= *i {
                    a.resize(*i + 1, Value::Null);
                }
                if last {
                    a[*i] = leaf;
                    return Ok(());
                }
                if a[*i].is_null() {
                    a[*i] = fresh();
                }
                &mut a[*i]
            }
        };
    }
    Err("empty field path".into())
}

fn decode_graph(
    quins: &[NQuin],
    resolve: impl Fn(u64) -> Result<String, String>,
) -> Result<SpeciesCorpus, String> {
    let ctx = context_id();
    let mut root_id = None;
    for q in quins {
        if q.parity != NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata) {
            return Err("species corpus quin parity mismatch".into());
        }
        if q.context != ctx {
            return Err("species corpus quin has a foreign context".into());
        }
        if q.metadata & LEAF_MASK == LEAF_TYPE {
            if resolve(q.object)? != TYPE_IRI || root_id.replace(q.subject).is_some() {
                return Err("species corpus must have exactly one corpus root".into());
            }
        }
    }
    let root_id = root_id.ok_or("Q42 volume is not a species corpus")?;

    let mut by_subject: BTreeMap<u64, Vec<&NQuin>> = BTreeMap::new();
    for q in quins {
        by_subject.entry(q.subject).or_default().push(q);
    }

    let mut doc = Value::Object(Map::new());
    let mut records: BTreeMap<&str, BTreeMap<u64, u64>> = BTreeMap::new();
    let corpus_prefix = format!("{SPECIES_CORPUS_CONTEXT}#corpus/");
    for q in by_subject.get(&root_id).map(Vec::as_slice).unwrap_or(&[]) {
        if q.metadata & LEAF_MASK == LEAF_LINK {
            let p = resolve(q.predicate)?;
            let kind = p
                .strip_prefix(&corpus_prefix)
                .and_then(|k| RECORD_KINDS.iter().find(|r| **r == k))
                .ok_or_else(|| format!("unknown corpus record link '{p}'"))?;
            let index = (q.metadata >> INDEX_SHIFT) & MAX_INDEX as u64;
            if records.entry(kind).or_default().insert(index, q.object).is_some() {
                return Err(format!("duplicate {kind} position {index}"));
            }
        }
    }
    decode_fields(&mut doc, "corpus", by_subject.get(&root_id), &resolve)?;

    for kind in RECORD_KINDS {
        let mut items = Vec::new();
        for (expected, (index, node)) in records.get(kind).into_iter().flatten().enumerate() {
            if *index != expected as u64 {
                return Err(format!("{kind} positions are not contiguous"));
            }
            let mut item = Value::Object(Map::new());
            decode_fields(&mut item, kind, by_subject.get(node), &resolve)?;
            items.push(item);
        }
        doc.as_object_mut().unwrap().insert(kind.to_string(), Value::Array(items));
    }
    if contains_null_hole(&doc) {
        return Err("species corpus has a missing list element".into());
    }
    let corpus: SpeciesCorpus =
        serde_json::from_value(doc).map_err(|e| format!("species corpus decode: {e}"))?;
    corpus.validate()?;
    Ok(corpus)
}

fn decode_fields(
    target: &mut Value,
    kind: &str,
    quins: Option<&Vec<&NQuin>>,
    resolve: &impl Fn(u64) -> Result<String, String>,
) -> Result<(), String> {
    let prefix = format!("{SPECIES_CORPUS_CONTEXT}#{kind}/");
    for q in quins.map(Vec::as_slice).unwrap_or(&[]) {
        let leaf_type = q.metadata & LEAF_MASK;
        if matches!(leaf_type, LEAF_DECIMAL_MIRROR | LEAF_LINK | LEAF_TYPE) {
            continue;
        }
        let p = resolve(q.predicate)?;
        let path = p
            .strip_prefix(&prefix)
            .ok_or_else(|| format!("field predicate '{p}' is outside {kind}"))?;
        let leaf = match leaf_type {
            LEAF_EMPTY_OBJECT => Value::Object(Map::new()),
            LEAF_EMPTY_ARRAY => Value::Array(Vec::new()),
            LEAF_STRING => Value::String(resolve(q.object)?),
            LEAF_NUMBER => {
                let text = resolve(q.object)?;
                Value::Number(
                    text.parse::<Number>()
                        .map_err(|_| format!("invalid number '{text}' at '{path}'"))?,
                )
            }
            LEAF_BOOL => {
                if q.object & INLINE_TAG_MASK != INLINE_TAG_BOOLEAN {
                    return Err(format!("expected boolean at '{path}'"));
                }
                Value::Bool(q.object & 1 != 0)
            }
            LEAF_NULL => Value::Null,
            other => return Err(format!("unknown leaf type {other} at '{path}'")),
        };
        insert(target, &parse_path(path)?, leaf)?;
    }
    Ok(())
}

fn contains_null_hole(v: &Value) -> bool {
    match v {
        Value::Array(a) => a.iter().any(|x| x.is_null() || contains_null_hole(x)),
        Value::Object(m) => m.values().any(contains_null_hole),
        _ => false,
    }
}
