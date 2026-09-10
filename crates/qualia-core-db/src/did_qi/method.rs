//! Create / read / update / deactivate for `did:qi` (spec §11–§14).
//!
//! Controller signatures are Ed25519 over `did:qi:document:v1` || 0x00 ||
//! unsignedDigest. The DID is SHA-256 of the QCDE-1 genesis payload.
//! Generation 0 is Create. Further updates after deactivate fail closed.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::document::{
    encode_unsigned, genesis_digest, proof_message, sha256_32, QiDocument, MAX_CANONICAL,
};
use super::document_decode::decode_canonical;
use super::git_object::MAX_RECORD;
use super::id::DidQi;
use super::service::check_relay_only;
use super::{QiError, QiStore};

pub(crate) const FLAG_UTXO_TOMBSTONE: u8 = 0x01;
const SIG_LEN: usize = 64;
const HDR_LEN: usize = 1 + 8 + 2;

pub(crate) struct RecordParts<'a> {
    pub flags: u8,
    pub generation: u64,
    pub canonical: &'a [u8],
    pub signature: [u8; SIG_LEN],
}

pub(crate) fn pack_record(
    flags: u8,
    generation: u64,
    canonical: &[u8],
    signature: &[u8; SIG_LEN],
    out: &mut [u8],
) -> Result<usize, QiError> {
    let n = HDR_LEN + canonical.len() + SIG_LEN;
    if canonical.len() > MAX_CANONICAL || n > MAX_RECORD {
        return Err(QiError::CanonicalTooLarge);
    }
    if out.len() < n {
        return Err(QiError::BufferTooSmall);
    }
    out[0] = flags;
    out[1..9].copy_from_slice(&generation.to_be_bytes());
    let len = u16::try_from(canonical.len()).map_err(|_| QiError::CanonicalTooLarge)?;
    out[9..11].copy_from_slice(&len.to_be_bytes());
    out[11..11 + canonical.len()].copy_from_slice(canonical);
    out[11 + canonical.len()..n].copy_from_slice(signature);
    Ok(n)
}

pub(crate) fn unpack_record(buf: &[u8]) -> Result<RecordParts<'_>, QiError> {
    if buf.len() < HDR_LEN + SIG_LEN {
        return Err(QiError::MalformedDocument);
    }
    let flags = buf[0];
    let generation = u64::from_be_bytes(buf[1..9].try_into().unwrap());
    let canon_len = u16::from_be_bytes(buf[9..11].try_into().unwrap()) as usize;
    let need = HDR_LEN + canon_len + SIG_LEN;
    if buf.len() != need {
        return Err(QiError::MalformedDocument);
    }
    let canonical = &buf[HDR_LEN..HDR_LEN + canon_len];
    let mut signature = [0u8; SIG_LEN];
    signature.copy_from_slice(&buf[need - SIG_LEN..need]);
    Ok(RecordParts {
        flags,
        generation,
        canonical,
        signature,
    })
}

fn public_from_secret(sk: &[u8; 32]) -> [u8; 32] {
    SigningKey::from_bytes(sk).verifying_key().to_bytes()
}

fn bind_controller(doc: &QiDocument, sk: &[u8; 32]) -> Result<QiDocument, QiError> {
    let pk = public_from_secret(sk);
    let mut out = *doc;
    if out.controller_pk == [0u8; 32] {
        out.controller_pk = pk;
    } else if out.controller_pk != pk {
        return Err(QiError::ControllerMismatch);
    }
    Ok(out)
}

fn sign_unsigned(sk: &[u8; 32], unsigned: &[u8]) -> Result<[u8; SIG_LEN], QiError> {
    let digest = sha256_32(unsigned);
    let mut msg = [0u8; 64];
    let n = proof_message(&digest, &mut msg)?;
    Ok(SigningKey::from_bytes(sk).sign(&msg[..n]).to_bytes())
}

fn verify_unsigned(pk: &[u8; 32], unsigned: &[u8], signature: &[u8; SIG_LEN]) -> Result<(), QiError> {
    let digest = sha256_32(unsigned);
    let mut msg = [0u8; 64];
    let n = proof_message(&digest, &mut msg)?;
    let vk = VerifyingKey::from_bytes(pk).map_err(|_| QiError::InvalidKey)?;
    let sig = Signature::from_bytes(signature);
    vk.verify(&msg[..n], &sig).map_err(|_| QiError::BadSignature)
}

fn write_signed<S: QiStore>(
    store: &mut S,
    id: &DidQi,
    flags: u8,
    generation: u64,
    canonical: &[u8],
    signature: &[u8; SIG_LEN],
) -> Result<u64, QiError> {
    let mut rec = [0u8; MAX_RECORD];
    let n = pack_record(flags, generation, canonical, signature, &mut rec)?;
    store.put(id, generation, &rec[..n])?;
    Ok(generation)
}

fn sync_service_generation(doc: &mut QiDocument) {
    let mut i = 0;
    while i < doc.service_count as usize {
        doc.services[i].generation = doc.generation;
        i += 1;
    }
}

pub fn create<S: QiStore>(
    store: &mut S,
    controller_sk: &[u8; 32],
    doc: &QiDocument,
) -> Result<DidQi, QiError> {
    let mut doc = bind_controller(doc, controller_sk)?;
    if doc.deactivated {
        return Err(QiError::Deactivated);
    }
    doc.generation = 0;
    doc.has_previous = false;
    sync_service_generation(&mut doc);
    check_relay_only(doc.services_slice())?;
    let id = DidQi(genesis_digest(&doc)?);
    let mut canonical = [0u8; MAX_CANONICAL];
    let n = encode_unsigned(&id, &doc, &mut canonical)?;
    let sig = sign_unsigned(controller_sk, &canonical[..n])?;
    write_signed(store, &id, 0, 0, &canonical[..n], &sig)?;
    Ok(id)
}

pub fn read<S: QiStore>(store: &S, id: &DidQi, out: &mut QiDocument) -> Result<u64, QiError> {
    let mut rec = [0u8; MAX_RECORD];
    let (_slot_gen, n) = store.get(id, &mut rec)?;
    let parts = unpack_record(&rec[..n])?;
    decode_canonical(parts.canonical, out)?;
    verify_unsigned(&out.controller_pk, parts.canonical, &parts.signature)?;
    if parts.flags & FLAG_UTXO_TOMBSTONE != 0 && !out.deactivated {
        return Err(QiError::TombstoneRequiresDeactivate);
    }
    Ok(parts.generation)
}

/// Spec §12.7: an older signed generation presented as current is stale.
pub fn read_generation<S: QiStore>(
    store: &S,
    id: &DidQi,
    claimed: u64,
    out: &mut QiDocument,
) -> Result<u64, QiError> {
    let gen = read(store, id, out)?;
    if claimed != gen {
        return Err(QiError::StaleGeneration);
    }
    Ok(gen)
}

pub fn update<S: QiStore>(
    store: &mut S,
    controller_sk: &[u8; 32],
    id: &DidQi,
    doc: &QiDocument,
) -> Result<u64, QiError> {
    let mut current = QiDocument::empty();
    let gen = read(store, id, &mut current)?;
    if current.deactivated {
        return Err(QiError::Deactivated);
    }
    let mut doc = bind_controller(doc, controller_sk)?;
    if doc.deactivated {
        return Err(QiError::Deactivated);
    }
    if public_from_secret(controller_sk) != current.controller_pk {
        return Err(QiError::ControllerMismatch);
    }
    let (prev, _, _) = load_canonical_digest(store, id)?;
    doc.generation = current.generation + 1;
    doc.created_unix = current.created_unix;
    doc.has_previous = true;
    doc.previous_digest = prev;
    sync_service_generation(&mut doc);
    check_relay_only(doc.services_slice())?;
    let mut canonical = [0u8; MAX_CANONICAL];
    let n = encode_unsigned(id, &doc, &mut canonical)?;
    let sig = sign_unsigned(controller_sk, &canonical[..n])?;
    write_signed(store, id, 0, gen + 1, &canonical[..n], &sig)
}

pub fn deactivate<S: QiStore>(
    store: &mut S,
    controller_sk: &[u8; 32],
    id: &DidQi,
) -> Result<u64, QiError> {
    let mut current = QiDocument::empty();
    let gen = read(store, id, &mut current)?;
    if current.deactivated {
        return Err(QiError::Deactivated);
    }
    if public_from_secret(controller_sk) != current.controller_pk {
        return Err(QiError::ControllerMismatch);
    }
    let (prev, _, _) = load_canonical_digest(store, id)?;
    current.deactivated = true;
    current.service_count = 0;
    current.generation += 1;
    current.has_previous = true;
    current.previous_digest = prev;
    let mut canonical = [0u8; MAX_CANONICAL];
    let n = encode_unsigned(id, &current, &mut canonical)?;
    let sig = sign_unsigned(controller_sk, &canonical[..n])?;
    write_signed(store, id, 0, gen + 1, &canonical[..n], &sig)
}

pub(crate) fn load_canonical_digest<S: QiStore>(
    store: &S,
    id: &DidQi,
) -> Result<([u8; 32], u64, u8), QiError> {
    let mut rec = [0u8; MAX_RECORD];
    let (_, n) = store.get(id, &mut rec)?;
    let parts = unpack_record(&rec[..n])?;
    Ok((sha256_32(parts.canonical), parts.generation, parts.flags))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::document::{genesis_digest, AkaEntry};
    use super::super::git_object::GitObjectStore;
    use super::super::service::{CscpMailbox, Disclosure};
    use super::super::{format_did, parse_did};

    fn sk() -> [u8; 32] {
        [7u8; 32]
    }

    fn sample_doc() -> QiDocument {
        let mut doc = QiDocument::empty();
        doc.services[0] = CscpMailbox::mailbox(public_from_secret(&sk()), Disclosure::ApprovedRelaysOnly);
        doc.service_count = 1;
        doc
    }

    #[test]
    fn create_then_read_round_trip() {
        let mut store = GitObjectStore::new();
        let doc = sample_doc();
        let id = create(&mut store, &sk(), &doc).unwrap();
        let mut did = [0u8; 64];
        let n = format_did(&id, &mut did).unwrap();
        assert!(did[..n].starts_with(b"did:qi:z"));
        assert_eq!(parse_did(&did[..n]).unwrap(), id);
        let mut out = QiDocument::empty();
        let gen = read(&store, &id, &mut out).unwrap();
        assert_eq!(gen, 0);
        assert_eq!(out.controller_pk, public_from_secret(&sk()));
        assert!(!out.deactivated);
        let mut bound = sample_doc();
        bound.controller_pk = public_from_secret(&sk());
        assert_eq!(genesis_digest(&bound).unwrap(), id.0);
    }

    #[test]
    fn update_bumps_generation_old_is_stale() {
        let mut store = GitObjectStore::new();
        let mut doc = sample_doc();
        let id = create(&mut store, &sk(), &doc).unwrap();
        doc.aka[0] = AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
        doc.aka_count = 1;
        let g1 = update(&mut store, &sk(), &id, &doc).unwrap();
        assert_eq!(g1, 1);
        let mut out = QiDocument::empty();
        assert_eq!(read(&store, &id, &mut out).unwrap(), 1);
        assert_eq!(out.generation, 1);
        assert_eq!(out.aka_count, 1);
        assert_eq!(
            &out.aka[0].bytes[..out.aka[0].len as usize],
            b"did:web:example.invalid"
        );
        assert!(out.has_previous);
        assert_eq!(out.service_count, 1);
        assert_eq!(out.services[0].contact_key, public_from_secret(&sk()));
        assert!(store.is_stale(&id, 0).unwrap());
        assert!(!store.is_stale(&id, 1).unwrap());
        let mut stale = QiDocument::empty();
        assert_eq!(
            read_generation(&store, &id, 0, &mut stale),
            Err(QiError::StaleGeneration)
        );
        assert_eq!(read_generation(&store, &id, 1, &mut stale).unwrap(), 1);
    }

    #[test]
    fn deactivate_then_update_fails_closed() {
        let mut store = GitObjectStore::new();
        let doc = sample_doc();
        let id = create(&mut store, &sk(), &doc).unwrap();
        let g = deactivate(&mut store, &sk(), &id).unwrap();
        assert_eq!(g, 1);
        let mut out = QiDocument::empty();
        read(&store, &id, &mut out).unwrap();
        assert!(out.deactivated);
        assert_eq!(
            update(&mut store, &sk(), &id, &doc),
            Err(QiError::Deactivated)
        );
        assert_eq!(
            deactivate(&mut store, &sk(), &id),
            Err(QiError::Deactivated)
        );
    }

    #[test]
    fn empty_store_read_fails_closed() {
        let store = GitObjectStore::new();
        let mut out = QiDocument::empty();
        assert_eq!(
            read(&store, &DidQi([4u8; 32]), &mut out),
            Err(QiError::NotFound)
        );
    }

    #[test]
    fn create_relay_only_direct_locator_rejected() {
        let mut store = GitObjectStore::new();
        let mut doc = sample_doc();
        doc.services[0] = CscpMailbox::direct([1u8; 32], Disclosure::ApprovedRelaysOnly);
        assert_eq!(
            create(&mut store, &sk(), &doc),
            Err(QiError::DirectLocatorForbidden)
        );
    }

    #[test]
    fn wrong_controller_cannot_update() {
        let mut store = GitObjectStore::new();
        let doc = sample_doc();
        let id = create(&mut store, &sk(), &doc).unwrap();
        assert_eq!(
            update(&mut store, &[9u8; 32], &id, &doc),
            Err(QiError::ControllerMismatch)
        );
    }
}
