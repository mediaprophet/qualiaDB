use super::*;
#[test]
fn deterministic_source_context_body_and_detached_binding_candidates() {
    let source = bundle(2);
    let before = source.clone();
    let assign = assignments(2);
    let a = publish(&source, &assign);
    let b = publish(&source, &assign);
    assert_eq!(a.context, b.context);
    assert_eq!(a.body, b.body);
    assert_eq!(a.contexts, b.contexts);
    assert_eq!(a.entries, b.entries);
    assert_eq!(a.identity, b.identity);
    assert_eq!(source, before);
    let body = decode_body(&a.body).unwrap();
    assert_eq!(body.context().source().source_digest, sha384(&source));
    assert_eq!(body.context().parameters(), parameters());
    let keys = crate::crypto::network::cose::KeyIds {
        ml_dsa_65: [8; 48],
        ed25519: [9; 48],
    };
    let binding = a
        .identity
        .binding(keys, 4, b"candidate-publication", b"test-audience");
    let descriptor = a.identity.descriptor_id();
    assert_eq!(binding.descriptor_digest, &descriptor.0);
    assert_eq!(binding.controller_epoch, 4);
}
#[test]
fn paged_inventory_requires_every_page_and_preserves_exact_source_mapping() {
    for n in [0, 2, 129] {
        let source = bundle(n);
        let assign = assignments(n);
        let a = publish(&source, &assign);
        let body = decode_body(&a.body).unwrap();
        let mut frame = [0u8; HASH_SCRATCH_BYTES];
        let h = admit_header(
            &source[..64],
            source.len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let mut scratch = vec![EntryScratch::EMPTY; n];
        let index = admit_index(
            &h,
            &source[h.index_offset() as usize..],
            IndexReadLimits::default(),
            &mut scratch,
        )
        .unwrap();
        let mut contexts = vec![ContextEntry::EMPTY; n];
        let mut verifier = InventoryVerifier::new(
            body,
            &source[..64],
            &index,
            &mut contexts,
            &mut frame,
            MAX_METADATA,
        )
        .unwrap();
        assert_eq!(verifier.finish(), Err(DescriptorError::Incomplete));
        let mut context_rows = [ContextEntry::EMPTY; PAGE_ROWS];
        for page in a.contexts.iter().rev() {
            verifier
                .accept_context_page(page, &mut context_rows, &mut frame)
                .unwrap();
        }
        let mut ids = vec![0; n];
        verifier.seal_contexts(&mut ids).unwrap();
        let mut rows = [EntryRecord::EMPTY; PAGE_ROWS];
        for page in a.entries.iter().rev() {
            verifier
                .accept_entry_page(page, &mut rows, &mut frame)
                .unwrap();
        }
        assert_eq!(verifier.finish(), Ok(a.identity));
        if let Some(page) = a.contexts.first() {
            assert_eq!(
                verifier.accept_context_page(page, &mut context_rows, &mut frame),
                Err(DescriptorError::Page)
            );
        }
        if let Some(page) = a.entries.first() {
            assert_eq!(
                verifier.accept_entry_page(page, &mut rows, &mut frame),
                Err(DescriptorError::Page)
            );
        }
    }
}
#[test]
fn bounded_source_changes_change_the_immutable_identity() {
    let assign = assignments(2);
    let a = publish(&bundle(2), &assign);
    let mut writer = BundleWriter::new();
    writer.add_file("entry-0", "blob", vec![], None).unwrap();
    writer
        .add_file("entry-1", "blob", vec![99; 3], None)
        .unwrap();
    let changed = writer.build().unwrap();
    let b = publish(&changed, &assign);
    assert_ne!(a.identity.context_id(), b.identity.context_id());
    assert_ne!(a.identity.descriptor_id(), b.identity.descriptor_id());
}

#[test]
fn padding_only_change_with_valid_crc_changes_full_source_commitment() {
    use crate::bundle::format::OFF_CRC32C;
    use crate::container_10d::crc32c::crc32c;
    let original = bundle(2);
    let h = admit_header(
        &original[..64],
        original.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 2];
    let index = admit_index(
        &h,
        &original[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut scratch,
    )
    .unwrap();
    let end = index
        .entries()
        .map(|e| {
            let e = e.unwrap();
            e.offset() + e.length()
        })
        .max()
        .unwrap() as usize;
    assert!(end < h.index_offset() as usize);
    let mut changed = original.clone();
    changed[end] ^= 0x80;
    changed[OFF_CRC32C..OFF_CRC32C + 4].fill(0);
    let crc = crc32c(&changed);
    changed[OFF_CRC32C..OFF_CRC32C + 4].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        &changed[h.index_offset() as usize..],
        &original[h.index_offset() as usize..]
    );
    for e in index.entries() {
        let e = e.unwrap();
        let range = e.offset() as usize..(e.offset() + e.length()) as usize;
        assert_eq!(&changed[range.clone()], &original[range]);
    }
    let a = publish(&original, &assignments(2));
    let b = publish(&changed, &assignments(2));
    let ac = decode_body(&a.body).unwrap().context().source();
    let bc = decode_body(&b.body).unwrap().context().source();
    assert_eq!(ac.index_digest, bc.index_digest);
    assert_ne!(ac.source_digest, bc.source_digest);
    assert_ne!(a.identity.context_id(), b.identity.context_id());
    assert_ne!(a.identity.descriptor_id(), b.identity.descriptor_id());
}
