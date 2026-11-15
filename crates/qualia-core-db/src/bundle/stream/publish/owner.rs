use crate::bundle::stream::descriptor::*;
use crate::bundle::stream::index::{
    AdmittedIndex, EntryScratch, ExtentLimit, admit_header, admit_index,
};
use crate::bundle::{BundleReadLimits, BundleReader};
use crate::crypto::network::{cose::MAX_ENVELOPE_BYTES, digest::sha384};
use crate::net::qdnf::types::StrongDigest;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryAssignment {
    pub id: u32,
    pub inner_version: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationClass {
    PlainCandidate,
    PrivateProjection,
    RecipientWrapped,
}
#[derive(Clone, Copy, Debug)]
pub struct PublisherOptions {
    pub parameters: ContextParameters,
    pub class: PublicationClass,
    pub metadata_limit: usize,
}
pub struct PlainPublisher<'a, 's> {
    pub(super) source: &'a [u8],
    pub(super) index: AdmittedIndex<'a>,
    pub(super) assignments: &'a [EntryAssignment],
    pub(super) parameters: ContextParameters,
    pub(super) source_commit: SourceCommit,
    pub(super) context_refs: &'s mut [PageRef],
    pub(super) entry_refs: &'s mut [PageRef],
    pub(super) context_next: usize,
    pub(super) entry_next: usize,
    pub(super) context_id: Option<StrongDigest>,
    pub(super) used: usize,
    pub(super) limit: usize,
    pub(super) poisoned: bool,
    pub(super) completed: Option<CandidateIdentity>,
}
impl<'a, 's> PlainPublisher<'a, 's> {
    /// Prepare a structurally validated publication candidate.
    /// For mapped/external storage the caller must retain an immutable snapshot
    /// with writer exclusion for this publisher's entire lifetime. A Rust slice
    /// alone does not establish exclusion against external writers.
    /// Page/outboard sinks stage candidates only: authority, disclosure policy
    /// and durable commit must succeed before public delivery or promotion.
    pub fn prepare(
        source: &'a [u8],
        limits: BundleReadLimits,
        assignments: &'a [EntryAssignment],
        options: PublisherOptions,
        index_scratch: &mut [EntryScratch<'a>],
        id_scratch: &mut [u32],
        context_refs: &'s mut [PageRef],
        entry_refs: &'s mut [PageRef],
    ) -> Result<Self, DescriptorError> {
        if options.class != PublicationClass::PlainCandidate {
            return Err(DescriptorError::Unsupported);
        }
        options.parameters.validate()?;
        if options.metadata_limit > MAX_METADATA || options.metadata_limit < MAX_ENVELOPE_BYTES {
            return Err(DescriptorError::Limit);
        }
        if assignments.len() > MAX_ENTRIES {
            return Err(DescriptorError::Limit);
        }
        let limits = BundleReadLimits {
            entries: limits.entries.min(MAX_ENTRIES),
            index_bytes: limits.index_bytes.min(MAX_METADATA),
            decoded_index_bytes: limits.decoded_index_bytes.min(MAX_METADATA),
            ..limits
        };
        let ids = id_scratch
            .get_mut(..assignments.len())
            .ok_or(DescriptorError::Capacity)?;
        for (out, a) in ids.iter_mut().zip(assignments) {
            *out = a.id;
        }
        ids.sort_unstable();
        if ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(DescriptorError::DuplicateId);
        }
        if index_scratch.len() < assignments.len() {
            return Err(DescriptorError::Capacity);
        }
        let pages = assignments.len().div_ceil(PAGE_ROWS);
        if context_refs.len() < pages || entry_refs.len() < pages {
            return Err(DescriptorError::Capacity);
        }
        let extent = u64::try_from(source.len()).map_err(|_| DescriptorError::Overflow)?;
        let header_bytes = source.get(..64).ok_or(DescriptorError::Source)?;
        let header = admit_header(header_bytes, extent, ExtentLimit::default(), limits.into())
            .map_err(|_| DescriptorError::Source)?;
        let start =
            usize::try_from(header.index_offset()).map_err(|_| DescriptorError::Overflow)?;
        let index_bytes = source.get(start..).ok_or(DescriptorError::Source)?;
        let index = admit_index(&header, index_bytes, limits.into(), index_scratch)
            .map_err(|_| DescriptorError::Source)?;
        // Cold owned metadata remains bounded/fallible, and is dropped before retaining this publisher.
        {
            let full = BundleReader::parse_with_limits(source, limits)
                .map_err(|_| DescriptorError::Source)?;
            if full.entries().len() != assignments.len() {
                return Err(DescriptorError::Coverage);
            }
        }
        let source_commit = SourceCommit {
            length: extent,
            version: header.probe().version,
            flags: header.probe().flags,
            header_digest: sha384(header_bytes),
            index_offset: header.index_offset(),
            index_length: header.index_length(),
            index_digest: sha384(index_bytes),
            source_digest: sha384(source),
        };
        context_refs[..pages].fill(PageRef::EMPTY);
        entry_refs[..pages].fill(PageRef::EMPTY);
        Ok(Self {
            source,
            index,
            assignments,
            parameters: options.parameters,
            source_commit,
            context_refs,
            entry_refs,
            context_next: 0,
            entry_next: 0,
            context_id: None,
            used: MAX_ENVELOPE_BYTES,
            limit: options.metadata_limit,
            poisoned: false,
            completed: None,
        })
    }
    pub fn page_count(&self) -> usize {
        self.assignments.len().div_ceil(PAGE_ROWS)
    }
    pub fn entry_count(&self) -> usize {
        self.assignments.len()
    }
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    pub const fn context_id(&self) -> Option<StrongDigest> {
        self.context_id
    }
    pub const fn source_commit(&self) -> SourceCommit {
        self.source_commit
    }
    pub const fn metadata_reserved_bytes(&self) -> usize {
        self.used
    }
    pub(super) fn live(&self) -> Result<(), DescriptorError> {
        if self.poisoned {
            Err(DescriptorError::Poisoned)
        } else {
            Ok(())
        }
    }
    pub(super) fn charge(&self, n: usize) -> Result<usize, DescriptorError> {
        self.used
            .checked_add(n)
            .filter(|v| *v <= self.limit)
            .ok_or(DescriptorError::Limit)
    }
}
