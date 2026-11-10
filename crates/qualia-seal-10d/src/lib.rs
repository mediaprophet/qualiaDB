//! Seal a triangle mesh through the official QualiaDB `.10d` compiler.
//!
//! This crate does not define a parallel container. It calls
//! [`compile_mesh_to_10d_with_extras_and_frames`] then attaches SKL1 with
//! [`encode_skeletal_section_into`] and [`attach_skeletal_section_into`].
//! Occupancy optionally seals sibling `.m64` / `.emf` and attaches VOX0, FEMC,
//! MRP1, FIEL, and MID2 `FIELD_KIND_ELECTROMAGNETIC` using the existing codecs.

mod attach;
mod cardiac;
mod occupancy;
mod volume_emf;
mod volume_m64;

pub use occupancy::{parse_occ1, OccupancyGrid, MAT_EMPTY, MAT_EPICARDIAL_FAT, MAT_LUMEN, MAT_MYOCARDIUM};
pub use volume_emf::encode_occupancy_emf;
pub use volume_m64::encode_occupancy_m64;

use qualia_core_db::container_10d::skeletal_container::attach_skeletal_section_into;
use qualia_core_db::render::assets::Mesh;
use qualia_core_db::render::compile_10d::{
    compile_mesh_to_10d_with_extras_and_frames, Compile10dError, Compile10dExtras,
};

const ATTACH_SLACK: usize = 8192;

#[derive(Debug)]
pub enum SealError {
    EmptyMesh,
    Obj(String),
    Compile(Compile10dError),
    Skeletal(String),
    Volume(String),
    Emf(String),
    Attach(String),
}

impl std::fmt::Display for SealError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMesh => write!(f, "mesh has no triangles"),
            Self::Obj(msg) => write!(f, "obj: {msg}"),
            Self::Compile(e) => write!(f, ".10d compile: {e}"),
            Self::Skeletal(msg) => write!(f, "SKL1: {msg}"),
            Self::Volume(msg) => write!(f, "m64: {msg}"),
            Self::Emf(msg) => write!(f, "emf: {msg}"),
            Self::Attach(msg) => write!(f, "10d attach: {msg}"),
        }
    }
}

impl std::error::Error for SealError {}

#[derive(Clone, Debug, Default)]
pub struct TriangleMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[u8; 4]>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct SealBundle {
    pub ten_d: Vec<u8>,
    pub m64: Option<Vec<u8>>,
    pub emf: Option<Vec<u8>>,
}

pub fn parse_obj(src: &str) -> Result<TriangleMesh, SealError> {
    let mut mesh = TriangleMesh::default();
    for (line_no, raw) in src.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap_or("");
        match tag {
            "v" => {
                let nums: Vec<f32> = parts.filter_map(|p| p.parse().ok()).collect();
                if nums.len() < 3 {
                    return Err(SealError::Obj(format!("line {}: vertex needs xyz", line_no + 1)));
                }
                mesh.positions.push([nums[0], nums[1], nums[2]]);
                if nums.len() >= 6 {
                    mesh.colors.push([
                        (nums[3].clamp(0.0, 1.0) * 255.0) as u8,
                        (nums[4].clamp(0.0, 1.0) * 255.0) as u8,
                        (nums[5].clamp(0.0, 1.0) * 255.0) as u8,
                        255,
                    ]);
                }
            }
            "vn" => {
                let nums: Vec<f32> = parts.filter_map(|p| p.parse().ok()).collect();
                if nums.len() >= 3 {
                    mesh.normals.push([nums[0], nums[1], nums[2]]);
                }
            }
            "f" => {
                let corners: Vec<&str> = parts.collect();
                if corners.len() < 3 {
                    return Err(SealError::Obj(format!("line {}: face needs 3 vertices", line_no + 1)));
                }
                let mut idx = Vec::with_capacity(corners.len());
                for c in corners {
                    let v = c.split('/').next().unwrap_or("");
                    let n: i32 = v.parse().map_err(|_| {
                        SealError::Obj(format!("line {}: bad index {c}", line_no + 1))
                    })?;
                    let resolved = if n < 0 {
                        (mesh.positions.len() as i32 + n) as u32
                    } else {
                        (n - 1) as u32
                    };
                    idx.push(resolved);
                }
                for t in 1..idx.len() - 1 {
                    mesh.indices.push(idx[0]);
                    mesh.indices.push(idx[t]);
                    mesh.indices.push(idx[t + 1]);
                }
            }
            _ => {}
        }
    }
    if mesh.positions.is_empty() || mesh.indices.len() < 3 {
        return Err(SealError::EmptyMesh);
    }
    Ok(mesh)
}

pub fn seal_mesh_to_10d(mesh: &TriangleMesh) -> Result<Vec<u8>, SealError> {
    Ok(seal_mesh(mesh, None)?.ten_d)
}

pub fn seal_mesh(
    mesh: &TriangleMesh,
    occupancy: Option<&OccupancyGrid>,
) -> Result<SealBundle, SealError> {
    if mesh.positions.is_empty() || mesh.indices.len() < 3 {
        return Err(SealError::EmptyMesh);
    }
    let compiled = compile_geometry(mesh)?;
    let (min, max) = bounds(&mesh.positions);
    let skl1 = cardiac::encode_cardiac_skl1(&mesh.positions, min, max)?;
    let mut ten_d = attach_skl1(&compiled, &skl1)?;
    let mut m64 = None;
    let mut emf = None;
    if let Some(grid) = occupancy {
        ten_d = attach::attach_volume_fields(&ten_d, mesh, grid)?;
        m64 = Some(volume_m64::encode_occupancy_m64(grid)?);
        emf = Some(volume_emf::encode_occupancy_emf(grid)?);
    }
    Ok(SealBundle { ten_d, m64, emf })
}

fn compile_geometry(mesh: &TriangleMesh) -> Result<Vec<u8>, SealError> {
    let (min, max) = bounds(&mesh.positions);
    let mut triangles = Vec::with_capacity(mesh.indices.len() / 3);
    for tri in mesh.indices.chunks_exact(3) {
        triangles.push([tri[0], tri[1], tri[2]]);
    }
    if triangles.is_empty() {
        return Err(SealError::EmptyMesh);
    }
    let qmesh = Mesh {
        positions: mesh.positions.clone(),
        triangles,
        min,
        max,
    };
    let reading: Vec<[f32; 4]> = (0..mesh.positions.len())
        .map(|i| {
            let c = mesh.colors.get(i).copied().unwrap_or([180, 70, 70, 255]);
            [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
                c[3] as f32 / 255.0,
            ]
        })
        .collect();
    let normals = if mesh.normals.len() == mesh.positions.len() {
        Some(sanitize_normals(&mesh.normals))
    } else {
        None
    };
    match compile_mesh_to_10d_with_extras_and_frames(
        &qmesh,
        &[],
        None,
        Compile10dExtras::default(),
        Some(&reading),
        normals.as_deref(),
        None,
    ) {
        Ok(bytes) => Ok(bytes),
        Err(_) if normals.is_some() => compile_mesh_to_10d_with_extras_and_frames(
            &qmesh,
            &[],
            None,
            Compile10dExtras::default(),
            Some(&reading),
            None,
            None,
        )
        .map_err(SealError::Compile),
        Err(e) => Err(SealError::Compile(e)),
    }
}

fn attach_skl1(compiled: &[u8], payload: &[u8]) -> Result<Vec<u8>, SealError> {
    let mut out = vec![0u8; compiled.len().saturating_add(payload.len()).saturating_add(ATTACH_SLACK)];
    match attach_skeletal_section_into(compiled, payload, &mut out) {
        Ok(written) => {
            out.truncate(written);
            Ok(out)
        }
        Err(_) => {
            let mut larger = vec![
                0u8;
                compiled
                    .len()
                    .saturating_add(payload.len())
                    .saturating_mul(2)
                    .saturating_add(65536)
            ];
            let written = attach_skeletal_section_into(compiled, payload, &mut larger)
                .map_err(|e| SealError::Skeletal(format!("attach: {e:?}")))?;
            larger.truncate(written);
            Ok(larger)
        }
    }
}

fn sanitize_normals(normals: &[[f32; 3]]) -> Vec<[f32; 3]> {
    normals
        .iter()
        .map(|n| {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len.is_finite() && len > 1.0e-6 {
                [n[0] / len, n[1] / len, n[2] / len]
            } else {
                [0.0, 0.0, 1.0]
            }
        })
        .collect()
}

pub(crate) fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::occupancy::synthetic_ball;
    use qualia_core_db::container_10d::header::Container10dHeader;
    use qualia_core_db::container_10d::manifold_identity_v2::{
        decode_manifold_identity_v2, FIELD_KIND_ELECTROMAGNETIC, TypedFieldRecord,
    };
    use qualia_core_db::container_10d::section::{parse_section_table, SectionType};
    use qualia_core_db::container_10d::skeletal_container::read_skeletal_section;

    fn triangle() -> TriangleMesh {
        let obj = "v 0 0 0 0.8 0.2 0.2\nv 1 0 0 0.8 0.2 0.2\nv 0 1 0 0.8 0.2 0.2\nvn 0 0 1\nvn 0 0 1\nvn 0 0 1\nf 1//1 2//2 3//3\n";
        parse_obj(obj).unwrap()
    }

    fn has_section(packed: &[u8], typ: SectionType) -> bool {
        let header = Container10dHeader::parse(packed).unwrap();
        let sections = parse_section_table(packed, &header).unwrap();
        sections.iter().any(|s| s.typ() == Some(typ))
    }

    #[test]
    fn seals_obj_triangle_with_cardiac_skl1() {
        let mesh = triangle();
        let packed = seal_mesh_to_10d(&mesh).unwrap();
        assert_eq!(&packed[0..4], b"10d\0");
        assert_eq!(&packed[4..6], &2u16.to_le_bytes());
        let section = read_skeletal_section(&packed).expect("SKL1");
        let counts = section.counts();
        assert_eq!(counts.joints, 6);
        assert_eq!(counts.vertices, 3);
        assert_eq!(counts.clips, 1);
        assert_eq!(counts.tracks, 6);
        assert_eq!(counts.keys, 42);
        assert!(section.clip(0).unwrap().looping);
    }

    #[test]
    fn occupancy_seals_m64_emf_and_10d_field_kind() {
        let mesh = triangle();
        let grid = synthetic_ball(16, 0.85);
        let bundle = seal_mesh(&mesh, Some(&grid)).unwrap();
        assert_eq!(&bundle.ten_d[0..4], b"10d\0");
        assert!(has_section(&bundle.ten_d, SectionType::SkeletalAnimation));
        assert!(has_section(&bundle.ten_d, SectionType::VoxelLattice));
        assert!(has_section(&bundle.ten_d, SectionType::CorrespondenceMap));
        assert!(has_section(&bundle.ten_d, SectionType::Reconstruction));
        assert!(has_section(&bundle.ten_d, SectionType::HyperspectralCube));
        assert!(has_section(&bundle.ten_d, SectionType::ManifoldIdentityV2));

        let header = Container10dHeader::parse(&bundle.ten_d).unwrap();
        let sections = parse_section_table(&bundle.ten_d, &header).unwrap();
        let mid = sections
            .iter()
            .find(|s| s.typ() == Some(SectionType::ManifoldIdentityV2))
            .unwrap();
        let payload = &bundle.ten_d[mid.byte_offset as usize
            ..(mid.byte_offset + mid.byte_length) as usize];
        let mut fields = [TypedFieldRecord::default(); 4];
        let (identity, count) = decode_manifold_identity_v2(payload, &[], &mut fields).unwrap();
        assert_eq!(count, 1);
        assert_eq!(fields[0].kind, FIELD_KIND_ELECTROMAGNETIC);
        assert_eq!(identity.coordinates[9], 0.12);

        let m64 = bundle.m64.expect("m64");
        assert_eq!(&m64[0..4], b"m64\0");
        let emf = bundle.emf.expect("emf");
        assert_eq!(&emf[0..8], b"Q42EMF64");
    }
}
