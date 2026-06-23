# CMLD — Context Markup Language Definitions

**Status:** Draft, tracking the implemented QualiaDB design (this supersedes the earlier
pre-implementation draft; where they differ, **the implemented solution governs**).
**Namespace:** `cml:` = `https://ns.webcivics.net/cml/` (shared with [CML](../cml/README.md)).
**Runtime surface:** `upsert_cmld_definition(term, context_did)`
([`qualia-client-core/src/api.rs`](../../../../../crates/qualia-client-core/src/api.rs)), surfaced as
a Webizen Desktop command; CML/CMLD URIs are resolved against the **dynamic Ontology Registry** at
ingest ([`engine/ingestion.rs`](../../../../../crates/qualia-client-core/src/engine/ingestion.rs)).

> **What changed from the early draft.** CMLD was first sketched as an *external JSON file* (`.ctx`,
> referenced via `<link rel="context">` and `data-cml-id`). The implemented system keeps the
> *purpose* — reusable, referenceable context definitions distinct from the inline content — but
> realises it as **definitions in the engine's Ontology Registry that bind a term to a context DID
> (concept hash)**, not as a sidecar file format. The `.ctx`/`data-cml-id` proposal is **superseded**.

---

## 1. What CMLD is

Where [CML](../cml/README.md) defines the **concept graph** (a concept *is* a context hash), **CMLD
is the definitions layer that maps the surface terms people write onto those concepts.** A CMLD
definition is a binding:

```
term  →  context DID  (= a CML concept = q_hash(concept IRI) = the NQuin context hash)
```

So CMLD answers: *"when this word appears, which concept (context sub-graph) does it mean?"* — the
disambiguation purpose of the original proposal, now resolved through the engine rather than a file.
A "Context Markup Language **Document**" is, in these terms, the **set of CMLD definitions** that give
a body of content its declared meanings, layered over the immutable TEXT.

## 2. Runtime model

- **Authoring:** `upsert_cmld_definition(term, context_did)` records that a `term` resolves to a
  `context_did`. The `context_did` denotes a CML concept (its IRI hashes to the concept's context
  field, NQuin Vector 4, bits `[0..55]`).
- **Resolution (ingest):** when content carrying CML/CMLD references is ingested, those URIs are
  resolved against the **dynamic Ontology Registry** — the term's declared `context_did` selects the
  concept's sub-graph, so an agent reads the *intended* meaning (and the logic bound to it) rather
  than guessing.
- **One hash-space:** because a CMLD definition's `context_did` is exactly the CML concept's context
  hash, a definition *joins* directly into the concept graph and its LOGIC sub-graphs — no separate
  index, no duplication.

> **Implementation status (honest).** The registry API and the ingest-time resolution path exist as
> the engine surface; full persistence/round-trip of CMLD entries and the PDF/VLM ingest path are
> being wired (some current returns are surface-level). This document specifies the *design the
> implementation embodies*, not a claim of end-to-end completeness.

## 3. Curation status applies

CMLD definitions are **CML-governed**: a definition is `cml:Proposed` when machine-suggested and
`cml:Attested` only on a signed authoritative human action. A term→concept binding that asserts
*sameness* (`skos:exactMatch`) is subject to the **Curation Prime Directive** and the
`cml:AttestedExactMatchShape` SHACL firewall exactly as in CML — machines may propose `closeMatch`;
only a human attests `exactMatch`. (See [CML §4](../cml/README.md).)

## 4. Relationship to CML

| | CML | CMLD |
|---|---|---|
| Defines | the **concept graph** (concept = context hash; TEXT→CONCEPT→LOGIC) | the **definitions** mapping terms → concepts |
| Unit | a `cml:Concept` (an `skos:Concept` + context hash) | a `term → context_did` binding |
| Lives in | `core-ontologies/cml.n3` + `concepts/*.n3` → `*.q42` | the dynamic Ontology Registry (`upsert_cmld_definition`) |
| Answers | *what is this concept, and what logic binds it?* | *which concept does this word mean?* |

## 5. Serialization

- **SOURCE / interchange:** RDF / N3 (a CMLD definition is RDF over the shared `cml:`/`skos:`
  vocabulary — `skos:closeMatch`/`exactMatch` from a term/label to a concept, with
  `cml:curationStatus`).
- **Wire:** **CBOR-LD** (one hash-space; `@context` expansion).
- **Execution:** resolved into the same context-hash space as CML, consumed by the Ontology Registry
  and the Sentinel VM.

---

*Authorship: Timothy C. Holborn (WebCivics). This revision was produced with AI tooling as an
instrument, tracking the implemented `upsert_cmld_definition` / Ontology-Registry design and the
`core-ontologies/cml.n3` axioms; the tool is not an author.*
