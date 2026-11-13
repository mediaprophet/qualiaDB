//! Fixed-schema shortest-encoding CBOR. No recursive/generic object model.
use super::{DescriptorError as E, MAX_ENCODED, PROFILE};
pub(super) struct W<'a> {
    out: Option<&'a mut [u8]>,
    pub pos: usize,
}
impl<'a> W<'a> {
    pub fn measure() -> Self {
        Self { out: None, pos: 0 }
    }
    pub fn output(out: &'a mut [u8]) -> Self {
        Self {
            out: Some(out),
            pos: 0,
        }
    }
    fn put(&mut self, b: &[u8]) -> Result<(), E> {
        let end = self.pos.checked_add(b.len()).ok_or(E::Overflow)?;
        if end > MAX_ENCODED {
            return Err(E::Limit);
        }
        if let Some(out) = self.out.as_mut() {
            out.get_mut(self.pos..end)
                .ok_or(E::Capacity)?
                .copy_from_slice(b);
        }
        self.pos = end;
        Ok(())
    }
    fn head(&mut self, major: u8, n: u64) -> Result<(), E> {
        let tag = major << 5;
        match n {
            0..=23 => self.put(&[tag | n as u8]),
            24..=255 => self.put(&[tag | 24, n as u8]),
            256..=65535 => {
                self.put(&[tag | 25])?;
                self.put(&(n as u16).to_be_bytes())
            }
            65536..=4294967295 => {
                self.put(&[tag | 26])?;
                self.put(&(n as u32).to_be_bytes())
            }
            _ => {
                self.put(&[tag | 27])?;
                self.put(&n.to_be_bytes())
            }
        }
    }
    pub fn uint(&mut self, n: u64) -> Result<(), E> {
        self.head(0, n)
    }
    pub fn array(&mut self, n: usize) -> Result<(), E> {
        self.head(4, u64::try_from(n).map_err(|_| E::Overflow)?)
    }
    pub fn map(&mut self, n: usize) -> Result<(), E> {
        self.head(5, u64::try_from(n).map_err(|_| E::Overflow)?)
    }
    pub fn bytes(&mut self, b: &[u8]) -> Result<(), E> {
        self.head(2, u64::try_from(b.len()).map_err(|_| E::Overflow)?)?;
        self.put(b)
    }
    pub fn text(&mut self, s: &str) -> Result<(), E> {
        self.head(3, u64::try_from(s.len()).map_err(|_| E::Overflow)?)?;
        self.put(s.as_bytes())
    }
    pub fn null(&mut self) -> Result<(), E> {
        self.put(&[0xf6])
    }
    pub fn profile(&mut self, n: usize) -> Result<(), E> {
        self.map(n)?;
        self.uint(0)?;
        self.text(PROFILE)?;
        self.uint(1)?;
        self.uint(1)
    }
}
#[derive(Clone, Copy)]
pub(super) struct R<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
}
impl<'a> R<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, E> {
        if bytes.len() > MAX_ENCODED {
            return Err(E::Limit);
        }
        Ok(Self { bytes, pos: 0 })
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], E> {
        let end = self.pos.checked_add(n).ok_or(E::Overflow)?;
        let b = self.bytes.get(self.pos..end).ok_or(E::Encoding)?;
        self.pos = end;
        Ok(b)
    }
    fn head(&mut self, major: u8) -> Result<u64, E> {
        let first = self.take(1)?[0];
        if first >> 5 != major {
            return Err(E::Encoding);
        }
        let ai = first & 31;
        let n = match ai {
            0..=23 => u64::from(ai),
            24 => u64::from(self.take(1)?[0]),
            25 => u64::from(u16::from_be_bytes(
                self.take(2)?.try_into().map_err(|_| E::Encoding)?,
            )),
            26 => u64::from(u32::from_be_bytes(
                self.take(4)?.try_into().map_err(|_| E::Encoding)?,
            )),
            27 => u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| E::Encoding)?),
            _ => return Err(E::Encoding),
        };
        if (ai == 24 && n < 24)
            || (ai == 25 && n <= 255)
            || (ai == 26 && n <= 65535)
            || (ai == 27 && n <= u32::MAX as u64)
        {
            return Err(E::Encoding);
        }
        Ok(n)
    }
    pub fn uint(&mut self) -> Result<u64, E> {
        self.head(0)
    }
    pub fn u32(&mut self) -> Result<u32, E> {
        u32::try_from(self.uint()?).map_err(|_| E::Length)
    }
    pub fn key(&mut self, n: u64) -> Result<(), E> {
        if self.uint()? != n {
            return Err(E::Field);
        }
        Ok(())
    }
    pub fn array(&mut self) -> Result<usize, E> {
        usize::try_from(self.head(4)?).map_err(|_| E::Length)
    }
    pub fn fixed_array(&mut self, n: usize) -> Result<(), E> {
        if self.array()? != n {
            return Err(E::Length);
        }
        Ok(())
    }
    pub fn map(&mut self, n: usize) -> Result<(), E> {
        if self.head(5)? != n as u64 {
            return Err(E::Field);
        }
        Ok(())
    }
    pub fn bytes(&mut self) -> Result<&'a [u8], E> {
        let n = usize::try_from(self.head(2)?).map_err(|_| E::Length)?;
        self.take(n)
    }
    pub fn text(&mut self) -> Result<&'a str, E> {
        let n = usize::try_from(self.head(3)?).map_err(|_| E::Length)?;
        core::str::from_utf8(self.take(n)?).map_err(|_| E::Encoding)
    }
    pub fn digest(&mut self) -> Result<crate::net::qdnf::types::StrongDigest, E> {
        Ok(crate::net::qdnf::types::StrongDigest(
            self.bytes()?.try_into().map_err(|_| E::Length)?,
        ))
    }
    pub fn optional_digest(&mut self) -> Result<Option<crate::net::qdnf::types::StrongDigest>, E> {
        if self.bytes.get(self.pos) == Some(&0xf6) {
            self.pos += 1;
            Ok(None)
        } else {
            self.digest().map(Some)
        }
    }
    pub fn zero(&mut self) -> Result<(), E> {
        if self.uint()? != 0 {
            return Err(E::Unsupported);
        }
        Ok(())
    }
    pub fn profile(&mut self, n: usize) -> Result<(), E> {
        self.map(n)?;
        self.key(0)?;
        if self.text()? != PROFILE {
            return Err(E::Unsupported);
        }
        self.key(1)?;
        if self.uint()? != 1 {
            return Err(E::Unsupported);
        }
        Ok(())
    }
    pub fn finish(&self) -> Result<(), E> {
        if self.pos != self.bytes.len() {
            return Err(E::Encoding);
        }
        Ok(())
    }
}
pub(super) fn encode(
    out: &mut [u8],
    mut write: impl FnMut(&mut W<'_>) -> Result<(), E>,
) -> Result<usize, E> {
    let mut size = W::measure();
    write(&mut size)?;
    if out.len() < size.pos {
        return Err(E::Capacity);
    }
    let mut writer = W::output(out);
    write(&mut writer)?;
    Ok(writer.pos)
}
