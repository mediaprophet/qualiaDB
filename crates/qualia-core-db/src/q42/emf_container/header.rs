//! EMF Container 64-byte Header and Cell Record Definitions.
//!
//! Provides the top-level 64-byte container header and 64-byte electromagnetic
//! wave-optical voxel cell records for `.emf` (`EMF-64`) containers.

use crate::container_10d::crc32c::crc32c;
use bytemuck::{Pod, Zeroable};

/// Magic identifier for .emf container files: ASCII `Q42EMF64`.
pub const EMF_CONTAINER_MAGIC: [u8; 8] = *b"Q42EMF64";

/// Current EMF container format version.
pub const EMF_CONTAINER_VERSION: u16 = 1;

/// Flag: chunks are organized as 8x8x8 3D volumetric bricks (512 cells = 32 KB).
pub const EMF_FLAG_3D_BRICK_8X8X8: u16 = 1 << 0;
/// Flag: full Stokes polarization vector (I, Q, U, V) is populated.
pub const EMF_FLAG_STOKES_POLARIZED: u16 = 1 << 1;
/// Flag: multi-spectral bands enabled.
pub const EMF_FLAG_SPECTRAL_BANDS: u16 = 1 << 2;
/// Flag: BLAKE3-Bao verified outboard hash tree enabled.
pub const EMF_FLAG_BAO_VERIFIED: u16 = 1 << 3;

/// Fail-closed decoded dense-volume cap: 8³ cells × 2048 bricks.
pub const MAX_DENSE_CELLS: usize = 1_048_576;

/// Permittivity of free space epsilon_0 in F/m.
pub const EPSILON_0: f32 = 8.854_187_8e-12;
/// Permeability of free space mu_0 in H/m (or N/A^2).
pub const MU_0: f32 = 1.256_637_1e-6;
/// Speed of light in vacuum c in m/s.
pub const SPEED_OF_LIGHT: f32 = 299_792_458.0;

/// Error variants for EMF container validation and decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmfError {
    InvalidMagic,
    InvalidVersion(u16),
    InvalidHeaderSize,
    ChecksumMismatch { stored: u32, computed: u32 },
    BaoHashMismatch { chunk_index: usize },
    BufferTooSmall { required: usize, provided: usize },
    FileByteLimit,
    ChunkLimitExceeded,
    PayloadOutOfBounds,
    PayloadOverlap,
    PayloadUnaligned,
    DirectoryBounds,
    DirectoryAlignment,
    ArithmeticOverflow,
    /// Directory Hilbert keys do not round-trip for any order in 1..=8
    /// inside `grid_dims_bricks`.
    HilbertOrder,
    /// Decoded `[t][z][y][x]` cell count exceeds [`MAX_DENSE_CELLS`].
    DenseVolumeCap { cells: usize, cap: usize },
}

impl std::fmt::Display for EmfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid EMF container magic"),
            Self::InvalidVersion(v) => write!(f, "Unsupported EMF version: {v}"),
            Self::InvalidHeaderSize => write!(f, "Invalid EMF header size"),
            Self::ChecksumMismatch { stored, computed } => {
                write!(f, "EMF CRC-32C mismatch: stored {stored:#010x}, computed {computed:#010x}")
            }
            Self::BaoHashMismatch { chunk_index } => {
                write!(f, "EMF Bao hash mismatch in chunk {chunk_index}")
            }
            Self::BufferTooSmall { required, provided } => {
                write!(f, "EMF buffer too small: required {required}, provided {provided}")
            }
            Self::FileByteLimit => write!(f, "EMF file exceeds max_file_bytes"),
            Self::ChunkLimitExceeded => write!(f, "EMF chunk limit exceeded"),
            Self::PayloadOutOfBounds => write!(f, "EMF payload slice out of bounds"),
            Self::PayloadOverlap => write!(f, "EMF payload ranges overlap"),
            Self::PayloadUnaligned => write!(f, "EMF payload is not 64-byte aligned"),
            Self::DirectoryBounds => write!(f, "EMF chunk directory out of bounds"),
            Self::DirectoryAlignment => write!(f, "EMF chunk directory not 64-byte aligned"),
            Self::ArithmeticOverflow => write!(f, "Arithmetic overflow in EMF calculation"),
            Self::HilbertOrder => write!(f, "EMF Hilbert brick order could not be inferred"),
            Self::DenseVolumeCap { cells, cap } => {
                write!(f, "EMF dense volume {cells} cells exceeds cap {cap}")
            }
        }
    }
}

impl std::error::Error for EmfError {}

/// Fixed 64-byte EMF Container Header (L1 cache-line DOD).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct EmfContainerHeader {
    /// 0x00..0x08: ASCII Magic `Q42EMF64`
    pub magic: [u8; 8],
    /// 0x08..0x0A: Container version (1)
    pub version: u16,
    /// 0x0A..0x0C: Container flags
    pub flags: u16,
    /// 0x0C..0x10: Total chunk / brick count
    pub chunk_count: u32,
    /// 0x10..0x18: Byte offset to chunk directory table
    pub directory_offset: u64,
    /// 0x18..0x20: Reference epoch timestamp in seconds
    pub reference_epoch_s: f64,
    /// 0x20..0x2C: Grid dimensions in bricks [nx, ny, nz]
    pub grid_dims_bricks: [u32; 3],
    /// 0x2C..0x30: Physical voxel spacing in meters
    pub voxel_size_meters: f32,
    /// 0x30..0x3C: Domain spatial origin in meters [ox, oy, oz]
    pub origin_meters: [f32; 3],
    /// 0x3C..0x40: CRC-32C checksum of bytes 0..60
    pub header_crc32c: u32,
}

impl Default for EmfContainerHeader {
    fn default() -> Self {
        Self {
            magic: EMF_CONTAINER_MAGIC,
            version: EMF_CONTAINER_VERSION,
            flags: EMF_FLAG_3D_BRICK_8X8X8,
            chunk_count: 0,
            directory_offset: 64,
            reference_epoch_s: 0.0,
            grid_dims_bricks: [0, 0, 0],
            voxel_size_meters: 1.0,
            origin_meters: [0.0, 0.0, 0.0],
            header_crc32c: 0,
        }
    }
}

impl EmfContainerHeader {
    /// Compute the CRC-32C over the first 60 bytes of the header.
    #[inline]
    pub fn compute_crc32c(&self) -> u32 {
        let bytes: &[u8; 64] = bytemuck::cast_ref(self);
        crc32c(&bytes[..60])
    }

    /// Update `header_crc32c` with the computed CRC-32C.
    #[inline]
    pub fn update_crc(&mut self) {
        self.header_crc32c = self.compute_crc32c();
    }

    /// Validate magic, version, and CRC-32C checksum.
    #[inline]
    pub fn validate(&self) -> Result<(), EmfError> {
        if self.magic != EMF_CONTAINER_MAGIC {
            return Err(EmfError::InvalidMagic);
        }
        if self.version != EMF_CONTAINER_VERSION {
            return Err(EmfError::InvalidVersion(self.version));
        }
        let computed = self.compute_crc32c();
        if self.header_crc32c == computed {
            Ok(())
        } else {
            Err(EmfError::ChecksumMismatch {
                stored: self.header_crc32c,
                computed,
            })
        }
    }
}

/// 64-byte Electromagnetic & Wave-Optical Voxel Cell Record (16 × f32).
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct EmfCellRecord64 {
    /// Electric field vector [Ex, Ey, Ez] in V/m
    pub e_field: [f32; 3],
    /// Magnetic field vector [Bx, By, Bz] in Tesla
    pub b_field: [f32; 3],
    /// Poynting flux magnitude in W/m²
    pub poynting_flux: f32,
    /// Optical/EM wavelength in nm
    pub wavelength_nm: f32,
    /// Real refractive index n
    pub refractive_n: f32,
    /// Extinction coefficient kappa
    pub extinction_k: f32,
    /// Coherence phase in radians
    pub coherence_phase: f32,
    /// Stokes parameter Q
    pub stokes_q: f32,
    /// Stokes parameter U
    pub stokes_u: f32,
    /// Stokes parameter V
    pub stokes_v: f32,
    /// Spectral band index and intensity
    pub spectral_sigma: f32,
    /// Measurement / simulation confidence weight [0.0..1.0]
    pub confidence_alpha: f32,
}

impl Default for EmfCellRecord64 {
    fn default() -> Self {
        Self {
            e_field: [0.0, 0.0, 0.0],
            b_field: [0.0, 0.0, 0.0],
            poynting_flux: 0.0,
            wavelength_nm: 0.0,
            refractive_n: 1.0,
            extinction_k: 0.0,
            coherence_phase: 0.0,
            stokes_q: 0.0,
            stokes_u: 0.0,
            stokes_v: 0.0,
            spectral_sigma: 0.0,
            confidence_alpha: 1.0,
        }
    }
}

impl EmfCellRecord64 {
    /// Upcast a legacy 52-byte record (13 × f32) into a 64-byte aligned cell record.
    ///
    /// Upcasts `[amplitude, phase, frequency, manifold[10]]` to full 64B electromagnetic state.
    pub fn from_legacy_52b(legacy: &[f32; 13]) -> Self {
        let amp = legacy[0];
        let phase = legacy[1];
        let freq = legacy[2];
        let wavelength_nm = if freq > 0.0 {
            (SPEED_OF_LIGHT as f64 * 1e9 / (freq as f64)) as f32
        } else {
            0.0
        };
        let e_field = [amp, 0.0, 0.0];
        let b_mag = amp / SPEED_OF_LIGHT;
        let b_field = [0.0, b_mag, 0.0];
        let poynting_flux = (amp * amp) / (SPEED_OF_LIGHT * MU_0);

        Self {
            e_field,
            b_field,
            poynting_flux,
            wavelength_nm,
            refractive_n: 1.0,
            extinction_k: 0.0,
            coherence_phase: phase,
            stokes_q: legacy[3],
            stokes_u: legacy[4],
            stokes_v: legacy[5],
            spectral_sigma: legacy[6],
            confidence_alpha: 1.0,
        }
    }

    /// Compute Poynting vector S = (E x B) / mu_0 in W/m².
    #[inline]
    pub fn poynting_vector(&self) -> [f32; 3] {
        let ex = self.e_field[0];
        let ey = self.e_field[1];
        let ez = self.e_field[2];
        let bx = self.b_field[0];
        let by = self.b_field[1];
        let bz = self.b_field[2];

        let cx = ey * bz - ez * by;
        let cy = ez * bx - ex * bz;
        let cz = ex * by - ey * bx;

        [cx / MU_0, cy / MU_0, cz / MU_0]
    }

    /// Magnitude of the computed Poynting flux vector |S|.
    #[inline]
    pub fn poynting_flux_magnitude(&self) -> f32 {
        let s = self.poynting_vector();
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    }

    /// |E| in V/m.
    #[inline]
    pub fn e_magnitude(&self) -> f32 {
        (self.e_field[0] * self.e_field[0]
            + self.e_field[1] * self.e_field[1]
            + self.e_field[2] * self.e_field[2])
            .sqrt()
    }

    /// Optical frequency in Hz from `wavelength_nm`, or 0 when the cell is quasi-static.
    #[inline]
    pub fn optical_frequency_hz(&self) -> f32 {
        if self.wavelength_nm > 0.0 {
            (SPEED_OF_LIGHT as f64 / (self.wavelength_nm as f64 * 1e-9)) as f32
        } else {
            0.0
        }
    }

    /// σ used by AcousticPlane and CIE display. Authored `spectral_sigma` wins;
    /// otherwise fold 400–700 nm into [0, 1] the same way `sigma_to_wavelength_nm` inverts.
    #[inline]
    pub fn effective_sigma(&self) -> f32 {
        if self.spectral_sigma != 0.0 {
            self.spectral_sigma
        } else if self.wavelength_nm > 0.0 {
            ((self.wavelength_nm - 400.0) / 300.0).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Total electromagnetic energy density u_EM = 0.5 * (epsilon_0 * |E|^2 + |B|^2 / mu_0) in J/m³.
    #[inline]
    pub fn energy_density(&self) -> f32 {
        let e_sq = self.e_field[0] * self.e_field[0]
            + self.e_field[1] * self.e_field[1]
            + self.e_field[2] * self.e_field[2];
        let b_sq = self.b_field[0] * self.b_field[0]
            + self.b_field[1] * self.b_field[1]
            + self.b_field[2] * self.b_field[2];

        0.5 * (EPSILON_0 * e_sq + b_sq / MU_0)
    }
}
