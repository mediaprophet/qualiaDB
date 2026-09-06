# QDNF Ontologically Defined Contracts over CBOR-LD

**Status:** Normative design 0.1; proposed contract profile
**Date:** 2026-09-05

## 1. Role

QDNF contracts are ontology-defined linked-data graphs carried as **CBOR-LD**. Their terms identify
parties, resources, purposes, permissions, prohibitions, duties, quantities, funding, and settlement
conditions. The ontology supplies meaning; CBOR-LD supplies the compact representation. Signatures,
validation, ratification, and QPolicy execution supply distinct checks.

This contract profile serves commons agreements, service quotes, resource leases, contribution
receipts, and related governance objects. Its first consumer is
[Commons and Resource Economics](./commons-and-resource-economics.md). The same representation can
support nonmonetary agreements without requiring a payment service.

QFrame headers and bounded routing-control envelopes retain their fixed layouts. Contract payloads
MUST negotiate `qdnf:feature:ontological-contracts` and its exact major/schema profile, and use its
CBOR-LD representation. Major 1 is the initial classical record profile. The
[PQ replacement](./post-quantum-security.md) requires separately versioned dual-proof and typed
SHA-384 commitments, with new vectors; it cannot reinterpret major-1 signature/digest bytes.
Plain CBOR with arbitrary integer fields is not a substitute for a negotiated semantic contract.

## 2. Semantic contract bundle

A contract binds an immutable semantic bundle by a full cryptographic digest:

| Artifact | Purpose | Required binding |
|---|---|---|
| JSON-LD context and dependencies | Expand terms, datatypes, language, and container mappings | Exact content digest and declared context identifiers |
| Ontology modules | Define parties, duties, quantities, and domain relationships | Version/content digest and explicit import closure |
| SHACL shapes | Validate required properties, cardinality, types, and constraints | Shape graph digest and supported validation profile |
| N3 rules | Derive applicable duties, exceptions, discharge, and threshold transitions | Ruleset digest and bounded evaluator/entailment profile |
| CBOR-LD compression tables / Q42 lexicon mapping | Map compact codes to full terms and typed values | Exact table/registry digest and codec profile/version |
| Agreement terms | State the accepted values, permissions, and obligations | Signed contract bytes, contract ID/version, and predecessor |

The manifest lists the complete dependency closure; default limits are 16 artifacts, 1 MiB total
uncompressed artifact bytes, and the existing nesting-depth ceiling of 8. Expanded graphs and
compiled rules have separately declared node, edge, instruction, output, and workspace limits;
small compressed input does not authorize unbounded expansion. The total pass remains within 42 MiB.

Contexts are resolved from bundled artifacts, Q42 storage, or an already verified local cache. Cold
acquisition may use authorized QResolve/QSync content fetches with explicit budgets. Validation and
packet processing MUST NOT fetch mutable HTTP contexts or dereference arbitrary ontology IRIs.
An HTTPS vocabulary IRI can name a term without requiring DNS/HTTP to interpret a pinned bundle.

Missing artifacts return a bounded dependency requirement on the authenticated negotiation channel;
they do not permit delivery or debit. Unknown rules, mappings, or critical terms prevent acceptance
of the affected contract. Translation of labels is allowed; silent translation of obligations into
different ontology terms is not. A mapping between ontology versions is itself a reviewed, pinned
artifact, and changed contractual meaning requires new acceptance.

## 3. Ontology and unit semantics

Use explicit terms for:

- natural person, organization, steward, service provider, operator, agent, delegate, and beneficiary;
- target resource, action, purpose, context, permission, prohibition, duty, and consent reference;
- quantity kind, unit, integer coefficient/scale, evidence state, scope, and measurement interval;
- price, asset/issuer, rate basis, funding allocation, contribution, cap, and remaining obligation;
- quotation, acceptance, delivery evidence, settlement state, finality, adjustment, and closure; and
- decision authority, appeal, amendment, withdrawal, expiry, and licence transition.

Energy and time use the joule and second as reference units. Device time, elapsed time, human work,
airtime, and availability have distinct quantity kinds even when their unit is the same. Rate
terms identify both the settlement unit and the resource quantity in the denominator. SHACL and
the bounded arithmetic validator reject incompatible dimensions; converting a quantity into a price
requires an accepted valuation rule.

The profile SHOULD map suitable permission/duty terms to
[W3C ODRL](https://www.w3.org/TR/odrl-model/) and validate RDF graphs with an explicitly supported
[SHACL profile](https://www.w3.org/TR/shacl/). Local ontologies can add cultural, cooperative, or
domain terms. A contract MUST state which semantics are required; an implementation cannot drop
an unfamiliar duty and accept the remaining graph. Passing SHACL establishes declared graph
constraints, not signature validity, evidence truth, spending authority, or payment finality.

Agent and operator roles remain separate. Signing with an agent key does not identify its human
principal or prove delegation. Contextual party identifiers and their authority proofs are scoped
to the agreement; a semantic match or `owl:sameAs` assertion cannot widen that authority.

### 3.1 Readable example

This JSON-LD authoring fragment illustrates a sponsor-funded energy/time allowance. CBOR-LD encodes
the same linked terms using the pinned codec/table profile. The `qdnf:econ:` vocabulary below is
proposed, not an already published ontology. A complete contract also needs the bundle manifest,
party authority, purpose, validity, shapes/rules, and acceptance; this fragment is not a spend grant.

```json
{
  "@context": {
    "econ": "qdnf:econ:",
    "xsd": "http://www.w3.org/2001/XMLSchema#"
  },
  "@id": "urn:example:commons-agreement:1",
  "@type": "econ:CommonsAgreement",
  "econ:fundingMode": {"@id": "econ:CommunityFunded"},
  "econ:sponsor": {"@id": "urn:example:community-pool:1"},
  "econ:resource": {"@id": "urn:example:translation-catalogue:1"},
  "econ:energyLimit": {
    "@id": "urn:example:commons-agreement:1:energy-limit",
    "@type": "econ:ResourceLimit",
    "econ:quantityKind": {"@id": "econ:Energy"},
    "econ:unit": {"@id": "econ:Joule"},
    "econ:coefficient": {"@value": 120, "@type": "xsd:nonNegativeInteger"},
    "econ:scale": {"@value": 0, "@type": "xsd:integer"}
  },
  "econ:timeLimit": {
    "@id": "urn:example:commons-agreement:1:time-limit",
    "@type": "econ:ResourceLimit",
    "econ:quantityKind": {"@id": "econ:ElapsedTime"},
    "econ:unit": {"@id": "econ:Second"},
    "econ:coefficient": {"@value": 6, "@type": "xsd:nonNegativeInteger"},
    "econ:scale": {"@value": 0, "@type": "xsd:integer"}
  }
}
```

Limits are prescribed allowances; they are not measured usage. A linked ResourceUsage record
separately reports measured, estimated, or unknown quantities and their evidence. A sponsor's
funding acceptance is separately verified before any debit to that pool.

## 4. Encoding, signatures, and semantic identity

The interoperability reference is the
[CBOR-LD 1.0 Working Draft of 19 August 2026](https://www.w3.org/TR/2026/WD-cbor-ld-10-20260819/).
It is a Working Draft, so the QDNF profile pins an exact revision and encoding/table choices before
freezing its test vectors. Q42 term compaction and compliance with that draft require an explicit
mapping and interoperability evidence; the shared name “CBOR-LD” does not prove identical formats.

The signed record contains a small deterministic-CBOR wrapper with contract-profile/version,
semantic-bundle digest, operation/audience bindings, and the **exact CBOR-LD payload bytes**. It uses
the [COSE_Sign1 profile](./cryptographic-profile.md#9-stored-signatures-and-cose), with an explicit
contract-record type and version in the protected signature context. The payload and interpretation
bundle are covered together; changing a compression table or context invalidates the binding.

Verification MUST use the received signed bytes. It MUST NOT expand, relabel, recompact, or serialize
the graph again and then assume the old signature covers those different bytes. Deterministic CBOR
encoding does not by itself canonicalize all semantically equivalent RDF graphs. A future graph-
equivalence digest requires a separately versioned canonicalization profile and vectors. Signed
byte identity remains the base record identity; alternate authoring forms are not automatically
the same acceptance or payment instruction.

The codec profile specifies tags, term codes, compression tables, ordering, datatypes, and rejected
constructs. Contract node/resource identifiers are absolute; obligation-bearing nodes have explicit
IDs. Ordered conditions use explicit list semantics. Preserve named graph scope, language tags,
datatypes, and set/list distinctions across expansion and compaction. Unknown terms may survive as
inert extension data only when the pinned shapes/rules explicitly classify them as nonoperative.

`q_hash` values accelerate dispatch after full-IRI and mapping verification. They are not global
ontology identifiers, content digests, signature evidence, or a licence to reinterpret a collision.
Persist the full semantic source and bundle references alongside compiled NQuin indexes.

## 5. Validation and execution

```text
bounded framing + profile negotiation
  -> signature / issuer / audience / replay checks
  -> pinned context, ontology, table, shapes and rules verification
  -> bounded CBOR-LD expansion and datatype/unit checks
  -> SHACL validation and authorized N3 rule compilation
  -> agreement ratification + capability/consent + funding reservation
  -> compiled QPolicy decision for the exact operation
  -> bounded service execution and linked CBOR-LD receipts
```

All stages retain block and revocation precedence. The source graph is data, not an instruction to
run arbitrary code. Only locally supported, explicitly authorized N3/SHACL constructs are evaluated.
Unknown built-ins, recursive imports, unsupported entailment, resource exhaustion, and ambiguity
produce a typed non-allow outcome. Open-world absence does not become consent; the policy profile
must explicitly define the facts required to authorize an action.

Cold compilation uses bounded workspaces to produce fixed-size policy/obligation handles for the
existing deontic, epistemic, paraconsistent, LTL, and M-of-N mechanisms as appropriate. Hot evaluators
consume caller-owned buffers and perform no ontology loading, string allocation, or network fetch.
The 48-byte NQuin ABI and canonical opcode/type-tag allocations remain unchanged.

Acceptance binds the graph bytes, semantic bundle, evaluator profile, and quote version that the
parties reviewed. Receipts link that acceptance and the resulting decision digest. Later context,
ontology, rate, or rule updates cannot silently reprice or reinterpret active contracts. Amendments
have a predecessor and new acceptance under the stated authority; historical terms remain verifiable.

Contract bundles, signed acceptance, and linked receipts use the
[QualiaDB/Q42 storage lifecycle](./core-storage-and-cache.md). Preserve exact signed bytes alongside
indexed NQuin projections and pin the immutable storage generation for each verified handle. The
contract service adapts the existing core; it does not introduce another ontology database.

## 6. Negotiation and failure behavior

`qdnf:feature:ontological-contracts` is required for a channel exchanging these contracts. The
economics profile additionally requires its own agreed feature. Neither feature adds a QFrame type
nor permits QPolicy bypass. Unsupported contract semantics stop the affected governed operation;
ordinary authorized chat, donated connectivity, and other compatible services may continue.

The initial design permits authenticated dependency negotiation within small quotas before an
economic obligation is accepted. It forbids silent fallback to plain CBOR/JSON, a different ontology,
or hash-only term interpretation. A legacy importer must expose its semantic mapping and produce
a new, explicitly accepted record rather than reusing the legacy signature as native authority.

## 7. Required implementation evidence

Before interoperable contract support is advertised, demonstrate:

1. independent CBOR-LD encode/decode vectors with the exact selected draft and table bundle;
2. byte-exact signature verification plus preservation of expanded IRIs, datatypes, lists, and graphs;
3. rejection of replaced context/ontology/table bytes, missing imports, incompatible codec, and
   unknown obligation-bearing terms without a plain-CBOR downgrade;
4. identical validated policy results with networking disabled and all dependencies bundled;
5. energy/time/rate dimensional validation, including unknown measurements and overflow;
6. bounded expansion, SHACL/N3 compilation, and zero-heap execution under adversarial inputs;
7. no implicit authority from ontology equivalence, relationship claims, or an agent's signature; and
8. amendments cannot reinterpret prior acceptance, replay payment, or erase disputed receipts.

[Source and Current-Stack Review](./source-and-current-stack-review.md) records existing code and
remaining gaps. The [implementation plan](./implementation-conformance.md) owns schema, ontology,
compiler, and interoperability work; this document does not claim those are already implemented.
