//! Mode 0x05: Fixed 64-Byte Biological Constitutive & Densitometric Record (`M64BioTissueRecord`).
//!
//! Represents spatially varying bone density, trabecular fabric, muscle fiber orientation,
//! myosteatosis, and SDF fascia sliding in 64-byte cache-aligned bricks (`16x16x16` Hilbert
//! bricks in `LRM3` anatomical micrometer coordinates). Per-record CRC-32C occupies bytes
//! `60..64`, matching `M64ShellRecord`, `M64StrataColumnRecord`, and `M64PhaseJunctureRecord`.

use super::chunk_directory::M64Error;
use crate::container_10d::crc32c::crc32c;
use half::f16;

pub const BIO_TISSUE_RECORD_BYTES: usize = 64;

pub const TISSUE_CLASS_EMPTY: u16 = 0;
pub const TISSUE_CLASS_CORTICAL_BONE: u16 = 1;
pub const TISSUE_CLASS_TRABECULAR_BONE: u16 = 2;
pub const TISSUE_CLASS_HYALINE_CARTILAGE: u16 = 3;
pub const TISSUE_CLASS_CALCIFIED_TESSERAE: u16 = 4;
pub const TISSUE_CLASS_CUTICULAR_SCLERITE: u16 = 5;
pub const TISSUE_CLASS_SKELETAL_MUSCLE: u16 = 6;
pub const TISSUE_CLASS_MYOCARDIUM: u16 = 7;
pub const TISSUE_CLASS_FASCIA_APONEUROSIS: u16 = 8;
pub const TISSUE_CLASS_ADIPOSE: u16 = 9;
pub const TISSUE_CLASS_LUMEN: u16 = 10;
pub const TISSUE_CLASS_EPICARDIAL_FAT: u16 = 11;

pub const FLAG_REMODELING_INTACT: u16 = 0;
pub const FLAG_REMODELING_OSTEOPENIC: u16 = 1 << 0;
pub const FLAG_REMODELING_OSTEOPOROTIC: u16 = 1 << 1;
pub const FLAG_REMODELING_SOFT_CALLUS: u16 = 1 << 2;
pub const FLAG_REMODELING_HARD_CALLUS: u16 = 1 << 3;
pub const FLAG_REMODELING_SCLEROTIC: u16 = 1 << 4;
pub const FLAG_REMODELING_IMPLANT: u16 = 1 << 5;

/// Mode 0x05: Fixed 64-Byte Biological Constitutive, Densitometric & Fiber Record.
///
/// Fits in a single CPU L1 cache line and maps 1:1 to WGSL `array<vec4<u32>, 4>`.
/// Zero implicit padding bytes; `crc32c` covers bytes `0..60`.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct M64BioTissueRecord {
    /// 8B (0x00..0x08): Packed cell/element header (SKL2 element index, voxel local coords, flags).
    pub header: u64,
    /// 2B (0x08..0x0A): Canonical anatomical tissue class identifier.
    pub tissue_class_id: u16,
    /// 2B (0x0A..0x0C): Ontogeny, remodeling, healing, and pathology state flags.
    pub ontogeny_pathology_flags: u16,
    /// 2B (0x0C..0x0E): Apparent wet density rho_app in kg/m^3.
    pub apparent_density_kg_m3: u16,
    /// 2B (0x0E..0x10): Quantitative CT volumetric BMD in mg HA/cm^3.
    pub mineral_vbmd_mg_cm3: u16,
    /// 2B (0x10..0x12): Bone volume fraction (BV/TV) or porosity in Q16 (0..65535 -> 0.0..1.0).
    pub porosity_bv_tv_q16: u16,
    /// 2B (0x12..0x14): Degree of mineralization / calcification ratio in Q16 (0..65535 -> 0.0..1.0).
    pub mineralization_ratio_q16: u16,
    /// 2B (0x14..0x16): Principal longitudinal Young's modulus E_1 in f16 MPa.
    pub elastic_e1_mpa_f16: u16,
    /// 2B (0x16..0x18): Transverse Young's modulus E_2 in f16 MPa.
    pub elastic_e2_mpa_f16: u16,
    /// 2B (0x18..0x1A): Out-of-plane Young's modulus E_3 in f16 MPa.
    pub elastic_e3_mpa_f16: u16,
    /// 2B (0x1A..0x1C): In-plane shear modulus G_12 in f16 MPa.
    pub shear_g12_mpa_f16: u16,
    /// 8B (0x1C..0x24): Orthotropic trabecular/collagen fabric orientation quaternion (x, y, z, w) in SNORM16.
    pub fabric_quat_snorm16: [i16; 4],
    /// 4B (0x24..0x28): Primary fiber/osteon/fascicle direction on S^2 as octahedral 16-bit pair.
    pub fiber_dir_oct16: [i16; 2],
    /// 1B (0x28..0x29): Skeletal muscle CT radiodensity in signed HU (clamped -128..127).
    pub muscle_hu_q8: i8,
    /// 1B (0x29..0x2A): Intramuscular adipose tissue (IMAT / PDFF) fraction in Q8 (0..255 -> 0.0..1.0).
    pub imat_frac_q8: u8,
    /// 2B (0x2A..0x2C): Signed distance to nearest anatomical fascia/aponeurosis plane in f16 mm.
    pub fascia_sdf_mm_f16: u16,
    /// 2B (0x2C..0x2E): Normalized tissue perfusion / metabolic activity in Q16 (0..65535 -> 0.0..1.0).
    pub perfusion_metabolic_q16: u16,
    /// 2B (0x2E..0x30): Explicit alignment padding / reserved bytes (must be zero).
    pub reserved: [u8; 2],
    /// 8B (0x30..0x38): Packed epistemic tier, observation method, observer role, stability & confidence.
    pub epistemic_method_packed: u64,
    /// 4B (0x38..0x3C): Subsurface optical/radiometric albedo + scattering index into Banked LUT.
    pub optical_lut_index: u32,
    /// 4B (0x3C..0x40): CRC-32C checksum of bytes 0..60 (Total = 64B).
    pub crc32c: u32,
}

impl Default for M64BioTissueRecord {
    fn default() -> Self {
        Self {
            header: 0,
            tissue_class_id: TISSUE_CLASS_EMPTY,
            ontogeny_pathology_flags: FLAG_REMODELING_INTACT,
            apparent_density_kg_m3: 0,
            mineral_vbmd_mg_cm3: 0,
            porosity_bv_tv_q16: 0,
            mineralization_ratio_q16: 0,
            elastic_e1_mpa_f16: 0,
            elastic_e2_mpa_f16: 0,
            elastic_e3_mpa_f16: 0,
            shear_g12_mpa_f16: 0,
            fabric_quat_snorm16: [0, 0, 0, i16::MAX],
            fiber_dir_oct16: [0, 0],
            muscle_hu_q8: 0,
            imat_frac_q8: 0,
            fascia_sdf_mm_f16: 0,
            perfusion_metabolic_q16: 0,
            reserved: [0; 2],
            epistemic_method_packed: 0,
            optical_lut_index: 0,
            crc32c: 0,
        }
    }
}

impl M64BioTissueRecord {
    /// Compute the CRC-32C over the first 60 bytes of the record.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; BIO_TISSUE_RECORD_BYTES] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.crc32c = self.compute_crc32c();
    }

    /// Validate the CRC-32C of this biological tissue record.
    #[inline]
    pub fn validate(&self) -> Result<(), M64Error> {
        let computed = self.compute_crc32c();
        if self.crc32c == computed {
            Ok(())
        } else {
            Err(M64Error::ChecksumMismatch {
                stored: self.crc32c,
                computed,
            })
        }
    }

    #[inline]
    pub fn set_elastic_moduli_mpa(&mut self, e1: f32, e2: f32, e3: f32, g12: f32) {
        self.elastic_e1_mpa_f16 = f16::from_f32(e1).to_bits();
        self.elastic_e2_mpa_f16 = f16::from_f32(e2).to_bits();
        self.elastic_e3_mpa_f16 = f16::from_f32(e3).to_bits();
        self.shear_g12_mpa_f16 = f16::from_f32(g12).to_bits();
    }

    #[inline]
    pub fn elastic_e1_mpa(&self) -> f32 {
        f16::from_bits(self.elastic_e1_mpa_f16).to_f32()
    }

    #[inline]
    pub fn elastic_e2_mpa(&self) -> f32 {
        f16::from_bits(self.elastic_e2_mpa_f16).to_f32()
    }

    #[inline]
    pub fn elastic_e3_mpa(&self) -> f32 {
        f16::from_bits(self.elastic_e3_mpa_f16).to_f32()
    }

    #[inline]
    pub fn shear_g12_mpa(&self) -> f32 {
        f16::from_bits(self.shear_g12_mpa_f16).to_f32()
    }

    #[inline]
    pub fn set_fascia_sdf_mm(&mut self, sdf_mm: f32) {
        self.fascia_sdf_mm_f16 = f16::from_f32(sdf_mm).to_bits();
    }

    #[inline]
    pub fn fascia_sdf_mm(&self) -> f32 {
        f16::from_bits(self.fascia_sdf_mm_f16).to_f32()
    }

    #[inline]
    pub fn porosity_bv_tv(&self) -> f32 {
        f32::from(self.porosity_bv_tv_q16) / 65535.0
    }

    #[inline]
    pub fn set_porosity_bv_tv(&mut self, ratio: f32) {
        self.porosity_bv_tv_q16 = (ratio.clamp(0.0, 1.0) * 65535.0).round() as u16;
    }

    #[inline]
    pub fn mineralization_ratio(&self) -> f32 {
        f32::from(self.mineralization_ratio_q16) / 65535.0
    }

    #[inline]
    pub fn set_mineralization_ratio(&mut self, ratio: f32) {
        self.mineralization_ratio_q16 = (ratio.clamp(0.0, 1.0) * 65535.0).round() as u16;
    }

    #[inline]
    pub fn imat_fraction(&self) -> f32 {
        f32::from(self.imat_frac_q8) / 255.0
    }

    #[inline]
    pub fn set_imat_fraction(&mut self, ratio: f32) {
        self.imat_frac_q8 = (ratio.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    #[inline]
    pub fn perfusion_metabolic(&self) -> f32 {
        f32::from(self.perfusion_metabolic_q16) / 65535.0
    }

    #[inline]
    pub fn set_perfusion_metabolic(&mut self, ratio: f32) {
        self.perfusion_metabolic_q16 = (ratio.clamp(0.0, 1.0) * 65535.0).round() as u16;
    }

    /// Encode a unit 3D fiber direction vector onto the octahedron in `[-32767, 32767]^2`.
    #[inline]
    pub fn set_fiber_dir(&mut self, dir: [f32; 3]) {
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let n = if len > 1e-8 {
            [dir[0] / len, dir[1] / len, dir[2] / len]
        } else {
            [0.0, 0.0, 1.0]
        };
        let l1 = n[0].abs() + n[1].abs() + n[2].abs();
        let mut ox = n[0] / l1;
        let mut oy = n[1] / l1;
        if n[2] < 0.0 {
            let sx = if ox >= 0.0 { 1.0 } else { -1.0 };
            let sy = if oy >= 0.0 { 1.0 } else { -1.0 };
            let nx = (1.0 - oy.abs()) * sx;
            let ny = (1.0 - ox.abs()) * sy;
            ox = nx;
            oy = ny;
        }
        self.fiber_dir_oct16 = [
            (ox.clamp(-1.0, 1.0) * 32767.0).round() as i16,
            (oy.clamp(-1.0, 1.0) * 32767.0).round() as i16,
        ];
    }

    /// Decode the octahedral 16-bit fiber direction back into a unit 3D vector.
    #[inline]
    pub fn fiber_dir(&self) -> [f32; 3] {
        let ox = f32::from(self.fiber_dir_oct16[0]) / 32767.0;
        let oy = f32::from(self.fiber_dir_oct16[1]) / 32767.0;
        let z = 1.0 - ox.abs() - oy.abs();
        let (x, y) = if z < 0.0 {
            let sx = if ox >= 0.0 { 1.0 } else { -1.0 };
            let sy = if oy >= 0.0 { 1.0 } else { -1.0 };
            ((1.0 - oy.abs()) * sx, (1.0 - ox.abs()) * sy)
        } else {
            (ox, oy)
        };
        let len = (x * x + y * y + z * z).sqrt();
        if len > 1e-8 {
            [x / len, y / len, z / len]
        } else {
            [0.0, 0.0, 1.0]
        }
    }

    /// Encode a unit quaternion `[x, y, z, w]` into `fabric_quat_snorm16`.
    #[inline]
    pub fn set_fabric_quat(&mut self, q: [f32; 4]) {
        let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        let n = if len > 1e-8 {
            [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        self.fabric_quat_snorm16 = [
            (n[0].clamp(-1.0, 1.0) * 32767.0).round() as i16,
            (n[1].clamp(-1.0, 1.0) * 32767.0).round() as i16,
            (n[2].clamp(-1.0, 1.0) * 32767.0).round() as i16,
            (n[3].clamp(-1.0, 1.0) * 32767.0).round() as i16,
        ];
    }

    /// Decode `fabric_quat_snorm16` into a normalized `[x, y, z, w]` quaternion.
    #[inline]
    pub fn fabric_quat(&self) -> [f32; 4] {
        let q = [
            f32::from(self.fabric_quat_snorm16[0]) / 32767.0,
            f32::from(self.fabric_quat_snorm16[1]) / 32767.0,
            f32::from(self.fabric_quat_snorm16[2]) / 32767.0,
            f32::from(self.fabric_quat_snorm16[3]) / 32767.0,
        ];
        let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        if len > 1e-8 {
            [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
        } else {
            [0.0, 0.0, 0.0, 1.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m64_bio_tissue_record_size_and_alignment() {
        assert_eq!(std::mem::size_of::<M64BioTissueRecord>(), 64);
        assert_eq!(std::mem::align_of::<M64BioTissueRecord>(), 64);
    }

    #[test]
    fn test_m64_bio_tissue_record_crc_roundtrip_and_corruption() {
        let mut rec = M64BioTissueRecord::default();
        rec.header = 0x0001_0002_0003_0004;
        rec.tissue_class_id = TISSUE_CLASS_CORTICAL_BONE;
        rec.apparent_density_kg_m3 = 1850;
        rec.mineral_vbmd_mg_cm3 = 1120;
        rec.set_porosity_bv_tv(0.92);
        rec.set_mineralization_ratio(0.68);
        rec.set_elastic_moduli_mpa(18_000.0, 10_500.0, 10_200.0, 3_600.0);
        rec.set_fiber_dir([0.0, 0.6, 0.8]);
        rec.set_fabric_quat([0.0, 0.0, 0.0, 1.0]);
        rec.muscle_hu_q8 = 55;
        rec.set_imat_fraction(0.05);
        rec.set_fascia_sdf_mm(-1.25);
        rec.set_perfusion_metabolic(0.45);
        rec.epistemic_method_packed = 0xDEAD_BEEF_CAFE_BABE;
        rec.optical_lut_index = 42;
        rec.update_crc();

        assert!(rec.validate().is_ok());

        let bytes = bytemuck::bytes_of(&rec);
        let decoded: M64BioTissueRecord = bytemuck::pod_read_unaligned(bytes);
        assert_eq!(decoded, rec);
        assert!((decoded.elastic_e1_mpa() - 18_000.0).abs() < 32.0);
        assert!((decoded.porosity_bv_tv() - 0.92).abs() < 1e-4);
        let dir = decoded.fiber_dir();
        assert!((dir[1] - 0.6).abs() < 1e-3 && (dir[2] - 0.8).abs() < 1e-3);

        let mut corrupted = rec;
        corrupted.mineral_vbmd_mg_cm3 ^= 1;
        assert!(matches!(
            corrupted.validate(),
            Err(M64Error::ChecksumMismatch { .. })
        ));
    }
}
