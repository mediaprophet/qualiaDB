# Immersive SPARQL hypermedia profile for QualiaDB

**Status:** internal review candidate v0.2, 2026-07-12  
**Working name:** Qualia Immersive SPARQL Profile (QISP)  
**Primary implementation area:** `crates/qualia-core-db/src/sparql_library/`  
**Native compute dependencies:** `specialized_libs/computational_geometry/`, `tensor/`,
`render/`, `.10d` containers, daemon query services, identity/credentials, and Solid bridge  
**Goal:** expose QualiaDB's spatial, temporal, mesh, Tensor10D, provenance, and authorization
capabilities through an interoperable RDF/SPARQL surface without encoding dense numerical
payloads as triples or weakening the engine's deterministic, bounded, zero-copy design.

**Review posture:** this is an implementation and incubation plan, not a normative standard.
The eventual profile will use RFC 2119/8174 requirement language; this plan deliberately uses
ordinary language except when restating an upstream requirement. Every external reviewer comment
should be attachable to a requirement ID, decision, phase, or acceptance test added below.

---

## 0. Executive decision

Build QISP as a **profile and extension vocabulary over RDF 1.2 / SPARQL 1.2**, not as a
forked query language.

The design has four layers:

1. **Standards-compatible semantic layer** — normal RDF terms, RDF 1.2 triple terms,
   SPARQL query forms, GeoSPARQL where it applies, PROV-O, OWL-Time, QUDT, VC Data Model
   2.0, DIDs, and standard SPARQL result negotiation.
2. **Content-addressed dense asset layer** — meshes, `.10d` sections, tensor buffers,
   trajectories, BVHs, and derived geometry remain compact binary assets. RDF describes
   and links them; it does not triplify every vertex or tensor cell.
3. **Typed computational extension layer** — stable function IRIs map to bounded native
   geometry/tensor kernels. Lightweight predicates can run synchronously. Constructive
   operations use a query-scoped result arena and explicit resource budgets.
4. **Optional asynchronous hypermedia layer** — long-running work is submitted to a job
   resource with status, cancellation, provenance, and result links. This is an advertised
   QISP protocol extension, not falsely presented as standard SPARQL Protocol behavior.

The renderer is a consumer and accelerator, not the semantic source of truth. Exact
geometry comes from the computational-geometry library. Renderer/GPU approximations must
be explicitly labelled and checked against deterministic CPU oracles where correctness
matters.

---

## 1. Why this fits the codebase

QualiaDB already contains most of the necessary substrate:

| Existing surface | Reuse in QISP |
|---|---|
| `sparql_ast.rs`, parser, planner, executor | Fixed-capacity SPARQL AST and execution pipeline |
| `Pattern::StarTriple` and RDF-star parsers | Compatibility input while migrating to RDF 1.2 triple terms |
| `geosparql.rs` and custom functions | Standards-aligned 2D geospatial baseline |
| `sparql_websocket.rs` | Subscription concepts; not yet the job protocol |
| `query_frontend.rs` | Scan-free `.10d` BVH/kd-tree coarse filtering |
| `boolean_3`, `nary_csg`, `corefine_3d` | Exact/fine constructive geometry |
| `geometry_workspace.rs` | Caller-owned, byte-budgeted, cancellable construction arena |
| geometry capability manifests | Admission control and hot/cold operation classification |
| `Tensor10D` and resident substrate | Fixed-layout 10D values and bounded nearest-neighbour search |
| `.10d` sections and `webizen-render` | Portable mesh/tensor representation and visualization |
| PROV-O filters and temporal graph support | Derived-result lineage, validity, and invalidation |
| identity credentials and Solid bridge | Holder-controlled credential retrieval and verification |
| daemon SSE graph revisions | Live invalidation/materialized-view notification pattern |

The current uncommitted GeoSPARQL/parser work and
`docs/plans/sparql-full-implementation.md` are direct prerequisites. QISP must not create a
second parser, extension registry, literal table, or protocol endpoint while those foundations
are active. Phase 1 starts only after the full-SPARQL slices for expressions, group patterns,
extension dispatch, and the relevant RDF 1.2 syntax have stable public contracts.

### 1.1 Current-state honesty audit

The substrate is substantial, but the plan must not confuse a type or module with an end-to-end
capability:

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

Before Phase 0 exits, convert this table into a versioned implementation inventory with links to
tests. Any `Partial` capability remains visibly partial in the service description.

**Known current-state divergence to fix (recorded here so it is not lost):** as of the completed
full-SPARQL work, `sparql_filter.rs`'s expression evaluator has a catch-all arm
(`_ => Ok(EvalResult::Boolean(true))`) for builtin functions that are not yet implemented — it
silently returns *true* rather than raising an expression error. This directly violates §4.3
("SPARQL expression errors must not silently become `false`" — here it is silently `true`, which
is worse: an unimplemented predicate passes every row). The GeoSPARQL `Function::Custom` dispatch
added in the same work is honest (it errors when a geometry literal can't be resolved), but the
builtin catch-all must be changed to a proper expression error before QISP relies on FILTER
semantics. Small and self-contained; do it in Phase 1's error-semantics work (`QISP-R06`) or
sooner.

---

## 2. Standards baseline

### 2.1 Normative foundations to reuse

- [SPARQL 1.2 Query Language](https://www.w3.org/TR/sparql12-query/) for query syntax,
  extension functions, query forms, and RDF 1.2 triple terms.
- [SPARQL 1.2 Protocol](https://www.w3.org/TR/sparql12-protocol/) for the conformant
  `/sparql` request/response surface and content negotiation.
- [SPARQL 1.2 Service Description](https://www.w3.org/TR/sparql12-service-description/)
  for endpoint discovery, supported languages, result formats, features, and extension
  functions.
- [RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/) and RDF 1.2 concrete syntaxes.
- [OGC GeoSPARQL 1.1](https://www.ogc.org/standards/geosparql/) for geographic features,
  CRS-aware geometry literals, topological relations, and `geof:` functions.
- [Spatial Data on the Web Best Practices](https://www.w3.org/TR/sdw-bp/) for stable
  identifiers, appropriate encodings, CRS, spatial links, versioning, privacy, and ethics.
- [W3C Verifiable Credentials Data Model 2.0](https://www.w3.org/TR/vc-data-model-2.0/),
  [VC Data Integrity 1.0](https://www.w3.org/TR/vc-data-integrity/), and
  [DID Core](https://www.w3.org/TR/did-core/) for identity and credential verification.
- PROV-O for derivation/activity/agent lineage, OWL-Time for intervals and instants, QUDT
  for quantities and units, SHACL for data/profile validation, and Solid/LDP for
  holder-controlled retrieval where deployed.
- [RFC 7240](https://www.rfc-editor.org/rfc/rfc7240.html) for the optional
  `Prefer: respond-async` preference and [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457.html)
  for HTTP Problem Details on the non-SPARQL job API.
- [OGC API — 3D GeoVolumes](https://ogcapi.ogc.org/geovolumes/) as an interoperability
  reference for discovering and retrieving 3D content; QISP should complement rather than
  recreate a general-purpose 3D asset delivery API.

SPARQL/RDF 1.2 documents are still moving through the W3C process at the date of this plan.
Phase 0 must pin the exact dated specification snapshots and test-suite commits used for
implementation. The profile can advertise draft support, but must distinguish it from stable
SPARQL 1.1 conformance until the upstream specifications reach Recommendation.

### 2.2 Important standards corrections to the source conversation

1. Use the VC 2.0 context `https://www.w3.org/ns/credentials/v2`. Do not invent a
   `did:Agent` class; a DID is an identifier, while agent class semantics come from a
   suitable vocabulary such as FOAF, schema.org, or a QISP class.
2. RDF/SPARQL 1.2 triple terms use `<<( ?s ?p ?o )>>`. Metadata about a triple term is
   expressed through a reifier, normally with `rdf:reifies`. The older
   `<< ?s ?p ?o >> :authorizedBy ?vc` form remains only a legacy RDF-star compatibility
   syntax until a deliberate migration policy is implemented.
3. A VC is not automatically trusted because it parses or is linked from a triple.
   Authorization requires cryptographic verification, status checking, issuer/policy
   evaluation, audience/purpose checks, and time/scope checks.
4. Geometry ceasing to intersect does not by itself revoke every authorization artifact.
   It invalidates the occupancy condition. The policy decision and any materialized edge
   must then expire or be invalidated explicitly and observably.
5. Native memory pointers must never cross the network API. External terms identify
   content-addressed assets; pointer/offset resolution remains process-local and validated.

### 2.3 Conformance labels

Every endpoint advertises one or more independent conformance classes:

- `qisp:CoreRdf12`
- `qisp:Sparql12Query`
- `qisp:GeoSparql11`
- `qisp:DenseAssetReferences`
- `qisp:SpatialPredicates`
- `qisp:ConstructiveGeometry`
- `qisp:Tensor10D`
- `qisp:AsyncJobs`
- `qisp:CredentialBoundRelations`
- `qisp:MaterializedSpatialViews`
- `qisp:StreamingSparql` (RSP-QL-aligned continuous/windowed queries — see §15d)
- `qisp:WebRtcTransport` (browser peer-to-peer federation/streaming — see §15e)

An implementation must not claim the parent profile merely because one extension function
is registered. Publish a machine-readable capability graph and a human-readable profile.

### 2.4 Specification and namespace governance

Use four maturity labels:

1. **Editor's Draft** — vocabulary and semantics can change; no compatibility promise.
2. **Implementer Draft** — IRIs are stable within `0.x`; changes require migration notes.
3. **Candidate Profile** — feature-frozen, two implementations, complete conformance suite.
4. **Stable Profile** — permanent versioned IRIs, published implementation report, documented
   errata and deprecation process.

Each published profile records:

- dated upstream standard versions and test-suite commits;
- namespace owner and continuity plan;
- semantic-versioning and deprecation policy;
- IRI persistence, HTTPS content negotiation, and archived copies;
- patent/licensing statement for specification text, ontology, schemas, and tests;
- change log with machine-readable term status (`experimental`, `stable`, `deprecated`).

Terms are never silently repurposed. A semantic change gets a new term or namespace version.

---

## 3. Data model: graph descriptors, not triple bloat

### 3.1 Working namespaces

Use separate ontology, function, and datatype namespaces so predicates are not confused
with executable functions:

```text
qisp:  https://standards.qualiadb.org/immersive/0.1#
qispf: https://standards.qualiadb.org/immersive/function/0.1#
qispd: https://standards.qualiadb.org/immersive/datatype/0.1#
```

These are provisional versioned IRIs. Before public standardization, publish HTTPS
content negotiation for Turtle, JSON-LD, RDF/XML if required, and HTML documentation.
Do not mint `1.0` until the interoperability suite has two independent consumers.

### 3.2 Core resource classes

```text
qisp:SpatialAsset
qisp:MeshAsset
qisp:TensorAsset
qisp:TensorProfile
qisp:Trajectory
qisp:SpatialZone
qisp:CoordinateFrame
qisp:TransientGeometry
qisp:DerivedGeometry
qisp:Computation
qisp:CapabilitySet
qisp:AuthorizationDecision
qisp:MaterializedSpatialRelation
```

### 3.3 Core properties

```text
qisp:hasDenseRepresentation
qisp:contentDigest
qisp:mediaType
qisp:byteLength
qisp:sectionType
qisp:coordinateFrame
qisp:crs
qisp:tensorProfile
qisp:dimensionOrder
qisp:unit
qisp:validDuring
qisp:derivedAsset
qisp:approximationMode
qisp:errorBound
qisp:expiresAt
qisp:authorizedBy
qisp:policy
qisp:decision
qisp:proofDigest
```

Reuse `geo:hasGeometry`, `geo:asWKT`, `prov:wasDerivedFrom`, `prov:wasGeneratedBy`,
`prov:used`, `prov:generatedAtTime`, `time:hasBeginning`, `time:hasEnd`, VC 2.0 terms,
and QUDT terms instead of minting synonyms.

### 3.4 Dense asset reference

The public RDF term is a stable URI, preferably a content-addressed URI or DID URL. Its
descriptor graph carries:

- digest algorithm and digest;
- media type (`model/gltf-binary`, registered `.10d` type when available, or a provisional
  vendor type during incubation);
- byte length and optional byte-range/section descriptor;
- coordinate frame, CRS, dimension order, and units;
- topology/manifold assumptions;
- provenance and access policy;
- integrity status and lifecycle state.

Internally, resolve that URI to a bounded record such as `DenseAssetRef` containing a
60-bit token, generation number, section kind, offset, length, and digest prefix. Never
put a Rust address into an `NQuin`. Generation checks prevent stale-handle reuse.

### 3.5 Tensor10D profile

The Qualia profile fixes the ordered dimensions as:

```text
[q, v, w, x, y, z, t, alpha, mu, sigma]
```

The ontology must describe their meanings, datatypes, units, coordinate frame, valid
ranges, topology rules, and whether a dimension is spatial, temporal, epistemic, spectral,
or categorical. External systems may treat the payload as an opaque asset while still
understanding its profile metadata.

Do not present all ten components as physical dimensions. `q`, `v`, `w`, `alpha`, `mu`,
and `sigma` have Qualia-specific semantics and require an explicit profile URI.

### 3.6 Representation negotiation and lexical contracts

QISP conformance must not require another implementation to decode `.10d`. A dense asset URI
should offer one or more representations according to its capability and policy:

| Representation | Intended use |
|---|---|
| GeoSPARQL WKT/GML literal | small interoperable 2D/2.5D geometry |
| glTF/GLB | portable render mesh and scene exchange |
| OGC-compatible 3D resource link | large 3D discovery/delivery where applicable |
| `.10d` | Qualia-native compiled mesh/tensor/index path |
| bounded tensor buffer | native/WASM/GPU Tensor10D exchange |
| descriptor-only RDF | discovery when payload disclosure is denied |

Use ordinary HTTP content negotiation and typed `Link` relations for alternate representations.
Every representation carries the same logical asset identity plus a representation-specific
digest; transformations between representations have PROV-O activities and declared loss/error.

Define canonical lexical forms before adding datatypes:

- asset reference: absolute IRI, never an opaque numeric pointer literal;
- digest: multibase/multihash or an explicitly named algorithm plus canonical lowercase bytes;
- quantity: numeric RDF literal plus QUDT unit, or a profile-defined compound result resource;
- timestamp/interval: XSD/OWL-Time lexical forms with timezone rules;
- Tensor10D inline value, if permitted at all: exactly ten finite values plus a profile IRI;
- geometry/tensor result reference: IRI with query/job scope and expiry discoverable from RDF.

Reject non-canonical, non-finite, mixed-profile, or ambiguous-unit values before native dispatch.

---

## 4. Function model

### 4.1 Function families

| Family | Example functions | Expected execution class |
|---|---|---|
| Topological | `qispf:intersects`, `contains`, `touches` | hot predicate after coarse filter |
| Proximity | `distance`, `withinDistance`, `nearest` | hot query, caller-buffered |
| Visibility | `lineOfSight`, `occludes` | ray/BVH; exact or labelled approximation |
| Measurement | `volume`, `surfaceArea`, `centroid` | hot if precomputed/simple; otherwise bounded |
| Temporal | `intersectsAt`, `trajectoryIntersects`, `sliceAtTime` | bounded trace/window evaluation |
| Tensor | `tensorDistance`, `tensorWithin`, `tensorSlice`, `knn` | resident substrate / GPU batch |
| Constructive | `intersectionGeometry`, `unionGeometry`, `differenceGeometry` | cold bounded arena |
| Transform | `transform`, `reproject`, `buffer` | cold bounded unless fixed scalar transform |

GeoSPARQL names and semantics take precedence for operations already covered by
GeoSPARQL. QISP functions are for mesh, volumetric, temporal, tensor, or explicitly
higher-dimensional behavior that GeoSPARQL does not define.

Every function has a published signature table covering argument RDF term kinds, accepted asset
classes/profiles, CRS rules, result term/datatype, unit, empty-input behavior, error conditions,
exactness, determinism, complexity class, and whether it is legal in FILTER, BIND, ORDER BY,
GROUP BY, or only an explicit job operation.

### 4.2 No untyped `u64 -> u64` public function contract

The current extension registry is useful but too weak for constructive operations. Add a
typed signature registry:

```rust
pub enum ImmersiveValueKind {
    Boolean,
    Scalar,
    Quantity,
    AssetRef,
    GeometryRef,
    TensorRef,
    Instant,
    Interval,
}

pub enum ExecutionClass {
    HotZeroHeap,
    ColdBoundedSync,
    AsyncRequired,
}

pub struct ImmersiveFunctionDescriptor {
    pub iri_hash: u64,
    pub args: [ImmersiveValueKind; 4],
    pub arg_count: u8,
    pub result: ImmersiveValueKind,
    pub execution: ExecutionClass,
    pub deterministic: bool,
    pub exactness: ExactnessClass,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
}
```

Query parsing remains a cold authoring tier. Evaluation uses fixed records and borrowed
registries. Function descriptors connect to the existing computational-geometry capability
manifests rather than duplicating resource limits.

### 4.3 Error semantics

SPARQL expression errors remain expression errors; they must not silently become `false`.
QISP defines stable error codes for:

- unknown or stale asset reference;
- unsupported CRS/profile conversion;
- invalid/non-manifold geometry;
- dimension/profile mismatch;
- output or workspace budget exceeded;
- exactness requirement unavailable;
- authorization denied;
- cancelled or expired computation;
- non-deterministic backend disallowed.

HTTP Problem Details may describe protocol/job errors. SPARQL result rows may expose an
optional diagnostic graph only when the requester asks for it.

### 4.4 Functional semantics, memoization, units, and exactness

SPARQL optimizers may reorder or repeat expression evaluation. Therefore QISP expression
functions are **referentially transparent within one query snapshot**:

- no durable graph mutation, asset publication, authorization grant, or external message occurs
  merely because a FILTER/BIND function is evaluated;
- the result depends only on canonical arguments, the pinned dataset/asset snapshot, the declared
  execution profile, and a query-stable time/seed where applicable;
- repeated calls with the same key return the same RDF term and reuse one bounded memo entry;
- speculative evaluation can be discarded without an externally visible partial asset;
- durable persistence is a separate governed SPARQL Update or job-result promotion operation.

Expression evaluation never performs network fetches, Solid retrieval, credential presentation,
or federated asset transfer. The planner/admission layer must resolve an already-authorized local
immutable payload or classify the request as a prefetch/job workflow. This keeps FILTER semantics
bounded and prevents repeated optimizer evaluation from causing I/O or consent prompts.

Constructive expression functions may populate a private query arena as an implementation detail,
but the externally visible result is deterministic and query-scoped. The memo key is:

```text
hash(function IRI + semantic version,
     canonical argument terms,
     input asset digests + generations,
     dataset revision,
     CRS/profile,
     exactness/backend policy,
     query-stable time/seed where semantically relevant)
```

Memo capacity is fixed and budgeted. Exhaustion is an expression error, never an unbounded map.

Exactness is an explicit argument or a query/job profile—not an invisible server preference.
Initial profiles:

- `qisp:Exact` — robust/exact predicates and constructions where supported;
- `qisp:DeterministicApproximate` — reproducible approximation with declared absolute/relative
  error bounds;
- `qisp:InteractiveApproximate` — renderer/GPU-oriented result, never accepted for
  rights-affecting policy without a separate exact verification.

Numeric measurements must state their unit. Prefer the GeoSPARQL signature where it exists. For
QISP-only measurements, either accept an explicit unit IRI and return a numeric literal in that
unit, or return a QUDT-described quantity resource. Do not return an undocumented bare float.

---

## 5. Query syntax and examples

### 5.1 Synchronous predicate query

```sparql
VERSION "1.2"
PREFIX qisp:  <https://standards.qualiadb.org/immersive/0.1#>
PREFIX qispf: <https://standards.qualiadb.org/immersive/function/0.1#>

SELECT ?agent ?zone
WHERE {
  ?agent qisp:activeSpatialAsset ?agentAsset .
  ?zone  a qisp:SpatialZone ;
         qisp:hasDenseRepresentation ?zoneAsset .
  FILTER(qispf:intersects(?agentAsset, ?zoneAsset, qisp:Exact))
}
```

### 5.2 Constructive geometry query

Constructive functions return a query-scoped geometry reference, not a copied mesh literal.

```sparql
VERSION "1.2"
PREFIX rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX prov:  <http://www.w3.org/ns/prov#>
PREFIX unit:  <http://qudt.org/vocab/unit/>
PREFIX qisp:  <https://standards.qualiadb.org/immersive/0.1#>
PREFIX qispf: <https://standards.qualiadb.org/immersive/function/0.1#>

CONSTRUCT {
  ?derived a qisp:TransientGeometry ;
           qisp:hasDenseRepresentation ?intersection ;
           prov:wasDerivedFrom ?left, ?right ;
           qisp:volumeMeasure ?volume ;
           qisp:volumeUnit unit:M3 ;
           qisp:expiresAt ?expiry .
}
WHERE {
  ?left  qisp:hasDenseRepresentation ?leftAsset .
  ?right qisp:hasDenseRepresentation ?rightAsset .
  BIND(qispf:intersectionGeometry(?leftAsset, ?rightAsset, qisp:Exact) AS ?intersection)
  BIND(qispf:volume(?intersection, unit:M3, qisp:Exact) AS ?volume)
  FILTER(?volume > 0)
  BIND(qispf:transientResource(?intersection) AS ?derived)
  BIND(qispf:defaultExpiry() AS ?expiry)
}
```

`qispf:transientResource` must deterministically scope its URI to the query/job and result
slot. `BNODE()` may still be supported, but it is unsuitable as the only dereferenceable
identity for a generated dense asset.

### 5.3 Credential-bound relation using RDF 1.2 triple terms

```sparql
VERSION "1.2"
PREFIX rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX vc:   <https://www.w3.org/2018/credentials#>
PREFIX qisp: <https://standards.qualiadb.org/immersive/0.1#>

CONSTRUCT {
  ?relation rdf:reifies ?occupancy ;
            qisp:authorizedBy ?decision ;
            prov:generatedAtTime ?now .
  ?decision a qisp:AuthorizationDecision ;
            qisp:decision qisp:Granted ;
            qisp:proofDigest ?proofDigest ;
            qisp:expiresAt ?expiry .
}
WHERE {
  BIND(<<( ?agent qisp:occupies ?volume )>> AS ?occupancy)
  BIND(qispf:reifierFor(?occupancy) AS ?relation)
  ?decision qisp:subject ?agent ;
            qisp:scope ?volume ;
            qisp:decision qisp:Granted ;
            qisp:proofDigest ?proofDigest ;
            qisp:expiresAt ?expiry .
  BIND(NOW() AS ?now)
}
```

The computation that creates `?decision` is separate from geometry. It verifies a VC/VP
against policy and exposes only the minimum necessary decision metadata. Raw proof material
or unrelated credential claims are not copied into the spatial graph by default.

---

## 6. Execution architecture

### 6.1 Coarse-to-fine planning

The planner recognizes typed QISP calls and creates an explicit pipeline:

```text
semantic graph scan
  -> asset/profile/authorization resolution
  -> BVH/kd-tree/AABB candidate reduction
  -> exact geometry or tensor kernel
  -> result binding / query-scoped asset
  -> RDF result serialization or renderer notification
```

Add physical operators rather than hiding all work inside `FILTER`:

```rust
ImmersiveAssetResolve
SpatialIndexScan
SpatialPredicate
TensorPredicate
ConstructiveGeometry
CredentialDecisionJoin
MaterializedRelationScan
```

Each operator declares maximum scratch, output size, execution class, exactness, and
cancellation behavior. The planner rejects impossible budgets before execution.

### 6.2 Query-scoped arena and asset registry

Create a `QueryComputeContext` owned by the endpoint/executor:

- borrowed graph and lexicon;
- caller-owned `GeometryWorkspace` bytes;
- fixed-capacity result-handle table;
- asset resolver with generation validation;
- capability and policy snapshot;
- cancellation token and deadline;
- deterministic seed and backend policy;
- provenance activity identifier;
- output byte/row limits.

The context pins one dataset revision, asset-registry generation set, policy snapshot, requester
principal, query-start instant, and function-registry version. Authorization is checked before
payload resolution and again before result disclosure/promotion. This prevents a query from
mixing revisions or continuing on a stale grant after a long computation. If a required snapshot
cannot be held within the budget, execution fails or becomes a job with an explicit isolation
contract.

Hot predicates never allocate. Cold construction may allocate only through a bounded
workspace or an explicitly classified cold builder, and the public output remains a
caller-buffered/content-addressed asset.

The executor must cap not only bytes but also function invocations, pairwise candidate expansions,
memo entries, geometry primitive count, recursion-equivalent work-stack depth, GPU submissions,
wall-clock deadline, and serialized result size. A small input graph can otherwise induce an
unbounded Cartesian product of expensive calls.

### 6.3 CPU, GPU, and renderer roles

- CPU exact/adaptive predicates are the reference semantics.
- GPU kernels are admitted only through capability manifests with deterministic input
  packing, bounded buffers, tolerance/error metadata, and CPU oracle tests.
- The renderer receives stable asset handles and scene deltas. It must not own the only
  copy of a query result or determine access policy from pixels.
- `occludes` has at least two profiles: exact mesh ray test and render-space approximation.
  A caller can require `qisp:Exact`; an approximate result carries an error bound/profile.

### 6.4 Transient lifecycle

Default derived assets are query-scoped and disappear when serialization completes. A
caller may request a leased transient asset with an expiry and dereferenceable job URI.
Persistence requires a separate authorized mutation/materialization operation.

All derived assets record:

- input digests and graph revision;
- function IRI and version;
- backend/exactness profile;
- deterministic seed if applicable;
- time, agent, policy decision, and expiry;
- output digest and byte length.

---

## 7. Protocol surfaces

### 7.1 Conformant SPARQL endpoint

`/sparql` remains SPARQL Protocol conformant:

- GET or POST query forms;
- normal SELECT/ASK result formats;
- RDF formats for CONSTRUCT/DESCRIBE;
- content negotiation and version announcement;
- synchronous execution within advertised limits;
- deterministic rejection when a request requires asynchronous execution.

Do not return a QISP job document in place of a SPARQL result without explicit opt-in.

### 7.2 Asynchronous QISP job profile

Expose a separate endpoint, for example `/qisp/jobs`, or require an explicit
`Prefer: respond-async` extension on `/sparql`.

Submission returns `202 Accepted` and a job URI. The job resource supports:

- `GET` status and progress;
- `DELETE`/cancel with authorization;
- result links with negotiated RDF/SPARQL-result/dense-asset media types;
- SSE progress/completion events;
- optional WebSocket subscription for interactive browsers;
- expiry, cleanup, and idempotency keys;
- digest-bound requester DID/session and audit provenance.

When `Prefer: respond-async` is honored, return `Preference-Applied: respond-async`,
`202 Accepted`, and an absolute `Location` job URI. Because RFC 7240 intentionally does not
define the job resource, the QISP profile must define its state machine, representations, retry
behavior, and terminal result semantics. Responses varying on `Prefer` follow the RFC's cache
requirements. Protocol errors use RFC 9457 problem types with stable HTTPS type IRIs and avoid
leaking internal paths, geometry details, policy rules, or principal identifiers.

Minimum state machine:

```text
admitted -> queued -> running -> succeeded
                    |          -> failed
                    |          -> cancelled
                    -> expired (only before execution)
succeeded/failed/cancelled -> expired -> purged
```

State transitions are monotonic and revisioned. Cancellation is best-effort during a kernel
partition but immediately blocks result promotion/disclosure. A successful job result is still a
leased resource until explicitly promoted.

Reuse the geometry cancellation token and daemon SSE patterns. Do not make the current
in-memory WebSocket session structures the sole durable job registry.

### 7.3 Federation

Federated `SERVICE` calls exchange semantic bindings and asset URIs, never process-local
handles. Before shipping a dense asset, negotiate:

- supported QISP/GeoSPARQL profiles;
- maximum input/output bytes;
- accepted media types and range requests;
- CRS/Tensor10D profile support;
- exactness/backend requirements;
- authentication, purpose, and data-use policy.

Prefer remote predicate execution near the data. Asset transfer is a fallback governed by
policy, digest verification, and byte budgets.

### 7.4 Discovery and graceful degradation

Dereferencing the SPARQL endpoint without a query returns a standard SPARQL Service Description.
Advertise QISP through `sd:feature`, supported extension-function IRIs, result formats, limits,
and a link to the full QISP capability graph. Draft SPARQL 1.2 service-description terms are
version-pinned; retain SPARQL 1.1-compatible discovery where possible.

A client that does not understand QISP can still:

- query asset metadata and provenance as ordinary RDF;
- retrieve a standard alternate representation when authorized;
- ignore QISP-specific triples without changing core graph meaning;
- receive a normal SPARQL expression error for an unsupported function;
- discover that an operation is unavailable instead of receiving a fabricated fallback result.

---

## 8. Authorization, privacy, and human-rights constraints

1. Geometry kernels receive geometry/tensor handles, not credential contents.
2. Credential verification receives the minimum presentation needed for the policy, not
   a holder's complete credential collection.
3. A policy decision is scoped to subject, relying party, purpose, spatial resource,
   time/graph revision, and requested operation.
4. Record verification outcome and proof digest; do not publish the full proof or private
   claims unless explicitly requested and authorized.
5. Holder-controlled Solid retrieval is optional infrastructure, not a prerequisite for
   VC interoperability and not evidence of trust by itself.
6. Prevent spatial inference leaks: enforce minimum result counts/precision, coordinate
   redaction, sensitivity lanes, query-rate controls, and restricted materialization.
7. Never cache a denied/private geometry result into a public materialized graph.
8. Revocation/status changes, graph revisions, trajectory changes, and expiry invalidate
   dependent decisions and materialized spatial relations.
9. Provide accessible, non-immersive RDF/HTML/tabular alternatives for every public query
   capability. Immersive presentation must not become an access barrier.

Threat-model at least: stale handles, malicious meshes, decompression bombs, geometric
complexity attacks, CRS confusion, NaN/Infinity, GPU nondeterminism, job enumeration,
cross-agent cache leakage, replayed presentations, correlation through precise location,
and federation SSRF/data exfiltration.

---

## 9. Materialized spatial views

Materialization is an optimization, not a change in truth semantics.

Create a derived relation only when its dependency key is known:

```text
hash(function version,
     input asset digests,
     graph revision,
     coordinate frame / CRS,
     time window,
     exactness profile,
     authorization scope)
```

Store the relation in a dedicated named graph with PROV-O lineage, generation time,
expiry, sensitivity class, and invalidation link. The query planner may substitute the
view only when the dependency key and requester's policy scope match.

The background maintainer consumes graph/asset revision events and either recomputes or
invalidates affected relations. It must never infer that a cached relation is current
merely because its input URIs have not changed.

---

## 10. Reviewable requirements and dependency ownership

### 10.1 Requirement register

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

### 10.2 Critical path and ownership boundaries

```text
Full-SPARQL parser/extension work
  -> Phase 0 profile snapshot + current-state inventory
  -> Phase 1 RDF 1.2/literal contracts
  -> Phase 2 asset registry + typed values
  -> Phase 3 planner/admission
  -> Phase 4 synchronous predicates
  -> Phase 5 constructive query results
       -> Phase 6 async jobs
       -> Phase 7 credential decisions
       -> Phase 8 views/federation
  -> Phase 9 interoperability/incubation
```

| Concern | Canonical owner | QISP rule |
|---|---|---|
| General SPARQL grammar/algebra | full-SPARQL implementation plan | extend once; no QISP parser fork |
| GeoSPARQL 2D/CRS semantics | `geosparql.rs` + OGC profile | reuse; do not redefine under QISP |
| Dense geometry algorithms | computational geometry library | thin typed adapters only |
| Geometry resource limits | geometry capability manifests/workspaces | reference canonical limits |
| Tensor semantics/runtime | `tensor/` and `.10d` contracts | publish profile; do not create a second tensor |
| Rendering | `render/`, `webizen-render` | consume handles/results only |
| Graph mutation/durability | daemon graph + governed WAL path | result promotion goes through this boundary |
| Credentials/policy | identity credentials + deontic/governance modules | QISP joins verified decisions; it does not invent a verifier |
| Jobs | shared daemon/local scheduler abstraction | extend/extract common service; avoid a QISP-only scheduler silo |

Any phase that needs to change a canonical owner first adds the missing contract and tests there,
then adds the QISP adapter. Cross-module types live at the narrowest shared ABI boundary.

---

## 11. Implementation phases

### Phase 0 — profile artifacts and architecture records

Deliver:

- `docs/standards/qisp/0.1/profile.md`
- `docs/standards/qisp/0.1/qisp.ttl`
- `docs/standards/qisp/0.1/qisp-functions.ttl`
- `docs/standards/qisp/0.1/qisp-shapes.ttl`
- `docs/standards/qisp/0.1/examples/`
- ADRs for dense asset identity, RDF 1.2 migration, exactness, and async protocol.

Gate (`QISP-R05`, `QISP-R18`): SHACL-valid examples, stable provisional IRIs, explicit
conformance classes, current-state inventory, and no claim of W3C/OGC standard status.

### Phase 1 — RDF 1.2 term and literal foundation

Primary files:

- `sparql_ast.rs`
- `sparql_grammar/`
- `sparql_parser.rs`
- RDF-star parsers/serializers
- lexicon/resolver and frame layout as needed

Work:

- represent RDF 1.2 triple terms and reifiers correctly;
- add version announcement parsing;
- retain a documented legacy RDF-star parser mode;
- replace evaluator-owned `String` recovery with borrowed/caller-buffered lexical access;
- add typed literal support for geometry/tensor/asset reference datatypes.

Gate (`QISP-R01`, `QISP-R02`, `QISP-R06`): SPARQL 1.1 regression suite passes; RDF 1.2
syntax round-trips; no new heap activity inside evaluator hot loops.

### Phase 2 — dense asset registry and typed values

Add modules such as:

```text
sparql_library/immersive/mod.rs
sparql_library/immersive/value.rs
sparql_library/immersive/asset_registry.rs
sparql_library/immersive/profile.rs
sparql_library/immersive/functions.rs
```

Work:

- validated, generation-safe `DenseAssetRef` records;
- `.10d`, mesh, WKT, trajectory, and Tensor10D adapters;
- typed function descriptors connected to capability manifests;
- resolver APIs that borrow or write into caller buffers;
- SHACL validation for descriptors and Tensor10D profiles.

Gate (`QISP-R03`–`QISP-R05`, `QISP-R16`): stale/forged handles fail closed; asset
descriptors and a standard alternate representation round-trip; no address leakage.

### Phase 3 — planner and admission control

Primary files:

- `sparql_planner.rs`
- new `immersive/planner.rs`
- geometry capability manifests

Work:

- add explicit immersive physical operators;
- push AABB/BVH/kd-tree filters before exact kernels;
- estimate candidate count, scratch, output, and transfer bytes;
- classify hot, cold synchronous, and async-required queries;
- preserve deterministic operator and reduction order.

Gate (`QISP-R08`, `QISP-R09`): explain plans show coarse/fine stages, invocation/cardinality
limits, and budgets; over-budget queries fail before partial construction.

### Phase 4 — synchronous predicate and measurement MVP

Implement in this order:

1. asset/profile resolution;
2. `intersects`, `contains`, `withinDistance`;
3. exact `distance`, `volume`, `surfaceArea` where supported;
4. `lineOfSight`/`occludes` with explicit exactness profile;
5. Tensor10D distance/radius/kNN through the resident substrate.

Reuse GeoSPARQL for its defined domain. QISP mesh/tensor semantics must not alter `geof:`
results.

Gate (`QISP-R05`, `QISP-R07`, `QISP-R09`, `QISP-R10`): deterministic CPU fixtures,
buffer-boundary tests, parity-valid emitted NQuins, unit/exactness checks, and zero-allocation
tests for every `HotZeroHeap` operation.

### Phase 5 — constructive geometry and CONSTRUCT results

Work:

- `intersectionGeometry`, `unionGeometry`, `differenceGeometry`, transform, temporal slice;
- query-scoped `GeometryWorkspace` and result registry;
- deterministic transient URIs and lease/expiry semantics;
- provenance graph emission;
- `.10d`/GLB result serialization by content negotiation.

Gate (`QISP-R06`, `QISP-R08`, `QISP-R10`, `QISP-R12`): boolean operations are exact or
labelled approximate; cancellation leaves no reachable partial asset; optimizer reordering is
observationally equivalent; identical inputs/profile/seed produce identical digest.

### Phase 6 — asynchronous job service

Primary integration areas:

- `services/daemon_query.rs`
- `services/webizen_server.rs`
- `sparql_endpoint.rs`
- `sparql_websocket.rs`
- reusable bounded job/cancellation infrastructure

Work:

- admission-to-job transition;
- fixed-capacity/budgeted job table and durable metadata as required;
- 202/status/cancel/result/SSE flows;
- authenticated result access and cleanup;
- renderer/browser notifications using asset handles.

Gate (`QISP-R01`, `QISP-R11`): restart/expiry behavior is documented; cancellation is tested;
jobs cannot be enumerated across agents; `/sparql` remains protocol conformant.

### Phase 7 — VC-bound spatial policy

Primary integration areas:

- `identity/credentials/`
- deontic/policy modules
- PROV-O and temporal graph support
- Solid bridge as an optional retrieval adapter

Work:

- typed authorization request and decision records;
- VC 2.0 verification/status adapters;
- purpose/scope/time/audience/issuer policy checks;
- RDF 1.2 reified relation output;
- dependency-driven invalidation without publishing sensitive claims.

Gate (`QISP-R13`): parser-only credentials never authorize; revoked/expired/wrong-scope
credentials deny; snapshot/TOCTOU behavior is tested; selective-disclosure claims remain honest
about actual cryptography.

### Phase 8 — materialized views and federation

Work:

- dependency-keyed cache graph;
- event-driven invalidation/recompute;
- policy-partitioned cache entries;
- `SERVICE` capability negotiation and remote execution;
- range/digest verified dense asset transfer fallback.

Gate (`QISP-R14`, `QISP-R15`): cached and live results are equivalent for the same dependency
key; no cross-policy cache reuse; federation failure is bounded and does not expose local assets.

### Phase 9 — interoperability and standards incubation

Deliver:

- public ontology/profile with permanent versioned IRIs;
- conformance test manifest and EARL-style reports;
- reference queries/results and malformed/adversarial cases;
- at least one external RDF client and one independent implementation/prototype;
- implementation report documenting extensions versus standards;
- standards venue assessment: OGC for spatial functions/profile, W3C Community Group for
  RDF/SPARQL hypermedia patterns, and liaison rather than unilateral branding.

Only after two implementations and vocabulary stability should `0.1` advance toward `1.0`.

Gate (`QISP-R16`–`QISP-R18`): a generic RDF client and an independent QISP consumer pass the
published suite; conformance reports match advertised modules; accessibility and namespace
continuity reviews have no unresolved blockers.

---

## 12. Test and benchmark matrix

### Syntax and protocol

- SPARQL 1.1 regression queries remain valid.
- SPARQL 1.2 version labels and triple terms parse/serialize correctly.
- Legacy RDF-star syntax is either translated or rejected with a precise mode error.
- SELECT/ASK/CONSTRUCT/DESCRIBE content negotiation matches protocol requirements.
- Async opt-in never changes ordinary `/sparql` responses silently.

### Geometry and tensor correctness

- empty, degenerate, non-manifold, self-intersecting, huge, NaN, and mixed-CRS inputs;
- coarse filter has no false negatives relative to exact phase;
- exact CPU oracle versus GPU/WGSL within declared tolerance;
- CSG conservation/topology invariants and deterministic output hashes;
- Tensor10D profile mismatch and dimension ordering failures;
- temporal boundary cases and trajectory interpolation rules.

### Resource behavior

- zero heap for all hot predicates and resident tensor searches;
- 42 MiB pass admission and clean `BudgetExceeded` behavior;
- output buffer full, result table full, deadline, cancellation, and cleanup;
- adversarial geometric complexity cannot create unbounded recursion or work queues;
- concurrent jobs have isolated arenas, deterministic partition/reduction, and no stale
  handle reuse.

### Security and privacy

- forged asset digest/offset/generation;
- malicious remote `SERVICE` and SSRF targets;
- job enumeration and unauthorized cancel/result retrieval;
- expired/revoked/replayed VC/VP and wrong subject/audience/purpose/scope;
- private coordinate leakage through errors, cache keys, progress events, and materialized
  relations;
- sensitivity lane and policy propagation into derived assets.

### Performance acceptance targets

Set measured targets from the benchmark corpus rather than promising universal latency.
Record at minimum:

- candidate reduction ratio and nodes touched;
- parse/plan/coarse/fine/serialize time;
- CPU/GPU bytes transferred and kernel time;
- workspace peak and output bytes;
- cache hit/miss/invalidation counts;
- exactness profile and hardware/adapter identity.

---

## 13. MVP definition

The first honest interoperable release is complete when all of the following work end to
end:

1. An external SPARQL client discovers QISP capabilities.
2. RDF describes two content-addressed `.10d`/mesh assets with CRS/profile metadata.
3. A normal SELECT query runs a BVH-pruned exact `intersects` predicate.
4. A CONSTRUCT query creates an intersection asset in a bounded workspace and returns a
   transient URI plus PROV-O lineage and volume.
5. An over-budget version becomes an authenticated cancellable job via explicit async
   opt-in.
6. The Webizen renderer dereferences the returned asset and displays it without becoming
   its semantic owner.
7. A verified, scoped VC decision can be linked to an occupancy triple term through
   `rdf:reifies`, without publishing the holder's unrelated claims.
8. Changing an input asset, graph revision, trajectory, credential status, or expiry
   invalidates the dependent decision/materialized relation.
9. SPARQL 1.1 regressions, RDF 1.2 syntax tests, geometry correctness tests, zero-heap hot
   tests, and the 42 MiB admission tests all pass.

---

## 14. Non-goals

- Triplifying all mesh vertices, voxel cells, or tensor values.
- Exposing Rust pointers, GPU buffer addresses, or unchecked file offsets as RDF literals.
- Redefining GeoSPARQL functions with Qualia-specific semantics.
- Claiming the provisional vocabulary is already a W3C or OGC standard.
- Treating rendering output as exact computational geometry.
- Treating possession, parsing, or RDF linkage of a VC as authorization.
- Persisting every constructive query result automatically.
- Making Solid, a particular DID method, Apple unified memory, or one GPU vendor mandatory.
- Allowing async extensions to make the standard SPARQL endpoint non-conformant.

---

## 15. External review questions and decision register

The external review should answer these before implementation passes Phase 1:

| Decision | Question | Recommended starting position |
|---|---|---|
| QISP-D01 | Is “QISP” sufficiently distinct and understandable? | keep as working name only pending search/community feedback |
| QISP-D02 | Who controls and preserves `standards.qualiadb.org`? | document legal/operational owner and archival continuity before publishing IRIs |
| QISP-D03 | Which dated RDF/SPARQL 1.2 snapshots are implemented? | pin current W3C publications plus exact rdf-tests commit; update deliberately |
| QISP-D04 | What is the canonical external asset URI scheme? | HTTPS content-addressed resource with alternates; DID URL optional, never mandatory |
| QISP-D05 | How is exactness selected in syntax? | explicit final function argument for MVP; evaluate query-level profile after interop testing |
| QISP-D06 | What does a measurement return? | explicit unit argument + numeric literal for MVP; QUDT resource for compound uncertainty |
| QISP-D07 | Is constructive BIND acceptable to other SPARQL implementers? | yes only with snapshot-pure deterministic semantics; otherwise move construction to job API |
| QISP-D08 | Should async opt-in share `/sparql`? | separate `/qisp/jobs` first; add `Prefer` only after proxy/cache conformance testing |
| QISP-D09 | Which 3D exchange format is mandatory? | RDF descriptor + GLB alternate; `.10d` remains optional Qualia-native profile |
| QISP-D10 | Which credential securing/status mechanisms are in v0.1? | name a narrow verified set; all others fail as unsupported, never “best effort” |
| QISP-D11 | Where should standardization/incubation occur? | seek OGC/W3C community feedback after running code and a second consumer exist |
| QISP-D12 | What is the first rights-sensitive use case? | none in MVP; demonstrate policy plumbing without granting physical access or legal entitlement |

Reviewer deliverables requested:

- standards/conformance gap list with dated references;
- RDF and function-model critique, especially triple-term and side-effect semantics;
- geometry/CRS/unit/exactness critique;
- HTTP async/cache/security critique;
- VC/privacy/human-rights threat-model critique;
- implementability review against fixed-capacity/42 MiB constraints;
- recommendation on vocabulary scope, naming, and standards venue;
- explicit blocking issues versus non-blocking improvements.

Record decisions in this table with date, decision maker/review body, rationale, affected
requirements, migration impact, and superseding decision ID. Do not resolve a contested semantic
issue only in code comments.

---

## 15b. Permissive commons infrastructure (missing from v0.2 — add as a first-class concern)

QISP as drafted is asset- and person-centric. It under-specifies the **permissive commons**:
the shared, openly-licensed, public-good layer that Qualia already reaches (Chora's flagship
worlds, `.10d` provenance containers carrying `source`/`licence`/`attestation`, `canvas_rights`
deontic placement gating, and ~90 curated open-data endpoints). The commons is not "someone
else's private graph" — it is a distinct tier with its own rules, and the profile must name it
so that private and commons data never silently mix.

Guiding distinction (Timothy's design, from the Chora plan): **world-of-man** (a digital *twin*
of a real thing — often private, high-fidelity, consent-scoped) versus **world-of-god** (an
*approximation*, never a twin — the shared commons: maps, sky, biosphere, GLAM/council/SDG open
data). QISP must let a person query across *both* without collapsing them.

Requirements to add (candidate IDs `QISP-R19`–`QISP-R23`):

- **R19 — Licence and attribution are first-class, not optional metadata.** Every commons asset
  carries a machine-readable licence (SPDX / `dct:license` / ODbL / CC), required attribution,
  and provenance chain. A query result derived from commons inputs propagates the *union* of
  their licence obligations; the engine refuses to emit a result whose licence obligations it
  cannot satisfy, rather than dropping them silently.
- **R20 — The private↔commons boundary is enforced, both directions.** A private asset is never
  published into a commons graph without an explicit, authorized, revocable contribution act
  (a governed Update, not a query side-effect); a commons asset is never re-licensed to
  "private/all-rights-reserved" by materialization or derivation. Sensitivity lanes (§8) and the
  licence obligations of §R19 travel together into every derived/materialized asset.
- **R21 — Commons contribution is an accountable, reversible act.** Planting an asset into a
  shared world is an ed25519-signed, WAL-recorded contribution (Chora's `publish_planted_asset`
  is the seed) with contributor DID, licence grant, and an invalidation/withdrawal path —
  consistent with §12's promotion boundary. Contribution ≠ surrender of authorship.
- **R22 — Commons federation is licence- and provenance-aware.** A federated `SERVICE` (§7.3)
  against a commons endpoint negotiates licence compatibility and attribution obligations
  alongside capability, and refuses to import an asset whose terms it cannot honour. External
  open-data adapters (WMS/STAC/GBIF/OSM/IVOA/CKAN, `domains/geospatial/adapters/`) become
  provenance-preserving *commons ingestion* paths: today most build a correct request but do not
  yet convert responses into provenance-carrying `NQuin`s/`.10d` — that conversion is the QISP
  commons-ingestion adapter, and it must stamp source, licence, retrieval time, and digest.
- **R23 — Commons queries degrade to open standards.** A commons endpoint answers a generic RDF
  client (§7.4) *and* is discoverable as open data (a `void:Dataset` / DCAT description, an OGC
  service where geospatial). The commons is the interoperability face of the system: it should be
  the *easiest* part to consume with plain SPARQL 1.1 and standard licences, because its purpose
  is to be shared.

Non-goal to add to §14: **the commons must never become a laundering path** — private, denied,
or sensitivity-classified data must never reach a commons graph, cache, or materialized view, and
a commons licence must never be silently upgraded to a restrictive one.

---

## 15c. Opportunities beyond interop — the remarkable version

v0.2 is a rigorous *interop* layer. The remarkable version is QISP as the query **spine** that
unifies Qualia subsystems that already exist (audited real, dated — not vendored) and that no
other RDF engine combines. Each is an *adapter* onto the typed function registry (§4.2) and the
authorization join (§7), reusing a canonical owner (§10.2) — never a fork. Ordered by leverage:

1. **Certified, content-addressed WGSL kernels (via `wgsl_forge`).** §6.3 asks for "GPU kernels
   admitted through capability manifests with CPU-oracle tests." `wgsl_forge` (22.6k lines)
   *already* certifies every kernel against an exact CPU differential oracle before use and emits
   to WGSL/HLSL/MSL/PTX/CUDA-C/SPIR-V. Make the **shader a QISP asset**: content-addressed, with a
   provenance record of which oracle it passed, its error bound, and backend. Then §4.4 exactness
   profiles become *proofs of certification*, not promises — and because it is WGSL, the *same
   certified kernel runs in the browser via WebGPU*, so the immersive layer is portable to the web
   with byte-identical, oracle-checked compute. ("Write once, certified, runs native + web.")
2. **Zero-knowledge policy proofs.** §8.6 uses redaction/minimum-counts. `crypto/zk_proofs.rs` is
   *real* Groth16 over BLS12-381 (soundness-verified, `deontic_access` circuit). A query can return
   a **zk proof that "the result satisfies policy P" / "the subject is within the zone" without
   disclosing the coordinates** — turning `qisp:proofDigest` from a hash into an actual proof. This
   is the anti-surveillance/human-rights differentiator.
3. **Natural-language query, grounded-or-refused.** No NL layer exists in the plan; Qualia has
   in-process LLM inference *and* the mandatory `validate_output` grounding gate (≥1 provenance
   citation or reject). A person asks their graph in plain language; the system authors the QISP,
   runs it, and the answer is **grounded in provenance or refused — never hallucinated.**
4. **Authorization as rights logic, not config.** §8 checks VCs against "policy." Qualia has real
   Hohfeldian jural relations (`jural.rs`), STIT (`stit.rs`), duress-aware capacity (`capacity.rs`
   — voidable at the victim's election), and the N3Logic Rights Ontology VM (UDHR/ICCPR/CRPD). A
   `qisp:AuthorizationDecision` becomes "a privilege correlative to a duty, granted with capacity,
   not under duress," evaluable against actual rights instruments — legal-grade accountability.
5. **Multi-modal rendering — σ heard as well as seen.** The plan is visual. σ (the 10th Tensor10D
   axis) is *shared EMF truth* that `render::spectral` (colour) **and** `render::acoustic` (pitch)
   both encode, and `ComputeUniverse` reserves an `AcousticPlane` zone. A query result rendered as
   **sound** is both a novel immersive modality and the accessibility answer §8.9 requires (a blind
   person *hears* the result).
6. **Provable time-travel.** §9 is cache invalidation; Qualia has `AS OF`/`AT TIME` parsing + a
   SHA-256 Merkle-DAG history (`platform/git_bridge.rs`). "Query my graph as it was, *provably*" —
   the substrate for erasure-prevention and post-death continuity.
7. **Domain compute as QISP function families.** The plan's functions are geo/tensor; Qualia has 9
   real domain libraries + 178 solvers (clinical scores, DFT, portfolio VaR, …). Exposed as typed
   functions with provenance, the immersive canvas visualizes *any* domain, not only geometry.

**Thesis.** The plan's closing line — "RDF names the world; compact assets carry its dense form;
bounded kernels compute over it; provenance and policy make the result accountable" — under-claims
what is already built. Add three words: **certified, private, accountable-as-rights.** Every result
computed by an oracle-certified kernel that runs native *and* in-browser; disclosable as a
zero-knowledge proof rather than raw data; authorized against actual human-rights logic; answerable
in natural language; rendered to colour *or sound*; over provable, commons-aware history. No RDF
store combines these, and every piece is real (or one bridge away) in the tree.

---

## 15d. Streaming / continuous SPARQL (missing entirely from v0.2 — add as a capability + phase)

An immersive, spatio-temporal system is inherently a *streaming* one: sensor/health telemetry,
live geo layers (NASA GIBS, active-fire/weather feeds), agent trajectories, and the person's own
edits all arrive continuously. v0.2 has request/response query, async jobs (§7.2), materialized
views (§9), and SSE change-notification — but **no streaming SPARQL**: no standing continuous
queries, no window operators, no incremental result deltas.

**Current state (honest).** Continuous/windowed stream reasoning (RSP-QL / C-SPARQL / CQELS) is
**not implemented** — the only "window" is a `WindowType::Tumbling` *data structure* in the
decorative `sparql_mm.rs`, unrelated to continuous graph query. Result *streaming* is **partial**:
`sparql_websocket.rs` chunks results, but only *after* the executor has eagerly materialized a
`Vec<BindingRow>` — there is no lazy/backpressured evaluation. So both senses of "streaming SPARQL"
are open.

**Standards baseline.** SPARQL 1.1/1.2 do not standardize streaming; the reference work is the W3C
RDF Stream Processing (RSP) Community Group — **RSP-QL** (a unifying model over C-SPARQL and
CQELS-QL), with time-based/triple-based **windows** (tumbling/sliding), **stream sources**, and
**continuous evaluation** producing result *streams* (RStream/IStream/DStream — full result,
inserted deltas, deleted deltas). QISP should implement an RSP-QL-aligned extension, advertised as
a distinct draft conformance class — never conflated with stable SPARQL query conformance, and
degrading cleanly (a non-streaming client still runs the one-shot form).

**Reuse (no forks, per §10.2).** SSE graph revisions (`subscribe_graph_revisions`) as the
continuous re-evaluation trigger; the materialized-view dependency-keying and invalidation (§9) as
incremental-maintenance machinery; the WAL/DAG temporal substrate and `AS OF` parsing for window
bounds; `sparql_websocket.rs`'s subscription bookkeeping and chunking for transport; async jobs
(§7.2) for long-lived registered queries; and the daemon telemetry broadcast + geo adapters as
first stream sources.

**Requirements to add (candidate `QISP-R24`–`QISP-R28`):**

- **R24 — Standing queries are explicit, authenticated, and revisioned.** A `REGISTER`-style
  continuous query is a first-class resource (register / status / deregister), scoped to a
  principal, with its stream sources, window spec, output policy, and lifecycle recorded — reusing
  the job-resource state machine (§7.2), not a new silo.
- **R25 — Windows are bounded and declared.** Time-based (`RANGE`/`STEP`) and triple-count windows,
  tumbling or sliding, with a hard cap on window size and retained tuples. A window can never grow
  unbounded; over-budget is an error, not silent truncation.
- **R26 — Output is incremental and honest.** Emit RStream/IStream/DStream (full / inserted /
  deleted) deltas; a consumer is told whether it is receiving a full re-materialization or a delta.
  Deltas are provenance-stamped and monotonic per window instant.
- **R27 — Continuous evaluation is bounded, snapshot-consistent, and backpressured.** Each
  evaluation runs against one pinned dataset revision (§6.2), within per-tick byte/row/invocation
  budgets; a slow consumer applies backpressure (drop-oldest with a reported gap, or bounded
  buffer) rather than growing memory. Evaluation must not allocate on the hot path.
- **R28 — Streaming honours authorization, privacy, and the commons boundary continuously.** Policy
  (§8), sensitivity lanes, licence obligations (§15b), and revocation are re-checked as the stream
  advances; a credential/policy change or expiry stops emission. A denied or private tuple never
  reaches a stream, cache, or materialized view — the §14 laundering non-goal applies per tick.

**Architectural note.** True streaming needs the executor to evaluate *incrementally* rather than
returning `Result<Vec<BindingRow>>`. Options: (a) an iterator/generator execution mode that yields
bindings lazily; (b) incremental view maintenance driven by graph-revision deltas (compute only the
change, not the whole answer). (b) fits the existing materialized-view + SSE-revision machinery best
and avoids a second executor. Either way, hot evaluation stays caller-buffered and zero-heap.

**Phase placement.** Slot as **Phase 8b — Streaming / continuous SPARQL** (after §9 materialized
views and the async job service, whose infrastructure it reuses). Conformance class:
`qisp:StreamingSparql`. Gate (`QISP-R24`–`QISP-R28`): a registered windowed query over a live stream
emits correct incremental deltas under backpressure and budget, stops on policy/credential change,
and never leaks a private/denied tuple; the one-shot `/sparql` path is unaffected.

**Non-goal (add to §14):** unbounded windows, silent buffer growth, at-most-once/at-least-once
ambiguity presented as exactly-once, and any streamed result that outruns its authorization,
licence, or snapshot without saying so.

---

## 15e. Peer-to-peer transport: SPARQL/QISP over WebRTC (add as a transport binding)

v0.2 assumes a client↔server `/sparql` endpoint and HTTP-style federation (§7.3). It has no
**peer-to-peer** transport — yet Qualia's thesis is *personal platform provider, no operator*
(the social-network plan: connection-identifier rendezvous → mutual-auth → WireGuard
SocialWebNet mesh). A person's browser should be able to federate directly with a peer's browser
graph, with no server in the middle. WebRTC is the browser-native way to do that, and it pairs with
the web-portable immersive layer (WGSL/WebGPU) and with §15d streaming (SCTP data channels give
ordered/unordered delivery + backpressure — ideal for streaming result deltas to a peer).

**This is a transport binding, not a new query language.** "QISP over WebRTC data channels" carries
the *same* federation semantics (§7.3), streaming semantics (§15d), authorization (§8), and
commons/private boundary (§15b) — over a peer connection instead of an HTTP endpoint.

**Current state (honest).** Not implemented. The native P2P transport is WireGuard (`boringtun`
present but unused; shells to `wg`); the only WebRTC in the tree is a self-labelled **mock**
`BenchmarkAction::P2pSwarm` peer. Greenfield.

**Reuse (no fork, per §10.2).** Signaling (SDP offer/answer + ICE candidate exchange) reuses the
**connection-identifier / tiered-rendezvous** fabric already designed in the social-network plan —
do **not** stand up a new signaling server. Peer identity reuses the **mutual-auth
challenge-response** ("it's actually Bob"); DTLS encrypts the channel but does not authorize.
Query/stream semantics reuse §7.3 and §15d. WireGuard remains the *native-peer* binding; WebRTC is
the *browser-peer* binding — both behind one transport-agnostic federation layer.

**Requirements to add (candidate `QISP-R29`–`QISP-R33`):**

- **R29 — A data channel is network egress; govern it like remote `SERVICE`.** Opening a peer
  channel and shipping/answering a query is an egress act subject to the same policy gate; it is not
  enabled by default and never carries a process-local handle or pointer (§2.2 point 5).
- **R30 — Authenticate the peer before any query; DTLS ≠ authorization.** The mutual-auth
  challenge-response must succeed (both directions) before a query/update/subscription is accepted,
  and every operation is still VC/policy-scoped per §8 — a verified channel is not a trusted grant.
- **R31 — Signaling must not leak the social graph.** Rendezvous exchanges only what ICE needs;
  it must not disclose a peer's contact list, presence, or graph contents, and it degrades to the
  offline/mailbox path when no direct route exists.
- **R32 — Relay and address-exposure privacy is first-class.** STUN reveals IPs and TURN relays see
  traffic patterns — a metadata leak that matters for an anti-surveillance system. Offer a
  metadata-private path (the **Nym opt-in** from the social-network plan) and never force a
  third-party TURN relay without disclosing it; a relay never sees authorization plaintext.
- **R33 — The commons/private boundary and licence obligations travel P2P.** A private or denied
  tuple never crosses a peer channel; commons assets carry their licence/attribution (§15b) to the
  peer; the §14 laundering non-goal applies per peer exchange.

**Placement.** A transport binding under §7.3 (federation) and §15d (streaming), delivered alongside
the async/streaming work (Phase 8b) once mutual-auth and rendezvous have stable contracts.
Conformance/transport label: `qisp:WebRtcTransport` (browser P2P) beside an implied
`qisp:WireGuardTransport` (native P2P), both advertised in the service description.

**Non-goal (add to §14):** an unauthenticated or default-on peer query surface; signaling that
discloses the social graph; a TURN relay presented as end-to-end private; and any P2P exchange that
outruns its authorization, licence, or snapshot without saying so.

---

## 16. Handoff checklist for each implementation phase

- [ ] Confirm overlap with current uncommitted SPARQL/GeoSPARQL work before editing.
- [ ] Update the QISP profile, ontology, SHACL, examples, and capability graph together.
- [ ] Classify every native operation as `HotZeroHeap`, `ColdBounded`, or async-required.
- [ ] Add correctness, budget, cancellation, determinism, and privacy tests.
- [ ] Run `cargo test -p qualia-core-db --lib` plus affected renderer/Solid/identity tests.
- [ ] Update `AGENTS.md`, `HANDOVER.md`, directory indexes, and session notes only when the
      implementation—not merely this plan—lands.
- [ ] Publish conformance claims only for passing profile modules.

The architectural invariant is simple: **RDF names and explains the world; compact assets
carry its dense form; bounded native kernels compute over it; provenance and policy make
the result accountable.**
