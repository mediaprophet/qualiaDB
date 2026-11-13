//! Cold original-source admission; original bytes remain borrowed and unchanged.
use super::{EntryRange, ProtectionError as Error, SourceObject, coverage};
use crate::bundle::format::BUNDLE_HEADER_SIZE;
use crate::bundle::stream::descriptor::MAX_ENTRIES;
use crate::bundle::stream::index::{
    AdmittedIndex, EntryScratch, ExtentLimit, admit_header, admit_index,
};
use crate::bundle::{BundleReadLimits, BundleReader};
use crate::crypto::network::object_stream::{MAX_METADATA, MAX_OBJECT_BYTES};

/// A private structural preparation, not an authenticated/encrypted publication.
/// Deliberately no Debug: retained index bytes contain private names and metadata.
pub(crate) struct PreparedSource<'a, 's> {
    source: &'a [u8],
    index: AdmittedIndex<'a>,
    objects: &'s [SourceObject],
    metadata_length: usize,
}
impl<'a, 's> PreparedSource<'a, 's> {
    /// The host must retain immutable snapshot/writer exclusion for external or
    /// mapped storage throughout this lifetime. Rust's slice borrow alone does
    /// not exclude external writers. This does not generate IDs, keys or rights.
    ///
    /// All interval/order scratch is supplied by the caller and remains private.
    /// Full BundleReader validation uses its existing bounded cold materialization,
    /// then drops it before returning; this is not a zero-allocation hot API.
    pub(crate) fn prepare(
        source: &'a [u8],
        limits: BundleReadLimits,
        index_scratch: &mut [EntryScratch<'a>],
        entries: &mut [EntryRange],
        objects: &'s mut [SourceObject],
        order: &mut [u32],
    ) -> Result<Self, Error> {
        let extent = u64::try_from(source.len()).map_err(|_| Error::Overflow)?;
        if extent > MAX_OBJECT_BYTES || source.len() > limits.file_bytes {
            return Err(Error::Limit);
        }
        let metadata_cap = usize::try_from(MAX_METADATA).map_err(|_| Error::Overflow)?;
        let limits = BundleReadLimits {
            entries: limits.entries.min(MAX_ENTRIES),
            index_bytes: limits.index_bytes.min(metadata_cap),
            decoded_index_bytes: limits.decoded_index_bytes.min(metadata_cap),
            ..limits
        };
        let raw_header = source.get(..BUNDLE_HEADER_SIZE).ok_or(Error::Source)?;
        let header = admit_header(
            raw_header,
            extent,
            ExtentLimit {
                max_logical_bytes: MAX_OBJECT_BYTES,
            },
            limits.into(),
        )
        .map_err(|_| Error::Source)?;
        let count = usize::try_from(header.entry_count()).map_err(|_| Error::Overflow)?;
        if index_scratch.len() < count {
            return Err(Error::Capacity);
        }
        let start = usize::try_from(header.index_offset()).map_err(|_| Error::Overflow)?;
        let raw_index = source.get(start..).ok_or(Error::Source)?;
        let index = admit_index(&header, raw_index, limits.into(), index_scratch)
            .map_err(|_| Error::Source)?;
        let count = coverage::prepare(&index, entries, objects, order)?;
        let metadata_length = coverage::metadata_length(count, header.index_length())?;
        {
            // Canonical full-format owner verifies whole CRC and every original
            // SHA256 payload; no second QBDL/CBOR parser is introduced here.
            let full =
                BundleReader::parse_with_limits(source, limits).map_err(|_| Error::Source)?;
            if full.entries().len() != index.sizing().entry_count() {
                return Err(Error::Coverage);
            }
        }
        Ok(Self {
            source,
            index,
            objects: &objects[..count],
            metadata_length,
        })
    }
    pub(super) fn checked_header(&self) -> crate::bundle::stream::index::CheckedHeader {
        self.index.header()
    }
    pub(crate) fn objects(&self) -> &[SourceObject] {
        self.objects
    }
    pub(crate) fn entry_count(&self) -> usize {
        self.index.sizing().entry_count()
    }
    pub(crate) const fn metadata_length(&self) -> usize {
        self.metadata_length
    }
    pub(crate) fn header_bytes(&self) -> &'a [u8] {
        &self.source[..BUNDLE_HEADER_SIZE]
    }
    pub(crate) fn index_bytes(&self) -> &'a [u8] {
        self.index.encoded_bytes()
    }
    pub(crate) fn source_bytes(&self) -> &'a [u8] {
        self.source
    }
    pub(crate) fn object_bytes(&self, ordinal: usize) -> Result<&'a [u8], Error> {
        let row = self.objects.get(ordinal).ok_or(Error::Coverage)?;
        let start = usize::try_from(row.offset()).map_err(|_| Error::Overflow)?;
        let end = row
            .offset()
            .checked_add(row.length())
            .ok_or(Error::Overflow)?;
        let end = usize::try_from(end).map_err(|_| Error::Overflow)?;
        self.source.get(start..end).ok_or(Error::Coverage)
    }
}
