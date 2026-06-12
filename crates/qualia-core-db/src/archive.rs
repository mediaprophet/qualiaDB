#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapOptions;
use std::fs::File;
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Cursor, Read};
use std::path::Path;

/// Magic number for `.q42` luminary archive.
pub const Q42_MAGIC: [u8; 4] = [0x51, 0x34, 0x32, 0x00]; // "Q42\0"

/// Fixed-size Preamble (64 bytes) memory-mapped directly.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Q42Preamble {
    pub magic: [u8; 4],
    pub version: u16,
    pub global_flags: u16,

    // Dictionary Manifest (4 pointers x 4 bytes)
    // Each pointer is a 2-byte offset (relative to 0x40) and 2-byte size
    pub dict_manifest_standard: [u16; 2],
    pub dict_manifest_permissive: [u16; 2],
    pub dict_manifest_bilateral: [u16; 2],
    pub dict_manifest_spatiotemporal: [u16; 2],

    // Tier Index Manifest (4 pointers x 8 bytes)
    // Each pointer is a 4-byte physical offset and 4-byte size indicating where the Jump Tables begin
    pub index_manifest_standard: [u32; 2],
    pub index_manifest_permissive: [u32; 2],
    pub index_manifest_bilateral: [u32; 2],
    pub index_manifest_spatiotemporal: [u32; 2],

    pub eof_marker: u64,
}

/// Fixed-size Jump Table entry (12 bytes).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Q42JumpEntry {
    pub virtual_chunk_id: u32,
    pub physical_offset_low: u32,
    pub physical_offset_high: u16, // 6-byte offset total
    pub frame_size: u16,
}

impl Q42JumpEntry {
    pub fn physical_offset(&self) -> u64 {
        (self.physical_offset_low as u64) | ((self.physical_offset_high as u64) << 32)
    }
}

/// The main Q42 Archive reader utilizing memory-mapping and zero-deserialization structs.
#[cfg(not(target_arch = "wasm32"))]
pub struct Q42Archive {
    mmap: memmap2::Mmap,
}

#[cfg(not(target_arch = "wasm32"))]
impl Q42Archive {
    /// Opens and memory-maps a `.q42` file, instantly verifying the magic preamble.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        if mmap.len() < 64 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "File too small to contain Q42 Preamble",
            ));
        }

        let archive = Self { mmap };
        let preamble = archive.preamble();

        if preamble.magic != Q42_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Q42 Magic Number",
            ));
        }

        Ok(archive)
    }

    /// Casts the first 64 bytes into the `Q42Preamble` instantly.
    pub fn preamble(&self) -> &Q42Preamble {
        unsafe { &*(self.mmap.as_ptr() as *const Q42Preamble) }
    }

    /// Reads a specific jump table from the mapped memory.
    pub fn read_jump_table(&self, offset: u32, size_bytes: u32) -> &[Q42JumpEntry] {
        let start = offset as usize;
        let end = start + size_bytes as usize;
        let count = size_bytes as usize / std::mem::size_of::<Q42JumpEntry>();

        unsafe {
            std::slice::from_raw_parts(self.mmap[start..end].as_ptr() as *const Q42JumpEntry, count)
        }
    }

    /// Reads a dictionary from the dictionary sector.
    pub fn read_dictionary(&self, offset: u16, size_bytes: u16) -> &[u8] {
        let start = 0x40 + offset as usize;
        let end = start + size_bytes as usize;
        &self.mmap[start..end]
    }

    /// Fetches and decompresses a specific 128KB frame dynamically using the embedded Zstd dictionary.
    pub fn decompress_frame(&self, entry: &Q42JumpEntry, dict: &[u8]) -> io::Result<Vec<u8>> {
        let start = entry.physical_offset() as usize;
        let end = start + entry.frame_size as usize;
        let compressed_data = &self.mmap[start..end];

        let mut decoder =
            zstd::stream::Decoder::with_dictionary(Cursor::new(compressed_data), dict)?;
        let mut output = Vec::with_capacity(128 * 1024);
        decoder.read_to_end(&mut output)?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_zero_deserialization_header_alignment() {
        assert_eq!(std::mem::size_of::<Q42Preamble>(), 64);
        assert_eq!(std::mem::size_of::<Q42JumpEntry>(), 12);
    }

    #[test]
    fn test_q42_archive_mapping() {
        let mut file = NamedTempFile::new().unwrap();

        let preamble = Q42Preamble {
            magic: Q42_MAGIC,
            version: 1,
            global_flags: 0,
            dict_manifest_standard: [0, 0],
            dict_manifest_permissive: [0, 0],
            dict_manifest_bilateral: [0, 0],
            dict_manifest_spatiotemporal: [0, 0],
            index_manifest_standard: [0, 0],
            index_manifest_permissive: [0, 0],
            index_manifest_bilateral: [0, 0],
            index_manifest_spatiotemporal: [0, 0],
            eof_marker: 64,
        };

        unsafe {
            let bytes = std::slice::from_raw_parts(&preamble as *const _ as *const u8, 64);
            file.write_all(bytes).unwrap();
        }

        let archive = Q42Archive::open(file.path()).unwrap();
        let mapped_preamble = archive.preamble();

        let magic = mapped_preamble.magic;
        let version = mapped_preamble.version;
        let eof = mapped_preamble.eof_marker;

        assert_eq!(magic, Q42_MAGIC);
        assert_eq!(version, 1);
        assert_eq!(eof, 64);
    }
}
