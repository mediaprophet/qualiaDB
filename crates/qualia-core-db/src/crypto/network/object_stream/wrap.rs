//! Hybrid metadata wrap wire framing. Raw secret helpers stay inside the crate.
use super::codec::{Reader, Writer};
use super::{
    Error, MAX_METADATA, RecipientKeyIds, RecipientRecord, WRAP_BYTES, WRAP_CONTEXT_BYTES,
};
use crate::crypto::network::{aead, digest::sha384, kdf::hkdf_sha384};
use crate::net::qdnf::types::StrongDigest;
use zeroize::{Zeroize, Zeroizing};
#[derive(Clone, Copy)]
pub struct Wrap<'a> {
    bytes: &'a [u8],
    pub auth: StrongDigest,
    pub key_ids: RecipientKeyIds,
    pub ephemeral: &'a [u8; 32],
    pub ciphertext: &'a [u8; 1088],
    pub encrypted_master: &'a [u8; 32],
    pub tag: &'a [u8; 16],
}
impl<'a> Wrap<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() != WRAP_BYTES {
            return Err(Error::Range);
        }
        let mut r = Reader::new(bytes, b"QBDL-PWR-V1\0")?;
        r.suite()?;
        r.zero()?;
        let auth = r.digest()?;
        let key_ids = RecipientKeyIds {
            ml_kem_768: r.digest()?,
            x25519: r.digest()?,
        };
        let ephemeral: &[u8; 32] = r.take(32)?.try_into().unwrap();
        let ciphertext = r.take(1088)?.try_into().unwrap();
        let encrypted_master = r.take(32)?.try_into().unwrap();
        let tag = r.take(16)?.try_into().unwrap();
        r.end()?;
        for d in [auth, key_ids.ml_kem_768, key_ids.x25519] {
            super::types::nonzero(&d.0)?;
        }
        super::types::nonzero(ephemeral)?;
        Ok(Self {
            bytes,
            auth,
            key_ids,
            ephemeral,
            ciphertext,
            encrypted_master,
            tag,
        })
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
}
#[derive(Clone, Copy)]
pub struct WrapContext<'a> {
    record: RecipientRecord<'a>,
    length: u64,
    ephemeral: &'a [u8; 32],
    ciphertext: &'a [u8; 1088],
}
impl<'a> WrapContext<'a> {
    pub fn new(
        record: RecipientRecord<'a>,
        metadata_length: u64,
        ephemeral: &'a [u8; 32],
        ciphertext: &'a [u8; 1088],
    ) -> Result<Self, Error> {
        if metadata_length == 0 || metadata_length > MAX_METADATA {
            return Err(Error::Range);
        }
        super::types::nonzero(ephemeral)?;
        Ok(Self {
            record,
            length: metadata_length,
            ephemeral,
            ciphertext,
        })
    }
    pub fn record(&self) -> RecipientRecord<'a> {
        self.record
    }
    pub fn matches(&self, wrap: Wrap<'_>) -> Result<(), Error> {
        if wrap.auth != self.record.id()
            || wrap.key_ids != self.record.key_ids()
            || wrap.ephemeral != self.ephemeral
            || wrap.ciphertext != self.ciphertext
        {
            Err(Error::Unauthorized)
        } else {
            Ok(())
        }
    }
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, Error> {
        let f = self.record.fields();
        let ids = self.record.key_ids();
        let mut w = Writer::new(out, WRAP_CONTEXT_BYTES, b"QBDL-PWC-V1\0")?;
        w.u16(1)?;
        for d in [self.record.id(), f.context, f.descriptor, f.recipient] {
            w.put(&d.0)?;
        }
        w.u64(f.key_epoch)?;
        w.u64(self.length)?;
        w.put(&ids.ml_kem_768.0)?;
        w.put(&ids.x25519.0)?;
        w.put(self.ephemeral)?;
        w.put(self.ciphertext)?;
        w.finish()
    }
}
pub fn encode_wrap(
    c: WrapContext<'_>,
    encrypted: &[u8; 32],
    tag: &[u8; 16],
    out: &mut [u8],
) -> Result<usize, Error> {
    let ids = c.record.key_ids();
    let mut w = Writer::new(out, WRAP_BYTES, b"QBDL-PWR-V1\0")?;
    w.u16(1)?;
    w.u16(0)?;
    w.put(&c.record.id().0)?;
    w.put(&ids.ml_kem_768.0)?;
    w.put(&ids.x25519.0)?;
    w.put(c.ephemeral)?;
    w.put(c.ciphertext)?;
    w.put(encrypted)?;
    w.put(tag)?;
    w.finish()
}
fn key(kem: &[u8; 32], dh: &[u8; 32], aad: &[u8], out: &mut [u8; 32]) -> Result<(), Error> {
    super::types::nonzero(dh)?;
    let mut ikm = Zeroizing::new([0; 64]);
    ikm[..32].copy_from_slice(kem);
    ikm[32..].copy_from_slice(dh);
    hkdf_sha384(
        Some(&sha384(aad).0),
        &ikm[..],
        b"qbdl-stream-protected-v1/metadata-wrap-key\0",
        out,
    )
}
pub(crate) fn seal_master_raw(
    c: WrapContext<'_>,
    kem: &[u8; 32],
    dh: &[u8; 32],
    master: &[u8; 32],
    out: &mut [u8],
) -> Result<usize, Error> {
    super::types::nonzero(master)?;
    if out.len() < WRAP_BYTES {
        return Err(Error::Capacity);
    }
    let mut aad = [0; WRAP_CONTEXT_BYTES];
    c.encode(&mut aad)?;
    let mut kek = Zeroizing::new([0; 32]);
    key(kem, dh, &aad, &mut kek)?;
    let mut encrypted = Zeroizing::new(*master);
    let mut tag = [0; 16];
    aead::encrypt_in_place(&kek, &[0; 12], &aad, &mut encrypted[..], &mut tag)?;
    encode_wrap(c, &encrypted, &tag, out)
}
pub(crate) fn open_master_raw(
    c: WrapContext<'_>,
    wrap: Wrap<'_>,
    kem: &[u8; 32],
    dh: &[u8; 32],
    out: &mut [u8; 32],
) -> Result<(), Error> {
    out.zeroize();
    c.matches(wrap)?;
    let mut aad = [0; WRAP_CONTEXT_BYTES];
    c.encode(&mut aad)?;
    let mut kek = Zeroizing::new([0; 32]);
    key(kem, dh, &aad, &mut kek)?;
    let mut master = Zeroizing::new(*wrap.encrypted_master);
    aead::decrypt_in_place(&kek, &[0; 12], &aad, &mut master[..], wrap.tag)?;
    super::types::nonzero(&master[..])?;
    out.copy_from_slice(&master[..]);
    Ok(())
}
