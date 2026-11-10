//! Mode 0x04: Fixed 64-Byte Radial-Edge B-Rep shell face record (`M64ShellRecord`).
//!
//! One cache-line record per manifold face. Per-record CRC-32C occupies the last
//! four bytes, matching STRATA / PhaseJuncture policy.

use super::chunk_directory::M64Error;
use crate::container_10d::crc32c::crc32c;

pub const SHELL_RECORD_BYTES: usize = 64;

/// Fixed 64-Byte B-Rep shell face record (Mode 0x04).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M64ShellRecord {
    pub face_id: u32,
    pub vertex_indices: [u32; 3],
    pub edge_indices: [u32; 3],
    pub normal: [f32; 3],
    pub centroid: [f32; 3],
    pub crease: f32,
    pub flags: u32,
    pub record_crc32c: u32,
}

impl Default for M64ShellRecord {
    fn default() -> Self {
        Self {
            face_id: 0,
            vertex_indices: [0; 3],
            edge_indices: [0; 3],
            normal: [0.0, 1.0, 0.0],
            centroid: [0.0; 3],
            crease: 0.0,
            flags: 0,
            record_crc32c: 0,
        }
    }
}

impl M64ShellRecord {
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; SHELL_RECORD_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    #[inline]
    pub fn update_crc(&mut self) {
        self.record_crc32c = self.compute_crc32c();
    }

    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        let computed = self.compute_crc32c();
        if self.record_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.record_crc32c,
                computed,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m64_shell_record_size_crc() {
        assert_eq!(std::mem::size_of::<M64ShellRecord>(), 64);
        assert_eq!(std::mem::align_of::<M64ShellRecord>(), 64);
        let mut rec = M64ShellRecord::default();
        rec.face_id = 7;
        rec.update_crc();
        assert!(rec.validate().is_ok());
        rec.face_id ^= 1;
        assert!(matches!(
            rec.validate(),
            Err(M64Error::ChecksumMismatch { .. })
        ));
    }
}
