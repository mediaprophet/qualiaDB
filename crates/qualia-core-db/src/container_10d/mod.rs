//! `.10d` versioned container codecs and shared native/WASM SDK boundaries.
//!
//! Source includes v1/v2 headers, aligned section tables and CRC integrity;
//! quantized meshes, tensor nodes, provenance, materials, UV/frame streams,
//! typed MID2 payload references, morph/skeletal data and other specialized
//! sections. Presence of an API is not qualification of every combination,
//! consumer, target or scientific interpretation.
//!
//! Current numeric and typed-payload contracts are maintained in
//! `docs/manuals/standards/10d-container-format.md` and
//! `docs/manuals/standards/10d-typed-field-payloads.md`. Those companions identify
//! the exact native conformance cases and the limits of their evidence.
//!
//! Axis roles remain q/v/w selectors, x/y/z/t/alpha/sigma coordinates, and mu
//! a coordinate carrier. Metric completeness must reflect actual runtime math;
//! no source-only completeness or native/WASM parity claim follows here.
//! Legacy API/file names containing `identity` remain compatibility spellings;
//! identifiers, qualified referents and integrity digests retain distinct roles.

pub mod axis_role;
pub mod conformance;
pub mod crc32c;
pub mod field_section;
pub mod header;
pub mod integrity;
/// Bounded integration of the v2 identity manifest with the versioned section envelope.
pub mod manifold_identity_container;
pub mod material_container;
pub mod material_section;
mod material_section_codec;
pub use manifold_identity_container::{
    decode_container_manifold_identity, encode_container_with_manifold_identity,
    encode_container_with_manifold_identity_extensions, ManifoldContainerError,
};
/// Version-2 manifold identity and typed scientific/presentation field manifest.
pub mod manifold_identity_v2;
pub mod mesh_section;
pub mod metric_check;
pub mod metric_trait_gate;
pub mod morph_section;
pub mod node_section;
pub mod normal_section;
pub mod provenance_section;
pub mod section;
pub mod skeletal_container;
/// Versioned rig, skin influence and motor clip records.
pub mod skeletal_section;
pub mod surface_reading;
pub mod texture_coordinate_container;
pub mod texture_coordinate_section;
/// Borrowed typed-field selection and existing-codec container readers.
pub mod typed_payloads;
// `topology_section` and `spatial_index_section` depend on
// `crate::specialized_libs::computational_geometry`, which is itself gated
// `#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]` at
// `lib.rs`. The slim `--features portal` WASM build (no `wasm-scientific`)
// configures `specialized_libs` out, so these two section modules must carry
// the same gate or the crate fails to compile for wasm32. Their only external
// consumers (`tool.rs`, `query_frontend.rs`) live inside `specialized_libs`
// and carry the same gate, so this is symmetric — no portal-path regression
// (the portal renderer upload path uses `mesh_section`, which has no
// `specialized_libs` dependency).
pub mod entity_ontology_section;
pub mod impostor_section;
pub mod procedural_synthesis;
pub mod spatial_index_section;
pub mod surfel_section;
pub mod topology_section;
pub mod voxel_section;

pub use voxel_section::*;

pub use axis_role::{
    AxisRole, AXIS_ORDER, COORDINATE_AXES, MU_AXIS, PROPOSED_AXIS_ROLES, SELECTOR_AXES,
};
pub use crc32c::{crc32c, crc32c_update};
pub use entity_ontology_section::{
    decode_entity_ontology_section_into, decode_personal_ontology_section_into,
    encode_entity_ontology_section, encode_personal_ontology_section, entity_ontology_encoded_len,
    personal_ontology_encoded_len, AffordanceBinaryRecord, EntityOntologyMiniHeader,
    EntityOntologySectionError, PersonalOntologyMiniHeader, PersonalOntologySectionError,
    AFFORDANCE_RECORD_SIZE, ENTITY_ONTOLOGY_MINI_HEADER_SIZE, ENTITY_ONTOLOGY_SECTION_VERSION,
    MAGIC_EONT, MAGIC_PONT, MAX_AFFORDANCES_PER_SECTION, PERSONAL_ONTOLOGY_MINI_HEADER_SIZE,
    PERSONAL_ONTOLOGY_SECTION_VERSION,
};
pub use header::{
    Container10dHeader, HeaderParseError, FLAG_DEFAULT_DISPOSITION_REFUSE, HEADER_BYTE_SIZE,
    HEADER_VERSION, LEGACY_HEADER_VERSION, MAGIC_10D, MAX_SECTION_COUNT, SUPPORTED_HEADER_VERSIONS,
};
pub use impostor_section::{
    decode_impostor_section, encode_impostor_section, hemi_octahedral_decode,
    hemi_octahedral_encode, sample_impostor_blend_weights, ImpostorAtlasMiniHeader,
    ImpostorBlendSample, ImpostorSectionError, FLAG_DEPTH_IN_ALPHA, FLAG_HEMI_OCTAHEDRAL,
    FLAG_OCT32_NORMALS, FLAG_PBR_METALLIC_ROUGHNESS, IMPOSTOR_MINI_HEADER_SIZE,
    IMPOSTOR_SECTION_VERSION, MAGIC_IMPO,
};
pub use integrity::{
    compute_whole_file_crc32c, seal_whole_file_crc32c, verify_whole_file_crc32c, IntegrityError,
};
pub use manifold_identity_v2::{
    decode_manifold_identity_v2, encode_manifold_identity_v2,
    encode_manifold_identity_v2_with_extensions, encoded_len as manifold_identity_v2_encoded_len,
    IntegrityIndexDigest, ManifestV2Error, ManifoldIdentityV2, StableEntityId, TypedFieldRecord,
    BUILTIN_FIELD_KINDS, FIELD_KIND_ELECTROMAGNETIC, FIELD_KIND_SCALAR, FIELD_KIND_SPECTRAL,
    FIELD_KIND_TENSOR, FIELD_KIND_VECTOR, FIELD_KIND_SURFACE_FRAMES, FIELD_KIND_MORPH_TARGETS,
    MANIFOLD_IDENTITY_V2_AXIS_ORDER,
    MANIFOLD_IDENTITY_V2_FIELD_RECORD_SIZE, MANIFOLD_IDENTITY_V2_HEADER_SIZE,
    MANIFOLD_IDENTITY_V2_MAGIC, MANIFOLD_IDENTITY_V2_MAX_BYTES, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    MANIFOLD_IDENTITY_V2_VERSION,
};
pub use material_container::{
    attach_material_section, material_container_counts, read_material_section_into,
    MaterialContainerError,
};
pub use material_section::{
    decode_material_section_into, encode_material_section,
    encoded_len as material_section_encoded_len, material_section_counts,
    validate_material_bindings, MaterialRecord, MaterialSectionError, OpacityMode, ShadingModel,
    SubmeshRange, TextureAddressMode, TextureFilterMode, TextureMinFilter, TextureSampler,
    TextureTransform, MATERIAL_RECORD_SIZE, MATERIAL_SECTION_HEADER_SIZE, MATERIAL_SECTION_VERSION,
    MATERIAL_V1_RECORD_SIZE, MAX_MATERIALS, MAX_SUBMESH_RANGES, SUBMESH_RANGE_SIZE,
    TEXTURE_SAMPLER_RECORD_SIZE,
};
pub use mesh_section::{
    decode_mesh_section, encode_mesh_section, parse_mesh_header, MeshMiniHeader, MeshSectionError,
    FLAG_U16_INDICES, MAX_TRIANGLE_COUNT, MAX_VERTEX_COUNT, MESH_MINI_HEADER_SIZE,
};
pub use morph_section::{
    apply_morph_targets_zero_heap, decode_morph_section_into, dequantize_morph_delta,
    encode_morph_section, morph_section_encoded_len, quantize_morph_delta, MorphSectionError,
    MorphSectionHeader, QuantizedMorphDelta, FLAG_MORPH_DENSE, FLAG_MORPH_HAS_NORMALS,
    FLAG_MORPH_HAS_TANGENTS, FLAG_MORPH_ONTOGENETIC_BIND, FLAG_MORPH_SPARSE, MAGIC_MRP1,
    MAX_MORPH_DELTAS, MAX_MORPH_TARGETS, MORPH_SECTION_HEADER_SIZE, MORPH_SECTION_VERSION,
    QUANTIZED_MORPH_DELTA_SIZE,
};
pub use metric_check::{
    proposed_metric_descriptor, verify_descriptor_against_reality, MetricBranchDescriptor,
    MetricCompletenessDescriptor, MetricDivergence, MetricKind, BOUNDARY_CLIQUE_BRANCH_INDEX,
    METRIC_BRANCH_COUNT,
};
pub use metric_trait_gate::{
    measure_aabb_extents_zero_heap, verify_fauna_vs_vehicle_scale_invariant_zero_heap,
    verify_metric_trait_bounds_zero_heap, MetricTraitEnvelope, MetricTraitGateViolation,
    ENVELOPE_AQUILA_AUDAX, ENVELOPE_CORYMBIA_APARRERINJA, ENVELOPE_FIELD_UTILITY_4WD,
    ENVELOPE_MACROPUS_GIGANTEUS, ENVELOPE_ORNITHORHYNCHUS_ANATINUS, ENVELOPE_VOMBATUS_URSINUS,
};
pub use node_section::{
    parse_node_header, read_node, read_node_aos, read_node_soa, read_node_soa_lane,
    transpose_aos_to_soa, transpose_soa_to_aos, write_node_q_at, write_node_section_aos,
    write_node_section_soa, NodeMiniHeader, NodeSectionError, AXIS_COUNT, LAYOUT_AOS, LAYOUT_SOA,
    MAX_NODE_COUNT, NODE_MINI_HEADER_SIZE, TENSOR10D_SIZE,
};
pub use normal_section::{
    decode_mesh_frames, decode_mesh_frames_into, decode_mesh_frames_with_limits,
    inspect_mesh_frames, MeshFrameDecodeLimits, MeshFrameLayout,
    decode_normals, decode_tangents, encode_mesh_frames, encode_normals,
    frame_encoded_len, is_normal_section, MeshFrameStreams, NormalSectionError, MAX_NORMAL_COUNT,
    NORMAL_SECTION_HEADER_SIZE, NORMAL_SECTION_VERSION,
};
pub use procedural_synthesis::{
    decode_procedural_synthesis_section, encode_procedural_synthesis_section,
    evaluate_allometric_dimension, evaluate_log_spiral_2d, evaluate_pga_motor_oscillation,
    evaluate_smooth_minimum_sdf, evaluate_stress_cleavage_partition, evaluate_voronoi_cellular_f1,
    ProceduralSynthesisError, ProceduralSynthesisHeader, FLAG_ALLOMETRIC_GROWTH,
    FLAG_CELLULAR_VORONOI, FLAG_IMPLICIT_FILLET, FLAG_LOG_SPIRAL_ODE, FLAG_PGA_MOTOR_DYNAMICS,
    FLAG_STRESS_FRACTURE, MAGIC_PROC, PROCEDURAL_SYNTHESIS_HEADER_SIZE,
    PROCEDURAL_SYNTHESIS_SECTION_VERSION,
};
pub use provenance_section::{
    decode_provenance_section, encode_provenance_section, encoded_len as provenance_encoded_len,
    validate_provenance, ProvenanceMiniHeader, ProvenanceSectionError, ProvenanceSidecar,
    ProvenanceSidecarView, FLAG_HAS_VC, PROVENANCE_MAGIC, PROVENANCE_MINI_HEADER_SIZE,
    PROVENANCE_SECTION_VERSION,
};
pub use typed_payloads::{
    inspect_container_mesh_frames, read_container_mesh_frames,
    read_container_mesh_frames_into, read_container_mesh_frames_with_limits,
    read_container_morph_targets_into, TypedPayload,
    TypedPayloadError, TypedPayloadResolver,
};
pub use section::{
    encode_container, parse_section_table, AlignmentTier, SectionDescriptor, SectionInput,
    SectionTableError, SectionType, SECTION_DESCRIPTOR_SIZE,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub use spatial_index_section::{
    decode_spatial_index_section, encode_spatial_index_section, DecodedSpatialIndex,
    SpatialIndexMiniHeader, SpatialIndexSectionError, SPATIAL_INDEX_MINI_HEADER_SIZE,
};
pub use surfel_section::{
    decode_octahedral_normal, decode_surfel_section, encode_octahedral_normal,
    encode_surfel_section, generate_surfel_from_cluster, SurfelBinaryRecord, SurfelSectionError,
    SurfelSectionMiniHeader, FLAG_SURFEL_GAUSSIAN_COVARIANCE, FLAG_SURFEL_HIERARCHICAL, MAGIC_SURF,
    SURFEL_MINI_HEADER_SIZE, SURFEL_RECORD_SIZE, SURFEL_SECTION_VERSION,
};
pub use texture_coordinate_container::{
    attach_texture_coordinate_section, read_texture_coordinate_section_into,
    TextureCoordinateContainerError,
};
pub use texture_coordinate_section::{
    decode_texture_coordinates_into, encode_texture_coordinate_section,
    texture_coordinate_section_count, TextureCoordinateSectionError,
    TEXTURE_COORDINATE_SECTION_HEADER_SIZE, TEXTURE_COORDINATE_SECTION_VERSION,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub use topology_section::{
    decode_topology_section, encode_topology_section, TopologyMiniHeader, TopologySectionData,
    TopologySectionError, TOPOLOGY_MINI_HEADER_SIZE,
};

/// Versioned `.10d` container ABI. Increment only when public POD layouts or
/// caller-buffer contracts change. P0.1 sets this to 1; P0.2/P0.3 may raise it
/// when the section table + CRC lands.
pub const CONTAINER_10D_ABI_VERSION: u32 = 1;

// Explicit opaque retention is separate from strict typed section admission.
pub use section::{
    copy_opaque_container_into, inspect_opaque_container,
    replace_known_section_bytes_preserving_layout, OpaqueContainerView,
    OpaquePreservationBudget, OpaquePreservationError,
};
