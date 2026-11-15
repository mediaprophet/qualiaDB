# SKL1 skeletal animation section

Source implementation: `container_10d::skeletal_section` and
`container_10d::skeletal_container`. Container section tag: 16
(`SectionType::SkeletalAnimation`), word aligned, blob descriptor with zero
stride and element count. Existing per-section CRC and whole-file seal apply.
This format supports rigid joint transforms; authoring scale must be baked
into geometry before rigging. Runtime interpolation uses VibeScript motors.

The codec is implemented in source but has not yet been compiled or executed.
Portal animation playback and imported GLB skin conversion remain unfinished.
Recognizing the section does not establish an animated rendering capability.

All integers and floats are little endian. The payload has no implicit host
padding, pointers or strings. Its exact length is required; trailing bytes,
unknown versions, flags and nonzero reserved fields are rejected.

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

Authored, currently unexecuted fixtures cover canonical round trips followed
by native skinning, invalid inverse binds and weights, key ranges, loop seams,
binary padding, vertex mismatch, output capacity and sealed mesh preservation.
