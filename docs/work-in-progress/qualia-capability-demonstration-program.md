# QualiaDB Capability Demonstration and Upstream Completion Brief

Status: **development-agent handoff**, 2026-10-03. This is an upstream
QualiaDB programme. Rolling Commons is a demanding reference application
that exercises QualiaDB; it is not the reason to hard-code game rules into
the engine. A capability is complete when it is reusable, documented,
versioned, tested and demonstrated in the full Qualia WASM profile. The
game then consumes it and reports any further gap back upstream.

## Mission and scope

Deliver a coherent QualiaDB platform for mutable semantic applications,
spatial worlds, rich media, authored behaviour and offline browser use.
The RTS game should visibly demonstrate Q42, `.10d`, HCF/HMC, VibeScript,
Qualia rendering, computational geometry, geospatial processing, Qualia
Audio, persistence and optional P64 inference. Improvements belong in the
owning QualiaDB crate as **general capabilities**. Product-specific assets,
scenario rules, prices, characters and UI composition remain in the game.

The full `qualia-core-db/wasm-full` capability profile is the browser target.
Do not route game-required features through `wasm-webcivics`, rename the
Qualia package as WebCivics, or alter either project's licence. A game need
that reveals a platform gap is an upstream work item; it is not a reason to
introduce another renderer, database, asset format, pathfinder, audio engine,
scripting runtime or inference gateway inside the game.

## Current local checkpoint: verify claims rather than assume completion

At inspected base commit `726f95d7`, the checkout has **uncommitted work**
in `q42/journal.rs`, `simulation/fixed_tick.rs`, bundle tests, WASM bridges,
WAL, build scripts and rendering. These are valuable candidate
implementations; their presence or unit tests do not yet establish a
reproducible release. Reinspect the tree before work because another agent
is actively checking it. Record the actual tested commit and working-tree
digest in every receipt; do not call the base commit a pin for uncommitted
changes.

One concrete integration distinction: `WasmQ42Session` currently wraps
`MutableQ42Session<Cursor<Vec<u8>>>` and exports the bytes. That is a
memory-backed journal interface, not proof that OPFS writes, flush, restart,
quota failure and recovery work. The raw Quin WAL's WASM path likewise
holds a `Vec<u8>` in memory. Reuse both where appropriate, but require a
durable browser storage adapter before claiming Q42 persistence complete.
Similarly, `FixedTickWorld` is a generic candidate kernel; prove command
ordering, permission/rule integration, journal receipts, cross-target
replay and scale before claiming a complete multi-agent application runtime.

The existing `crates/qualia-audio/` is substantial. The audio task is to
select and expose existing production, effects, event, media and spatial
audio capabilities in the full browser profile, then fill measured gaps.
Do not commission a parallel game-only audio library.

## Platform-wide definition of done

Every work packet below must satisfy **all applicable** conditions:

1. **Generic contract:** public, versioned API with domain-neutral names,
   typed inputs/results, explicit units and error semantics. A game term
   may appear in a fixture, never in a general kernel or wire format.
2. **One authority:** Q42/Qualia validation and simulation own facts and
   mutations. Render, audio, Vibe and P64 consume projections or submit
   typed proposals; they cannot silently mutate authoritative state.
3. **Format integrity:** canonical bytes, version negotiation, explicit
   digests/provenance, fail-closed malformed/unknown required features,
   migration policy and round-trip conformance. Q42 `NQuin` remains six
   `u64` fields (48 bytes) with five-field parity. Do not call an entire
   transaction a 48-byte record.
4. **Target parity:** relevant native and `wasm32-unknown-unknown` builds,
   with an actual browser proof through `wasm-full`. A native-only API or
   memory-backed WASM stub is incomplete for an offline browser claim.
5. **Bounded execution:** follow `AGENTS.md`: zero heap in hot kernels,
   caller-owned buffers, bounded cold construction through workspace
   budgets, deterministic order and the 42 MB Sentinel constraints where
   applicable. Record CPU, memory, GPU and package-size measurements.
6. **Rights and privacy:** preserve authored content licences and
   provenance separately from QualiaDB's licence. Private Q42 sessions
   default to restricted/Sanctuary publication behaviour. No public
   redistribution or sensitive context exposure is inferred from a file
   extension or hash.
7. **Two consumers:** prove the public capability in Rolling Commons **and**
   at least one non-game Qualia reference fixture (for example, a POET
   workspace, Studio scene, graph editor or spatial analysis app). The
   second fixture must exercise the same public API without game-specific
   branches. A capability can be upstream-verified before game integration,
   but it is not platform-complete without this reuse proof.
8. **Receipts:** focused unit and negative tests, conformance vectors,
   native/WASM/browser integration, a reproducible build pin, and concise
   documentation. Add the exact test command, result and limitation to the
   capability ledger rather than a blanket "implemented" assertion.

## Work packages and reusable API outcomes

| Stream / existing QG IDs | Generic QualiaDB outcome | Minimum proof before platform-complete |
|---|---|---|
| **A. Build and profile baseline** | Reproducible native and `wasm-full` artifacts, fixed toolchain/build inputs, feature manifest and package provenance | Clean checkout builds on declared CI hosts; browser loads the full profile; tested commit and tool versions recorded; WebCivics is not substituted |
| **B. Mutable Q42 and storage** (QG-05, QG-15) | Versioned mutable session over sealed `.q42` checkpoint and an accepted journal profile; transactional add/remove/lexicon, validation, query, checkpoint, migration, OPFS/native adapters | 1,000 mixed transactions, torn-write recovery, wrong-base/pack rejection, restart persistence and identical native/WASM graph/event digests; a non-game editor uses the same session |
| **C. Deterministic application simulation** (QG-09) | Fixed-tick command scheduler and reducer interface, seeded randomness, typed receipts, bounded queues, pause/speed and replay; no game economy baked in | Mixed commands and rejected actions replay identically native/WASM; state and journal receipts agree; second consumer runs a non-game workflow |
| **D. Canonical content container** (QG-01, QG-14) | One versioned HMC producer/reader and manifest contract for sealed mixed content, with explicit HCF/document relation and asset dependencies | Native/browser pack round trip for `.10d`, Q42, HCF and audio; byte-identical extraction; tamper, rights, missing dependency and incompatible-version failures |
| **E. Geometry and `.10d` assets** (QG-02, QG-03, QG-13) | Reproducible source/recipe/import → validated mesh/material/rig/motion → `.10d` compiler and browser loader; Q42 semantic identity/digest binding | Two distinct source assets and one animated rig compile deterministically, survive pack/reload/replay and semantic picking; malformed topology/clip/rights fail |
| **F. Geospatial and navigation** (QG-07, QG-08 conditional, QG-11, QG-17) | Terrain transforms, tile/road/placement surfaces, collision/spatial indexes and deterministic pathfinding over Qualia geometry; governed source acquisition if on-demand mode is chosen | Original offline sector with seams, slope/footprint rejection and 100-agent changing-route fixture; data-shaped source includes licence, CRS, transform and privacy evidence |
| **G. Rendering and RTS interaction** (QG-10, QG-12) | QualiaPortal/Webizen camera, world hit test, semantic selection sets, group commands, minimap/overlays, scene graph, instancing/culling/LOD/material/lighting/effects | Representative busy scene meets declared browser frame/memory/load budgets; pick IDs survive LOD and resize; same entities/actions reachable on Qualia-owned low-spec view |
| **H. Qualia Audio integration** (QG-18 audio) | Expose existing Qualia Audio production/media/event/spatial/sonic capabilities via governed browser output and content packs; captions, buses and replay cue policy | One complete scene has original ambient, command, construction and outage audio from Qualia APIs; mute/captions/reduced-sensory mode and event-once replay work; non-game audio fixture uses same API |
| **I. VibeScript host** (QG-04, QG-16) | Discoverable capability catalog for Q42 query, preview, typed proposal, rule receipt and authoring diagnostics; gas/effect/authority scopes | A Vibe cell reads live state and proposes an action; direct mutation and undeclared capability are denied; same host pattern works in game and POET/Studio |
| **J. P64 inference** (QG-06, conditional) | Model/version/content boundary with graph-scoped retrieval, bounded context, typed proposals and offline no-model path | One local browser call reports model/memory/latency and validated proposal; disabling P64 leaves authored applications functional |
| **K. Browser UX, storage and accessibility** (QG-18 input) | Versioned input actions, keyboard/focus/announcement contract, OPFS recovery, offline loading and accessible presentation surfaces | Complete essential interaction without mouse or sound; storage full/evicted/corrupt scenarios recover visibly; settings and saves survive restart where supported |

**Conditional does not mean replaceable.** P64 and on-demand geography may be
deferred by product decisions, but any chosen feature must still use the
Qualia-owned path. Existing rich audio, rendering, geometry or Q42 code
should be extended and exposed, not rewritten because a game wrapper is
easier.

## Detailed instructions at critical seams

### Mutable state: checkpoint, journal and browser durability

Use the Q42 format family; see `docs/work-in-progress/q42-mutable-journal-game-proposal.md`.
Audit the new `q42/journal.rs` against the existing WAL, Q42 volume writer,
publication classifier and graph API. Decide one authoritative transaction
path and document how add/remove operations, lexicon bindings, commit
records, checkpoint generations and rule receipts map to it. Do not retain
two contradictory write paths without an explicit migration contract.

Supply a real Qualia OPFS adapter or equivalent durable browser backing for
the WASM session. An exported in-memory byte vector is useful for tests but
does not satisfy save/reload after tab/browser restart. Test short writes,
flush errors, quota, storage eviction, locks and checkpoint interruption.
The game must be able to replace its temporary N3 text/event-line save with
the public Q42 session without custom serialization.

### Simulation: generic kernel, policy and command order

Audit `simulation/fixed_tick.rs` with adversarial fixtures. Define one
canonical ordering of simultaneous commands, including ties; reordering a
queue by swap-remove must not accidentally change execution order. A
rejected command produces a receipt and no state change. Queue/output
capacity exhaustion is explicit, with no silent dropped receipt. Bind the
kernel to Q42 transactions, role/policy checks, N3/SHACL/deontic rule
evaluation and journal commit. Keep game-specific verbs and tuning in
authored schemas/content, not in fixed-tick core code.

### Assets and formats: source ownership and versioned handoff

Reconcile the core HMC bundle and any other `.hmc` producer before declaring
interchange. Make a typed pack manifest for entry kind, source/compiled
digest, rights, dependencies and format version. `.10d` holds spatial and
animated asset data; Q42 holds semantic facts and stable asset references;
HCF holds authored hypermedia; HMC distributes immutable content; P64 holds
optional model weights. No format absorbs another just to simplify a game
loader. The game-owned source catalog is a blockout fixture, not a finished
asset pipeline.

### Presentation, audio and Vibe authority

Bind renderer picking to stable Q42 identity rather than mesh-array index.
Maintain full Qualia GPU path and a Qualia-owned low-spec path. Use existing
Qualia Audio for production and playback; add missing browser bindings or
content hooks upstream. VibeScript may query, author and propose under a
capability manifest; it never bypasses command validation or mutates Q42
directly. The browser's JavaScript remains an input/DOM host bridge, not a
second engine.

## Evidence matrix for each capability

For each row in the work-package table, the development agent records:

| Field | Required content |
|---|---|
| Owner | Qualia crate/module and public entry point, plus person/agent responsible |
| Version | tested commit; feature profile; API/format version; migration compatibility |
| Status | candidate, partial, upstream verified, browser verified, game integrated, platform complete, or blocked with exact reason |
| Contract | types, units, bounds, privacy/licence class, error behaviour, authority boundary |
| Tests | native, WASM, browser, malformed/failure, replay/conformance, zero-heap or bounded-memory proof as relevant |
| Measurement | declared device/browser baseline, p50/p95 time, memory high-water, bundle/pack size and load time |
| Consumers | game fixture and independent non-game fixture using the same public API |
| Handoff | game dependency revision, asset/schema digest, sample command/pack, integration receipt and known limits |

Test counts or a green unit suite alone are not sufficient. A statement
such as "QG-15 complete" should identify the exact browser restart fixture
and storage adapter, not just `Q42Journal::recover_transactions` passing
against `Cursor<Vec<u8>>`.

## Practical execution order

1. **Freeze evidence:** snapshot current status/diff, coordinate with the
   active checker, and select a tested revision. Review new journal,
   simulation and bundle code for format collisions, genericity and
   implementation/test mismatches. Avoid overwriting concurrent work.
2. **Secure the foundations:** CI/toolchain, `wasm-full`, HMC producer,
   Q42 mutable transaction path and actual OPFS durability. Close native
   and browser conformance before expanding game wrappers.
3. **Connect authority:** fixed-tick commands → rules/policy → Q42 delta
   → journal commit → replayable projection. Prove both game and non-game
   consumers against this seam.
4. **Complete content flow:** geometry/terrain/material/animation source
   → `.10d` and Q42 manifest → HMC → Portal semantic scene. Build one
   complete original asset family before high-volume art production.
5. **Complete experience flow:** Qualia renderer/input, Qualia Audio,
   VibeScript, accessibility and offline recovery. Measure on declared
   browser/device baselines.
6. **Conditional extras:** P64 and live geography after their policy,
   privacy, memory and no-model/offline contracts pass.
7. **Return integration receipts:** pin the verified Qualia revision in
   Rolling Commons, run the game playthrough, update its capability ledger,
   and send any newly found generic gap back to this upstream programme.

## Explicit non-goals

- No game-specific API names, scenario entities, prices or moral scoring in
  Qualia core. A simulation profile may be generic; a Rolling Commons branch
  inside the format or engine is not.
- No capability claim based solely on documentation, compilation, in-memory
  tests or a native-only path when the browser game needs durability.
- No duplicate engine under the game, no WebCivics substitution, and no
  licence conflation between Qualia binaries and game content.
- No public export of player journals or sensitive Q42 data by default.

The finished platform should be demonstrably more capable because the game
found its limits, while remaining directly useful to other QualiaDB apps.
