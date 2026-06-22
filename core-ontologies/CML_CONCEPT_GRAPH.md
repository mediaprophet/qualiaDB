# CML Concept-Graph Architecture

**Status:** architectural foundation (structural, immutable). Companion to `PLAN.md` §5/§5a.
**Scope:** how the values-credentials corpus binds *verbatim instrument text* → *concepts* →
*logic* on the bare-metal Qualia engine, within the 512 MB RAM floor.
**Out of scope (deliberately):** the heuristics/thresholds by which a machine *proposes* a
`skos:closeMatch` — that is a policy + tuning problem for a later **Neuro-Symbolic Sieve**
specification, not a structural one. This note locks the structure so ingestion can proceed.

Legend: ✅ built today · 🔶 proposed by this note · ⏭ later spec.

---

## 0. Why this layer exists

After faithful Turtle ingest (`PLAN.md` §5), each article is still **one opaque `originalText`
blob** with a *whole-article* heuristic deontic tag. That cannot express *which* duty binds *whom*
under *what* condition — and worse, naive de-duplication across instruments would **flatten**
legally distinct norms (Convention 105 forced-labour ≠ the 1926 Slavery Convention). The concept
layer makes the *clause-level norm* a first-class, governable object while keeping the source text
verbatim.

---

## 1. Hardware alignment (why this is efficient, not just elegant)

The Qualia record is a 48-byte **Super-Quin** — six `u64` vectors `[S, P, O, C, M, parity]`:

| Vector | Field | Role here |
|---|---|---|
| 1 | `subject` | concept / clause / agent id |
| 2 | `predicate` | relation (deontic op, SKOS, PROV) |
| 3 | `object` | value / class / term |
| **4** | **`context`** | **the Concept hash** — 56-bit graph/context id (bits [0..55]) |
| 5 | `metadata` | role-keyed flags (routing tier, defeater bit, modality) |
| 6 | `parity` | XOR-fold checksum |

**The decisive mapping: a Concept *is* a Context hash (Vector 4).** A concept's "sub-graph" is
simply every quin whose `context` equals that hash. This is not new machinery — it is the engine's
native named-graph mechanism (the CML/Contextual-Workspace model: *"a document is a query for all
quins intersecting a context hash"*). The Sentinel VM (`webizen.rs`) evaluates a modality by
**masking on the context hash** and firing the rules registered for it — Deontic, Temporal,
Defeasible logics swap in/out against the same concept without loading the whole graph. ✅

**The file shape that keeps it under 512 MB.** A Qualia database was originally a *folder*:

- `.lex` — lexicon: maps heavy string URIs/literals → localized `u64` (semantic compression).
- `.bidx` — block index: byte-offsets + hash ranges for the 40 KB SuperBlocks (search / HTTP Range).
- `.q42` — the LZ4-compressed 40 KB SuperBlocks (the data).

These merged into the **Unified `.q42`** — `.lex` and `.bidx` now sit uncompressed at the **front of
the file as a Master Header** (`[0..256)` header → `lex` → `bidx` → block-dir → data). ✅ This
preserves the **two-step fetch** natively from one self-contained archive: (1) fetch the small
header/`bidx`, binary-search it for the block holding a hash; (2) issue one HTTP Range request for
that single 40 KB SuperBlock and decompress only it — never the whole 200 MB corpus.

> As of 2026-06-22 the streaming ingest **populates** the front-of-file lexicon, so literal text is
> recoverable from the `.q42` alone (verified: Article 1's text round-trips verbatim). The TEXT
> layer below depends on exactly this. ✅

---

## 2. The three-layer model (nanopublication-aligned)

```
  TEXT      verbatim originalText                          immutable, cold in the .lex lexicon
   │          ▲  realized-by  (Web Annotation: target = media-fragment selector)
   ▼          │
  CONCEPT   a concept identifier = a Context hash (Vector 4) that OWNS a sub-graph
   │          (skos:Concept · stable · many text fragments may realize one concept)
   ▼
  LOGIC     per-concept sub-graph(s): Deontic · Temporal · Defeasible · Argumentation · …
            executed dynamically by the Sentinel, masked on the concept's Context hash
```

This is the **nanopublication** pattern exactly: an *assertion* graph (LOGIC) + a *provenance*
graph (the `realized-by` fragment + attribution) + *pub-info* (the curation signature, §4). Each
layer maps to engine reality:

- **TEXT** ✅ — `originalText` literals, stored cold and recovered through the front-of-file
  `Q42LEX`. Never edited (legal fidelity / admissibility). The unit of provenance.
- **CONCEPT** 🔶 — a first-class node, **represented natively as the Context hash**. Concepts carry
  `skos:prefLabel`/`definition` and relate via SKOS (`broader`/`narrower`/`related`). One concept
  may be `realized-by` many fragments (across articles, instruments, languages, the universalised
  `amendedText`).
- **LOGIC** 🔶 — modality sub-graphs bound to the concept's context. The **Deontic** sub-graph is
  already executable: it compiles via `deontic::compile_norm_quin` → evaluates via the
  `values_evaluate` path I wired (Active / Defeated / Expired). ✅ Temporal (LTL/CTL/Allen),
  Defeasible (`q42:unless`), Argumentation, etc. attach the same way.

---

## 3. SKOS / Web Annotation binding

**Concept vocabulary** (SKOS):
```turtle
concept:DutyToSuppressForcedLabour
    a            skos:Concept ;
    skos:prefLabel "duty to suppress forced labour"@en ;
    skos:definition "A ratifying party's obligation to suppress and not make use of forced labour." .
```

**Text → concept link** (W3C Web Annotation — `body` = concept, `target` = fragment selector). Carry
**multiple co-selectors** so the anchor self-heals if text is re-sourced/re-segmented:
```turtle
[] a oa:Annotation ;
   oa:hasBody   concept:DutyToSuppressForcedLabour ;
   oa:hasTarget [ oa:hasSource <…convention-105#article-1> ;
                  oa:hasSelector
                    [ a oa:FragmentSelector ;  oa:conformsTo <rfc5147> ; rdf:value "char=0,87" ] ,
                    [ a oa:TextQuoteSelector ; oa:exact "Each Member … undertakes to suppress …" ] ;
                  values:integrityHash "q_hash(span)" ] .
```
- `FragmentSelector` (RFC 5147 `#char=`/`#line=`) = the literal media-fragment id — fast.
- `TextQuoteSelector` (exact + prefix/suffix) = robust / self-healing.
- `integrityHash` = the span verified against the recovered `originalText` (now possible, §1).

---

### 3.1 The standard-ontology stack

Reuse established vocabularies per layer so the data is interoperable and we invent no schemas.
Pattern throughout: **adopt the standard *vocabulary*; execute with the native bounded engine.**

| Layer / sub-graph | Standard vocab | How used here | Status |
|---|---|---|---|
| TEXT — structure | **dcterms**, **ELI**, FRBR (Work/Expression) | `dcterms:title/date/conformsTo/references`; ELI for article/clause subdivisions + `amends`/`repeals`/`transposes`; FRBR **Work ↔ Concept**, **Expression ↔ `originalText`** | dcterms ✅; ELI 🔶 |
| TEXT → Concept | **W3C Web Annotation** (`oa:`) | `realized-by` media-fragment selectors (§3) | 🔶 |
| CONCEPT | **SKOS** | identity, `broader`/`narrower`/`related`, `closeMatch`/`exactMatch` | 🔶 |
| Provenance / curation | **PROV-O**, **Nanopublications** | `prov:wasGeneratedBy` (machine) vs `prov:wasAttributedTo` (signed human); nanopub = assertion + provenance + pub-info **per LOGIC sub-graph** (not per concept) | PROV-O decided ✅ |
| Deontic | **ODRL** + native VM | ODRL *expresses* `duty`/`permission`/`prohibition` + `assignee`/`assigner`; **compiles to** `OP_OBLIGATE/PERMIT/FORBID` + `q42:unless` defeaters that `values_evaluate` runs (ODRL has no defeasibility/lifecycle) | `deontic_logic.rs` ✅ |
| Temporal | **OWL-Time** *vocab only* | `time:Interval`, Allen relations as labels — evaluated by the native `interval_reasoning.rs` / `temporal_ltl.rs`, **not an OWL reasoner** | engine ✅; vocab 🔶 |
| Spatial / jurisdiction | **GeoSPARQL** *vocab only* | jurisdiction geometry — evaluated by native **RCC-8** (`spatio_temporal.rs`) | RCC-8 ✅; vocab 🔶 |
| Agents / orgs | **W3C ORG**, `values:Agent/State`, **VCDM** | ORG for UN bodies/committees; `values:*` agent lattice; VCDM for credentialed attributes | partly ✅ |
| Data sovereignty | **DPV** | consent / legal basis where personal data is implicated | 🔶 |
| Natural world | **SOSA/SSN**, **QUDT** | observations (energy/soil/yield) + unit-validated quantities (SHACL-checkable) | 🔶 (wellfair scope) |
| Crypto / signatures | **`sec:`** (Security Vocabulary) | `sec:proof` / `sec:Ed25519Signature` for the "human disposes" signed ratification (§4) + VCDM proofs | enforce ✅ / vocab 🔶 |
| Concept anchoring | **`wd:`** (Wikidata) | external `skos:closeMatch`/`exactMatch` to entities for disambiguation | 🔶 |
| Cataloging | **`dcat:`** | corpus + each `.q42` as a `dcat:Distribution`; dataset discovery | 🔶 |
| Man-made kinds | **`owl:`** | taxonomy of *constructed* kinds **only** (legal / institutional / artifact), fenced per §5a — never the natural world | 🔶 |
| Software | **`doap:`** | describe QApps / software projects (man-made) | peripheral |

**Refinements (deliberate, not omissions):**
- **No `foaf:Person` for natural persons.** A person is not a generic web resource — `q42:Principal`
  is `rdfs:Class`, not `owl:Thing`. Model persons with `values:NaturalPerson` + **VCDM** + SHACL
  state-enumeration over a *fabric* of identifiers (§5a). FOAF/VCard only for non-sensitive
  organizational contact metadata, if at all.
- **`owl:` is fenced to the world of man** — constructed kinds only, never the natural world (§5a).
- **OWL-Time / GeoSPARQL are vocabularies, not reasoners here** — see the "vocab only" rows.
- **Nanopub granularity is the sub-graph**, so each norm/bound carries its own provenance + signature.
- **Canonicalize dual-served vocabularies at ingest.** `schema.org` is served at both `http://` and
  `https://` (and similar exist); the two forms hash differently and would *fork* a concept. Normalize
  to one canonical form (https) before `q_hash`, so terms never split.
- **External mappings carry reference, not ontological commitment.** A `skos:closeMatch`/`exactMatch`
  to `schema:`/`wd:` adopts the *identifier* (interop, disambiguation) but **never** inherits the
  target's class axioms — e.g. never `schema:Physician ⊑ Place`. See the §5a bidirectional firewall.

## 4. The Curation Prime Directive (load-bearing)

> **Automated systems may assert only `skos:closeMatch` or `skos:related`.
> Only a cryptographically-signed human action may assert `skos:exactMatch`.**

Concept identity — *when are two clauses the same concept?* — is **curation-grade**, because here
the difference between Convention 105 and the 1926 Slavery Convention is the difference between legal
protection and administrative violence. Auto-asserting sameness from vector proximity would recreate
the algorithmic flattening this system exists to defeat.

This enforces the Principal–Agent duty of care via the existing governance substrate:

- **Machine proposes** 🔶 — the probabilistic / vector worker writes a *provisional* quin:
  `[MachineAgent] proposes [skos:closeMatch concept:B] {confidence: 0.89}`. It is a proposal, never
  a fact.
- **Human disposes** 🔶 — ratification requires a cryptographic signature routed through the
  **governance tier** (`metadata` routing `0b10`, bilateral/governance) before a permanent
  `skos:exactMatch` is written.
- **Enforcement is already wired** ✅ — `agent-accountability.n3` (`UnsubstantiatedClaimFlag`) +
  `fire_guard_rules` already flag agents that assert as fact what was never substantiated. A
  machine-written `skos:exactMatch` without a ratifying signature is, by that rule, a conduct
  violation — so the directive is *self-policing* on the engine we already have.

---

## 5. The per-clause deontic feature schema (Hohfeld)

Treaty clauses are not only duties — they also grant **powers** ("may denounce this Convention") and
**immunities** ("no derogation shall be made"). The structured feature-set is Hohfeld's eight jural
relations (four correlative pairs):

| Active incident | Correlative | Treaty example | values vocab |
|---|---|---|---|
| duty | claim-right | "shall suppress forced labour" | ✅ `borneBy` / `heldBy` / `correlativeDuty` |
| privilege (liberty) | no-right | "may make reservations" | 🔶 add (permission) |
| power | liability | "may denounce this Convention" | 🔶 add |
| immunity | disability | "non-derogable right" | 🔶 add |

A clause's **norm feature record** → discrete quins:
`{ provenance: fragment-selector ; relation: Hohfeld pair ; modality: Obligate|Forbid|Permit ;
bearer: borneBy ; counterparty: heldBy ; action/object ; condition → q42:unless defeater ;
temporalScope ; curationStatus }`. This maps 1:1 onto `compile_norm_quin` + the defeater mechanism —
**no new engine**, a structured front-end producing quins `values_evaluate` already consumes. ✅

---

## 5a. Modeling discipline: the domain-keyed OWL boundary; SHACL as the enforcement firewall

**The OWL boundary is keyed by *what kind of thing the ontology may constitute* — not a blanket ban.**

- **World of man (constructed) → OWL is appropriate.** Legal persons, corporations, public
  authorities, institutions, treaties/documents, contracts, software agents, man-made artifacts *are*
  human definitions — they exist because we declared them. OWL's open-world, definitional machinery is
  meaningful and safe here.
- **Natural world, including humans (given) → RDFS + SHACL only; never OWL.** Natural persons, ecology,
  physical observations are **prior to the ontology**. A person is not an `owl:Thing` to be classified
  into existence by a reasoner. The model may *describe* and *validate*, but must never *constitute* or
  *infer* being. This is the anti-flattening commitment at the meta level: the model serves the person;
  it does not define them. (`q42:Principal` is `rdfs:Class` not `owl:Class`; `sh:not owl:Thing` guards
  stay — they *are* the boundary.)

Two constraints keep this sound under load:

1. **OWL describes *kinds*, never *adjudicates norms*.** Even inside the world of man, the
   deontic/temporal/legal *reasoning* (the harm-relevant part) runs through the deterministic VM +
   SHACL — **not** OWL inference. OWL gets the taxonomy of constructed kinds and institutional
   structure; it never decides whether an obligation is in force. Determinism stays on the load-bearing
   path in every domain.
2. **The boundary is a bidirectional firewall.** (a) OWL inferences in the man-made graph must never
   cross over and reclassify a natural-world entity. (b) **External mappings carry *reference*, not
   *ontological commitment*:** when a natural-world concept maps to an external vocabulary
   (`skos:closeMatch`/`exactMatch` to `schema:`, `wd:`, …) we take the *identifier* for interop and
   disambiguation, but **never inherit its class axioms**. A `closeMatch` to `schema:Physician` may say
   "relates to that web identifier"; it may *never* drag `Physician ⊑ Place` back across. (This also
   contains OWL's unbounded-reasoning / merge-ambiguity cost to the man-made subgraphs, so the 512 MB
   floor and the natural-world graph's determinism are never at its mercy.)

> **Rationale (why this boundary exists).** Typed semantics were *original* to the web, not a later
> add-on. The first program (ENQUIRE, 1989) carried **typed links** — relationships where the link
> asserts "is a member of", "depends on" — and these were dropped when the web scaled for mass
> adoption. At WWW94 (TimBL, *The Future of the Web*, CERN, May 1994 —
> [videos.cern.ch/record/2671957](https://videos.cern.ch/record/2671957)) he both called to *re-add*
> that semantic layer and framed protocol design as constitutional — "constitutions for the web" —
> because protocol actions affect people and designers are defining a society. So the flattening was a
> **scaling compromise, not the founding intent.** The ~2000 ontology *stack* (OWL out of DARPA's DAML
> agent-markup lineage; FOAF as the social-web convenience) re-added semantics but with a
> commercial/organizational telos, encoded *as ontology*: `schema:Physician` is a subclass of
> `LocalBusiness`/`Place`, **not** `Person` — a doctor modeled as a findable location, because the
> paying use case was commercial local-search. When such vocabularies were page metadata this was
> harmless. Now that human access to healthcare, identity, benefits, and rights is *mediated through*
> these definitions, the category error becomes **structural harm** — the informatics layer can no
> longer represent a person *as* a person.
>
> This architecture is therefore a **restoration** (the typed relations dropped for scale — now the
> concept-graph), a **completion** (the governance/"constitutions" layer the founding vision called for
> but could not yet build — the deontic + curation layers), and a **correction**: the man-made/natural
> split + firewall add the one discipline that vision *lacked* — exactly what would have stopped a
> commercial taxonomy from redefining the given world. OWL stays in the constructed world it was built
> for; the given world is only ever described and validated, never inferred into being.

The lattice already encodes the split: `LegalPerson`/`CorporatePerson`/`PublicAuthority`/institutions/
documents → OWL-eligible; `NaturalPerson`/environment/observations → RDFS+SHACL only. (`foaf:Person`
is a man-made social-web construct, so it can only ever be a WebID *identifier* pointing at a natural
person — never the model of one; §5b.)

The engine still *infers* on the natural-world side — the R1/R2/R3 N3 rules are monotonic
forward-chaining — but via bounded rules, not OWL-DL. SHACL's **Closed-World Assumption** then declares
what *must* be true: fast, bounded, mathematically predictable.

**Two enforcement mechanisms — they compose, they are not interchangeable:**

| | Question it answers | Mechanism | What it cannot do |
|---|---|---|---|
| **Deontic + Temporal VM** | *Is this norm binding **now**?* (Active / Defeated / Expired) | `compile_norm_quin` + `q42:unless` defeaters + Allen/LTL bounds → `values_evaluate`; masked on the concept's context hash | check real-world *state* |
| **SHACL firewall** | *Does reality **comply** with a binding norm?* | `sh:NodeShape` validation, CWA, bounded (`shacl_compiler.rs`, `validate_shacl` MCP) ✅ | defeasibility, temporal lifecycle, conflict resolution |

> **The VM adjudicates *validity*; SHACL enforces *compliance*. A shape is applicable only when its
> norm is Active.** Making SHACL the *sole* layer would silently discard defeasibility and the
> in-force lifecycle — the very things `values_evaluate` exists to compute. So: **SHACL is the primary
> *enforcement* layer, downstream of the VM that determines whether the boundary is currently binding.**

A clause therefore becomes a **compliance template** whose applicability the VM gates:
```turtle
values:ForcedLabourProhibitionShape
    a sh:NodeShape ;
    sh:targetClass values:AgentState ;
    sh:property [ sh:path values:deonticModality ; sh:hasValue values:Obligate ] ;
    sh:property [ sh:path values:operationalState ; sh:not [ sh:hasValue values:ExploitativeCondition ] ] ;
    dcterms:conformsTo  convention:ILO_Convention_105 ;   # dcterms maps the global reference
    dcterms:provenance  curation:CML_Engine_Hash .         # + PROV-O signature chain (§4)
```

**Persons & the natural world under this discipline.** Do not seek a monolithic "being" ontology.
Model a person as an **enumerated agent-state over a fabric of cryptographic identifiers** (VCDM
credentials + related datasets), validated by SHACL shapes — the *identifier ≠ identity* principle made
mechanical. Natural-world variables (`SOSA/SSN` observations, `QUDT` units) become SHACL-validatable
data points hanging off the relevant concept, never OWL-inferred.

## 5b. Solid — a portability / exit-right export target (NOT a design substrate)

Solid is **not** a layer we build on, and **not** the envelope. It is a **backwards-compatibility export
target + interop boundary** — a guarantee of *non-capture*. The design freedom that makes Qualia better
(the quin engine, the concept-graph, the man-made/natural split, credential-gated vaults) comes precisely
from **not** inheriting Solid's choices (pod mechanics, foaf-centric WebID-as-identity, OWL-everywhere).
Two purposes, both mission-critical:

1. **Right to leave.** If a person stops using Qualia, they can **export** their data to Solid and walk
   away. No lock-in is itself a dignity guarantee.
2. **Reciprocal institutional interop.** Institutions handling personal-data-ownership semantics can
   expose a Solid interop layer, so the *intents and purposes* of this work survive beyond any single
   platform — the portability propagates outward.

So the Solid mapping is a deliberately **lossy export profile** — lossy *out* (Qualia's semantics are
richer), never lossy *in*:

| On export → Solid | Qualia source | Status |
|---|---|---|
| WebID / `vcard:` profile | `values:NaturalPerson` (+ VCDM, SHACL state) — *identity stays here; the WebID is only a pointer* | 🔶 |
| `ldp:` Containers / Resources | `.q42` volume / concept-graph (served via the `bidx` 2-step range fetch) | primitives ✅ / endpoints 🔶 |
| `acl:` authorizations | `SubgraphKey` (AES-GCM/HKDF/X25519) + `.qchk` capability + governance tier | enforce ✅ / map 🔶 |
| `pim:Storage` pod + `solid:typeIndex` | credential-gated personal vault + concept discovery | 🔶 |
| `sioc:` threads · `ui:` views | `chat_*` layer · Contextual Workspace | 🔶 |

`foaf:Agent`-as-WebID is fine as an *identifier* on export; `foaf:Person`-as-a-model-of-being is refused
(it is a man-made social-web construct — §5a). (Solid's default config aliases `rdf:`→the RDFS namespace
and `XML:`→XSD; we use `rdfs:`/`xsd:` properly.)

## 6. End-to-end pilot — `concept:DutyToSuppressForcedLabour`

**TEXT** (✅ recoverable today):
```
<…convention-105#article-1> values:originalText
  "Each Member … undertakes to suppress and not to make use of any form of forced or compulsory labour …"
```
**CONCEPT** (context hash `Cdsf = q_hash("concept:DutyToSuppressForcedLabour")`):
```turtle
concept:DutyToSuppressForcedLabour a skos:Concept ; skos:prefLabel "duty to suppress forced labour"@en .
# realized-by  <…convention-105#article-1>  (Web Annotation, §3)
```
**LOGIC** — quins written into context `Cdsf`:
```turtle
# deontic sub-graph  → compile_norm_quin(State, OP_OBLIGATE, suppress, forcedLabour, context=Cdsf)
{ values:State  values:obligates  act:suppressForcedLabour }            # bearer = ratifying Party
# temporal sub-graph (curation overlay; instrument file carries adoption date 1957-06-25)
{ Cdsf  values:inForceFrom  "1959-01-17" }
```
**Query pathway** (the proof we will run): `values_evaluate` over context `Cdsf` →
**Active obligation, borne by `values:State`, in force, no active defeater** — citing the Article 1
text span as provenance. The R1 universalisation rule (`borneBy values:Agent ⇒ borneBy values:State`)
and `values_check`'s anti-capture guard already operate on these same quins. ✅

---

## 7. Build order (this note → pilot → tuning)

1. ✅ **Front-of-file lexicon** — literal recovery (done 2026-06-22; the TEXT-layer prerequisite).
2. ✅ **This architecture note** — the 3-layer structure (this file).
3. ✅ **CML axioms + concept layer** — `cml.n3` authored (concept = `skos:Concept` + context-hash;
   `cml:realizedBy` Web-Annotation realization; `cml:asserts` logic binding; `cml:curationStatus`
   Proposed/Attested + the Prime-Directive SHACL shape; `cml:recognitionBasis` RPL; the ontology
   stack — skos/oa/prov/sec/dcterms/time/odrl/values). Pilot concept built
   (`concepts/duty-to-suppress-forced-labour.n3`): ingests via `turtle_doc` (26 triples), queries
   cleanly, and its `cml:realizedBy` resolves to the **same** `…#article-1` IRI the corpus produced
   (one hash-space — the concept JOINS the instrument). Ontology gate green.
4. ✅ **Deontic sub-graph (pilot)** — the concept `cml:asserts` a `values:Obligation` (State ·
   `requires` · SuppressForcedLabour). (Temporal/Allen + SHACL compliance-shapes: next.)
5. ✅ **Pilot run — math against the concept.** `cml_concept_deontic_pilot` (`webizen.rs`): the
   concept hash IS the deontic sub-graph context; `compile_norm_quin` + `evaluate_deontic_contract`
   over it → **Active** (in force); an `unless lawfully-authorised` defeater in the same sub-graph →
   **Defeated**. Full lib 1054/1054. *(Scale-up: concept layer across all 102 instruments + temporal
   bounds + the SHACL firewall remain.)*
6. ⏭ **Neuro-Symbolic Sieve spec** (separate) — `closeMatch` proposal heuristics: vector-distance
   thresholds, prompt structure, what counts as evidence. Policy/tuning, not structure.
