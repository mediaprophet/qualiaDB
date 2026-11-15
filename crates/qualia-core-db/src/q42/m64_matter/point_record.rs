//! 64-Byte Aligned Point-Tensor Record (.m64 / .e64 Mode 0x00)
//!
//! Maps directly to a single CPU L1 cache line & binds zero-copy to WebGPU/Vulkan vertex buffers.
//! Used for LiDAR, geological strata samples, and geodetic sensor point-clouds.

use bytemuck::{Pod, Zeroable};

/// 64-Byte Aligned Point Record for LiDAR, geological strata samples, and sensor point-clouds.
#[repr(C, align(64))]
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
pub struct M64PointRecord {
    /// Local X offset in meters from chunk origin
    pub x: f32,
    /// Local Y offset in meters from chunk origin
    pub y: f32,
    /// Elevation Z in meters relative to datum
    pub z: f32,
    /// Return intensity or sensor signal strength
    pub intensity: f32,
    /// Quantized surface normal vector [-32767..32767]
    pub normal: [i16; 3],
    /// ASPRS LiDAR classification (e.g. 2=Ground, 3=Low Veg, 5=High Veg, 9=Water)
    pub classification: u8,
    /// Pulse return index
    pub return_number: u8,
    /// 8-band spectral reflectance signature (visible + NIR + SWIR)
    pub spectral_sig: [u8; 8],
    /// Packed physical attributes: [density: u8, hardness: u8, hydraulic_porosity: u8, flags: u8]
    pub physical_props: u32,
    /// UNIX survey timestamp (epoch seconds)
    pub timestamp: u32,
    /// FNV-1a hash of data source or sensor DID
    pub provenance_did_hash: u64,
    /// Alignment padding & hardware-reserved space (16 bytes)
    pub _reserved: [u8; 16],
}

/// Backward-compatible type alias for E64PointRecord.
pub type E64PointRecord = M64PointRecord;

impl Default for M64PointRecord {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            intensity: 1.0,
            normal: [0, 0, 32767],
            classification: 2, // Default: Ground
            return_number: 1,
            spectral_sig: [180, 160, 140, 200, 210, 190, 170, 150],
            physical_props: 0x01_02_03_00,
            timestamp: 0,
            provenance_did_hash: 0,
            _reserved: [0; 16],
        }
    }
}

impl M64PointRecord {
    /// Returns the normalized normal vector as `[f32; 3]`.
    #[inline]
    pub fn unpack_normal(&self) -> [f32; 3] {
        const SCALE: f32 = 1.0 / 32767.0;
        [
            self.normal[0] as f32 * SCALE,
            self.normal[1] as f32 * SCALE,
            self.normal[2] as f32 * SCALE,
        ]
    }

    /// Sets the normal vector from normalized `[f32; 3]`.
    #[inline]
    pub fn pack_normal(&mut self, n: [f32; 3]) {
        self.normal = [
            (n[0].clamp(-1.0, 1.0) * 32767.0) as i16,
            (n[1].clamp(-1.0, 1.0) * 32767.0) as i16,
            (n[2].clamp(-1.0, 1.0) * 32767.0) as i16,
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m64_point_record_size_and_alignment() {
        assert_eq!(std::mem::size_of::<M64PointRecord>(), 64);
        assert_eq!(std::mem::align_of::<M64PointRecord>(), 64);
    }

    #[test]
    fn test_normal_packing() {
        let mut pt = M64PointRecord::default();
        pt.pack_normal([0.0, 0.0, 1.0]);
        let unpacked = pt.unpack_normal();
        assert!((unpacked[2] - 1.0).abs() < 1e-4);
        assert!(unpacked[0].abs() < 1e-4);
        assert!(unpacked[1].abs() < 1e-4);
    }
}
