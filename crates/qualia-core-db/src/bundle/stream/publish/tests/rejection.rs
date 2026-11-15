use super::*;
#[test]
fn private_duplicate_ids_bad_source_and_short_scratch_refuse_before_candidate() {
    let source = bundle(2);
    let mut assign = assignments(2);
    let mut index = [EntryScratch::EMPTY; 2];
    let mut ids = [0; 2];
    let mut refs = [PageRef::EMPTY; 32];
    let mut entries = [PageRef::EMPTY; 32];
    assign[1].id = assign[0].id;
    assert_eq!(
        PlainPublisher::prepare(
            &source,
            BundleReadLimits::default(),
            &assign,
            options(),
            &mut index,
            &mut ids,
            &mut refs,
            &mut entries
        )
        .err(),
        Some(DescriptorError::DuplicateId)
    );
    for class in [
        PublicationClass::PrivateProjection,
        PublicationClass::RecipientWrapped,
    ] {
        let o = PublisherOptions { class, ..options() };
        assert_eq!(
            PlainPublisher::prepare(
                &[],
                BundleReadLimits::default(),
                &[],
                o,
                &mut [],
                &mut [],
                &mut refs,
                &mut entries
            )
            .err(),
            Some(DescriptorError::Unsupported)
        );
    }
    let assign = assignments(2);
    assert_eq!(
        PlainPublisher::prepare(
            &source,
            BundleReadLimits::default(),
            &assign,
            options(),
            &mut index,
            &mut [],
            &mut refs,
            &mut entries
        )
        .err(),
        Some(DescriptorError::Capacity)
    );
    let mut changed = source.clone();
    changed[64] ^= 1;
    assert_eq!(
        PlainPublisher::prepare(
            &changed,
            BundleReadLimits::default(),
            &assign,
            options(),
            &mut index,
            &mut ids,
            &mut refs,
            &mut entries
        )
        .err(),
        Some(DescriptorError::Source)
    );
}
#[test]
fn failed_context_sink_outboard_or_entry_sink_permanently_poison() {
    for failure in 0..3 {
        let source = bundle(2);
        let assign = assignments(2);
        let mut index = [EntryScratch::EMPTY; 2];
        let mut ids = [0; 2];
        let mut refs = [PageRef::EMPTY; 32];
        let mut entries = [PageRef::EMPTY; 32];
        let mut p = PlainPublisher::prepare(
            &source,
            BundleReadLimits::default(),
            &assign,
            options(),
            &mut index,
            &mut ids,
            &mut refs,
            &mut entries,
        )
        .unwrap();
        let mut out = [0; MAX_ENCODED];
        let mut frame = [0; HASH_SCRATCH_BYTES];
        if failure == 0 {
            assert_eq!(
                p.emit_context_page(0, &mut out, &mut frame, |_, _| Err(DescriptorError::Sink)),
                Err(DescriptorError::Sink)
            );
        } else {
            p.emit_context_page(0, &mut out, &mut frame, |_, _| Ok(()))
                .unwrap();
            let n = p.encode_context_root(&mut out, &mut frame).unwrap();
            let root = out[..n].to_vec();
            let result = p.emit_entry_page(
                0,
                &mut out,
                &mut frame,
                |_, _| {
                    if failure == 1 {
                        Err(DescriptorError::Sink)
                    } else {
                        Ok(())
                    }
                },
                |_, _| {
                    if failure == 2 {
                        Err(DescriptorError::Sink)
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(result, Err(DescriptorError::Sink));
            assert_eq!(
                p.finish(&root, &mut out, &mut frame),
                Err(DescriptorError::Poisoned)
            );
        }
        assert!(p.is_poisoned());
        assert_eq!(
            p.encode_context_root(&mut out, &mut frame),
            Err(DescriptorError::Poisoned)
        );
    }
}
#[test]
fn unwind_in_an_outboard_sink_cannot_finalize() {
    let source = bundle(2);
    let assign = assignments(2);
    let mut index = [EntryScratch::EMPTY; 2];
    let mut ids = [0; 2];
    let mut refs = [PageRef::EMPTY; 32];
    let mut entries = [PageRef::EMPTY; 32];
    let mut p = PlainPublisher::prepare(
        &source,
        BundleReadLimits::default(),
        &assign,
        options(),
        &mut index,
        &mut ids,
        &mut refs,
        &mut entries,
    )
    .unwrap();
    let mut out = [0; MAX_ENCODED];
    let mut frame = [0; HASH_SCRATCH_BYTES];
    p.emit_context_page(0, &mut out, &mut frame, |_, _| Ok(()))
        .unwrap();
    p.encode_context_root(&mut out, &mut frame).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        p.emit_entry_page(
            0,
            &mut out,
            &mut frame,
            |_, _| panic!("fixture sink unwind"),
            |_, _| Ok(()),
        )
    }));
    assert!(result.is_err());
    assert!(p.is_poisoned());
}
#[test]
fn changed_cross_context_and_omitted_pages_never_complete() {
    let source = bundle(129);
    let assign = assignments(129);
    let a = publish(&source, &assign);
    let body = decode_body(&a.body).unwrap();
    let mut frame = [0; HASH_SCRATCH_BYTES];
    let h = admit_header(
        &source[..64],
        source.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut index_rows = vec![EntryScratch::EMPTY; 129];
    let index = admit_index(
        &h,
        &source[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut index_rows,
    )
    .unwrap();
    let mut contexts = vec![ContextEntry::EMPTY; 129];
    let mut v = InventoryVerifier::new(
        body,
        &source[..64],
        &index,
        &mut contexts,
        &mut frame,
        MAX_METADATA,
    )
    .unwrap();
    let mut ctx = [ContextEntry::EMPTY; PAGE_ROWS];
    let mut ids = [0; 129];
    v.accept_context_page(&a.contexts[0], &mut ctx, &mut frame)
        .unwrap();
    assert_eq!(v.seal_contexts(&mut ids), Err(DescriptorError::Incomplete));
    let mut changed = a.contexts[1].clone();
    let end = changed.len() - 1;
    changed[end] ^= 1;
    assert!(
        v.accept_context_page(&changed, &mut ctx, &mut frame)
            .is_err()
    );
    v.accept_context_page(&a.contexts[1], &mut ctx, &mut frame)
        .unwrap();
    v.seal_contexts(&mut ids).unwrap();
    let mut rows = [EntryRecord::EMPTY; PAGE_ROWS];
    let e = decode_entry_page(&a.entries[0], &mut rows).unwrap();
    let mut bad = [0; MAX_ENCODED];
    let n = encode_entry_page_into(
        StrongDigest([99; 48]),
        e.number,
        e.first,
        &rows[..e.rows],
        &mut bad,
    )
    .unwrap();
    assert_eq!(
        v.accept_entry_page(&bad[..n], &mut rows, &mut frame),
        Err(DescriptorError::Context)
    );
    let mut changed = a.entries[0].clone();
    let end = changed.len() - 1;
    changed[end] ^= 1;
    assert_eq!(
        v.accept_entry_page(&changed, &mut rows, &mut frame),
        Err(DescriptorError::Commitment)
    );
    v.accept_entry_page(&a.entries[0], &mut rows, &mut frame)
        .unwrap();
    assert_eq!(v.finish(), Err(DescriptorError::Incomplete));
    v.accept_entry_page(&a.entries[1], &mut rows, &mut frame)
        .unwrap();
    assert_eq!(v.finish(), Ok(a.identity));
}

#[test]
fn self_consistent_duplicate_context_ids_still_cannot_complete() {
    let source = bundle(2);
    let assign = assignments(2);
    let a = publish(&source, &assign);
    let old = decode_body(&a.body).unwrap();
    let mut frame = [0; HASH_SCRATCH_BYTES];
    let mut ctx_rows = [ContextEntry::EMPTY; PAGE_ROWS];
    let h = decode_context_page(&a.contexts[0], &mut ctx_rows).unwrap();
    ctx_rows[1].id = ctx_rows[0].id;
    let mut page = [0; MAX_ENCODED];
    let n = encode_context_page_into(0, 0, &ctx_rows[..h.rows], &mut page).unwrap();
    let refs = [PageRef {
        number: 0,
        first: 0,
        rows: 2,
        encoded_length: n as u32,
        digest: domain_digest(DigestDomain::ContextPage, &page[..n], &mut frame).unwrap(),
    }];
    let c = old.context();
    let input = ContextInput {
        parameters: c.parameters(),
        source: c.source(),
        entries: 2,
        pages: &refs,
    };
    let mut context = [0; MAX_ENCODED];
    let m = encode_context_into(&input, &mut context).unwrap();
    let context_id = domain_digest(DigestDomain::Context, &context[..m], &mut frame).unwrap();
    let entry_refs = [old.page_refs().get(0).unwrap()];
    let input = BodyInput {
        context_id,
        context_root: &context[..m],
        generation: 0,
        predecessor: None,
        entries: 2,
        pages: &entry_refs,
    };
    let mut body = [0; MAX_ENCODED];
    let b = encode_body_into(&input, &mut body).unwrap();
    let decoded = decode_body(&body[..b]).unwrap();
    let header = admit_header(
        &source[..64],
        source.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut idx = [EntryScratch::EMPTY; 2];
    let index = admit_index(
        &header,
        &source[header.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut idx,
    )
    .unwrap();
    let mut contexts = [ContextEntry::EMPTY; 2];
    let mut v = InventoryVerifier::new(
        decoded,
        &source[..64],
        &index,
        &mut contexts,
        &mut frame,
        MAX_METADATA,
    )
    .unwrap();
    v.accept_context_page(&page[..n], &mut ctx_rows, &mut frame)
        .unwrap();
    assert_eq!(
        v.seal_contexts(&mut [0; 2]),
        Err(DescriptorError::DuplicateId)
    );
    assert_eq!(v.finish(), Err(DescriptorError::Incomplete));
}
#[test]
fn actual_metadata_cumulative_limit_refuses_without_marking_a_page_present() {
    let source = bundle(2);
    let assign = assignments(2);
    let a = publish(&source, &assign);
    let body = decode_body(&a.body).unwrap();
    let mut frame = [0; HASH_SCRATCH_BYTES];
    let h = admit_header(
        &source[..64],
        source.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut idx = [EntryScratch::EMPTY; 2];
    let index = admit_index(
        &h,
        &source[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut idx,
    )
    .unwrap();
    let mut contexts = [ContextEntry::EMPTY; 2];
    let base = a.body.len() + a.context.len() + crate::crypto::network::cose::MAX_ENVELOPE_BYTES;
    let mut v = InventoryVerifier::new(
        body,
        &source[..64],
        &index,
        &mut contexts,
        &mut frame,
        base + a.contexts[0].len() - 1,
    )
    .unwrap();
    let mut rows = [ContextEntry::EMPTY; PAGE_ROWS];
    assert_eq!(
        v.accept_context_page(&a.contexts[0], &mut rows, &mut frame),
        Err(DescriptorError::Limit)
    );
    assert_eq!(v.metadata_reserved_bytes(), base);
    assert_eq!(
        v.seal_contexts(&mut [0; 2]),
        Err(DescriptorError::Incomplete)
    );
}
