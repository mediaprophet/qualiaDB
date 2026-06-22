# core-ontologies — Design Rationale (the deep "why")

The dense reasoning behind the values / CML layer, authored by Timothy. Extracted **verbatim**
from `PLAN.md` §10.2a–§10.2d on 2026-06-22 so the live plan reads lean — nothing changed, these
are the original essays, relocated. `PLAN.md` links back here.

---

### 10.2a Design requirement — make the ABSENCE legible (the unborne correlative duty)
Because that telos (amplified by US-English as the global ICT substrate) funded only what served
profit, the human-rights / natural-world semantics are **absent from the online corpus** — the
*negative imprint* of the funding structure (not-profitable → not-represented → not-reasoned-over).
The protection asymmetry is the Hohfeldian form of the same fact: corporate agents carry **immunities**
(limited liability, insurance), while the duty-bearers of the commons carry the **correlative duty
unfunded and unprotected**. Requirement: the values layer must represent the **unborne correlative
duty** as a first-class, queryable fact — R2 already derives that a held right's correlative duty is
borne by *every* agent; the structural deficiency is the case where **no resourced agent bears it**.
The engine should surface "right held · correlative duty · no funded bearer" so the omission becomes a
*named, cited, reasoned-over deficiency*, not an invisible void. Scope honesty: this addresses the
**informatics** dimension (presence/legibility/machine-actionability), not redistribution — but
legibility is the precondition for anything downstream. (`PendingImplementation`; relates to the
wellfair / foundational-supports track.)

### 10.2b The liability-trap, the inspectability inversion, and subject ≠ beneficiary
**Why the work is structurally unfunded (sharper than §10.2a).** An employee-agent has no *power* to
commit corporate resources to human-rights work lacking a profit nexus — it would expose the corporate
person to liability — so the agent role itself forbids it. Doing the work requires *exiting* the agency
relationship (clear of non-compete / IP-assignment / NDA capture), i.e. forgoing income. The structure
therefore *selects* the few who bear total cost; that selection effect is the deeper cause of the
corpus-absence. (Hohfeldian: agent under a *disability*, corporate person holding an *immunity*; the
exit needed to act removes the material foundation.) **IP capture is the third jaw:** standard
ICT/liberal-arts contracts assign *IP ownership* to the employer, so even the products of thought are
enclosed — work distilling shared human values into primitives, done under employment, becomes
assignable IP (owned, then unused, the purpose being unprofitable). **Corollary requirement: the
values-credential ontologies must be a commons** — openly licensed, un-assignable, un-capturable — or
they reproduce the very enclosure they exist to resist. **Commons ≠ uncompensated:** the wrong is
*theft* (works lifted without the author's knowledge or benefit) or public-good work left *unfunded* —
not openness. Open-licensed output and fair compensation for the author are *compatible*; the fix is to
**fund the commons / support the person**, never to enclose the work (enclosure reproduces the theft).
The mechanical defense against silent misappropriation is the **provenance/attribution layer** —
tamper-evident authorship, `prov:wasAttributedTo`, third-party-guaranteed origin, signed records
(components exist: signed provenance, PROV-O, the Merkle-DAG; the operational anti-theft system is
substrate).

**Governance principle — inspectability of power, protection of persons, due-process-gated.** The
human-centric inversion of surveillance: make the *constructed / powerful* entity's conduct evidentially
inspectable (provenanced, tamper-evident, attributable) so it can be **adjudicated by a court** with
lawful consequence — while the *natural person* is shielded, never the surveilled object. The engine
makes conduct legible and admissible; it is **not a court** (`requiresHumanReview`; accountability =
routing/review, §10.3). Unifies the protection-asymmetry fix, accountability-as-routing, and the
man-made/natural boundary (CML §5a). Components already being assembled — signed provenance, the
court-admissible record, agent-accountability routing, deontic evaluation — are *substrate*, not yet the
operational system.

**Subject ≠ beneficiary (jurisdictional disenfranchisement).** A US-origin, US-English substrate encodes
one nation's constructs as global defaults; most of the world's population is *subject to* those
definitions (bound by their liabilities) without being *beneficiaries* with standing in them — "legal
aliens" to the system that governs them. Requirement: (1) ground rights in the **international**
instruments + the universalisation transform (R1 derives the State's duty *a fortiori* from a duty borne
by *every* agent — de-particularising universal personhood, not nation-granted benefit); (2) make
"subject-without-standing" legible like the unborne-duty gap, and compute a person's rights against
**international baselines that travel with them** (jurisdiction-follows-the-person; `sense.n3`
`JurisdictionRegion`/`validIn`).

Together with §10.2a, the values layer must make **three structural absences** computable: the duty no
one is paid to carry, the institutional conduct no one can inspect, and the person bound by a system that
grants them no standing.

### 10.2c Values are universal at the core, plural in expression (selfhood / sense requirement)
Shared humanitarian values are universal in their core (dignity, non-harm, reciprocity — the Golden Rule
recurs across nearly all faith traditions) but **distilled and transmitted through culturally, faith-,
and linguistically-specific primitives.** The model must hold both layers and do two opposite things at
once: **never flatten the particular into the universal** (collapsing a tradition's value into a generic
primitive erases it — the same category-violence as `Physician ⊑ Place`, applied to culture/faith) and
**never fragment the universal into incommensurable particulars** (relativism that loses the shared
ground). Mechanism: the concept-graph — a universal value-concept ← `realized-by` ←
culturally/faith/linguistically-specific expressions; SKOS `broader`/`narrower`/`related` + `closeMatch`
across traditions + multilingual labels; the Curation Prime Directive holds (machine *proposes*
`closeMatch`; only human — here **culturally-authoritative** — curation asserts `exactMatch`).

These are **personal attributes of the natural person** — worldview-bearing frames through which a person
interprets value and meaning — exactly where `foaf:Person` (a contact card) fails, and why the spine is
purpose-built: **`agency.n3` + `selfhood.n3` + `humanitarian-ict.n3`** (the selfhood/personhood/
humanitarian-ICT ontology). Man-made/natural nuance (CML §5a): the *traditions themselves* are
constructed (institutions, canons — world of man); the person's *belonging to and expression of* them is
an attribute of the **natural person** — given, held, protected, never owned or flattened (RDFS+SHACL;
protected in ICCPR Art 27, freedom of religion, the cultural-diversity declaration). Sense-making
(`sense.n3`: `interpretationMode`, `validIn`) is then resolving a universal value-concept into the
person's particular frame, and back. Continuity: the *needs* layer already refused the single Western
ladder (Capabilities / Ubuntu / Recognition / Max-Neef over Maslow); this is the same pluralism at the
*values* layer.

Status: the universal rights core is partly done (lattice, instruments, R1/R2); the
culturally/faith/linguistically-specific value-primitives + their linkage are **not** — the most
curation-heavy, least auto-assertable layer (getting it wrong is culturally injurious). Build slowly,
with culturally-authoritative people; never scale-fast. (`PendingImplementation`.)

### 10.2e Permissive Commons — the licensing framework (cost-recovering, self-extinguishing)
The license that resolves §10.2b's *commons ≠ uncompensated*. **Not gratis-free** ("free work does not
exist" — gratis externalises the creator's cost onto the unsupported person, §10.2d) and **not enclosure**
(perpetual rent). Instead a **self-extinguishing obligation that terminates in the true commons** once
creators are made whole. Release trigger = **compensation completion** (not a clock, not nothing).
> NB: supersedes the earlier CC0/CC-BY suggestion — those are exactly the gratis model this rejects.

Mechanism:
1. **Record production cost** — resources actually consumed (labour-time, materials); attestable via the
   provenance layer (`prov:wasAttributedTo` the workers) + QUDT-quantified; **signed, not self-asserted**
   (cost integrity guards against inflation — anti-theft discipline applied to the cost ledger).
2. **ObligationCost = ProductionCost × ROIMultiplier.** The multiplier is the *fair-return* knob: a
   governance-set policy value under a **hard SHACL cap** (`sh:maxInclusive` ≈ 10×, never 100× — the
   anti-rent-extraction firewall; structural cap vs tunable value, as in the closeMatch split). =
   "what they should have been paid" (cost + bounded benefit).
3. **Differentiated by agent category** — ODRL (`odrl:duty`/`odrl:assignee`) + the values agent lattice set
   obligation rates/terms per `NaturalPerson` / `CorporatePerson` / `PublicAuthority` / non-profit, etc.
4. **Pay down** via ILP micropayments and/or sponsorship lump-sums.
5. **Discharge → ObligationFree** — when cumulative compensation ≥ ObligationCost the economic obligation is
   *discharged* and the work passes into the true commons. A deontic norm with a discharge condition:
   `Active`(Outstanding) → `Discharged`(ObligationFree), computed like the `values_evaluate` lifecycle.

**"Obligation-free" is economic ONLY.** The payment obligation extinguishes; **attribution never does** —
the provenance/authorship trail (§10.2b anti-theft) persists permanently. Discharge frees the *use*, not the
*credit*.

Open design questions (Timothy's steer — policy, not structure):
- **Discharge model:** per-agent debt vs **collective pool** (total compensation from all sources meets the
  cost → obligation-free for everyone; category-differentiated *rates* feed the pool). His phrasing implies
  the collective pool — to confirm.
- **Obligation composition:** does a derivative inherit upstream works' un-discharged obligations (an
  obligation dependency-graph — the dialectical/dependency modality)?
- **Unit of account** (fiat / token / resource-unit) for cost + ROI denomination.
- **Multiplier band + who sets it**, under the hard cap.

Adjacent prior art (interop/grounding, NOT reattribution — distinct synthesis): time-delayed-open (BSL /
"fair source") releases on a *clock*; Permissive Commons releases on *compensation completion*.
Steward-ownership / exit-to-community and data-dignity compensation are kin in spirit, none identical. Mostly
expressible with existing components (deontic discharge + ODRL + agent lattice + SHACL cap + provenance +
QUDT + ILP). `PendingImplementation`; would become `permissive-commons.n3` + a short spec, and license
`core-ontologies/` itself.

### 10.2f Civilizational scope — whose knowledge the substrate must carry (and on whose terms)
Beyond the SDGs, the values layer must bring into the ICT/AI substrate the knowledge held by the
devalued and the at-risk-of-erasure — or those peoples and worldviews are written out of an AI-mediated
future (what is absent from the substrate is absent from the future people must live in). The
expectation that those who hold this knowledge do the work **"for free" to count as good** is itself the
exploitation (§10.2d; "free work does not exist"). Three fronts and their mechanisms:

- **Linguistic justice.** All mother tongues + languages of prayer as **first-class**, not English-
  default: multilingual SKOS labels on every value-concept + the sense layer (`interpretationMode`,
  `validIn`). Anti-flattening at the linguistic level — an English-only substrate erases other
  conceptual worlds from the machine-mediated future (same fight as `Physician ⊑ Place`).
  ✅ **Plumbing prerequisite done (2026-06-22):** the `.q42` lexicon + the `turtle_doc` ingest parser
  are now **Unicode-correct** — byte-exact UTF-8 round-trip for Arabic / CJK / Ge'ez / em-dash /
  curly-quote literals, regression-tested (`non_ascii_literals_roundtrip_intact`). (A latent
  `byte as char` defect would have corrupted non-ASCII text and could panic mid-codepoint; current
  corpus literals were ASCII-only so no live data was harmed, but the multilingual layer cannot be
  built on a parser that mangles its scripts.)
- **Traditional knowledge (e.g. medicine), WITHOUT biopiracy.** Knowledge-holders (often remote, often
  older women whose work the market priced at zero) *start* the corpus; domain experts (biochemistry,
  …) *validate + improve* it collaboratively (Contextual-Workspace reader=writer; co-location not
  required). Anti-appropriation guarantees: **provenance/attribution** (originating community credited
  — §10.2b), **Permissive Commons** (community *compensated* on use, incl. by pharma — §10.2e), and
  **governance by CARE + FAIR** — FAIR alone *enables* biopiracy; **CARE** (GIDA: Collective benefit,
  **Authority to control**, Responsibility, Ethics) prevents it, *including the right to WITHHOLD*
  (sacred/secret knowledge is not auto-open; the community decides what enters the corpus and on what
  terms). Curation = culturally-authoritative *consent*, incl. consent to **not** encode (extends the
  §4 Prime Directive: the curator may also say "not for the corpus").
- **Transposition of state-era obligations to the modern agent lattice.** Instruments were written by/
  for **States** in pre-internet language; transpose so the agents now wielding rights-affecting power
  (corporations, platforms, AI/ICT) **bear the correlative duties** *while the BENEFIT stays anchored to
  the natural person.* **Typed, not blanket:** (i) *universal* duties (don't torture, don't enable
  abuse) → every agent (R1 a-fortiori); (ii) *public-law* duties (judicial remedy, legislate) → stay
  with `PublicAuthority` (`specialisedFor`; a platform cannot "provide a court"); (iii) *new ICT-age*
  duties → platforms/AI acquire correlative duties the drafters could not foresee. Duties extend
  **outward** (more bearers); rights/benefits stay with `NaturalPerson` — **G1 enforces the asymmetry**
  (a transposed duty must never become a captured right) = "beneficial relations for human beings
  specifically, alongside correlated responsibilities." Pre-internet → ICT-age language update rides the
  sense layer (Originalist/Living `interpretationMode`, `amendedText`). Wired primitives: R1/R2,
  `specialisedFor`, disjoint lattice, G1, concept-graph; the systematic transposition + language-update
  overlay are curation-heavy and **not yet done**. (`PendingImplementation`.)

### 10.2g CBOR-LD as the broadening interchange / wire format (one hash-space; `.q42` native)
**Yes — adopt CBOR-LD as the broadening INTERCHANGE + transmission format, NOT as the native compute
truth.** Rationale: CBOR-LD is the W3C VC/JSON-LD ecosystem's compact binary wire format — and these are
*values CREDENTIALS*, so they must travel as that ecosystem's native format (VCDM / mobile credentials);
compact-binary suits mobile (wellfair) + the Nym mixnet; its `@context` term-dictionary mirrors the
`.q42` front-of-file lexicon. **Boundary-not-substrate**, as with Solid/ODRL: CBOR-LD at the edges
(ingest + wire/export), `.q42` (NQuin + hash-lexicon) as the native compute/storage truth; the q42
lexicon is the engine-side analog of the CBOR-LD `@context` (clean transcoding bridge).

State (verified 2026-06-22): CBOR-LD **ingest** (`cbor_parser.rs::parse_cbor_ld_stream`), **serialize**
(`CborLdStarSerializer`, `vault_manifest::to_cbor_ld`), and **wire use** (daemon, p2p, vault manifests)
all exist — so "CBOR-LD natively" is substantially true *at the boundary*. **TWO defects mean it does NOT
yet broaden the *same* graph (it builds a parallel one):**
1. ✅ **Hash inconsistency — FIXED (2026-06-22).** `cbor_parser.rs` `hash_str`/`hash_bytes` now delegate
   to `generate_60bit_token` (were `DefaultHasher`/SipHash), and `Type::String` values are read via
   `.str()` (were mis-read via `.bytes()`). The same IRI/literal now hashes **identically** across
   CBOR-LD and Turtle/N3/SPARQL → one joinable graph. Test `cbor_terms_share_one_hash_space_with_turtle`
   (hand-built CBOR map → quin hashes == `generate_60bit_token` of the terms, ≠ the old SipHash value).
   Full lib 1053/1053; daemon/p2p/vault consumers didn't rely on the old hash, so no wire breakage.
2. **Naive-CBOR, not full CBOR-LD.** Hashes whatever string/int it finds; does **not** expand compact
   term-codes against a JSON-LD `@context` to canonical IRIs. Real CBOR-LD interop = expand-to-IRI,
   *then* hash.

**Unifying principle (third time we've hit it — after prefix-expansion and Unicode):** every input
format must funnel to ONE hash-space — *canonicalise the term to its full IRI/literal, then
`generate_60bit_token`.* Then CBOR-LD genuinely broadens: one graph, many formats —
N3 / N3-star / Turtle / Turtle-star / RDF-XML / JSON-LD / **CBOR-LD** / KML → quins → `.q42` (native) →
CBOR-LD (wire/export). ✅ The keystone (unified term hashing) is **done**; remaining follow-on: full
CBOR-LD **`@context` expansion** (compact term-codes → canonical IRIs *then* hash) and numeric/datatype
**literal canonicalisation** across formats. (Cross-ref §5 `.q42` layer, §10.2c multilingual.)

**Lexicon methodology is medium-agnostic (any modality, any medium).** The hash path is *byte-based*
(`generate_60bit_token` over canonical bytes), so it is the SAME methodology for text, a byte-string,
an image, audio, or a spectral signature — language in any medium maps in by hashing its canonical
bytes. The `Q42LEX` `LexiconEntry` is already a **tagged** payload (`String` 0x01 / `EmbeddedTriple`
0x02 / `Webizen` 0x03), so it extends to non-text media by adding a modality-tagged variant
(e.g. `Media` = modality byte + blob/content-hash ref) — *without changing the hash path*. This is the
lexicon side of the NQuin modality flags and "multimodal-as-physics". (Extension `PendingImplementation`;
the methodology already supports it — the Unicode fix proved text; media is the same shape.)

**Credential lineage — Open Badges v3 / baked credentials.** The early credential work became
**IMS Open Badges v3.0** ([imsglobal.org/spec/ob/v3p0](https://www.imsglobal.org/spec/ob/v3p0)) — a W3C
Verifiable Credential that can be **baked into an image** (PNG `iTXt` / SVG metadata), so an *image*
carries a credential = semantic statements + claims + logic. That is precisely "language in a medium":
the medium is an image, the language is the embedded VC. Ingest path: extract the baked VC →
JSON-LD/**CBOR-LD** → (now-unified) hash → quins → `.q42`; the lexicon may also hold the carrier image
as a media entry. Align the values-credentials with **Open Badges v3 + VCDM** as the credential
surface. (`PendingImplementation`: baked-image VC extraction + the media `LexiconEntry`.)

### 10.2h Capability credentials — RPL + gap analysis (Open Badges lineage; SDG / Peace-Infrastructure)
The Open Badges lineage (§10.2g) began as **micro-credentials in education**, with the objective of
**Recognition of Prior Learning (RPL)**: recognising real skills/knowledge gained outside formal
schooling. The credential machinery (VC/Open Badges + concept-graph + curation) generalises from
*values*-credentials to **capability-credentials** — a *sibling domain of the same engine*, carrying a
load-bearing anti-deficit stance.

Model (reuses existing machinery — no new engine):
- **Capability** = a concept (SKOS `broader`/`narrower`/`related`, multilingual): "potable-water system
  construction", "solar-array maintenance", "malaria prophylaxis (traditional)".
- **LearningClaim** = a verifiable claim (Open Badges v3 / VCDM) that a person holds a capability, with
  the ontological distinctions: **`recognitionBasis`** = `Formal` | `PriorLearning` | `Experiential` |
  `PeerAttested` (RPL is **first-class** — "no degree" ≠ "knows nothing"); **`evidence`** (PROV-O: what
  backs it); **`assessmentStatus`** = `Proposed` | `Attested` (curation-gated — machine *proposes* a
  `closeMatch` between lived experience and a capability; a **contextually-authoritative assessor**
  confirms — §4 Prime Directive applied to skills).
- **Gap analysis** = a query; the *make-the-absence-legible* pattern (§10.2a) applied to skills:
  `Gap = Required ∖ Available` (SKOS subsumption/`closeMatch`), where *Required* is a project's capability
  set — often a **dependency graph** ("to build X need A+B+C"; TimBL's "if missing, what's blocked?" →
  dependency/dialectical modality). Output: present capabilities · genuine gaps · **clarifications**
  (ambiguous `closeMatch` → assessment, never auto-denied).

**Anti-deficit inversion (load-bearing).** The default — "no degrees → bring experts, locals know
nothing" — is the flattening/erasure pattern again. RPL **inverts** it: recognise what *is* present
first, then identify *genuine* gaps, then target support that **completes existing capability** rather
than assuming its absence. Skills-layer form of "inspectability of power / recognition of the custodian";
continuous with the **Capabilities approach** (Sen/Nussbaum) already in the needs foundation.
Applications: portable verifiable CVs; and **SDG / Peace-Infrastructure** deployment — when resources are
sent to construct Peace Infrastructure, a *computable* capability gap-analysis targets support precisely
instead of parachuting in on a deficit assumption (development-as-recognition, not development-as-erasure).

Reuses: concept-graph + SKOS (capabilities), VC / Open-Badges-v3 / VCDM (claims), PROV-O (evidence),
gap-as-legible-absence (§10.2a), curation discipline (§4), dependency modality, Permissive Commons
(recognise + compensate local skill). (`PendingImplementation`; sibling domain of the values-credential
machinery — values-credentials encode *rights*, capability-credentials encode *learning*, one engine.)

### 10.2i Document codec — bake/extract credentials & semantics in/from documents (PDF; invoices, payslips, pathology)
Extends the baked-credential model (§10.2g, Open Badges → image) to **PDF and everyday documents**. Two
directions, both reusing the one-hash-space CBOR-LD pipeline (§10.2g) and the medium-agnostic lexicon.

**ENCODE (bake) — semantics → document.** Embed a *signed* VC / CBOR-LD / `.q42` payload in a PDF via
**PDF/A-3 embedded files** (ISO 19005-3 — the mechanism Factur-X uses) + **XMP** (ISO 16684, RDF/XML
metadata). Signed (ed25519 / ML-DSA) → tamper-evident → court-admissible (wellfair). **Tool: a virtual
"printer driver"** — print-to-PDF (or post-process an existing PDF) that bakes + signs the semantic
layer; **separately distributable** as a standalone utility AND exposable as a qualiaDB capability/MCP
tool (one codec, two distributions).

**DECODE (extract) — document → semantics**, tiered by reliability:
1. **Baked (ours)** → *lossless*: read the embedded VC/CBOR-LD/`.q42` → quins (round-trips like the
   `.q42` lexicon).
2. **Structured-standard embedded** → parse the standard: **invoices** (Factur-X / ZUGFeRD / UBL /
   EN 16931 embedded XML), **pathology/medical** (HL7 FHIR + LOINC/SNOMED), **payslips** (payroll
   schemas) → quins via the domain libraries (`financial_modeling`, `medical`).
3. **Unstructured** (arbitrary scan/PDF) → OCR + layout + LLM-assisted mapping → **curation-gated**:
   machine *proposes* claims (`closeMatch`, confidence), human *confirms* — **never auto-assert** a
   pathology value or a financial figure (§4 Prime Directive; a wrong medical value is injurious).
   Hardest, lowest-reliability tier.

**Why it matters (wellfair intake).** Invoices, payslips, pathology results are the documents a life
generates — exactly the **person-held verifiable claims** the wellfair life-record is built from
([[project_wellfair_purpose]]): income verification (benefits; RPL-of-work-experience §10.2h), health
trends over time (pathology series), court-admissible evidence. The codec turns the pile of documents a
person receives into a queryable, verifiable, gap-analysable graph they hold. Continuous with §10.2h: a
payslip *is* work-experience evidence (RPL); a pathology series *is* a queryable health trend.

Reuses: medium-agnostic lexicon + one-hash-space CBOR-LD pipeline (§10.2g), VC/Open-Badges/VCDM, PROV-O +
signing (court-admissibility), domain libraries (financial/medical), curation discipline (§4). Standards
to align: PDF/A-3, XMP, Factur-X/EN 16931/UBL, HL7 FHIR/LOINC/SNOMED. (`PendingImplementation`: PDF
embed/extract codec + virtual-printer tool + structured-standard parsers; unstructured tier curation-gated,
lowest-priority/reliability.)

### 10.2j Chained credentials — plurality of claims, derivation/authorization chains, status propagation
Confirmed central (the engine is a graph; chains are paths, credentials/claims/instruments are typed
nodes/edges):
- **Plurality is native:** a credential is a subject node, its claims are its quins (many
  predicate-object edges); a Verifiable Presentation bundles many credentials (VCDM).
- **Two kinds of chain** (typed edges in the concept-graph):
  - **Derivation / evidence** — C is `prov:wasDerivedFrom` / cites `evidence` A, B (RPL: "master" from
    "apprentice + safety"; Open Badges v3 `EndorsementCredential` = a credential about another credential).
  - **Authorization / delegation** — authority to *assert* C is delegated down a chain (ZCAP-LD capability
    chains, DID issuer-authorization); roots in a legally/culturally-authoritative root; §4 Prime Directive
    forbids a machine self-authorising a link.
- **Instrument-anchored:** chains frequently root in an **instrument** (UN convention as `ValuesCredential`)
  — a fulfilment/relevance claim chains *up* to the instrument's obligation/right (values/deontic layer).
- **Cryptographic substrate already exists:** the **Merkle-DAG / WAL** (`wal.rs` `prev_dag_hash`,
  `checkpoint_to_dag`, DagNode store) hash-links records → tamper-evident, court-admissible credential
  chains (wellfair).
- **Key subtlety — status PROPAGATES along the chain.** A chained credential's validity is *computed, not
  static*: revoke/expire/**defeat** an upstream credential → downstream dependents must be re-evaluated.
  This is the deontic lifecycle (`Active`/`Defeated`/`Expired`, `values_evaluate`) + the dependency/
  dialectical modality (TimBL "if A removed, what stops working?") + `credentialStatus`/revocation.
  Revocation/defeat propagation along the dependency DAG is the load-bearing, non-trivial part.

Standards: VCDM (`evidence`, `credentialStatus`, Verifiable Presentations, `termsOfUse`), Open Badges v3
(`EndorsementCredential`), PROV-O (`wasDerivedFrom`/`wasAttributedTo`), ZCAP-LD (delegation), SKOS
(`broader`/`related`). Reuses concept-graph + deontic lifecycle + dependency modality + Merkle-DAG +
curation (§4). (`PendingImplementation`; a property the credential layer must have — confirmed central.)

### 10.2d Foundational supports are the ROOT dependency (the deepest absence)
Fair compensation, anti-theft, rights, agency, the works themselves — all presuppose a natural person
**materially supported enough to exist and act.** Foundational supports (food, shelter, security, the
conditions of continued existence) are not the first item in a list; they are the **root dependency** of
the agency graph. In the dependency/counterfactual terms of the modalities ("if removed, what stops
working?"), the answer for foundational support is *everything downstream*. So the engine should
represent it so: foundational support as the root node whose **unmet state propagates "capacity
undermined" to every dependent right and work.** This re-orders the three structural absences
(§10.2a/§10.2b): the unmet duty to support the person is **not one missing duty among them — it is the
one whose absence voids the others** (an unsupported person cannot sustain the work, pursue the claim, or
exercise the right). It sharpens the existing chain (*foundational supports → selfhood → personhood →
agency → rights*) into a dependency with a computable root; grounded in Capabilities / Max-Neef (material
preconditions of agency), not Maslow's ladder. Scope honesty: the engine makes this *ordering legible* —
it does not provide the supports; but the prevailing order keeps the foundation *invisible* (treating the
supported person as given, reasoning only about the works/rights on top), which is how foundational work
ends up unsupported while the work is used. (`PendingImplementation`.)

