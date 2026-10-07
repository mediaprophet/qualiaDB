# AAA Graphics Swarm Implementation Work Order

Status: **active work order; implementation and verification evidence must be recorded by the assigned agent**.  
Owner: QualiaDB/Webizen graphics programme.  
Parent specification: `C:/github/game-demo/docs/planning/31-aaa-graphics-engine-gap-register.md`.  
Primary implementation repository: `C:/github/qualiaDB`.

This work order turns the AAA graphics gap register into bounded, parallel work packages. It is written for an implementation agent or a coordinated agent swarm that will make code changes, integrate them, verify the result, and update the parent register. Reading this file is not evidence that any capability is implemented.

## 1. Mission and non-negotiable direction

Deliver a production path for cinematic interactive worlds in QualiaDB/Webizen, beginning with the stylized animated-film vertical slice and preserving a shared route to photorealistic rendering. Native and browser/WASM are first-class runtime targets. Devices with different APIs, features, bandwidth, memory and performance must select the strongest safe profile and degrade without corrupting semantics or losing the scene.

The engine owns canonical scene planning, typed materials, the `.10d` container, rendering semantics, quality policy and recovery. Native and WASM adapters may schedule different work and choose different quality tiers, but stable entity identity, 10D manifold meaning, material interpretation and scene/replay meaning must remain portable. Use VibeScript for policy or host logic where it can replace JavaScript without creating a second implementation of core semantics.

Preserve and advance the existing 10D manifold work. The Tensor10D coordinates `[q,v,w,x,y,z,t,α,μ,σ]` are identity-bearing manifold address/state structure interpreted under declared domain and coordinate conventions; they are not a digest and are not replaced by a colour point cloud or an EMF-only representation. Keep EMF and other scientific fields as typed parts within the versioned shared `.10d` container. Q42 stable entity IDs remain continuity handles; cryptographic hashes are integrity/index aids, never the identity itself.

The 42 MiB Prolog Sentinel bounds semantic execution, not graphics assets, GPU allocations, or model/texture residency. Graphics memory must have separately named, measured, adapter-aware budgets and bounded failure behavior. Never “solve” pressure by hiding graphics asset memory inside the semantic sentinel or by silently increasing the sentinel.

## 2. Collaboration and worktree rules

Before editing, inspect the exact base revision, dirty-worktree state, `AGENTS.md`, the parent register, target modules, feature flags, current tests, and any in-progress agent work. The main worktree may contain user changes. Do not reset, clean, stash, checkout over, or broadly format unrelated files. Keep all changes within the assigned lane and coordinate shared-file edits before making them.

Use isolated worktrees for agents that need the same repo unless the orchestrator says the shared checkout is safe and each lane owns disjoint paths. Each lane produces a small reviewable commit or patch and reports: changed paths, public API, invariants, tests added, known gaps, and exact commands actually run. Do not claim a test passed if it was only compiled or if the adapter/browser path did not execute.

Parallel agents should implement substantial independent slices before the integration verification milestone. Avoid a compile/test cycle after every small edit. The coordinator should batch coherent implementation, run formatting/static checks during development, then perform one comprehensive compile and focused test batch per integration milestone. If a long compile fails, collect all diagnostics from that run, repair them as a batch, and re-run the smallest valid verification set. Never launch overlapping Cargo builds against the same target directory.

The coordinator owns the parent register update. Agents provide evidence; they do not mark broader parent capabilities complete based on local success. Keep the register honest and retain open gates.

## 3. Engineering constraints

Apply repository `AGENTS.md` and these graphics-specific rules:

1. **Zero allocation in frame/hot paths.** Culling, selection, LOD choice, animation sampling, render submission, telemetry update, and caller-buffered readback do not allocate. Allocate or grow only during bounded cold construction / authoring. Preallocate reusable scratch; enforce a caller-visible capacity and fail closed or use an explicit quality fallback.
2. **Bound every resource.** Every asset decode, decompression, texture upload, render target, mesh/instance stream, pipeline cache and readback buffer has checked sizes and an explicit budget. Rejection leaves the previously admitted scene usable. Graphics allocations use graphics budget/accounting, separate from the semantic Sentinel.
3. **Preserve the stable ABI.** Do not repurpose `NQuin`/six-u64 semantic fields for transient graphics data. Version `.10d` section kinds, record layouts, shader buffer ABIs and WASM exports. Retain valid v1 reads while v2 is the canonical writer unless an approved migration says otherwise.
4. **Deterministic identity and order.** Keep semantic IDs through import, mesh/material splits, culling, compaction, LOD, selection, picking, save/load and replay. A transient draw slot is not a stable ID. Fixed ordering/reduction rules are required where output feeds a digest or replay.
5. **Portable fallbacks.** Every optional feature has an explicit capability gate and a lower-tier behavior that preserves essential content and meaning. Unknown capabilities select the conservative tier. Native capability does not imply browser availability, nor does a successful compile imply runtime support.
6. **Focused ownership.** New Rust implementation files should normally be under 500 lines. Keep cold compilation, runtime scheduling, backend implementation, receipts and tests separate. Avoid adding unrelated behavior to already oversized coordinator modules; decompose them in the same programme.
7. **VibeScript direction.** Move graphics selection/recovery policy and portable rules into VibeScript where its current type system and execution model fit. Do not replace mature native GPU/WASM API calls with script wrappers that only obscure the same implementation. Retire duplicate JavaScript policy only after generated/browser integration is verified.
8. **No false evidence.** Distinguish code present, target checked, test executed, adapter-backed, browser-backed and measured performance. Record GPU/API/driver, target triple, features, build identity, scene dimensions/counts and measurement method.

## 4. Current implementation lanes

The capability IDs map to the parent register `31-aaa-graphics-engine-gap-register.md`. Work in dependency order, but parallelize the independent lanes below. A lane may discover a prerequisite gap; report it and implement the smallest shared prerequisite only after coordinating ownership.

### Lane A — AG-08/AG-16: instances, visibility, LOD and semantic picking

**Goal:** make large repeated worlds cheap to submit while selection continues to identify the same semantic object after culling and LOD changes.

**Required work:**

- Complete the current instance ABI v2 route across native, WASM, Portal and Webizen adapters. Validate transform, precomputed inverse-transpose normal, reflection sign, semantic identity and alignment-independent packed input at admission.
- Keep stable source order and use camera-frustum selection with reusable caller buffers. Keep shadow visibility independent so off-camera casters remain represented. Avoid re-uploading an unchanged visibility set; key invalidation on camera, actor/model transform, mesh bounds, source instance revision and viewport aspect.
- Add LOD records to `.10d` or the versioned mesh manifest with measured geometric error, bounds, material/rig compatibility and transition policy. Use projected screen-space error, camera projection, resolution and a hysteresis band. Preserve source index and semantic ID. Define how morph/transition avoids visible popping; use a lower-cost deterministic fallback when transition support is unavailable.
- Wire stable semantic mesh picking into the same depth-tested visibility contract as tensor picking. Alpha-mask holes must not select; transparent selection needs a declared rule. Test occlusion among tensor and mesh geometry. Return the original 64-bit semantic ID, not a compacted draw index.
- Establish independently budgeted mesh batches. Respect WebGPU vertex-buffer, storage binding, texture and draw limits; do not assume a single maximum-size buffer works across devices.
- Keep CPU selection as a correctness oracle and degraded path. GPU culling/compaction/indirect draws are optional accelerators until feature, deterministic identity, fallback and measurement gates pass.

**Acceptance evidence:** stable identity after camera motion/cull/reorder/LOD transition; fully visible, fully hidden, boundary and invalid-bounds cases; alpha-mask pixel fixture; tensor-vs-mesh depth fixture; 10,000-instance stress; zero allocations in repeated hot-path selection; browser and native execution receipts; benchmark with instance count, visible ratio, CPU submit time, GPU time and upload bytes.

### Lane B — AG-19/AG-20: quality profiles, resource admission and recovery

**Goal:** pick a quality mode from observed capabilities and budgets; adapt under sustained load or resource refusal without corrupting the world.

**Required work:**

- Normalize native adapter/device facts and browser `GPUAdapter`/WebGL2 facts into one serializable capability snapshot. Record API, limits, optional features, texture formats, float renderability, timestamp support, compute/storage support and explicit unknowns. Request only features actually needed by the selected profile.
- Define deterministic **Conservative, Low, Balanced, High and Ultra** profiles with render scale, shadow map/cascade count, AO method/resolution/taps, bloom/HDR, MSAA/TAA or temporal upscaling, texture/streaming residency, crowd/vegetation/particle caps and mesh/instance limits. Keep disabled/unavailable distinct from unimplemented.
- Admit frame targets, texture residency, upload staging, geometry, readback and transient scratch against separate per-device budgets. Include format byte cost, mips, dimensions, samples, alignment, and reserved headroom. Admission failure downgrades one feature/tier deterministically and reports the reason.
- Add sustained frame-time/thermal/resource-pressure adaptation with hysteresis and minimum dwell; prevent rapid tier oscillation. Never reduce authoritative simulation rate, Tensor10D meaning, stable identity, or replay input to meet a graphics target.
- Recovery covers context/device loss, WebGPU adapter/device recreation, WebGL context loss, WASM memory pressure and partial content admission. Retain an explicit reload/rehydration manifest; release only owned resources; preserve authored state; recover to the best available renderer or report unavailable.
- Put portable quality/recovery decisions in VibeScript where practical; keep GPU operations in typed Rust/WASM adapters. Existing browser probe is an availability recommendation, not proof that the presentation canvas or renderer initialization succeeded.

**Acceptance evidence:** table-driven unknown/partial capabilities; deterministic fallback sequence; refusal at each reservation stage; no leaks across resize/device-loss/recovery; adaptation hysteresis under synthetic sustained loads; parity tests for native/WASM policy; telemetry counters with explicit units; browser execution in at least Chrome and one additional browser/adapter where available.

### Lane C — AG-02/03/04: materials, texture package and streaming

**Goal:** deliver efficient high-quality materials without stalling frame presentation or confusing semantic/manifold data with presentation payloads.

**Required work:**

- Close versioned MAT3 semantics: base colour, metallic/roughness, normal, occlusion, emissive, stylized ramp, six UV transforms/samplers and alpha coverage modes. Preserve vertex colour as a multiplier. Specify linear/sRGB interpretation, tangent handedness, alpha cutoff and material defaults.
- Implement production texture ingestion for external/data URI and embedded images, strict MIME/signature checks, KTX2/BasisU and supported image decoders, transcode selection from actual format support, dimension/decoded-byte ceilings and failure-safe rollback. Do not trust declared decompressed size without checked arithmetic and budgets.
- Add coarse-mip-first residency and streaming with per-texture request priority, cancellation, bounded staging and deterministic eviction (pinned/visible/near/semantic importance before distance/age). Use GPU block compression when a supported target exists; retain portable RGBA fallback.
- Preserve mip semantics: average in linear light for sRGB colour, keep data maps linear, renormalize normal vectors, and preserve alpha-test coverage for the authored cutoff. Qualify sampler states and UV transforms by pixel readback.
- Keep asset decode and HMC/package verification off the render hot path. Make artifact handoff from local Qualia pipeline explicit; source bytes, package digests and decoded/resident bytes are separate receipt fields.

**Acceptance evidence:** MAT1/MAT2 read compatibility; MAT3 corruption/version/digest tests; KTX2 capability routing plus portable fallback; rollback on each failure point; checkerboard/alpha/normal/metal-rough pixel fixtures; streaming under constrained residency; measurable upload stalls and GPU memory; native and browser executions.

### Lane D — AG-05/06/07: cinematic lighting and output

**Goal:** build a coherent image pipeline that supports authored stylized output first and photorealistic output on the same scene/material contract.

**Required work:**

- Build lighting around physically meaningful units and explicit color spaces. Keep stylized ramps/bands, contour/rim controls and authored art direction as a first-class renderer mode sharing geometry, materials, lights, identity and exposure.
- Extend the current direct GGX baseline with image-based lighting/environment probes, prefiltered specular, diffuse irradiance and multiple light types. Define shadow filtering and limits; use cascaded directional maps for sun, bounded atlas/array policies for spots/points, bias controls, and a documented contact/soft-shadow tier. Avoid unbounded per-fragment loops.
- Evaluate shadow filtering based on measured quality/perf; comparison PCF remains the fallback. Add temporal denoising only with stable history rejection/reprojection and a quality fallback.
- Evolve current AO into explicit quality modes; compare GTAO/CACAO-like bounded horizon approaches and optional denoising. Keep AO on indirect ambient only unless the lighting model explicitly says otherwise. Use depth/normal-aware upsample and skip pass when disabled/unavailable.
- Add a frame graph/resource dependency model for scene color, linear depth, normals, velocity/history, shadow, AO, bloom, exposure and output transforms. Allocate pass targets from the capability/budget profile and preserve the established no-AO/no-HDR path.
- Define exposure, view transform, display encoding and HDR output contracts. SDR fallback remains reliable; do not silently call tone-mapped SDR “HDR”. Validate wide gamut/display metadata separately when output surfaces support it.

**Acceptance evidence:** reference scenes for stylized and physically based materials; shadow/AO pixel oracles; exposure ramps/highlight preservation; resize/failure fallback; GPU captures or timestamps where supported; performance and memory receipts per profile.

### Lane E — AG-09/10/12/13/15: world scale and motion

**Goal:** extend the current retained mesh path into large, animated, reactive worlds.

**Required work:**

- Terrain: multiresolution patches/quadtree or clipmap with source-aligned elevation/precision; screen-space error selection; crack stitching/skirt fallback; bounded async tile streaming and deterministic edge ownership. Preserve geographic coordinate references and source provenance.
- Water: depth/shore intersection, stable normals/reflections, foam/shoreline response, transparency ordering and lower-tier opaque/wave fallback. Water must not leak through depth or overwrite authoritative terrain/entity meaning.
- Rigs/clips: versioned skeleton and animation records compatible with `.10d`; bounded joint counts, deterministic clip time, skinning palette residency, normal/tangent updates, animation LOD and CPU fallback. Avoid per-frame allocation.
- Vegetation/effects: deterministic wind fields in the declared manifold/world frame; GPU instancing; bounded hierarchical culling; pooled event-driven particles with fixed capacity, stable random seeds and graceful drop/degrade rules.
- Character/botanical families: reusable typed assets and semantic anchors, with profile-specific LOD and animation. Storybook/Community Grounds stylized-first content remains the first approval baseline; photoreal content is a later profile, not a parallel semantic format.

**Acceptance evidence:** terrain seam/precision stress, water shore/depth pixels, rig/clip round trips and deterministic playback, vegetation/particle capacity overflow behavior, representative device budgets, and integrated game scenes.

### Lane F — AG-14/17/18: interchange, style and interaction

**Goal:** finish the semantic-to-visual authoring loop and keep identity readable to humans.

**Required work:**

- Specify `.10d` v2 structured manifold identity/profile/coordinate semantics and typed parts in the shared file format. Keep one envelope with independently versioned sections. The universal Tensor10D manifold coordinates and typed scientific/EMF field sections remain explicit. Define canonical identity resolution and how edits/replay preserve identity; digest remains optional integrity/index support.
- Complete import/export for the supported GLB subset with deterministic manifests, dependency hashes, material/normal/UV/rig/LOD retention, explicit unsupported-feature reports and round-trip rules.
- Add style profiles for Storybook, Earthlight, Community Grounds and a physically based mode. Profiles are data, not shader forks or separate world formats. Include lighting, palette, material response, atmosphere, animation cadence and accessibility contrast controls.
- Implement semantic selection outlines/highlights, hover/focus state, decals/labels and feedback events across WebGPU, WebGL2 and low-cost fallback. Picking must use depth and alpha semantics, return stable identity, and never make selection depend on draw order.
- Integrate VibeScript for portable authoring/presentation policies; replace JavaScript only where policy logic is actually duplicated. Keep UI framework code as a host concern where appropriate.

**Acceptance evidence:** `.10d` v1/v2 round trips, malformed section and cross-version tests, manifold identity not hash demonstration in fixtures, import/export dependency replay, style golden captures, semantic pick before/after LOD/streaming, and keyboard/accessibility states.

## 5. Parallel dispatch plan

The coordinator should dispatch one agent or small team to each lane with exact path ownership, upstream revision and acceptance tests. Use the following integration order:

1. **Foundation:** quality profiles/admission + package/identity ABI audits; finish current instance visibility, semantic picking and screen-space LOD selector.
2. **Content path:** MAT3/texture ingestion and `.10d` v2 typed identity/field sections. These lanes must agree on stable identity and manifests before game integration.
3. **Image path:** frame graph, shadows/AO, environment lighting, exposure/output, stylized profile. Coordinate shader ABI before edits; keep fallback pipeline executable.
4. **World path:** terrain/water/rig/vegetation/particles; depend on resource profiles, LOD and streaming.
5. **Game integration:** load an actual game-demo scene via the supported package path; run native and browser smoke scenes; produce captures and cost receipts.

Each dispatched prompt must name its exclusive files, shared API dependencies, allowed VibeScript changes, prohibited files, and exact output/report format. For unavoidable shared-file edits, one integration owner makes the edit after lane proposals are reviewed.

## 6. Verification cadence and gates

Do not spend an hour recompiling after every small implementation. Use a layered cadence:

- **During implementation:** `rustfmt` on scoped Rust files; `git diff --check`; inspect file/feature boundaries; use fast parser/unit harnesses only if they are truly cheap and do not duplicate the central test.
- **After a coherent lane batch:** one shared compile for the exact target/profile(s), followed by all newly added focused tests for the lane and relevant existing regressions. Run MSVC on Windows for native work when installed; run `wasm32-unknown-unknown` with supported no-default-features/WASM feature sets for browser code.
- **At each integration milestone:** compile the core renderer and `webizen-render`; compile WASM exports; execute focused native tests; run shader validation (Naga or actual pipeline creation); run adapter-backed pixel tests when the capability exists. Collect all errors from the batch and repair them together.
- **For release qualification:** browser GPU behavior, native GPU backend coverage, actual game scene, visual baselines, stress/memory, sustained performance, recovery and package provenance. Keep a named non-GPU fallback when CI lacks an adapter.

An ordinary compile only proves type/feature compatibility. A `--no-run` test build only proves test compilation. A Naga parse is not adapter pipeline creation. A successful initialization is not visual correctness. A single pixel fixture is not performance evidence. Label each receipt accurately.

Performance comparisons must include machine/adapter, driver, runtime/browser, build profile, feature set, scene package digest, resolution, frame count, warmup, percentile method, and memory methodology. Report at least median and p95 frame time, CPU submission time, GPU pass time when measurable, bytes uploaded, resident/transient VRAM, visible/total instances, triangles and draw calls. Do not optimize by changing simulation semantics or deleting visible content without an explicit quality-policy decision.

## 7. Parent tracker update protocol

After the work batch, update `C:/github/game-demo/docs/planning/31-aaa-graphics-engine-gap-register.md`:

1. Add or refine the evidence under the matching AG row. State exactly what changed, files/APIs, runtime target, commands, execution status, measured device/backend and test counts.
2. Change a status to **Verified** only when every acceptance gate declared for that capability has evidence. Otherwise retain **In progress** and list the next concrete gate.
3. Preserve earlier historical entries; add a dated follow-up that clearly supersedes stale claims rather than silently erasing history.
4. Update cross-capability dependency notes, open risks, and the implementation tracker. Keep AG-08, AG-14, AG-19, AG-20 progress consistent across the table and detailed sections.
5. Correct stale statements (such as an old SDK/toolchain barrier) only after current local evidence supports the correction. Record source/build/package identities; dirty worktrees do not provide reproducible pins.
6. Add links to research at the claim they justify and separate externally published guidance from Qualia measurements.

The work-order owner should return a concise completion report with implemented lane IDs, exact register section updated, paths changed, validation results, remaining blockers and actual evidence limitations.

## 8. Authoritative implementation references

These primary or standard-setting references inform proposed techniques; they do not prove the implementation is optimal for Qualia workloads. Benchmark alternatives on actual game scenes and target hardware before choosing a default.

- [WebGPU specification and API reference](https://gpuweb.github.io/gpuweb/) — portability, features, limits, render/compute and indirect draws. Query runtime limits; the spec's baseline values do not describe every adapter.
- [Khronos glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) — asset/material/texture/sampler interchange and compatibility rules.
- [KTX 2.0 specification](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html) — mip streaming, Basis Universal and supercompression container behavior; validate supported payloads and size bounds.
- [meshoptimizer](https://meshoptimizer.org/) — cache/vertex/index optimization, simplification and meshlet/cluster LOD options; compare attribute error and deformation needs.
- [AMD FidelityFX CACAO](https://gpuopen.com/fidelityfx-cacao/) — production AO reference and quality/performance tradeoffs; retain a portable Qualia-owned fallback and compare on multiple adapters.
- [W3C WebGPU API](https://www.w3.org/TR/webgpu/) — normative browser GPU behavior. Device features and limits must be requested/checked, never inferred from a browser name.

Additional references may be added by lanes, prioritizing specifications, vendor implementation papers/docs and primary algorithm papers. A benchmark claim must cite the exact tested build and workload rather than borrowing a vendor headline.

### 8.1 Research-informed optimization requirements

The following techniques are requirements to evaluate and implement where their capability gates and workload measurements justify them. They are not permission to claim a vendor's optimization is already optimal for QualiaDB.

**Geometry and draw submission**

- Run cache/overdraw/fetch reordering, simplification, quantization and meshlet generation during bounded asset cooking, not per-frame. Keep this Tier-2 construction work outside renderer hot paths; preserve source-to-cooked primitive and semantic-identity maps.
- Partition large static meshes into bounded clusters with precomputed conservative bounds. Select clusters hierarchically and reject by frustum first; add normal-cone/back-face and occlusion rejection only when conservative bounds and temporal visibility rules prevent missing visible geometry. Keep a non-meshlet indexed-draw representation for adapters where cluster/compute/indirect support is absent or slower.
- Treat meshlet vertex/triangle limits as profile data selected for the target API/adapter, not universal constants. Budget worst-case builder scratch using documented bounds before allocation, trim cooked output to actual use, and record the selected cluster limits in the asset receipt.
- Use CPU culling as the initial portable correctness path. GPU compute compaction and indirect submission are optional after exact feature/limit checks, stable identity mapping, shader validation, and measured wins. Do not assume `indirect-first-instance`, subgroups, timestamp queries, or any compression format is universally available in WebGPU.
- Use projected geometric error with camera projection and output resolution for LOD; add a hysteresis interval and deterministic transition policy. Attribute-aware simplification must protect UV/material seams, hard normals, skinning/deformation constraints, and stylized silhouette features. Validate LOD error on final shading/animation fixtures as well as position-only metrics.

**Textures and residency**

- Author a platform-neutral KTX2/Basis payload when it reduces distribution cost, then select a supported GPU block-compressed transcode target from the live adapter feature set. Reject unsupported supercompression or payloads safely; keep a bounded RGBA fallback where the format budget permits.
- Request and upload the smallest useful mip levels first so the scene becomes presentable before full detail arrives. Track source bytes, transcode scratch, staging bytes, resident GPU bytes and pinned bytes separately. Stream later mips through bounded staging and cancel obsolete requests; use deterministic priority and eviction so camera movement cannot create allocator spikes or nondeterministic residency.
- Compute all dimensions, mip/block counts, alignments and decompressed lengths with checked arithmetic before reserving memory. Admission must include row/placement alignment and format block dimensions; never trust package-declared decoded sizes by themselves.
- Generate/filter mip chains according to semantic: average sRGB color in linear light, keep scalar/data maps linear, renormalize normal maps, and retain alpha-test coverage at the material cutoff. Use pixel fixtures to compare against the declared reference, since a fast but semantically wrong mip chain produces visible quality regressions.

**Lighting, AO, and adaptive quality**

- Keep direct lighting bounded by a small authored light set or tiled/clustered light lists; precompute environment irradiance and roughness-prefiltered specular maps during authoring where possible. Preserve a deterministic direct-light fallback if IBL resources or formats fail admission.
- Compare full-resolution AO with reduced-resolution generation plus depth/normal-aware bilateral upsampling. Use fixed quality presets first; only use sample-adaptive AO when its extra importance-map passes are cheaper on the target adapter. Retain depth-aware blur and tune radius/fade separately from quality level to avoid halos and contact darkening.
- Resolve dynamic resolution and tier changes from sustained frame-time/thermal pressure with hysteresis and minimum dwell, not single-frame spikes. Keep the adjustment order explicit (for example: render scale, AO resolution/taps, shadow distance/resolution, effect disablement, then residency/LOD caps) and record each decision. Simulation, semantic identity and replay input are outside this adaptation loop.
- Timestamp queries are an optional measurement source. If unavailable or disallowed, use CPU frame/submit timing and report that GPU timings are unavailable; do not infer GPU cost from browser name or adapter vendor.

**Evidence gates for adopting an optimization**

Each optimization selected as the default path must include a baseline and candidate measurement from the same representative scene/package, camera path, resolution, warmup and build profile. Report CPU construction/submission time, p50/p95 frame time, GPU pass time only when measured, bytes uploaded, resident/transient memory, draw/cluster/triangle counts, visual error, and the fallback that ran. Include both native and browser evidence before making a cross-target default claim. Otherwise ship it as an explicitly gated experiment or keep the established fallback.

Primary research references for these requirements:

- [meshoptimizer README and meshlet guidance](https://github.com/zeux/meshoptimizer/blob/master/README.md) — offline cache/vertex optimization, simplification and cluster bounds; hardware sizing examples are starting points and require Qualia benchmarking.
- [meshoptimizer meshlet API](https://github.com/zeux/meshoptimizer/blob/master/src/meshoptimizer.h) — builder scratch bounds, supported per-meshlet limits and conservative cluster bounds.
- [WebGPU specification](https://gpuweb.github.io/gpuweb/) and [supported limits reference](https://gpuweb.github.io/types/interfaces/GPUSupportedLimits.html) — required capability/limit queries and validation boundaries.
- [KTX 2.0 specification](https://registry.khronos.org/KTX/specs/2.0/ktxspec.v2.html) — independently stored mip levels, coarse-mip-first streaming rationale, block/alignment rules, and graceful rejection of unknown formats/supercompression.
- [glTF KHR_texture_basisu extension](https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_texture_basisu) — portable Basis Universal texture payloads and runtime transcode intent.
- [FidelityFX CACAO technique documentation](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/combined-adaptive-compute-ambient-occlusion/) and [CACAO settings](https://gpuopen.com/manuals/fidelityfx_sdk/reference_documentation/structs/ffx_cacao_settings/) — quality tiers, reduced-resolution AO with bilateral upsampling, adaptive sample budgeting and edge-aware blur as comparative reference techniques.

## 9. Completion definition

This work order is complete only when assigned lanes have shipped reviewable implementation, compile and focused tests are green for the named targets, the parent register has accurate progress/evidence and remaining gates, and the game integration path has concrete next steps. The overall AAA programme is **not complete** merely because this work order is completed; capability statuses remain governed by their own acceptance criteria.

## 10. Current execution checkpoint

Checkpoint recorded **2026-10-08** from the last successful workspace session; re-inspect the current checkout before relying on it.

- **AG-08:** A camera-driven instance visibility submission path was added to the WebGPU frame sequence, keeping full source instances for shadow casting and compacting camera-visible instances for forward/AO work. A tagged R32Uint mesh-picking path was added; the returned visible slot resolves through a retained per-pick snapshot to the full 64-bit semantic identity. Alpha-mask cutoff is mirrored in the pick fragment, and the Portal API exposes selected semantic identity while preserving tensor-index picking.
- **AG-08/AG-16:** A caller-buffered projected-screen-error LOD selector with hysteresis, fail-open bounds behavior, and source/semantic identity retention was added in the LOD module.
- **AG-19:** A normalized native/WASM capability snapshot and deterministic Conservative/Low/Balanced/High/Ultra budget profile selector was added. Its proposed texture-residency budget is separate from the 42 MiB semantic Sentinel.
- **Build fix:** The empty mesh-instance upload path received a local identity transform after the prior test-target compile exposed an out-of-scope constant.
- **Verification:** These combined changes have not passed a compile or test run. The previous shell command runner failed before process creation after a machine reboot; Cargo process status and current file state must be checked first. Do not record these slices as verified until the batched MSVC check, focused tests, and applicable shader validation complete.
- **Parent register:** The external game-demo register has not yet been updated with this checkpoint. Update it only after the checkout is re-inspected and verification evidence is available; report any inaccessible external path separately.

### 10-A. Active implementation and verification update (2026-10-08)

- `render/quality_runtime.rs` now contains a fixed-memory presentation-quality adaptation policy with sustained-load/recovery hysteresis, minimum dwell, conservative unknown/critical fallback, tier clamps, counters, and bounded receipts. Six focused tests are present. It has been formatted and manually reviewed; module registration and Cargo execution remain pending.
- `render/texture_stream_policy.rs` now contains a caller-buffered deterministic mip-residency planner with checked block-compressed byte calculations, coarse-mip-first requests, priority ordering, deterministic unrequested eviction, and all-or-nothing output. Seven focused tests are present. Its author ran formatting and `git diff --check`; module registration and Cargo execution remain pending.
- A standalone `.10d` v2 identity/typed-field codec and tests are being drafted in new `container_10d` files. The author reports fixed Tensor10D coordinate values with explicit profile/domain/convention IDs, stable entity identity distinct from an optional digest, and extensible typed fields including EMF. Review and integration have not finished; treat this as in progress.
- The first consolidated MSVC renderer test invocation compiled successfully and ran 460 tests, with 429 passing and 31 failing. Its dominant cascade was a WGSL mesh binding at group 7 against the device's four-group limit, followed by a 48-byte bloom uniform declared as a 32-byte minimum; isolated failures included material binding-manifest drift, stale target-size expectations, and PNG limit classification. These findings were repaired and the full renderer suite passed in checkpoint 10-B below.
- The `.10d` v2 manifest, outer section type 15, v1-header rejection, bounded container integration and reference-range validation have now been wired locally. Runtime quality adaptation and texture residency planner modules are registered. Their new tests were not included in the completed test invocation because registration happened after it started. The quality policy now owns its current tier, preserves a monotonic-clock high-water mark after regressions, and starts recovery dwell only after a valid clock returns. Its policy is intentionally fast-down/slow-up; critical pressure immediately requests Conservative.
- Portable initial render-tier selection now executes the same shared `graphics_backend.vibe` policy on native and WASM; Rust remains responsible for converting the selected tier into checked adapter-specific budgets. VibeScript load/evaluation is confined to the cold profile-construction path, with conservative fallback on script failure.
- A separate swarm lane is implementing the four-group WebGPU layout across mesh, shadow, AO-prepass, picking and material draw paths. It has identified a 0–3 mapping and is editing owned pipeline/layout files; no Cargo run is active. The coordinator corrected the bloom uniform minimum and frame-target expected byte totals. All new sections and fixes remain unverified until the next combined MSVC run.
- Research-informed implementation requirements were added to §8.1, with primary references for meshoptimizer clusters, WebGPU capability limits, KTX2 streaming/alignment and FidelityFX CACAO quality paths. These references guide engineering choices; they do not constitute performance evidence for QualiaDB.

### 10-B. Consolidated implementation and validation checkpoint (2026-10-08)

- The renderer portability migration now places all mesh, shadow, AO-prepass and semantic-pick shader resources within WebGPU bind groups 0–3. Material factors and six texture/sampler slots share one group. WGSL contract tests enforce the portable group range; no renderer shader or draw path references group 4 or above.
- Native pick readback reserves R32Uint zero as the portable clear/no-hit value and writes tensor indices with a +1 bias. This removes the ambiguity between a miss and valid tensor node zero; mesh IDs remain high-bit tagged. The GPU/CPU differential now additionally checks that index zero remains pickable.
- The material pixel contracts now compare against the exact linear clear input and the final sRGB output encoding. PNG configured-limit failures are mapped to `DecodedImageExceedsLimit` instead of being collapsed into `InvalidImage`. Texture-stream test fixtures now obey their mip-count and residency-budget contracts. Bloom and fixed-buffer byte expectations were reconciled with the actual resident uniform/cascade sizes.
- `cargo fmt --all` and `git diff --check` completed. MSVC command: `cargo test --lib --offline --target x86_64-pc-windows-msvc -p qualia-core-db render:: -- --test-threads=1`; result: **476 passed, 0 failed**, finished in 20.21 seconds after a 5m22s compile. This includes native adapter-backed pixel tests in this environment, renderer shader contracts, PNG bounds, quality runtime, mip planning, and the VibeScript policy host tests. It does not qualify browser GPU execution or performance.
- The browser-target `cargo check --offline --target wasm32-unknown-unknown -p webizen-render --no-default-features --features qualia` completed successfully in 4m49s. It emitted an unused-import warning in the `.10d` envelope; those test-only imports have since been moved under `#[cfg(test)]`. Focused MSVC command `cargo test --lib --offline --target x86_64-pc-windows-msvc -p qualia-core-db container_10d -- --test-threads=1` then passed **152/152**, including all five manifold identity v2 codec tests, both container-envelope tests and the v2 outer-header gate. The complete browser check was before that import-only cleanup; rerun it only if subsequent WASM-visible code changes.
- Parent register `C:/github/game-demo/docs/planning/31-aaa-graphics-engine-gap-register.md` remains separate from this work-order update. Its capability rows and evidence should be updated after the WASM check and the focused 10D test result are known. Remaining programme gates include browser WebGPU execution, full asset transcode/stream integration, measured performance profiles, and the larger game vertical slice.

### 10-C. Identity compiler adapter and expanded validation (2026-10-08)

- Hardened MID2 field encoding so built-in kinds are accepted by default and extension kinds require the same explicit allow-list at encode and decode boundaries. `StableEntityId` is explicitly an opaque globally unique continuity handle supplied by a collision-checking identity authority; the codec never derives identity from a content digest. Coordinates normalize signed zero on encode and reject non-canonical negative zero on decode. Generic envelope checks validate section references and byte ranges; owning schemas remain responsible for kind-specific payload interpretation.
- Added an opt-in, caller-buffered mesh compiler adapter that attaches MID2 to an already compiled `.10d` container. It verifies the input header and whole-file CRC, preserves all existing section bytes/metadata, validates extension kinds and field references, rejects duplicate identity sections, and seals the output CRC. Existing legacy mesh compilation remains unchanged; a convenience cold-path API returns an owned `Vec`.
- Parallel review confirmed the implementation boundary and identified the next renderer integration seam. `load_hmc_asset` currently resolves/decodes full PNG/JPEG images and uploads a full generated mip chain; its pure mip planner is not yet connected. Safe initial work can admit a deterministic priority prefix and allocate a smaller coarse-base texture. Frame-time refinement/eviction still needs bind-group rebuild or stable indirection because material bind groups retain views. KTX2/Basis transcoding and persistent source-level support remain open.
- Validation: `cargo fmt --all` and `git diff --check` passed. Full MSVC library test binary compiled; the full suite began 9,241 tests but did not complete cleanly: paged CUDA attention panics because this machine has no CUDA driver library (`cuda.dll`/`nvcuda.dll`), and `inference::ternary_gpu::tests::ternary_ffn_resident_matches_cpu_oracle` terminated with Windows `STATUS_ACCESS_VIOLATION`. The CUDA failure was reproduced in isolation; these are outside the renderer and MID2 changes. Focused prebuilt-binary runs passed **153/153 `.10d` tests** and **478/478 renderer tests**, including native GPU pixel and picker checks. The updated browser target check `cargo check --offline --target wasm32-unknown-unknown -p webizen-render --no-default-features --features qualia` succeeded (2m02s) with four pre-existing warnings.
- Remaining work: implement coarse-first/partial-budget texture admission and HMC integration; retain/refine source mip ownership and safe material rebinding; add actual KTX2/Basis decode/transcode; execute browser WebGPU tests; measure device-specific budgets/performance; complete the stylized-first plus photoreal material/lighting vertical slice. The overall AAA programme remains in progress.

### 10-D. Coarse-first HMC texture admission (2026-10-08)

- Added an allocation-free image metadata preflight API so scene loaders can validate source dimensions and derive mip counts before decoding pixels.
- Extended the deterministic mip planner with a best-effort API that admits one global priority prefix under independent incremental residency and upload-byte budgets. It reports admitted/deferred mip counts and byte totals, preserves output on errors, and retains the previous all-or-nothing planner for callers that need it.
- Added caller-buffered CPU single-level RGBA8 reduction for linear-light sRGB colour, linear data, and renormalized normal maps, including odd dimensions and preflighted output capacity. Added GPU initial coarse-base upload and precomputed mip-window upload paths that physically allocate only the selected mip suffix and reserve the corresponding resident/upload bytes. Existing resident texture identities remain cache hits; the API does not claim to refine or replace an already bound resource.
- Integrated the planner with native `VolumetricRenderer::load_hmc_asset_with_texture_budget`. The loader plans from metadata, decodes only selected images, CPU-reduces them to the admitted first mip, and uses GPU-generated remaining levels. Deferred maps bind typed neutral textures; the returned `HmcTextureAdmissionReport` exposes requested/resident/deferred interpretations and mip/byte totals. The legacy loader remains available and requests the full mip chain. Alpha-mask textures are admitted only when mip zero fits because coarse CPU alpha coverage correction is not yet available. Frame-time refinement/eviction remains deferred until material bind groups can be safely rebuilt or use stable indirection.
- File-size guidance: moved the existing volumetric test module to `webizen-render/src/volumetric_tests.rs`; the production `volumetric.rs` is now 1,090 lines.
- Validation: MSVC `qualia-core-db` renderer tests passed **491/491** and `.10d` tests passed **153/153** on the latest test binary. Native `webizen-render --features qualia --lib` passed **63/63**, including the coarse-level CPU tests. Browser `cargo check --offline --target wasm32-unknown-unknown -p webizen-render --no-default-features --features qualia` passed in 1m02s. Formatting and `git diff --check` passed. These checks do not include a full browser GPU pixel run, HMC end-to-end asset fixture, alpha-mask coarse degradation, texture refinement, KTX2/Basis transcoding, visual-quality comparison, or performance receipts.
- The previously recorded whole-library CUDA absence and separate ternary-GPU access violation remain unresolved environment/broader-suite issues; the renderer, `.10d`, and Webizen suites above pass independently.

### 10-E. Alpha-mask coarse mips and KTX2 inspection (2026-10-08)

- Added `downsample_alpha_mask_level_into`, a caller-buffered alpha-mask mip primitive that derives its target coverage from the authored base image, area-filters RGB in linear light, filters alpha linearly, and deterministically corrects/quantizes coverage. It handles odd dimensions and preserves the destination on validation/capacity errors. The existing generic reducer continues to reject alpha-mask mode so callers must select the explicit coverage-aware API.
- Wired the HMC coarse-mip path to apply coverage correction against the original decoded base at every generated level. Replaced the previous alpha-mask deferral fixture with a coarse mixed-coverage assertion. This improves degradation quality for budget-constrained mask textures; GPU upload and live HMC admission still require the batched test gate below.
- Added an allocation-free `render::texture_ktx2::Ktx2Document` parser that validates the KTX2 header, level index, section ranges/order, DFD total-size and block framing, standard supercompression parameters, and level bounds/alignment/padding, then exposes borrowed metadata and level bytes. It supports structural inspection of standard scheme IDs 0–3 and rejects registered vendor schemes pending per-scheme validators. It does not decode DFD format/sample semantics, enforce the compressed-format `levelCount=0` rule, inflate, Basis-transcode, choose a GPU format, or upload. This is a container inspection layer, not KTX2 texture support.
- Split `cpu_texture_mips` tests into a separate test module to keep the implementation file under the 500-line library guidance. `volumetric.rs` is near the 1,200-line ownership escalation threshold; avoid unrelated growth there and split a distinct texture-loading lifecycle if further behavior is added.
- The texture planner's independent review found deterministic ordering, error atomicity, and consistent deferred accounting. Integration caveats: its resident budget is only as accurate as caller-supplied physical `resident_bytes`; eviction currently drops whole unrequested textures and cannot downgrade a requested texture's fine mip residency.
- Validation: after correcting the alpha test-discovered double application of the GPU rounding bias, MSVC `cargo test --lib --offline --target x86_64-pc-windows-msvc -p qualia-core-db render:: -- --test-threads=1` passed **505/505**; the suite includes the new KTX2 structural tests and caller-buffered alpha tests. Native `cargo test --lib --offline --target x86_64-pc-windows-msvc -p webizen-render --features qualia -- --test-threads=1` passed **63/63**, including the HMC coarse alpha coverage test. Browser `cargo check --offline --target wasm32-unknown-unknown -p webizen-render --no-default-features --features qualia` passed in 1m12s. `cargo fmt --all` and `git diff --check` passed. An earlier all-target Cargo invocation compiled the library test binary but failed while parallel integration targets memory-mapped its large `.rlib` (Windows paging-file error 1455); rerunning the scoped `--lib` commands passed.

### 10-F. RGBA8 KTX2 base-level path and HMC graceful degradation (2026-10-08)

- Added `render::texture_ktx2_rgba8::inspect_ktx2_rgba8_base_level` as an allocation-free format/size preflight over the borrowed `Ktx2Document`. It accepts only standard `VK_FORMAT_R8G8B8A8_UNORM` (37) and `VK_FORMAT_R8G8B8A8_SRGB` (43), 2D non-array single-face textures, `typeSize=1`, and no supercompression; it verifies checked `width × height × 4` size against level-zero encoded and declared uncompressed lengths. The returned metadata reports dimensions, required output bytes, and colour interpretation. `decode_ktx2_rgba8_base_level` calls the inspector, verifies caller capacity, then copies exact level-zero bytes; no output is modified on validation/capacity errors and the decoder allocates nothing.
- Routed HMC texture MIME `image/ktx2` (case-insensitive and tolerant of surrounding whitespace and MIME parameters) through the same inspector/decode path. Inspection supplies bounded dimensions before pixel output allocation. Other unsupported image types/formats remain typed decode errors.
- Updated native HMC asset loading to treat a digest-verified resource's unsupported image codec/format or decode refusal as a per-resource deferred interpretation: the mesh and other textures continue, and the affected material map uses its typed neutral fallback. Package/resource resolution and digest verification failures remain fatal. A dedicated end-to-end HMC test for unsupported KTX2 fallback has not been added; current evidence is the decoder/preflight fixture tests plus code-path review and the existing renderer suite.
- Exact KTX2 limits: the existing parser performs structural header, section, level-index, bounds, alignment, padding, and DFD-envelope framing checks. The RGBA8 path only copies the uncompressed base level for vkFormat 37/43. It does not validate DFD sample semantics, decode remaining mip levels, upload GPU block-compressed data, transcode Basis, or support BasisLZ, Zstd, ZLIB, other compression, or other `vkFormat` values. Those require separate bounded format/transcoder and device-capability work; do not describe this as general KTX2/Basis support.
- Focused decoder tests cover UNORM/SRGB, inspector-only byte requirements, MIME-parameter routing, unsupported format/container modes, exact-length rejection, and caller-buffer atomicity. Latest batched validation already reported by the coordinator: MSVC core renderer tests **510/510**, native `webizen-render` **63/63**, and the WASM target check passed. Formatting and `git diff --check` were run for this documentation update; no Cargo tests were rerun for this checkpoint. There is no HMC end-to-end fallback test or browser GPU/pixel validation in this evidence.
