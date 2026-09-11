//! Spec §20 vector checks for the Qualia Identifier runtime.
//!
//! Vector 1 unsigned digest, RFC 8032 proof, and §16 git object id of the
//! **signed** QCDE-1 document. `create` stores those signed bytes, so
//! `GitObjectStore::object_id_of` matches the published git id.

#![cfg(test)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::document::{
    encode_signed, encode_unsigned, genesis_digest, proof_message, sha256_32, signed_git_object_id,
    AkaEntry, QiDocument, MAX_CANONICAL, MAX_SIGNED,
};
use super::document_decode::ingest_unsigned_json;
use super::git_object::GitObjectStore;
use super::id::{format_did, DidQi, MAX_DID_TEXT};
use super::method::{create, deactivate, read, update};
use super::service::{CscpMailbox, Disclosure, RelayHint};
use super::QiError;

fn hex32(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (n(s[i * 2]) << 4) | n(s[i * 2 + 1]);
        i += 1;
    }
    out
}

fn hex64(s: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        out[i] = (n(s[i * 2]) << 4) | n(s[i * 2 + 1]);
        i += 1;
    }
    out
}

fn n(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

/// RFC 8032 test 1 secret (matches published public `d75a98…`).
fn rfc8032_sk() -> [u8; 32] {
    hex32(b"9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
}

fn rfc8032_pk() -> [u8; 32] {
    hex32(b"d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
}

fn vector1_unsigned_doc() -> (DidQi, QiDocument) {
    let mut genesis = QiDocument::empty();
    genesis.controller_pk = rfc8032_pk();
    genesis.services[0] = CscpMailbox::mailbox(
        sha256_32(b"did:qi:test-vector:contact-key-0"),
        Disclosure::ApprovedRelaysOnly,
    );
    genesis.services[0].hints[0] =
        RelayHint::new(b"invite-1", sha256_32(b"did:qi:test-vector:operator-0")).unwrap();
    genesis.services[0].hint_count = 1;
    genesis.service_count = 1;
    let id = DidQi(genesis_digest(&genesis).unwrap());
    let mut published = genesis;
    published.generation = 0;
    (id, published)
}

#[test]
fn vector1_unsigned_proof_and_git_id() {
    let (id, doc) = vector1_unsigned_doc();
    let mut did = [0u8; MAX_DID_TEXT];
    let n = format_did(&id, &mut did).unwrap();
    assert_eq!(&did[..n], b"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu");
    assert_eq!(
        SigningKey::from_bytes(&rfc8032_sk())
            .verifying_key()
            .to_bytes(),
        rfc8032_pk()
    );

    let mut unsigned = [0u8; MAX_CANONICAL];
    let un = encode_unsigned(&id, &doc, &mut unsigned).unwrap();
    let digest = sha256_32(&unsigned[..un]);
    assert_eq!(
        digest,
        hex32(b"73432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c6744")
    );

    let mut msg = [0u8; 64];
    let mn = proof_message(&digest, &mut msg).unwrap();
    let sig = SigningKey::from_bytes(&rfc8032_sk()).sign(&msg[..mn]);
    let published_sig = hex64(b"fd8b12c03ff43f40de5521b0a14b53e82f6c62c054d9454141c01dd26834c5455e46416148758fd76d8b21a01d230dbe4efdd08da351239abaae987698a34209");
    assert_eq!(sig.to_bytes(), published_sig);
    let vk = VerifyingKey::from_bytes(&rfc8032_pk()).unwrap();
    vk.verify(&msg[..mn], &Signature::from_bytes(&published_sig))
        .unwrap();
    assert_eq!(
        signed_git_object_id(&id, &doc, &sig.to_bytes()).unwrap(),
        hex32(b"18962a639bdbe09b3998105716944d34b9da606d2d945701e6d68eff90aacddd")
    );

    let mut signed = [0u8; MAX_SIGNED];
    let sn = encode_signed(&id, &doc, &sig.to_bytes(), &mut signed).unwrap();
    assert_eq!(sn, 1550);

    let mut store = GitObjectStore::new();
    let stored = create(&mut store, &rfc8032_sk(), &doc).unwrap();
    assert_eq!(stored, id);
    assert_eq!(
        store.object_id_of(&id).unwrap(),
        hex32(b"18962a639bdbe09b3998105716944d34b9da606d2d945701e6d68eff90aacddd")
    );
}

#[test]
fn vector2_hostname_alias_and_stale_generation() {
    let mut store = GitObjectStore::new();
    let (_id, doc) = vector1_unsigned_doc();
    let id = create(&mut store, &rfc8032_sk(), &doc).unwrap();
    let mut next = doc;
    next.aka[0] = AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
    next.aka_count = 1;
    next.has_hostname_alias = true;
    next.hostname_did_web = AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
    let g1 = update(&mut store, &rfc8032_sk(), &id, &next).unwrap();
    assert_eq!(g1, 1);
    let mut out = QiDocument::empty();
    read(&store, &id, &mut out).unwrap();
    assert_eq!(out.generation, 1);
    assert!(out.has_hostname_alias);
    assert_eq!(
        &out.hostname_did_web.bytes[..out.hostname_did_web.len as usize],
        b"did:web:example.invalid"
    );
    assert_eq!(out.aka_count, 1);
    assert!(out.has_previous);
    assert_eq!(out.service_count, 1);
    assert_eq!(
        out.services[0].contact_key,
        sha256_32(b"did:qi:test-vector:contact-key-0")
    );
    assert_eq!(out.services[0].hint_count, 1);
    let hid_len = out.services[0].hints[0].hint_id_len as usize;
    assert_eq!(&out.services[0].hints[0].hint_id[..hid_len], b"invite-1");
    assert!(!out.services[0].public_dht);
    let digest = super::document::unsigned_digest(&id, &out).unwrap();
    assert_eq!(
        digest,
        hex32(b"6180c97eb650da657c0e3801d421da7d6e9f0c39da568cb9fa0311d1d07dd88a")
    );
    assert_eq!(
        super::method::read_generation(&store, &id, 0, &mut QiDocument::empty()),
        Err(QiError::StaleGeneration)
    );
}

#[test]
fn vector3_deactivate_clears_locators_and_matches_git_id() {
    let mut store = GitObjectStore::new();
    let (_id, doc) = vector1_unsigned_doc();
    let id = create(&mut store, &rfc8032_sk(), &doc).unwrap();
    let mut next = doc;
    next.aka[0] = AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
    next.aka_count = 1;
    next.has_hostname_alias = true;
    next.hostname_did_web = AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
    update(&mut store, &rfc8032_sk(), &id, &next).unwrap();
    let g2 = deactivate(&mut store, &rfc8032_sk(), &id).unwrap();
    assert_eq!(g2, 2);
    let mut out = QiDocument::empty();
    read(&store, &id, &mut out).unwrap();
    assert_eq!(out.generation, 2);
    assert!(out.deactivated);
    assert_eq!(out.service_count, 0);
    assert_eq!(out.aka_count, 0);
    assert!(!out.has_hostname_alias);
    assert!(out.has_previous);
    let digest = super::document::unsigned_digest(&id, &out).unwrap();
    assert_eq!(
        digest,
        hex32(b"03d83bce6f4abfcf88fc48da535e407d61090f0a95678a02e02cae50ecef311d")
    );
    let mut msg = [0u8; 64];
    let mn = proof_message(&digest, &mut msg).unwrap();
    let sig = SigningKey::from_bytes(&rfc8032_sk()).sign(&msg[..mn]);
    let published_sig = hex64(b"3c9de6e392eef7b1c5a367e705cbab7b93c6e34f56b635827a4251f8402bb1f10e3b7be11a7e557cae88470814b3b6edf5f017396d193532cba03480ada7cb03");
    assert_eq!(sig.to_bytes(), published_sig);
    let mut signed = [0u8; MAX_SIGNED];
    let sn = encode_signed(&id, &out, &sig.to_bytes(), &mut signed).unwrap();
    assert_eq!(sn, 1222);
    assert_eq!(
        store.object_id_of(&id).unwrap(),
        hex32(b"834846958cca9eb34fd2abcf559372f703fbe87da7d37f7b0f876673bce21860")
    );
    assert_eq!(
        signed_git_object_id(&id, &out, &sig.to_bytes()).unwrap(),
        hex32(b"834846958cca9eb34fd2abcf559372f703fbe87da7d37f7b0f876673bce21860")
    );
    assert_eq!(
        update(&mut store, &rfc8032_sk(), &id, &doc),
        Err(QiError::Deactivated)
    );
}

#[test]
fn vector4_bytes_not_stored() {
    let json = br#"{"@context":["https://www.w3.org/ns/did/v1","https://w3id.org/security/suites/ed25519-2020/v1","https://webizen.network/ns/did-qi/v1"],"assertionMethod":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"authentication":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"capabilityInvocation":["did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0"],"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","qi":{"createdUnix":1788998400,"deactivated":false,"generation":0},"service":[{"id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#mailbox","serviceEndpoint":{"contactKeyMultibase":"zX8CKVGdsbqkmGWCUgEaiFBMxvRuaTzZM1ESdfwnrEbk","disclosure":"ApprovedRelaysOnly","generation":0,"ipv6":"2001:db8::1","locatorKind":"direct","port":4242,"publicDht":false,"relayHints":[]},"type":"CscpMailbox"}],"verificationMethod":[{"controller":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu","id":"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu#key-0","publicKeyMultibase":"z6MktwupdmLXVVqTzCw4i46r4uGyosGXRnR3XjN4Zq7oMMsw","type":"Ed25519VerificationKey2020"}]}"#;
    let mut out = QiDocument::empty();
    assert_eq!(
        ingest_unsigned_json(json, &mut out),
        Err(QiError::DirectLocatorForbidden)
    );
    let mut store = GitObjectStore::new();
    let mut doc = QiDocument::empty();
    doc.controller_pk = rfc8032_pk();
    doc.services[0] = CscpMailbox::direct(
        sha256_32(b"did:qi:test-vector:contact-key-0"),
        Disclosure::ApprovedRelaysOnly,
    );
    doc.service_count = 1;
    assert_eq!(
        create(&mut store, &rfc8032_sk(), &doc),
        Err(QiError::DirectLocatorForbidden)
    );
}
