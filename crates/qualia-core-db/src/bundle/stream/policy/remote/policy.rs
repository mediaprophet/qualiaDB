//! Versioned whole-source disclosure policy artifact counted in the closure budget.
use super::PolicyError as E;
use crate::crypto::network::digest::sha384;
use crate::crypto::network::object_stream::MAX_OBJECT_BYTES;
use crate::net::qdnf::types::{OperationId, StrongDigest};
use crate::semantic_instruments::resolve::bounded_closure::{
    ClosureCandidate, MAX_CLOSURE_EDGES, MAX_CLOSURE_NODES, MAX_CLOSURE_ORIGINAL_BYTES,
};

const MAGIC: &[u8] = b"QBDL-WSD-POL-V1\0";
const VERSION: u8 = 1;
const FIXED: usize = MAGIC.len() + 1 + 1 + 48 + 8 + 48 + 48 + 48 + 1 + 48 + 48 + 16 + 48 + 48 + 48;
pub const MAX_DISCLOSURE_POLICY_BYTES: usize = FIXED + 16 * 48;
pub const MAX_CLOSURE_LABELS: usize = 16;

/// Whole-source coverage bits. This profile admits only the complete set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCoverage(u8);

impl SourceCoverage {
    pub const HEADER: u8 = 1;
    pub const INDEX: u8 = 2;
    pub const GAPS: u8 = 4;
    pub const ENTRIES: u8 = 8;
    pub const WHOLE: Self = Self(Self::HEADER | Self::INDEX | Self::GAPS | Self::ENTRIES);

    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_whole(self) -> bool {
        self.0 == Self::WHOLE.0
    }

    fn admit(self) -> Result<Self, E> {
        if self.is_whole() {
            Ok(self)
        } else {
            Err(E::Graph)
        }
    }
}

/// Already-counted closure occupancy. Policy bytes/nodes charge this budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureBudget {
    pub original_bytes: usize,
    pub nodes: usize,
    pub edges: usize,
}

impl ClosureBudget {
    pub fn from_candidate(candidate: &ClosureCandidate<'_, '_>) -> Self {
        Self {
            original_bytes: candidate.original_byte_count(),
            nodes: candidate.nodes().len(),
            edges: candidate.edges().len(),
        }
    }

    pub fn charge(self, artifact_len: usize) -> Result<Self, E> {
        if artifact_len == 0 || artifact_len > MAX_DISCLOSURE_POLICY_BYTES {
            return Err(E::Limit);
        }
        let nodes = self.nodes.checked_add(1).ok_or(E::Limit)?;
        let original_bytes = self
            .original_bytes
            .checked_add(artifact_len)
            .ok_or(E::Limit)?;
        if nodes > MAX_CLOSURE_NODES
            || self.edges > MAX_CLOSURE_EDGES
            || original_bytes > MAX_CLOSURE_ORIGINAL_BYTES
        {
            return Err(E::Limit);
        }
        Ok(Self {
            original_bytes,
            nodes,
            edges: self.edges,
        })
    }

    pub fn admit_counted(&self) -> Result<(), E> {
        if self.nodes == 0
            || self.nodes > MAX_CLOSURE_NODES
            || self.edges > MAX_CLOSURE_EDGES
            || self.original_bytes > MAX_CLOSURE_ORIGINAL_BYTES
        {
            return Err(E::Limit);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisclosurePolicyFields {
    pub coverage: SourceCoverage,
    pub source: StrongDigest,
    pub source_length: u64,
    pub controller: StrongDigest,
    pub publisher: StrongDigest,
    pub source_label: StrongDigest,
    pub closure_labels: [StrongDigest; MAX_CLOSURE_LABELS],
    pub closure_label_count: u8,
    pub recipient: StrongDigest,
    pub purpose: StrongDigest,
    pub operation: OperationId,
    pub consumer: StrongDigest,
    pub rule: StrongDigest,
    pub graph: StrongDigest,
}

#[derive(Clone, Copy, Debug)]
pub struct DisclosurePolicyView<'a> {
    encoded: &'a [u8],
    fields: DisclosurePolicyFields,
}

impl<'a> DisclosurePolicyView<'a> {
    pub const fn fields(&self) -> DisclosurePolicyFields {
        self.fields
    }
    pub const fn encoded_bytes(&self) -> &'a [u8] {
        self.encoded
    }
    pub fn digest(&self) -> StrongDigest {
        sha384(self.encoded)
    }
}

pub fn encode_disclosure_policy_into(
    fields: DisclosurePolicyFields,
    budget: ClosureBudget,
    out: &mut [u8],
) -> Result<usize, E> {
    fields_valid(fields)?;
    let n = FIXED + fields.closure_label_count as usize * 48;
    budget.charge(n)?;
    let out = out.get_mut(..n).ok_or(E::Capacity)?;
    let mut pos = 0;
    put(out, &mut pos, MAGIC);
    put(out, &mut pos, &[VERSION, fields.coverage.bits()]);
    put(out, &mut pos, &fields.source.0);
    put(out, &mut pos, &fields.source_length.to_be_bytes());
    for d in [fields.controller, fields.publisher, fields.source_label] {
        put(out, &mut pos, &d.0);
    }
    put(out, &mut pos, &[fields.closure_label_count]);
    for i in 0..fields.closure_label_count as usize {
        put(out, &mut pos, &fields.closure_labels[i].0);
    }
    for d in [fields.recipient, fields.purpose] {
        put(out, &mut pos, &d.0);
    }
    put(out, &mut pos, &fields.operation.0);
    for d in [fields.consumer, fields.rule, fields.graph] {
        put(out, &mut pos, &d.0);
    }
    debug_assert_eq!(pos, n);
    Ok(n)
}

pub fn decode_disclosure_policy(bytes: &[u8]) -> Result<DisclosurePolicyView<'_>, E> {
    if bytes.len() > MAX_DISCLOSURE_POLICY_BYTES {
        return Err(E::Limit);
    }
    if bytes.len() < FIXED || !bytes.starts_with(MAGIC) {
        return Err(E::Encoding);
    }
    let mut pos = MAGIC.len();
    if bytes[pos] != VERSION {
        return Err(E::Encoding);
    }
    pos += 1;
    let coverage = SourceCoverage::from_bits(bytes[pos]).admit()?;
    pos += 1;
    let source = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let source_length = u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    pos += 8;
    let controller = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let publisher = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let source_label = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let label_count = bytes[pos] as usize;
    pos += 1;
    if label_count > MAX_CLOSURE_LABELS {
        return Err(E::Limit);
    }
    if bytes.len() != FIXED + label_count * 48 {
        return Err(E::Encoding);
    }
    let mut closure_labels = [StrongDigest::ZERO; MAX_CLOSURE_LABELS];
    for i in 0..label_count {
        closure_labels[i] = digest(&bytes[pos..pos + 48]);
        pos += 48;
    }
    let recipient = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let purpose = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let operation = OperationId(bytes[pos..pos + 16].try_into().unwrap());
    pos += 16;
    let consumer = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let rule = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let graph = digest(&bytes[pos..pos + 48]);
    let fields = DisclosurePolicyFields {
        coverage,
        source,
        source_length,
        controller,
        publisher,
        source_label,
        closure_labels,
        closure_label_count: label_count as u8,
        recipient,
        purpose,
        operation,
        consumer,
        rule,
        graph,
    };
    fields_valid(fields)?;
    Ok(DisclosurePolicyView {
        encoded: bytes,
        fields,
    })
}

/// Policy original must already be a counted closure node inside existing limits.
pub fn bind_policy_to_closure<'a>(
    encoded: &'a [u8],
    candidate: &ClosureCandidate<'_, 'a>,
) -> Result<DisclosurePolicyView<'a>, E> {
    let view = decode_disclosure_policy(encoded)?;
    ClosureBudget::from_candidate(candidate).admit_counted()?;
    let digest = view.digest();
    let node = candidate
        .nodes()
        .iter()
        .find(|n| n.digest == digest)
        .ok_or(E::Graph)?;
    if node.original_bytes != encoded {
        return Err(E::Graph);
    }
    Ok(view)
}

fn fields_valid(f: DisclosurePolicyFields) -> Result<(), E> {
    f.coverage.admit()?;
    if f.source_length < 64 || f.source_length > MAX_OBJECT_BYTES {
        return Err(E::Reference);
    }
    if f.closure_label_count as usize > MAX_CLOSURE_LABELS {
        return Err(E::Limit);
    }
    for d in [
        f.source,
        f.controller,
        f.publisher,
        f.source_label,
        f.recipient,
        f.purpose,
        f.consumer,
        f.rule,
        f.graph,
    ] {
        if d.is_zero() {
            return Err(E::Reference);
        }
    }
    if f.operation.is_zero() {
        return Err(E::Reference);
    }
    let n = f.closure_label_count as usize;
    for i in 0..n {
        if f.closure_labels[i].is_zero() {
            return Err(E::Reference);
        }
        if i != 0 && f.closure_labels[i - 1].0 >= f.closure_labels[i].0 {
            return Err(E::Encoding);
        }
    }
    for i in n..MAX_CLOSURE_LABELS {
        if !f.closure_labels[i].is_zero() {
            return Err(E::Encoding);
        }
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> StrongDigest {
    StrongDigest(bytes.try_into().expect("validated fixed-width digest"))
}

fn put(out: &mut [u8], pos: &mut usize, bytes: &[u8]) {
    out[*pos..*pos + bytes.len()].copy_from_slice(bytes);
    *pos += bytes.len();
}
