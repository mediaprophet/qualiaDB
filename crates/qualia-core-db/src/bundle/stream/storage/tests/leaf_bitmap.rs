use super::*;
#[test]
fn actual_final_leaf_proof_is_required_before_any_integrity_mark() {
    let a = [7u8; LEAF_BYTES as usize];
    let b = [8u8; LEAF_BYTES as usize];
    let tail = [9u8; 3];
    let mut f = fields(2 * LEAF_BYTES + 3, Representation::Plaintext);
    let tree = TreeSpec::new(
        f.context,
        f.entry,
        f.generation,
        f.representation,
        0,
        f.logical_length,
    )
    .unwrap();
    let mut builder = TreeBuilder::new(tree);
    let mut ab = None;
    builder.append(&a, |_| Ok(())).unwrap();
    builder
        .append(&b, |node| {
            if node.start_leaf == 0 && node.leaf_count == 2 {
                ab = Some(node.digest);
            }
            Ok(())
        })
        .unwrap();
    builder.append(&tail, |_| Ok(())).unwrap();
    f.root = builder.finish(|_| Ok(())).unwrap();
    let identity = StorageIdentity::new(f).unwrap();
    let path = [ab.unwrap()];
    let view = verify_leaf_view(&identity, 2, &tail, &path).unwrap();
    assert_eq!(view.logical_length(), 3);
    assert_eq!(view.bytes(), &tail);
    let mut payload = [0xff; BITMAP_BYTES];
    let mut page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    assert_eq!(page.has_integrity_mark(2), Ok(false));
    assert_eq!(page.record_integrity(&view), Ok(MarkStatus::New));
    assert_eq!(page.record_integrity(&view), Ok(MarkStatus::AlreadyMarked));
    assert_eq!(page.payload()[0], 4);
    assert_eq!(
        verify_leaf_view(&identity, 2, &tail[..2], &path).err(),
        Some(StorageError::Proof(ProofError::Length))
    );
    assert_eq!(
        verify_leaf_view(&identity, 3, &tail, &path).err(),
        Some(StorageError::Proof(ProofError::Index))
    );
    let mut wrong = path;
    wrong[0].0[0] ^= 1;
    assert_eq!(
        verify_leaf_view(&identity, 2, &tail, &wrong).err(),
        Some(StorageError::Proof(ProofError::RootMismatch))
    );
    assert_eq!(page.has_integrity_mark(3), Err(StorageError::Bounds));
}
#[test]
fn full_identity_is_required_even_when_the_leaf_math_is_the_same() {
    let data = [2u8; 1];
    let expected = one_leaf(&data, Representation::Plaintext, 0);
    let mut payload = [0; BITMAP_BYTES];
    let mut page = CandidateBitmapPage::new(expected, 0, &mut payload).unwrap();
    for field in 0..12 {
        let mut f = *expected.fields();
        match field {
            0 => f.descriptor.0[0] ^= 1,
            1 => f.context.0[0] ^= 1,
            2 => f.entry ^= 1,
            3 => f.generation ^= 1,
            4 => {
                f.representation = Representation::Ciphertext;
                f.storage_class = StorageClass::SharedCiphertextCandidate;
            }
            5 => {
                f.representation = Representation::Ciphertext;
                f.storage_class = StorageClass::SharedCiphertextCandidate;
                f.key_epoch = 7;
            }
            6 => f.logical_length = 2,
            7 => f.root.0[0] ^= 1,
            8 => f.privacy_partition.0[0] ^= 1,
            9 => f.policy_generation ^= 1,
            10 => f.storage_generation ^= 1,
            _ => f.descriptor.0[47] ^= 1,
        }
        let mut bytes = [2u8; 18];
        if field == 7 {
            bytes[0] = 3;
        }
        let n = if f.representation == Representation::Ciphertext {
            f.logical_length as usize + 16
        } else {
            f.logical_length as usize
        };
        let spec = TreeSpec::new(
            f.context,
            f.entry,
            f.generation,
            f.representation,
            f.key_epoch,
            f.logical_length,
        )
        .unwrap();
        f.root = leaf_hash(&spec, 0, &bytes[..n]).unwrap();
        let changed = StorageIdentity::new(f).unwrap();
        let view = verify_leaf_view(&changed, 0, &bytes[..n], &[]).unwrap();
        assert_eq!(
            page.record_integrity(&view),
            Err(StorageError::IdentityMismatch)
        );
    }
    assert_eq!(page.has_integrity_mark(0), Ok(false));
}
#[test]
fn ciphertext_view_does_not_remove_tag_or_claim_plaintext_authentication() {
    let bytes = [13; 17];
    let identity = one_leaf(&bytes, Representation::Ciphertext, 9);
    let view = verify_leaf_view(&identity, 0, &bytes, &[]).unwrap();
    assert_eq!(view.logical_length(), 1);
    assert_eq!(view.bytes().len(), 17);
    assert_eq!(
        verify_leaf_view(&identity, 0, &bytes[..16], &[]).err(),
        Some(StorageError::Proof(ProofError::Length))
    );
}
#[test]
fn page_bounds_overflow_and_empty_entries_have_no_implied_zero_leaf() {
    let identity = StorageIdentity::new(fields(
        (LEAVES_PER_PAGE + 1) * LEAF_BYTES,
        Representation::Plaintext,
    ))
    .unwrap();
    let mut bytes = [0; BITMAP_BYTES];
    let page = CandidateBitmapPage::new(identity, 0, &mut bytes).unwrap();
    assert_eq!(
        page.has_integrity_mark(LEAVES_PER_PAGE),
        Err(StorageError::PageMismatch)
    );
    drop(page);
    let page = CandidateBitmapPage::new(identity, 1, &mut bytes).unwrap();
    assert_eq!(page.has_integrity_mark(0), Err(StorageError::PageMismatch));
    assert_eq!(page.has_integrity_mark(LEAVES_PER_PAGE), Ok(false));
    drop(page);
    assert_eq!(
        CandidateBitmapPage::new(identity, u64::MAX, &mut bytes).err(),
        Some(StorageError::Overflow)
    );
    let empty = StorageIdentity::new(fields(0, Representation::Plaintext)).unwrap();
    assert_eq!(
        CandidateBitmapPage::new(empty, 0, &mut bytes).err(),
        Some(StorageError::Bounds)
    );
    assert_eq!(
        verify_leaf_view(&empty, 0, &[], &[]).err(),
        Some(StorageError::Proof(ProofError::Index))
    );
}
#[test]
fn zero_filled_bitmap_is_not_zero_filled_verified_data() {
    let bytes = [0u8; 4];
    let identity = one_leaf(&bytes, Representation::Plaintext, 0);
    let mut payload = [0; BITMAP_BYTES];
    let mut page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    assert_eq!(page.has_integrity_mark(0), Ok(false));
    assert!(verify_leaf_view(&identity, 0, &[], &[]).is_err());
    assert_eq!(page.has_integrity_mark(0), Ok(false));
    let actual = verify_leaf_view(&identity, 0, &bytes, &[]).unwrap();
    page.record_integrity(&actual).unwrap();
    assert_eq!(page.has_integrity_mark(0), Ok(true));
}
