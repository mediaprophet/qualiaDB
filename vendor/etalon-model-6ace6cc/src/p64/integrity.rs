/// Integrity is always selected explicitly at the parse boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityMode {
    Full,
    Metadata,
    Structure,
}

const fn table() -> [u32; 256] {
    let mut values = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & 0u32.wrapping_sub(crc & 1));
            bit += 1;
        }
        values[i] = crc;
        i += 1;
    }
    values
}

const TABLE: [u32; 256] = table();

/// CRC-32C (Castagnoli, reflected), with the standard initial and final XOR.
pub fn crc32c(bytes: &[u8]) -> u32 {
    let mut state = Crc32c::new();
    state.update(bytes);
    state.finish()
}

/// Incremental unkeyed CRC-32C; checksum agreement is not authentication.
#[derive(Clone, Copy, Debug)]
pub struct Crc32c(u32);

impl Default for Crc32c {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc32c {
    pub const fn new() -> Self {
        Self(u32::MAX)
    }
    pub fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = TABLE[((self.0 ^ u32::from(byte)) & 255) as usize] ^ (self.0 >> 8);
        }
    }
    pub const fn finish(self) -> u32 {
        !self.0
    }
}
