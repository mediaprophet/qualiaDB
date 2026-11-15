use super::*;
use crate::crypto::network::{kem, x25519::X25519Secret};
use std::cell::Cell;
fn recipient_fields<'a>(context: D, ml: &'a [u8; 1184], dh: &'a [u8; 32]) -> RecipientFields<'a> {
    RecipientFields {
        descriptor: D([1; 48]),
        context,
        recipient: D([2; 48]),
        publisher: D([3; 48]),
        service: D([4; 48]),
        purpose: D([5; 48]),
        instrument: D([6; 48]),
        scope: D([7; 48]),
        operation: [8; 16],
        closure: D([9; 48]),
        policy: D([10; 48]),
        consumer: D([11; 48]),
        source: D([12; 48]),
        source_length: 64,
        key_epoch: 1,
        not_before: 0,
        expires: now() + 300,
        ml_public: ml,
        dh_public: dh,
    }
}
fn recipient_request<'a>(
    record: RecipientRecord<'a>,
    context: Context<'a>,
) -> RecipientUnwrapRequest<'a> {
    RecipientUnwrapRequest {
        ml_key_id: "ml_private",
        dh_key_id: "dh_private",
        ml_policy_id: "ml_private",
        dh_policy_id: "dh_private",
        record,
        context,
        grant: grant(KeyPurpose::ObjectRecipient),
        access: access(),
    }
}
#[test]
fn actual_recipient_keys_policy_and_fences() {
    let mut m = manager();
    let (ml, ml_public) = m
        .generate_key_pair(
            "ml".into(),
            KeyType::Private,
            KeyAlgorithm::Kyber,
            SecurityLevel::High,
        )
        .unwrap();
    let (dh, dh_public) = m
        .generate_key_pair(
            "dh".into(),
            KeyType::Private,
            KeyAlgorithm::X25519,
            SecurityLevel::High,
        )
        .unwrap();
    m.store_key(ml).unwrap();
    m.store_key(dh).unwrap();
    let mut cb = [0; 164];
    encode_context(
        ContextFields {
            nonce: [1; 32],
            scope_token: [2; 32],
            generation: 0,
            key_epoch: 1,
            predecessor: D::ZERO,
            objects: 0,
            metadata_length: 1,
        },
        &[],
        &mut cb,
    )
    .unwrap();
    let context = Context::parse(&cb).unwrap();
    let mut rb = [0; RECIPIENT_BYTES];
    encode_recipient(
        recipient_fields(
            context.id(),
            ml_public.key_data.as_slice().try_into().unwrap(),
            dh_public.key_data.as_slice().try_into().unwrap(),
        ),
        &mut rb,
    )
    .unwrap();
    let record = RecipientRecord::parse(&rb).unwrap();
    let control = ManagedOperationControl::new(1).unwrap();
    assert!(
        m.begin_recipient_unwrap(recipient_request(record, context), &control, &now)
            .is_err()
    );
    m.install_managed_signing_policy(policy("ml_private", vec![KeyOperation::Decrypt]));
    m.install_managed_signing_policy(policy("dh_private", vec![KeyOperation::Derive]));
    for mode in 0..5 {
        let mut r = recipient_request(record, context);
        match mode {
            0 => r.access.authenticated_methods = &[],
            1 => r.access.ip_address = "foreign",
            2 => r.ml_key_id = "dh_private",
            3 => r.ml_policy_id = "absent",
            _ => r.grant.authority_generation = 2,
        }
        assert!(m.begin_recipient_unwrap(r, &control, &now).is_err());
    }
    let mut expired = policy("ml_private", vec![KeyOperation::Decrypt]);
    expired.time_restrictions.end_date = Some(now() - 1);
    m.install_managed_signing_policy(expired);
    assert!(
        m.begin_recipient_unwrap(recipient_request(record, context), &control, &now)
            .is_err()
    );
    m.install_managed_signing_policy(policy("ml_private", vec![KeyOperation::Decrypt]));
    let (ss, ct) = kem::encapsulate(record.fields().ml_public).unwrap();
    let ephemeral = X25519Secret::from_bytes([42; 32]);
    let ep = ephemeral.public();
    let ds = ephemeral.diffie_hellman(record.fields().dh_public).unwrap();
    let wc = WrapContext::new(record, 1, &ep, &ct).unwrap();
    let master = [37; 32];
    let mut wb = [0; WRAP_BYTES];
    seal_master_raw(wc, &ss, &ds, &master, &mut wb).unwrap();
    let scope = ObjectScope::new(context.id(), 0, 0, 1, 1).unwrap();
    let mut ciphertext = [91];
    let mut tag = [0; 16];
    seal_leaf_raw(&master, scope, 0, &mut ciphertext, &mut tag).unwrap();
    let mut opened = m
        .begin_recipient_unwrap(recipient_request(record, context), &control, &now)
        .unwrap()
        .unwrap(Wrap::parse(&wb).unwrap())
        .unwrap();
    let mut plain = ciphertext;
    opened.decrypt_next(&mut plain, &tag).unwrap();
    assert_eq!(plain, [91]);
    drop(opened);
    for mode in 0..4 {
        let control = ManagedOperationControl::new(1).unwrap();
        let ticks = Cell::new(0);
        let clock = || {
            let n = ticks.get();
            ticks.set(n + 1);
            if mode == 2 && n >= 2 { u64::MAX } else { now() }
        };
        let op = m
            .begin_recipient_unwrap(recipient_request(record, context), &control, &clock)
            .unwrap();
        if mode == 0 {
            control.cancel().unwrap()
        }
        if mode == 1 {
            control.advance_generation(2).unwrap()
        }
        let mut corrupt = wb;
        if mode == 3 {
            corrupt[WRAP_BYTES - 1] ^= 1;
        }
        assert!(op.unwrap(Wrap::parse(&corrupt).unwrap()).is_err());
    }
    for mode in 0..2 {
        let control = ManagedOperationControl::new(1).unwrap();
        let mut opened = m
            .begin_recipient_unwrap(recipient_request(record, context), &control, &now)
            .unwrap()
            .unwrap(Wrap::parse(&wb).unwrap())
            .unwrap();
        let mut bad_tag = tag;
        if mode == 0 {
            control.cancel().unwrap()
        } else {
            bad_tag[0] ^= 1
        }
        let mut plain = ciphertext;
        assert!(opened.decrypt_next(&mut plain, &bad_tag).is_err());
        assert_eq!(plain, [0]);
        assert!(opened.poisoned);
    }
    let mut changed = rb;
    changed[RECIPIENT_BYTES - 1] ^= 1;
    let changed = RecipientRecord::parse(&changed).unwrap();
    let op = m
        .begin_recipient_unwrap(recipient_request(changed, context), &control, &now)
        .unwrap();
    assert!(op.unwrap(Wrap::parse(&wb).unwrap()).is_err());
}
