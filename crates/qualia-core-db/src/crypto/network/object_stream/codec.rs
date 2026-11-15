use super::{Error, PageRef};
use crate::net::qdnf::types::StrongDigest;
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8], magic: &[u8; 12]) -> Result<Self, Error> {
        let mut r = Self { bytes, at: 0 };
        if r.take(12)? != magic {
            return Err(Error::Malformed);
        }
        Ok(r)
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Range)?;
        let out = self.bytes.get(self.at..end).ok_or(Error::Truncated)?;
        self.at = end;
        Ok(out)
    }
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?.try_into().map_err(|_| Error::Truncated)
    }
    pub fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub fn digest(&mut self) -> Result<StrongDigest, Error> {
        Ok(StrongDigest(self.array()?))
    }
    pub fn suite(&mut self) -> Result<(), Error> {
        if self.u16()? != 1 {
            Err(Error::Downgrade)
        } else {
            Ok(())
        }
    }
    pub fn zero(&mut self) -> Result<(), Error> {
        if self.u16()? != 0 {
            Err(Error::Malformed)
        } else {
            Ok(())
        }
    }
    pub fn end(self) -> Result<(), Error> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}
pub(super) struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}
impl<'a> Writer<'a> {
    pub fn raw(out: &'a mut [u8], needed: usize) -> Result<Self, Error> {
        if out.len() < needed {
            return Err(Error::Capacity);
        }
        Ok(Self {
            out: &mut out[..needed],
            at: 0,
        })
    }
    pub fn new(out: &'a mut [u8], needed: usize, magic: &[u8; 12]) -> Result<Self, Error> {
        if out.len() < needed {
            return Err(Error::Capacity);
        }
        let mut w = Self {
            out: &mut out[..needed],
            at: 0,
        };
        w.put(magic)?;
        Ok(w)
    }
    pub fn put(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.at.checked_add(b.len()).ok_or(Error::Range)?;
        self.out
            .get_mut(self.at..end)
            .ok_or(Error::Capacity)?
            .copy_from_slice(b);
        self.at = end;
        Ok(())
    }
    pub fn u16(&mut self, v: u16) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn u32(&mut self, v: u32) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn u64(&mut self, v: u64) -> Result<(), Error> {
        self.put(&v.to_be_bytes())
    }
    pub fn finish(self) -> Result<usize, Error> {
        if self.at != self.out.len() {
            Err(Error::Malformed)
        } else {
            Ok(self.at)
        }
    }
}
pub(super) fn read_ref(b: &[u8]) -> Result<PageRef, Error> {
    if b.len() != 56 {
        return Err(Error::Truncated);
    }
    Ok(PageRef {
        page: u16::from_be_bytes(b[..2].try_into().unwrap()),
        rows: u16::from_be_bytes(b[2..4].try_into().unwrap()),
        length: u32::from_be_bytes(b[4..8].try_into().unwrap()),
        digest: StrongDigest(b[8..].try_into().unwrap()),
    })
}
pub(super) fn write_ref(w: &mut Writer<'_>, r: PageRef) -> Result<(), Error> {
    w.u16(r.page)?;
    w.u16(r.rows)?;
    w.u32(r.length)?;
    w.put(&r.digest.0)
}
pub(super) fn check_ref(
    r: PageRef,
    n: usize,
    page: usize,
    header: usize,
    row: usize,
) -> Result<(), Error> {
    if r.page as usize != page
        || r.rows as usize != super::types::rows(n, page)?
        || r.length as usize != header + row * r.rows as usize
    {
        return Err(Error::Malformed);
    }
    super::types::nonzero(&r.digest.0)
}
