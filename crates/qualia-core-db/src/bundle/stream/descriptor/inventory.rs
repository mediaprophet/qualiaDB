//! Whole-inventory structural completion requires exact authenticated-page commitments.
use super::*;
use crate::bundle::stream::index::{AdmittedIndex, ExtentLimit, IndexReadLimits, admit_header};
use crate::bundle::stream::proof::{Representation, TreeSpec, empty_root};
use crate::crypto::network::{cose::MAX_ENVELOPE_BYTES, digest::sha384};

pub struct InventoryVerifier<'a, 's> {
    body: BodyView<'a>,
    index: &'a AdmittedIndex<'a>,
    contexts: &'s mut [ContextEntry],
    context_seen: u32,
    entry_seen: u32,
    sealed: bool,
    used: usize,
    limit: usize,
    identity: CandidateIdentity,
}
fn mask(n: usize) -> u32 {
    if n == 32 { u32::MAX } else { (1u32 << n) - 1 }
}
impl<'a, 's> InventoryVerifier<'a, 's> {
    pub fn new(
        body: BodyView<'a>,
        raw_header: &[u8],
        index: &'a AdmittedIndex<'a>,
        contexts: &'s mut [ContextEntry],
        framing: &mut [u8],
        metadata_limit: usize,
    ) -> Result<Self, DescriptorError> {
        let c = body.context();
        let source = c.source();
        if contexts.len() < c.entry_count() {
            return Err(DescriptorError::Capacity);
        }
        if metadata_limit > MAX_METADATA {
            return Err(DescriptorError::Limit);
        }
        let limits = IndexReadLimits {
            index_bytes: index.encoded_bytes().len(),
            entries: MAX_ENTRIES,
            ..IndexReadLimits::default()
        };
        let checked = admit_header(raw_header, source.length, ExtentLimit::default(), limits)
            .map_err(|_| DescriptorError::Source)?;
        if checked.probe() != index.header().probe()
            || source.version != checked.probe().version
            || source.flags != checked.probe().flags
            || source.index_offset != checked.index_offset()
            || source.index_length != checked.index_length()
            || checked.entry_count() as usize != c.entry_count()
            || sha384(raw_header) != source.header_digest
            || sha384(index.encoded_bytes()) != source.index_digest
        {
            return Err(DescriptorError::Source);
        }
        let context = domain_digest(DigestDomain::Context, c.encoded_bytes(), framing)?;
        if context != body.context_id() {
            return Err(DescriptorError::Context);
        }
        let descriptor = domain_digest(DigestDomain::Body, body.encoded_bytes(), framing)?;
        let used = body
            .encoded_bytes()
            .len()
            .checked_add(c.encoded_bytes().len())
            .and_then(|n| n.checked_add(MAX_ENVELOPE_BYTES))
            .ok_or(DescriptorError::Overflow)?;
        if used > metadata_limit {
            return Err(DescriptorError::Limit);
        }
        Ok(Self {
            body,
            index,
            contexts,
            context_seen: 0,
            entry_seen: 0,
            sealed: false,
            used,
            limit: metadata_limit,
            identity: CandidateIdentity {
                descriptor,
                context,
                generation: c.parameters().generation,
                entries: c.entry_count() as u32,
            },
        })
    }
    fn charge(&self, n: usize) -> Result<usize, DescriptorError> {
        self.used
            .checked_add(n)
            .filter(|v| *v <= self.limit)
            .ok_or(DescriptorError::Limit)
    }
    pub const fn metadata_reserved_bytes(&self) -> usize {
        self.used
    }
    pub fn accept_context_page(
        &mut self,
        bytes: &[u8],
        rows: &mut [ContextEntry],
        framing: &mut [u8],
    ) -> Result<(), DescriptorError> {
        let h = decode_context_page(bytes, rows)?;
        let want = self.body.context().page_refs().get(h.number as usize)?;
        let bit = 1u32.checked_shl(h.number).ok_or(DescriptorError::Page)?;
        if self.context_seen & bit != 0 {
            return Err(DescriptorError::Page);
        }
        if h.first != want.first
            || h.rows != want.rows as usize
            || bytes.len() != want.encoded_length as usize
        {
            return Err(DescriptorError::Coverage);
        }
        if domain_digest(DigestDomain::ContextPage, bytes, framing)? != want.digest {
            return Err(DescriptorError::Commitment);
        }
        let used = self.charge(bytes.len())?;
        let first = h.first as usize;
        self.contexts[first..first + h.rows].copy_from_slice(&rows[..h.rows]);
        self.context_seen |= bit;
        self.used = used;
        Ok(())
    }
    pub fn seal_contexts(&mut self, id_scratch: &mut [u32]) -> Result<(), DescriptorError> {
        if self.sealed {
            return Ok(());
        }
        let c = self.body.context();
        if self.context_seen != mask(c.page_refs().len()) {
            return Err(DescriptorError::Incomplete);
        }
        let ids = id_scratch
            .get_mut(..c.entry_count())
            .ok_or(DescriptorError::Capacity)?;
        for (id, row) in ids.iter_mut().zip(&self.contexts[..c.entry_count()]) {
            *id = row.id;
        }
        ids.sort_unstable();
        if ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(DescriptorError::DuplicateId);
        }
        self.sealed = true;
        Ok(())
    }
    pub fn accept_entry_page<'p>(
        &mut self,
        bytes: &'p [u8],
        rows: &mut [EntryRecord<'p>],
        framing: &mut [u8],
    ) -> Result<(), DescriptorError> {
        if !self.sealed {
            return Err(DescriptorError::Incomplete);
        }
        let h = decode_entry_page(bytes, rows)?;
        if h.context != self.identity.context {
            return Err(DescriptorError::Context);
        }
        let want = self.body.page_refs().get(h.number as usize)?;
        let bit = 1u32.checked_shl(h.number).ok_or(DescriptorError::Page)?;
        if self.entry_seen & bit != 0 {
            return Err(DescriptorError::Page);
        }
        if h.first != want.first
            || h.rows != want.rows as usize
            || bytes.len() != want.encoded_length as usize
        {
            return Err(DescriptorError::Coverage);
        }
        if domain_digest(DigestDomain::EntryPage, bytes, framing)? != want.digest {
            return Err(DescriptorError::Commitment);
        }
        let used = self.charge(bytes.len())?;
        let mut actual = self.index.entries().skip(h.first as usize);
        for row in &rows[..h.rows] {
            let indexed = actual
                .next()
                .ok_or(DescriptorError::Coverage)?
                .map_err(|_| DescriptorError::Source)?;
            let declared = self
                .contexts
                .get(row.ordinal as usize)
                .ok_or(DescriptorError::Coverage)?;
            if row.id != declared.id
                || row.length != declared.length
                || row.offset != indexed.offset()
                || row.length != indexed.length()
                || row.kind != indexed.kind()
            {
                return Err(DescriptorError::Context);
            }
            if row.length == 0 {
                let tree = TreeSpec::new(
                    self.identity.context,
                    row.id,
                    self.identity.generation,
                    Representation::Plaintext,
                    0,
                    0,
                )
                .map_err(|_| DescriptorError::Proof)?;
                if empty_root(&tree).map_err(|_| DescriptorError::Proof)? != row.root {
                    return Err(DescriptorError::Commitment);
                }
            }
        }
        self.entry_seen |= bit;
        self.used = used;
        Ok(())
    }
    pub fn finish(&self) -> Result<CandidateIdentity, DescriptorError> {
        if !self.sealed || self.entry_seen != mask(self.body.page_refs().len()) {
            return Err(DescriptorError::Incomplete);
        }
        Ok(self.identity)
    }
}
