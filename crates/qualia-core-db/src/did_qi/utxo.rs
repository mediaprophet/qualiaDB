//! Bitcoin-family UTXO attestation for Qualia Identifier documents (spec §17).
//!
//! Parses OP_RETURN commitments from **caller-supplied** transaction bytes.
//! Live: `OP_RETURN 0x23 0x51 0x49 0x01 || unsignedDigest`.
//! Tombstone: `OP_RETURN 0x23 0x51 0x49 0xFF || 32 zero octets`.
//! No Chronik, no node, no network. Chain id is CAIP-2 `bip122:<32-hex>`.

use super::document::{sha256_32, QiDocument};
use super::id::DidQi;
use super::method::{load_canonical_digest, read};
use super::{QiError, QiStore};

pub const OP_RETURN: u8 = 0x6a;
pub const QI_MAGIC: [u8; 2] = [0x51, 0x49];
pub const LIVE_KIND: u8 = 0x01;
/// Registered tombstone kind in the 35-byte OP_RETURN push (`0xFF`).
pub const TOMBSTONE_OPCODE: u8 = 0xFF;
pub const MAX_CAIP2: usize = 72;

/// Bitcoin mainnet genesis, first 16 bytes of block hash (CAIP-2 `bip122`).
pub const CONSTITUTION_MAINNET: &[u8] = b"bip122:000000000019d6689c085ae165831e93";
/// Bitcoin testnet3 genesis identifier (distinct ledger, same DID).
pub const CONSTITUTION_TESTNET: &[u8] = b"bip122:000000000933ea01ad0ee984209779ba";

/// True when `chain` is an admitted BIP-122 constitution identifier (spec §17.1).
pub fn chain_admitted(chain: &[u8]) -> bool {
    chain == CONSTITUTION_MAINNET || chain == CONSTITUTION_TESTNET
}

/// Parse then admit. Well-formed but unlisted `bip122:` ids are `unsupported_chain`.
pub fn admit_chain(chain: &[u8]) -> Result<ChainId, QiError> {
    let id = parse_chain_id(chain)?;
    if !chain_admitted(id.as_bytes()) {
        return Err(QiError::UnsupportedChain);
    }
    Ok(id)
}

/// Bitcoin RPC display encoding of a 32-byte SHA-256d digest (byte-reversed hex).
pub fn txid_display(digest: &[u8; 32]) -> [u8; 64] {
    let mut out = [0u8; 64];
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut i = 0;
    while i < 32 {
        let b = digest[31 - i];
        out[i * 2] = HEX[(b >> 4) as usize];
        out[i * 2 + 1] = HEX[(b & 0x0f) as usize];
        i += 1;
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainId {
    pub bytes: [u8; MAX_CAIP2],
    pub len: u8,
}

impl ChainId {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    pub fn is_bip122(&self) -> bool {
        self.as_bytes().starts_with(b"bip122:")
    }
}

pub fn parse_chain_id(s: &[u8]) -> Result<ChainId, QiError> {
    if s.len() < 8 || s.len() > MAX_CAIP2 || !s.starts_with(b"bip122:") {
        return Err(QiError::MalformedChainId);
    }
    let hex = &s[7..];
    if hex.len() != 32 && hex.len() != 64 {
        return Err(QiError::MalformedChainId);
    }
    let mut i = 0;
    while i < hex.len() {
        match hex[i] {
            b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {}
            _ => return Err(QiError::MalformedChainId),
        }
        i += 1;
    }
    let mut id = ChainId {
        bytes: [0u8; MAX_CAIP2],
        len: s.len() as u8,
    };
    id.bytes[..s.len()].copy_from_slice(s);
    Ok(id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outpoint {
    pub txid: [u8; 32],
    pub vout: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtxoCommitment {
    pub outpoint: Outpoint,
    pub digest: [u8; 32],
    pub tombstone: bool,
}

fn compact_size(tx: &[u8], i: &mut usize) -> Result<u64, QiError> {
    if *i >= tx.len() {
        return Err(QiError::MalformedTx);
    }
    let first = tx[*i];
    *i += 1;
    match first {
        0xfd => read_int(tx, i, 2),
        0xfe => read_int(tx, i, 4),
        0xff => read_int(tx, i, 8),
        n => Ok(n as u64),
    }
}

fn read_int(tx: &[u8], i: &mut usize, n: usize) -> Result<u64, QiError> {
    if *i + n > tx.len() {
        return Err(QiError::MalformedTx);
    }
    let mut v = 0u64;
    let mut k = 0;
    while k < n {
        v |= (tx[*i + k] as u64) << (8 * k);
        k += 1;
    }
    *i += n;
    Ok(v)
}

fn skip_bytes(tx: &[u8], i: &mut usize, n: u64) -> Result<(), QiError> {
    let n = n as usize;
    if *i + n > tx.len() {
        return Err(QiError::MalformedTx);
    }
    *i += n;
    Ok(())
}

fn parse_qi_op_return(script: &[u8]) -> Result<Option<([u8; 32], bool)>, QiError> {
    if script.len() != 37 || script[0] != OP_RETURN || script[1] != 0x23 {
        return Ok(None);
    }
    if script[2] != QI_MAGIC[0] || script[3] != QI_MAGIC[1] {
        return Ok(None);
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&script[5..37]);
    match script[4] {
        LIVE_KIND => Ok(Some((digest, false))),
        TOMBSTONE_OPCODE => Ok(Some((digest, true))),
        _ => Err(QiError::MalformedTx),
    }
}

fn dsha256(bytes: &[u8]) -> [u8; 32] {
    sha256_32(&sha256_32(bytes))
}

pub fn extract_op_return(tx: &[u8]) -> Result<UtxoCommitment, QiError> {
    if tx.len() < 10 {
        return Err(QiError::MalformedTx);
    }
    let mut i = 4;
    let mut segwit = false;
    if tx.len() > 6 && tx[4] == 0x00 && tx[5] == 0x01 {
        segwit = true;
        i = 6;
    }
    let vin = compact_size(tx, &mut i)?;
    if vin == 0 || vin > 16 {
        return Err(QiError::MalformedTx);
    }
    let mut n_in = 0u64;
    while n_in < vin {
        skip_bytes(tx, &mut i, 32)?;
        let _ = read_int(tx, &mut i, 4)?;
        let script_len = compact_size(tx, &mut i)?;
        skip_bytes(tx, &mut i, script_len)?;
        let _ = read_int(tx, &mut i, 4)?;
        n_in += 1;
    }
    let vout_count = compact_size(tx, &mut i)?;
    if vout_count == 0 || vout_count > 16 {
        return Err(QiError::MalformedTx);
    }
    let mut found: Option<(u32, [u8; 32], bool)> = None;
    let mut v = 0u32;
    while v < vout_count as u32 {
        let _value = read_int(tx, &mut i, 8)?;
        let script_len = compact_size(tx, &mut i)? as usize;
        if i + script_len > tx.len() {
            return Err(QiError::MalformedTx);
        }
        let script = &tx[i..i + script_len];
        i += script_len;
        if let Some((digest, tombstone)) = parse_qi_op_return(script)? {
            if found.is_some() {
                return Err(QiError::MalformedTx);
            }
            found = Some((v, digest, tombstone));
        }
        v += 1;
    }
    if segwit {
        let mut n_in = 0u64;
        while n_in < vin {
            let items = compact_size(tx, &mut i)?;
            let mut k = 0u64;
            while k < items {
                let n = compact_size(tx, &mut i)?;
                skip_bytes(tx, &mut i, n)?;
                k += 1;
            }
            n_in += 1;
        }
    }
    if i + 4 != tx.len() {
        return Err(QiError::MalformedTx);
    }
    let (vout, digest, tombstone) = found.ok_or(QiError::MalformedTx)?;
    Ok(UtxoCommitment {
        outpoint: Outpoint {
            txid: dsha256(tx),
            vout,
        },
        digest,
        tombstone,
    })
}

pub fn verify_commitment(tx: &[u8], digest: &[u8; 32]) -> Result<Outpoint, QiError> {
    let c = extract_op_return(tx)?;
    if c.tombstone || &c.digest != digest {
        return Err(QiError::CommitmentMismatch);
    }
    Ok(c.outpoint)
}

pub fn encode_commitment_tx(
    prev_txid: &[u8; 32],
    prev_vout: u32,
    digest: &[u8; 32],
    tombstone: bool,
    out: &mut [u8],
) -> Result<usize, QiError> {
    let script_len = 37usize;
    let need = 4 + 1 + 32 + 4 + 1 + 4 + 1 + 8 + 1 + script_len + 4;
    if out.len() < need {
        return Err(QiError::BufferTooSmall);
    }
    let mut i = 0;
    out[i..i + 4].copy_from_slice(&1u32.to_le_bytes());
    i += 4;
    out[i] = 1;
    i += 1;
    out[i..i + 32].copy_from_slice(prev_txid);
    i += 32;
    out[i..i + 4].copy_from_slice(&prev_vout.to_le_bytes());
    i += 4;
    out[i] = 0;
    i += 1;
    out[i..i + 4].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    i += 4;
    out[i] = 1;
    i += 1;
    out[i..i + 8].copy_from_slice(&0u64.to_le_bytes());
    i += 8;
    out[i] = script_len as u8;
    i += 1;
    out[i] = OP_RETURN;
    i += 1;
    out[i] = 0x23;
    i += 1;
    out[i] = QI_MAGIC[0];
    i += 1;
    out[i] = QI_MAGIC[1];
    i += 1;
    if tombstone {
        out[i] = TOMBSTONE_OPCODE;
        i += 1;
        out[i..i + 32].copy_from_slice(&[0u8; 32]);
        i += 32;
    } else {
        out[i] = LIVE_KIND;
        i += 1;
        out[i..i + 32].copy_from_slice(digest);
        i += 32;
    }
    out[i..i + 4].copy_from_slice(&0u32.to_le_bytes());
    i += 4;
    Ok(i)
}

pub fn apply_utxo_attestation<S: QiStore>(
    store: &mut S,
    id: &DidQi,
    chain: &[u8],
    tx: &[u8],
) -> Result<u64, QiError> {
    admit_chain(chain)?;
    let (digest, generation) = load_canonical_digest(store, id)?;
    let c = extract_op_return(tx)?;
    if c.tombstone {
        let mut current = QiDocument::empty();
        read(store, id, &mut current)?;
        if !current.deactivated {
            return Err(QiError::TombstoneRequiresDeactivate);
        }
        return Ok(generation);
    }
    if c.digest != digest {
        return Err(QiError::CommitmentMismatch);
    }
    Ok(generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::document::QiDocument;
    use super::super::git_object::GitObjectStore;
    use super::super::method::{create, deactivate, read, update};
    use super::super::service::{CscpMailbox, Disclosure};
    use ed25519_dalek::SigningKey;

    fn sk() -> [u8; 32] {
        [5u8; 32]
    }

    fn sample_doc() -> QiDocument {
        let pk = SigningKey::from_bytes(&sk()).verifying_key().to_bytes();
        let mut doc = QiDocument::empty();
        doc.services[0] = CscpMailbox::mailbox(pk, Disclosure::ApprovedRelaysOnly);
        doc.service_count = 1;
        doc
    }

    fn tx_for(digest: &[u8; 32], tombstone: bool) -> ([u8; 160], usize) {
        let mut tx = [0u8; 160];
        let n = encode_commitment_tx(&[0x11; 32], 0, digest, tombstone, &mut tx).unwrap();
        (tx, n)
    }

    #[test]
    fn op_return_commitment_matches_wrong_fails_tombstone_deactivates() {
        let mut store = GitObjectStore::new();
        let id = create(&mut store, &sk(), &sample_doc()).unwrap();
        let (digest, gen) = load_canonical_digest(&store, &id).unwrap();
        assert_eq!(gen, 0);

        let (tx, n) = tx_for(&digest, false);
        let outpoint = verify_commitment(&tx[..n], &digest).unwrap();
        assert_eq!(outpoint.vout, 0);
        let extracted = extract_op_return(&tx[..n]).unwrap();
        assert_eq!(extracted.digest, digest);
        assert!(!extracted.tombstone);
        assert_eq!(
            apply_utxo_attestation(&mut store, &id, CONSTITUTION_MAINNET, &tx[..n]).unwrap(),
            0
        );

        let mut wrong = digest;
        wrong[0] ^= 0xff;
        let (bad, bn) = tx_for(&wrong, false);
        assert_eq!(
            verify_commitment(&bad[..bn], &digest),
            Err(QiError::CommitmentMismatch)
        );
        assert_eq!(
            apply_utxo_attestation(&mut store, &id, CONSTITUTION_MAINNET, &bad[..bn]),
            Err(QiError::CommitmentMismatch)
        );

        let (tomb, tn) = tx_for(&digest, true);
        let t = extract_op_return(&tomb[..tn]).unwrap();
        assert!(t.tombstone);
        assert_eq!(
            apply_utxo_attestation(&mut store, &id, CONSTITUTION_MAINNET, &tomb[..tn]),
            Err(QiError::TombstoneRequiresDeactivate)
        );
        let mut live = QiDocument::empty();
        read(&store, &id, &mut live).unwrap();
        assert!(!live.deactivated);
        assert_eq!(live.service_count, 1);

        deactivate(&mut store, &sk(), &id).unwrap();
        let (digest2, gen2) = load_canonical_digest(&store, &id).unwrap();
        assert_eq!(gen2, 1);
        let (tomb2, tn2) = tx_for(&digest2, true);
        let g = apply_utxo_attestation(&mut store, &id, CONSTITUTION_MAINNET, &tomb2[..tn2]).unwrap();
        assert_eq!(g, 1);
        let mut out = QiDocument::empty();
        read(&store, &id, &mut out).unwrap();
        assert!(out.deactivated);
        assert_eq!(out.service_count, 0);
        assert_eq!(
            update(&mut store, &sk(), &id, &sample_doc()),
            Err(QiError::Deactivated)
        );
    }

    #[test]
    fn caip2_bip122_parses_bitcoin_family() {
        let s = b"bip122:000000000019d6689c085ae165831e93";
        let id = parse_chain_id(s).unwrap();
        assert!(id.is_bip122());
        assert_eq!(parse_chain_id(b"eip155:1"), Err(QiError::MalformedChainId));
        assert_eq!(parse_chain_id(b"did:btc:x"), Err(QiError::MalformedChainId));
        assert!(chain_admitted(CONSTITUTION_MAINNET));
        assert!(chain_admitted(CONSTITUTION_TESTNET));
        assert_eq!(
            admit_chain(b"bip122:ffffffffffffffffffffffffffffffff"),
            Err(QiError::UnsupportedChain)
        );
        assert_eq!(admit_chain(b"eip155:1"), Err(QiError::MalformedChainId));
        assert_eq!(QiError::UnsupportedChain.token(), "unsupported_chain");
    }

    #[test]
    fn vector1_txid_display_is_byte_reversed() {
        let hex = b"010000000111111111111111111111111111111111111111111111111111111111111111110000000000ffffffff010000000000000000256a2351490173432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c674400000000";
        let mut tx = [0u8; 128];
        let n = hex.len() / 2;
        let mut i = 0;
        while i < n {
            tx[i] = hex_byte(hex[i * 2], hex[i * 2 + 1]);
            i += 1;
        }
        let c = extract_op_return(&tx[..n]).unwrap();
        assert_eq!(
            &txid_display(&c.outpoint.txid)[..],
            b"e9dfb2471de55f2d1702a5dd1c560a28c4a632640d5f8d0648613b69f6a5a176"
        );
        let mut wire = [0u8; 32];
        let wh = b"76a1a5f6693b6148068d5f0d6432a6c4280a561cdda502172d5fe51d47b2dfe9";
        i = 0;
        while i < 32 {
            wire[i] = hex_byte(wh[i * 2], wh[i * 2 + 1]);
            i += 1;
        }
        assert_eq!(c.outpoint.txid, wire);
    }

    #[test]
    fn spec_vector_live_tx_parses() {
        // Vector 1 raw tx (spec §20.1); unsignedDigest of that published document.
        let hex = b"010000000111111111111111111111111111111111111111111111111111111111111111110000000000ffffffff010000000000000000256a2351490173432eeabe01888f4770654fcd9a85b59605a2b3eb9d82b16b700c17345c674400000000";
        let mut tx = [0u8; 128];
        let n = hex.len() / 2;
        let mut i = 0;
        while i < n {
            tx[i] = hex_byte(hex[i * 2], hex[i * 2 + 1]);
            i += 1;
        }
        let c = extract_op_return(&tx[..n]).unwrap();
        assert!(!c.tombstone);
        assert_eq!(c.outpoint.vout, 0);
        assert_eq!(
            &c.digest[..4],
            &[0x73, 0x43, 0x2e, 0xea]
        );
    }

    fn hex_byte(a: u8, b: u8) -> u8 {
        fn n(c: u8) -> u8 {
            match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                _ => 0,
            }
        }
        (n(a) << 4) | n(b)
    }
}
