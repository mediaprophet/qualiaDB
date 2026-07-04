//! `.10d` living-container v1 — the normative header, axis-role taxonomy, and
//! metric-completeness descriptor that serialize the 10-D tensor runtime.
//!
//! This is the **P0.1 barrier task**: the surface every later P0 task (section
//! table, CRC-32C integrity, quantized-mesh section, Tensor10D node section,
//! renderer upload path, conformance vectors, WASM parity) consumes. It lands
//! alone first; no swarm fan-out on P0.1 itself.
//!
//! Scope of P0.1 (what is implemented-in-code here vs. spec-reserved):
//!
//! - **Implemented:** `AxisRole` taxonomy + the proposed (not-yet-frozen)
//!   Option A table; `MetricCompletenessDescriptor` + the
//!   `verify_descriptor_against_reality` gate that introspects
//!   `Tensor10D::full_distance`; `Container10dHeader` POD (64 bytes, repr(C),
//!   zero padding asserted) with `encode`/`parse` running every P0.1
//!   acceptance gate (bad magic / unknown version / undefined axis role /
//!   non-zero padding / metric-completeness divergence).
//! - **Spec-reserved (NOT yet implemented — do not report as working):**
//!   `header_crc32c` (P0.3 wires the shared CRC-32C delegated from
//!   `q42/p64_weight.rs`); the section table, tiered alignment, and
//!   per-section CRC (P0.2); the quantized-mesh section (P0.4); the Tensor10D
//!   node section (P0.5); the renderer upload path (P0.6); conformance
//!   vectors (P0.7); WASM parity build gate (P0.8).
//!
//! Two ⚑ format decisions were resolved by Timothy Charles Holborn on
//! 2026-07-04 and are encoded here as the not-yet-frozen defaults:
//!
//! 1. **Axis-role taxonomy (Option A):** `q,v,w` = `Selector`; `x,y,z,t,α,σ`
//!    = `Coordinate`; `μ` = `CoordinateCarrier` (dual-role: a coordinate that
//!    is also the in-band provenance carrier).
//! 2. **Metric-completeness (option b — document the limitation):** the header
//!    encodes the current `full_distance` reality. `v=0` Euclidean folds all
//!    seven COORDINATEs; `v=1` Cyclic and `v=2` Hyperbolic fold `x,y,z` only;
//!    `v>=3` Boundary clique folds no coordinate axes (byte-equality on `v`).
//!    P7.9 (making the non-Euclidean metrics axis-complete via product /
//!    warped-product manifolds and a weighted clique-graph) is deferred as
//!    future geometry design work — see the progress log.
//!
//! Reference: `docs/plans/native-computational-geometry.md` §4.1,
//! `docs/plans/native-computational-geometry-EXECUTION.md` P0.1, and the
//! cross-cutting gate "Honest axis-role taxonomy & metric-completeness
//! (queryability claim == code)".

pub mod axis_role;
pub mod conformance;
pub mod crc32c;
pub mod header;
pub mod integrity;
pub mod mesh_section;
pub mod metric_check;
pub mod node_section;
pub mod section;

pub use axis_role::{
    AxisRole, AXIS_ORDER, COORDINATE_AXES, MU_AXIS, PROPOSED_AXIS_ROLES, SELECTOR_AXES,
};
pub use crc32c::{crc32c, crc32c_update};
pub use header::{
    Container10dHeader, HeaderParseError, FLAG_DEFAULT_DISPOSITION_REFUSE, HEADER_BYTE_SIZE,
    HEADER_VERSION, MAGIC_10D, MAX_SECTION_COUNT,
};
pub use integrity::{
    compute_whole_file_crc32c, seal_whole_file_crc32c, verify_whole_file_crc32c, IntegrityError,
};
pub use metric_check::{
    proposed_metric_descriptor, verify_descriptor_against_reality, MetricBranchDescriptor,
    MetricCompletenessDescriptor, MetricDivergence, MetricKind, BOUNDARY_CLIQUE_BRANCH_INDEX,
    METRIC_BRANCH_COUNT,
};
pub use mesh_section::{
    decode_mesh_section, encode_mesh_section, parse_mesh_header, MeshMiniHeader, MeshSectionError,
    FLAG_U16_INDICES, MAX_TRIANGLE_COUNT, MAX_VERTEX_COUNT, MESH_MINI_HEADER_SIZE,
};
pub use node_section::{
    parse_node_header, read_node, read_node_aos, read_node_soa, read_node_soa_lane,
    transpose_aos_to_soa, transpose_soa_to_aos, write_node_q_at, write_node_section_aos,
    write_node_section_soa, NodeMiniHeader, NodeSectionError, AXIS_COUNT, LAYOUT_AOS, LAYOUT_SOA,
    MAX_NODE_COUNT, NODE_MINI_HEADER_SIZE, TENSOR10D_SIZE,
};
pub use section::{
    encode_container, parse_section_table, AlignmentTier, SectionDescriptor, SectionInput,
    SectionTableError, SectionType, SECTION_DESCRIPTOR_SIZE,
};

/// Versioned `.10d` container ABI. Increment only when public POD layouts or
/// caller-buffer contracts change. P0.1 sets this to 1; P0.2/P0.3 may raise it
/// when the section table + CRC lands.
pub const CONTAINER_10D_ABI_VERSION: u32 = 1;
