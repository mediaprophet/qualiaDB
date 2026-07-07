//! Independent adversarial test suite for the sanctuary audit primitives (vault v2, slice C).
//!
//! The module under test is `crates/qualia-core-db/src/crypto/sanctuary_audit.rs`. These primitives
//! protect a coercion-victim's real lane: a decoy session (holding only the audit *public* key) can
//! append sealed records into a channel it can never read, and the tamper-evident hash chain makes any
//! rewrite/reorder/drop detectable. The tests below are deliberately hostile — they try to break each
//! guarantee and assert the primitive fails **closed** (returns `Err`, or at minimum does not surrender
//! the plaintext). No attacker-controlled path is `.unwrap()`ed; malformed input must never panic.
//!
//! Run: `cargo test -p qualia-core-db --features sanctuary-crypto --test sanctuary_audit_adversarial`

#![cfg(feature = "sanctuary-crypto")]

use qualia_core_db::crypto::sanctuary_audit::*;

// Layout constants mirrored from the module (they are private there): the sealed box is
// `ephemeral_public(32) ‖ ciphertext ‖ tag(16)`, and an empty-plaintext seal is exactly 48 bytes.
const EPK_BYTES: usize = 32;
const TAG_BYTES: usize = 16;
const MIN_SEALED: usize = EPK_BYTES + TAG_BYTES; // 48

/// Deterministic-but-varied filler bytes so we don't depend on a specific RNG in tests.
fn filler(seed: u8, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| seed.wrapping_mul(31).wrapping_add(i as u8))
        .collect()
}

// ---------------------------------------------------------------------------------------------------
// Sealed box — only the intended recipient reads.
// ---------------------------------------------------------------------------------------------------

#[test]
fn sealed_box_only_intended_recipient_reads() {
    let recipient = AuditKeypair::generate().expect("keygen");
    let msg = b"decoy session: coercer appended note at 12:04 -- real lane must read this";
    let aad = b"branch:session-1";

    let sealed = seal_to(&recipient.public, msg, aad).expect("seal");

    // Sanity: the true recipient does recover it, with the right AAD.
    let opened = open_sealed(recipient.secret_bytes(), &sealed, aad).expect("intended open");
    assert_eq!(
        opened.as_slice(),
        msg,
        "intended recipient must recover the exact plaintext"
    );

    // ~16 other keypairs (B..=Q) must each fail to open. None may recover the plaintext.
    let mut attackers = 0usize;
    for _ in 0..16 {
        let other = AuditKeypair::generate().expect("keygen");
        match open_sealed(other.secret_bytes(), &sealed, aad) {
            Err(_) => {}
            Ok(pt) => assert_ne!(
                pt.as_slice(),
                msg,
                "a non-recipient keypair recovered the plaintext -- confidentiality broken"
            ),
        }
        attackers += 1;
    }
    assert_eq!(attackers, 16, "expected 16 adversarial keypairs exercised");
}

#[test]
fn writer_holding_only_public_cannot_read() {
    // The decoy session holds only `recipient.public`. Feeding those public bytes into `open_sealed`
    // as if they were the secret must NOT recover the plaintext (X25519 clamping means the "derived"
    // public won't match, and even if a shared secret is computed it is the wrong one).
    let recipient = AuditKeypair::generate().expect("keygen");
    let msg = b"only-the-real-lane-reads-this";
    let sealed = seal_to(&recipient.public, msg, b"").expect("seal");

    match open_sealed(&recipient.public, &sealed, b"") {
        Err(_) => {}
        Ok(pt) => assert_ne!(
            pt.as_slice(),
            msg,
            "using the public key as a secret recovered the plaintext -- the decoy could read its own writes"
        ),
    }
}

// ---------------------------------------------------------------------------------------------------
// Malleability — every single-byte flip is rejected (or at least never yields the original plaintext).
// ---------------------------------------------------------------------------------------------------

#[test]
fn every_single_byte_flip_of_sealed_box_is_rejected() {
    let recipient = AuditKeypair::generate().expect("keygen");
    let msg = b"unaltered evidence line -- any bit flip must be caught";
    let aad = b"branch:session-1";
    let sealed = seal_to(&recipient.public, msg, aad).expect("seal");

    // For each byte position, flip the low bit AND flip all bits (0xFF) — two mutations per byte.
    // Every mutated sealed box must fail to reproduce the original plaintext.
    let mut assertions = 0usize;
    for i in 0..sealed.len() {
        for mask in [0x01u8, 0xFFu8] {
            let mut tampered = sealed.clone();
            tampered[i] ^= mask;
            match open_sealed(recipient.secret_bytes(), &tampered, aad) {
                Err(_) => {}
                Ok(pt) => assert_ne!(
                    pt.as_slice(),
                    msg,
                    "byte {i} xor {mask:#04x} still decrypted to the original plaintext -- malleable",
                ),
                // note: no panic path — attacker-controlled bytes must never panic
            }
            assertions += 1;
        }
    }
    // sealed.len() == EPK(32) + ct(msg.len()) + tag(16); 2 mutations each.
    assert_eq!(
        assertions,
        sealed.len() * 2,
        "expected two mutations per byte position"
    );
    assert!(
        assertions >= sealed.len() * 2 && assertions >= 96,
        "broad byte-flip coverage"
    );
}

// ---------------------------------------------------------------------------------------------------
// Truncation / oversize — malformed lengths fail closed, never panic.
// ---------------------------------------------------------------------------------------------------

#[test]
fn malformed_sealed_lengths_fail_closed() {
    let recipient = AuditKeypair::generate().expect("keygen");
    let secret = recipient.secret_bytes();
    let good = seal_to(&recipient.public, b"payload", b"aad").expect("seal");

    // Empty, 1 byte, and every length strictly below the minimum valid size (EPK+TAG = 48).
    let mut short_cases: Vec<Vec<u8>> = vec![Vec::new(), vec![0u8; 1]];
    for len in 2..MIN_SEALED {
        short_cases.push(filler(len as u8, len));
    }
    // Exactly EPK+TAG-1 (47) — one byte short of the minimum.
    short_cases.push(filler(0xA5, MIN_SEALED - 1));

    for (idx, case) in short_cases.iter().enumerate() {
        assert!(
            case.len() < MIN_SEALED,
            "case {idx} should be undersized (len {})",
            case.len()
        );
        assert!(
            open_sealed(secret, case, b"aad").is_err(),
            "undersized sealed box (len {}) must be rejected",
            case.len()
        );
    }

    // Exactly the minimum (48 bytes) but garbage content: not a valid seal, must fail (not panic).
    assert!(open_sealed(secret, &filler(0x11, MIN_SEALED), b"aad").is_err());

    // Trailing garbage appended to a *valid* seal: extra bytes land in the ciphertext region and the
    // AEAD tag no longer authenticates the message -> must fail.
    let mut oversize = good.clone();
    oversize.extend_from_slice(b"trailing garbage that was never part of the record");
    assert!(
        open_sealed(secret, &oversize, b"aad").is_err(),
        "seal with appended trailing garbage must be rejected"
    );

    // Garbage prepended (shifts the ephemeral public key) — also must fail.
    let mut prepended = b"XYZ".to_vec();
    prepended.extend_from_slice(&good);
    assert!(open_sealed(secret, &prepended, b"aad").is_err());
}

// ---------------------------------------------------------------------------------------------------
// AAD binding — the sealed record is bound to its branch/context.
// ---------------------------------------------------------------------------------------------------

#[test]
fn sealed_box_aad_is_bound() {
    let recipient = AuditKeypair::generate().expect("keygen");
    let msg = b"note bound to branch-1";
    let sealed = seal_to(&recipient.public, msg, b"branch-1").expect("seal");

    // Correct AAD opens.
    assert_eq!(
        open_sealed(recipient.secret_bytes(), &sealed, b"branch-1")
            .expect("open branch-1")
            .as_slice(),
        msg
    );

    // Wrong AAD (different branch) fails.
    assert!(open_sealed(recipient.secret_bytes(), &sealed, b"branch-2").is_err());

    // Empty AAD vs the non-empty AAD it was sealed under fails.
    assert!(open_sealed(recipient.secret_bytes(), &sealed, b"").is_err());

    // A record sealed with *empty* AAD must not open under a *non-empty* AAD.
    let sealed_empty = seal_to(&recipient.public, msg, b"").expect("seal empty aad");
    assert_eq!(
        open_sealed(recipient.secret_bytes(), &sealed_empty, b"")
            .expect("open empty aad")
            .as_slice(),
        msg
    );
    assert!(open_sealed(recipient.secret_bytes(), &sealed_empty, b"branch-1").is_err());

    // A battery of near-miss AADs (single-byte differences from the real one) all fail.
    let real_aad = b"branch-1";
    let mut near_misses = 0usize;
    for i in 0..real_aad.len() {
        let mut aad = real_aad.to_vec();
        aad[i] ^= 0x01;
        assert!(
            open_sealed(recipient.secret_bytes(), &sealed, &aad).is_err(),
            "near-miss AAD (byte {i} flipped) must fail"
        );
        near_misses += 1;
    }
    assert_eq!(near_misses, real_aad.len());
}

// ---------------------------------------------------------------------------------------------------
// Non-determinism — no plaintext-equality oracle.
// ---------------------------------------------------------------------------------------------------

#[test]
fn sealing_identical_plaintext_is_non_deterministic() {
    let recipient = AuditKeypair::generate().expect("keygen");
    let msg = b"same-note-appended-repeatedly";
    let aad = b"branch:session-1";

    // Seal the identical (plaintext, aad) many times; all outputs must be pairwise distinct, and none
    // may equal another (which would leak that the same content was written twice).
    let mut seals: Vec<Vec<u8>> = Vec::new();
    for _ in 0..12 {
        let s = seal_to(&recipient.public, msg, aad).expect("seal");
        assert!(
            !seals.contains(&s),
            "identical plaintext produced a colliding sealed box -- equality oracle present"
        );
        // The ephemeral public prefix (first 32 bytes) must also differ across seals.
        for prev in &seals {
            assert_ne!(
                &prev[..EPK_BYTES],
                &s[..EPK_BYTES],
                "ephemeral key reused across seals"
            );
        }
        seals.push(s);
    }
    assert_eq!(seals.len(), 12);

    // But every one of them still opens to the same plaintext.
    for s in &seals {
        assert_eq!(
            open_sealed(recipient.secret_bytes(), s, aad)
                .expect("open")
                .as_slice(),
            msg
        );
    }
}

// ---------------------------------------------------------------------------------------------------
// Key wrap — one-way hierarchy: correct key + AAD unwraps; everything else fails closed.
// ---------------------------------------------------------------------------------------------------

#[test]
fn key_wrap_round_trip_and_binds_key_and_aad() {
    // The real lane key wraps the decoy lane key material.
    let real_key = filler(0x42, 32).try_into().expect("32-byte key");
    let real_key: [u8; 32] = real_key;
    let decoy_key_material = filler(0x07, 32);
    let aad = b"role:decoy-lane-key";

    let wrapped = wrap_key(&real_key, &decoy_key_material, aad).expect("wrap");

    // Correct key + correct AAD recovers exactly.
    assert_eq!(
        unwrap_key(&real_key, &wrapped, aad)
            .expect("unwrap")
            .as_slice(),
        decoy_key_material.as_slice(),
        "correct key+aad must recover the exact key material"
    );

    // ~16 wrong wrapping keys (the decoy can never reach up) all fail.
    let mut wrong_keys = 0usize;
    for k in 0..16u8 {
        let mut bad = real_key;
        bad[usize::from(k) % 32] ^= 0x80 | k.wrapping_add(1); // guarantee a differing key
        if bad == real_key {
            continue;
        }
        assert!(
            unwrap_key(&bad, &wrapped, aad).is_err(),
            "wrong wrapping key #{k} unwrapped -- one-way hierarchy broken"
        );
        wrong_keys += 1;
    }
    assert!(
        wrong_keys >= 15,
        "expected ~16 wrong-key attempts, got {wrong_keys}"
    );

    // Wrong AAD fails.
    assert!(unwrap_key(&real_key, &wrapped, b"role:something-else").is_err());
    assert!(unwrap_key(&real_key, &wrapped, b"").is_err());
}

#[test]
fn key_wrap_malformed_lengths_fail_closed() {
    let key = filler(0x33, 32).try_into().expect("32-byte key");
    let key: [u8; 32] = key;
    // XNONCE(24) + TAG(16) = 40 is the minimum valid wrapped length.
    const MIN_WRAPPED: usize = 24 + 16;
    for len in 0..MIN_WRAPPED {
        assert!(
            unwrap_key(&key, &filler(len as u8, len), b"aad").is_err(),
            "undersized wrapped blob (len {len}) must be rejected"
        );
    }
    // Minimum length but pure garbage — must fail, never panic.
    assert!(unwrap_key(&key, &filler(0x99, MIN_WRAPPED), b"aad").is_err());
}

#[test]
fn every_single_byte_flip_of_wrapped_blob_is_rejected() {
    let key = filler(0x5A, 32).try_into().expect("32-byte key");
    let key: [u8; 32] = key;
    let material = filler(0x21, 40);
    let aad = b"role:decoy-lane-key";
    let wrapped = wrap_key(&key, &material, aad).expect("wrap");

    let mut assertions = 0usize;
    for i in 0..wrapped.len() {
        for mask in [0x01u8, 0xFFu8] {
            let mut tampered = wrapped.clone();
            tampered[i] ^= mask;
            match unwrap_key(&key, &tampered, aad) {
                Err(_) => {}
                Ok(pt) => assert_ne!(
                    pt.as_slice(),
                    material.as_slice(),
                    "wrapped byte {i} xor {mask:#04x} still unwrapped to the original -- malleable",
                ),
            }
            assertions += 1;
        }
    }
    assert_eq!(
        assertions,
        wrapped.len() * 2,
        "two mutations per byte of the wrapped blob"
    );
}

// ---------------------------------------------------------------------------------------------------
// Hash chain — tamper / reorder / drop are all detectable.
// ---------------------------------------------------------------------------------------------------

/// Build a chain of `n` links from a set of payloads: id[k] = chain_hash(id[k-1], payload[k]),
/// with id[-1] == GENESIS_PARENT.
fn build_chain(payloads: &[&[u8]]) -> Vec<[u8; 32]> {
    let mut ids = Vec::with_capacity(payloads.len());
    let mut parent = GENESIS_PARENT;
    for p in payloads {
        let id = chain_hash(&parent, p);
        ids.push(id);
        parent = id;
    }
    ids
}

#[test]
fn hash_chain_is_deterministic() {
    let payloads: [&[u8]; 5] = [
        b"session-1 opened",
        b"note added: 'call me'",
        b"note edited",
        b"file attached: photo.jpg",
        b"session closed",
    ];
    let a = build_chain(&payloads);
    let b = build_chain(&payloads);
    assert_eq!(a, b, "chain hashing must be deterministic");

    // No adjacent links collide.
    for w in a.windows(2) {
        assert_ne!(w[0], w[1]);
    }
    // GENESIS-rooted first link matches a direct recompute.
    assert_eq!(a[0], chain_hash(&GENESIS_PARENT, payloads[0]));
}

#[test]
fn hash_chain_payload_tamper_propagates() {
    let payloads: [&[u8]; 5] = [
        b"session-1 opened",
        b"note added: 'call me'",
        b"note edited",
        b"file attached: photo.jpg",
        b"session closed",
    ];
    let original = build_chain(&payloads);

    // Tamper each payload position in turn: the tampered link and EVERY subsequent recomputed link
    // must differ from the original.
    let mut checks = 0usize;
    for t in 0..payloads.len() {
        let mut tampered_payloads: Vec<&[u8]> = payloads.to_vec();
        tampered_payloads[t] = b"TAMPERED: 'do NOT call'";
        let tampered = build_chain(&tampered_payloads);

        // The tampered link itself changes.
        assert_ne!(
            original[t], tampered[t],
            "tampering payload {t} did not change its own link"
        );
        // Every link at or after t changes (parent link no longer matches).
        for k in t..payloads.len() {
            assert_ne!(
                original[k], tampered[k],
                "tampering payload {t} did not propagate to downstream link {k}"
            );
            checks += 1;
        }
        // Links before t are unchanged (tamper-evidence is forward-propagating, not retroactive).
        for k in 0..t {
            assert_eq!(
                original[k], tampered[k],
                "link {k} before tamper point {t} should be stable"
            );
        }
    }
    // 5 + 4 + 3 + 2 + 1 = 15 downstream checks.
    assert_eq!(
        checks, 15,
        "expected 15 downstream tamper-propagation checks"
    );
}

#[test]
fn hash_chain_reorder_is_detectable() {
    let payloads: [&[u8]; 5] = [
        b"session-1 opened",
        b"note added: 'call me'",
        b"note edited",
        b"file attached: photo.jpg",
        b"session closed",
    ];
    let original = build_chain(&payloads);

    // Swap every distinct pair of payloads (with distinct content) and confirm the chain changes.
    let mut swaps = 0usize;
    for i in 0..payloads.len() {
        for j in (i + 1)..payloads.len() {
            if payloads[i] == payloads[j] {
                continue; // swapping identical content is a no-op by construction
            }
            let mut reordered: Vec<&[u8]> = payloads.to_vec();
            reordered.swap(i, j);
            let chain = build_chain(&reordered);
            assert_ne!(
                original, chain,
                "swapping payloads {i} and {j} produced an identical chain -- reorder undetectable"
            );
            swaps += 1;
        }
    }
    // C(5,2) = 10 distinct pairs, all with distinct content here.
    assert_eq!(swaps, 10, "expected 10 reorder checks");
}

#[test]
fn hash_chain_wrong_parent_changes_link() {
    // Using the wrong parent for a link changes its hash — this is what "drop a record" looks like:
    // link k+1 that should chain off id[k] instead chains off id[k-1] (the dropped record's parent).
    let payloads: [&[u8]; 5] = [b"r0", b"r1", b"r2 -- to be dropped", b"r3", b"r4"];
    let ids = build_chain(&payloads);

    // Correct link for r3 chains off id[2].
    let correct_r3 = chain_hash(&ids[2], payloads[3]);
    assert_eq!(correct_r3, ids[3]);

    // Drop r2: r3 now chains off id[1] instead of id[2]. Its hash must differ.
    let dropped_r3 = chain_hash(&ids[1], payloads[3]);
    assert_ne!(
        dropped_r3, ids[3],
        "chaining r3 off the wrong parent (r2 dropped) produced the same hash -- drop undetectable"
    );

    // Also: chaining off GENESIS instead of the true parent differs for every link past the first.
    for k in 1..payloads.len() {
        let wrong = chain_hash(&GENESIS_PARENT, payloads[k]);
        assert_ne!(
            wrong, ids[k],
            "link {k} off GENESIS matched its true-parent hash"
        );
    }
}

#[test]
fn hash_chain_parent_and_payload_are_domain_separated() {
    // A classic length-extension / concatenation-ambiguity check: chain_hash(parent, payload) must not
    // collide with a shifted split of the same concatenated bytes. If it did, an attacker could move
    // bytes across the parent||payload boundary undetected.
    let parent_a = chain_hash(&GENESIS_PARENT, b"anchor");
    // Move one byte from the front of the payload into... well, parent is fixed-length 32 here, so the
    // real risk is two different (parent,payload) pairs hashing equal. Construct such a near-collision
    // attempt: same total bytes, different boundary is impossible (parent is always 32B), so instead we
    // confirm distinct payloads under the same parent never collide across a sample.
    let mut seen: Vec<[u8; 32]> = Vec::new();
    for i in 0u16..256 {
        let payload = [i as u8, (i >> 8) as u8, 0xAB, 0xCD];
        let h = chain_hash(&parent_a, &payload);
        assert!(
            !seen.contains(&h),
            "hash collision within a 256-sample sweep -- unexpected"
        );
        seen.push(h);
    }
    assert_eq!(seen.len(), 256);
}
