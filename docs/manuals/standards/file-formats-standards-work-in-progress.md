# File-format standards — resumable work checkpoint

**Checkpoint date:** 2026-11-12 (Australia/Sydney)  
**State:** Completed. The [format overview](file-formats-overview.md) links all six finished standards and their conformance record. The material below is historical research, not the current specification.  
**Requested destination:** `C:/github/qualiaDB/docs/manuals/standards`

## Completion update — 12 November 2026

Completed the six implementation-aligned format entry points, common overview and defect/conformance register. Reconciled P64 v4 and ETLIMD1/ETLMOE2 metadata, 10D's 24-byte descriptors, both HMC encodings, and E64's lack of additional functionality beyond its M64 compatibility spelling. The user-supplied expert proposal is preserved verbatim under `references/` and explicitly remains non-normative. Current EMF v2/M64 codec 3 use full BLAKE3; old prefix behavior below is historical and defective.

Completion verification: 61 production-module conformance tests and 33 Etalon default-feature tests (including 21 P64 tests) passed. Prior producer checks remain 7 electromagnetics and 5 geodesy tests. The [conformance record](file-format-conformance.md) states exact scope, commands and untested areas. No network protocol implementation or running MCP replacement was part of this documentation completion.

## Historical checkpoint follows

## Resumption update — 12 November 2026

The laptop-travel pause ended. The original research below is a historical snapshot; its statements that no code changed or tests ran describe that earlier checkpoint only.

The latest requested deliverable is now the [QBDL streaming expert advisory brief](qbdl-streaming-expert-advisor-brief.md). It follows the user's clarified direction: a native QDNF content-service extension, HTTP backward compatibility, post-quantum cryptography, and integration with existing Permissive Commons systems. QPU historical-solution reuse is one application, not the protocol's architectural center. Local source links were checked. No streaming wire profile or service registration has been frozen.

Related authorized integrity repair is implemented in the working tree: EMF v2 and M64/E64 codec 3 carry full 32-byte BLAKE3 digests in a 64-byte trailer (remaining 32 bytes zero). Legacy prefix encodings require explicit migration admission. Headers/records retain their sizes; payload access excludes trailers. This is full-digest checking, not Bao proof verification or PQ network authentication. Hypermedia EMF and M64 sealed production paths were updated. Existing raw M64 encodings remain distinct and do not claim full-hash assurance.

Verification so far: the focused production-module conformance harness passed 61 tests; the actual asset-production-vibe integration tests passed 7 electromagnetics and 5 geodesy tests. Geodesy initially exposed an obsolete producer self-check expecting a nonzero truncated prefix; it was corrected to require a verified full-hash chunk and no legacy-prefix chunk, and all 5 tests then passed. No running MCP binary was replaced or restarted. The six comprehensive file-format standards have not yet been finished; continue that separate deliverable from the source findings below, applying the new integrity layouts rather than the historical prefix layouts.

Files added for the repair: `crates/qualia-core-db/src/q42/chunk_integrity/` and `tools/container-format-conformance/`. Changed codec files: EMF header/directory/writer/reader/module, M64 directory/checked reader, q42 module export. Related Hypermedia files: `geodesy_ops.rs`, `electromagnetics_ops.rs`. Preserve unrelated working-tree changes. Recheck source status before continuing; independent development is active.

## Authorized deliverable

Write comprehensive standards documents for **EMF, E64, M64, 10D, P64, and HMC**. HMC was added explicitly during research. Cover physical layouts, exact byte offsets and primitive encodings, version compatibility, record/section registries, integrity, validation, limits, semantic boundaries, producer/consumer behavior, examples/conformance evidence, and implementation references. Maintain useful existing documents and update the standards index.

**User correction that governs the work:** truncated BLAKE3 checking is a **bug, not a feature**. Record existing prefix bytes for compatibility, but do not endorse truncated hashes as the normative integrity design or call their comparison Bao proof verification. Separate observed wire behavior, required integrity guarantees, known defects, and migration work. At the original travel checkpoint, no integrity implementation changes had been made; see the resumption update for subsequent repairs.

At the original travel checkpoint, only this research document had been added, no final standard was complete, and no tests or background processes had been run. Subsequent work is recorded immediately below.

## Work approach on resumption

1. Recheck working-tree status and relevant source changes before continuing; other work is active.
2. Finish focused source reads listed below. Source bodies supersede stale prose/comments; do not turn reader omissions or bugs into approved design.
3. Write a family overview and six clear format entry points. Expand/reconcile existing M64/P64 documentation. Preserve the valuable 10D numeric companion, either by retaining it with a broader standard entry point or carefully integrating its content.
4. Include a defect/compatibility register, particularly EMF/M64 truncated BLAKE3 and HMC profile disagreements. Do not allocate new wire fields/version numbers without an approved implementation design.
5. Verify tables against structs/codecs, source links and local cross-links, integrity scope, offset arithmetic, and examples. Label source inspection versus actual test execution honestly.
6. Update `DIRECTORY_INDEX.md` and relevant existing standards navigation. Deliver links to finished documents.

## Existing standards and important corrections

Directory already contains:

- `10d-container-format.md`: despite its filename, currently a focused QuantizedMesh numeric companion, not a complete envelope specification.
- `10d-opaque-preservation.md`, `10d-typed-field-payloads.md`, `10d-skeletal-animation.md`, `q42-10d-tensor-standard.md`: useful companion material requiring review.
- `m64-matter-container.md`: substantially stale. Says header CRC covers only first 56 bytes; reader only admits raw codec 0; no Bao check; omits shell mode. Current source differs as detailed below.
- `m64-georaster-v3.md`: developmental GRM3 profile with detailed 128-byte metadata table; preserve status and distinguish from default v2 and legacy v1 raster writer.
- `p64-weight-container-standard.md`: long, useful document but internally contradictory v3/v4 statements, obsolete reserved fields, and unconditional page alignment claims conflicting with layer packing.
- `DIRECTORY_INDEX.md`: dated inventory; currently describes P64 v3.

Repository rules: QualiaDB `AGENTS.md` was inspected for constraints and handoff instructions. Its historical modality task list is unrelated to this docs task. The user's Synth contract requires consulting the Hypermedia function index before claiming functions absent. Relevant E64 entries were found there. No missing-format claim is justified by filename absence.

## EMF — verified findings

Authoritative files under `crates/qualia-core-db/src/q42/emf_container/`:
`header.rs`, `chunk_directory.rs`, `writer.rs`, `reader.rs`. Main implementations have been read.

- Magic `Q42EMF64` (8 bytes), version `u16 = 1`, little-endian current reader.
- Header 64 bytes: magic@0[8], version@8 u16, flags@10 u16, chunk_count@12 u32, directory_offset@16 u64, reference_epoch_s@24 f64, grid_dims_bricks@32 u32[3], voxel_size_meters@44 f32, origin_meters@48 f32[3], header_crc32c@60 u32. Header CRC covers `[0,60)`.
- Header flags bits 0..3: 8³ bricks, Stokes polarization, spectral bands, nominal Bao verified. Do not imply full Stokes I is an independently stored scalar: cell carries Q/U/V and flux.
- Directory row 64 bytes: Hilbert key@0 u64, payload offset@8 u64, length@16 u32, cell_count@20 u32, spectral_band_mask@24 u16, flags@26 u16, time_slice_index@28 u32, bao_hash_prefix@32[16], reserved@48[12], CRC@60 u32 (covers first 60 bytes).
- Cell 64 bytes, 16 f32 lanes: E[3]@0; B[3]@12; poynting_flux@24; wavelength_nm@28; refractive_n@32; extinction_k@36; coherence_phase@40; stokes_q/u/v@44/48/52; spectral_sigma@56; confidence_alpha@60.
- 8³ = 512 cells = 32768 bytes per full brick. Macro keys use 3D Hilbert; local cell order uses 3-bit Morton interleaving (x bits at 3i, y at 3i+1, z at 3i+2).
- Writer sorts stably by spatial Hilbert key only, directory starts at 64, payloads follow directory. It does not enforce exactly 512 cells in `add_brick_chunk`; structural parser only requires length = count*64. Full-brick semantic conformance must be distinguished from permissive structural admission.
- **Integrity defect:** writer computes `blake3::hash(payload)[0..16]`, stores it as `bao_hash_prefix`, and sets row flag bit 1. Reader compares the same prefix if row bit 1 is set OR prefix is nonzero. No outboard Bao proof is serialized/verified by this path. Header bit 3 is not the trigger. Preserve compatibility description; mark as bug requiring actual full-digest/proof design and migration.
- Parse defaults: 16 MiB file, 4096 chunks; explicit limits available. Checks header and row CRCs, offsets/checked arithmetic, alignment, metadata/payload overlap, pairwise payload overlap, byte count and conditional prefix comparison. Does not establish scientific validity, sorted directory or complete-brick semantics.
- Dense unpack limits 1,048,576 cells across all times. Dimensions in cells = brick dimensions*8; nt=max(time_slice)+1. Dense ordering `[t][z][y][x]`; missing cells receive `EmfCellRecord64::default()` (not raw zero; n=1, confidence=1).
- Hilbert order is not serialized. Dense unpack tries `[2,3,1,4,5,6,7,8]`, requiring keys round-trip within dimensions and unique `(x,y,z,time)`; empty directory returns order 2. This is an ambiguity/compatibility concern, not a universal Hilbert convention.
- Lookup by Hilbert key alone does not disambiguate time slices.
- `effective_sigma`: nonzero authored sigma wins; otherwise clamp((wavelength_nm-400)/300,0,1) if wavelength positive, else zero.
- Legacy 52-byte helper maps amplitude/phase/frequency and selected manifold lanes to the new cell; it does not establish a second supported container magic.
- `.10d` MID2/FIEL names the electromagnetic field; durable scientific cells remain sibling `.emf`. Read `render/portal_load_emf.rs`, seal `volume_emf.rs`, and relevant conversion source before specifying consumer behavior.

## E64 / M64 — verified findings

Authoritative directory: `crates/qualia-core-db/src/q42/m64_matter/`.
Read: `layout.rs`, `chunk_directory.rs`, `point_record.rs`, `holo_record.rs`, `strata_record.rs`, `phase_juncture.rs`, `shell_record.rs`, first 385 lines of `checked_reader.rs`, `lrm3.rs`, first 180 lines of `brick3d.rs`.

- `m64\0` canonical; `e64\0` legacy alias with identical wire interpretation. `E64Header`, `E64PointRecord`, `E64HoloRecord`, `E64ShellRecord` are aliases. E64 is not EMF.
- Default version constant 2; checked reader admits 1,2,3. V1/v2 header_size=64; v3 header_size=192. V3 checked reader requires canonical `m64\0`, so do NOT promise E64 alias support for GRM3.
- Header 64 bytes: magic@0[4], version@4 u32, header_size@8 u64, total_file_size@16 u64, spatial_origin@24 f32[2] lat/lon degrees, elevation_datum@32 f32[2] base altitude metres/scale, chunk_table_offset@40 u64, chunk_count@48 u64, CRC@56 u32, container_flags@60 u32.
- **Current writer header CRC covers concatenation `[0,56)` + `[60,64)`; legacy first-56-byte CRC remains accepted.** Not equivalent to zeroing the CRC field over all 64 bytes. Legacy check leaves flags outside CRC protection; clearly document this limitation.
- Known container flag bits: 4 = 16³ bricks, 5 = LRM3. Checked reader rejects other bits.
- Directory 64 bytes: chunk_uuid@0[16], Hilbert key@16 u64, offset@24 u64, length@32 u32, count@36 u32, SRID@40 u32, LUT epoch@44 u16, mode@46 u8, codec@47 u8, bao prefix@48[12], CRC@60 u32 (first 60 bytes).
- Modes 0 Point, 1 Holo, 2 Strata, 3 PhaseJuncture, **4 Shell**. All records 64 bytes. Point/Holo have no embedded record CRC; other three have CRC over first 60 bytes stored at 60.
- Current checked reader admits codecs 0 and 2; rejects 1 (LZ4) despite descriptor comment naming it. Both admitted modes still have raw count*64 payloads.
- **Integrity defect:** codec 2 OR nonzero prefix invokes `blake3::hash(payload)[0..12]` comparison. No Bao proof tree verification here. Same user-directed defect treatment as EMF.
- Explicit file/chunk/total-record budgets and one caller scratch interval per chunk; exact total_file_size match; nonempty chunks; checked arithmetic; metadata and pairwise payload overlap; CRCs scanned for Strata/Phase/Shell.
- No promise that physical coordinate values, shell references or scientific units are validated by structural admission.
- Shell fields: face_id@0 u32; vertex_indices@4 u32[3]; edge_indices@16 u32[3]; normal@28 f32[3]; centroid@40 f32[3]; crease@52 f32; flags@56 u32; CRC@60.
- LRM3 128-byte block at header.header_size, entirely before directory, activated by bit 5. Magic/version/CRC validated; numeric scientific validity is not comprehensively enforced by block `validate`.
- LRM3: magic@0[4], version@4 u16=1, scale_tier@6 u8, frame@7 u8 (0 Cartesian,1 cylindrical,2 spherical), DID hash@8 u64, origin_metres@16 f64[3], quaternion xyzw@40 f32[4], scale_um@56 f32[3], bbox_min_um@68 i32[3], bbox_max_um@80 i32[3], reserved@92[32], CRC@124 over first 124 bytes. Cartesian transform helper scales by 1e-6, rotates, translates; do not invent cylindrical/spherical conversion behavior.
- 16³ = 4096 records = 262144 bytes, 4-bit local Morton order; 3D Hilbert code supports order up to 21 (clamps/masks inputs). Profile must state order/frame rather than imply serialized fields absent in base header.
- GRM3 metadata layout already in `m64-georaster-v3.md`; read `georaster/wire.rs` and `validation.rs`, plus `qualia-geospatial/src` writer before reconciling.
- Check ancillary LUT record semantics in `lut_entry.rs` and `banked_lut.rs`; do not invent embedded LUT section offsets.

## 10D — verified findings

Authoritative directory: `crates/qualia-core-db/src/container_10d/`.
Read `header.rs`, registry/descriptor definitions in `section.rs`, `section/table_validation.rs`, most of `integrity.rs`, `axis_role.rs`, first 245 lines `metric_check.rs`. Some long tool output was truncated; reread targeted portions as needed.

- Magic `10d\0`, current writer version 2, reader supports 1 and 2; fixed header 64 bytes.
- Header: magic@0[4], version@4 u16, flags@6 u16, axis_roles@8[10], zero pad@18[2], metric descriptor@20[32], whole-file CRC@52 u32, table_offset@56 u32, section_count@60 u32. Flag bit0 default disposition refuse. Header-only parse does NOT verify whole-file CRC.
- CRC stored at 52 is whole-file CRC with `[52,56)` treated as zero. Explicit seal/check functions in `integrity.rs`; do not repeat stale header comments that call it unused/reserved.
- Axis order `[q,v,w,x,y,z,t,alpha,mu,sigma]`; role codes Undefined0 rejected, Selector1, Coordinate2, Carrier3, CoordinateCarrier4. Writer defaults `[1,1,1,2,2,2,2,2,4,2]`; parser accepts other nonzero defined roles. t is coordinate time; mu is provenance coordinate/carrier.
- Metric descriptor 4*8-byte rows: v_class u8, metric_kind u8, folded_axes u16, reserved u32. Default Euclidean folds axes3..9; cyclic and hyperbolic xyz only; catchall row probes v=3, no axes. Current source also defines extended metric tags; do not imply header has serialized rows for each extended topology.
- Descriptor 24 bytes: type@0 u8, alignment_tier@1 u8, reserved16@2, byte_offset@4 u32, byte_length@8, stride@12, element_count@16, crc32c@20.
- Tiers 0/1/2/3 = 1/4/16/64-byte alignment. Names CacheLine/Page do not mean OS page size.
- Max sections1024; canonical writer sorts unique types, zero padding. Nonzero stride requires stride*count=payload length. Table is borrowed as typed descriptors, so input-base alignment matters independently of wire offsets. Empty sections occupy no bytes; trailing padding not checked by table padding helper.
- Registry: 1 QuantizedMesh;2 Tensor10DNodes;3 Reconstruction;4..6 reserved Governance/TemporalIndex/ManifoldHeadTable;7 ProvenanceSidecar;8 FieldSidecar;9 CorrespondenceMap;10 Topology;11 SpatialIndex;12 SurfaceReading;13 Materials;14 TextureCoordinates;15 ManifoldIdentityV2;16 SkeletalAnimation;17 EntityOntology;18 ProceduralSynthesis;19 ImpostorAtlas;20 HierarchicalSurfels;21 HyperspectralCube;22 VoxelLattice;23 ClusterDag. MID2 type15 requires outer v2. Strict parser refuses undefined/reserved/unknown; separate opaque preservation API exists.
- Existing mesh companion accurately details 40-byte miniheader, quantized u16 xyz, u16/u32 triangles, finite ordered bounds, f64 interior decode and exact endpoint bits; retain these details.
- Still read section-specific codecs and existing typed/opaque/skeletal/tensor docs to provide a complete payload map and references. No claim that merely recognizing a section tag proves full semantic conformance.

## P64 — verified findings

Canonical mechanics now in **Etalon** `C:/github/etalon/crates/etalon-model/src/p64/`; Qualia `q42/p64_weight/layout.rs` re-exports them. Read Etalon `layout.rs`, `reader.rs`, `integrity.rs`. Existing standard read (some output truncation); needs substantial reconciliation.

- Magic `p64\0`, writer constant version4, current `P64View::parse` requires exactly4. Do not promise v3 soft alias acceptance.
- 64-byte header offsets in existing document mostly correct: role table8, tensor table12, tokenizer16, hparams20, string table24, checksum28, manifold32, tensor_count36, page_size40, reserved44..64.
- Header flags0..6: LE, raw transcode, ternary, Q4K_SOA, layer-major, layer-pack, layer-schedule. Page default16384; configured log2 8..30, zero selects default.
- HParams bytes28..56 are **not reserved**: head_dim@28, head_dim_swa@32, sliding_window@36, shared_kv_layers@40 (u32), logit_softcap@44 f32, architecture@48 u32, arch_flags@52 u32. Only56..64 reserved. Read/write helpers zero final reserved bytes; original-byte preservation must use borrowed file bytes.
- Tensor descriptor last8 bytes are **not reserved**: alt_dtype@56 u16, precision_views_mask@58 u16, alt_blob_offset@60 u32. Other offsets match existing standard. Need inspect `views.rs`, writers and execution consumers before specifying alternate-view extent/checksum coverage (no alt length field in descriptor).
- Precision masks bits0..6 F32,F16,BF16,Q8_0,Q4_K,SOA,TERNARY_158.
- Additional roles12 attn_subln,13 ffn_subln,0x20 MoE router,0x21 gate experts,0x22 up experts,0x23 down experts,0x24 shared gate,0x25 shared up,0x26 shared down; unknown0xfffe/global ingest sentinel0xffff.
- `P64View::parse` integrity explicitly Structure / Metadata / Full. Full checks metadata CRC and each primary blob CRC. Structure checks neither; Metadata checks only metadata. Do not claim every parse call verifies all blobs.
- Metadata order enforced hp_end<=tensor_start; tensor_end<=strings<=manifold; manifold_end<=tokenizer<=checksums; checksum_end<=file length. Need call out remaining validation gaps rather than imply parser enforces all reserved/unknown flags/finiteness/shape semantics.
- Blob floor align(checksum_end,page). Default all blobs page aligned. With layer-pack: first blob and first following a manifold_idx change page aligned; consecutive same-manifold blobs 256 aligned. All primary blobs ordered/nonoverlapping/in bounds.
- Optional schedule checked on `schedule_bytes()` access, not certified during parse; starts64-aligned after hparams, ends<=tensor table. Semantic schedule consistency still needs consumer validation.
- Metadata CRC first checksum entry covers `[0,checksum_offset)`; tensor CRC table has N subsequent u32 values. CRC is corruption detection, not authentication.
- Need inspect Etalon builder/converter tests and Qualia compatibility reader, model architecture registry, tokenizer Q42T and alt-view behavior. Existing standard's living-draft note points to `docs/plans/p64-decode-upgrade-plan.md`; update relevant tracking if documenting decode gaps.

## HMC — verified findings and scope

**At least two actual encodings use `.hmc` in this tree. Do not conflate them.**

### Core/Synth transparent bundle: QBDL v1

Read `crates/qualia-core-db/src/bundle/{format.rs,writer.rs,reader.rs,reader_limits.rs}` and Synth `src/production/package.rs` first180 lines.

- Synth production calls Core BundleWriter/BundleReader. Magic **QBDL**, not HMC1, version1.
- Fixed64-byte explicitly LE header: magic@0[4],version@4 u16,flags@6 u16,entry_count@8 u32,index_offset@12 u64,index_length@20 u64,total_length@28 u64,CRC@36 u32,reserved@40[24]. Unaligned u64 wire offsets require byte decoding, not a padded native struct.
- Whole-file CRC with bytes36..40 zeroed. Index is CBOR footer, payloads verbatim. Base alignment64; optional header flag1 uses4096 for entries/index. A64-aligned outer entry does not universally establish absolute4096/16384 alignment of embedded P64; relative slices preserve inner offsets.
- CBOR index is array of maps: required key text,kind text,offset u64,length u64,sha256 Vec<u8>; optional meta Option<Vec<u8>>, omitted when None. Current ordinary serde Vec serialization may be CBOR integer arrays; bounded preflight admits byte strings or definite u8 arrays. Inspect actual fixture before publishing exact canonical bytes.
- Writer preserves insertion order, not lexical key order. Empty/duplicate keys rejected; general writer does not enforce all read budgets or kind validation.
- Legacy `parse` checks magic/version/length/whole CRC and entry bounds; 4096 alignment when flag set. Does not establish all bounded-profile guarantees (duplicates/count/digest/unknown fields etc.).
- `parse_with_limits` preflights definite untagged CBOR, exactly known5..6 fields, uniqueness, types, sizes, index ends exactly at EOF, header count matches index, reserved zero; **currently demands flags==0**, so refuses writer's4096 flag. Mark compatibility/implementation defect, not absent feature.
- Bounded reader additionally requires nonempty keys/kinds, SHA32,64 alignment, unique keys, disjoint ranges, and verifies all SHA256 payloads. Defaults file128MiB,index1MiB,entries4096,key4096B,kind64B,meta64KiB,decoded-index logical reservation8MiB (4x retained storage; not measured RSS).
- General keys are logical identifiers; extractor must separately prevent traversal. Native mmap reader uses legacy parse. Full SHA digest without a trusted external commitment is not issuer authentication.
- Still read `streaming_loader.rs`/`streaming_hydration.rs`, entity manifest profiles and tests; distinguish header+index planning from whole-file verification and range-specific cryptographic assurance.
- Synth `synth/production.json`, schema `synth.production/v1`: producer/version,source revisions,retained inputs {key,kind,sha256}. Inputs under inputs/, outputs under outputs/ except files producer2 content-addressed image textures retain textures/sha256/ keys. Reopen verifies resources and producer compatibility; retained source/inputs drive replay.

### Semantic-library document HMC: ZIP + JSON

Read `crates/qualia-semantic-library/src/container/mod.rs` first330 lines; still read `manifest.rs` and tests.

- ZIP archive, `manifest.json` written first and Stored. Source at source/<sanitized-name>; derived document HTML/text/CML/chunks; embeddings and meta subdirectories.
- Source already-compressed MIME types Stored; other members Deflated. Not QBDL transparent mmap layout.
- Manifest `format` must equal "hmc". Asset checksums are **full BLAKE3 hex**, independent of EMF/M64 prefix bug. `from_bytes` opens/reads manifest; `verify()` separately hashes all listed assets.
- Reader is allocation based; decompression/member budgets and path validation need explicit profile requirements; do not claim implemented safeguards without inspection.
- Some CLI code sniffs literal HMC1 (`qualia-cli/src/handlers/entity_verify.rs`), but a corresponding HMC1 writer has not been traced. This is an investigation item, not proof of another wire specification.

## Working-tree preservation

Initial `git -C C:/github/qualiaDB status --short` already showed unrelated modified source in render/assets, vertebrate_anatomy, qualia-mcp authoring/registry/tools, and webizen-render, plus new anatomy/preview sources and graph-theatre PNGs. Preserve all of that. No commit, push, cleanup, or source modification was performed for this standards task.

