//! Native first-paint QBDL producer. Browser/wasm never calls `InventoryVerifier::finish`.
use crate::bundle::stream::descriptor::{
    decode_body, CandidateIdentity, ContextEntry, ContextParameters, DescriptorError, EntryRecord,
    InventoryVerifier, HASH_SCRATCH_BYTES, MAX_ENCODED, MAX_ENTRIES, MAX_METADATA, MAX_PAGES,
    PAGE_ROWS, PageRef,
};
use crate::bundle::stream::index::{
    EntryScratch, ExtentLimit, IndexReadLimits, admit_header, admit_index,
};
use crate::bundle::{BundleError, BundleReadLimits, BundleWriter, BUNDLE_HEADER_SIZE};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use super::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions};

const KIND_10D: &str = "10d";
const INNER_VERSION: u64 = 1;

/// Intact `.10d` member. `key` must be unique; kind is always `"10d"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstPaintSource<'a> {
    pub key: &'a str,
    pub bytes: &'a [u8],
}

/// Producer four-tuple as lowercase SHA-384 hex plus generation/entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateIdentityHex {
    pub descriptor_sha384: String,
    pub context_sha384: String,
    pub generation: u64,
    pub entries: u32,
}

#[derive(Debug)]
pub enum FirstPaintError {
    EmptySources,
    Bundle(BundleError),
    Descriptor(DescriptorError),
}

impl core::fmt::Display for FirstPaintError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptySources => write!(f, "first-paint sources empty"),
            Self::Bundle(error) => write!(f, "first-paint bundle {error}"),
            Self::Descriptor(error) => write!(f, "first-paint {error}"),
        }
    }
}

impl core::error::Error for FirstPaintError {}

impl From<BundleError> for FirstPaintError {
    fn from(value: BundleError) -> Self {
        Self::Bundle(value)
    }
}

impl From<DescriptorError> for FirstPaintError {
    fn from(value: DescriptorError) -> Self {
        Self::Descriptor(value)
    }
}

impl CandidateIdentityHex {
    /// Sidecar body for `packages/qbdl/inventory-identity.json` after a native finish.
    pub fn sidecar_json(&self) -> String {
        format!(
            "{{\n  \"schema\": \"webizen.ewm-inventory-identity/v1\",\n  \"status\": \"inventory-verifier-finish\",\n  \"inner_ranges\": false,\n  \"magic\": \"QBDL\",\n  \"streaming_default\": false,\n  \"browser_inventory_verifier\": false,\n  \"host_bind\": \"CandidateIdentity::from_parts\",\n  \"candidate\": {{\n    \"descriptor_sha384\": \"{}\",\n    \"context_sha384\": \"{}\",\n    \"generation\": {},\n    \"entries\": {}\n  }}\n}}\n",
            self.descriptor_sha384, self.context_sha384, self.generation, self.entries
        )
    }
}

fn hex48(digest: StrongDigest) -> String {
    hex::encode(digest.as_bytes())
}

fn first_paint_options() -> PublisherOptions {
    PublisherOptions {
        parameters: ContextParameters {
            nonce: {
                let digest = sha384(b"webizen.ewm-first-paint/v1/nonce");
                let mut nonce = [0u8; 32];
                nonce.copy_from_slice(&digest.0[..32]);
                nonce
            },
            generation: 0,
            predecessor: None,
            semantic_contract: sha384(b"webizen.ewm-first-paint/v1/semantic-contract"),
            policy: sha384(b"webizen.ewm-first-paint/v1/policy"),
            consumer_profile: sha384(b"webizen.ewm-first-paint/v1/consumer-profile"),
        },
        class: PublicationClass::PlainCandidate,
        metadata_limit: MAX_METADATA,
    }
}

/// One QBDL-profile HMC of intact first-paint members. Flags stay 0 (not ZIP, not 4096).
pub fn build_first_paint_hmc(sources: &[FirstPaintSource<'_>]) -> Result<Vec<u8>, FirstPaintError> {
    if sources.is_empty() {
        return Err(FirstPaintError::EmptySources);
    }
    if sources.len() > MAX_ENTRIES {
        return Err(FirstPaintError::Descriptor(DescriptorError::Limit));
    }
    let mut writer = BundleWriter::new();
    for source in sources {
        writer.add_file(source.key, KIND_10D, source.bytes.to_vec(), None)?;
    }
    Ok(writer.build()?)
}

/// BundleWriter → PlainPublisher → InventoryVerifier::finish. Native only.
pub fn publish_first_paint(
    sources: &[FirstPaintSource<'_>],
) -> Result<CandidateIdentityHex, FirstPaintError> {
    let source = build_first_paint_hmc(sources)?;
    let n = sources.len();
    let assignments: Vec<EntryAssignment> = (0..n)
        .map(|i| EntryAssignment {
            id: (i as u32) + 1,
            inner_version: INNER_VERSION,
        })
        .collect();
    let mut index = vec![EntryScratch::EMPTY; n];
    let mut ids = vec![0u32; n];
    let mut context_refs = [PageRef::EMPTY; MAX_PAGES];
    let mut entry_refs = [PageRef::EMPTY; MAX_PAGES];
    let mut publisher = PlainPublisher::prepare(
        &source,
        BundleReadLimits::default(),
        &assignments,
        first_paint_options(),
        &mut index,
        &mut ids,
        &mut context_refs,
        &mut entry_refs,
    )?;
    let mut output = vec![0u8; MAX_ENCODED];
    let mut frame = vec![0u8; HASH_SCRATCH_BYTES];
    let mut contexts = Vec::new();
    let mut entries = Vec::new();
    for page in 0..publisher.page_count() {
        publisher.emit_context_page(page, &mut output, &mut frame, |bytes, _| {
            contexts.push(bytes.to_vec());
            Ok(())
        })?;
    }
    let context_n = publisher.encode_context_root(&mut output, &mut frame)?;
    let context = output[..context_n].to_vec();
    for page in 0..publisher.page_count() {
        publisher.emit_entry_page(
            page,
            &mut output,
            &mut frame,
            |_, _| Ok(()),
            |bytes, _| {
                entries.push(bytes.to_vec());
                Ok(())
            },
        )?;
    }
    let published = publisher.finish(&context, &mut output, &mut frame)?;
    let body_n = publisher.body_encoded_length(&context)?;
    let body = output[..body_n].to_vec();
    drop(publisher);

    let body_view = decode_body(&body)?;
    let header_bytes = source
        .get(..BUNDLE_HEADER_SIZE)
        .ok_or(DescriptorError::Source)?;
    let header = admit_header(
        header_bytes,
        source.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .map_err(|_| DescriptorError::Source)?;
    let mut scratch = vec![EntryScratch::EMPTY; n];
    let index_start =
        usize::try_from(header.index_offset()).map_err(|_| DescriptorError::Overflow)?;
    let index_bytes = source.get(index_start..).ok_or(DescriptorError::Source)?;
    let admitted = admit_index(
        &header,
        index_bytes,
        IndexReadLimits::default(),
        &mut scratch,
    )
    .map_err(|_| DescriptorError::Source)?;
    let mut context_rows_store = vec![ContextEntry::EMPTY; n];
    let mut verifier = InventoryVerifier::new(
        body_view,
        header_bytes,
        &admitted,
        &mut context_rows_store,
        &mut frame,
        MAX_METADATA,
    )?;
    let mut context_rows = [ContextEntry::EMPTY; PAGE_ROWS];
    for page in &contexts {
        verifier.accept_context_page(page, &mut context_rows, &mut frame)?;
    }
    let mut seal_ids = vec![0u32; n];
    verifier.seal_contexts(&mut seal_ids)?;
    let mut entry_rows = [EntryRecord::EMPTY; PAGE_ROWS];
    for page in &entries {
        verifier.accept_entry_page(page, &mut entry_rows, &mut frame)?;
    }
    let verified = verifier.finish()?;
    if verified != published {
        return Err(FirstPaintError::Descriptor(DescriptorError::Commitment));
    }
    let bound = CandidateIdentity::from_parts(
        verified.descriptor_id(),
        verified.context_id(),
        verified.generation(),
        verified.entry_count(),
    )?;
    Ok(CandidateIdentityHex {
        descriptor_sha384: hex48(bound.descriptor_id()),
        context_sha384: hex48(bound.context_id()),
        generation: bound.generation(),
        entries: bound.entry_count(),
    })
}
