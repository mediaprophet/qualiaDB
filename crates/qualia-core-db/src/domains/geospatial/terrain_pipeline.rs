//! DEM heightfield → terrain mesh → geodetically anchored scene coordinates (P1).

use super::dem::{generate_terrain_mesh, TerrainMesh};
use super::geodetic::{ecef_to_enu, lat_lon_alt_to_ecef};

/// A terrain tile positioned at a geodetic anchor.
#[derive(Debug, Clone)]
pub struct GeodeticTerrainTile {
    pub mesh: TerrainMesh,
    /// Scene-space ENU origin for this tile (metres).
    pub enu_origin: (f64, f64, f64),
    /// Geodetic centre of the tile.
    pub centre_lat: f64,
    pub centre_lon: f64,
    pub centre_alt_m: f64,
}

/// Build a terrain mesh from a heightfield and anchor it relative to a world origin.
pub fn compile_terrain_tile(
    heightfield: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    centre_lat: f64,
    centre_lon: f64,
    centre_alt_m: f64,
    world_origin_lat: f64,
    world_origin_lon: f64,
    world_origin_alt_m: f64,
) -> GeodeticTerrainTile {
    let mesh = generate_terrain_mesh(heightfield, width, height, cell_size_m);

    let (cx, cy, cz) = lat_lon_alt_to_ecef(centre_lat, centre_lon, centre_alt_m);
    let enu = ecef_to_enu(
        cx,
        cy,
        cz,
        world_origin_lat,
        world_origin_lon,
        world_origin_alt_m,
    );

    GeodeticTerrainTile {
        mesh,
        enu_origin: enu,
        centre_lat,
        centre_lon,
        centre_alt_m,
    }
}

use crate::container_10d::header::Container10dHeader;
use crate::container_10d::integrity::seal_whole_file_crc32c;
use crate::container_10d::mesh_section::{encode_mesh_section, encoded_len};
use crate::container_10d::provenance_section::{encode_provenance_section, ProvenanceSidecar};
use crate::container_10d::section::{encode_container, AlignmentTier, SectionInput, SectionType};
use crate::render::assets::Mesh;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Compiles a DEM heightfield into a native `.10d` QuantizedMesh tensor file,
/// wrapping it with CBOR-LD provenance for full compliance.
pub fn compile_and_encode_10d_tile(
    heightfield: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    source_bytes: Vec<u8>,
    licence: String,
    cbor_ld_metadata: Vec<u8>,
) -> Result<Vec<u8>, String> {
    compile_and_encode_10d_tile_with_media_type(
        heightfield,
        width,
        height,
        cell_size_m,
        source_bytes,
        "application/octet-stream".into(),
        licence,
        cbor_ld_metadata,
    )
}

/// Compile and encode a georeferenced heightfield with an explicit source media type.
pub fn compile_and_encode_10d_tile_with_media_type(
    heightfield: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
    source_bytes: Vec<u8>,
    source_media_type: String,
    licence: String,
    cbor_ld_metadata: Vec<u8>,
) -> Result<Vec<u8>, String> {
    validate_heightfield(heightfield, width, height, cell_size_m)?;

    // 1. Generate standard TerrainMesh
    let terrain = generate_terrain_mesh(heightfield, width, height, cell_size_m);

    // 2. Convert to render::assets::Mesh
    let mut triangles = Vec::with_capacity(terrain.indices.len() / 3);
    for chunk in terrain.indices.chunks_exact(3) {
        triangles.push([chunk[0], chunk[1], chunk[2]]);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for p in &terrain.vertices {
        for i in 0..3 {
            if p[i] < min[i] {
                min[i] = p[i];
            }
            if p[i] > max[i] {
                max[i] = p[i];
            }
        }
    }
    if terrain.vertices.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
    }

    let render_mesh = Mesh {
        positions: terrain.vertices,
        triangles,
        min,
        max,
    };

    // 3. Encode QuantizedMesh Section
    let mesh_need = encoded_len(render_mesh.positions.len(), render_mesh.triangles.len());
    let mut mesh_payload = vec![0u8; mesh_need];
    let mesh_size =
        encode_mesh_section(&render_mesh, &mut mesh_payload).map_err(|e| e.to_string())?;
    mesh_payload.truncate(mesh_size);

    // 4. Build Provenance Sidecar
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock predates Unix epoch: {error}"))?
        .as_secs();
    let version_hash: [u8; 32] = Sha256::digest(&source_bytes).into();
    let mut sidecar = ProvenanceSidecar::new(source_bytes, source_media_type, licence);
    sidecar.semantic_metadata = cbor_ld_metadata;
    sidecar.timestamp_epoch_s = now;
    sidecar.version_hash = version_hash;

    let sidecar_need = sidecar.source_bytes.len()
        + sidecar.source_media_type.len()
        + sidecar.licence.len()
        + sidecar.vc.len()
        + sidecar.semantic_metadata.len()
        + 1024;
    let mut sidecar_payload = vec![0u8; sidecar_need];
    let sidecar_size =
        encode_provenance_section(&sidecar, &mut sidecar_payload).map_err(|e| e.to_string())?;
    sidecar_payload.truncate(sidecar_size);

    // 5. Encode 10D Container
    let h = Container10dHeader::proposed();
    let inputs = [
        SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &mesh_payload,
        },
        SectionInput {
            section_type: SectionType::ProvenanceSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &sidecar_payload,
        },
    ];

    // Allocate 10D container size plus header padding
    let mut out = vec![0u8; mesh_size + sidecar_size + 1024];
    let n = encode_container(&h, &inputs, &mut out).map_err(|e| e.to_string())?;
    out.truncate(n);

    seal_whole_file_crc32c(&mut out);

    Ok(out)
}

fn validate_heightfield(
    heightfield: &[f32],
    width: usize,
    height: usize,
    cell_size_m: f64,
) -> Result<(), String> {
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| "heightfield dimensions overflow".to_string())?;
    if width < 2 || height < 2 || heightfield.len() != expected {
        return Err(format!(
            "heightfield needs at least 2x2 samples and exactly width*height values (got {} values for {width}x{height})",
            heightfield.len()
        ));
    }
    if !cell_size_m.is_finite() || cell_size_m <= 0.0 {
        return Err("terrain cell size must be finite and greater than zero".into());
    }
    if heightfield.iter().any(|elevation| !elevation.is_finite()) {
        return Err("heightfield contains a non-finite elevation sample".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_tile_has_enu_origin() {
        let heights = vec![0.0f32; 9];
        let tile = compile_terrain_tile(
            &heights, 3, 3, 10.0, -33.8688, 151.2093, 0.0, -33.8688, 151.2093, 0.0,
        );
        assert_eq!(tile.mesh.vertices.len(), 9);
        assert!((tile.enu_origin.0).abs() < 1.0);
        assert!((tile.enu_origin.1).abs() < 1.0);
    }
}
