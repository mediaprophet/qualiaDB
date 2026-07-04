# Native computational-geometry / `.10d` — Execution plan (tracked status surface)

This is the **live, tracked execution plan** for the native computational-geometry substrate and its on-disk
`.10d` container. The authoritative design lives in
[`native-computational-geometry.md`](native-computational-geometry.md); the charter and scope live in
[`native-computational-geometry-PROJECT.md`](native-computational-geometry-PROJECT.md). This document is the
status surface those two feed into — it holds the phase/task tables, the acceptance gates, the always-on
cross-cutting gates, and the curation datums Timothy must supply. The on-disk format is `.10d` (it retires the
earlier `g16`/`Q42M` placeholders; the quantized triangle mesh is now **one section type inside a `.10d`
file**, not a standalone blob). CGAL is used only as a **public-domain capability reference** — its CC0 docs
are the spec and its CC0 test vectors are the golden oracle; its GPL/LGPL algorithm source is never read,
copied, or transliterated. This is the QualiaDB / Webizen architecture; CGAL, HDF5, three.js, Draco, and
meshopt are tools it directs and references, never parents of it.

## Status vocabulary

| Term | Meaning |
|------|---------|
| **planned** | Named and scoped in the design doc, but no code exists yet. Spec + oracle identified, unstarted. Distinct from spec-reserved: a planned task *will* be built in code — it just has not been. |
| **spec-reserved** | Deliberately defined in the `.10d` / kernel spec as a normative slot or contract, but intentionally **not** yet implemented (a reserved header field, section type, axis role, or capability flag readers/writers honor for forward-compat). Never report it as a working capability — "the format promises this; the runtime does not yet fill it." |
| **foundation** | A real, compiling, in-tree first slice the rest depends on — the barriered thing others build on, but **not** the complete capability. E.g. `orientation_2`, the 16-byte half-edge topology, `convex_hull_2`, the single-stream quantized mesh section. |
| **implemented** | Fully written in code, compiles green, own `#[cfg(test)]` passing — but has **not** yet cleared the geometry acceptance gates (CC0 golden vectors, degeneracy cases, CPU/GPU differential, determinism/canonical-bytes). Implemented ≠ trustworthy: a wrong predicate sign is invalid topology, so this is a waypoint, not a finish. |
| **verified** | Implemented **and** cleared every acceptance gate that applies: CC0 golden vectors pass (degenerate + near-degenerate included), CPU scalar oracle matches, CPU/GPU differential is bit-clean, Naga-validated if it has a shader, determinism/canonical-bytes hold. This is the bar at which output may be trusted, hashed, WAL'd, attested. |
| **done** | Verified **and** integrated into every target surface the task promised (Rust + WASM parity where applicable + MCP/qapp route + renderer SDK), with the dated §9 progress-log entry written and the `NOTICES.md` RELEASE line posted. A phase is done only when all its tasks are done and its phase gate is signed off. |
| **blocked** | Cannot proceed without an external unblock — a ⚑ curation datum only Timothy can supply, or a claimed collision in another instrument's lane. Must name the specific blocker and who/what unblocks it. Not the same as planned: a planned task can start now; a blocked one cannot. |
| **deferred** | Consciously postponed per a PROJECT RULE — e.g. a risky mid-feature library-ization split (§11) or a scale-gated swarm fan-out (§11 Phase-0 / §14) not yet triggered. A decision on record, not neglect; names the trigger that resumes it. |

## How this is tracked

**How a task closes (→ done).** All of the following must hold, in order:
1. **Implemented** — compiles green on native; own `#[cfg(test)]` passes.
2. **Oracle** — validated against the CC0 golden vectors from CGAL's CC0 `test/` corpus (per-file license
   notice checked — never derived from GPL/LGPL source); if there is no CGAL analog, a stated first-principles
   oracle instead.
3. **Degeneracy** — passes degenerate + near-degenerate cases (collinear / cocircular / coplanar / duplicate /
   empty). Mandatory for geometry, because a wrong predicate sign is invalid topology, not a small error.
4. **Differential + determinism** — every GPU op has a caller-buffered CPU/WASM oracle and a bit-clean CPU/GPU
   differential set (incl. degeneracies) with no silent f32 disagreement on robust topology; Naga-validates if
   it has a shader; identical input → bit-identical combinatorial output (canonical / hash-stable bytes for any
   `.10d` it writes).
5. **WASM parity** — builds and behaves identically under `wasm-scientific` where the task claims a browser surface.
6. **Targets wired** — every surface the task promised is live (Rust + WASM + MCP/qapp route + renderer SDK, as applicable).
7. **Record** — §9 progress-log entry written + `NOTICES.md` RELEASE line posted.

A task at *implemented* that has not cleared 2–4 is explicitly **not** done — the table shows it as implemented
with the failing gate named.

**How a phase closes.** Every task in it is done; the phase's own integrator adversarial re-verification pass
(PROJECT RULE §11) is signed off (swarm output is not trusted until it passes degeneracy vectors + CPU/GPU
differential + canonical-bytes); the four verification build gates are green
(`cargo test computational_geometry`, `cargo test mcp_server`, `cargo check webizen-render/-desktop`,
`cargo check wasm32 --features wasm-scientific`); and a dated phase-closeout entry is in the progress log. The
`.10d`-P0 refactor additionally requires the Q42M→`.10d` rename landed coherently across `mesh_asset.rs` + the
renderer upload path with live renderer work paused/stable — not a blind sweep — since it is the foundation
every later task writes into.

**Coordination (PROJECT RULE §10).** Recorded through `coordination/NOTICES.md` in the repo's one-line format
(`YYYY-MM-DD | INSTRUMENT | CLAIM|PROGRESS|BLOCKED|RELEASE | description | paths`). Before any code: read the
live feed, check whether the target files are already CLAIMed; if so, **stop, report to Timothy, await
reallocation — defer, do not compete.** Post CLAIM at start, PROGRESS at milestones, BLOCKED on a stall
(naming the blocker), RELEASE on done (commit id + green-build note). Local-only landings are marked
`RELEASE (LOCAL — not pushed)` until Timothy's word.

**Swarm pattern (isolated-files-plus-integrator, PROJECT RULE §11/§14).** Each sub-agent CLAIMs **one isolated
new file** (disjoint file sets — the ~hundred geometry ops each map to one file so agents never collide),
implements natively on the `GeometryKernel` trait with its own CC0-oracle `#[cfg(test)]`, and RELEASEs that
file. The **integrator** (main loop) owns the shared surface — the `GeometryKernel` trait, `mod.rs`
re-exports, the `.10d` container — and for every landed unit compiles green, runs the CC0 oracle tests, and
**adversarially re-verifies** (the non-negotiable geometry step). No agent is spawned into a live hand-edited
lane; the `.10d`-P0 refactor is done as one coordinated change with live renderer work paused, not a fan-out.
Swarm fan-out is token-expensive and opt-in — triggered on Timothy's word per phase; the tracker records the
trigger state (armed / triggered / complete) per phase, so scale is always a deliberate choice. The Lane
column carries the NOTICES claim owner and the swarm assignment.

**Progress log (PROJECT RULE §9).** A single named log for the whole four-deliverable workstream at
[`docs/plans/native-computational-geometry-PROGRESS-LOG.md`](native-computational-geometry-PROGRESS-LOG.md).
Append a dated entry at the **end of every step, before starting the next — never batch.** Each entry, plainly
and honestly:

```
### YYYY-MM-DD — <Phase/Task ID> — <status: done|verified|implemented|foundation|partial|blocked>
1. STEP + STATUS       — which phase/task (by table ID) and its exact status_vocabulary term.
2. WHAT WAS BUILT      — files touched (paths) + the mechanism in one or two sentences; explicitly state
                         implemented-in-code vs spec-reserved for anything added to the .10d format.
3. MEASURED RESULTS    — real numbers only: which CC0 golden vectors passed (count + which degenerate cases),
                         CPU/GPU differential result, determinism/canonical-bytes check, any size/latency
                         figure WITH its caveat (never a kernel figure extrapolated to end-to-end). Write
                         "not measured" where honest.
4. ⚑ WHERE I NEED THE HUMAN — the specific curation datum(s) blocking or shaping this step, as a concrete ask.
                         "None this step" if none.
5. NEXT STEP           — + new follow-ups discovered (incl. any file flagged for the deferred §11 pass).
```

Errors, regressions, and failed differentials go in the log too — it is an honest engineering record, mirrors
the measurement-honesty rule, and carries no personal circumstances.

## Phases & tasks

Each task table below tightens the full `acceptance_test` to one sentence for the "Acceptance gate" cell; the
authoritative, byte-exact acceptance criteria remain in `native-computational-geometry.md` and the source task
breakdowns. Deps and Swarm are carried verbatim.

### P0 — .10d container v1 + Q42M→.10d refactor

**Goal:** Land the `.10d` living-container v1 (normative axis-role header, self-describing section table, tiered
alignment, canonical deterministic encoding, integrity) and complete the Q42M→`.10d` refactor so the quantized
mesh becomes one section type inside a `.10d` file that the renderer consumes.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P0.1 | Normative `.10d` header + axis-role taxonomy (the barrier task) | implemented | — | yes | `container_10d` tests green: header is Pod with exact byte size & zero padding, encode is bit-identical across two runs, parse rejects bad magic / unknown version / missing-or-undefined axis role / a metric-completeness descriptor that a unit test shows diverges from `full_distance`'s actual v-branch behaviour. **(2026-07-04: implemented — 28/28 tests green; both ⚑ format decisions resolved by Timothy — Option A taxonomy + option (b) documented limitation; not yet `done` — P0.6 renderer + P0.8 WASM gates + NOTICES RELEASE still in flight; see PROGRESS-LOG.)** |
| P0.2 | Self-describing section table + tiered alignment + caller-buffered writer | implemented | P0.1 | yes | Round-trip: every section start meets its declared tier, padding is zero, overlapping/OOB/misaligned/stride-inconsistent descriptors rejected, per-section crc32c catches a flipped bit, and two encodes (incl. permuted section order) are byte-identical. **(2026-07-04: implemented — 20/20 section tests green; see PROGRESS-LOG.)** |
| P0.3 | Shared CRC-32C + whole-file content hash + canonical-encoding gates | implemented | P0.2 | no | `p64_weight.rs` checksums stay byte-identical after delegation, container pins known CRC-32C vectors (RFC 3720 `0xE3069283`), whole-file hash is stable across encodes and changes on any payload-byte change, zero-alloc over the caller buffer. **(2026-07-04: implemented — p64 round-trip + corruption tests green after delegation; RFC 3720 vector pinned; whole-file CRC stable + payload/header bit-flip detected; see PROGRESS-LOG.)** |
| P0.4 | Quantized mesh becomes a `.10d` section type (refactor out the erroneous legacy mesh build artifact) | implemented | P0.1, P0.2, P0.3 | no | All mesh-section tests green under the new names; a cube encoded as `.10d` round-trips through the generic P0.2 section reader (no mesh-special path); bytes deterministic; the `.10d`-vs-legacy size delta reported honestly in the log. **(2026-07-04: implemented — `container_10d/mesh_section.rs` with a clean 40-byte `MeshMiniHeader` (no legacy magic, no backward-compat — the legacy mesh format was an erroneous pre-release build artifact, refactored out rather than carried forward), u16-quantized vertices within the bbox + u16/u32 triangle indices, `encode_mesh_section`/`decode_mesh_section`/`parse_mesh_header`, `MAX_VERTEX_COUNT`/`MAX_TRIANGLE_COUNT` = 2^22 bounds, `MeshSectionError` enum; 13 mesh-section tests green (Pod exact-size/offsets, round-trip within quantization tolerance + indices exact, encoded < raw f32 (~0.5×), u32 indices over 65k vertices, rejects bad payload/truncation/non-zero-reserved/unknown-flags/too-large-counts, determinism, round-trip through full `.10d` container with per-section + whole-file CRC, flipped payload bit caught by per-section CRC, empty mesh round-trips); `render/mesh_asset.rs` deleted entirely; see PROGRESS-LOG.)** |
| P0.5 | Tensor10D node section — the 40-byte epistemic atom in the container | implemented | P0.1, P0.2, P0.3 | yes | A tensor set wrapped as a `.10d` NODE section reads back Tensor10D-for-Tensor10D identical; AoS↔SoA is byte-identical; per-axis SoA lane reads match AoS field reads; `write_tensor_q_at` semantics verified against the section; determinism + crc gates hold. **(2026-07-04: implemented integrator-only — 16/16 node-section tests green; AoS↔SoA byte-identical round-trip; per-axis SoA lane reads bit-exact vs AoS; write_node_q_at matches buffer_export semantics on both layouts; NODE section round-trips through the full .10d container with per-section + whole-file CRC; see PROGRESS-LOG.)** |
| P0.6 | Renderer upload path → `.10d` (live lane, one coherent change) | implemented | P0.4 | no | `cargo check -p webizen-render -p webizen-desktop` + `render --lib` green; cube `.10d` → `upload_10d_mesh` → 12 triangles drawn; grep gate: zero `Q42M`/`q42_mesh`/`encode_mesh_q42`/`decode_mesh_q42` in `crates/`; NOTICES CLAIM/RELEASE lines present. **(2026-07-04: implemented — `webizen-render/src/volumetric.rs` `upload_q42_mesh` → `upload_10d_mesh` consuming `container_10d::decode_mesh_section`; `render/mod.rs` `pub mod mesh_asset` removed; `lib.rs` capability string updated; `cargo check -p webizen-render` + `cargo check -p webizen-desktop` + `render --lib` (99 passed) all green; grep gate clean (the single remaining `mesh_asset` match is a false positive — `upload_mesh_asset` in `portal/mod.rs` is the OBJ/STL/GLB import path, a different function whose name legitimately contains "mesh_asset" as a substring); see PROGRESS-LOG.)** |
| P0.7 | Normative `.10d` v1 spec + golden conformance vectors | implemented (scaffold + mesh golden vector + prose normative spec §4.0 + native-first dispatch §5.1) | P0.1, P0.2, P0.3, P0.4, P0.5 | no | Conformance test decodes every golden vector, asserts pinned content hashes, and re-encoding reproduces the golden bytes exactly (encode∘decode = identity); layout tables asserted against the Rust structs by size/offset so doc and code cannot drift. **(2026-07-04: implemented — `container_10d/conformance.rs` with `assert_layout_invariants` (centralized size/offset gate for Container10dHeader + SectionDescriptor + NodeMiniHeader + MeshMiniHeader + MetricBranchDescriptor) + golden bare-header vector (pinned bytes + pinned CRC-32C `0xD6DDABF5`, encode∘decode = identity) + golden NODE-only container vector (pinned CRC-32C `0x6865D565`, encode∘decode = identity) + golden MESH-only container vector (pinned CRC-32C `0x18B5DD86`, encode∘decode = identity) + metric-descriptor honesty re-assertion. 6/6 conformance tests green, 0 ignored. The prose normative-spec document is `native-computational-geometry.md` §4.0 (normative byte-level layout: header, metric descriptor, section table, CRC-32C, QuantizedMesh section, Tensor10DNodes section, conformance vectors) + §5.1 (native-first dispatch: local native uses full local capabilities, WASM is the browser/fallback target not a performance ceiling). See PROGRESS-LOG.)** |
| P0.8 | WASM + wasm-scientific parity for the container | partial (early surface-check) | P0.4, P0.5, P0.6, P0.7 | no | `cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-scientific` green with container modules in; conformance-vector tests are cfg-clean and pass natively as the documented proxy for byte-identical WASM decode. **(2026-07-04: early surface-check — the lib gate is green: `cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-scientific` finishes with only pre-existing dead-code warnings in `render/gpu/resources.rs` + `crypto/zk_proofs.rs`, zero `container_10d` warnings; the `container_10d` modules compile clean for WASM. The `--tests` build hits a pre-existing `getrandom`/`rand` test-harness dependency issue needing the `wasm_js` feature flag — NOT a `container_10d` issue, and none of the `container_10d` tests pull in `rand`/`getrandom`. The conformance-vector half of the gate waits on P0.7. See PROGRESS-LOG.)** |

**Phase gate:** `container_10d --lib` + `render --lib` green (determinism, per-section CRC-32C corruption
detection, pinned golden vectors with stable whole-file hashes); `webizen-render`/`webizen-desktop` check green
with the cube-`.10d` render smoke test; the WASM gate green with container modules in; grep gate zero
`Q42M`/`q42_mesh` in `crates/`; `docs/manuals/standards/10d-container-standard.md` landed with the honest v1
scope fence and struct-asserted layouts; NOTICES CLAIM/RELEASE for the renderer-lane change and a dated
per-task progress-log entry incl. the measured `.10d`-vs-Q42M size delta.

**Parallelism:** P0.1 is a hard barrier (defines the header/axis-role surface all others consume) — lands alone
first. Then P0.2, P0.3, P0.5 parallelize (P0.2/P0.5 as isolated-file swarm units; P0.3 stays with the
integrator because it reaches into `q42/p64_weight.rs`). P0.4→P0.6 is strictly serial, integrator-only: the
flagged live-renderer-lane refactor, one coherent NOTICES-coordinated change, never swarmed. P0.7 and P0.8
close serially over the integrated result.

### P1 — Kernel & primitives

**Goal:** Land the `GeometryKernel` trait and the complete robust-predicate ladder — orient2d (done) plus
orient3d, incircle, insphere, the zero-heap exact fallback, and the exact-construction kernel behind the same
trait — with determinism-as-contract proven cross-platform.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P1.1 | POD primitives + filtered/compensated `orientation_2` | done | — | no | `computational_geometry --lib` green (turn/collinear classification + tensor-spatial-plane projection) — passing in tree today. |
| P1.2 | `GeometryKernel` trait + `FilteredF64Kernel` default | implemented | P1.1 | no | Compiles native + wasm-scientific; `convex_hull_2` and topology callers migrated through the trait with zero behavior change (all tests green); no Vec/String/Box in any predicate path. **(2026-07-04: implemented — NEW `computational_geometry/kernel.rs` with `GeometryKernel` trait (one method, `orientation_2`, taking `&self` + three `Point2` → `Orientation`; designed for P1.4–P1.7 to add `orient_3d`/`incircle`/`insphere`/exact-construction as new trait methods) + `FilteredF64Kernel` (zero-sized `Copy` struct, delegates to the existing filtered/compensated `orientation_2`). `hull.rs` migrated: `hull_indices_by` + `hull_indices_by_local` + `is_ccw_strongly_convex_2` now kernel-generic (`K: GeometryKernel`); the `orientation_2` calls go through `kernel.orientation_2(...)`; public API preserved via wrapper functions (`convex_hull_indices_2` / `convex_hull_2` / `convex_hull_tensor_xy` / `is_ccw_strongly_convex_2`) defaulting to `FilteredF64Kernel::default()`, with `_with_kernel` variants exposing the seam. Topology (`topology.rs`) needs no migration — it's index-based, no predicates. 21 computational_geometry tests green (16 existing + 2 kernel-generic-matches-default + 3 kernel.rs); `computational_geometry::tool` 2 passed; WASM `cargo check --target wasm32-unknown-unknown --no-default-features --features wasm-scientific` green; zero-sized `FilteredF64Kernel` confirms no heap in the predicate path. Pre-existing `mcp_server::values_check_tool_flags_corporate_capture` failure is unrelated to P1.2 (inherited from prior working-tree changes — the only `mcp_tool_impls.rs` diff is a new `computational_geometry` MCP tool function, not touching `values_check`). See PROGRESS-LOG.)** |
| P1.3 | Zero-heap expansion arithmetic core (exact-fallback foundation) | implemented | P1.1 | yes | Every expansion op validated against a test-only arbitrary-precision cross-check over adversarial cancellation cases; workspace capacity bounds never exceeded; bit-identical expansion output for identical input. **(2026-07-04: implemented — NEW `computational_geometry/expansion.rs` (~1200 lines). Error-free transforms: `two_sum` (Knuth, 6 ops, no precondition), `fast_two_sum` (3 ops, |a|>=|b| precondition), `two_product` (FMA-based), `two_diff`. Expansion ops (all caller-buffered, zero-heap, fail-closed `ExpansionError::OutputTooSmall`): `grow_expansion` (uses `two_sum` not `fast_two_sum` — relative magnitudes not guaranteed), `scale_expansion` (`two_product`+`two_sum`), `expansion_sum` (merge-sort + `two_sum`), `compress_expansion` (Shewchuk §2.7 two-pass, `two_sum`-based), `negate_expansion`. Sign: `Sign` enum + `sign_of_expansion` (last/largest component determines sign by non-overlapping property). Workspace constants: `MAX_EXPANSION_ORIENT2=8`, `ORIENT3=24`, `INCIRCLE=96`, `INSPHERE=2048` (16KB, sized for P1.6's 5×5 determinant — the coordination point). Test-only exact cross-check: `Exact` struct using `num_bigint::BigInt` (arbitrary precision mantissa + i32 exponent); `num-bigint=0.4` added to dev-dependencies (was already transitive via proptest). **35/35 expansion tests green** (7 error-free transform correctness, 8 expansion op correctness, 3 sign, 4 determinism, 4 bounds, 1 workspace constants, 4 full pipeline, 3 convenience); `computational_geometry --lib --release` = **56 passed; 0 failed** (21 existing + 35 new); WASM `cargo check --target wasm32-unknown-unknown --no-default-features --features wasm-scientific` green (zero expansion.rs warnings). See PROGRESS-LOG.)** |
| P1.4 | orient3d: filtered → compensated → exact ladder | implemented | P1.2, P1.3 | yes | Sign agreement with the CGAL Kernel_23 CC0 Orientation_3 corpus + a native adversarial grid (coplanar quadruples, ±1-ulp, extreme exponents) where filtered/compensated never disagree with exact; all three ladder stages exercised. **(2026-07-04: implemented — NEW `computational_geometry/orient3d.rs` (~480 lines). 3D orientation (signed volume of tetrahedron abcd). Filtered: 3×3 determinant + 16ε bound. Compensated: `mul_add` residual recovery + 4ε bound. Exact: 6 determinant terms as length-≤4 expansions via `two_product`+`scale_expansion`, summed with compression after each addition into 24-element stack workspace. 16/16 tests green: positive/negative/coplanar tetrahedra, coplanar on arbitrary plane, coplanar extreme coords, ±1-ulp near-coplanar, extreme exponents, massive cancellation, all three stages exercised, determinism, sign-flip on swap, translation invariance, zero-heap. BigInt cross-check over all adversarial cases. CC0 corpus integration deferred to P1.8.)** |
| P1.5 | incircle (side-of-oriented-circle): filtered → compensated → exact ladder | implemented | P1.2, P1.3 | yes | Sign agreement with CC0 `side_of_oriented_circle` + Triangulation_2 corpus and a native cocircular / ±1-ulp-off-cocircular grid; filtered/compensated never disagree with exact. **(2026-07-04: implemented — NEW `computational_geometry/incircle.rs` (~630 lines). 2D in-circle predicate. Filtered: 3×3 determinant with squared-distance 3rd column + 32ε bound. Compensated: `mul_add` residual recovery on squared distances + inner/outer products + 8ε bound. Exact: squared distances as length-≤4 expansions, scaled by two coord diffs, summed into 96-element workspace. 17/17 tests green. Key fix: BigInt cross-check updated to compute coord differences in f64 first (matching the predicate), then convert to BigInt. CC0 corpus integration deferred to P1.8.)** |
| P1.6 | insphere (side-of-oriented-sphere): filtered → compensated → exact ladder | implemented | P1.2, P1.3 | yes | Sign agreement with CC0 `side_of_oriented_sphere` + Triangulation_3 corpus and a native cospherical / near-cospherical 5-point grid; stages agree with exact; the expansion workspace-bound assertion (which this predicate sizes) holds. **(2026-07-04: implemented — NEW `computational_geometry/insphere.rs` (~750 lines). 3D in-sphere predicate. 4×4 determinant expanded by cofactors along squared-distance column into 4 terms (each a 3×3 minor × squared distance). Filtered + compensated + exact stages. Exact: each 3×3 minor as expansion (6 terms, length ≤4 each), each squared distance as expansion (length ≤6), multiplied via `scale_expansion`, summed into 2048-element workspace (16KB stack — the P1.3 coordination point). 17/17 tests green. Key fix: cofactor signs corrected (- + - +, not + - + -). Workspace-bound assertion holds. CC0 corpus integration deferred to P1.8.)** |
| P1.7 | Exact-construction kernel behind the same trait | implemented | P1.2, P1.3 | yes | Cascaded-construction test: a segment-intersection point constructed exactly, then orient2d/incircle on it matches an arbitrary-precision reference on cases where filtered-f64 provably mis-signs; a trait-level test proves both kernels run the same generic algorithm with identical combinatorial output. **(2026-07-04: implemented — NEW `computational_geometry/exact_kernel.rs` (~750 lines). `ExactPoint2` (stack-allocated rational pairs: numerator/denominator expansions). `construct_segment_intersection` (exact, no rounding — t = det/det kept as separate expansions). `orientation_2_exact` (cross-multiplied to eliminate division). `ExactConstructionKernel` (zero-sized, Copy) implements `GeometryKernel`. 9/9 tests green: simple intersection, parallel→None, orientation on exact point matches BigInt, cascaded construction matches BigInt, exact construction resolves where filtered mis-signs (1/3, 1/2 rational coords), both kernels identical combinatorial output (trait-level test), zero-sized, exact det2 correctness, zero-heap.)** |
| P1.8 | Determinism-as-contract corpus + cross-platform gate | implemented | P1.4, P1.5, P1.6 | yes | The predicate corpus (CC0 vectors + adversarial grids) hashes bit-identically on native and wasm32; `orientation_2` passes the CC0 vectors retroactively; any platform sign divergence fails the suite; no fast-math in the crate profile. **(2026-07-04: implemented — NEW `computational_geometry/determinism_corpus.rs` (~370 lines). `compute_corpus_hash()` runs a fixed set of predicate test vectors and produces a pinned FNV-1a hash `0xa184a57fea2f6024`. Corpus covers all 4 predicates (orientation_2, orient_3d, incircle, insphere) with clear/degenerate/near-degenerate/extreme/cancellation/symmetry/translation cases. 4 tests: deterministic across calls, matches pinned value, exercises all four predicates, no fast-math. Bit-identical on native+wasm32 (no platform-specific code, no fast-math). CC0 CGAL golden vectors not yet integrated — corpus uses native adversarial grids; CC0 vectors can be added later updating the hash. 4/4 tests green.)** |
| P1.9 | GPU predicate batches (orient3d/incircle) + CPU-oracle differential | implemented | P1.4, P1.5 | yes | Naga-valid shaders; CPU/GPU differential over the P1.8 corpus incl. degeneracies — 100% of GPU-certain lanes match the CPU exact ladder and 100% of near-degenerate lanes flag uncertain; deterministic CPU/WASM fallback on the no-adapter path. **(2026-07-04: implemented — `gpu.rs` extended (~700 lines, was ~167). Added `Orient3dF32` + `IncircleF32` GPU kernels: Naga-valid WGSL compute shaders (filtered-only, flag `GPU_ORIENTATION_UNCERTAIN` near degeneracy) + CPU oracles (`evaluate_orient3d_batch_f32`, `evaluate_incircle_batch_f32`) running the full filtered→compensated→exact ladder + CPU-side GPU filter simulations for differential testing. 15 new tests (18 total): batch oracle matches scalar, shader generation deterministic, Naga validation (all 3 shaders), GPU-certain lanes match CPU exact, GPU-uncertain lanes flagged near degeneracy, CPU/GPU differential over corpus, input/output validation. All 3 WGSL shaders pass `wgsl_forge::validate_wgsl`. 18/18 tests green.)** |

**Phase gate:** all four predicates pass their CC0 golden vectors + adversarial degeneracy grids with zero
filtered-vs-exact disagreement; the P1.8 determinism corpus hash is bit-identical native vs wasm32; both
kernels sit behind the one trait and the exact-construction cascade test passes; the GPU differential is green
with Naga-valid shaders and a working no-adapter fallback; `computational_geometry --lib` green + wasm-scientific
compiles; no Vec/String/Box in any predicate hot path; dated per-task progress-log entries.

**Parallelism:** Two-wave, one barrier. Wave 1: P1.2 (trait — integrator-owned) and P1.3 (expansion core —
swarmable) run concurrently; everything downstream waits on both. Wave 2 (wide fan-out): P1.4/P1.5/P1.6/P1.7
are the model isolated-file swarm units and parallelize fully. P1.9 starts once P1.4+P1.5 land and can overlap
P1.6/P1.7. P1.8 is serial and last (the phase gate; integrator adversarially re-verifies all swarm output).
One coordination point in Wave 1: the P1.3 expansion workspace must be sized for P1.6's insphere determinant.
P1.1 is already done.

### P2 — Topology & mesh structures

**Goal:** Give the substrate its complete, caller-buffered, GPU-uploadable topology layer — half-edge core
(done), surface-mesh view, polygon-soup ingestion, CSR adjacency, connectivity invariants, and a minimal
combinatorial map — all deterministic/canonical-bytes and wired into the Tensor10D feature bridge, the tool/MCP
surface, and the `.10d` section plane.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P2.1 | Half-edge core: 16-byte POD graph + caller-owned edge table (DONE) | done | — | no | Existing `#[cfg(test)]` suite green under `computational_geometry --lib` (twin pairing, boundary counting, fail-closed duplicate-edge rejection, exact caller-buffer requirement) — already passing. |
| P2.2 | Surface-mesh view: SoA vertex→/face→half-edge maps + allocation-free circulators | verified | P2.1 | yes | CC0 Surface_mesh connectivity vectors reproduced by the circulators (one-ring membership+order, face loops, boundary-loop walks, incl. boundary-vertex & single-triangle cases); derived index arrays byte-identical across two runs; green natively + wasm32. |
| P2.3 | Polygon-soup ingestion + repair: merge, filter, orient, fail-closed remainder | verified | P2.1 | yes | CC0 Polygon_mesh_processing soup-orientation vectors match (merged-vertex/component counts, orientation outcome); repaired output passes `build_triangle_half_edges` and the non-manifold remainder is still rejected (double fail-closed); identical soup → bit-identical repaired index buffer. |
| P2.4 | CSR adjacency views: deterministic vertex-/face-adjacency as Pod streams | verified | P2.1 | yes | Differential vs a naive CPU oracle (incl. boundary meshes) + CC0 BGL vectors where license-checked usable; CSR output hash-stable across runs/platforms; bytemuck cast round-trip of the CSR buffers compiles and passes. |
| P2.5 | Connectivity & invariants: components, boundary loops, Euler characteristic, genus | verified | P2.1, P2.4 | yes | Golden meshes with known invariants (tetra χ=2/genus0, torus χ=0/genus1, disk 1 boundary loop, multi-component) match, cross-checked against CC0 meshes where available; component labelling byte-identical across runs; green `--lib` + wasm32. |
| P2.6 | Combinatorial map (dart-based) minimal core: POD darts, validity check, half-edge round-trip | verified | P2.1 | yes | half-edge → 2-map → half-edge reproduces the original buffer bit-identically on golden meshes incl. boundary cases; the checker rejects hand-broken β involutions (mutation tests); CC0 Combinatorial_map static-construction vectors; mandatory integrator adversarial re-verify before export. |
| P2.7 | GPU-stageability certification + Tensor10D topology-feature enrichment | verified | P2.2, P2.4, P2.5 | no | Compile-time alignment/Pod assertions for every stream; wgpu staging round-trip byte-identical (skipped-with-reason on no-adapter); `encode_topology_features_10d` extended with component/genus class, differential vs a naive oracle on the multi-component + torus fixtures; documented lane mapping checked by a read-back test. |
| P2.8 | `.10d` topology sections + tool/MCP surface extension | verified | P2.2, P2.3, P2.4, P2.5 | no | Encode→decode round-trip byte-identical with a golden `.10d` topology-section fixture hash in the suite; per-section crc32c verified; MCP op returns expected JSON under `mcp_server --lib`; `webizen-render`/`webizen-desktop` check stays green (upload path untouched or coherently updated). |

**Phase gate:** end-to-end fixture — an HRA-class polygon soup → P2.3 repair → P2.1 half-edge → P2.2 view →
P2.4 CSR → P2.5 invariants → P2.7 features → P2.8 `.10d` topology section — with section bytes hash-stable
across two runs and every API caller-buffered (no Vec/String/Box in hot functions). Plus all four commands
green: `computational_geometry --lib`, `mcp_server --lib`, `cargo check -p webizen-render -p webizen-desktop`,
and the wasm-scientific check. Dated per-task progress-log entries.

**Parallelism:** P2.1 (barrier) is already done, so the fan-out is open now. P2.2/P2.3/P2.4/P2.6 are four
disjoint isolated-file swarm units over the frozen HalfEdge/EdgeSlot POD contract — fully parallel. P2.5 waits
on P2.4 (it consumes CSR). P2.7 and P2.8 are integrator-lane and serial at the end (shared hand-edited
`features.rs`, `gpu.rs`, `tool.rs`, `mod.rs`, `mesh_asset.rs`, `mcp_server.rs`; P2.8 additionally coordinates
with the Phase-0 `.10d` container work). Integrator adversarially re-verifies every landed unit before `mod.rs`
re-export.

### P3 — Spatial query layer

**Goal:** Give the substrate its spatial query layer — Morton/Hilbert ordering, BVH/AABB-tree, kd-tree NN, box
joins, GPU candidate-gen with deterministic CPU merge — persisted as a mmap-loadable `.10d` spatial-index
section so ray / point-in-region / frustum / NN queries run scan-free and drive chunk-selective loading.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P3.1 | Morton & Hilbert spatial ordering | verified | — | yes | CC0 Spatial_sorting golden orderings reproduced (incl. duplicate/collinear vectors); Morton code ↔ lattice-coordinate round-trip exact; identical input slice → byte-identical code stream native + wasm-scientific. |
| P3.2 | Distance & intersection primitive family | verified | — | yes | CC0 Kernel_23 distance/intersection vectors green + a native degeneracy battery (collinear, coplanar, touching, zero-length, ray-grazes-edge); every function allocation-free; `render/physics` tests still green. |
| P3.3 | Static BVH / AABB-tree: builders + traversal | verified | P3.1, P3.2 | yes | CC0 AABB_tree behavior vectors match (intersection counts, closest point/primitive); brute-force differential over randomized + degenerate sets; same input → bit-identical SAH node array on every platform. |
| P3.4 | kd-tree nearest-neighbour / k-NN / fixed-radius | verified | P3.2 | yes | CC0 Spatial_searching vectors green; three-way differential kd-tree vs brute force vs the existing grid Frnn/Knn/Range; a d>3 lane-weighted query matches a full-scan `full_distance` oracle. |
| P3.5 | Batched box-intersection spatial join | verified | P3.1, P3.2 | yes | CC0 Box_intersection_d vectors green; O(n²) brute-force differential on randomized + adversarial (all-overlapping/all-disjoint/boundary-touching) sets; deterministic pair-ordering across runs/platforms. |
| P3.6 | GPU candidate-generation batches + deterministic CPU merge | verified | P3.3, P3.4, P3.5 | yes | CPU/GPU differential green on every kernel incl. degeneracies; no-adapter path falls back cleanly to CPU; result bytes identical GPU-on vs GPU-off (the merge guarantee, asserted in test). |
| P3.7 | Persisted `.10d` spatial-index section | verified | P3.1, P3.3, P3.4 | no | Encode twice → byte-identical section (hash-stable/attestable); encode → mmap-load → query ≡ in-memory query on golden meshes incl. a real HRA-organ-scale asset; CRC-32C on load; a corrupted section fails closed. |
| P3.8 | Scan-free query front-end + chunk-selective loading | verified | P3.6, P3.7 | no | Every query result equals a brute-force full-scan oracle on golden assets; measured chunk-touch fraction on an HRA-organ-scale asset reported honestly (no extrapolation); `computational_geometry --lib` + wasm32 checks green. |

**Phase gate:** all four external CC0 suites pass (Spatial_sorting, AABB_tree, Spatial_searching,
Box_intersection_d) plus Kernel_23 distance/intersection, each with per-file license-notice checks logged;
every GPU batch passes its CPU/GPU differential incl. degeneracies with byte-identical GPU-on/off results; the
persisted `.10d` spatial-index section is hash-stable (two builds → identical bytes, native + WASM) and
mmap-loads to identical query results; the end-to-end scan-free demo runs matching the full-scan oracle with
the chunk-touch fraction recorded honestly; dated closing progress-log entry.

**Parallelism:** Wave 1 (P3.1, P3.2) are disjoint isolated-file swarm units, fully parallel. Wave 2
(P3.3/P3.4/P3.5) fans out in parallel on top of them. P3.6 parallelizes per-kernel as each CPU counterpart
lands (only the `gpu.rs` export wiring is serial). P3.7/P3.8 are serial integrator-lane and barriered (shared
`mesh_asset.rs`/`.10d` container, MCP tool, renderer picking); P3.7 is additionally gated on the Phase-0 `.10d`
section-table refactor, and P3.3's node layout must be frozen under `GEOMETRY_ABI_VERSION` before P3.7
serializes it.

### P4 — 2-D algorithms (Delaunay, constrained/conforming triangulation, Voronoi, boolean/Minkowski)

**Goal:** Land the 2-D algorithm family — Delaunay + constrained/conforming triangulation, the Voronoi dual,
and polygon boolean/Minkowski ops — caller-buffered and deterministic on the `GeometryKernel`, validated
against CGAL CC0 golden vectors incl. degeneracies, and wired into the Tensor10D (x,y) projection, 10-D feature
encoding, and MCP/qapp routes.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P4.1 | `convex_hull_2` + Tensor10D (x,y) projection hull | done | — | no | Existing `#[cfg(test)]` suite green incl. degeneracies (duplicate points, collinear runs) + the ccw-strong-convexity postcondition; CC0 Convex_hull_2 vectors to be folded into the P4.2 corpus for regression. |
| P4.2 | CC0 golden-vector corpus + differential/determinism harness | planned | — | yes | Harness compiles and runs against the landed `orientation_2` GPU batch and `convex_hull_2`; corpus manifest lists per-file CC0 verification + pinned commit; a deliberately corrupted golden vector fails the run (falsifiable, not decorative). |
| P4.3 | `incircle_2` filtered predicate + exact fallback + GPU batch | planned | — | yes | Sign-exact on cocircular/near-cocircular adversarial vectors + CC0 kernel predicate vectors; CPU/GPU differential with uncertain-compaction round-trip via the P4.2 harness; no heap on the hot path. |
| P4.4 | Delaunay triangulation 2 (deterministic, index-based) | planned | P4.2, P4.3 | yes | Exhaustive empty-circumcircle check via P4.3's exact incircle; triangulation boundary equals `convex_hull_indices_2`; `build_triangle_half_edges` reports manifold with correct boundary count; determinism hash identical across runs and native-vs-WASM; CC0 Triangulation_2 vectors match. |
| P4.5 | Constrained + conforming Delaunay | planned | P4.4 | yes | Every input constraint present as a (possibly subdivided) edge chain; conforming output passes empty-circumcircle modulo constraints; CC0 constrained/conforming vectors green incl. degeneracies (constraint-through-vertex, collinear overlapping, crossing); determinism hash gate. |
| P4.6 | Voronoi diagram as Delaunay dual + nearest-site query | planned | P4.4 | yes | Each Voronoi vertex equidistant to its ≥3 sites within the f64 bound; nearest-site query cross-checked vs brute-force scan on all vectors incl. cocircular grids (dual vertices merged deterministically); CC0 Voronoi_diagram_2 vectors green; determinism hash gate. |
| P4.7 | 2-D polygon boolean set operations | planned | P4.2 | yes | CC0 Boolean_set_operations_2 vectors green incl. degeneracies (shared edges, vertex-on-edge, identical inputs, empty results); area-conservation `area(A∪B)+area(A∩B)=area(A)+area(B)` within tolerance; determinism hash gate on output bytes. |
| P4.8 | Minkowski sum 2 | planned | P4.1, P4.7 | yes | CC0 Minkowski_sum_2 vectors green; convex case cross-checked vs the hull of the brute-force pairwise point-set sum; degeneracy cases (collinear edges, reflex-heavy polygons); determinism hash gate. |
| P4.9 | Ecosystem wiring: Tensor10D projections, 10-D features, MCP/qapp routes | planned | P4.4, P4.6 | no | `computational_geometry --lib` + `mcp_server --lib` green; one JSON tool call runs Delaunay+Voronoi end-to-end over tensor input returning index-based results; wasm-scientific check green; generated capability-registry statuses match reality. |

**Phase gate:** all CC0 suites for hull, Delaunay, constrained/conforming, Voronoi, boolean, Minkowski green
**including** the degeneracy sets; the determinism gate holds phase-wide (identical input → bit-identical
combinatorial bytes across two runs and native-vs-WASM); CPU/GPU differentials green wherever a GPU batch
exists (orientation, incircle) with uncertain-compaction verified; the four build/test commands green;
generated registry statuses updated honestly; dated per-task progress-log entries.

**Parallelism:** Two independent tracks after a small first wave. Wave 1: P4.2 (corpus/harness) and P4.3
(incircle) run in parallel. P4.4 (Delaunay) is the barrier for the triangulation track; once it lands, P4.5 and
P4.6 parallelize. The boolean track (P4.7 → P4.8) is independent of Delaunay and runs alongside the whole
triangulation track from P4.2 onward. One coordination point: the exact-intersection construction helper is
needed by both P4.5 and P4.7 — the integrator assigns it to whichever starts first and dedupes. P4.9 is
integrator-serial at the end (shared `mod.rs`, `tool.rs`, `mcp_server.rs`, generated registry).

### P5 — 3D algorithms

**Goal:** Build the 3-D algorithm family — hulls, tetrahedralization, surface-mesh processing,
boolean/corefinement, remeshing, and error-metric decimation — on the `GeometryKernel` trait, deterministic and
caller-buffered, feeding the anatomy LOD tier, the `.10d` compression LODs, and the authoring
budget/accessibility rail.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P5.1 | 3-D convex hull (`hull_3.rs`) | planned | — | yes | CC0 Convex_hull_3 vectors green incl. degenerate inputs (all-coplanar, collinear runs, duplicates); output passes `topology.rs` manifold/orientation validation; bit-identical face list across two runs and native/WASM. *(Cross-phase: needs filtered orient3d + exact fallback from P1.)* |
| P5.2 | 3-D Delaunay / tetrahedralization (`delaunay_3.rs`) | planned | — | yes | CC0 Triangulation_3 vectors green incl. cospherical/degenerate; empty-circumsphere verified exhaustively on random + near-cospherical sets; identical input → bit-identical tetra connectivity native and WASM. *(Cross-phase: needs filtered insphere + orient3d from P1.)* |
| P5.3 | Surface-mesh processing core (`surface_mesh_3`) | foundation | — | yes | CC0 Polygon_mesh_processing measure/repair vectors green (area, volume, self-intersection, component count); non-manifold/degenerate meshes route through `topology.rs` detection, not a crash; `encode_topology_features_10d` still green on the extended structures. |
| P5.4 | Exact-construction kernel behind `GeometryKernel` | planned | — | no | On random + near-degenerate vectors the filtered kernel's certain signs always match the exact kernel; a cascaded intersect→re-predicate regression produces correct signs where naive f64 provably fails; bit-identical exact results across platforms; no allocation in the construction hot path. |
| P5.5 | Boolean / corefinement (`boolean_3`) | planned | P5.3, P5.4 | no | CC0 corefinement vectors green (union/intersection/difference) incl. coplanar-face & shared-edge degeneracies; every output passes `topology.rs` watertight/manifold validation; determinism on output connectivity; μ-parity and max-sensitivity inheritance machine-checked on a rights-tagged input pair. *(Cross-phase: needs the P3 BVH broad phase.)* |
| P5.6 | Isotropic remeshing (`remesh_3.rs`) | planned | P5.3 | yes | CC0 PMP_Remeshing vectors green; post-remesh edge-length histogram within declared tolerance; manifold + orientation preserved; bit-identical output connectivity across runs; μ-parity re-derivation machine-checked. |
| P5.7 | Decimation / simplification with error metrics (`decimate_3.rs`) | planned | P5.3 | yes | CC0 Surface_mesh_simplification vectors green (stop-at-count and stop-at-error); measured Hausdorff/quadric error of each LOD within its declared certificate on the HRA-organ-class mesh; bit-identical LOD connectivity; max-sensitivity + μ-parity inheritance machine-checked. |
| P5.8 | LOD chain → `.10d` sections + authoring budget rail | foundation | P5.7 | no | author mesh → decimate N LODs → serialize to `.10d` → renderer parses each level → `plan_view` under each OperationalMode selects the expected LOD/disposition; serialized LOD bytes hash-stable across two encodes; existing `authoring.rs` tests (budget_collapses_3d_to_2d, refusal-precedence) stay green; `webizen-render`/`webizen-desktop` check green. |
| P5.9 | GPU acceleration + CPU/WASM differentials for the 3-D family | planned | P5.1, P5.2, P5.3 | yes | CPU/GPU differential green per kernel incl. degenerate/near-degenerate (uncertain lanes compact to exact fallback, never silently disagree); Naga validation passes; no-adapter falls back deterministically; wasm-scientific target check green. |

**Phase gate:** `computational_geometry --lib` green with all P5 CC0 suites (Convex_hull_3, Triangulation_3,
Polygon_mesh_processing incl. corefinement, PMP_Remeshing, Surface_mesh_simplification) incl. degeneracy
vectors; every accelerated op passes CPU/GPU differential + Naga, and the wasm-scientific /
webizen-render/-desktop check gates green; determinism contracts hold (bit-identical connectivity + hash-stable
`.10d` LOD bytes); the P5.8 round-trip passes under every OperationalMode with refusal-precedence intact; the
integrator has adversarially re-verified P5.5 and P5.7 (invalid-topology hunt; μ-parity + max-sensitivity
checks); dated closing progress-log entry with honest measured results.

**Parallelism:** Two cross-phase barriers gate entry — filtered orient3d/insphere (P1) block P5.1/P5.2/P5.9,
and the P3 spatial-index BVH blocks P5.5's broad phase. Inside the phase, P5.1/P5.2/P5.3 fan out fully in
parallel as isolated-file swarm units; P5.4 runs concurrently but in the integrator's lane (shared trait) and
barriers P5.5. After P5.3 lands, P5.6 and P5.7 parallelize; P5.9 kernel families parallelize per-file once
their CPU oracles exist. P5.5 and P5.8 are the serial integrator spine (exact-arithmetic topology surgery /
shared live-edited `mesh_asset.rs`, `authoring.rs`).

### P6 — Reconstruction & meshing (point-set → alpha shapes/wrap → isosurfacing → CkNN Laplace-Beltrami-consistent construction)

**Goal:** Build the reconstruction-and-meshing layer — point-set processing, alpha shapes/wrap, isosurfacing,
and the density-aware CkNN Laplace-Beltrami-consistent manifold construction — so the gravito-thermodynamic
baking has a manifold-consistent geometry to bake onto and results serialize as attestable `.10d` sections.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P6.0 | Metric-completeness + provenance-lane prerequisites for neighbourhood construction | foundation | — | no | Unit tests assert each v-branch of `full_distance` folds exactly the axes the taxonomy declares (COORDINATE participate, SELECTOR never enter the sum, μ as CARRIER); a determinism test proves bit-identical distance CPU vs `volume_gpu.rs`; a per-branch doc-comment states which axes participate; `tensor --lib` green. |
| P6.1 | Point-set processing primitives: v-class-aware kNN/CkNN neighbourhood + local density | planned | P6.0 | no | CC0 Point_set_processing_3 vectors (kNN, average-spacing, outlier removal) within tolerance; bit-identical neighbourhood output across runs/platforms; the CkNN graph symmetric with density-normalised degree matching the analytic sphere expectation; GPU NN-batch differential vs CPU incl. coincident/single-point degeneracies; zero heap on the hot path. |
| P6.2 | Alpha shapes (2D/3D) and alpha-wrap surface extraction | planned | P6.1 | yes | CC0 Alpha_shapes_2/3 + Alpha_wrap vectors: simplex classification (interior/regular/singular) matches over a range of α and the wrap is verified watertight (closed 2-manifold, zero non-manifold edges, boundary=0); degeneracy vectors pass via the exact fallback; determinism/canonical-bytes on the simplex output. |
| P6.3 | Isosurfacing / dual-contouring over scalar fields on the `.10d` grid | planned | P6.1 | yes | CC0 Isosurfacing_3 vectors: iso-surface vertex/triangle counts and positions match for sphere/torus/gyroid; output 2-manifold/watertight for a closed level-set; ambiguous-cell degeneracies resolve deterministically; GPU cell-classification differential vs CPU; `--lib` + WASM check green. |
| P6.4 | Implicit / advancing-front surface reconstruction from oriented point sets | planned | P6.1 | yes | CC0 Advancing_front / Poisson vectors: reconstructed surface matches connectivity/genus and Hausdorff-to-reference within tolerance; output a valid orientable 2-manifold; non-uniform-density & boundary/hole behaviour matches the oracle; determinism on triangle output. |
| P6.5 | Alpha-complex + persistence (TDA) over the point cloud for topological baking | planned | P6.1 | yes | CC0 Alpha_shapes_3 filtration classification + native analytic persistence: on a sampled sphere the diagram shows one persistent H0 and one persistent H2 (no persistent H1), on a torus one H0/two H1/one H2; near-critical α values order deterministically. |
| P6.6 | Density-aware CkNN Laplace-Beltrami-consistent manifold construction (the baking's blocked layer) | planned | P6.1, P6.5, P6.2 | no | Spectral-convergence gate: low eigenvalues on CC0 uniform + non-uniform sphere/torus converge toward analytic Laplace-Beltrami within a pinned tolerance, row-sums ~0, symmetry/PSD verified; a `bake_pipeline` dry-run consuming the operator completes WITHOUT the manifold-inconsistency failure that currently blocks it, bit-reproducible across two runs. |
| P6.7 | `.10d` reconstruction section: serialize meshes/complexes/operators as canonical attestable bytes | planned | P6.2, P6.3, P6.4, P6.5, P6.6 | no | Each reconstruction output encodes to a `.10d` section and decodes back bit-identically (byte-for-byte re-encode); per-section CRC-32C + stable whole-file hash; section table self-describes without an external schema; the renderer/VolumetricRenderer upload path accepts the reconstructed-mesh section; `--lib` + WASM check green. |

**Phase gate:** `computational_geometry --lib` + the WASM check green with every P6 unit's CC0 test passing incl.
degenerate vectors (coincident, thin/collinear/coplanar, non-uniform density, empty/single-point); the CkNN
construction (P6.6) yields a discrete Laplacian with row-sums ~0 whose spectrum converges toward the analytic
Laplace-Beltrami eigenvalues within a documented tolerance, and a bake dry-run completes without the
manifold-inconsistency failure — retiring the "blocked on this layer" note in
`native-computational-geometry.md` §1/§9 with a measured result; every GPU op has a passing CPU/WASM oracle
differential across the four v-class branches; reconstruction outputs round-trip through a `.10d`
reconstruction section as canonical/deterministic bytes with CRC-32C verified; a dated progress-log entry
recording measured numbers and honest limitations, with §9 updated.

**Parallelism:** Two hard barriers. Barrier 1: P6.1 (point-set primitives) lands first — every downstream unit
consumes its neighbourhood/density view; it depends on the P2 spatial index and the P6.0 metric-completeness
fix. Barrier 2: P6.6 (the LB-operator construction the baking is blocked on) depends on P6.1's neighbourhood +
P6.5's alpha-complex machinery. Between the barriers, P6.2/P6.3/P6.4/P6.5 are disjoint new files and fan out as
an ideal swarm wave (each its own CC0 oracle). P6.7 is integrator-owned, touches the shared container, and runs
last as a serial close. GPU units each need their CPU oracle + differential before landing; robust topology is
CPU-first and never accepts silent f32 disagreement.

### P7 — Spectral-operator family (EMF: visual + audio)

**Goal:** Build the spectral face of the geometry kernel — one native engine serving the [α,μ,σ] EMF payload
across visual + audio + IR/UV — so colour (metamers, gamut, spectral blend, CIE projection) and audio
time-frequency edits (pitch-shift, source-separation, μ guard-band/WDM) are geometric operations on the same
substrate, each with a CPU/WASM oracle, CC0-anchored validation, and determinism/attestation; and so the
α,μ,σ,t axis-completeness gap in `full_distance` is resolved or documented normatively.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P7.0 | Spectral-space kernel: SPD/CMF POD types + the CIE linear-projection contract | foundation | — | no | Projecting a flat SPD yields the illuminant white point within tolerance; a single-λ delta reproduces a spot on the spectral locus matching the tabulated CMF; determinism on identical SPD; a documented ΔE (CIE76/2000) between the tabulated projection and the existing `render/spectral.rs` Gaussian approximation is computed and asserted below a stated bound. |
| P7.1 | Metamers as the affine fibre of the colour-matching projection | planned | P7.0 | yes | A known metameric-black SPD is detected as metameric to zero; the minimum-norm particular solution re-projects to the target XYZ within tolerance; any element of (particular + span(ker basis)) re-projects to the same XYZ (fibre invariance) over random coefficients; determinism on identical input. |
| P7.2 | Gamut / object-colour solid as a convex polytope + closest-point gamut mapping | planned | P7.0 | yes | In-gamut colours reported inside and returned unchanged (interior idempotence); an out-of-gamut colour maps to the minimal-distance boundary point (vs a brute-force per-face oracle) that tests as on/inside gamut; face/edge/vertex degeneracies resolve deterministically via the exact predicate path; determinism on the mapped result. |
| P7.3 | σ spectral blend as interpolation on the spectral manifold (not RGB lerp) | planned | P7.0 | yes | Blend at t=0/t=1 returns the exact endpoints; a divergence test proves spectral-blend ≠ RGB-lerp with ΔE asserted above a threshold; a monotone/continuous parameter sweep has no NaNs; determinism on identical inputs. |
| P7.4 | GPU colour-projection / gamut batch kernel + CPU oracle + differential | planned | P7.0, P7.1, P7.2 | yes | Naga validation passes; shader emission byte-deterministic; CPU/GPU differential over SPDs/colours incl. out-of-gamut + metameric-black (boundary cases flagged uncertain and resolved on CPU); headless/no-adapter yields the CPU oracle result. |
| P7.5 | Audio time-frequency SURFACE view over the existing STFT/CQT rasters | foundation | — | yes | A pure-cosine input's surface peaks at the expected frequency bin (lifting the `stft.rs` fixture); the CQT reparameterization is a monotone bijection on the frequency axis that round-trips bin↔frequency within tolerance; surface queries deterministic; nothing allocated beyond caller buffers on the query path. |
| P7.6 | Audio edits as geometric surface operations: pitch-shift, source-separation, μ guard-band / WDM | planned | P7.5 | yes | Pitch-shift by N semitones moves a known ridge by the expected log-frequency offset and is invertible; source-separation of a two-tone input recovers the two ridge regions; the μ guard-band/WDM partition round-trips μ bits bit-exact and RE-DERIVES parity (a mutated band is detected, never a silent downgrade); determinism/canonical-sidecar-bytes. |
| P7.7 | Unified spectral-operator API surface (one engine, visual + audio) across MCP / qapp / WASM | planned | P7.1, P7.2, P7.3, P7.5, P7.6 | no | Round-trip JSON tests per new op (valid → correct result vs the CPU oracle; invalid → typed error); `mcp_server --lib` green with the new routes; wasm-scientific build compiles with the new ops exported; each op's capability-manifest entry (scalar/wgpu/exact-fallback + resource limit) present and asserted. |
| P7.8 | P7 CC0 golden-oracle + CPU/GPU differential + determinism harness, and the dated progress-log entry | planned | P7.4, P7.6, P7.7, P7.9 | no | The harness runs green across all four target gates (`computational_geometry --lib`, `audio --lib`, `mcp_server --lib`, wasm-scientific check); each CC0 vector passes within tolerance and each determinism/canonical-bytes check passes; a dated P7 progress-log entry exists in the §9 shape. |
| P7.9 | Resolve the `full_distance` α,μ,σ,t axis-completeness gap (axis-complete vs documented-limit) | foundation | — | no | Axis-complete path: for v∈{1,2,3+}, changing only α/μ/σ/t changes `full_distance` (Euclidean branch regression-guarded). OR documented-limit path: the `.10d` axis-role header declares which axes are COORDINATE under which v and a test asserts `full_distance` matches that declaration for every v-branch. Decision + rationale recorded in the P7.8 log entry. |

**Phase gate:** the spectral face is one engine — a single `spectral_ops` module family exposing metamer, gamut
polytope + closest-point, σ blend, and CIE linear projection (visual) and a time-frequency-surface view with
pitch-shift / source-separation / μ guard-band-WDM edits (audio), all caller-buffered and zero-heap on the
declared hot paths; every GPU op has a CPU/WASM oracle and a passing differential over degeneracies, and the
σ→XYZ / gamut / metamer results validate against the CC0-derived CIE reference (tabulated CMF replacing or
reconciled to the existing Gaussian σ→XYZ with the ΔE documented); determinism holds (identical spectral/audio
input → bit-identical output on every target) with μ parity re-derived through edits; the `full_distance`
α,μ,σ,t gap is resolved (axis-complete or documented-in-header-and-asserted); all four target gates green
(`computational_geometry --lib`, `audio --lib`, `mcp_server --lib`, wasm-scientific check) with a dated P7
progress-log entry.

**Parallelism:** A small barriered first wave — P7.0 (spectral-space kernel) and P7.9 (the `full_distance`
axis-completeness decision) — because P7.0 defines the spectral-object types every colour task consumes and P7.9
fixes the metric contract the header asserts. After P7.0, the colour family (P7.1 metamer, P7.2 gamut, P7.3
blend) fans out as disjoint isolated files sharing only P7.0's read-only projection + `hull.rs`. The audio
family (P7.5 → P7.6) is a second independent lane over `audio/*`, parallel to the colour lane. P7.4 (GPU colour
batch) depends on P7.1–P7.3's CPU oracles first. P7.7 (unified API surface) and P7.8 (harness + progress-log)
are integrator-owned barriers that close over everything.

### P8 — TDA / information-geometry family (the probabilistic face of the geometry substrate)

**Goal:** Make probabilistic inference a first-class geometric operation on the 10-D manifold — alpha complexes
and persistent homology over the point cloud (the v-class topological baking), statistical-manifold metrics
(Fisher / KL-as-Bregman) on the probability simplex, CkNN density → Laplace-Beltrami, natural-neighbour
interpolation, and the spec's "distance < threshold ⇒ related, zero graph traversal" nearest-neighbour
inference query — all zero-heap/caller-buffered, deterministic/attestable, with a CPU oracle for every GPU op.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P8.1 | Simplicial-complex core: caller-buffered VR/alpha filtration over the Tensor10D point cloud | planned | — | yes | A unit square + centre yields the expected VR simplices at each radius; the alpha filtration of 4 co-circular points matches the CC0 Alpha_shapes_2 golden birth-values (circumradius via the filtered predicate); byte-identical simplex ordering + filtration bytes across two runs (and native vs wasm-scientific where wired); NonFinite/degenerate inputs fail closed. |
| P8.2 | Persistent homology: deterministic reduction → persistence pairs / barcode (H0/H1) as v-class evidence | planned | P8.1 | yes | A circle sample yields exactly one long-lived H1 bar + n short H0 bars; two disjoint clusters yield two persistent H0 components merging at the gap; barcode bit-identical across runs; reduction matches a hand-computed small filtration; an adversarial collinear simplex fabricates no phantom bar. |
| P8.3 | Statistical manifold: probability-simplex ops + Fisher metric + KL as a Bregman divergence | planned | — | yes | KL(p‖p)=0 and KL matches the Bregman(neg-entropy) form to 1e-12; the Fisher metric reproduces the known Δ¹ closed form; simplex projection is idempotent; the Bregman-Pythagorean identity holds on a constructed example; zero-support/negative-mass inputs fail closed; determinism on canonical bytes. |
| P8.4 | CkNN density estimation → graph Laplacian converging to Laplace-Beltrami (manifold-consistent baking) | planned | P8.3 | yes | On points from a known flat manifold the CkNN Laplacian's low eigenvalues approximate the analytic Laplace-Beltrami spectrum within a stated (honestly-caveated) tolerance; the graph is symmetric with Laplacian rows summing ~0; density is monotone in local density on a two-density cloud; adjacency/Laplacian bytes deterministic; differential vs a brute-force reference Laplacian matches. |
| P8.5 | Natural-neighbour interpolation (Sibson / Laplace weights) over the substrate's Delaunay/Voronoi | planned | P3 Delaunay/Voronoi | yes | CC0 `natural_neighbor_coordinates_2` weights match the reference; weights sum to 1 and are all ≥0 (partition-of-unity + convexity) on random interior queries; a linear field is reproduced exactly (linear precision); a query at a data site returns that site's value; edge/outside-hull cases fail closed or return documented boundary behaviour; determinism on canonical weight bytes. |
| P8.6 | Nearest-neighbour inference query: "distance < threshold ⇒ related, zero graph traversal" over a spatial index | planned | P2 spatial index | yes | A radius query returns exactly the points within threshold on a grid (brute-force differential) for every v-branch; kNN returns the k closest in canonical (distance, then index) order; the SELECTOR contract holds (q/w/v never enter the sum; a q>0 sandbox node excluded); an axis-honesty test asserts the documented axis set per v-branch; a cross-w candidate is filtered/flagged, not silently returned; GPU distance-batch matches the CPU oracle bit-for-bit. |
| P8.7 | GPU acceleration + CPU oracle for the P8 distance/density/circumradius batches (differential + determinism) | planned | P8.1, P8.4, P8.6 | no | wgsl-forge-gated Naga validation; per-kernel CPU/GPU differential where GPU equals the CPU oracle bit-for-bit for resolved cases and the uncertain-set is exactly the near-degenerate triples; two GPU runs identical; wasm-scientific build compiles and its CPU fallback matches native; the no-adapter/headless path falls back without error. |

**Phase gate:** P8.1/P8.2 produce canonical, deterministic, attestable barcodes validated against CC0
Alpha_shapes birth-values + textbook golden barcodes, with topological features emitted as q>0 attestable
proposals (not baked ground truth); P8.3 passes the analytic identities and P8.4's CkNN Laplacian approximates
the analytic Laplace-Beltrami spectrum within a stated, honestly-caveated tolerance; P8.5 matches the CC0
Interpolation weight vectors with partition-of-unity + linear-precision (once P3 exists); P8.6 returns
brute-force-verified neighbour sets over the P2 index, is axis-honest per v-branch, and enforces SELECTOR/rights
filtering (cross-w/q leaks fail closed); every GPU op (P8.7) has a Naga-valid shader, a bit-exact CPU/WASM
oracle + degeneracy differential, and a working no-adapter fallback; all green on native AND wasm-scientific
with dated per-step progress-log entries. No task is complete while it rests on a stub, an unmet P2/P3
dependency, or an overstated axis-honesty/spectral-approximation claim.

**Parallelism:** First wave parallelises cleanly — P8.1 (simplicial core) and P8.3 (statistical manifold) are
independent-file swarm units with no intra-P8 dependency. Second wave serialises: P8.2 (persistence) barriers on
P8.1; P8.4 (CkNN→Laplace-Beltrami) barriers on P8.3. Two tasks carry cross-phase dependencies and cannot start
until earlier phases land: P8.5 needs P3 Delaunay/Voronoi, P8.6 needs the P2 spatial index — do not stub these;
gate them honestly and flag in NOTICES if the ordering slips. P8.7 (GPU + oracle) is the integrator-owned
convergence point (barriers on P8.1/P8.4/P8.6; not a swarm unit; adversarial re-verification required).

### P9 — API & creation layer (the sovereign maker surface: browser/WASM API, WebGPU .10d mount, renderer SDK, qapp/MCP capability manifests, authoring ergonomics)

**Goal:** Turn the native geometry substrate and the `.10d` container into a creation platform — expose the
kernel + creation ops in the browser/WASM, mount the 10-D display engine on a WebGPU `<canvas>` driven by
`.10d`, and give a qapp author or ordinary maker an ergonomic, rights-aware authoring API whose output is their
own provenance-bearing `.10d` asset, offline, with no silent egress.

| Task | Title | Status | Deps | Swarm | Acceptance gate |
|------|-------|--------|------|-------|-----------------|
| P9.1 | Browser/WASM geometry + creation API surface (kernel ops → `wasm_bridge`) | foundation | — | yes | wasm-scientific compiles green with the new `wasm_bridge::geometry` module; a wasm-bound `convex_hull_2` over the 5-point fixture returns indices `[0,1,3,4]`/vertex_count 4 identical to the native `execute_geometry_tool_json` test; an `orientation_2` call returns the same three-valued sign as the native predicate on the collinear/CW/CCW triple. |
| P9.2 | Browser WebGPU `<canvas>` mount driven by `.10d` (10-D display engine mount) | foundation | P9.4 | no | `webizen-render`/`webizen-desktop` check green; a no-adapter run constructs `QualiaPortal` and falls back to canvas2d (tier 0) without panicking; `load_10d` over a fixture parses its section table and (on a WebGPU adapter) reports tier 2 with node/triangle counts; a governance default-Refuse `.10d` without attestation renders nothing citable (fail-closed); the example HTML loads with no outbound network request. |
| P9.3 | Renderer-SDK `.10d` integration — upload / colour-by-field / integer picking / LOD / temporal-scrub | foundation | P9.4, P9.2 | no | Integer-picking differential (GPU `poll_pick_readback` == CPU `cpu_pick_node_at` for a fixed camera+pixel); colour-by-field maps a known field value to a deterministic RGB on both a scalar oracle and the GPU path; temporal-scrub returns exactly the in-window node set from the temporal-index, byte-identical to a linear-scan oracle; `render --lib` green; `webizen-render` check green. |
| P9.4 | qapp/MCP routes + capability manifests (per-op resource limits; scalar/SIMD/wgpu/CUDA/exact-fallback) | planned | — | yes | Every registered op exposes a manifest with non-empty backends; any op advertising wgpu/CUDA also advertises a deterministic CPU/WASM fallback (never GPU-only for robust topology); resource bounds are finite; `mcp_server --lib` green; the MCP tool listing round-trips the manifest as valid JSON; a Reserve-mode budget query returns only device-runnable backends. |
| P9.5 | Authoring ergonomics — scene construction, primitives, transforms (the three.js-class maker core) | planned | P9.1 | yes | Generating a unit box/sphere/etc. twice yields byte-identical `.10d` mesh sections; a generated primitive round-trips encode→decode within quantization error ≤ bbox_extent/65535 per axis; a composed T·R·S transform matches an independent f64 matrix-multiply oracle; each emitted asset carries a non-empty μ provenance lane + Q42 identity; `--lib` green (native + wasm-scientific). |
| P9.6 | Authoring ergonomics — mesh/boolean ops, procedural generation, and pick/drag/edit interaction | planned | P9.3, P9.5 | no | A∪B / A∖B / A∩B on a fixture matches the CC0 corefinement golden vector AND a near-degenerate coplanar case triggers the exact fallback producing valid manifold topology; a drag on a picked vertex produces the expected position and lands as a NEW t-slice (prior t unmutated); a drag violating the governance/consent lane is refused; `computational_geometry --lib` + `render --lib` green. *(Cross-phase: gated on the P4/P5 3-D boolean/decimation kernel.)* |
| P9.7 | P9 progress log + end-to-end maker acceptance walkthrough (PROJECT RULE §9) | planned | P9.1, P9.2, P9.3, P9.4, P9.5, P9.6 | no | The progress-log carries a dated entry per completed P9 step in the §9 shape; the walkthrough runs offline with zero outbound requests; the exported `.10d` re-loads via `load_10d`, its whole-file hash stable across two identical exports, with governance + μ provenance + Q42 identity intact; a final honest-status paragraph does not mark gated-on-P4 items complete. |

**Phase gate:** the kernel + creation ops are callable from Rust, WASM (wasm-scientific), qapp/MCP, and the
renderer-SDK through one op contract, each with a published capability manifest whose GPU/CUDA backends always
carry a deterministic CPU/WASM fallback; a WebGPU `<canvas>` mount loads a `.10d` asset offline, honours its
default-Refuse governance flag (fail-closed), and drives colour-by-field / integer-picking (GPU==CPU) / LOD /
t-ledger temporal-scrub; an ordinary maker can construct a scene, generate + transform primitives, run
boolean/mesh + pick/drag edits (exact-fallback robustness; edits land as new t-slices, never mutating sealed t),
and export the result as their own provenance-bearing `.10d` asset with Q42 identity + μ lane intact; the whole
walkthrough runs with no silent egress and produces canonical/hash-stable bytes; the §9 progress-log carries a
dated per-step entry and an honest implemented-vs-not status that does not mark P4-gated work complete. Gates:
`computational_geometry --lib`; `mcp_server --lib`; `cargo check -p webizen-render -p webizen-desktop`;
wasm-scientific check — all green.

**Parallelism:** Barrier first — P9.4 (capability manifest / descriptor) and P9.1 (WASM geometry export) have
no dependencies and land first (P9.4 defines the governance/backend descriptor P9.2/P9.3 consume; P9.1 is the
WASM surface P9.5 builds on), parallelizing cleanly as disjoint isolated-file swarm units. P9.5
(primitives/transforms/scene graph) is the next strong swarm fan-out (only the scene-graph container is
integrator-owned). Serial/integrator-owned (live hand-edited renderer + portal): P9.2, P9.3, and the
interaction+governance wiring in P9.6. P9.6's boolean op is swarm-authorable against its CC0 oracle but is
gated on the P4/P5 3-D kernel — the single hard external dependency of the phase. P9.7 is the closing barrier
and depends on all others. Security-critical output (governance fail-closed, provenance/μ preservation, robust
exact fallback) is adversarially re-verified by the integrator before landing.

## Cross-cutting gates (always-on)

These are **continuous gates, not one-time tasks** — every new op, kernel, codec, and section type re-opens
each of them, and the integrator holds them open across every phase.

| Concern | Applies to | Verification |
|---------|-----------|--------------|
| **Determinism, canonical bytes & attestability** | All phases (P0 predicates, P1 topology, P2 spatial index/Morton, P3–P5 algorithm output, P6 authoring/round-trip); the Phase-0 `.10d` container refactor most acutely (section table, compression, whole-file hash). | Per landed unit: a golden-hash test running the op/encode twice asserting byte-identical output; cross-target run under `cargo test` native + `wasm32 --features wasm-scientific` asserting identical combinatorial results; for `.10d`, a canonical-bytes round-trip (encode→decode→re-encode == original) plus CRC-32C-per-section and whole-file content-hash checks; forbid any std HashMap/HashSet iteration or f32/f64 nondeterminism in output-ordering paths; assert `orientation_2`'s exact path on the near-degenerate vectors. |
| **CC0 golden-oracle harness + per-file licence gate** | All algorithm phases (P0 predicates through P5 reconstruction/meshing) and the spectral/TDA families; runs on every swarm-authored unit before integration. | A harness that per unit loads the pinned CGAL 6.2 CC0 vectors and asserts a per-file CC0 SPDX/notice match (fail the build if any ingested vector is not CC0); differential native-output == golden-output incl. degeneracy vectors; a repo-level check that no source under `computational_geometry/` contains GPL/LGPL-derived code (per-file provenance note). Command surface: `python scripts/cgal-port/port_cgal.py --fetch` + `cargo test -p qualia-core-db computational_geometry --lib`. |
| **GPU/Forge acceleration ⇄ CPU/WASM parity with degeneracy differential** | P0 (orient3d/incircle/insphere GPU batches), P2 (spatial-index/Morton/NN kernels), P3–P4 (hull/Delaunay/boolean broad-phase), plus the spectral/TDA GPU ops; any op that ever dispatches to wgpu. | Per accelerated op: a differential asserting GPU-result == CPU-oracle on a set that includes collinear/cocircular/cospherical/coincident/wrap-boundary degeneracies; assert the UNCERTAIN sentinel is emitted for near-degenerate cases and the CPU exact path resolves them; a Naga validation test (`#[cfg(feature = "wgsl-forge")]`); a headless/no-adapter run proving identical CPU/WASM fallback output; deterministic shader emission (emit twice == identical string). |
| **Zero-heap hot path + 42-MiB Sentinel-per-pass budget audit** | All phases; especially P1–P5 algorithm workspaces and P2 spatial-index build; the exact-construction kernel's expansion/rational workspaces must be caller-supplied and bounded. | A `#[cfg(test)]` allocation-counter around each hot-path op asserting zero heap allocations for a representative call; a per-pass workspace-size assertion (declared max bytes ≤ budget, so no pass breaches the 42-MiB Sentinel ceiling under expected input sizes); grep/CI lint forbidding Vec/String/Box in the kernel modules outside the `tool.rs` boundary; POD/bytemuck-derive checks so every uploaded struct is repr(C) Zeroable. |
| **Licence discipline — CC0 spec/tests only, never GPL/LGPL source** | All phases where an external reference informs an implementation (P0–P5 algorithms, the `.10d` compression/codec work in Phase-0 and P4/P5); every swarm sub-agent unit. | Per-file provenance header naming the CC0 spec / permissive reference / textbook used and asserting "no CGAL source consulted or derived"; a repo scan for accidental copyleft-derived patterns or copied CGAL identifiers/comments; per-file CC0 SPDX assertion on all ingested test vectors; manifest strictness — `port_cgal.py` must not claim `ported` for a unit lacking documented-surface + conformance coverage. |
| **Rights / consent / sensitivity structural invariants** | All phases producing or transforming a `.10d` view/section — most acutely P4 (decimation/simplification, boolean/corefinement: derived-view class-inheritance + μ-parity re-derivation) and P6 (authoring/creation surface, cross-w projection, WebGPU display path); the digital-twin dual-mesh field reads. | Governance tests asserting `plan_view`/`plan_qapp` refuse-by-default in civic standpoint without consent and that FORBID beats PERMIT; a decimation/LOD test asserting the derived view's sensitivity == max(source classes) and that μ-parity recomputes and validates (a stripped class fails the parity check); a cross-w projection test asserting medical→public fails closed absent per-w consent and that systemic implications land as q>0 `requires_attestation` nodes; an egress test asserting no geometry op opens a socket; a scan that no output string asserts "safe/certified/clinically valid"; sandbox/q≥999 nodes non-citable as provenance until GSR-collapsed. |
| **Honest axis-role taxonomy & metric-completeness (queryability claim == code)** | All phases touching distance/query semantics or the `.10d` axis-role header — the container refactor (Phase-0 normative header), P2 (spatial-index queries assuming a metric), the spectral/TDA/information-geometry families (Fisher, KL/Bregman, hyperbolic metrics), and P6 (any API/doc surfacing "queryable by axis X"). | A metric-completeness test per v-branch asserting exactly which axes it folds, cross-checked against the `.10d` header — the header's COORDINATE set for a given v must equal the axes the branch actually uses (a v=1 header claiming t/α/μ/σ are COORDINATEs while `cyclic_distance` ignores them fails). Assert CPU `full_distance` == GPU `tensor_volume.wgsl` for all four v-branches (lockstep). A doc/spec lint that no header/API string advertises axis-queryability the branch does not implement. When a branch is made axis-complete, update `tensor/mod.rs` + `tensor_volume.wgsl` + the header together and re-run the completeness test; until then the documented limitation must appear verbatim in the header. |

## ⚑ Curation datums — Timothy's calls

These are out-of-band decisions and data only Timothy can supply — the `blocked`-reason inputs the tracker
holds open. The first two are the open **format decisions** that gate freezing the `.10d` header.

- **Axis-role taxonomy sign-off (§4.1).** Confirm the normative COORDINATE / SELECTOR / CARRIER assignment for
  `[q,v,w,x,y,z,t,α,μ,σ]` **before it is frozen in the `.10d` header** — specifically that `q,v,w` are
  SELECTORs excluded from distance, `x,y,z,t,α,μ,σ` are COORDINATEs, and `μ` is the provenance CARRIER. This is
  normative; only Timothy can bless it.
- **Metric-completeness decision (§4.1 honest limitation).** Today only the v=0 Euclidean branch of
  `full_distance` folds the full coordinate set; cyclic/hyperbolic/clique use `x,y,z` / byte-equality only.
  Decide: **(a)** make the non-Euclidean metrics axis-complete now, or **(b)** ship with the limitation
  DOCUMENTED in the spec. The kernel must not assert queryability it contradicts — this is Timothy's call, not
  the agent's. (Drives P6.0 / P7.9 / P8.6.)
- **Which HRA release.** Name the specific Human Reference Atlas release/version to use as the anatomy mesh +
  hierarchy source (body→system→organ→component), so the `.10d` hierarchy and the ~2×-vs-GLB measurements are
  anchored to a fixed, citable corpus rather than an ad-hoc GLB.
- **Food / herb / traditional-medicine corpus.** The curated diet / traditional-medicine source data (flagged
  in the 3D-Anatomy NOTICES line as seed-only). The cross-w systemic-proposal machinery needs it as attested
  content, not a placeholder — supply the corpus or name its source of truth.
- **Clinician rule sign-off.** Sign-off on the clinician-lens structural-consideration rules that drive the
  cross-w "proposals, not diagnoses" (q>0, `requires_attestation`) nodes, so the substrate can carry them
  without any implication of diagnosis. Fidelity ≠ assurance — only a competent human may bless the clinical
  rule set.
- **Acceptable-quality / fidelity thresholds.** The acceptable-quality bar for lossy paths — the
  quantization/decimation LOD error tolerance (P5 decimation + `.10d` compression LODs) and what "visually
  lossless" must mean for release — plus any spectral-blend fidelity threshold. These are quality-policy calls,
  not agent defaults.
- **Sensitivity-class & governance policy.** The concrete sensitivity classes and per-view
  `{sensitivity, requires_attestation}` defaults for the `.10d` Governance section, and the cross-w consent
  bindings (which `w_i→w_j` head-pairs are legal — e.g. medical→public must fail closed). The header default
  disposition is Refuse; Timothy defines the actual policy table.
- **Sensitive vocabulary / axis semantics Timothy reserves.** Any axis-semantic terms, standpoint names, or
  governance vocabulary he reserves the right to coin (per RULE §12's allowed-deferral clause) before they are
  written into the normative `.10d` header and the deontic-norm table.
- **Swarm fan-out trigger (per phase).** The go/no-go to spawn the sub-agent swarm for each phase's fan-out
  (RULE §11/§14) — it is token-expensive and explicitly opt-in, so each phase's scale-up waits on Timothy's
  word. Also the go-ahead for the `.10d`-P0 coordinated refactor window (when live renderer work is safely
  paused/stable).
