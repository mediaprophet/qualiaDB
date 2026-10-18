//! QualiaDB Geospatial Engine (`qualia-geospatial`)
//!
//! Provides coordinate semantics, geodetic frames, hydro-enforced raster DEM compilation,
//! Hilbert space-filling curve spatial indexing, OGC WFS/WMS vector and raster streaming,
//! road/stream corridor generation, linguistic isogloss boundaries, and spatial asset manifests.

pub mod corridor;
pub mod frame;
pub mod geotiff;
pub mod hilbert;
pub mod isogloss;
pub mod manifest;
pub mod wfs;
pub mod wms_streamer;

pub use corridor::{CorridorKind, HermiteCorridorSegment, SplineControlPoint};
pub use frame::{
    GeodeticCoord, GeodeticFrame, SRID_GDA2020, SRID_MGA_ZONE_55, SRID_MGA_ZONE_56,
    SRID_WEB_MERCATOR, SRID_WGS84, VERTICAL_DATUM_AHD, VERTICAL_DATUM_ELLIPSOIDAL,
};
pub use geotiff::{
    compile_raster_to_m64, compile_raster_to_m64_with_codec, decode_terrain_rgb_mm,
    decode_terrarium_mm, encode_terrain_rgb, CompressionCodec, ElevationRaster, IngestionError,
    CHUNK_CELL_COUNT, CHUNK_GRID_SIZE,
};
pub use hilbert::{hilbert_to_xy, xy_to_hilbert};
pub use isogloss::{DiachronicIsogloss, SonicToponymStreamer, SpatioTemporalLect, TimeFrequencyBox};
pub use manifest::SpatialAssetManifest;
pub use wfs::{GeologicUnitRecord, GeologicVectorLayer};
pub use wms_streamer::{
    TextureArraySlot, TileCoordinate, WmsTileStreamer, MAX_CACHE_LAYERS, TILE_BYTES,
    TILE_DIMENSION_PX,
};

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::q42::m64_matter::{
        M64ChunkDirectoryEntry, M64Header, M64StrataColumnRecord, M64_MAGIC,
    };

    #[test]
    fn test_geodetic_frame_projection_roundtrip() {
        let frame = GeodeticFrame::default();
        let target = GeodeticCoord {
            latitude_deg: -35.795,
            longitude_deg: 148.315,
            elevation_ahd_mm: 520_000,
        };

        let (x_mm, y_mm, z_mm) = frame.project_to_local_mm(&target);
        assert_eq!(z_mm, 520_000 - 350_000);

        let (unproj_lat, unproj_lon) = frame.unproject_from_local_mm(x_mm, y_mm);
        assert!((unproj_lat - target.latitude_deg).abs() < 1e-6);
        assert!((unproj_lon - target.longitude_deg).abs() < 1e-6);

        let dggs_hash = frame.to_dggs_cell_hash(&target, 12);
        assert_ne!(dggs_hash, 0);
    }

    #[test]
    fn test_hilbert_curve_roundtrip() {
        for x in [0u32, 1, 15, 63, 127] {
            for y in [0u32, 2, 31, 63, 127] {
                let d = xy_to_hilbert(x, y, 8);
                let (rx, ry) = hilbert_to_xy(d, 8);
                assert_eq!(x, rx);
                assert_eq!(y, ry);
            }
        }
    }

    #[test]
    fn test_terrain_rgb_and_terrarium_elevation_roundtrip() {
        for elev_m in [-400, 0, 350, 1250, 2228] {
            let elev_mm = elev_m * 1000;
            let rgb = encode_terrain_rgb(elev_mm);
            let decoded_mm = decode_terrain_rgb_mm(rgb[0], rgb[1], rgb[2]);
            assert!((decoded_mm - elev_mm).abs() <= 100);
        }

        let terrarium_mm = decode_terrarium_mm(128, 0, 0);
        assert_eq!(terrarium_mm, 0);
    }

    #[test]
    fn test_raster_to_m64_compilation() {
        let frame = GeodeticFrame::default();
        let width = 64;
        let height = 64;
        let elevations = vec![500_000; width * height];
        let raster = ElevationRaster {
            width,
            height,
            cell_size_mm: 30_000,
            elevations_mm: &elevations,
        };

        let mut buffer = vec![0u8; 1_000_000];
        let bytes_written = compile_raster_to_m64(&raster, &frame, 1, &mut buffer)
            .expect("compilation succeeds");

        assert!(bytes_written > 0);

        let header: &M64Header = bytemuck::from_bytes(&buffer[..std::mem::size_of::<M64Header>()]);
        assert_eq!(header.magic, M64_MAGIC);
        assert_eq!(header.chunk_count, 1);
        assert!(header.validate().is_ok());

        let dir_offset = std::mem::size_of::<M64Header>();
        let dir_entry: &M64ChunkDirectoryEntry = bytemuck::from_bytes(
            &buffer[dir_offset..dir_offset + std::mem::size_of::<M64ChunkDirectoryEntry>()],
        );
        assert_eq!(dir_entry.srid_geodetic_code, SRID_GDA2020);
        assert_eq!(dir_entry.record_count, 4096);
        assert!(dir_entry.validate().is_ok());

        let payload_offset = dir_entry.byte_offset as usize;
        let strata: &M64StrataColumnRecord = bytemuck::from_bytes(
            &buffer[payload_offset..payload_offset + std::mem::size_of::<M64StrataColumnRecord>()],
        );
        assert_eq!(strata.surface_elevation_mm, 500_000);
        assert!(strata.validate().is_ok());
    }

    #[test]
    fn test_geologic_wfs_unit_code_resolution_and_bank1_recipes() {
        let layer = GeologicVectorLayer::default();

        assert_eq!(layer.resolve_unit_to_lut_index("Jmb"), Some(128));
        assert_eq!(layer.resolve_unit_to_lut_index("Gda"), Some(129));
        assert_eq!(layer.resolve_unit_to_lut_index("Unknown"), None);

        let recipes = layer.compile_bank_1_recipes();
        assert_eq!(recipes[0].bulk_density_kg_m3, 2850);
        assert_eq!(recipes[1].bulk_density_kg_m3, 2680);
        assert!(recipes[0].validate().is_ok());
        assert!(recipes[1].validate().is_ok());
    }

    #[test]
    fn test_wms_tile_streamer_lru_and_addressing() {
        let mut streamer = WmsTileStreamer::default();
        let coord1 = TileCoordinate { zoom: 12, x: 3820, y: 2450 };
        let coord2 = TileCoordinate { zoom: 12, x: 3821, y: 2450 };

        let slot1 = streamer.allocate_slot(&coord1);
        let slot2 = streamer.allocate_slot(&coord2);
        assert_ne!(slot1, slot2);

        assert_eq!(streamer.lookup_tile(&coord1), Some(slot1));
        assert_eq!(streamer.lookup_tile(&coord2), Some(slot2));
    }

    #[test]
    fn test_hermite_corridor_conformer() {
        let p0 = SplineControlPoint {
            pos_xy_mm: [0.0, 0.0],
            elevation_mm: 500_000,
            tangent_xy: [1000.0, 0.0],
        };
        let p1 = SplineControlPoint {
            pos_xy_mm: [1000.0, 0.0],
            elevation_mm: 510_000,
            tangent_xy: [1000.0, 0.0],
        };

        let road = HermiteCorridorSegment {
            p0,
            p1,
            kind: CorridorKind::Road { width_mm: 8000, crown_height_mm: 150 },
        };
        let mut road_col = M64StrataColumnRecord::default();
        road_col.surface_elevation_mm = 505_000;
        assert!(road.conform_strata_column(500.0, 0.0, &mut road_col));
        assert_eq!(road_col.layer_compaction_pct[0], 240);
        assert!(road_col.validate().is_ok());
    }

    #[test]
    fn test_spatial_asset_manifest_ntriples() {
        let manifest = SpatialAssetManifest::new_snowy_ga_dem(0x0123_4567_89AB_CDEF);
        let ntriples = manifest.to_ntriples();
        assert!(ntriples.contains("eco:SpatialAsset/AU-GA-DEM-Talbingo"));
        assert!(ntriples.contains("https://creativecommons.org/licenses/by/4.0/"));
        assert!(ntriples.contains("7844"));
        assert!(ntriples.contains("Htj2kWavelet"));
        assert!(ntriples.contains("POLYGON(("));
        assert!(ntriples.contains("R84400123"));
    }

    #[test]
    fn test_sonic_toponym_streamer_and_doppler() {
        let frame = GeodeticFrame::default();
        let sacred_site = GeodeticCoord {
            latitude_deg: -35.78,
            longitude_deg: 148.29,
            elevation_ahd_mm: 350_000,
        };

        let tf_box = TimeFrequencyBox {
            start_nanos: 0,
            end_nanos: 4_000_000_000, // 4-second oral chant
            min_freq_hz: 110.0,
            max_freq_hz: 1200.0,
            harmonic_mask: 0x1F,
            _padding: 0,
        };

        let streamer = SonicToponymStreamer::new(
            sacred_site,
            tf_box,
            0xCAFE_BABE,
            85.0, // 85 dB SPL at 1m
        );

        // Listener 100m away
        let listener_100m = GeodeticCoord {
            latitude_deg: -35.7809,
            longitude_deg: 148.29,
            elevation_ahd_mm: 350_000,
        };

        let spl_100m = streamer.evaluate_listener_spl_db(&listener_100m, &frame);
        // Inverse-square law: 20 * log10(100) = 40 dB attenuation -> ~45 dB
        assert!(spl_100m > 40.0 && spl_100m < 50.0);

        // Doppler shift: listener approaching at 34.3 m/s (~123 km/h, 0.1 Mach)
        let approach_factor = streamer.evaluate_doppler_factor(
            [0.0, 34.3, 0.0], // Heading north towards source
            &listener_100m,
            &frame,
        );
        // Pitch rises by ~10% (factor ~1.10)
        assert!((approach_factor - 1.10).abs() < 0.02);
    }
}
