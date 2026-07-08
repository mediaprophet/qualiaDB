//! P13.10 - FEM-ready mesh certificates and canonical encoding.
//!
//! The certificate is intentionally compact: it records solver-facing counts,
//! material/boundary metadata, quality extrema, adjacency counts, provenance,
//! and a CRC-guarded byte representation suitable for a `.10d` sidecar section.

use super::mixed_cell_topology::{
    extract_boundary_faces, validate_mixed_cells, BoundaryFace, MixedCell, MixedTopologyError,
};
use super::primitives::Point3;

pub const FEM_CERT_MAGIC: [u8; 4] = *b"FEMC";
pub const FEM_CERT_VERSION: u8 = 1;
pub const FEM_CERT_ENCODED_LEN: usize = 4 + 1 + 3 + 8 * 6 + 8 * 2 + 8 + 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryMarker {
    pub entity_id: u32,
    pub marker: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialRegion {
    pub material: u16,
    pub first_cell: u32,
    pub cell_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FemCertificate {
    pub vertex_count: u64,
    pub cell_count: u64,
    pub boundary_marker_count: u64,
    pub material_region_count: u64,
    pub boundary_face_count: u64,
    pub adjacency_face_count: u64,
    pub min_quality: f64,
    pub max_quality: f64,
    pub provenance_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FemCertificateError {
    Topology(MixedTopologyError),
    EmptyMesh,
    BoundaryMarkerOutOfBounds { entity_id: u32 },
    MaterialRegionOutOfBounds { first_cell: u32, cell_count: u32 },
    OutputTooSmall { required: usize },
    PayloadTooShort,
    BadMagic,
    UnsupportedVersion { got: u8 },
    CrcMismatch,
}

impl From<MixedTopologyError> for FemCertificateError {
    fn from(err: MixedTopologyError) -> Self {
        Self::Topology(err)
    }
}

pub fn build_fem_certificate(
    vertices: &[Point3],
    cells: &[MixedCell],
    boundary_markers: &[BoundaryMarker],
    material_regions: &[MaterialRegion],
    provenance_hash: u64,
) -> Result<FemCertificate, FemCertificateError> {
    if vertices.is_empty() || cells.is_empty() {
        return Err(FemCertificateError::EmptyMesh);
    }
    let topology = validate_mixed_cells(vertices, cells)?;
    let mut faces = vec![
        BoundaryFace {
            kind: 0,
            vertex_count: 0,
            material: 0,
            vertices: [0; 4],
            owner_cell: 0,
        };
        cells.len() * 6
    ];
    let boundary_face_count = extract_boundary_faces(cells, &mut faces)?;
    for marker in boundary_markers {
        if marker.entity_id as usize >= boundary_face_count {
            return Err(FemCertificateError::BoundaryMarkerOutOfBounds {
                entity_id: marker.entity_id,
            });
        }
    }
    for region in material_regions {
        if region.first_cell as usize >= cells.len()
            || region.first_cell as usize + region.cell_count as usize > cells.len()
        {
            return Err(FemCertificateError::MaterialRegionOutOfBounds {
                first_cell: region.first_cell,
                cell_count: region.cell_count,
            });
        }
    }

    let adjacency_face_count = (count_interior_face_incidences(cells) / 2) as u64;
    Ok(FemCertificate {
        vertex_count: vertices.len() as u64,
        cell_count: cells.len() as u64,
        boundary_marker_count: boundary_markers.len() as u64,
        material_region_count: material_regions.len() as u64,
        boundary_face_count: boundary_face_count as u64,
        adjacency_face_count,
        min_quality: topology.min_signed_volume,
        max_quality: topology.min_signed_volume,
        provenance_hash,
    })
}

pub fn encode_fem_certificate(
    cert: &FemCertificate,
    out: &mut [u8],
) -> Result<usize, FemCertificateError> {
    if out.len() < FEM_CERT_ENCODED_LEN {
        return Err(FemCertificateError::OutputTooSmall {
            required: FEM_CERT_ENCODED_LEN,
        });
    }
    out[..4].copy_from_slice(&FEM_CERT_MAGIC);
    out[4] = FEM_CERT_VERSION;
    out[5..8].fill(0);
    let mut offset = 8usize;
    for v in [
        cert.vertex_count,
        cert.cell_count,
        cert.boundary_marker_count,
        cert.material_region_count,
        cert.boundary_face_count,
        cert.adjacency_face_count,
    ] {
        out[offset..offset + 8].copy_from_slice(&v.to_le_bytes());
        offset += 8;
    }
    out[offset..offset + 8].copy_from_slice(&cert.min_quality.to_le_bytes());
    offset += 8;
    out[offset..offset + 8].copy_from_slice(&cert.max_quality.to_le_bytes());
    offset += 8;
    out[offset..offset + 8].copy_from_slice(&cert.provenance_hash.to_le_bytes());
    offset += 8;
    let crc = crc32c(&out[..offset]);
    out[offset..offset + 4].copy_from_slice(&crc.to_le_bytes());
    Ok(FEM_CERT_ENCODED_LEN)
}

pub fn decode_fem_certificate(data: &[u8]) -> Result<FemCertificate, FemCertificateError> {
    if data.len() < FEM_CERT_ENCODED_LEN {
        return Err(FemCertificateError::PayloadTooShort);
    }
    if data[..4] != FEM_CERT_MAGIC {
        return Err(FemCertificateError::BadMagic);
    }
    if data[4] != FEM_CERT_VERSION {
        return Err(FemCertificateError::UnsupportedVersion { got: data[4] });
    }
    let stored = u32::from_le_bytes(
        data[FEM_CERT_ENCODED_LEN - 4..FEM_CERT_ENCODED_LEN]
            .try_into()
            .unwrap(),
    );
    let got = crc32c(&data[..FEM_CERT_ENCODED_LEN - 4]);
    if stored != got {
        return Err(FemCertificateError::CrcMismatch);
    }
    let mut offset = 8usize;
    let read_u64 = |data: &[u8], offset: &mut usize| {
        let v = u64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
        *offset += 8;
        v
    };
    let vertex_count = read_u64(data, &mut offset);
    let cell_count = read_u64(data, &mut offset);
    let boundary_marker_count = read_u64(data, &mut offset);
    let material_region_count = read_u64(data, &mut offset);
    let boundary_face_count = read_u64(data, &mut offset);
    let adjacency_face_count = read_u64(data, &mut offset);
    let min_quality = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let max_quality = f64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let provenance_hash = read_u64(data, &mut offset);
    Ok(FemCertificate {
        vertex_count,
        cell_count,
        boundary_marker_count,
        material_region_count,
        boundary_face_count,
        adjacency_face_count,
        min_quality,
        max_quality,
        provenance_hash,
    })
}

fn count_interior_face_incidences(cells: &[MixedCell]) -> usize {
    let mut faces = vec![
        BoundaryFace {
            kind: 0,
            vertex_count: 0,
            material: 0,
            vertices: [0; 4],
            owner_cell: 0,
        };
        cells.len() * 6
    ];
    let boundary = extract_boundary_faces(cells, &mut faces).unwrap_or(0);
    let total: usize = cells
        .iter()
        .map(|c| match c.kind {
            5 => 4,
            8 => 6,
            _ => 0,
        })
        .sum();
    total.saturating_sub(boundary)
}

fn crc32c(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0x82F63B78 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::super::mixed_cell_topology::MixedCell;
    use super::*;

    fn tet_fixture() -> (Vec<Point3>, Vec<MixedCell>) {
        (
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
                Point3::new(0.0, 0.0, 1.0),
            ],
            vec![MixedCell::tetra(0, 1, 2, 3, 7)],
        )
    }

    #[test]
    fn builds_certificate_for_tet_mesh() {
        let (v, c) = tet_fixture();
        let cert = build_fem_certificate(
            &v,
            &c,
            &[BoundaryMarker {
                entity_id: 0,
                marker: 11,
            }],
            &[MaterialRegion {
                material: 7,
                first_cell: 0,
                cell_count: 1,
            }],
            0xCAFE,
        )
        .unwrap();
        assert_eq!(cert.vertex_count, 4);
        assert_eq!(cert.cell_count, 1);
        assert_eq!(cert.boundary_face_count, 4);
        assert!(cert.min_quality > 0.0);
    }

    #[test]
    fn certificate_round_trips_canonically() {
        let (v, c) = tet_fixture();
        let cert = build_fem_certificate(&v, &c, &[], &[], 42).unwrap();
        let mut bytes = [0u8; FEM_CERT_ENCODED_LEN];
        let n = encode_fem_certificate(&cert, &mut bytes).unwrap();
        assert_eq!(n, FEM_CERT_ENCODED_LEN);
        let decoded = decode_fem_certificate(&bytes).unwrap();
        assert_eq!(decoded, cert);
        let mut bytes2 = [0u8; FEM_CERT_ENCODED_LEN];
        encode_fem_certificate(&decoded, &mut bytes2).unwrap();
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn crc_catches_corruption() {
        let (v, c) = tet_fixture();
        let cert = build_fem_certificate(&v, &c, &[], &[], 42).unwrap();
        let mut bytes = [0u8; FEM_CERT_ENCODED_LEN];
        encode_fem_certificate(&cert, &mut bytes).unwrap();
        bytes[12] ^= 0x40;
        assert_eq!(
            decode_fem_certificate(&bytes),
            Err(FemCertificateError::CrcMismatch)
        );
    }
}
