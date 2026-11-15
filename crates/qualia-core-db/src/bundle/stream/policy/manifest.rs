//! Exact canonical manifest framing; hashes and semantic graph owners are reused.
use super::types::MAGIC;
use super::{MAX_MANIFEST_BYTES, ManifestFields, PolicyError as E};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use crate::semantic_instruments::resolve::{
    DependencyKind,
    bounded_closure::{ClosureCandidate, MAX_CLOSURE_ORIGINAL_BYTES},
};
const ROW: usize = 57;
const HEADER: usize = MAGIC.len() + 48 * 5 + 8 + 4;
#[derive(Clone, Copy, Debug)]
pub struct ManifestView<'a> {
    encoded: &'a [u8],
    fields: ManifestFields,
    rows: &'a [u8],
    edges: &'a [u8],
}
impl<'a> ManifestView<'a> {
    pub const fn fields(&self) -> ManifestFields {
        self.fields
    }
    pub const fn encoded_bytes(&self) -> &'a [u8] {
        self.encoded
    }
    pub fn digest(&self) -> StrongDigest {
        sha384(self.encoded)
    }
    fn row(&self, index: usize) -> (StrongDigest, u8, u64) {
        let row = &self.rows[index * ROW..(index + 1) * ROW];
        (
            digest(&row[..48]),
            row[48],
            u64::from_be_bytes(row[49..57].try_into().unwrap()),
        )
    }
    pub(super) fn matches(&self, candidate: &ClosureCandidate<'_, '_>) -> Result<(), E> {
        check_fields(self.fields, candidate)?;
        if self.rows.len() / ROW != candidate.nodes().len()
            || self.edges.len() / 2 != candidate.edges().len()
            || candidate
                .original_byte_count()
                .checked_add(self.encoded.len())
                .filter(|n| *n <= MAX_CLOSURE_ORIGINAL_BYTES)
                .is_none()
        {
            return Err(E::Limit);
        }
        for i in 0..self.rows.len() / ROW {
            let (id, kind, length) = self.row(i);
            let node = candidate
                .nodes()
                .iter()
                .find(|n| n.digest == id)
                .ok_or(E::Graph)?;
            if kind != kind_tag(node.kind) || length != node.original_bytes.len() as u64 {
                return Err(E::Graph);
            }
        }
        for pair in self.edges.chunks_exact(2) {
            let from = self.row(pair[0] as usize).0;
            let to = self.row(pair[1] as usize).0;
            if !candidate
                .edges()
                .iter()
                .any(|e| e.dependent == from && e.dependency == to)
            {
                return Err(E::Graph);
            }
        }
        Ok(())
    }
}
fn digest(bytes: &[u8]) -> StrongDigest {
    StrongDigest(bytes.try_into().expect("validated fixed-width digest"))
}
fn kind_tag(kind: DependencyKind) -> u8 {
    match kind {
        DependencyKind::Ontology => 1,
        DependencyKind::Lexicon => 2,
        DependencyKind::Dataset => 3,
        DependencyKind::LawPackage => 4,
        DependencyKind::Instrument => 5,
        DependencyKind::NativeModule => 6,
    }
}
fn fields_valid(f: ManifestFields) -> Result<(), E> {
    if f.source_length < 64
        || f.source_length > (1u64 << 40)
        || [f.source, f.semantic_root, f.policy, f.consumer, f.issuer]
            .iter()
            .any(|d| d.is_zero())
    {
        return Err(E::Reference);
    }
    Ok(())
}
fn check_fields(f: ManifestFields, c: &ClosureCandidate<'_, '_>) -> Result<(), E> {
    fields_valid(f)?;
    if f.semantic_root != c.root_digest()
        || !c.nodes().iter().any(|n| n.digest == f.policy)
        || !c.nodes().iter().any(|n| n.digest == f.consumer)
    {
        return Err(E::Reference);
    }
    Ok(())
}
pub fn decode_manifest(bytes: &[u8]) -> Result<ManifestView<'_>, E> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(E::Limit);
    }
    if bytes.len() < HEADER || !bytes.starts_with(MAGIC) {
        return Err(E::Encoding);
    }
    let mut pos = MAGIC.len();
    let source = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let source_length = u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    pos += 8;
    let semantic_root = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let policy = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let consumer = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let issuer = digest(&bytes[pos..pos + 48]);
    pos += 48;
    if bytes[pos] != 1 {
        return Err(E::Encoding);
    }
    pos += 1;
    let nodes = bytes[pos] as usize;
    pos += 1;
    let edges = u16::from_be_bytes(bytes[pos..pos + 2].try_into().unwrap()) as usize;
    pos += 2;
    if !(1..=16).contains(&nodes) || edges > 256 {
        return Err(E::Limit);
    }
    if bytes.len() != pos + nodes * ROW + edges * 2 {
        return Err(E::Encoding);
    }
    let fields = ManifestFields {
        source,
        source_length,
        semantic_root,
        policy,
        consumer,
        issuer,
    };
    fields_valid(fields)?;
    let view = ManifestView {
        encoded: bytes,
        fields,
        rows: &bytes[pos..pos + nodes * ROW],
        edges: &bytes[pos + nodes * ROW..],
    };
    let mut total = bytes.len() as u64;
    for i in 0..nodes {
        let (id, kind, length) = view.row(i);
        if id.is_zero() || !(1..=6).contains(&kind) || (i != 0 && view.row(i - 1).0.0 >= id.0) {
            return Err(E::Encoding);
        }
        total = total.checked_add(length).ok_or(E::Limit)?;
        if total > MAX_CLOSURE_ORIGINAL_BYTES as u64 {
            return Err(E::Limit);
        }
    }
    for reference in [semantic_root, policy, consumer] {
        if !(0..nodes).any(|i| view.row(i).0 == reference) {
            return Err(E::Reference);
        }
    }
    let mut previous = None;
    for pair in view.edges.chunks_exact(2) {
        let key = (pair[0], pair[1]);
        if pair[0] as usize >= nodes
            || pair[1] as usize >= nodes
            || pair[0] == pair[1]
            || previous.is_some_and(|p| p >= key)
        {
            return Err(E::Encoding);
        }
        previous = Some(key);
    }
    Ok(view)
}
/// Canonicalize an already structurally checked candidate into caller storage.
pub fn encode_manifest_into(
    fields: ManifestFields,
    candidate: &ClosureCandidate<'_, '_>,
    out: &mut [u8],
) -> Result<usize, E> {
    check_fields(fields, candidate)?;
    let nodes = candidate.nodes();
    let length = HEADER + nodes.len() * ROW + candidate.edges().len() * 2;
    if length > MAX_MANIFEST_BYTES
        || candidate
            .original_byte_count()
            .checked_add(length)
            .filter(|n| *n <= MAX_CLOSURE_ORIGINAL_BYTES)
            .is_none()
    {
        return Err(E::Limit);
    }
    let out = out.get_mut(..length).ok_or(E::Capacity)?;
    let mut order = [0usize; 16];
    for i in 0..nodes.len() {
        order[i] = i;
    }
    // Fixed insertion sort: bounded, deterministic, allocation-free and nonrecursive.
    for i in 1..nodes.len() {
        let value = order[i];
        let mut j = i;
        while j != 0 && nodes[order[j - 1]].digest.0 > nodes[value].digest.0 {
            order[j] = order[j - 1];
            j -= 1;
        }
        order[j] = value;
    }
    let mut pos = 0;
    fn put(out: &mut [u8], pos: &mut usize, bytes: &[u8]) {
        out[*pos..*pos + bytes.len()].copy_from_slice(bytes);
        *pos += bytes.len();
    }
    put(out, &mut pos, MAGIC);
    put(out, &mut pos, &fields.source.0);
    put(out, &mut pos, &fields.source_length.to_be_bytes());
    for id in [
        fields.semantic_root,
        fields.policy,
        fields.consumer,
        fields.issuer,
    ] {
        put(out, &mut pos, &id.0);
    }
    put(out, &mut pos, &[1, nodes.len() as u8]);
    put(
        out,
        &mut pos,
        &(candidate.edges().len() as u16).to_be_bytes(),
    );
    for i in &order[..nodes.len()] {
        let node = nodes[*i];
        put(out, &mut pos, &node.digest.0);
        put(out, &mut pos, &[kind_tag(node.kind)]);
        put(
            out,
            &mut pos,
            &(node.original_bytes.len() as u64).to_be_bytes(),
        );
    }
    for (from, i) in order[..nodes.len()].iter().enumerate() {
        for (to, j) in order[..nodes.len()].iter().enumerate() {
            if candidate
                .edges()
                .iter()
                .any(|e| e.dependent == nodes[*i].digest && e.dependency == nodes[*j].digest)
            {
                put(out, &mut pos, &[from as u8, to as u8]);
            }
        }
    }
    debug_assert_eq!(pos, length);
    Ok(length)
}

mod transport;
pub use transport::{ManifestArtifact,ManifestDependency};
