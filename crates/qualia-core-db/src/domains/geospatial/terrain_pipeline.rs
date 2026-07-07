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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_tile_has_enu_origin() {
        let heights = vec![0.0f32; 9];
        let tile = compile_terrain_tile(
            &heights,
            3,
            3,
            10.0,
            -33.8688,
            151.2093,
            0.0,
            -33.8688,
            151.2093,
            0.0,
        );
        assert_eq!(tile.mesh.vertices.len(), 9);
        assert!((tile.enu_origin.0).abs() < 1.0);
        assert!((tile.enu_origin.1).abs() < 1.0);
    }
}