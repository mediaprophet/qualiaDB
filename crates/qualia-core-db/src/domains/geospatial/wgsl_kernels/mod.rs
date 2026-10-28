// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 QualiaDB Contributors (adapted from EarthCraft, Nerthus, and RustGIS under MIT/Apache-2.0)
//! Round 8 (`GEO-S8`) `wgsl_forge`-Certified GPU Compute Kernels, VibeScript Cell -> WGSL
//! Map Algebra Compiler, and Zero-Heap CPU Parity Oracles.

pub mod geo_erosion_hydro_kernels;
pub mod geo_noise_filter_kernels;
pub mod geo_viewshed_algebra_kernels;

pub use geo_erosion_hydro_kernels::{
    certify_wgsl_erosion_kernel, cpu_oracle_824_pipe_step_into,
    emit_wgsl_824_pipe_hydraulic_erosion, emit_wgsl_coupled_6layer_strata_erosion,
    emit_wgsl_d8_dinfinity_flow_routing, ErosionGpuUniforms, GeoWgslError,
    GeoWgslKernelDescriptor,
};
pub use geo_noise_filter_kernels::{
    cpu_oracle_noise_terrace_biome_into, emit_wgsl_landform_primitives_and_filters,
    emit_wgsl_multifractal_noise_cascade, emit_wgsl_sentinel2_biome_splatting,
    validate_wgsl_noise_shader, GeoNoiseWgslError, NoiseFilterGpuUniforms,
};
pub use geo_viewshed_algebra_kernels::{
    compile_vibe_cell_to_wgsl_map_algebra, cpu_oracle_compiled_map_algebra_into,
    emit_wgsl_horn_twi_spi_geomorphons, emit_wgsl_radial_viewshed_and_hillshade,
    GeoAlgebraWgslError, ViewshedAlgebraGpuUniforms, WgslMapAlgebraToken,
};
