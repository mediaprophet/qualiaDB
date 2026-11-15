// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) EarthCraft contributors & Nerthus contributors (adapted under MIT License)
//! Round 2 (`GEO-S2`) & Round 3 (`GEO-S3`) Procedural Terrain, 4D Geomorphology,
//! `.m64` `0x02` Stratigraphic Column, and 323-Province Sentinel-2 Spectral Engine.

pub mod erosion_fixed824;
pub mod geomorph_qa_report;
pub mod hydro_stream_power;
pub mod m64_strata_constituents;
pub mod noise_simd_lut;
pub mod terrain_adjust_filter;
pub mod terrain_dag_workspace;
pub mod terrain_primitives;
pub mod terrain_selectors_combine;
pub mod thermal_snow_glacier;

pub use erosion_fixed824::{
    carve_meander_channel, f32_to_fixed824, fixed824_to_f32, step_coupled_6species_erosion,
    step_hydraulic_pipe_fixed824, trace_erosion_droplet, CoupledErosionParams, StrataCell6,
    FIXED_8_24_SCALE,
};
pub use geomorph_qa_report::{
    audit_geomorph_realism, compute_hacks_law_and_concavity, compute_hypsometric_curve,
    compute_strahler_order_and_bifurcation, GeomorphQaReport,
};
pub use hydro_stream_power::{
    apply_coastal_wave_cliff_terrace, breach_depressions_least_cost, compute_d8_flow,
    compute_dinf_flow_accumulation, priority_flood_fill_depressions, step_fastscape_stream_power,
};
pub use m64_strata_constituents::{
    crc32c, derive_surface_constituents, evaluate_sentinel2_biome_albedo, BiomeSpectralStop,
    M64StrataCell, SurfaceConstituents8, M64_MAGIC, M64_STRATA_SUBMAGIC,
    SENTINEL2_BIOME_PROVINCE_COUNT,
};
pub use noise_simd_lut::{
    eval_fractal_grid_into, evaluate_fractal_2d, gabor_anisotropic_2d, opensimplex2s_2d,
    perlin_2d, perlin_2d_deriv, perlin_3d, perm_hash, value_noise_2d, voronoi_2d,
    wavelet_bandlimited_2d, FractalConfig, FractalMode, VoronoiSample, GRAD3, RANPERM,
};
pub use terrain_adjust_filter::{
    apply_autolevel, apply_clamp_normalize, apply_curve_lut, apply_fold,
    apply_histogram_equalize, apply_shear_grid, apply_stratify, apply_terrace,
    filter_aperture_3x3, filter_chamfer_distance_transform, filter_curvature_map,
    filter_dilate_3x3, filter_erode_morph_3x3, filter_gaussian_5x5, filter_median_3x3,
};
pub use terrain_dag_workspace::{
    DagNodeKernel, DagNodeSpec, TerrainDagError, TerrainDagWorkspace, MAX_DAG_NODES,
};
pub use terrain_primitives::{
    generate_landform_grid_into, sample_landform, LandformKind, LandformParams,
};
pub use terrain_selectors_combine::{
    apply_post_process_chain, combine_layers, domain_warp_grid, select_ambient_occlusion,
    select_convexity_protrusion, select_flow_accumulation_proxy, select_height,
    select_rock_exposure_and_treeline, select_slope_aspect, selector_band_weight, CombineMode,
    PostProcessOp,
};
pub use thermal_snow_glacier::{
    step_aeolian_sand_saltation, step_glacier_sia_carving, step_landslide_debris_runout,
    step_snow_wind_and_avalanche, step_thermal_talus_creep,
};
