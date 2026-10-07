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
