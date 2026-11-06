//! Local Relative Metric (LRM3) Header Block for M64 Containers.
//!
//! Provides a 128-byte (2 × 64B cache line) header for micro-scale, medical,
//! anatomical, and laboratory local reference frames (SRID = 0 = `M64_SRID_LOCAL_MICROMETER`).

use crate::container_10d::crc32c::crc32c;
use bytemuck::{Pod, Zeroable};
use super::chunk_directory::M64Error;

/// Magic identifier for LRM3 local relative metric block: ASCII `LRM3`.
pub const LRM3_MAGIC: [u8; 4] = *b"LRM3";

/// Current LRM3 format version.
pub const LRM3_VERSION: u16 = 1;

/// Exact byte length of the LRM3 header block (128 bytes = 2 × 64B cache lines).
pub const LRM3_BLOCK_BYTES: usize = 128;

/// Cartesian local metric frame in micrometers (default).
pub const COORD_FRAME_CARTESIAN_MICROMETER: u8 = 0;
/// Cylindrical local frame for tubular anatomical structures (e.g. vascular, neural).
pub const COORD_FRAME_CYLINDRICAL_ANATOMY: u8 = 1;
/// Spherical local frame for organoid/cellular centroid geometry.
pub const COORD_FRAME_SPHERICAL_LOCAL: u8 = 2;

/// 128-byte Local Relative Metric (LRM3) Header (2 × 64B cache lines).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct M64LocalMetricHeader {
    /// 0x00..0x04: ASCII Magic `LRM3`
    pub magic: [u8; 4],
    /// 0x04..0x06: LRM3 version (1)
    pub version: u16,
    /// 0x06..0x07: Scale tier (S in 0..7)
    pub scale_tier: u8,
    /// 0x07..0x08: Coordinate frame kind (0=Cartesian, 1=Cylindrical, 2=Spherical)
    pub coord_frame_kind: u8,
    /// 0x08..0x10: Entity / specimen / patient DID hash anchor
    pub anchor_did_hash: u64,
    /// 0x10..0x28: World anchor origin offset in meters [x, y, z]
    pub origin_offset_m: [f64; 3],
    /// 0x28..0x38: Orientation quaternion [x, y, z, w] relative to anchor
    pub orientation_quat: [f32; 4],
    /// 0x38..0x44: Anisotropic scale in micrometers per integer unit [sx, sy, sz]
    pub anisotropic_scale_um: [f32; 3],
    /// 0x44..0x50: Bounding box minimum in micrometers [min_x, min_y, min_z]
    pub bbox_min_um: [i32; 3],
    /// 0x50..0x5C: Bounding box maximum in micrometers [max_x, max_y, max_z]
    pub bbox_max_um: [i32; 3],
    /// 0x5C..0x7C: Reserved for expansion (32 bytes)
    pub reserved: [u8; 32],
    /// 0x7C..0x80: CRC-32C checksum of bytes 0..124
    pub header_crc32c: u32,
}

impl Default for M64LocalMetricHeader {
    fn default() -> Self {
        Self {
            magic: LRM3_MAGIC,
            version: LRM3_VERSION,
            scale_tier: 0,
            coord_frame_kind: COORD_FRAME_CARTESIAN_MICROMETER,
            anchor_did_hash: 0,
            origin_offset_m: [0.0, 0.0, 0.0],
            orientation_quat: [0.0, 0.0, 0.0, 1.0],
            anisotropic_scale_um: [1.0, 1.0, 1.0],
            bbox_min_um: [0, 0, 0],
            bbox_max_um: [0, 0, 0],
            reserved: [0u8; 32],
            header_crc32c: 0,
        }
    }
}

impl M64LocalMetricHeader {
    /// Compute CRC-32C over the first 124 bytes of the header.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; 128] = bytemuck::cast_ref(self);
        crc32c(&bytes[..124])
    }

    /// Update `header_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.header_crc32c = self.compute_crc32c();
    }

    /// Validate magic, version, and CRC-32C checksum.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        if self.magic != LRM3_MAGIC {
            return Err(M64Error::InvalidMagic);
        }
        if self.version != LRM3_VERSION {
            return Err(M64Error::InvalidVersion(self.version));
        }
        let computed = self.compute_crc32c();
        if self.header_crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.header_crc32c,
                computed,
            })
        }
    }

    /// Convert local integer raw coordinates `[x, y, z]` to world coordinates in meters.
    ///
    /// Applies anisotropic micrometer scale, rotates by `orientation_quat`, and offsets
    /// by `origin_offset_m`.
    #[inline]
    pub fn local_i32_to_meters(&self, xyz_raw: [i32; 3]) -> [f64; 3] {
        // Convert to meters in local frame
        let vx = (xyz_raw[0] as f64) * (self.anisotropic_scale_um[0] as f64) * 1e-6;
        let vy = (xyz_raw[1] as f64) * (self.anisotropic_scale_um[1] as f64) * 1e-6;
        let vz = (xyz_raw[2] as f64) * (self.anisotropic_scale_um[2] as f64) * 1e-6;

        // Quaternion rotation: v' = v + 2*w*(q x v) + 2*(q x (q x v))
        let qx = self.orientation_quat[0] as f64;
        let qy = self.orientation_quat[1] as f64;
        let qz = self.orientation_quat[2] as f64;
        let qw = self.orientation_quat[3] as f64;

        // t = 2 * (q x v)
        let tx = 2.0 * (qy * vz - qz * vy);
        let ty = 2.0 * (qz * vx - qx * vz);
        let tz = 2.0 * (qx * vy - qy * vx);

        // v' = v + w * t + (q x t)
        let rx = vx + qw * tx + (qy * tz - qz * ty);
        let ry = vy + qw * ty + (qz * tx - qx * tz);
        let rz = vz + qw * tz + (qx * ty - qy * tx);

        [
            self.origin_offset_m[0] + rx,
            self.origin_offset_m[1] + ry,
            self.origin_offset_m[2] + rz,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lrm3_header_size_and_alignment() {
        assert_eq!(
            std::mem::size_of::<M64LocalMetricHeader>(),
            128,
            "M64LocalMetricHeader must be exactly 128 bytes"
        );
        assert_eq!(
            std::mem::align_of::<M64LocalMetricHeader>(),
            64,
            "M64LocalMetricHeader must be 64-byte aligned"
        );
    }

    #[test]
    fn test_lrm3_header_crc_roundtrip() {
        let mut header = M64LocalMetricHeader::default();
        header.anchor_did_hash = 0x1234_5678_9ABC_DEF0;
        header.origin_offset_m = [10.0, 20.0, 30.0];
        header.anisotropic_scale_um = [2.0, 2.0, 5.0];
        header.bbox_min_um = [-1000, -1000, 0];
        header.bbox_max_um = [1000, 1000, 5000];

        header.update_crc();
        assert_ne!(header.header_crc32c, 0);
        assert!(header.validate().is_ok());

        header.scale_tier ^= 1;
        assert!(matches!(header.validate(), Err(M64Error::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_lrm3_coordinate_transformation() {
        let mut header = M64LocalMetricHeader::default();
        header.origin_offset_m = [100.0, 200.0, 300.0];
        header.anisotropic_scale_um = [1.0, 1.0, 1.0]; // 1 um per unit
        header.orientation_quat = [0.0, 0.0, 0.0, 1.0]; // identity

        let world_m = header.local_i32_to_meters([1_000_000, 0, 0]); // 1 meter offset in X
        assert!((world_m[0] - 101.0).abs() < 1e-9);
        assert!((world_m[1] - 200.0).abs() < 1e-9);
        assert!((world_m[2] - 300.0).abs() < 1e-9);
    }
}
