//! Expiring opaque rendezvous records. No public DHT. Common relay selection.

pub const MAX_RECORDS: usize = 16;
pub const MAX_BODY: usize = 128;
pub const MAX_RELAY_SET: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub cap: [u8; 16],
    pub expiry_unix: u32,
    pub len: u16,
    pub body: [u8; MAX_BODY],
}

impl Record {
    pub const fn empty() -> Self {
        Self {
            cap: [0; 16],
            expiry_unix: 0,
            len: 0,
            body: [0; MAX_BODY],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RendezvousStore {
    recs: [Record; MAX_RECORDS],
    len: usize,
}

impl RendezvousStore {
    pub const fn new() -> Self {
        Self {
            recs: [Record::empty(); MAX_RECORDS],
            len: 0,
        }
    }

    pub fn put(&mut self, cap: [u8; 16], expiry_unix: u32, body: &[u8]) -> Result<(), ()> {
        if body.len() > MAX_BODY || cap == [0u8; 16] {
            return Err(());
        }
        let mut i = 0;
        while i < self.len {
            if self.recs[i].cap == cap {
                self.recs[i].expiry_unix = expiry_unix;
                self.recs[i].len = body.len() as u16;
                self.recs[i].body[..body.len()].copy_from_slice(body);
                return Ok(());
            }
            i += 1;
        }
        if self.len >= MAX_RECORDS {
            return Err(());
        }
        let mut r = Record::empty();
        r.cap = cap;
        r.expiry_unix = expiry_unix;
        r.len = body.len() as u16;
        r.body[..body.len()].copy_from_slice(body);
        self.recs[self.len] = r;
        self.len += 1;
        Ok(())
    }

    pub fn get(&self, cap: &[u8; 16], now_unix: u32) -> Option<&[u8]> {
        let mut i = 0;
        while i < self.len {
            if &self.recs[i].cap == cap {
                if now_unix >= self.recs[i].expiry_unix {
                    return None;
                }
                return Some(&self.recs[i].body[..self.recs[i].len as usize]);
            }
            i += 1;
        }
        None
    }
}

/// First common allowed relay. Disjoint sets do not connect automatically.
pub fn common_relay(a: &[u64], b: &[u64]) -> Option<u64> {
    let mut i = 0;
    while i < a.len() && i < MAX_RELAY_SET {
        let mut j = 0;
        while j < b.len() && j < MAX_RELAY_SET {
            if a[i] != 0 && a[i] == b[j] {
                return Some(a[i]);
            }
            j += 1;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_expiry_and_common_relay() {
        let mut s = RendezvousStore::new();
        let cap = [1u8; 16];
        s.put(cap, 50, b"hint").unwrap();
        assert_eq!(s.get(&cap, 10), Some(&b"hint"[..]));
        assert!(s.get(&cap, 50).is_none());
        assert_eq!(common_relay(&[3, 5], &[9, 5]), Some(5));
        assert!(common_relay(&[3], &[9]).is_none());
    }
}
