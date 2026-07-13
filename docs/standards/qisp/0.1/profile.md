# Qualia Immersive SPARQL Profile (QISP) — 0.1

> ## Status banner
>
> **Editor's Draft — provisional, NOT a W3C or OGC standard, no compatibility promise.**
>
> This document, its ontology, shapes, function catalog, and examples are an **Editor's Draft** at
> maturity level 0.1. The vocabulary and semantics can change without notice; the IRIs are
> **provisional** and versioned. Every term is `experimental` status (§ maturity governance). Nothing
> here is endorsed by, submitted to, or conformant with any W3C or OGC standard. QISP is a *profile
> and extension vocabulary layered over* RDF 1.2 / SPARQL 1.2, GeoSPARQL 1.1, PROV-O, OWL-Time, QUDT,
> and the W3C VC Data Model 2.0 — it reuses those, and claims none of their status. No implementation
> is claimed complete: capabilities marked *Partial* or *absent* in the current-state inventory below
> are genuinely partial or absent.

**Working name:** Qualia Immersive SPARQL Profile (QISP) — a working name only, pending
search/community feedback (decision QISP-D01).
**Version / maturity:** 0.1 / Editor's Draft. **Date:** 2026-07-13.
**Source of truth:** `docs/plans/immersive-sparql-hypermedia-profile.md` (internal review candidate v0.2).

---

## 1. Abstract

QISP exposes QualiaDB's spatial, temporal, mesh, Tensor10D, provenance, and authorization
capabilities through an **interoperable RDF/SPARQL surface** — without encoding dense numerical
payloads as triples and without weakening the engine's deterministic, bounded, zero-copy design.

The core move: **RDF names and explains the world; compact content-addressed assets carry its dense
form; bounded native kernels compute over it; provenance and policy make the result accountable.**
Meshes, `.10d` sections, tensor buffers, trajectories, and BVHs remain compact binary assets that RDF
describes and links rather than triplifies. Stable function IRIs map to bounded native geometry/tensor
kernels; lightweight predicates run synchronously, constructive operations use a query-scoped result
arena with explicit budgets, and long-running work is submitted to an advertised (never disguised)
asynchronous job protocol. The renderer is a consumer and accelerator, **not** the semantic source of
truth: exact geometry comes from the computational-geometry library, and any renderer/GPU
approximation must be explicitly labelled and checked against a deterministic CPU oracle.

This is an incubation profile. It is designed so that a non-QISP RDF client still gets useful,
honest results, and so that a QISP endpoint never claims a capability it has not implemented.

---

## 2. Four-layer architecture (§0)

QISP is built as a **profile and extension vocabulary over RDF 1.2 / SPARQL 1.2, not a forked query
language**, in four layers:

1. **Standards-compatible semantic layer** — ordinary RDF terms, RDF 1.2 triple terms, SPARQL query
   forms, GeoSPARQL where it applies, PROV-O, OWL-Time, QUDT, VC Data Model 2.0, DIDs, and standard
   SPARQL result negotiation.
2. **Content-addressed dense asset layer** — meshes, `.10d` sections, tensor buffers, trajectories,
   BVHs, and derived geometry stay compact binary assets. RDF describes and links them; it does not
   triplify every vertex or tensor cell.
3. **Typed computational extension layer** — stable function IRIs map to bounded native
   geometry/tensor kernels. Lightweight predicates can run synchronously; constructive operations use
   a query-scoped result arena and explicit resource budgets.
4. **Optional asynchronous hypermedia layer** — long-running work is submitted to a job resource with
   status, cancellation, provenance, and result links. This is an advertised QISP protocol extension,
   **not** falsely presented as standard SPARQL Protocol behavior.

The renderer is a consumer/accelerator, not the semantic source of truth. Exact geometry comes from
the computational-geometry library; renderer/GPU approximations are explicitly labelled and checked
against deterministic CPU oracles where correctness matters.

---

## 3. Conformance classes (§2.3)

Every endpoint advertises one or more **independent** conformance classes. An implementation **must
not** claim the parent profile merely because one extension function is registered; it publishes a
machine-readable capability graph *and* a human-readable profile.

| Conformance class | One-line definition |
|---|---|
| `qisp:CoreRdf12` | RDF 1.2 triple terms/reifiers with a version-pinned model and legacy RDF-star migration. |
| `qisp:Sparql12Query` | SPARQL 1.2 query surface incl. version announcement and triple terms (draft; distinct from stable 1.1). |
| `qisp:GeoSparql11` | OGC GeoSPARQL 1.1 features, CRS-aware literals, topological relations, and `geof:` functions — reused, not redefined. |
| `qisp:DenseAssetReferences` | Content-addressed dense assets described/linked by RDF with validated, generation-safe references. |
| `qisp:SpatialPredicates` | Hot, caller-buffered topological/proximity/visibility/measurement predicates after coarse index filtering. |
| `qisp:ConstructiveGeometry` | Cold, byte/work/deadline-bounded, cancellable constructions returning query-scoped geometry references. |
| `qisp:Tensor10D` | The Qualia Tensor10D profile and its distance/radius/kNN/slice operators over the resident substrate. |
| `qisp:AsyncJobs` | An advertised, explicit, authenticated, monotonic, expiring job protocol — never returned in place of SPARQL. |
| `qisp:CredentialBoundRelations` | Verified, scoped VC decisions linked to triple terms via `rdf:reifies`, exposing minimum metadata. |
| `qisp:MaterializedSpatialViews` | Dependency-keyed, policy-partitioned, event-invalidated derived spatial relations (optimization only). |
| `qisp:StreamingSparql` | RSP-QL-aligned continuous/windowed queries with incremental deltas (draft; §15d). *Not implemented at 0.1.* |
| `qisp:WebRtcTransport` | Browser peer-to-peer transport binding carrying the same semantics over WebRTC data channels (§15e). *Not implemented at 0.1.* |

---

## 4. Maturity-label governance (§2.4)

Four maturity labels govern every published profile:

1. **Editor's Draft** — vocabulary and semantics can change; **no compatibility promise**. *(QISP is
   here at 0.1.)*
2. **Implementer Draft** — IRIs are stable within `0.x`; changes require migration notes.
3. **Candidate Profile** — feature-frozen, two independent implementations, complete conformance suite.
4. **Stable Profile** — permanent versioned IRIs, published implementation report, documented errata
   and deprecation process.

Each published profile records: dated upstream standard versions and test-suite commits; namespace
owner and continuity plan; semantic-versioning and deprecation policy; IRI persistence + HTTPS content
negotiation + archived copies; a patent/licensing statement for text, ontology, schemas, and tests;
and a change log with machine-readable term status (`experimental`, `stable`, `deprecated`). **Terms
are never silently repurposed** — a semantic change gets a new term or a new namespace version. At 0.1
every term is `experimental`, and open governance decisions (namespace ownership/continuity — QISP-D02,
dated snapshot pinning — QISP-D03, licensing) are **not yet resolved**.

---

## 5. Namespaces (§3.1)

Separate ontology, function, and datatype namespaces keep predicates distinct from executable
functions:

```text
qisp:  https://standards.qualiadb.org/immersive/0.1#
qispf: https://standards.qualiadb.org/immersive/function/0.1#
qispd: https://standards.qualiadb.org/immersive/datatype/0.1#
```

These are **provisional versioned IRIs**. Before any public standardization the namespaces must
publish HTTPS content negotiation for Turtle, JSON-LD, RDF/XML if required, and HTML documentation.
`1.0` must not be minted until the interoperability suite has two independent consumers.

---

## 6. Current-state inventory (§1.1)

**This table is a Phase-0 gate requirement (QISP-R05, QISP-R18) and is reproduced faithfully from the
plan's honesty audit.** It must not confuse a type or module with an end-to-end capability. Before
Phase 0 exits it is to become a versioned implementation inventory with links to tests; **any
`Partial` capability remains visibly partial in the service description** and no capability below is
upgraded to "done".

| Surface | Current status to assume | Gap QISP must close |
|---|---|---|
| SPARQL algebra | Broad fixed-capacity AST/planner/executor | Parser and live extension wiring are actively being completed |
| RDF-star | Legacy syntax/parsers and hashed embedded triples exist | RDF 1.2 triple-term model, reifiers, syntax, and serialization migration |
| GeoSPARQL | Active partial 2D WKT work | Conformance audit, CRS/unit handling, literal resolution, full error semantics, no false claims |
| WKT geometry | Cold parser uses heap-backed geometry collections | Keep outside hot predicates or add a bounded compiled representation |
| `.10d` spatial index | Scan-free BVH/kd-tree query frontend exists | Asset registry, authorization, query planner integration, stable external descriptor |
| Computational geometry | Broad real library with hot/cold manifests | Typed query adapters, per-operation budgets, exactness contract, result lifecycle |
| Tensor10D | Fixed 40-byte value and resident bounded search | Public profile semantics, stable asset mapping, query operators, federation contract |
| Renderer | Real native/WASM volumetric rendering and picking | Stable result-handle consumption; renderer must not define query truth |
| WebSocket/SSE | Subscription and graph-revision patterns exist | Authenticated durable job state, replay/resume, expiry, and cancellation semantics |
| VC support | Credential models/codecs and selected verification paths exist | VC 2.0 conformance inventory, status/policy integration, non-fabricating decision join |

**Additional honesty notes carried from the plan:**

- **Filter builtins (resolved 2026-07-12).** A former catch-all that silently returned `true` for any
  unimplemented builtin has been replaced with named, fail-closed errors. Genuinely implemented
  (resolver-backed, unit-tested): `REGEX`, `CONTAINS`, `STRSTARTS`, `STRENDS`, `STRLEN`, plus
  `SAMETERM` and `IF`. Value-producing `BIND` is implemented. Still deferred (now honest errors, not
  fabrication): the string-*producing* builtins (`CONCAT`/`SUBSTR`/`UCASE`/`LCASE`/`STRBEFORE`/
  `STRAFTER`/`ENCODE_FOR_URI`), `COALESCE`, and the date/time, `UUID`/`STRUUID`, `RAND`, IRI/BNODE
  construction, `STRLANG`/`STRDT`, and `LANGMATCHES` builtins — the residual QISP-R06 work.
- **Streaming SPARQL (§15d):** continuous/windowed reasoning is **not implemented**; result streaming
  is only *partial* (chunking after eager materialization). `qisp:StreamingSparql` is a planned
  Phase 8b class.
- **WebRTC transport (§15e):** **not implemented** (greenfield; the only WebRTC in the tree is a
  self-labelled mock). `qisp:WebRtcTransport` is a planned transport binding.

---

## 7. Requirement register (QISP-R01 … QISP-R33)

Consolidated from §10.1 (core, R01–R18), §15b (permissive commons, R19–R23), §15d (streaming,
R24–R28), and §15e (WebRTC transport, R29–R33). Each requirement is reviewable and carries primary
evidence.

### 7.1 Core (§10.1)

| ID | Requirement | Primary evidence |
|---|---|---|
| QISP-R01 | Ordinary `/sparql` behavior remains conformant and backwards compatible | upstream SPARQL tests + protocol tests |
| QISP-R02 | RDF 1.2 triple terms/reifiers have a version-pinned model and legacy migration | syntax/round-trip fixtures |
| QISP-R03 | Dense payloads remain out of NQuin/RDF expansion and use validated asset references | ABI and forged-handle tests |
| QISP-R04 | External identities never expose process/GPU addresses or unchecked offsets | security tests and API review |
| QISP-R05 | Function signatures, units, CRS/profile, exactness, and errors are machine-readable | ontology + SHACL + service description |
| QISP-R06 | Expression functions are query-snapshot-pure and safely repeatable/reorderable | memoization and optimizer-order tests |
| QISP-R07 | Hot operations are caller-buffered and allocation measured | `assert_zero_alloc` evidence |
| QISP-R08 | Cold construction is byte/work/deadline bounded and cancellable | workspace/admission tests |
| QISP-R09 | Coarse filtering has no false negatives relative to the exact phase | corpus/oracle property tests |
| QISP-R10 | Approximate/GPU results declare profile and error; rights decisions require permitted assurance | CPU/GPU oracle + policy tests |
| QISP-R11 | Async behavior is explicit, authenticated, monotonic, expiring, and does not corrupt `/sparql` | job state/protocol tests |
| QISP-R12 | Result persistence is a separate governed operation with provenance | mutation/WAL/promotion tests |
| QISP-R13 | Authorization is scoped and checked before resolution and disclosure | VC/policy/TOCTOU tests |
| QISP-R14 | Materialized results are dependency-keyed, policy-partitioned, and invalidated | live-versus-cache equivalence tests |
| QISP-R15 | Federation negotiates capability and moves no local handles | two-endpoint interop/security tests |
| QISP-R16 | Non-QISP clients retain useful RDF discovery and alternate representations | generic client test |
| QISP-R17 | Every public capability has a non-immersive accessible representation | accessibility review/test |
| QISP-R18 | Conformance claims are module-granular and backed by published reports | manifest + report validation |

### 7.2 Permissive commons (§15b)

| ID | Requirement | Primary evidence |
|---|---|---|
| QISP-R19 | Licence and attribution are first-class, not optional metadata; a result propagates the union of its inputs' licence obligations and refuses to emit obligations it cannot satisfy | licence-propagation + refusal tests |
| QISP-R20 | The private↔commons boundary is enforced both directions — no unauthorized publication into commons, no silent re-licensing of commons to private; sensitivity lanes + licence obligations travel into every derived asset | boundary + derivation tests |
| QISP-R21 | Commons contribution is an accountable, reversible act (ed25519-signed, WAL-recorded, with contributor DID, licence grant, and withdrawal path) | contribution/withdrawal + WAL tests |
| QISP-R22 | Commons federation is licence- and provenance-aware; open-data ingestion stamps source, licence, retrieval time, and digest | federation + ingestion provenance tests |
| QISP-R23 | Commons queries degrade to open standards (generic RDF client, `void:`/DCAT discovery, OGC service where geospatial) | open-data discovery + generic client tests |

### 7.3 Streaming / continuous SPARQL (§15d)

| ID | Requirement | Primary evidence |
|---|---|---|
| QISP-R24 | Standing queries are explicit, authenticated, and revisioned first-class resources (register/status/deregister), reusing the job state machine | continuous-query lifecycle tests |
| QISP-R25 | Windows are bounded and declared (time-based `RANGE`/`STEP` or triple-count, tumbling/sliding) with hard caps; over-budget is an error, not silent truncation | window-bound + over-budget tests |
| QISP-R26 | Output is incremental and honest — RStream/IStream/DStream (full/inserted/deleted) deltas, provenance-stamped and monotonic per window instant, with the consumer told which it receives | delta-correctness tests |
| QISP-R27 | Continuous evaluation is bounded, snapshot-consistent, and backpressured (one pinned revision per tick; per-tick budgets; drop-oldest-with-reported-gap or bounded buffer; no hot-path allocation) | backpressure + snapshot + zero-alloc tests |
| QISP-R28 | Streaming honours authorization, privacy, and the commons boundary continuously — policy/lanes/licence/revocation re-checked as the stream advances; a denied/private tuple never reaches a stream, cache, or view | per-tick policy + leak tests |

### 7.4 Peer-to-peer WebRTC transport (§15e)

| ID | Requirement | Primary evidence |
|---|---|---|
| QISP-R29 | A data channel is network egress and is governed like remote `SERVICE` — not default-on, never carrying a process-local handle/pointer | egress-policy + handle-isolation tests |
| QISP-R30 | Authenticate the peer before any query; DTLS ≠ authorization — mutual-auth challenge-response (both directions) must succeed, and every operation stays VC/policy-scoped | mutual-auth + per-op policy tests |
| QISP-R31 | Signaling must not leak the social graph — rendezvous exchanges only what ICE needs and degrades to offline/mailbox when no direct route exists | signaling-privacy tests |
| QISP-R32 | Relay and address-exposure privacy is first-class — offer a metadata-private path (Nym opt-in), never force an undisclosed TURN relay, and no relay sees authorization plaintext | metadata-leak + relay-disclosure tests |
| QISP-R33 | The commons/private boundary and licence obligations travel P2P — no private/denied tuple crosses a peer channel; commons assets carry licence/attribution to the peer | per-peer boundary + laundering tests |

---

## 8. What a non-QISP client still gets (graceful degradation, §7.4)

Dereferencing the SPARQL endpoint without a query returns a standard **SPARQL Service Description**;
QISP is advertised through `sd:feature`, supported extension-function IRIs, result formats, limits,
and a link to the full QISP capability graph. Draft SPARQL 1.2 service-description terms are
version-pinned, and SPARQL 1.1-compatible discovery is retained where possible.

A client that does not understand QISP can still:

- **query asset metadata and provenance as ordinary RDF** — the descriptor graph is plain triples;
- **retrieve a standard alternate representation** (WKT/GML, glTF/GLB, an OGC 3D resource link, or
  descriptor-only RDF) when authorized;
- **ignore QISP-specific triples** without changing the core graph meaning;
- **receive a normal SPARQL expression error** for an unsupported function — errors stay expression
  errors and never silently become `false` (§4.3);
- **discover that an operation is unavailable** instead of receiving a fabricated fallback result.

This degradation is a hard requirement (QISP-R16), paired with the accessibility requirement that
every public capability also has a non-immersive RDF/HTML/tabular representation (QISP-R17): immersive
presentation must never become an access barrier.

---

## 9. Companion artifacts in this directory

| File | Purpose |
|---|---|
| `qisp.ttl` | The ontology: classes (§3.2), properties (§3.3), conformance classes (§2.3), exactness profiles (§4.4), and the Tensor10D profile with per-dimension metadata (§3.5). |
| `qisp-functions.ttl` | The typed function catalog (§4.1/§4.2): value-kinds, arg counts, execution class, determinism, exactness, and FILTER/BIND/ORDER BY legality per function. |
| `qisp-shapes.ttl` | SHACL shapes validating dense-asset descriptors (§3.4), Tensor10D assets (§3.5/§3.6), and authorization decisions (§5.3). |
| `examples/` | The three example queries (§5) plus a README mapping each to conformance classes and requirements. |
| `adr/` | Architecture Decision Records: dense-asset identity, RDF 1.2 migration, exactness model, async protocol. |

---

## 10. Phase 0 gate (§11)

Phase 0 exits only when (QISP-R05, QISP-R18): the examples are SHACL-valid, the provisional IRIs are
stable, the conformance classes are explicit, the current-state inventory (§6 above) is published, and
**there is no claim of W3C/OGC standard status**. These artifacts are the deliverable; a conforming
runtime is later-phase work and is **not** claimed here.
