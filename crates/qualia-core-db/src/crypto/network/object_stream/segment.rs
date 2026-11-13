//! Exact segment and leaf framing. Raw secret helpers are crate-private only.
use super::codec::Writer;
use super::{
    Error, LEAF_AAD_BYTES, LEAF_BYTES, MAX_METADATA, MAX_OBJECT_BYTES, MAX_SEGMENTS,
    SEGMENT_INFO_BYTES, SEGMENT_LEAVES,
};
use crate::crypto::network::{aead, kdf::hkdf_sha384};
use crate::net::qdnf::types::StrongDigest;
use zeroize::{Zeroize, Zeroizing};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectScope {
    context: StrongDigest,
    id: u32,
    generation: u64,
    epoch: u32,
    length: u64,
}
impl ObjectScope {
    pub fn new(
        context: StrongDigest,
        id: u32,
        generation: u64,
        epoch: u32,
        length: u64,
    ) -> Result<Self, Error> {
        super::types::nonzero(&context.0)?;
        if epoch == 0
            || length > MAX_OBJECT_BYTES
            || (id == 0 && (length == 0 || length > MAX_METADATA))
        {
            return Err(Error::Range);
        }
        Ok(Self {
            context,
            id,
            generation,
            epoch,
            length,
        })
    }
    pub const fn context(&self) -> StrongDigest {
        self.context
    }
    pub const fn id(&self) -> u32 {
        self.id
    }
    pub const fn generation(&self) -> u64 {
        self.generation
    }
    pub const fn epoch(&self) -> u32 {
        self.epoch
    }
    pub const fn length(&self) -> u64 {
        self.length
    }
    pub const fn role(&self) -> u8 {
        if self.id == 0 { 0 } else { 1 }
    }
    pub const fn leaf_count(&self) -> u64 {
        self.length.div_ceil(LEAF_BYTES)
    }
    pub fn leaf_length(&self, index: u64) -> Result<usize, Error> {
        if index >= self.leaf_count() {
            return Err(Error::Range);
        }
        Ok((self.length - index * LEAF_BYTES).min(LEAF_BYTES) as usize)
    }
    pub fn segment(&self, index: u64) -> Result<u32, Error> {
        self.leaf_length(index)?;
        let n = index / SEGMENT_LEAVES;
        if n >= MAX_SEGMENTS {
            return Err(Error::Range);
        }
        Ok(n as u32)
    }
}
fn common(w: &mut Writer<'_>, s: ObjectScope, segment: u32) -> Result<(), Error> {
    w.u16(1)?;
    w.put(&s.context.0)?;
    w.put(&[s.role()])?;
    w.u32(s.id)?;
    w.u64(s.generation)?;
    w.u32(s.epoch)?;
    w.u32(segment)
}
pub fn encode_segment_info(s: ObjectScope, segment: u32, out: &mut [u8]) -> Result<usize, Error> {
    if u64::from(segment) >= s.leaf_count().div_ceil(SEGMENT_LEAVES)
        || u64::from(segment) >= MAX_SEGMENTS
    {
        return Err(Error::Range);
    }
    let mut w = Writer::raw(out, SEGMENT_INFO_BYTES)?;
    w.put(b"qbdl-stream-protected-v1/segment-key\0")?;
    common(&mut w, s, segment)?;
    w.u64(s.length)?;
    w.finish()
}
pub fn encode_leaf_aad(s: ObjectScope, index: u64, out: &mut [u8]) -> Result<usize, Error> {
    let len = s.leaf_length(index)?;
    let mut w = Writer::raw(out, LEAF_AAD_BYTES)?;
    w.put(b"qbdl-stream-protected-v1/leaf-aad\0")?;
    common(&mut w, s, s.segment(index)?)?;
    w.u64(index)?;
    w.u64(s.length)?;
    w.u32(len as u32)?;
    w.put(&[u8::from(index + 1 == s.leaf_count())])?;
    w.finish()
}
pub fn leaf_nonce(s: ObjectScope, index: u64) -> Result<[u8; 12], Error> {
    s.leaf_length(index)?;
    let mut out = [0; 12];
    out[4..].copy_from_slice(&index.to_be_bytes());
    Ok(out)
}
pub(crate) fn derive_segment(
    master: &[u8; 32],
    s: ObjectScope,
    segment: u32,
    out: &mut [u8; 32],
) -> Result<(), Error> {
    super::types::nonzero(master)?;
    let mut info = [0; SEGMENT_INFO_BYTES];
    encode_segment_info(s, segment, &mut info)?;
    hkdf_sha384(Some(&s.context.0), master, &info, out)
}
pub(crate) fn seal_leaf_raw(
    master: &[u8; 32],
    s: ObjectScope,
    index: u64,
    buffer: &mut [u8],
    tag: &mut [u8; 16],
) -> Result<(), Error> {
    if buffer.len() != s.leaf_length(index)? {
        return Err(Error::Range);
    }
    let mut key = Zeroizing::new([0; 32]);
    derive_segment(master, s, s.segment(index)?, &mut key)?;
    let mut aad = [0; LEAF_AAD_BYTES];
    encode_leaf_aad(s, index, &mut aad)?;
    let result = aead::encrypt_in_place(&key, &leaf_nonce(s, index)?, &aad, buffer, tag);
    if result.is_err() {
        buffer.zeroize();
        tag.zeroize();
    }
    result
}
pub(crate) fn open_leaf_raw(
    master: &[u8; 32],
    s: ObjectScope,
    index: u64,
    buffer: &mut [u8],
    tag: &[u8; 16],
) -> Result<(), Error> {
    if buffer.len() != s.leaf_length(index)? {
        return Err(Error::Range);
    }
    let mut key = Zeroizing::new([0; 32]);
    derive_segment(master, s, s.segment(index)?, &mut key)?;
    let mut aad = [0; LEAF_AAD_BYTES];
    encode_leaf_aad(s, index, &mut aad)?;
    let result = aead::decrypt_in_place(&key, &leaf_nonce(s, index)?, &aad, buffer, tag);
    if result.is_err() {
        buffer.zeroize();
    }
    result
}
