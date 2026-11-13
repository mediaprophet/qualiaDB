# 10D container standard

**Revision:** 1, 12 November 2026. **Status:** Internal implementation-aligned standard. Writers emit version 2; readers also admit version 1 with its section restrictions. This document specifies the envelope and payload registry; the linked companions specify mesh numerics, typed fields, skeletal animation and opaque preservation. See the [format overview](file-formats-overview.md).

## 1. Scope

A `.10d` is a typed, sectioned container for geometry, Tensor10D nodes, scientific/presentation fields and associated structures. It is not simply a triangle file, nor is every recognized section automatically executable. The identity is the four bytes `31 30 64 00`. The same physical header layout spans v1/v2; typed MID2 identity belongs to v2.

EMF electromagnetic cells remain in the sibling `.emf` format. A 10D MID2 field reference and a FIEL declaration may describe or associate that field; they do not create an undocumented EMF-cell section. HMC may bundle the siblings without changing either encoding.

## 2. Fixed header: 64 bytes

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | bytes | `10d` plus NUL |
| 4 | 2 | u16 | Version: 2 current, 1 legacy readable |
| 6 | 2 | u16 | Flags; bit 0 default disposition Refuse |
| 8 | 10 | u8[10] | Axis roles |
| 18 | 2 | bytes | Zero padding |
| 20 | 32 | metric descriptor | Four 8-byte rows |
| 52 | 4 | u32 | Whole-container CRC-32C |
| 56 | 4 | u32 | Absolute section-table offset |
| 60 | 4 | u32 | Section count, maximum 1024 |

All numeric fields are little-endian. The CRC covers the **entire file with bytes [52,56) treated as zero**, not only the header. Calling the header parser alone does not perform whole-file integrity verification. The table pointer/count must either both be zero or locate a bounded table after the header; a nonzero table has at most 1024 entries.

Axis order is `q, v, w, x, y, z, t, alpha, mu, sigma`. Role tags are 0 undefined (rejected), 1 selector, 2 coordinate, 3 carrier and 4 coordinate/carrier. The proposed/default roles are `1,1,1,2,2,2,2,2,4,2`. These roles and the metric contract are checked against the runtime rather than treated as decorative metadata. Coordinate semantics are specified in the [Tensor10D companion](q42-10d-tensor-standard.md).

Each metric row stores v-class u8, metric-kind u8, axis-mask u16 and reserved u32. Metric-kind tags are 0 undefined, 1 Euclidean, 2 cyclic/toroidal, 3 hyperbolic, 4 boundary clique, 5 spherical HEALPix, 6 Lorentzian spacetime and 7 multiply-connected portal. Registry recognition alone does not expand the four serialized branch rows. The default describes Euclidean behavior over axes 3–9, cyclic/hyperbolic behavior over x/y/z, and a catch-all class 255 probing v=3 with no participating axes. This declaration is tied to the current `full_distance` behavior; it is not a promise of arbitrary extra topology or a user-defined metric interpreter.

## 3. Section table: 24 bytes per descriptor

**The descriptor is 24 bytes, not 32.**

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 1 | u8 | Section type |
| 1 | 1 | u8 | Alignment tier |
| 2 | 2 | u16 | Reserved, zero |
| 4 | 4 | u32 | Absolute payload offset |
| 8 | 4 | u32 | Payload length |
| 12 | 4 | u32 | Element stride |
| 16 | 4 | u32 | Element count |
| 20 | 4 | u32 | CRC-32C over exactly the payload bytes |

Alignment tiers 0, 1, 2 and 3 mean 1, 4, 16 and 64 bytes respectively. Writers order sections by ascending type, reject duplicate types and insert zero inter-section padding. `stride × count` must equal payload length under checked arithmetic; structured payloads may use a byte-oriented descriptor while their inner codec supplies actual record counts. Offsets/lengths are u32 and cannot be silently widened by embedding in a 64-bit outer container.

Strict parsing checks supported types, table/payload ranges, overlap, declared alignment, reserved fields, size consistency, applicable version constraints, section CRCs and padding between occupied ranges. Native borrowed table views additionally require suitably aligned backing memory. Trailing padding is not comprehensively validated by the current table parser; canonical writers zero their padding. Whole-file CRC verification remains a separate necessary check.

## 4. Allocated section types

| ID | Name | Payload contract |
|---:|---|---|
| 0 | Undefined | Rejected |
| 1 | QuantizedMesh | 40-byte mini-header, quantized vertices, triangle indices |
| 2 | Tensor10DNodes | 40-byte epistemic nodes; node codec and tensor semantics |
| 3 | Reconstruction | Reconstruction output; interpretation requires its consumer profile |
| 4–6 | Reserved | Governance, temporal index, manifold-head table; strict admission rejects |
| 7 | ProvenanceSidecar | Source/licence/credential material; presence is not credential verification |
| 8 | FieldSidecar | Versioned scientific or presentation field payload; dispatch by inner codec |
| 9 | CorrespondenceMap | FEMC finite-element/surface correspondence |
| 10 | Topology | Half-edges, CSR adjacency and connectivity summary |
| 11 | SpatialIndex | BVH/kd-tree arrays |
| 12 | SurfaceReading | Vertex-aligned RGBA8 reading, not a whole-part material |
| 13 | Materials | Versioned material records and submesh ranges |
| 14 | TextureCoordinates | UV01 vertex-aligned TEXCOORD_0 |
| 15 | ManifoldIdentityV2 | MID2 stable identity and typed field references; v2 only |
| 16 | SkeletalAnimation | SKL1 rig, skin influences and motor clips |
| 17 | EntityOntology | EONT behavior and affordance data |
| 18 | ProceduralSynthesis | Procedural geometry/surface descriptors |
| 19 | ImpostorAtlas | IMPO directional atlas |
| 20 | HierarchicalSurfels | SURF surfel/Gaussian primitives |
| 21 | HyperspectralCube | Scientific multiband field payload |
| 22 | VoxelLattice | VOX0 sparse voxel structure |
| 23 | ClusterDag | CDAG hierarchical meshlet cluster DAG |

Recognition of a registry ID establishes the outer type only. Apply its inner codec/version, cardinality, reference and resource checks before use. In particular, type 19 is **ImpostorAtlas**, not a morph-target section. Morph payloads use typed field associations.

## 5. Geometry and rendering payload profiles

### Quantized mesh

The mini-header stores flags u16 at 0 (bit 0 chooses u16 rather than u32 triangle indices), zero u16 at 2, vertex count u32 at 4, triangle count u32 at 8, minimum f32[3] at 12, maximum f32[3] at 24 and zero u32 at 36. It is followed by three u16 coordinates per vertex, then three indices per triangle. The decoder caps each count at 4,194,304 and checks bounds, indices and payload size. The precise dequantization and endpoint treatment are specified in the [numeric companion](10d-container-format.md); an independent implementation must follow that contract rather than a superficially similar float formula.

### Related geometry payloads

| Codec | Structural anchor | Additional obligations |
|---|---|---|
| Topology | 32-byte header; bounded half-edge and CSR arrays; maximum 2,097,152 half-edges | Validate vertex/face links and array relationships |
| Spatial index | 32-byte header; maximum 1,048,576 nodes | Validate node ranges and referenced geometry |
| UV01 | 16-byte header and UV coordinates | Preserve vertex correspondence; seam splitting may change vertex count |
| Materials | 24-byte header, 24-byte submesh rows; current v4 material records 464 bytes, previous 448 and legacy v1 256 | Select record size by inner version; validate submesh ranges and referenced resources |
| SurfaceReading | Vertex-order RGBA8 samples | Do not substitute for UV textures, normals or a complete BSDF |
| EONT | 40-byte header, 48-byte affordance rows; maximum 256 affordances | Interpret permissions/behavior through the application; no implicit execution |
| IMPO | 64-byte header | Atlas dimensions, orientation and image references must be validated |
| SURF | 64-byte header, 32-byte records | Validate finite geometry and rendering parameters |
| VOX0 | 64-byte header, 16-byte nodes | Bound graph traversal and resolve material LUT references |
| CDAG | 32-byte header, 64-byte node records; maximum 262,144 nodes | Meshlet limits 64 vertices and 124 triangles; validate hierarchy and ranges |

Detailed binary implementations are linked in §9. These profile versions are independent of the outer 10D header version. A future payload revision requires its own discriminator and readers; enlarging a struct without a revision is not compatible evolution.

## 6. Typed scientific fields and MID2

MID2 has a 112-byte header and 48-byte binding rows, with at most 256 bindings. Its kind registry includes scalar 1, vector 2, tensor 3, spectral 4, electromagnetic `0x0100`, frames `0x0200`, morph `0x0201` and geometry/topology complex `0x0300`. These are **not** the same numeric namespace as FIEL type tags. Resolve references under their specific unit, domain, identity and support conventions.

The FIEL v0 schema uses a 12-byte header: u32 magic at 0, u16 version 0 at 4, u16 field count at 6, zero u32 at 8. Its implemented magic integer is `0x4649454c`; little-endian serialization produces bytes `4c 45 49 46` (ASCII `LEIF`). The source calls this schema FIEL, but an implementation must use the actual wire value, not silently change it to the ASCII spelling. That naming/byte-order discrepancy requires a versioned repair if changed.

Each 32-byte FIEL descriptor contains name hash u64 at 0, unit-IRI hash u64 at 8, type u8 at 16, support u8 at 17, representation u8 at 18, zero u8 at 19, zero u32 at 20 and sample offset u64 at 24. Type tags 0–3 mean scalar/vector/tensor/spectral; support 0–3 means region/point/continuant/stream; representation 0–4 means grid/mesh/particles/analytic/sampled. A zero sample offset denotes a declaration without a raster payload. Raster-bearing v0 payloads add 16-byte grid metadata and little-endian f32 samples at section-relative offsets. The old comment “no samples yet” does not remove implemented raster support.

NRM1 and FRM1 supply normals/tangent frames. MRP1 morphs use a 32-byte header and 16-byte delta records, bounded to 256 targets and 1,048,576 deltas. FEMC uses a 16-byte header and 16-byte bindings, capped at 1,048,576 bindings. Their typed resolution and coexistence rules are defined in the [typed field companion](10d-typed-field-payloads.md). SKL1 has its own [skeletal companion](10d-skeletal-animation.md). Unknown field semantics must be preserved or refused, never guessed from the outer section alone.

## 7. Admission, preservation and integrity

A complete admission pipeline validates the header and metric/axis contract, whole-file CRC, section table and each section CRC, then the inner codec and all cross-section references needed for the requested operation. Apply explicit byte/count/scratch budgets before allocation. Geometry validity, scientific units, provenance credentials and governance authorization are additional checks.

Strict readers reject unknown and reserved section IDs. The separate [opaque preservation contract](10d-opaque-preservation.md) permits bounded copying and specified replacement without interpreting unknown bytes. Preservation does not admit execution or guarantee a future reader's meaning. Maintain unknown ranges and zero/unchanged layout according to that API; a general-purpose decode/re-encode that drops unknown data is not preservation.

The full BLAKE3 repair to EMF/M64 does **not** add a 10D hash field or trailer. Current 10D integrity is CRC-32C. Adversarial delivery requires a trusted outer commitment, such as a verified bundle entry bound to an authenticated manifest. CRC success, provenance text and an untrusted digest are not signatures. A proposed QDNF segment proof must be explicitly specified and cannot reuse an occupied or reserved header field without a versioned extension.

## 8. Determinism and illustrative placement

Canonical ordering makes the same admitted section set encode identically even when supplied in a different order. Byte identity additionally depends on stable payload bytes, versions and header values. For a single 64-byte payload with alignment tier 3: header [0,64), descriptor [64,88), zero padding [88,128), payload [128,192). The descriptor points to 128 and carries its payload CRC; the final whole-container CRC is written at offset 52 after all bytes are settled. This example does not dictate one element stride for all codecs.

Changes to section bytes require recalculating their section CRC and whole-file CRC, and any enclosing digest/signature. A codec migration must preserve semantic identity and explicit losses, not merely change the outer version integer.

## 9. Implementation and companions

Source authority: [header](../../../crates/qualia-core-db/src/container_10d/header.rs), [section registry/writer](../../../crates/qualia-core-db/src/container_10d/section.rs), [table checks](../../../crates/qualia-core-db/src/container_10d/section/table_validation.rs), [integrity](../../../crates/qualia-core-db/src/container_10d/integrity.rs), [module and payload codecs](../../../crates/qualia-core-db/src/container_10d/mod.rs), [MID2](../../../crates/qualia-core-db/src/container_10d/manifold_identity_v2.rs), [FIEL](../../../crates/qualia-core-db/src/container_10d/field_section.rs), [materials](../../../crates/qualia-core-db/src/container_10d/material_section_codec.rs), [VOX0](../../../crates/qualia-core-db/src/container_10d/voxel_section.rs) and [CDAG](../../../crates/qualia-core-db/src/container_10d/cluster_dag_section.rs). Refer to [conformance](file-format-conformance.md) for evidence and known gaps.
