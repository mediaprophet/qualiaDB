# Native capability-ontology & dispatch — ecosystem optimization plan

**Status:** plan of record (2026-07-04). Design direction captured; build deliberately sequenced *after* the
capability substrate matures (see §7 and §9). Nothing here is built yet beyond the pre-existing pieces named
in §4.
**Owner / lane:** Claude (Opus 4.8) — this document. Does **not** touch the live computational-geometry lane
(Devin / GLM-5.2) or its tracker.
**Design substrate it optimizes:** [`native-computational-geometry.md`](native-computational-geometry.md) (the
geometry engine + `.10d` container), [`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md),
[`../manuals/computational-3d-assets-and-digital-twins.md`](../manuals/computational-3d-assets-and-digital-twins.md),
[`native-auditory-language-and-music-intelligence.md`](native-auditory-language-and-music-intelligence.md).
**Homes the raw notes in:** [`advanced-stuff.md`](advanced-stuff.md) (QPU elastic scaler → §5; time-physics /
`t`-axis → §6).

This is the QualiaDB / Webizen architecture. CBOR-LD, HDF5, SQL/XLA/Halide-style query planners, and QPU
runtimes are instruments this work directs and references — never parents of it. The synthesis, the axes, the
capability surface, and the dispatch model are its own.

---

## 0. What this is, and what it is *not*

**What it is.** A single coordinating layer that makes the *whole* QualiaDB capability surface — computational
geometry, the specialized libraries, the spectral/EMF operators, inference, the solver substrate — addressable
through **ontological directives stored in q42**, and executes them through a **determinism-preserving planner**
that compiles those directives down to tight, zero-ontology hot kernels. The thesis (developed and stress-tested
in design conversation, 2026-07-04) is that this yields a *better ecosystem-wide performance outcome* than
direct imperative call-sites — not by making any single call faster, but by exposing the whole intended
computation to global optimization.

**What it is not.**

- It is **not** a rewrite of any capability. The geometry kernel, the specialized libs, the spectral operators
  keep being built exactly as planned in their own documents. This layer sits *above* them and calls them.
- It is **not** a new inner-loop abstraction. The ontology never enters a hot loop (§2, invariant 1). If it did,
  it would destroy the zero-heap contract the whole engine depends on.
- It is **not** started now. Optimizing a capability surface is only worthwhile once enough of that surface
  exists; building a planner over a half-built kernel is premature. This plan is **design-on-record + cheap
  groundwork now, planner build later** (§7 phase gates, §9 trigger).
- It does **not** displace `.10d` or q42. It *raises their value*: content-addressed, ontology-tagged,
  rights-bearing inputs are the precondition for every optimization the planner performs (§3).

---

## 1. The problem — a macro optimization, not a micro one

A direct Rust call into a kernel is near-zero overhead. Routing that same call through a capability ontology
adds indirection — resolve the term, bind inputs, dispatch. **Per single call, that is a tax, not a saving.**
If the story ended there this layer would be a regression.

The win is that **declarative directives expose the whole intended computation to a planner before any of it
runs**, and a planner can do what imperative call-sites structurally cannot:

- **Global fusion & reordering** — see the entire DAG of intended operations, fuse adjacent geometry / spectral
  / compute steps, elide dead ones, batch like ops. (This is why declarative engines out-run hand-written
  imperative loops *at scale*: the declarative form hands the optimizer the graph.)
- **Content-addressed common-subexpression elimination** — because inputs are content-addressed, two directives,
  even from different qapps, asking for the same operation on the same bytes are computed once and memoized.
  Imperative call-sites can't see each other; a planner can.
- **Capability-aware routing** — a directive says *what* it needs ("convex hull, exact predicates,
  deterministic"); the resolver picks *how* — filtered-f64 vs exact-construction kernel, CPU vs GPU vs WASM vs
  QPU, precision, with fallback. Always the optimal implementation for the actual data and device, never a
  compile-time-frozen choice. (This is the QPU elastic scaler of §5, generalized.)
- **Locality batching** — group operations over the same spatial region, pull those quins once via the
  DB-native spatial query, run them batched. High cache-hit, SIMD/GPU-shaped.
- **Rights- & epistemic-driven elision** — the one no external planner has. Because directives target a surface
  that *includes* `v` (rights), `q` (certainty), `w` (domain), the planner prunes work the Webizen VM would
  deny *before* a GPU cycle is spent, and skips sandbox (`q≥999`) contexts (§4).

Honest bound: this is a **macro** optimization, realized across a whole workload, and the "large win at scale"
is a reasoned architectural expectation, **not** a measured number. It only materializes if the two invariants
in §2 hold; get either wrong and the result is slower *and* non-attestable.

---

## 2. The architecture

A computation flows: **directives in q42 → planner → compiled plan → zero-ontology kernels → provenance
receipt.**

- **Capability ontology** — a versioned, content-hashed vocabulary describing each capability: its operation,
  input/output shapes, **determinism class**, device affinity, rights/epistemic preconditions, and a rough cost
  model. Lives as a CBOR-LD context + q42 lexicon terms (§3).
- **Directives** — a computation is a subgraph of directive-quins referencing capability terms + content-
  addressed inputs. SHACL-shaped, `q_hash`-coded.
- **Planner** — consumes the directive DAG, emits a compiled plan: fused, reordered, device-assigned,
  rights-pre-filtered, memoized against a content-addressed plan cache.
- **Compile-down** — the plan runs as tight kernels with the ontology *entirely absent* from execution.
- **Receipt** — plan + input hashes → a WAL provenance receipt, meaningful because the optimizer preserved the
  bits.

Crucially, this layer **does not introduce a new constraint/validation vocabulary**. The *admissibility* facet —
"is this request valid, permitted, rights-gated, and where does it validate?" — is already the SHACL-extensions
subsystem (§4.1), and the capability ontology reuses it wholesale. This layer adds **only** the facet SHACL
structurally cannot express: *execution* metadata (determinism class, device affinity, cost model) and the
*planner* (directive DAG → fused/reordered/memoized compiled plan). SHACL answers "admissible?"; the planner
answers "how to fuse, route, and memoize for throughput?" — two different questions.

### The two load-bearing invariants

Both are easy to get wrong, and either mistake inverts the result:

1. **Coarse granularity, compile *down* to a zero-ontology hot plan.** If an `orient2d` predicate resolved
   through the ontology inside the inner loop, that would put graph/string lookups in the hottest path and break
   the no-`Vec`/`String`/`Box`-in-hot-paths invariant the engine is built on. Dispatch happens **once, at plan
   time**; the compiled plan then runs as pure kernels. Plans are content-addressed and cached (same directive
   DAG → same compiled plan → optimizer runs once, hot plan reused). Blur this boundary and you pay the
   indirection tax on every primitive with none of the amortization.
2. **The optimizer is determinism-preserving, or attestability dies.** This is non-negotiable for this
   architecture. The planner may reorder independent ops and reassign devices, but it may **not** reassociate
   floating-point arithmetic in a way that changes the bits, and geometric *decisions* (predicate signs) stay
   exact regardless of plan. A provenance receipt over a bit-changed result is meaningless; determinism-as-
   attestability must survive the optimizer intact.

---

## 3. Ontology-as-codebook — the mechanism already chosen (CBOR-LD), running at two layers

Qualia already serializes with **CBOR-LD** (preferred), and CBOR-LD's compression *is* "the ontology is the
codebook": it resolves every term against its `@context` and replaces verbose IRIs/property names with compact
integer term codes, then binary-packs with type-specific transforms. The more of a payload is expressible in a
shared vocabulary, the smaller it gets. There is a real term dictionary in tree
([`q42/q42_lexicon.rs`](../../crates/qualia-core-db/src/q42/q42_lexicon.rs)), a CBOR parser
([`sparql_library/parsers/cbor_parser.rs`](../../crates/qualia-core-db/src/sparql_library/parsers/cbor_parser.rs)),
and the VC/credential + vault-manifest layers ride it.

So a **computational-capability ontology becomes a CBOR-LD `@context`**: every capability directive — the
operation, its inputs, its rights preconditions — collapses to integer term codes. Structured directive graphs
(the exact thing a planner consumes) are almost entirely vocabulary + a few scalars, so they compress hard.

The elegant part, already true of the stack, is that this runs at **two layers of one principle**:

| Layer | Mechanism | Property |
|---|---|---|
| At rest / on-wire | CBOR-LD integer term codes against `q42_lexicon.rs` | portable, ontology-compressed, standards-coherent, dedup-by-content-hash |
| In memory / hot path | `q_hash()` — FNV-1a compile-time `u64` URI codes in the zero-copy 48-byte NQuin arena | no runtime string allocation; strings already replaced by compact codes |

Both replace strings with compact codes. A capability ontology plugs into *both* — a CBOR-LD context on disk and
a set of `q_hash` codes in the arena.

**The refinement to insist on:** for the compression to be deterministic and attestable the capability context
must be **canonical, versioned, and pinned by content hash** — writer and reader must resolve identical term
codes. Pinned context → deterministic term codes → hash-stable CBOR-LD bytes → a WAL provenance receipt. If the
context floats, the same directive serializes to different bytes and attestability is lost.

**The honest limit, so the performance claim stays precise:** CBOR-LD's win is **at rest** — storage, I/O,
dedup, portability, semantic query, attestability — **not** the runtime hot path. It is a *decode boundary*: you
deserialize into the zero-copy arena to execute; you do not mmap CBOR-LD bytes as the hot structure. Runtime
speed comes from the arena and the compiled plan. Two layers, two different wins — keep them named separately so
nobody promises a runtime speedup that actually lives in the storage layer, or vice-versa.

### 3.1 Two forms — portable interchange vs local execution — and the streaming/network path

The portability question and the performance question are the same question seen from two ends of one compile
pipeline; the resolution is the classic source-vs-compiled split, made rights-aware:

- **Portable / at-rest / on-wire / streamed form = the CBOR-LD ontological encoding.** Self-describing (terms
  resolve against a pinned `@context`), compact (ontology-as-codebook), canonical (pinned context →
  deterministic bytes → attestable), and *incrementally decodable* (CBOR is binary and length-prefixed — it
  streams). A peer receiving a q42 asset needs only the shared ontology (context, referenced by hash), not your
  VM's opcode ISA. This is already the project's chosen serialization (q42 / vault is CBOR / CBOR-LD only).
- **Local execution form = the compiled SHACL opcodes + the cached plan.** Opcodes run in the SLG VM zero-heap
  and bounded (< 1 ms / constraint set); the compiled plan runs as tight kernels. This form is *not* portable
  (VM-specific) and is never shipped over the wire.

You compile the portable form → the execution form **once**, cached by the source's content hash. So *"which is
more performant for local runtime"* has a precise answer — the **compiled** form is, and the §2-invariant-1 rule
(the ontology never enters the hot loop) guarantees the portable form imposes zero runtime tax. *"Is it better
to have both"* also has a precise answer — **yes, as source and compiled forms of one thing, not two parallel
constraint systems.** Do not hand-maintain two sources of truth; derive the opcode/plan form locally from the
ontological encoding.

**Streaming and network read/write** are where the split pays off:

- **Untrusted ingress → fail-closed admissibility gate first.** An asset streamed from a network source is
  validated by the existing SHACL admissibility gate (§4.1) on the decoded form *before* any execution — Critical
  (identity/consent/safety) violations never degrade. This is the security boundary; it is already zero-heap and
  sub-millisecond.
- **Progressive/partial graphs stay usable.** CBOR's incremental decode + the SHACL extensions'
  `degrade_violations` / `OperationMode` model (non-critical off-grid violations degrade to non-blocking, a
  partial subgraph stays usable) *is* the streamed/partial-graph semantics — already built.
- **Content-address streamed assets by multihash** → integrity, dedup, validate-once-memoize; a re-stream of the
  same bytes hits the cache, and canonical CBOR-LD lets the receiver verify a provenance receipt attached by the
  sender.
- **Amortize the planner only for reuse.** A transient one-shot streamed asset runs the admissibility gate +
  direct execution and skips full plan optimization; a computation that will be reused compiles and caches a
  plan. The planner is a throughput optimizer, not an ingress tax.

Net: the ontological (CBOR-LD) encoding is the right *portable / streamable / attestable* form; the SHACL opcodes
+ cached plan are the right *local execution* form; the content-addressed compile cache is what makes "portable
AND fast" non-contradictory across local, at-rest, and network-streamed use.

---

## 4. What already exists — this is a generalization, not a new invention

The pattern is not foreign to the stack; it exists for *one* capability and this plan lifts it to *all* of them.

- **`orchestrate_inference()`** already gates every LLM call: `validate_intent` → `infer` → `validate_output`
  against the N3Logic Rights Ontology. That **is** ontology-mediated dispatch, with the fail-closed rights
  pre-filter, for inference. §D4 generalizes exactly this to the whole capability surface.
- **`mcp_server.rs`** already exposes the specialized libs + the new `computational_geometry` tool as a callable
  surface — the census D0 starts from.
- **The daemon's `/query`** (SPARQL-style over quins) is already declarative query over the graph; extending it
  from *data* queries to *capability* queries (directives that invoke compute) is the move.
- **`webizen.rs::execute_vm_frame`** is already a frame executor — a sibling of the plan executor.
- **`q42_lexicon.rs`** is already the CBOR-LD term dictionary the capability context extends.

Honest status: at the *ecosystem* level this layer is **unbuilt**. What exists is the inference-only instance and
the substrate pieces above. This plan's job is to make the generalization real *when the surface is ready*.

### 4.1 Relationship to the existing SHACL extensions — reuse, not a parallel layer

The SHACL-extensions subsystem already exists and already owns the *admissibility* facet. It is reused, not
restated:

- `specialized_libs_shacl.rs` + `computational_maths_shacl.rs` + the `*_shacl.rs` family: per-operation
  `Configuration` structs → `to_opcodes()` → SLG-VM validation, backed by `shapes/*.shacl.ttl`. Already validate
  operation allow-lists, input shape/size/precision bounds, and domain compliance (HIPAA, FIPS, physician-review,
  CFL stability, mass/charge balance, …).
- `shacl_extensions/identity.rs`: **local-first shape-target routing** (`ShapeRoute` / `shapes_for_locus` /
  `route_is_local`), **severity degradation** (`OperationMode` / `degrade_violations` / `DegradationOutcome`,
  Critical fails closed), and **VC-gated targets** (`CredentialGate`) — all zero-heap, all compiling to
  `SlgOpcode`.
- `shacl/shacl_compiler.rs` + the SLG / Webizen VM (`SlgOpcode`, `execute_vm_frame`, `validate_intent`): the
  compile-and-execute path `orchestrate_inference` already uses as its gate.

The clean division — no duplication:

| Facet | Owner | Status |
|---|---|---|
| Admissibility: is the request valid / permitted? | SHACL extensions (`*_shacl.rs`) | built |
| Rights/consent gate, VC-gated targets, fail-closed | `shacl_extensions/identity.rs` | built |
| Local-first shape routing + severity degradation | `shacl_extensions/identity.rs` | built |
| Compile shapes → opcodes, run in SLG VM | `shacl/shacl_compiler.rs` + Webizen VM | built |
| **Determinism class, device affinity, cost model** | capability ontology (D0.2) | new, additive |
| **Directive DAG → fused/reordered/memoized plan** | planner (D2) | new, additive |
| **Capability-aware device routing (CPU/GPU/QPU)** | router (D3), *extending* the `route_is_local` / `OperationMode` / `degrade` model to devices | new, built on existing |

So the capability ontology is a thin *dispatch-metadata overlay keyed to the same q42 terms the SHACL shapes
already target* — it references the existing shapes as its precondition/rights gate and adds only what SHACL
cannot express. D1's directive validation *is* the existing SHACL gate; D3.1's local-first routing/degradation
*extends* `shacl_extensions/identity.rs` to devices; D4.1's rights elision *generalizes* the
`orchestrate_inference` gate. The genuinely new code is **D0.2 (metadata) + D2 (planner) only.**

---

## 5. The QPU elastic scaler is an instance of capability-routing (homing `advanced-stuff.md`)

`advanced-stuff.md` works through an elastic architecture that runs fully classically with no QPU and scales up
if QPU resources appear — a `TensorSolver` trait with `ClassicalSolver` / `QuantumSolver` implementations and a
`qpu_allocation_mode` scaler (Level 0 classical baseline / Level 1 sparse-calibration / Level 2 dedicated). Read
through this plan, **that scaler is precisely the capability-aware routing of §1** — route a directive to the
optimal backend by determinism class + device affinity + *availability*, degrading gracefully. It should not be
a bespoke solver-only mechanism; it should be one policy inside the dispatch router.

Constraints that make it safe and honest:

- **The classical baseline is the real path.** No QPU hardware is assumed. Level 0 (local CPU/GPU, deterministic)
  is always available and is the attestable path. QPU support is **spec-reserved**, not a dependency.
- **QPU results enter declared-uncertain.** A QPU returns a probabilistic state; its output is folded back as a
  result with declared uncertainty (`q`/`α`), **never silently promoted to ground truth**. It calibrates or
  augments the classical result; it does not replace the determinism contract on the baseline path.
- **No silent egress.** Routing to a remote/cloud QPU is an outbound action — it obeys the same consent/rights
  gate as any Remote backend (a signed VC from the Principal), and latency-sensitive routing (e.g. over a
  constrained uplink) must degrade to Level 0 rather than block.

---

## 6. Physics-property descriptors relative to the 10D projector (homing `advanced-stuff.md`)

The other thread in `advanced-stuff.md` is the *math of time*: the JILA millimetre-scale gravitational
time-dilation result (Jun Ye, 2022), and the distinction between **proper time** (the invariant interval along a
worldline — objective) and **coordinate time** (an observer's frame — subjective). This is design input for two
places, and it connects back to the earlier session direction that the 10D projector "should support physics,
and we should be able to define properties in the file in relation to it."

- **The `.10d` `t`-axis semantics.** `t` is the temporal / provenance ledger axis. The proper/coordinate
  distinction, and localized (per-region) time behaviour, are candidate refinements to how `t` is defined and
  what property descriptors a `.10d` region may carry. **This is a container-format concern that lives in the
  computational-geometry lane's §4** — it must be a *coordinated* change with Devin, not authored unilaterally
  here. This plan records the consideration and the ask; it does not edit the `.10d` header.
- **Physics properties as ontology-targetable descriptors.** The capability ontology (§2) is the natural place
  to describe physics operators (metric/curvature evaluation, field coupling, the objective↔subjective
  transform) as capabilities a directive can target, and to describe the *property fields* a `.10d` region
  exposes them over. This keeps physics as a first-class, routable, rights-gated capability rather than bespoke
  code — and the QPU scaler of §5 is exactly the elastic execution backend `advanced-stuff.md` asks for.

Honest status: this is **design-stage and needs Timothy's field-theory direction** (§10) — which physics is
in-scope, what the objective measure is per context category, and how far the `t`-axis definition should move.
Nothing is built or frozen here.

---

## 7. Phases & tasks (status vocabulary shared with the CG tracker)

Reuses the status vocabulary from
[`native-computational-geometry-EXECUTION.md`](native-computational-geometry-EXECUTION.md) (planned /
spec-reserved / foundation / implemented / verified / done / blocked / deferred). Everything below is `planned`
or `deferred` today — this is a plan of record, and its build is gated (§9). Per §4.1, **D1/D3/D4 reuse the
existing SHACL extensions** (admissibility, rights-gate, local-first routing, degradation) rather than build a
parallel constraint layer; only **D0.2 (dispatch metadata) and D2 (planner)** are genuinely new code.

| Task | Title | Status | Deps | Acceptance gate |
|------|-------|--------|------|-----------------|
| D0.1 | Capability-surface census | planned | — | A checked-in inventory generated from the *actual* MCP tool set + specialized-libs registry + geometry ops (not hand-listed); each entry names operation, in/out shape, determinism class, device affinity, rights precondition, cost note; a test asserts the census matches the live registry so it can't drift. |
| D0.2 | Capability-ontology vocabulary + pinned context | planned | D0.1 | The vocabulary is a CBOR-LD `@context` + `q42_lexicon` terms, content-hashed and versioned; two encodes of the same directive graph are byte-identical; changing the context version changes the hash; round-trips through `cbor_parser`. |
| D1.1 | Directive representation in q42 (schema + SHACL) | planned | D0.2 | A directive subgraph (capability term + content-addressed inputs) validates against its SHACL shape, rejects a malformed/rights-missing directive fail-closed, and `q_hash`-codes every term; no `Vec`/`String` in the directive-decode hot path. |
| D2.1 | Planner core (DAG → compiled plan) | planned | D1.1 | Given a directive DAG, emits a plan that fuses/reorders **without** changing bit-results (regression-guarded against a direct-call oracle); a determinism test proves identical DAG → bit-identical plan output; the plan carries no ontology references (invariant 1 asserted). |
| D2.2 | Content-addressed plan cache + CSE/memoization | planned | D2.1 | Same directive DAG hits the cache (optimizer runs once); two directives over identical content-addressed inputs compute once; a cache-key collision test and an invalidation-on-input-change test both pass. |
| D3.1 | Capability-aware routing + graceful degradation | planned | D2.1 | A directive routes to the declared-optimal backend by determinism class + device affinity + availability; the no-adapter / no-QPU path degrades to the deterministic Level-0 baseline with identical results; routing decisions are logged and reproducible. |
| D3.2 | QPU elastic scaler as a routing policy (§5) | spec-reserved | D3.1 | The `TensorSolver`-style Level 0/1/2 scaler is one router policy; Level 0 is always the attestable path; QPU output enters declared-uncertain (never auto-promoted); remote-QPU routing obeys the Remote consent/VC gate; no QPU hardware assumed. |
| D4.1 | Rights/epistemic elision (generalize `orchestrate_inference`) | planned | D2.1 | The planner prunes directives the Webizen VM would deny *before* compute and skips `q≥999` sandbox contexts; a derived plan inherits the most-restrictive rights class of its inputs; a rights-tagged directive pair is machine-checked to fail closed. |
| D5.1 | Provenance receipts over compiled plans | planned | D2.1, D4.1 | Plan + input hashes produce a WAL receipt; because the optimizer preserved bits, re-running the plan reproduces the receipt; a deliberately reassociated (bit-changed) plan is rejected as non-attestable. |
| D6.1 | Physics-capability descriptors + `.10d` `t`-axis input (§6) | blocked | D0.2 | **Blocked on Timothy's field-theory direction (§10).** When unblocked: physics operators appear as routable capabilities; the `t`-axis refinement is a *coordinated* change in the CG lane's §4, not authored here. |

**Phase gate (whole layer).** The planner passes its determinism regression against a direct-call oracle
(identical results, bit-for-bit) on a real multi-capability workload; the plan cache demonstrably avoids
recomputation; rights-elision fails closed on a restricted directive; a provenance receipt round-trips through
the WAL and is reproducible; every claim in the honest-status section is measured, not extrapolated; a dated
progress-log entry lands.

---

## 8. The ecosystem task landscape (the clarification)

There are now several plans whose task lists cross-reference each other. This is the single coordinating view.
**Each workstream's own tracker remains authoritative for its tasks — this table does not restate or override
them, and I do not edit another instrument's live tracker.**

| Workstream | Doc(s) | Current status | Lane | Relationship to this layer |
|---|---|---|---|---|
| Computational-geometry substrate + `.10d` container | `native-computational-geometry.md` (design) · `-EXECUTION.md` (tracker) · `-PROJECT.md` (charter) · `-PROGRESS-LOG.md` | **P0 done, P1 done, P2 in progress** (bot on **P2.8**, `.10d` topology sections) | **Devin (GLM-5.2) — live** | The geometric-operations engine + on-disk substrate this layer routes to and writes receipts over. **Authoritative; not edited here.** |
| Visual intelligence + generative 3D | `native-visual-intelligence-and-generative-3d.md` | Proposed; P0–P12, mostly `planned`, capability inventory honest | plan (no live build lane yet) | A capability *consumer*; its Phases 9–11 depend on CG P2/P4/P5 + `.10d` sidecars. A future dispatch client. |
| Computational 3D assets + digital twins | `../manuals/computational-3d-assets-and-digital-twins.md` | Capability manual; priorities 1–10 | manual | Names `.10d` as the compiled geometry/field sidecar; the F/A tier model applies to routed geometry outputs. |
| Auditory language + music intelligence | `native-auditory-language-and-music-intelligence.md` | Companion plan | plan | Spectral geometry (`σ` lane) + shared perception timeline (`t` lane) — spectral capabilities this layer can route. |
| Physics-of-time / QPU-elastic notes | `advanced-stuff.md` (raw capture) | Raw notes, previously no home | — | **Homed here:** QPU scaler → §5 / D3.2; time-physics → §6 / D6.1 (+ `.10d` `t`-axis input, CG-lane coordinated). |
| WellFair / 3D Anatomy Qapp | MASTER-EXECUTION-CHECKLIST + `wellfare-core/anatomy/` | S1–S5.0 done | Claude (Opus 4.8) | The first real end-to-end *consumer* of the whole path (records → geometry → `.10d` → renderer, rights-bounded). |
| **Capability ontology + dispatch (this plan)** | `native-capability-ontology-and-dispatch.md` | **NEW — D0–D6 planned/deferred** | Claude (Opus 4.8) | The ecosystem-optimization layer *above* the capability substrate. |
| Human-centric identity & biometric privacy and autonomy | `human-centric-identity-and-biometric-rights.md` | Considerations doc; nothing built | Claude (Opus 4.8) | Cross-cutting rights/identity concern informing CV, computational geometry, **and** this layer; injects the D0.2/D2.2/D4/D5 improvements (see §12). |

**Dependency ordering across workstreams.** This layer is deliberately **late**: it optimizes a capability
surface, so it is most valuable once several capabilities exist and have a stable ABI. The honest sequence:

1. **Now:** CG P2→P6 lands (Devin); WellFair consumes the substrate directly (imperative calls) — no dispatch
   layer needed for correctness.
2. **Cheap groundwork, anytime:** D0.1/D0.2 (census + ontology vocabulary) are a paper/vocabulary exercise over
   what *already* exists — low cost, no dependency on the planner, safe to do opportunistically. They also make
   the visual/auditory plans' capability inventories machine-checkable.
3. **Planner build (D2+):** gated on (a) enough of the capability surface existing to be worth optimizing — the
   trigger is "≥2 mature capability families with stable ABI beyond inference," e.g. geometry P1–P4 done + one
   of spectral/vision — and (b) Timothy's explicit go. Building the planner earlier is premature optimization.
4. **D6 (physics):** blocked on Timothy's field-theory direction independently of the above.

---

## 9. Honest status & sequencing trigger (2026-07-04)

- **Built:** nothing in this layer beyond the pre-existing pieces in §4 (the inference-only
  `orchestrate_inference` instance, the MCP surface, `q42_lexicon`, the daemon `/query`, `execute_vm_frame`).
- **This document:** design-on-record. It captures an architecture, homes `advanced-stuff.md`, and clarifies the
  cross-workstream task map. It is a **living** plan (Timothy directs incrementally) — expect refinement.
- **Deliberately deferred:** the planner build (D2+), per §8's sequencing. **Trigger to resume:** ≥2 mature
  capability families with stable ABI beyond inference **and** Timothy's go. Recording this as a deferral, not
  neglect: building a global optimizer over a half-built surface would be premature and unmeasurable.
- **Cheap-now, low-risk:** D0.1/D0.2 (census + pinned capability context) can start independently and pay off
  immediately as machine-checkable capability inventories for the visual/auditory plans.

## 10. ⚑ Where I need the human

Framed as concrete asks, answerable when you reach them (not gates on other work):

1. **Sequencing go/no-go.** Confirm the §8 order — CG substrate first, D0 groundwork opportunistically, planner
   build deferred to the named trigger — or pull the planner earlier if you want the optimization surface sooner.
2. **QPU scope (D3.2).** Is the QPU elastic scaler in-scope for this layer now (as spec-reserved routing with a
   classical baseline), or parked entirely until QPU access is real?
3. **Physics direction (D6 / §6).** The field-theory calls only you can make: which physics is in-scope
   (metric/curvature, EMF coupling, objective↔subjective time), what the objective measure is per context
   category, and how far the `.10d` `t`-axis definition should move (proper vs coordinate time, per-region
   dilation). This also sets the coordination with Devin's `.10d` §4.
4. **`advanced-stuff.md` disposition.** Confirm it's fine to treat that file as *raw notes now homed here* (QPU →
   §5, time-physics → §6) — I can leave it as-is (source capture) or trim it to a pointer; your call.

## 11. Progress log

Per PROJECT RULE §9, dated per-step entries for any build work go in a named log. Until the planner build is
triggered, this document + the CG progress log carry the record; a dedicated
`native-capability-ontology-and-dispatch-PROGRESS-LOG.md` starts with the first D-task that lands code.

---

## 12. Motivating use-case & cross-cutting concern: human-centric identity & biometric privacy and autonomy

The rights-gated capability routing in this plan was developed against a concrete, high-stakes use-case —
human-centric identity and biometric privacy and autonomy — which grew into a first-class, cross-cutting concern informing
the CV, computational-geometry, and this dispatch layer. It now lives in its own document:
[`human-centric-identity-and-biometric-rights.md`](human-centric-identity-and-biometric-rights.md).

**In one paragraph:** a DID is an *identifier*, not an identity; identity is a never-collapsing probabilistic
*fabric* over many identifiers + claims (`identity.rs`'s `DefinitiveCollapse` rejection). Biometric functions
should run *at the subject's locus* under VC + context-clause gates, returning a *scoped answer* (ideally a
ZK-provable predicate) rather than an extracted template — the data-subject as principal owner, not the
institution. Biometrics *are* geometry + spectral capabilities (so: reuse the substrate, don't silo), the
exact-predicate ladder makes them ZK-provable, and sensing is bi-directional so the person gets an actionable
counter-record. Harder cases (authority override, witness protection, personhood, spatiotemporal correlation,
person-controlled biometric unlock) and the honest limits are in that document.

**What it injects into this layer's D-tasks** (the dispatch-specific improvements — kept here because they modify
the plan of record):

- **D0.2** — add a **locus / control** axis (whose domain a capability may run in / over, not just which
  device; `route_is_local` is the seam) and a **disclosure tier** (boolean / ZK-predicate / scoped-attribute /
  full; return the least the requesting VC is entitled to).
- **D2.2** — the plan-cache key includes a **consent epoch**, so memoization respects revocation.
- **D4.1 / D5.1** — the rights gate + provenance receipts **cite the authorizing clause** (deontic rule + VC +
  context), which also holds the *authority* accountable (an override with no receipt is a detectable violation);
  a derived plan inherits the **most-restrictive sensitivity** of its inputs.

The curation datums it surfaces (identifier-family scope, obligation precedence, personhood vocabulary,
movement-log protection guarantees, biometric-unlock policy) are Timothy's — listed in that document's §11.
