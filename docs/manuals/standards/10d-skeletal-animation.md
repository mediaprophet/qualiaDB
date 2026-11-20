# SKL1, SKL2 & SSM1 Skeletal Animation and Biomechanical Rig Section

Source implementation: `container_10d::skeletal_section` (`mod.rs` and `skl2.rs`) and
`container_10d::skeletal_container`. Container section tag: 16
(`SectionType::SkeletalAnimation`), word aligned, blob descriptor with zero
stride and element count. Existing per-section CRC and whole-file seal apply.

All integers and floats are little-endian. No ASCII strings are stored in binary rig records: joint, clip, landmark, and pathway identifiers use 64-bit RDF `q_hash` identities resolved to full **UTF-8 / UTF-16** IRIs and multilingual labels via `.q42` `Q42LEX` or in-envelope CBOR-LD (`FIELD_KIND_SYMBOLIC_RDF_CBOR_LD = 0x0305`, `CBLD`).

---

## 1. `SKL1` Rigid Dual-Quaternion / PGA Motor Rig & Clip Codec (`SKL1`)

This format supports rigid joint transforms; authoring scale must be baked
into geometry before rigging. Runtime interpolation uses VibeScript / Etalon PGA motors (`etalon_render::skeletal`).

| Record | Bytes | Fields in order |
|---|---:|---|
| Header | 32 | `SKL1` magic (4), version u16 = 1, flags u16 = 0, joint/vertex/clip/track/key counts (five u32), reserved u32 = 0 |
| Joint | 144 | Parent u16 (`65535` for root), reserved (6), stable ID u64, rest-local motor (64), inverse-bind motor (64) |
| Clip | 32 | Stable ID u64, duration f64, first track u32, track count u32, looping u32 (0 or 1), reserved u32 = 0 |
| Track | 8 | Joint u16, key count u16, first key u32 |
| Key | 72 | Time f64, local motor (64) |
| Vertex skin | 24 | Four joint indices (four u16), four weights (four f32) |

Tables follow the header in the order shown. Motor components are eight f64:
`r_w, r_x, r_y, r_z, d_w, d_x, d_y, d_z`. Unit real quaternion norm and
real/dual orthogonality are checked. IDs are nonzero engine `q_hash` identities,
unique within their table; authoring provenance supplies the original names.

Joints must be parent-before-child. Composed rest transforms multiplied by
inverse bind must yield identity within 1e-5. Clips partition the complete
track table in order, and tracks partition the complete key table in order.
Each clip targets a joint at most once. Times are finite, strictly increasing
within a track, and within clip duration. Loops explicitly include time zero
and duration, with matching rigid poses across the seam. Missing joint tracks
use the rig rest pose. Negative visual time wraps for looping clips; nonlooping
clips clamp to their endpoints. Campaign time is separate.

Every vertex has four nonnegative finite weights summing to one within 1e-5.
Positive influences reference existing joints. Vertex count must match the
actual QuantizedMesh count, including UV seam duplication. The native chart
authoring lineage maps duplicated vertices back to their original influences.

Limits: 128 joints, 32 clips, 4096 tracks, 4096 total keys, 262144 vertices and
86400 seconds per clip. Maximum section size is below 7 MiB. The enclosing
container attachment rejects inputs or outputs over 42 MiB. Codec validation,
encoding, validated views and caller-buffered decoding allocate no heap;
constructing resident playback buffers is a separate cold lifecycle.

`SkeletalSection::parse` validates before exposing a zero-copy record view.
`decode_into` checks every caller buffer before writing any stream.
`attach_skeletal_section_into` preserves existing container sections, checks
mesh alignment and integrity, writes through the canonical section writer and
reseals only the written output span. Its capacity error reports the required
size. Duplicate skeletal sections are refused. Legacy static meshes may omit
this section; malformed optional sections are refused by game pack validation
and multi-asset portal admission.

Authored fixtures cover canonical round trips followed
by native skinning, invalid inverse binds and weights, key ranges, loop seams,
binary padding, vertex mismatch, output capacity and sealed mesh preservation.

---

## 2. `SKL2` Biomechanical Multi-Regime Rig & `SSM1` Statistical Shape Model Basis (`skl2.rs`)

`skeletal_section::skl2` extends `SectionType::SkeletalAnimation` (`16`) for multi-regime biological (`intenteco`) and mechanical (`qualia-poiesis`) kinematics up to the **128 MiB** `GRAPHICS_ASSET_RING_BYTES` (`SKL2_MAX_CONTAINER_BYTES`) envelope:

1. **Header (`32` bytes):** `SKL2` binary discriminant (`4B`), version `u16` (`1` for 32-byte joint descriptors, `2` for 64-byte cache-line-aligned joint descriptors), flags `u16`, joint count `u16` (max `256`), loop-closure count `u16`, landmark count `u16`, actuation-pathway count `u16`, SSM1 basis presence flag, and CRC-32C integrity check.
2. **Five Kinematic Solver Regimes (`KinematicSolverRegime`):**
   * `0 = ArticulatedRigidBodyDca` (Featherstone Divide-and-Conquer / ABA articulated rigid bodies)
   * `1 = CosseratRodContinuum` (plant stems, tails, flexible cables, vascular trees)
   * `2 = ExoskeletalCuticleShell` (arthropod bicondylar hinges, apodemes, hemolymph hydraulics)
   * `3 = HydrostaticMuscularContinuum` (tongue, trunk, cephalopod, soft-tissue musculature)
   * `4 = TurgorAuxinGrowth` (botanical phototropism/gravitropism and turgor-driven morphogenesis)
3. **Joint Descriptors (`Skl2JointDescriptorV1` `32B` / `Skl2JointDescriptorV2` `64B`):**
   * Stores parent index (`0xFFFF` for root), symmetry partner index, solver regime, joint flags (`SYMMETRIC`, `VESTIGIAL`, `HYDROSTATIC`, `EXOSKELETAL`, `LOOP_ANCHOR`), `SwingTwistConeLimits` (`16B` anatomical cone limits in radians), rest-pose anchor, and `EpistemicProvenanceWord` (`32b` bit-packed tier, observation method, observer role, temporal stability class, freshness, and confidence).
4. **Loop Closures, Landmarks & Actuation Pathways:**
   * `LoopClosureConstraintRecord`: Closed-chain mechanical linkages (4-bar suspension, ribcage, closed robotic chains).
   * `TopologicalLandmarkRecord`: Anthropometric / morphometric landmarks anchored to joints/vertices.
   * `ActuationPathwayRecord`: Hill-type muscle-tendon routing or hydraulic/electrical actuator lines of action.
5. **`SSM1` Statistical Shape Model Basis (`Ssm1BasisHeader` `32B` + `Ssm1ModeWeight` `16B`):**
   * Defines the canonical Mode-A Archetype PCA shape space (`mode_id: u32`, `eigenvalue: f32`, `min_sigma: f32`, `max_sigma: f32`).
   * **Per-Instance Realization:** Individual Mode-B instances supply their realized $\sigma_i$ mode weights via **`FIELD_KIND_INSTANCE_OVERRIDES = 0x0302` (`IOVR`)** (or `ssm_weights_q11: [i16; 4]` in **`FIELD_KIND_INSTANCE_BATCH = 0x0303` (`INST`)**), which combine zero-heap on the GPU with `EONT` ontogenetic/metabolic state (`compute_ontogenetic_morph_weights_zero_heap`).
