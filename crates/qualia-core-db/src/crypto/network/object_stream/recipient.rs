//! Private recipient record framing; parsing never recognizes issuer authority.
use super::codec::{Reader, Writer};
use super::{Error, MAX_OBJECT_BYTES, RECIPIENT_BYTES, RecipientKeyIds};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use sha2::{Digest, Sha384};
#[derive(Clone, Copy)]
pub struct RecipientFields<'a> {
    pub descriptor: StrongDigest,
    pub context: StrongDigest,
    pub recipient: StrongDigest,
    pub publisher: StrongDigest,
    pub service: StrongDigest,
    pub purpose: StrongDigest,
    pub instrument: StrongDigest,
    pub scope: StrongDigest,
    pub operation: [u8; 16],
    pub closure: StrongDigest,
    pub policy: StrongDigest,
    pub consumer: StrongDigest,
    pub source: StrongDigest,
    pub source_length: u64,
    pub key_epoch: u64,
    pub not_before: u64,
    pub expires: u64,
    pub ml_public: &'a [u8; 1184],
    pub dh_public: &'a [u8; 32],
}
impl RecipientFields<'_> {
    pub fn validate(&self) -> Result<(), Error> {
        for d in [
            self.descriptor,
            self.context,
            self.recipient,
            self.publisher,
            self.service,
            self.purpose,
            self.instrument,
            self.scope,
            self.closure,
            self.policy,
            self.consumer,
            self.source,
        ] {
            super::types::nonzero(&d.0)?;
        }
        super::types::nonzero(&self.operation)?;
        super::types::nonzero(self.ml_public)?;
        super::types::nonzero(self.dh_public)?;
        if self.key_epoch == 0
            || self.expires <= self.not_before
            || self.source_length < 64
            || self.source_length > MAX_OBJECT_BYTES
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
pub struct RecipientRecord<'a> {
    bytes: &'a [u8],
    fields: RecipientFields<'a>,
}
impl<'a> RecipientRecord<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() != RECIPIENT_BYTES {
            return Err(Error::Range);
        }
        let mut r = Reader::new(bytes, b"QBDL-PRA-V1\0")?;
        r.suite()?;
        r.zero()?;
        let fields = RecipientFields {
            descriptor: r.digest()?,
            context: r.digest()?,
            recipient: r.digest()?,
            publisher: r.digest()?,
            service: r.digest()?,
            purpose: r.digest()?,
            instrument: r.digest()?,
            scope: r.digest()?,
            operation: r.array()?,
            closure: r.digest()?,
            policy: r.digest()?,
            consumer: r.digest()?,
            source: r.digest()?,
            source_length: r.u64()?,
            key_epoch: r.u64()?,
            not_before: r.u64()?,
            expires: r.u64()?,
            ml_public: r.take(1184)?.try_into().unwrap(),
            dh_public: r.take(32)?.try_into().unwrap(),
        };
        r.end()?;
        fields.validate()?;
        Ok(Self { bytes, fields })
    }
    pub fn fields(&self) -> RecipientFields<'a> {
        self.fields
    }
    pub fn encoded(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn id(&self) -> StrongDigest {
        sha384(self.bytes)
    }
    pub fn key_ids(&self) -> RecipientKeyIds {
        recipient_key_ids(self.fields.ml_public, self.fields.dh_public)
    }
    pub fn valid_at(&self, now: u64) -> Result<(), Error> {
        if now < self.fields.not_before || now >= self.fields.expires {
            Err(Error::Unauthorized)
        } else {
            Ok(())
        }
    }
}
pub fn recipient_key_ids(ml: &[u8; 1184], dh: &[u8; 32]) -> RecipientKeyIds {
    fn hash(domain: &[u8], key: &[u8]) -> StrongDigest {
        let mut h = Sha384::new();
        h.update(domain);
        h.update((key.len() as u16).to_be_bytes());
        h.update(key);
        StrongDigest(h.finalize().into())
    }
    RecipientKeyIds {
        ml_kem_768: hash(b"qbdl-stream-protected-v1/recipient/ml-kem-768\0", ml),
        x25519: hash(b"qbdl-stream-protected-v1/recipient/x25519\0", dh),
    }
}
pub fn encode_recipient(f: RecipientFields<'_>, out: &mut [u8]) -> Result<usize, Error> {
    f.validate()?;
    let mut w = Writer::new(out, RECIPIENT_BYTES, b"QBDL-PRA-V1\0")?;
    w.u16(1)?;
    w.u16(0)?;
    for d in [
        f.descriptor,
        f.context,
        f.recipient,
        f.publisher,
        f.service,
        f.purpose,
        f.instrument,
        f.scope,
    ] {
        w.put(&d.0)?;
    }
    w.put(&f.operation)?;
    for d in [f.closure, f.policy, f.consumer, f.source] {
        w.put(&d.0)?;
    }
    for n in [f.source_length, f.key_epoch, f.not_before, f.expires] {
        w.u64(n)?;
    }
    w.put(f.ml_public)?;
    w.put(f.dh_public)?;
    w.finish()
}
