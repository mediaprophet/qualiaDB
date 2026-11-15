//! Caller-buffered candidate storage writer. This does not admit scientific schemas.
use super::*;

/// Raw stored payload and its uninterpreted directory fields. Codec 0 only.
#[derive(Clone, Copy, Debug)]
pub struct EntryInput<'a> {
    pub mode: u16,
    pub revision: u16,
    pub flags: u32,
    pub stream_id: u32,
    pub frame_id: u32,
    pub start_tick: i64,
    pub time_count: u64,
    pub spatial_key: u64,
    pub record_count: u64,
    pub stride: u32,
    pub codec: u16,
    pub descriptor_id: u32,
    pub payload: &'a [u8],
}
impl EntryInput<'_> {
    fn key(&self) -> (u32, i64, u64, u16, u32) {
        (
            self.stream_id,
            self.start_tick,
            self.spatial_key,
            self.mode,
            self.descriptor_id,
        )
    }
    fn validate(&self) -> Result<u64, Error> {
        if self.codec != 0 {
            return Err(Error::Codec);
        }
        if self.flags & !7 != 0
            || self.descriptor_id == 0
            || self.stride == 0
            || self.record_count == 0
        {
            return Err(Error::Directory);
        }
        if (self.flags & 2 == 0 && (self.start_tick != 0 || self.time_count != 0))
            || (self.flags & 2 != 0 && (self.time_count == 0 || self.stream_id == 0))
            || (self.flags & 4 == 0 && self.spatial_key != 0)
            || (self.flags & 4 != 0 && self.frame_id == 0)
        {
            return Err(Error::Directory);
        }
        let size = self
            .record_count
            .checked_mul(u64::from(self.stride))
            .ok_or(Error::Overflow)?;
        if size != self.payload.len() as u64 {
            return Err(Error::Directory);
        }
        Ok(size)
    }
}

/// Exact metadata bytes are preserved, including unknown critical information.
/// The caller must separately admit family schemas, references, modes and values.
#[derive(Clone, Copy, Debug)]
pub struct EnvelopeInput<'a> {
    pub family: Family,
    /// Bit 0 selects 4096-byte payload alignment, otherwise 64. Other bits reject.
    pub flags: u32,
    pub metadata: &'a [u8],
    pub entries: &'a [EntryInput<'a>],
}
impl EnvelopeInput<'_> {
    /// Validate every structural input and budget before returning the exact size.
    /// No allocation, hashing, output mutation or scientific admission occurs.
    pub fn encoded_len(&self, limits: Limits) -> Result<usize, Error> {
        if self.flags & !1 != 0 {
            return Err(Error::Header);
        }
        if self.entries.len() as u64 > u64::from(limits.entries)
            || self.metadata.len() as u64 > limits.metadata_bytes
        {
            return Err(Error::Limit);
        }
        metadata::validate(self.metadata, limits)?;
        let directory = (self.entries.len() as u64)
            .checked_mul(128)
            .ok_or(Error::Overflow)?;
        let meta_start = 128u64.checked_add(directory).ok_or(Error::Overflow)?;
        let end = meta_start
            .checked_add(self.metadata.len() as u64)
            .ok_or(Error::Overflow)?;
        let alignment = if self.flags & 1 == 0 { 64 } else { 4096 };
        let mut next = align(end, alignment)?;
        let mut decoded = 0u64;
        let mut prior = None;
        for entry in self.entries {
            let size = entry.validate()?;
            if prior.is_some_and(|p| p >= entry.key()) {
                return Err(Error::Order);
            }
            prior = Some(entry.key());
            decoded = decoded.checked_add(size).ok_or(Error::Overflow)?;
            if size > limits.chunk_bytes || decoded > limits.aggregate_decoded_bytes {
                return Err(Error::Limit);
            }
            next = align(next.checked_add(size).ok_or(Error::Overflow)?, alignment)?;
        }
        if next > limits.file_bytes {
            return Err(Error::Limit);
        }
        usize::try_from(next).map_err(|_| Error::Extent)
    }

    /// Validate the whole input and output capacity BEFORE touching output.
    /// On success only the returned prefix is changed; a longer buffer tail is kept.
    /// Borrow rules prevent payload/metadata aliasing the mutable output slice.
    pub fn encode_into<'out>(
        &self,
        output: &'out mut [u8],
        limits: Limits,
    ) -> Result<&'out mut [u8], Error> {
        let len = self.encoded_len(limits)?;
        if output.len() < len {
            return Err(Error::Extent);
        }
        let out = &mut output[..len];
        out.fill(0);
        let meta_start = 128 + self.entries.len() * 128;
        let meta_end = meta_start + self.metadata.len();
        out[meta_start..meta_end].copy_from_slice(self.metadata);
        let alignment = if self.flags & 1 == 0 { 64 } else { 4096 };
        let mut cursor = (meta_end + alignment - 1) & !(alignment - 1);
        for (index, entry) in self.entries.iter().enumerate() {
            let end = cursor + entry.payload.len();
            out[cursor..end].copy_from_slice(entry.payload);
            let row = &mut out[128 + index * 128..256 + index * 128];
            put16(row, 0, entry.mode);
            put16(row, 2, entry.revision);
            put32(row, 4, entry.flags);
            put32(row, 8, entry.stream_id);
            put32(row, 12, entry.frame_id);
            put64(row, 16, entry.start_tick as u64);
            put64(row, 24, entry.time_count);
            put64(row, 32, entry.spatial_key);
            put64(row, 40, cursor as u64);
            put64(row, 48, entry.payload.len() as u64);
            put64(row, 56, entry.payload.len() as u64);
            put64(row, 64, entry.record_count);
            put32(row, 72, entry.stride);
            // Codec and all reserved bytes were zeroed before writing.
            row[80..112].copy_from_slice(blake3::hash(entry.payload).as_bytes());
            put32(row, 112, entry.descriptor_id);
            seal_crc(row);
            cursor = (end + alignment - 1) & !(alignment - 1);
        }
        let header = &mut out[..128];
        header[..8].copy_from_slice(self.family.magic());
        put16(header, 8, self.family.version());
        put16(header, 10, 128);
        put32(header, 12, self.flags);
        put64(header, 16, len as u64);
        put64(header, 24, 128);
        put32(header, 32, self.entries.len() as u32);
        put16(header, 36, 128);
        put16(header, 38, 1);
        put64(header, 40, meta_start as u64);
        put64(header, 48, self.metadata.len() as u64);
        // Root and CRC fields are still zero; hash the exact encoded bytes.
        let mut hasher = blake3::Hasher::new();
        hasher.update(INDEX_DOMAIN);
        hasher.update(&out[..128]);
        hasher.update(&out[128..meta_start]);
        hasher.update(&out[meta_start..meta_end]);
        out[56..88].copy_from_slice(hasher.finalize().as_bytes());
        seal_crc(&mut out[..128]);
        Ok(out)
    }
}
fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
fn seal_crc(bytes: &mut [u8]) {
    let value = crate::container_10d::crc32c::crc32c(bytes);
    put32(bytes, 124, value);
}
