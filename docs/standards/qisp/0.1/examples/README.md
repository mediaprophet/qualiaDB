# QISP 0.1 example queries

**Status: Editor's Draft — provisional, NOT a W3C or OGC standard, no compatibility promise.**

These three queries are copied faithfully from the QISP plan §5. They are illustrative of the
draft syntax and semantics only; the extension functions, `VERSION "1.2"` announcement, and RDF 1.2
triple-term syntax they use are experimental and depend on the SPARQL/RDF 1.2 draft snapshots pinned
in Phase 0. A conforming implementation does not yet exist.

| File | Demonstrates | Conformance classes exercised | Requirements exercised |
|---|---|---|---|
| `01-synchronous-predicate.rq` | A hot, index-pruned exact `intersects` predicate in `FILTER` | `qisp:Sparql12Query`, `qisp:DenseAssetReferences`, `qisp:SpatialPredicates` | QISP-R01, R03, R05, R06, R07, R09 |
| `02-constructive-geometry.rq` | Bounded constructive geometry returning a query-scoped transient asset with PROV-O lineage and a unit-qualified volume | `qisp:Sparql12Query`, `qisp:DenseAssetReferences`, `qisp:ConstructiveGeometry` | QISP-R05, R06, R08, R10, R12 |
| `03-credential-bound-relation.rq` | A verified, scoped VC decision linked to an occupancy triple term via `rdf:reifies` | `qisp:CoreRdf12`, `qisp:CredentialBoundRelations` | QISP-R02, R05, R13 |

## 01 — synchronous predicate (§5.1)

Selects agents whose active spatial asset intersects a spatial zone. The `qispf:intersects` call is
a `HotZeroHeap` predicate that runs synchronously after coarse BVH/kd-tree filtering (QISP-R07,
R09). Exactness is an explicit final argument (`qisp:Exact`), never an invisible server preference
(QISP-R05/§4.4). The ordinary `SELECT` remains SPARQL-Protocol conformant (QISP-R01). The dense
assets are referenced, never triplified (QISP-R03).

## 02 — constructive geometry (§5.2)

A `CONSTRUCT` that computes an intersection solid in a bounded, cancellable arena and returns a
**query-scoped transient URI** (`qispf:transientResource`) rather than a copied mesh literal
(QISP-R08). The result carries `prov:wasDerivedFrom` lineage and a QUDT-unit-qualified volume
(`unit:M3`) so the measurement is never a bare undocumented float (QISP-R05/§4.4). The result is
transient and expires (`qisp:expiresAt`); persistence would be a separate governed operation
(QISP-R12). Expression functions are snapshot-pure and safely repeatable (QISP-R06).

## 03 — credential-bound relation (§5.3)

Uses an RDF 1.2 **triple term** `<<( ?agent qisp:occupies ?volume )>>` and attaches metadata through
a reifier with `rdf:reifies` (QISP-R02) — not the legacy RDF-star annotation syntax. The
`qisp:AuthorizationDecision` is produced by a separate verification computation that checks a VC/VP
against policy; only minimum decision metadata (decision, proof digest, expiry) is exposed, never the
holder's raw proof or unrelated claims (QISP-R13/§8). Linkage of a VC to a triple is **not**
authorization — the decision must be independently verified (§2.2 point 3).

### Known faithful-copy caveat

As written in the plan, query 03 references `qispf:reifierFor(...)` but declares no `qispf:` prefix
(the plan §5.3 omits it). The query is reproduced verbatim to stay faithful to the source; an
implementation running it would need the `qispf:` prefix declaration added
(`PREFIX qispf: <https://standards.qualiadb.org/immersive/function/0.1#>`).
