//! In-process git-like object store for Qualia Identifier documents.
//!
//! Blob encoding is `blob <len>\0` + bytes. Object id is SHA-256 of that
//! encoding (not git SHA-1, not GitHub). Generation is an annotated-tag
//! analogue (`qi/<generation>` as u64). Fixed slot cap; no unbounded map.

use sha2::{Digest, Sha256};

use super::id::DidQi;
use super::{QiError, QiStore};

pub const GIT_STORE_CAP: usize = 32;
pub const MAX_BLOB: usize = 4096;
pub const MAX_RECORD: usize = MAX_BLOB;

#[derive(Clone, Copy)]
struct GitSlot {
    did: [u8; 32],
    object_id: [u8; 32],
    generation: u64,
    len: u16,
    bytes: [u8; MAX_BLOB],
}

pub struct GitObjectStore {
    slots: [Option<GitSlot>; GIT_STORE_CAP],
}

impl GitObjectStore {
    pub const fn new() -> Self {
        Self {
            slots: [None; GIT_STORE_CAP],
        }
    }

    pub fn current_generation(&self, id: &DidQi) -> Result<u64, QiError> {
        match self.find_latest(id) {
            Some((_, slot)) => Ok(slot.generation),
            None => Err(QiError::NotFound),
        }
    }

    pub fn is_stale(&self, id: &DidQi, generation: u64) -> Result<bool, QiError> {
        Ok(generation < self.current_generation(id)?)
    }

    pub fn object_id_of(&self, id: &DidQi) -> Result<[u8; 32], QiError> {
        match self.find_latest(id) {
            Some((_, slot)) => Ok(slot.object_id),
            None => Err(QiError::NotFound),
        }
    }

    fn find_latest(&self, id: &DidQi) -> Option<(usize, GitSlot)> {
        let mut best: Option<(usize, GitSlot)> = None;
        let mut i = 0;
        while i < GIT_STORE_CAP {
            if let Some(slot) = self.slots[i] {
                if slot.did == id.0 {
                    match best {
                        None => best = Some((i, slot)),
                        Some((_, b)) if slot.generation >= b.generation => best = Some((i, slot)),
                        _ => {}
                    }
                }
            }
            i += 1;
        }
        best
    }

    fn find_slot(&self, id: &DidQi, generation: u64) -> Option<usize> {
        let mut i = 0;
        while i < GIT_STORE_CAP {
            if let Some(slot) = self.slots[i] {
                if slot.did == id.0 && slot.generation == generation {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn free_slot(&self) -> Option<usize> {
        let mut i = 0;
        while i < GIT_STORE_CAP {
            if self.slots[i].is_none() {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

impl Default for GitObjectStore {
    fn default() -> Self {
        Self::new()
    }
}

impl QiStore for GitObjectStore {
    fn put(&mut self, id: &DidQi, generation: u64, payload: &[u8]) -> Result<(), QiError> {
        if payload.len() > MAX_BLOB {
            return Err(QiError::CanonicalTooLarge);
        }
        let mut bytes = [0u8; MAX_BLOB];
        bytes[..payload.len()].copy_from_slice(payload);
        let slot = GitSlot {
            did: id.0,
            object_id: blob_object_id(payload),
            generation,
            len: payload.len() as u16,
            bytes,
        };
        if let Some(i) = self.find_slot(id, generation) {
            self.slots[i] = Some(slot);
            return Ok(());
        }
        let i = self.free_slot().ok_or(QiError::StoreFull)?;
        self.slots[i] = Some(slot);
        Ok(())
    }

    fn get(&self, id: &DidQi, out: &mut [u8]) -> Result<(u64, usize), QiError> {
        let (_, slot) = self.find_latest(id).ok_or(QiError::NotFound)?;
        let n = slot.len as usize;
        if out.len() < n {
            return Err(QiError::BufferTooSmall);
        }
        out[..n].copy_from_slice(&slot.bytes[..n]);
        Ok((slot.generation, n))
    }
}

pub fn write_decimal(n: usize, out: &mut [u8]) -> Result<usize, QiError> {
    if n == 0 {
        if out.is_empty() {
            return Err(QiError::BufferTooSmall);
        }
        out[0] = b'0';
        return Ok(1);
    }
    let mut tmp = [0u8; 20];
    let mut x = n;
    let mut i = 20;
    while x > 0 {
        i -= 1;
        tmp[i] = b'0' + (x % 10) as u8;
        x /= 10;
    }
    let len = 20 - i;
    if out.len() < len {
        return Err(QiError::BufferTooSmall);
    }
    out[..len].copy_from_slice(&tmp[i..]);
    Ok(len)
}

pub fn encode_blob(content: &[u8], out: &mut [u8]) -> Result<usize, QiError> {
    let mut dec = [0u8; 20];
    let dlen = write_decimal(content.len(), &mut dec)?;
    let need = 5 + dlen + 1 + content.len();
    if out.len() < need {
        return Err(QiError::BufferTooSmall);
    }
    out[..5].copy_from_slice(b"blob ");
    out[5..5 + dlen].copy_from_slice(&dec[..dlen]);
    out[5 + dlen] = 0;
    out[6 + dlen..need].copy_from_slice(content);
    Ok(need)
}

pub fn blob_object_id(content: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"blob ");
    let mut dec = [0u8; 20];
    let dlen = write_decimal(content.len(), &mut dec).unwrap_or(1);
    hasher.update(&dec[..dlen]);
    hasher.update([0u8]);
    hasher.update(content);
    hasher.finalize().into()
}

/// Annotated-tag analogue: writes `qi/<generation>` into `out`.
pub fn tag_name(generation: u64, out: &mut [u8]) -> Result<usize, QiError> {
    if out.len() < 3 {
        return Err(QiError::BufferTooSmall);
    }
    out[0] = b'q';
    out[1] = b'i';
    out[2] = b'/';
    let mut dec = [0u8; 20];
    let dlen = write_decimal(generation as usize, &mut dec)?;
    if out.len() < 3 + dlen {
        return Err(QiError::BufferTooSmall);
    }
    out[3..3 + dlen].copy_from_slice(&dec[..dlen]);
    Ok(3 + dlen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_object_id_deterministic_for_same_bytes() {
        let a = blob_object_id(b"qualia-identifier");
        let b = blob_object_id(b"qualia-identifier");
        let c = blob_object_id(b"qualia-identifier!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        let mut blob = [0u8; 64];
        let n = encode_blob(b"qualia-identifier", &mut blob).unwrap();
        assert_eq!(&blob[..5], b"blob ");
        assert_eq!(&blob[5..8], b"17\0");
        let hashed = {
            let mut h = Sha256::new();
            h.update(&blob[..n]);
            let d: [u8; 32] = h.finalize().into();
            d
        };
        assert_eq!(a, hashed);
    }

    #[test]
    fn store_put_get_and_stale_generation() {
        let mut store = GitObjectStore::new();
        let id = DidQi([11u8; 32]);
        store.put(&id, 1, b"gen-one").unwrap();
        store.put(&id, 2, b"gen-two").unwrap();
        let mut out = [0u8; 32];
        let (gen, n) = store.get(&id, &mut out).unwrap();
        assert_eq!(gen, 2);
        assert_eq!(&out[..n], b"gen-two");
        assert!(store.is_stale(&id, 1).unwrap());
        assert!(!store.is_stale(&id, 2).unwrap());
        let mut tag = [0u8; 16];
        let tn = tag_name(2, &mut tag).unwrap();
        assert_eq!(&tag[..tn], b"qi/2");
    }

    #[test]
    fn empty_store_get_fails_closed() {
        let store = GitObjectStore::new();
        let mut out = [0u8; 8];
        assert_eq!(store.get(&DidQi([1u8; 32]), &mut out), Err(QiError::NotFound));
    }

    #[test]
    fn store_full_fails_closed() {
        let mut store = GitObjectStore::new();
        let mut i = 0u8;
        while i < GIT_STORE_CAP as u8 {
            let mut did = [0u8; 32];
            did[0] = i;
            store.put(&DidQi(did), 1, b"x").unwrap();
            i += 1;
        }
        assert_eq!(
            store.put(&DidQi([0xff; 32]), 1, b"y"),
            Err(QiError::StoreFull)
        );
    }
}
