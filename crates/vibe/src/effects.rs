//! Effect classes.

use crate::ast::EffectClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effect {
    Pure = 0,
    Hot = 1,
    Cold = 2,
    Async = 3,
    External = 4,
}

impl Effect {
    /// Constructs from class.
    pub fn from_class(c: EffectClass) -> Effect {
        match c {
            EffectClass::Pure => Effect::Pure,
            EffectClass::Hot => Effect::Hot,
            EffectClass::Cold => Effect::Cold,
            EffectClass::Async => Effect::Async,
            EffectClass::External => Effect::External,
        }
    }

    /// Joins effect sets.
    pub fn join(self, other: Effect) -> Effect {
        self.max(other)
    }
}

/// Binding id → minimum effect required to call it.
pub fn binding_effect(path: &str) -> Effect {
    match path {
        // Runtime registry writes/reads and deferred execution cannot be Pure.
        "task.spawn"
        | "task.cancel"
        | "task.with_timeout"
        | "code.spawn_at"
        | "code.hash_ast"
        | "code.resolve"
        | "value.attach_provenance"
        | "value.verify_provenance"
        | "crypto.prove_zk_predicate"
        | "crypto.verify_zk_predicate" => Effect::Cold,
        // Stateless deterministic binding integrity; no registry or attestation.
        "crypto.commit_predicate" | "crypto.verify_commitment" => Effect::Pure,
        "task.status" | "Geodesy.rtree_query_bbox" => Effect::Hot,
        "Terrain.bake_dag"
        | "Terrain.erode_fixed824"
        | "Terrain.erode_coupled_6species"
        | "Terrain.erode_droplet"
        | "Terrain.carve_meander_channel"
        | "Terrain.hydro_priority_flood"
        | "Terrain.hydro_flow_dinf"
        | "Terrain.hydro_fastscape_spl"
        | "Terrain.coastal_wave_cliff"
        | "Terrain.thermal_talus_creep"
        | "Terrain.landslide_debris_runout"
        | "Terrain.glacier_sia_carve"
        | "Terrain.snow_wind_avalanche"
        | "Terrain.aeolian_sand_saltation"
        | "Terrain.strata_pack_m64"
        | "Terrain.geomorph_qa_audit" => Effect::Cold,
        "GisIo.read_geojson" | "GisIo.write_geojson" | "GisIo.plan_cog_range_read" => Effect::Pure,
        "GisIo.write_geopackage"
        | "GisIo.write_flatgeobuf"
        | "GisIo.write_shapefile"
        | "GisIo.write_geoparquet"
        | "GisIo.query_ogc_features_cql2"
        | "GisIo.query_wfs_arcgis_csw"
        | "GisIo.query_stac_catalog"
        | "GisIo.query_sensorthings_sos"
        | "GisIo.query_wcs_wms_wmts" => Effect::External,
        "GisGp.simplify_dp"
        | "GisGp.centroid_polylabel"
        | "GisGp.hausdorff_frechet"
        | "GisGp.tpi_tri_vrm_geomorphons"
        | "GisGp.map_algebra_cell_compile"
        | "WgslForge.geo_compile_cell_map_algebra"
        | "PoetGeo.geodesic_ruler_protractor"
        | "PoetGeo.orbital_camera_flythrough"
        | "Cartography.bivariate_choropleth_3x3"
        | "Cartography.simulate_cvd_palette"
        | "Cartography.audit_wcag_contrast"
        | "WebizenGeo.eval_mvt_style_zoom"
        | "WebizenGeo.eval_sky_atmosphere_limb"
        | "WebizenGeo.eval_water_wave_foam" => Effect::Pure,
        p if p.starts_with("GisIo.")
            || p.starts_with("GisGp.")
            || p.starts_with("WgslForge.geo_")
            || p.starts_with("PoetGeo.")
            || p.starts_with("Cartography.")
            || p.starts_with("WebizenGeo.") => Effect::Cold,
        p if p.starts_with("Geodesy.") || p.starts_with("Terrain.") || p.starts_with("GeoSparql.") => Effect::Pure,
        // Child effects are not yet represented in Task<T, effects>. Joining may
        // run an admitted effectful program, so dynamic joins are conservative.
        "task.await" | "task.nursery" => Effect::External,
        p if p.starts_with("math.") || p.starts_with("rdf.") => Effect::Pure,
        p if p.starts_with("web.")
            || p.starts_with("ui.")
            || p.starts_with("form.")
            || p.starts_with("input.")
            || p.starts_with("validate.")
            || p.starts_with("selection.")
            || p.starts_with("layout.")
            || p.starts_with("cbor_ld.") =>
        {
            Effect::Pure
        }
        "vec2" | "vec3" | "vec4" | "mat3" | "mat4" | "oklch" | "Ok" | "Err" => Effect::Pure,
        "quin.statement" | "receipt_empty" | "capability.resolve" => Effect::Pure,
        "graph.snapshot" | "graph.query" => Effect::Pure,
        "aura.validate" => Effect::Pure,
        "graph.stage" | "graph.commit" | "pulse.publish" | "capability.invoke" | "time.unix"
        | "time.now" => Effect::External,
        p if p.starts_with("Animation.") => Effect::Pure,
        p if crate::catalog::is_known(p) || crate::catalog::looks_like_catalog_path(p) => {
            Effect::External
        }
        _ => Effect::Pure,
    }
}

/// Capability required for the effect.
pub fn capability_for(path: &str) -> Option<&'static str> {
    match path {
        "graph.query" | "graph.snapshot" => Some("graph.read"),
        "graph.stage" | "graph.commit" => Some("graph.write"),
        "aura.validate" => Some("aura.validate"),
        "pulse.publish" => Some("pulse.publish"),
        "capability.invoke" => Some("capability.invoke"),
        p => crate::catalog::canonical_id(p),
    }
}
