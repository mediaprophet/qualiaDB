//! P6.7 — `.10d` reconstruction section: serialize meshes/complexes/operators
//! as canonical attestable bytes.
//!
//! This module defines a `.10d` section type for reconstruction outputs
//! (alpha shapes, isosurfaces, persistence barcodes, Laplacian operators).
//! Each output encodes to a self-describing section with per-section CRC-32C
//! and can be decoded back bit-identically.
//!
//! ## Format
//!
//! ```text
//! [4 bytes: magic "RCNS"]
//! [1 byte: version]
//! [1 byte: reconstruction type]
//! [2 bytes: flags]
//! [4 bytes: vertex count]
//! [4 bytes: triangle count]
//! [4 bytes: extra data length]
//! [vertex_count * 24 bytes: vertices (f64 x,y,z)]
//! [triangle_count * 12 bytes: triangles (u32 x3)]
//! [extra_data_length bytes: extra data (type-specific)]
//! [4 bytes: CRC-32C of all preceding bytes]
//! ```

use super::primitives::Point3;

// ───────────────────────────────────────────────────────────────────────────
//  Constants
// ───────────────────────────────────────────────────────────────────────────

/// Magic bytes for reconstruction sections: "RCNS".
pub const RECON_MAGIC: [u8; 4] = *b"RCNS";

/// Current version of the reconstruction section format.
pub const RECON_VERSION: u8 = 1;

/// Reconstruction section types.
pub const RECON_TYPE_ALPHA_SHAPE_2D: u8 = 0;
pub const RECON_TYPE_ALPHA_SHAPE_3D: u8 = 1;
pub const RECON_TYPE_ISOSURFACE: u8 = 2;
pub const RECON_TYPE_PERSISTENCE: u8 = 3;
pub const RECON_TYPE_LAPLACIAN: u8 = 4;

/// Header size (fixed part before vertex/triangle data).
pub const RECON_HEADER_SIZE: usize = 4 + 1 + 1 + 2 + 4 + 4 + 4;

/// CRC-32C polynomial (Castagnoli).
const CRC32C_POLY: u32 = 0x82F63B78;

// ───────────────────────────────────────────────────────────────────────────
//  Errors
// ───────────────────────────────────────────────────────────────────────────

/// Reconstruction section error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconSectionError {
    /// Payload too short for header.
    PayloadTooShort { got: usize, need: usize },
    /// Magic mismatch.
    MagicMismatch { got: [u8; 4] },
    /// Unsupported version.
    UnsupportedVersion { got: u8 },
    /// Unknown reconstruction type.
    UnknownType { got: u8 },
    /// Payload doesn't match declared sizes.
    SizeMismatch { expected: usize, got: usize },
    /// CRC-32C mismatch.
    CrcMismatch { expected: u32, got: u32 },
}

impl core::fmt::Display for ReconSectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PayloadTooShort { got, need } => {
                write!(f, "recon: payload too short, {got} < {need}")
            }
            Self::MagicMismatch { got } => write!(f, "recon: magic mismatch, got {:?}", got),
            Self::UnsupportedVersion { got } => write!(f, "recon: unsupported version {got}"),
            Self::UnknownType { got } => write!(f, "recon: unknown type {got}"),
            Self::SizeMismatch { expected, got } => {
                write!(f, "recon: size mismatch, expected {expected}, got {got}")
            }
            Self::CrcMismatch { expected, got } => write!(
                f,
                "recon: CRC mismatch, expected {expected:#x}, got {got:#x}"
            ),
        }
    }
}

impl std::error::Error for ReconSectionError {}

// ───────────────────────────────────────────────────────────────────────────
//  CRC-32C (Castagnoli)
// ───────────────────────────────────────────────────────────────────────────

/// Compute CRC-32C over a byte slice.
pub fn crc32c(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32C_POLY;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

// ───────────────────────────────────────────────────────────────────────────
//  Encoding
// ───────────────────────────────────────────────────────────────────────────

/// Encode a reconstruction mesh (vertices + triangles) as a `.10d`
/// reconstruction section.
///
/// `out` needs `RECON_HEADER_SIZE + vertices.len() * 24 + triangles.len() * 12 + 4` bytes.
///
/// Returns the number of bytes written.
pub fn encode_recon_section(
    recon_type: u8,
    flags: u16,
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    extra_data: &[u8],
    out: &mut [u8],
) -> Result<usize, ReconSectionError> {
    let needed =
        RECON_HEADER_SIZE + vertices.len() * 24 + triangles.len() * 12 + extra_data.len() + 4;
    if out.len() < needed {
        return Err(ReconSectionError::PayloadTooShort {
            got: out.len(),
            need: needed,
        });
    }

    let mut offset = 0usize;

    // Magic.
    out[offset..offset + 4].copy_from_slice(&RECON_MAGIC);
    offset += 4;

    // Version.
    out[offset] = RECON_VERSION;
    offset += 1;

    // Type.
    out[offset] = recon_type;
    offset += 1;

    // Flags.
    out[offset..offset + 2].copy_from_slice(&flags.to_le_bytes());
    offset += 2;

    // Vertex count.
    out[offset..offset + 4].copy_from_slice(&(vertices.len() as u32).to_le_bytes());
    offset += 4;

    // Triangle count.
    out[offset..offset + 4].copy_from_slice(&(triangles.len() as u32).to_le_bytes());
    offset += 4;

    // Extra data length.
    out[offset..offset + 4].copy_from_slice(&(extra_data.len() as u32).to_le_bytes());
    offset += 4;

    // Vertices (f64 x, y, z).
    for v in vertices {
        out[offset..offset + 8].copy_from_slice(&v.x.to_le_bytes());
        offset += 8;
        out[offset..offset + 8].copy_from_slice(&v.y.to_le_bytes());
        offset += 8;
        out[offset..offset + 8].copy_from_slice(&v.z.to_le_bytes());
        offset += 8;
    }

    // Triangles (u32 x3).
    for t in triangles {
        out[offset..offset + 4].copy_from_slice(&t[0].to_le_bytes());
        offset += 4;
        out[offset..offset + 4].copy_from_slice(&t[1].to_le_bytes());
        offset += 4;
        out[offset..offset + 4].copy_from_slice(&t[2].to_le_bytes());
        offset += 4;
    }

    // Extra data.
    out[offset..offset + extra_data.len()].copy_from_slice(extra_data);
    offset += extra_data.len();

    // CRC-32C.
    let crc = crc32c(&out[..offset]);
    out[offset..offset + 4].copy_from_slice(&crc.to_le_bytes());
    offset += 4;

    Ok(offset)
}

// ───────────────────────────────────────────────────────────────────────────
//  Decoding
// ───────────────────────────────────────────────────────────────────────────

/// Decoded reconstruction section.
pub struct DecodedRecon {
    pub recon_type: u8,
    pub flags: u16,
    pub vertices: Vec<Point3>,
    pub triangles: Vec<[u32; 3]>,
    pub extra_data: Vec<u8>,
}

/// Decode a `.10d` reconstruction section from bytes.
///
/// Verifies magic, version, and CRC-32C. Returns the decoded data.
pub fn decode_recon_section(bytes: &[u8]) -> Result<DecodedRecon, ReconSectionError> {
    if bytes.len() < RECON_HEADER_SIZE + 4 {
        return Err(ReconSectionError::PayloadTooShort {
            got: bytes.len(),
            need: RECON_HEADER_SIZE + 4,
        });
    }

    let mut offset = 0usize;

    // Magic.
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[offset..offset + 4]);
    offset += 4;
    if magic != RECON_MAGIC {
        return Err(ReconSectionError::MagicMismatch { got: magic });
    }

    // Version.
    let version = bytes[offset];
    offset += 1;
    if version != RECON_VERSION {
        return Err(ReconSectionError::UnsupportedVersion { got: version });
    }

    // Type.
    let recon_type = bytes[offset];
    offset += 1;
    if recon_type > RECON_TYPE_LAPLACIAN {
        return Err(ReconSectionError::UnknownType { got: recon_type });
    }

    // Flags.
    let flags = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;

    // Vertex count.
    let vert_count = u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]) as usize;
    offset += 4;

    // Triangle count.
    let tri_count = u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]) as usize;
    offset += 4;

    // Extra data length.
    let extra_len = u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]) as usize;
    offset += 4;

    // Verify total size.
    let expected = RECON_HEADER_SIZE + vert_count * 24 + tri_count * 12 + extra_len + 4;
    if bytes.len() < expected {
        return Err(ReconSectionError::SizeMismatch {
            expected,
            got: bytes.len(),
        });
    }

    // CRC-32C verification.
    let data_end = bytes.len() - 4;
    let stored_crc = u32::from_le_bytes([
        bytes[data_end],
        bytes[data_end + 1],
        bytes[data_end + 2],
        bytes[data_end + 3],
    ]);
    let computed_crc = crc32c(&bytes[..data_end]);
    if stored_crc != computed_crc {
        return Err(ReconSectionError::CrcMismatch {
            expected: stored_crc,
            got: computed_crc,
        });
    }

    // Vertices.
    let mut vertices = Vec::with_capacity(vert_count);
    for _ in 0..vert_count {
        let x = f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;
        let y = f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;
        let z = f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;
        vertices.push(Point3::new(x, y, z));
    }

    // Triangles.
    let mut triangles = Vec::with_capacity(tri_count);
    for _ in 0..tri_count {
        let a = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        let b = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        let c = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += 4;
        triangles.push([a, b, c]);
    }

    // Extra data.
    let extra_data = bytes[offset..offset + extra_len].to_vec();

    Ok(DecodedRecon {
        recon_type,
        flags,
        vertices,
        triangles,
        extra_data,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Hash
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a hash over encoded reconstruction section bytes.
pub fn recon_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mesh() -> (Vec<Point3>, Vec<[u32; 3]>) {
        let verts = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ];
        let tris = vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];
        (verts, tris)
    }

    #[test]
    fn encode_decode_round_trip() {
        let (verts, tris) = sample_mesh();
        let needed = RECON_HEADER_SIZE + verts.len() * 24 + tris.len() * 12 + 4;
        let mut buf = vec![0u8; needed];

        let written =
            encode_recon_section(RECON_TYPE_ISOSURFACE, 0x0001, &verts, &tris, &[], &mut buf)
                .unwrap();
        assert_eq!(written, needed);

        let decoded = decode_recon_section(&buf).unwrap();
        assert_eq!(decoded.recon_type, RECON_TYPE_ISOSURFACE);
        assert_eq!(decoded.flags, 0x0001);
        assert_eq!(decoded.vertices, verts);
        assert_eq!(decoded.triangles, tris);
        assert!(decoded.extra_data.is_empty());
    }

    #[test]
    fn encode_decode_with_extra_data() {
        let (verts, tris) = sample_mesh();
        let extra = [0xDE, 0xAD, 0xBE, 0xEFu8];
        let needed = RECON_HEADER_SIZE + verts.len() * 24 + tris.len() * 12 + extra.len() + 4;
        let mut buf = vec![0u8; needed];

        encode_recon_section(
            RECON_TYPE_ALPHA_SHAPE_3D,
            0,
            &verts,
            &tris,
            &extra,
            &mut buf,
        )
        .unwrap();

        let decoded = decode_recon_section(&buf).unwrap();
        assert_eq!(decoded.recon_type, RECON_TYPE_ALPHA_SHAPE_3D);
        assert_eq!(decoded.extra_data, extra);
    }

    #[test]
    fn encode_bit_identical_determinism() {
        let (verts, tris) = sample_mesh();
        let needed = RECON_HEADER_SIZE + verts.len() * 24 + tris.len() * 12 + 4;
        let mut buf1 = vec![0u8; needed];
        let mut buf2 = vec![0u8; needed];

        let w1 =
            encode_recon_section(RECON_TYPE_ISOSURFACE, 0, &verts, &tris, &[], &mut buf1).unwrap();
        let w2 =
            encode_recon_section(RECON_TYPE_ISOSURFACE, 0, &verts, &tris, &[], &mut buf2).unwrap();

        assert_eq!(w1, w2);
        assert_eq!(buf1, buf2);
        assert_eq!(recon_hash(&buf1), recon_hash(&buf2));
    }

    #[test]
    fn crc_catches_corruption() {
        let (verts, tris) = sample_mesh();
        let needed = RECON_HEADER_SIZE + verts.len() * 24 + tris.len() * 12 + 4;
        let mut buf = vec![0u8; needed];

        encode_recon_section(RECON_TYPE_ISOSURFACE, 0, &verts, &tris, &[], &mut buf).unwrap();

        // Flip a bit in the vertex data.
        buf[RECON_HEADER_SIZE + 5] ^= 0x01;

        assert!(matches!(
            decode_recon_section(&buf),
            Err(ReconSectionError::CrcMismatch { .. })
        ));
    }

    #[test]
    fn magic_mismatch_detected() {
        let (verts, tris) = sample_mesh();
        let needed = RECON_HEADER_SIZE + verts.len() * 24 + tris.len() * 12 + 4;
        let mut buf = vec![0u8; needed];

        encode_recon_section(RECON_TYPE_ISOSURFACE, 0, &verts, &tris, &[], &mut buf).unwrap();

        // Corrupt magic.
        buf[0] = b'X';

        assert!(matches!(
            decode_recon_section(&buf),
            Err(ReconSectionError::MagicMismatch { .. })
        ));
    }

    #[test]
    fn empty_mesh_round_trip() {
        let verts: Vec<Point3> = vec![];
        let tris: Vec<[u32; 3]> = vec![];
        let needed = RECON_HEADER_SIZE + 4;
        let mut buf = vec![0u8; needed];

        encode_recon_section(RECON_TYPE_LAPLACIAN, 0, &verts, &tris, &[], &mut buf).unwrap();

        let decoded = decode_recon_section(&buf).unwrap();
        assert!(decoded.vertices.is_empty());
        assert!(decoded.triangles.is_empty());
    }

    #[test]
    fn crc32c_known_values() {
        // CRC-32C of empty input.
        assert_eq!(crc32c(&[]), 0);
        // CRC-32C of "123456789" is 0xE3069283 (standard test vector).
        assert_eq!(crc32c(b"123456789"), 0xE3069283);
    }
}
