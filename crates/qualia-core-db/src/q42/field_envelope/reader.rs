use super::*;
/// Directory fields remain uninterpreted until family semantic admission.
#[derive(Clone, Copy, Debug)]
pub struct Entry<'a> {
    bytes: &'a [u8],
}
impl<'a> Entry<'a> {
    pub fn mode(self) -> u16 {
        u16le(self.bytes, 0)
    }
    pub fn revision(self) -> u16 {
        u16le(self.bytes, 2)
    }
    pub fn flags(self) -> u32 {
        u32le(self.bytes, 4)
    }
    pub fn stream_id(self) -> u32 {
        u32le(self.bytes, 8)
    }
    pub fn frame_id(self) -> u32 {
        u32le(self.bytes, 12)
    }
    pub fn start_tick(self) -> i64 {
        u64le(self.bytes, 16) as i64
    }
    pub fn time_count(self) -> u64 {
        u64le(self.bytes, 24)
    }
    pub fn spatial_key(self) -> u64 {
        u64le(self.bytes, 32)
    }
    pub fn payload_offset(self) -> u64 {
        u64le(self.bytes, 40)
    }
    pub fn stored_bytes(self) -> u64 {
        u64le(self.bytes, 48)
    }
    pub fn decoded_bytes(self) -> u64 {
        u64le(self.bytes, 56)
    }
    pub fn record_count(self) -> u64 {
        u64le(self.bytes, 64)
    }
    pub fn stride(self) -> u32 {
        u32le(self.bytes, 72)
    }
    pub fn descriptor_id(self) -> u32 {
        u32le(self.bytes, 112)
    }
    pub fn digest(self) -> &'a [u8] {
        &self.bytes[80..112]
    }
    pub fn raw_directory_bytes(self) -> &'a [u8] {
        self.bytes
    }
    fn key(self) -> (u32, i64, u64, u16, u32) {
        (
            self.stream_id(),
            self.start_tick(),
            self.spatial_key(),
            self.mode(),
            self.descriptor_id(),
        )
    }
}
/// Borrowed structural/index validation only; no scientific admission or authentication.
#[derive(Debug)]
pub struct StructuralView<'a> {
    bytes: &'a [u8],
    family: Family,
    directory: &'a [u8],
    metadata: &'a [u8],
}
impl<'a> StructuralView<'a> {
    pub fn open(bytes: &'a [u8], family: Family, limits: Limits) -> Result<Self, Error> {
        let magic = bytes.get(..8).ok_or(Error::Extent)?;
        if magic != family.magic() {
            return Err(Error::Family);
        }
        let h = bytes.get(..128).ok_or(Error::Extent)?;
        if u16le(h, 8) != family.version() {
            return Err(Error::Version);
        }
        crc(h)?;
        if bytes.len() as u64 > limits.file_bytes {
            return Err(Error::Limit);
        }
        if u16le(h, 10) != 128
            || u32le(h, 12) & !1 != 0
            || u64le(h, 16) != bytes.len() as u64
            || u64le(h, 24) != 128
            || u16le(h, 36) != 128
            || u16le(h, 38) != 1
        {
            return Err(Error::Header);
        }
        zero(&h[88..124])?;
        let count = u32le(h, 32);
        let meta_len = u64le(h, 48);
        if count > limits.entries || meta_len > limits.metadata_bytes {
            return Err(Error::Limit);
        }
        let dir_len = u64::from(count).checked_mul(128).ok_or(Error::Overflow)?;
        let meta_start = 128u64.checked_add(dir_len).ok_or(Error::Overflow)?;
        if u64le(h, 40) != meta_start {
            return Err(Error::Header);
        }
        let directory = extent(bytes, 128, dir_len)?;
        let metadata = extent(bytes, meta_start, meta_len)?;
        let alignment = if u32le(h, 12) & 1 != 0 { 4096 } else { 64 };
        let meta_end = meta_start.checked_add(meta_len).ok_or(Error::Overflow)?;
        let mut next = align(meta_end, alignment)?;
        zero(extent(bytes, meta_end, next - meta_end)?)?;
        let mut prior = None;
        let mut decoded = 0u64;
        for row in directory.chunks_exact(128) {
            crc(row)?;
            zero(&row[78..80])?;
            zero(&row[116..124])?;
            let e = Entry { bytes: row };
            if e.flags() & !7 != 0
                || e.descriptor_id() == 0
                || e.stride() == 0
                || e.record_count() == 0
            {
                return Err(Error::Directory);
            }
            if u16le(row, 76) != 0 {
                return Err(Error::Codec);
            }
            if (e.flags() & 2 == 0 && (e.start_tick() != 0 || e.time_count() != 0))
                || (e.flags() & 2 != 0 && e.time_count() == 0)
                || (e.flags() & 4 == 0 && e.spatial_key() != 0)
            {
                return Err(Error::Directory);
            }
            if prior.is_some_and(|p| p >= e.key()) {
                return Err(Error::Order);
            }
            prior = Some(e.key());
            let size = e
                .record_count()
                .checked_mul(u64::from(e.stride()))
                .ok_or(Error::Overflow)?;
            if size != e.stored_bytes() || size != e.decoded_bytes() || e.payload_offset() != next {
                return Err(Error::Directory);
            }
            decoded = decoded.checked_add(size).ok_or(Error::Overflow)?;
            if size > limits.chunk_bytes || decoded > limits.aggregate_decoded_bytes {
                return Err(Error::Limit);
            }
            extent(bytes, next, size)?;
            let end = next.checked_add(size).ok_or(Error::Overflow)?;
            next = align(end, alignment)?;
            zero(extent(bytes, end, next - end)?)?;
        }
        if next != bytes.len() as u64 {
            return Err(Error::Extent);
        }
        let mut root_header = [0; 128];
        root_header.copy_from_slice(h);
        root_header[56..88].fill(0);
        root_header[124..128].fill(0);
        let mut hasher = blake3::Hasher::new();
        hasher.update(INDEX_DOMAIN);
        hasher.update(&root_header);
        hasher.update(directory);
        hasher.update(metadata);
        if hasher.finalize().as_bytes()[..] != h[56..88] {
            return Err(Error::IndexDigest);
        }
        metadata::validate(metadata, limits)?;
        Ok(Self {
            bytes,
            family,
            directory,
            metadata,
        })
    }
    pub fn family(&self) -> Family {
        self.family
    }
    pub fn entry_count(&self) -> u32 {
        (self.directory.len() / 128) as u32
    }
    /// One-based directory index; zero is absent and rejected here.
    pub fn entry(&self, index: u32) -> Result<Entry<'a>, Error> {
        let n = index.checked_sub(1).ok_or(Error::EntryIndex)?;
        Ok(Entry {
            bytes: extent(self.directory, u64::from(n) * 128, 128)
                .map_err(|_| Error::EntryIndex)?,
        })
    }
    pub fn blocks(&self) -> Blocks<'a> {
        Blocks::new(self.metadata)
    }
    pub fn metadata_bytes(&self) -> &'a [u8] {
        self.metadata
    }
    pub fn index_digest(&self) -> &'a [u8] {
        &self.bytes[56..88]
    }
    /// Whole stored-chunk digest verification, still no scientific/numeric claim.
    pub fn verify_chunk(&self, index: u32) -> Result<&'a [u8], Error> {
        let e = self.entry(index)?;
        let bytes = extent(self.bytes, e.payload_offset(), e.stored_bytes())?;
        if blake3::hash(bytes).as_bytes()[..] != *e.digest() {
            return Err(Error::PayloadDigest);
        }
        Ok(bytes)
    }
}
