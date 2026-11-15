use crate::bundle::stream::{
    descriptor::{
        decode_body, decode_entry_page, domain_digest, CandidateIdentity, DescriptorError,
        DigestDomain, EntryRecord, SourceCommit, MAX_ENTRIES,
    },
    index::{admit_header, AdmittedIndex, ExtentLimit, IndexError, IndexReadLimits},
    policy::BoundClosure,
    proof::{empty_root, ProofError, Representation, TreeSpec},
};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    IdentityMismatch,
    Bounds,
    Descriptor(DescriptorError),
    Index(IndexError),
    Proof(ProofError),
}
impl From<DescriptorError> for SelectionError {
    fn from(e: DescriptorError) -> Self {
        Self::Descriptor(e)
    }
}
impl From<IndexError> for SelectionError {
    fn from(e: IndexError) -> Self {
        Self::Index(e)
    }
}
impl From<ProofError> for SelectionError {
    fn from(e: ProofError) -> Self {
        Self::Proof(e)
    }
}
/// Actual committed row from the current plaintext/epoch0 descriptor profile.
/// Its private provenance associates signed bytes; it never grants current rights.
#[derive(Clone, Copy, Debug)]
pub struct SelectedPlainEntry<'p> {
    candidate: CandidateIdentity,
    row: EntryRecord<'p>,
    source: SourceCommit,
}
impl<'p> SelectedPlainEntry<'p> {
    pub const fn candidate_identity(&self) -> CandidateIdentity {
        self.candidate
    }
    pub const fn entry_id(&self) -> u32 {
        self.row.id
    }
    pub const fn ordinal(&self) -> u32 {
        self.row.ordinal
    }
    pub const fn kind(&self) -> &'p str {
        self.row.kind
    }
    pub const fn inner_version(&self) -> u64 {
        self.row.inner_version
    }
    pub const fn offset(&self) -> u64 {
        self.row.offset
    }
    pub const fn logical_length(&self) -> u64 {
        self.row.length
    }
    pub const fn root(&self) -> StrongDigest {
        self.row.root
    }
    pub const fn source_commit(&self) -> SourceCommit {
        self.source
    }
    pub fn tree_spec(&self) -> Result<TreeSpec, SelectionError> {
        Ok(TreeSpec::new(
            self.candidate.context_id(),
            self.row.id,
            self.candidate.generation(),
            Representation::Plaintext,
            0,
            self.row.length,
        )?)
    }
    /// Exact association only; this does not recheck pins, expiry or any grant.
    pub fn check_closure(&self, closure: &BoundClosure<'_>) -> Result<(), SelectionError> {
        if self.candidate != closure.identity() {
            return Err(SelectionError::IdentityMismatch);
        }
        Ok(())
    }
}
#[allow(clippy::too_many_arguments)]
pub fn select_entry<'p>(
    closure: &BoundClosure<'_>,
    body_bytes: &[u8],
    raw_header: &[u8],
    index: &AdmittedIndex<'_>,
    entry_page: &'p [u8],
    ordinal: u32,
    rows: &mut [EntryRecord<'p>],
    framing: &mut [u8],
) -> Result<SelectedPlainEntry<'p>, SelectionError> {
    use SelectionError as E;
    let body = decode_body(body_bytes)?;
    let candidate = closure.identity();
    if domain_digest(DigestDomain::Body, body_bytes, framing)? != candidate.descriptor_id()
        || body.context_id() != candidate.context_id()
        || domain_digest(
            DigestDomain::Context,
            body.context().encoded_bytes(),
            framing,
        )? != candidate.context_id()
        || body.context().parameters().generation != candidate.generation()
        || body.context().entry_count() != candidate.entry_count() as usize
    {
        return Err(E::IdentityMismatch);
    }
    let source = body.context().source();
    let limits = IndexReadLimits {
        index_bytes: index.encoded_bytes().len(),
        entries: MAX_ENTRIES,
        ..IndexReadLimits::default()
    };
    let header = admit_header(raw_header, source.length, ExtentLimit::default(), limits)?;
    if header.probe() != index.header().probe()
        || source.version != header.probe().version
        || source.flags != header.probe().flags
        || source.index_offset != header.index_offset()
        || source.index_length != header.index_length()
        || header.entry_count() as usize != body.context().entry_count()
        || sha384(raw_header) != source.header_digest
        || sha384(index.encoded_bytes()) != source.index_digest
    {
        return Err(E::IdentityMismatch);
    }
    let h = decode_entry_page(entry_page, rows)?;
    let want = body.page_refs().get(h.number as usize)?;
    if h.context != candidate.context_id()
        || h.first != want.first
        || h.rows != want.rows as usize
        || entry_page.len() != want.encoded_length as usize
        || domain_digest(DigestDomain::EntryPage, entry_page, framing)? != want.digest
    {
        return Err(E::IdentityMismatch);
    }
    let local = ordinal.checked_sub(h.first).ok_or(E::Bounds)? as usize;
    let row = *rows
        .get(local)
        .filter(|_| local < h.rows)
        .ok_or(E::Bounds)?;
    let actual = index.entries().nth(ordinal as usize).ok_or(E::Bounds)??;
    if row.ordinal != ordinal
        || row.offset != actual.offset()
        || row.length != actual.length()
        || row.kind != actual.kind()
    {
        return Err(E::IdentityMismatch);
    }
    let selected = SelectedPlainEntry {
        candidate,
        row,
        source,
    };
    if row.length == 0 && empty_root(&selected.tree_spec()?)? != row.root {
        return Err(E::IdentityMismatch);
    }
    Ok(selected)
}
