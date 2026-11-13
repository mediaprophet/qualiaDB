use super::*;
use crate::bundle::stream::descriptor::*;
use crate::bundle::stream::index::{
    EntryScratch, ExtentLimit, IndexReadLimits, admit_header, admit_index,
};
use crate::bundle::{BundleReadLimits, BundleWriter};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
fn parameters() -> ContextParameters {
    ContextParameters {
        nonce: [1; 32],
        generation: 0,
        predecessor: None,
        semantic_contract: StrongDigest([2; 48]),
        policy: StrongDigest([3; 48]),
        consumer_profile: StrongDigest([4; 48]),
    }
}
fn options() -> PublisherOptions {
    PublisherOptions {
        parameters: parameters(),
        class: PublicationClass::PlainCandidate,
        metadata_limit: MAX_METADATA,
    }
}
fn bundle(count: usize) -> Vec<u8> {
    let mut w = BundleWriter::new();
    for i in 0..count {
        w.add_file(
            format!("entry-{i}"),
            "blob",
            if i == 0 {
                vec![]
            } else {
                vec![(i % 251) as u8; 3]
            },
            None,
        )
        .unwrap();
    }
    w.build().unwrap()
}
fn assignments(n: usize) -> Vec<EntryAssignment> {
    (0..n)
        .map(|i| EntryAssignment {
            id: (1000 + i) as u32,
            inner_version: 7,
        })
        .collect()
}
struct Artifact {
    context: Vec<u8>,
    body: Vec<u8>,
    contexts: Vec<Vec<u8>>,
    entries: Vec<Vec<u8>>,
    identity: CandidateIdentity,
}
fn publish(source: &[u8], assign: &[EntryAssignment]) -> Artifact {
    publish_with_options(source, assign, options())
}
fn publish_with_options(
    source: &[u8],
    assign: &[EntryAssignment],
    options: PublisherOptions,
) -> Artifact {
    let mut index = vec![EntryScratch::EMPTY; assign.len()];
    let mut ids = vec![0; assign.len()];
    let mut context_refs = [PageRef::EMPTY; 32];
    let mut entry_refs = [PageRef::EMPTY; 32];
    let mut publisher = PlainPublisher::prepare(
        source,
        BundleReadLimits::default(),
        assign,
        options,
        &mut index,
        &mut ids,
        &mut context_refs,
        &mut entry_refs,
    )
    .unwrap();
    let mut output = [0u8; MAX_ENCODED];
    let mut frame = [0u8; HASH_SCRATCH_BYTES];
    let mut contexts = Vec::new();
    let mut entries = Vec::new();
    for page in 0..publisher.page_count() {
        publisher
            .emit_context_page(page, &mut output, &mut frame, |bytes, _| {
                contexts.push(bytes.to_vec());
                Ok(())
            })
            .unwrap();
    }
    let n = publisher
        .encode_context_root(&mut output, &mut frame)
        .unwrap();
    let context = output[..n].to_vec();
    for page in 0..publisher.page_count() {
        publisher
            .emit_entry_page(
                page,
                &mut output,
                &mut frame,
                |_, _| Ok(()),
                |bytes, _| {
                    entries.push(bytes.to_vec());
                    Ok(())
                },
            )
            .unwrap();
    }
    let identity = publisher.finish(&context, &mut output, &mut frame).unwrap();
    let n = publisher.body_encoded_length(&context).unwrap();
    let body = output[..n].to_vec();
    let reserved = publisher.metadata_reserved_bytes();
    assert_eq!(
        publisher.finish(&context, &mut output, &mut frame).unwrap(),
        identity
    );
    assert_eq!(&output[..n], body.as_slice());
    assert_eq!(publisher.metadata_reserved_bytes(), reserved);
    Artifact {
        context,
        body,
        contexts,
        entries,
        identity,
    }
}
#[path = "tests/rejection.rs"]
mod rejection;
#[path = "tests/roundtrip.rs"]
mod roundtrip;

#[path = "tests/independent.rs"]
mod independent;
