# QISP (Immersive SPARQL Profile) — implementation progress log

Honest engineering record for `docs/plans/immersive-sparql-hypermedia-profile.md`.
Per CLAUDE.md §9. Branch `0.0.28`.

---

## 2026-07-13 — Implementable core: Phases 0, 2, 4 + residual R06 builtins

**Status:** the *implementable core* of the plan is landed, verified, and committed.
Phases 6–9 (async jobs, VC-bound policy, materialized views/federation, streaming,
WebRTC, standards incubation) are **not** started — they are genuinely multi-week and
several need external parties (two independent implementations, W3C/OGC liaison); they
are staged honestly, not faked.

### What was built (files + mechanism)

- **Phase 0 — profile artifacts** (`docs/standards/qisp/0.1/`): `profile.md`, ontology
  `qisp.ttl`, typed function catalog `qisp-functions.ttl`, SHACL `qisp-shapes.ttl`, the
  three example `.rq` queries, and 4 ADRs. All Editor's-Draft / provisional IRIs; no
  W3C/OGC status claimed; the §1.1 current-state honesty inventory and the full
  QISP-R01..R33 register are carried through.
- **Phase 2 — typed values + dense-asset registry** (`sparql_library/immersive/`):
  `value.rs` (ImmersiveValueKind / ExecutionClass / ExactnessClass /
  ImmersiveFunctionDescriptor, stable QispError codes), `asset_registry.rs` (bounded
  generation-safe `DenseAssetRegistry`; `DenseAssetRef` carries only validated numeric
  fields — no Rust address; fails closed on stale/forged handles, QISP-R03/R04),
  `profile.rs` (fixed Tensor10D dim order + inline-value validation).
- **Phase 4 — typed function registry + resident tensor predicates + admission**
  (`immersive/functions.rs` + `sparql_filter.rs`): the typed registry for all §4.1
  families (QISP-R05, no untyped `u64->u64` contract), GeoSPARQL deference recorded
  per-entry; real Tensor10D predicates `tensor_distance` / `tensor_within` /
  `tensor_knn_into` over the resident substrate (Phase 4 step 5); QISP admission wired
  into the SPARQL `Function::Custom` dispatch — `qispf:` topological/proximity functions
  execute via the GeoSPARQL engine, `qispf:knn` is rejected inline, QISP-owned mesh/tensor
  predicates return an honest "not yet executable inline" error (never fabricated).
- **QISP-R06 — string/temporal FILTER builtins** (`sparql_ast.rs` + `sparql_filter.rs`
  + `daemon_query.rs`): a `StringSink` (interior-mutable, carried by the Copy
  `TextResolver`) gives value-producing builtins a string-return channel; query-stable
  `now_ms`/`seed` give the temporal/UUID builtins referential transparency (§4.4).
  Implemented: `CONCAT`, `SUBSTR`, `UCASE`, `LCASE`, `ENCODE_FOR_URI`, `STRBEFORE`,
  `STRAFTER`, `COALESCE` (real unbound detection), `NOW` + `YEAR/MONTH/DAY/HOURS/MINUTES/
  SECONDS/TZ/TIMEZONE` (chrono), `UUID`/`STRUUID`/`BNODE`/`IRI`/`URI`.

### Measured results (real)

- Phase 0: all three `.ttl` parse under rdflib — 382 + 586 + 105 triples (valid, not
  eyeballed).
- Phase 2: 25 unit tests green (registry round-trip / tamper / stale / capacity /
  no-address; profile arity+finiteness; descriptor ABI).
- Phase 4: typed-registry + tensor-predicate + admission tests green, incl. the
  end-to-end `test_qisp_custom_dispatch_admission_and_geo_deference` (point-in-polygon
  `qispf:intersects` → true; `qispf:knn` rejected inline; `qispf:volume` honest error).
- R06: 8 new builtin tests green (produced-string round-trip via sink; NOW/YEAR
  query-stable; UUID stable-per-site; COALESCE skips unbound; no-sink/no-clock fail
  closed; residual RAND/lang-tag builtins fail closed).
- **Whole `sparql_library` suite: 292 tests pass, 0 failed.**

### Update 2026-07-13 (later) — float channel + tensor predicate execution

Two documented residuals were closed rather than left as follow-ups:

- **`EvalResult::Float(f64)` channel added.** `RAND` is now a real double in `[0,1)`
  (query-stable, per-occurrence-salted); comparison/arithmetic mix integers and reals
  (exact `Numeric/Numeric` term-hash paths preserved — not routed through `f64`). Adding
  the variant meant dropping the `Eq/Ord/Hash` derives on `EvalResult` (nothing
  sorts/hashes/set-keys it) — verified.
- **Tensor10D predicates execute inline.** `qispf:tensorDistance` (→ `Float`) and
  `qispf:tensorWithin` (→ `Boolean`) now run end-to-end in a FILTER from inline Tensor10D
  literals (ten finite values), through the resident-substrate metric — no longer just
  "admitted". Malformed literals fail closed.

### Honest residual (NOT fabricated — genuine infrastructure gaps)

- `LANG` / `LANGMATCHES` / `STRLANG` / `STRDT` — need a per-term language/datatype tag
  model the engine does not carry. Fail closed with a named error today.
- Phase-4 **execution** over QISP-owned **mesh/volumetric** predicates inline (e.g.
  `qispf:volume`, `qispf:intersectionGeometry`) needs terms to resolve to a
  `DenseAssetRef` → mesh payload (an ingestion path). Those descriptors are registered and
  *admitted*; execution is a later increment. Returns an honest error meanwhile. (Tensor
  predicates no longer fall here — they execute from inline literals, above.)

### ⚑ Where the human (Timothy) is needed

- **Tensor10D axis semantics — RESOLVED 2026-07-13.** Timothy pointed to the canonical
  Draft Standard `docs/manuals/standards/q42-10d-tensor-standard.md` §1.2, which already
  defines all ten axes: `q` = quantum/epistemic context, `v` = topological-class metric
  selector, `w` = manifold/domain index, and the **`alpha`/`mu`/`sigma` "Spectral-Logical
  Payload"** = the EMF-signal parameters (amplitude / modulation-phase / spectral-signature)
  across the *entire* EM spectrum, addressable over time via `t` — colour and sound are only
  perceptual projections (§1.3). The QISP `profile.rs` + `qisp.ttl` are corrected to match
  and cite the standard; **`mu` was fixed Epistemic→Spectral** and the "provisional" flags
  dropped. No open axis question remains.
- **Namespace governance (QISP-D02) — DECIDED 2026-07-13.** Timothy set the namespace
  authority to **`webizen.org`** (not `qualiadb.org`), matching the established house
  convention (`https://webizen.org/q42#`, `ns.webizen.org/q42/…`). All QISP IRIs are now
  `https://webizen.org/immersive/{,function/,datatype/}0.1#` across code, ontology,
  SHACL, examples, and the plan. Only the operational continuity/archival plan for the
  domain remains as ordinary governance — no open decision blocks the core.

### Next step

The next implementable increment is **Phase 3** (planner physical operators +
coarse-to-fine admission) and the **Phase-4 execution path** (resolve `DenseAssetRef` →
Tensor10D/mesh so the QISP-owned predicates run inline). Phases 6–9 remain staged.
