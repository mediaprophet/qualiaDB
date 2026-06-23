# CML — Context Markup Language

**Status:** Draft, tracking the implemented QualiaDB design (this supersedes the earlier
pre-implementation drafts in this folder; where they differ, **the implemented solution governs**).
**Namespace:** `cml:` = `https://ns.webcivics.net/cml/` (the term *identifiers*). The published
schema-definition ("sdo") documents resolve at **`https://sdo.webcivics.org/`**. The authoritative
machine axioms are [`core-ontologies/cml.n3`](../../../../../core-ontologies/cml.n3); architecture in
[`CML_CONCEPT_GRAPH.md`](../../../../../core-ontologies/CML_CONCEPT_GRAPH.md).
**Schema version:** `2` (see [`CML_VERSIONS.md`](../../../../../core-ontologies/CML_VERSIONS.md)).

> **What changed from the early draft.** CML was first sketched as inline `<cml:context>` HTML tags
> (and an external `.ctx` JSON sibling) for LLM term-disambiguation. The implemented system keeps the
> *purpose* — declarative, machine-readable context so agents infer meaning accurately — but realises
> it as a **concept-graph layer native to the engine**, not as a new HTML element set. The disambiguation
> use case survives as `skos:closeMatch`/`exactMatch` from a concept to external entities
> (Wikidata/Schema.org). The inline-element and `.ctx`-file proposals are **superseded**.

---

## 1. What CML is

CML is the layer that binds **verbatim source text → concepts → logic** on the bare-metal Qualia
engine. Its load-bearing idea:

> **A concept *is* a context hash.** A CML concept's IRI is hashed (`q_hash(IRI)`) into the 56-bit
> context field (bits `[0..55]`) of the 48-byte **Super-Quin** (`NQuin` Vector 4). The concept's
> "sub-graph" is *every quin whose `context` equals that hash*. This is the engine's native
> named-graph mechanism — no new machinery: *a document is a query for all quins intersecting a
> context hash.*

`cml:Concept` is `rdfs:subClassOf skos:Concept` — deliberately **RDFS + SHACL, never OWL** for the
given world (people, nature): a concept is *described and validated*, never *inferred into being*
(§5).

## 2. The three-layer model (nanopublication-aligned)

```
  TEXT      verbatim originalText           immutable; cold in the .q42 front-of-file lexicon (.lex)
   │   ▲  cml:realizedBy  (W3C Web Annotation: target = media-fragment selector + integrity hash)
   ▼   │
  CONCEPT  a concept identifier = a Context hash (Vector 4) that OWNS a sub-graph (skos:Concept)
   │       many text fragments — across instruments, languages, amended text — may realise ONE concept
   ▼
  LOGIC    per-concept modality sub-graph(s): Deontic · Temporal · Defeasible · Argumentation · …
           executed by the Sentinel VM (webizen.rs), masked on the concept's Context hash
```

- **TEXT** — `values:originalText` literals, stored cold and recovered through the front-of-file
  `Q42LEX`; never edited (legal fidelity / admissibility). The unit of provenance.
- **CONCEPT** — a first-class node, *represented natively as the context hash*; carries
  `skos:prefLabel`/`skos:definition` and relates via SKOS (`broader`/`narrower`/`related`).
- **LOGIC** — modality sub-graphs bound to the concept's context, evaluated by the deterministic VM
  (the deontic sub-graph compiles via `compile_norm_quin` → `evaluate_deontic_contract`).

## 3. Vocabulary (`cml.n3`)

| Term | Kind | Meaning |
|---|---|---|
| `cml:Concept` | `rdfs:subClassOf skos:Concept` | A concept node, realised as an NQuin context hash that owns a logic sub-graph. |
| `cml:realizedBy` | property | Concept ← a verbatim text span (an `oa:Annotation`: RFC 5147 `#char=` + `oa:TextQuoteSelector` + integrity hash). MANY realizations → ONE concept. |
| `cml:integrityHash` | property | Hash of the selected span, verified against the recovered `originalText`. |
| `cml:CurationStatus` · `cml:Proposed` · `cml:Attested` | class / instances | Provisional (machine) vs confirmed (signed authoritative human). |
| `cml:curationStatus` · `cml:proposedBy` · `cml:attestedBy` · `cml:confidence` | properties | The curation record: who proposed (`prov:wasGeneratedBy`), who ratified (`prov:wasAttributedTo` + `sec:` proof), and the machine confidence ∈ [0,1]. |
| `cml:RecognitionBasis` · `cml:recognitionBasis` | class / property | RPL is first-class: `cml:Formal` / `cml:PriorLearning` / `cml:Experiential` / `cml:PeerAttested`. |
| `cml:Modality` · `cml:asserts` · `cml:modality` | class / properties | Binds a concept to a logic assertion in a modality sub-graph (`cml:Deontic`/`Temporal`/`Defeasible`/`Argumentation`/`Spatial`/`Probabilistic`). The asserted quins carry `context = q_hash(concept IRI)` — the hash IS the sub-graph the VM masks on. |
| `cml:schemaVersion` · `cml:generatedBy` · `prov:wasDerivedFrom` | properties | Per-generated-concept stamps (reproducibility / staleness — §6). |

## 4. The Curation Prime Directive (load-bearing)

> **Automated systems may assert only `skos:closeMatch` or `skos:related`. Only a
> cryptographically-signed human action may assert `skos:exactMatch`.**

Concept identity — *when are two clauses the same concept?* — is curation-grade: auto-asserting
sameness from vector proximity would recreate the algorithmic flattening this system exists to
defeat (e.g. Convention 105 forced-labour ≠ the 1926 Slavery Convention). The machine **proposes**
(`cml:Proposed`, `skos:closeMatch`, with a confidence); a human **disposes** (`cml:Attested`,
`skos:exactMatch`, routed through the governance tier with a `sec:` signature). It is *self-policing*:
`cml:AttestedExactMatchShape` (SHACL, closed-world) makes a machine-written `exactMatch` without a
ratifying attestation a conduct violation (`agent-accountability.n3 UnsubstantiatedClaimFlag`).

## 5. Modeling discipline — the OWL boundary + SHACL firewall

- **World of man (constructed)** — legal persons, institutions, treaties, contracts, software agents,
  artifacts — **may** use OWL (they exist because we declared them).
- **Natural world, including humans (given)** — natural persons, ecology, observations — **RDFS +
  SHACL only, never OWL**. A person is not an `owl:Thing` to be classified into existence
  (`q42:Principal` is `rdfs:Class`; the `sh:not owl:Thing` guards *are* the boundary).
- **External mappings carry reference, not ontological commitment** — a `skos:closeMatch` to
  `schema:Physician` adopts the *identifier* for interop/disambiguation but never inherits its class
  axioms (never `Physician ⊑ Place`). The boundary is a bidirectional firewall.
- **Two enforcement layers compose:** the **Deontic + Temporal VM** answers *is this norm binding
  now?* (Active / Defeated / Expired); the **SHACL firewall** answers *does reality comply?* — and a
  shape is applicable only while its norm is Active. The VM adjudicates validity; SHACL enforces
  compliance.

## 6. Layered outputs + reproducibility

Per instrument the corpus emits three layers (see
[`core-ontologies/README.md`](../../../../../core-ontologies/README.md)):

1. **`*.n3`** — foundational N3/Turtle + N3Logic rules + SHACL shapes (**SOURCE**, human-curated).
2. **`*.cml.html`** — the CML-annotated human-readable rendering (**GENERATED**).
3. **`*.q42`** — the engine-native compiled volume (**GENERATED**).

The generated concept layer (`concepts/*.n3`) is reproducible from SOURCE at the stamped
`SCHEMA_VERSION` via `tools/reprocess_library.py` (full, or `--check` as a CI staleness gate). The
**SOURCE/GENERATED separation** is what makes regeneration non-destructive: regeneration rewrites
only the machine-`cml:Proposed` layer and **never** touches human `cml:Attested` attestations or
curation overlays. (At v2: 101 instruments → 3,518 concepts / 3,619 norms, initially all
`cml:Proposed`.)

## 7. Serialization & the engine mapping

- **SOURCE / interchange:** RDF / N3 / Turtle (the `cml.n3` axioms + `concepts/*.n3`).
- **Wire:** **CBOR-LD** is the primary transmission serialization (`@context` expansion = one
  hash-space).
- **Execution:** the **Unified `.q42`** archive — `[0..256)` header → `lex` → `bidx` → block-dir →
  LZ4 40 KB SuperBlocks — preserving a two-step range fetch (fetch the small `bidx`; one HTTP Range
  for the single 40 KB block). A concept's context hash selects its sub-graph for the Sentinel VM.

## 8. Relationship to the rest of the suite

- **[CMLD](../CMLD/README.md)** — the *definitions* layer: bindings of a surface **term → a context
  DID / concept hash**, resolved against the dynamic Ontology Registry. CML defines the concept graph;
  CMLD maps the words people write onto it.
- **Contextual Workspace** ([`contextual_workspace_spec.md`](contextual_workspace_spec.md)) — the QApp
  that authors over these context hashes (deep bidirectional transclusion: "render all quins
  intersecting Context A, plus transclusions from B and C").
- **`values:`** (`https://ns.webcivics.net/values/`) — the human-rights agent lattice + deontic
  vocabulary the LOGIC layer asserts over.

---

*Authorship: Timothy C. Holborn (WebCivics). This revision was produced with AI tooling as an
instrument, tracking the implemented `core-ontologies/cml.n3` design; the tool is not an author.
`cml/notes.md` is retained as superseded pre-implementation background.*
