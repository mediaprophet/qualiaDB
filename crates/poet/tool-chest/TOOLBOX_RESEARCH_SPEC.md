# Tool-Chest Spec — Research Toolbox

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Ontology:** [`qualia-ui/ontologies/research.n3`](../ontologies/research.n3) (N3 authoring → CBOR-LD runtime)

The `research` toolbox serves structured enquiry — defining scope and constraints, building corpora, studying social/socio-economic/spatio-temporal dynamics, inferring dark links, drawing conclusions through inference chains, and synthesising findings. It is **loosely coupled** to containers and **loosely related** to the investigation toolboxes — research may inform or be informed by investigations, but each can exist independently.

---

## 1. Toolbox: `research` (Structured Enquiry)

The `research` toolbox is for broad, exploratory, and generative enquiry. Unlike `investigate` (which is evidence-reconstructive) or `forecast` (which is predictive), research is about **understanding** — building knowledge from diverse sources, identifying patterns across dynamics, inferring hidden connections, and synthesising higher-order understanding.

### 1.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `research-board` | content | missing | Primary research workspace — questions, findings, corpus, dynamics, inferences on a spatial canvas. |
| `corpus-browser` | content | missing | Browse and manage the research corpus with provenance and confidence. |
| `literature-review` | content | missing | Structured literature review — papers, citations, themes, gaps. |
| `data-canvas` | content | missing | Multi-dataset exploration — join, filter, visualise, correlate. |
| `dynamic-map` | content | missing | Spatio-temporal dynamics — migration, diffusion, environmental change, urban dynamics. |
| `inference-graph` | content | missing | Inference chains — premises, conclusions, confidence. |
| `dark-link-explorer` | content | missing | Inferred/hidden/latent/suppressed connections with provenance gaps. |
| `synthesis-view` | content | missing | Synthesise findings into narrative, theory, model, ontology, or framework. |
| `question-tree` | panel | missing | Hierarchical research questions with dependencies and status. |
| `provenance-trace` | panel | missing | Provenance chains for findings, inferences, and corpus items. |
| `constraint-panel` | panel | missing | Research constraints — ethical, legal, resource, methodological, access, temporal. |
| `inspector` | panel | missing | Inspects selected question, finding, corpus item, or dark link. |

### 1.2 Tool-chains

#### `enquiry` — scope, purpose, and research questions

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `new-research` | Mutate | `name: string`, `purpose: Purpose` | Creates a new research project with a purpose (exploratory, explanatory, evaluative, generative, confirmatory, integrative, critical, applied) |
| `set-purpose` | Mutate | `research_iri: iri`, `purpose: Purpose` | Sets or adds a purpose statement with optional ontology constraint |
| `define-scope` | Mutate | `research_iri: iri`, `scope: Scope` | Defines scope — temporal, spatial, social, topical, methodological, with ontology constraint |
| `add-constraint` | Mutate | `research_iri: iri`, `constraint: Constraint` | Adds a constraint (ethical, legal, resource, methodological, access, temporal) |
| `add-question` | Mutate | `research_iri: iri`, `question: ResearchQuestion`, `parent: iri?` | Adds a research question (optionally as a sub-question) |
| `link-questions` | Mutate | `question_a: iri`, `question_b: iri`, `relation: string` | Links questions (depends-on, informs) |
| `set-question-status` | Mutate | `question_iri: iri`, `status: QuestionStatus` | Sets question status (open, in-progress, answered, reframed, deferred, dropped) |
| `link-investigation` | Mutate | `research_iri: iri`, `investigation_iri: iri` | Links a related investigation |
| `set-research-status` | Mutate | `research_iri: iri`, `status: ResearchStatus` | Sets research status (proposing, active, paused, concluded, ongoing, superseded) |
| `query-research` | Query | `research_iri: iri`, `filter: string` | Queries research project contents |

#### `corpus` — corpus building and management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-corpus-item` | Mutate | `research_iri: iri`, `item: CorpusItem` | Adds a corpus item (literature, dataset, document, media, web, sensor, oral, archival, derived) |
| `import-literature` | Mutate | `research_iri: iri`, `source: iri`, `format: string` | Imports literature from a source (DOI, arXiv, PubMed, local file) |
| `import-dataset` | Mutate | `research_iri: iri`, `source: iri`, `format: string` | Imports a dataset (CSV, JSON, CBOR-LD, sensor feed) |
| `import-web` | Mutate | `research_iri: iri`, `url: string` | Imports web content (page, API, linked data) |
| `set-corpus-confidence` | Mutate | `item_iri: iri`, `confidence: float` | Sets confidence in a corpus item's reliability |
| `tag-corpus-item` | Mutate | `item_iri: iri`, `tags: [string]` | Tags a corpus item for categorisation |
| `query-corpus` | Query | `research_iri: iri`, `filter: string` | Queries corpus by type, confidence, tags, provenance, or time range |
| `deduplicate-corpus` | Query | `research_iri: iri` | Finds duplicate or near-duplicate corpus items |
| `extract-from-corpus` | Query | `item_iri: iri`, `extraction_type: string` | Runs NLP extraction on a corpus item (entities, relations, topics, frames) |
| `annotate-corpus-item` | Mutate | `item_iri: iri`, `annotation: CBOR-LD` | Adds semantic annotations to a corpus item (context markup, entity links) |

#### `dynamics` — social, socio-economic, spatio-temporal analysis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `define-social-dynamics` | Mutate | `research_iri: iri`, `dynamics: SocialDynamics` | Defines social dynamics — networks, structures, power relations, cultural norms |
| `define-economic-dynamics` | Mutate | `research_iri: iri`, `dynamics: SocioEconomicDynamics` | Defines socio-economic dynamics — markets, inequality, capital flows, access patterns |
| `define-spatiotemporal-dynamics` | Mutate | `research_iri: iri`, `dynamics: SpatioTemporalDynamics` | Defines spatio-temporal dynamics — migration, diffusion, environmental change, urban dynamics |
| `link-dynamics` | Mutate | `research_iri: iri`, `link: DynamicLink` | Links two dynamics with a cross-dynamic link (may use a different ontology than either dynamic) |
| `query-dynamics` | Query | `research_iri: iri`, `filter: string` | Queries dynamics by type, scope, or linked dynamics |
| `analyse-social-network` | Query | `dynamics_iri: iri`, `analysis: string` | Analyses social network — centrality, communities, bridges, structural holes |
| `analyse-inequality` | Query | `dynamics_iri: iri`, `axes: [string]` | Analyses inequality patterns along specified axes (class, race, gender, geography) |
| `analyse-diffusion` | Query | `dynamics_iri: iri`, `model: string` | Analyses diffusion processes — fits models (S-curve, exponential, network-mediated) |
| `overlay-dynamics` | Query | `dynamics_a: iri`, `dynamics_b: iri` | Overlays two dynamics on a shared spatio-temporal frame for comparison |
| `detect-cross-dynamic-patterns` | Query | `research_iri: iri` | Detects patterns that emerge from the interaction of multiple dynamics |

#### `dark-links` — hidden connection inference

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `infer-dark-link` | Mutate | `research_iri: iri`, `from: iri`, `to: iri`, `type: DarkLinkType`, `method: string` | Infers a dark link (hidden, latent, suppressed, inferred) using a specified method |
| `detect-provenance-gaps` | Query | `research_iri: iri` | Detects provenance gaps — missing records, deleted references, unexplained temporal gaps, broken derivative chains |
| `detect-concealment-patterns` | Query | `research_iri: iri` | Detects patterns that suggest deliberate concealment — inconsistent metadata, selective omission, coordinated narrative |
| `link-dark-link-evidence` | Mutate | `dark_link_iri: iri`, `evidence: iri` | Links corpus items as indirect evidence for a dark link |
| `set-dark-link-confidence` | Mutate | `dark_link_iri: iri`, `confidence: float` | Sets confidence in an inferred dark link |
| `query-dark-links` | Query | `research_iri: iri`, `filter: string` | Queries dark links by type, confidence, inference method, or connected entities |
| `confirm-dark-link` | Mutate | `dark_link_iri: iri` | Promotes a dark link to a visible link (when sufficient evidence is gathered) |
| `refute-dark-link` | Mutate | `dark_link_iri: iri` | Marks a dark link as refuted (evidence contradicts the inference) |
| `trace-dark-link-provenance` | Query | `dark_link_iri: iri` | Traces the full provenance chain of a dark link — evidence, inference method, provenance gaps |

#### `inference` — drawing conclusions

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `make-inference` | Mutate | `research_iri: iri`, `type: InferenceType`, `premises: [iri]`, `reasoning: string` | Makes an inference (deductive, inductive, abductive, analogical, causal, counterfactual) from premises |
| `chain-inference` | Mutate | `inference_iri: iri`, `depends_on: iri` | Chains an inference on a prior inference (inference depends on inference) |
| `set-inference-confidence` | Mutate | `inference_iri: iri`, `confidence: float` | Sets confidence in an inference |
| `query-inferences` | Query | `research_iri: iri`, `filter: string` | Queries inferences by type, confidence, premises, or conclusion |
| `validate-inference` | Query | `inference_iri: iri` | Validates an inference — checks premise reliability, reasoning validity, and conclusion grounding |
| `trace-inference-chain` | Query | `inference_iri: iri` | Traces the full inference chain from premises to conclusion |
| `compare-inferences` | Query | `inference_a: iri`, `inference_b: iri` | Compares two inferences — consistency, overlap, contradiction |

#### `synthesis` — integrating findings

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-finding` | Mutate | `research_iri: iri`, `statement: string`, `evidence: [iri]`, `inference: iri?` | Creates a finding from evidence and optional inference |
| `set-finding-confidence` | Mutate | `finding_iri: iri`, `confidence: float` | Sets finding confidence |
| `mark-finding-contested` | Mutate | `finding_iri: iri`, `contested: bool` | Marks a finding as contested |
| `link-finding-to-question` | Mutate | `finding_iri: iri`, `question_iri: iri` | Links a finding to the research question it answers |
| `create-synthesis` | Mutate | `research_iri: iri`, `type: SynthesisType`, `findings: [iri]` | Creates a synthesis (narrative, systematic review, meta-analysis, theory, model, ontology, framework) |
| `add-finding-to-synthesis` | Mutate | `synthesis_iri: iri`, `finding_iri: iri` | Adds a finding to a synthesis |
| `query-findings` | Query | `research_iri: iri`, `filter: string` | Queries findings by confidence, contested status, or linked question |
| `query-syntheses` | Query | `research_iri: iri`, `filter: string` | Queries syntheses by type or included findings |
| `export-synthesis` | Mutate | `synthesis_iri: iri`, `format: string` | Exports a synthesis as a research output (paper, report, ontology, model, framework) |

#### `provenance` — tracing and verification

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `trace-provenance` | Query | `iri: iri` | Traces full provenance chain for a finding, inference, corpus item, or dark link |
| `verify-provenance` | Query | `iri: iri` | Verifies provenance integrity — no gaps, all sources documented, all transformations recorded |
| `detect-provenance-break` | Query | `iri: iri` | Detects breaks in provenance chains — missing sources, undocumented transformations, broken derivative links |
| `compare-provenance` | Query | `iri_a: iri`, `iri_b: iri` | Compares provenance chains of two items — shared sources, common contributors, derivative relationships |
| `export-provenance-report` | Mutate | `research_iri: iri`, `format: string` | Exports a provenance report for the entire research project |

### 1.3 Research workflow

```
    ┌──────────────────┐
    │  define-scope     │  ← what is in/out of bounds
    │  set-purpose      │  ← why are we doing this
    │  add-question     │  ← what are we asking
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  build corpus     │  ← gather sources, literature, data
    │  extract NLP      │  ← entities, relations, topics, frames
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  define dynamics  │  ← social, economic, spatio-temporal
    │  link dynamics    │  ← cross-dynamic links (different ontologies)
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  infer dark links │  ← hidden, latent, suppressed, inferred
    │  detect gaps      │  ← provenance gaps, concealment patterns
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  make inferences  │  ← deductive, inductive, abductive, causal
    │  chain inferences │  ← inference depends on inference
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  create findings  │  ← evidence + inference → finding
    │  synthesise       │  ← narrative, theory, model, ontology
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  export output    │  ← paper, dataset, model, ontology, framework
    │  provenance trace │  ← full audit trail
    └──────────────────┘
```

### 1.4 Dark link inference methods

| Method | Description | Dark Link Type |
|:-------|:------------|:---------------|
| Network analysis | Infers links from network structure — missing edges that should exist based on network topology | latent, inferred |
| Provenance gap detection | Detects missing provenance records, deleted references, broken derivative chains | suppressed, hidden |
| Pattern matching | Matches patterns across dynamics that suggest a connection | inferred |
| Counterfactual reasoning | Infers what must be true for observed outcomes to occur | inferred |
| NLP extraction | Extracts implicit relationships from text that are not explicitly stated | latent, inferred |
| Temporal correlation | Infers links from temporal co-occurrence or sequence | inferred |
| Spatial co-location | Infers links from spatial proximity or co-location | inferred |
| Narrative inconsistency | Detects inconsistencies in narratives that suggest concealed connections | hidden, suppressed |

### 1.5 Cross-dynamic ontology structures

Different dynamics may use different ontological structures for their links:

| Dynamic A | Dynamic B | Link ontology | Example |
|:----------|:----------|:--------------|:--------|
| Social network | Market structure | Social-economic ontology | A social tie that creates market advantage |
| Environmental change | Migration pattern | Climate-migration ontology | Drought that drives displacement |
| Inequality pattern | Health outcome | Socio-economic health ontology | Wealth gradient that produces health gradient |
| Urban dynamics | Social network | Urban-social ontology | Gentrification that restructures social ties |
| Capital flow | Political power | Political economy ontology | Financial leverage that creates political influence |

The `linkStructure` property on `DynamicLink` specifies which ontology governs the cross-dynamic link — it may be a third ontology, not the ontology of either dynamic.

### 1.6 Zero-shot deficit & progressive enrichment

At project start, the research project has minimal context — no corpus, no hypotheses, no dynamics mapped. The zero-shot deficit is the gap between what is needed and what is available. It is not a failure state but a starting state that progressively closes as agents contribute.

#### `bootstrap` — zero-shot starting point and progressive enrichment

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `assess-deficit` | Query | `research_iri: iri` | Assesses the zero-shot deficit — computes deficit score from corpus coverage, hypothesis coverage, dynamics coverage, and evidence coverage |
| `identify-frontier` | Query | `research_iri: iri` | Identifies the knowledge frontier — known zone, unknown zone, and unknown-unknown zone |
| `generate-bootstrap-hypotheses` | Mutate | `research_iri: iri`, `agent: iri`, `basis: string` | Generates bootstrap hypotheses from scope and purpose alone (before any corpus is built). Low confidence by design. |
| `refine-bootstrap` | Mutate | `bootstrap_iri: iri`, `new_statement: string`, `evidence: [iri]`, `agent: iri` | Refines a bootstrap hypothesis with new evidence (status → refined) |
| `confirm-bootstrap` | Mutate | `bootstrap_iri: iri`, `evidence: [iri]`, `agent: iri` | Confirms a bootstrap hypothesis with sufficient evidence (status → confirmed) |
| `reframe-bootstrap` | Mutate | `bootstrap_iri: iri`, `new_statement: string`, `reason: string`, `agent: iri` | Reframes a bootstrap hypothesis into a better formulation (status → reframed) |
| `disprove-bootstrap` | Mutate | `bootstrap_iri: iri`, `evidence: [iri]`, `agent: iri` | Disproves a bootstrap hypothesis with contradicting evidence (status → disproven) |
| `supersede-bootstrap` | Mutate | `bootstrap_iri: iri`, `new_bootstrap_iri: iri`, `reason: string`, `agent: iri` | Supersedes a bootstrap with a better hypothesis (status → superseded) |
| `promote-bootstrap` | Mutate | `bootstrap_iri: iri`, `investigation_iri: iri`, `agent: iri` | Promotes a bootstrap to a full inv:Hypothesis in a linked investigation (once sufficient evidence accumulates) |
| `record-enrichment` | Mutate | `research_iri: iri`, `type: EnrichmentType`, `item: iri`, `agent: iri`, `unlocked_capability: string?`, `closed_gap: iri?` | Records an enrichment event — new corpus item, finding, inference, dark link, dynamics, or cross-dynamic link |
| `query-enrichment-history` | Query | `research_iri: iri`, `filter: string` | Queries enrichment events by type, agent, capability unlocked, or gap closed |
| `query-capabilities` | Query | `research_iri: iri` | Lists currently available analysis capabilities based on accumulated context |
| `suggest-enrichment` | Query | `research_iri: iri`, `agent: iri` | Suggests what to do next — which gaps to close, which capabilities to unlock, which bootstrap hypotheses to refine |
| `query-frontier` | Query | `research_iri: iri`, `zone: string` | Queries the knowledge frontier by zone (known, unknown, unknown-unknown) |
| `identify-unknown-unknowns` | Query | `research_iri: iri`, `agent: iri` | Identifies potential unknown-unknowns through cross-domain analogy, gap analysis, or agent suggestion |

### 1.7 Directory structure

```
toolboxes/research/
├── mod.rs
├── ontology.n3                  ← imports research.n3
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── enquiry/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   ├── corpus/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   ├── dynamics/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   ├── dark_links/
│   │   ├── mod.rs
│   │   └── tools/               ← 9 tools
│   ├── inference/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   ├── synthesis/
│   │   ├── mod.rs
│   │   └── tools/               ← 9 tools
│   ├── provenance/
│   │   ├── mod.rs
│   │   └── tools/               ← 5 tools
│   └── bootstrap/
│       ├── mod.rs
│       └── tools/               ← 15 tools
```

### 1.8 Capability scope

- `graph:read`, `graph:mutate` — reading/writing research graph (questions, findings, inferences, links)
- `provenance:read`, `provenance:write` — provenance tracing and recording
- `datasource:query` — querying external datasources for corpus building
- `nlp:analyze` — running NLP extraction on corpus items
- `neural:infer` — running neural models for pattern detection and dark link inference (optional)
- `ontology:load`, `ontology:query` — loading and querying scope/purpose/dynamic ontologies
- `research:read`, `research:mutate` — research-specific operations

---

## 2. Research manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `research-board` | centre | Primary workspace |
| `corpus-browser` | left | Corpus management |
| `inference-graph` | right | Inference chains |
| `question-tree` | right (panel) | Research questions |
| `dynamic-map` | centre (tab) | Spatio-temporal dynamics |
| `dark-link-explorer` | centre (tab) | Dark link inference |
| `synthesis-view` | centre (tab) | Synthesis |
| `literature-review` | left (tab) | Literature |
| `data-canvas` | left (tab) | Dataset exploration |
| `provenance-trace` | bottom (panel) | Provenance chains |
| `constraint-panel` | bottom (panel) | Research constraints |

---

## 3. Relationship to investigation toolboxes

Research and investigation are related but distinct:

| Aspect | Research | Investigation |
|:-------|:---------|:--------------|
| Direction | Exploratory, generative | Reconstructive (forensic) or predictive (forecast) |
| Questions | Open-ended, evolving | Specific, case-bounded |
| Evidence | Corpus-based, diverse sources | Chain of custody, admissibility-focused |
| Output | Findings, theories, models, ontologies | Case conclusions, risk assessments |
| Links | Cross-dynamic, dark links | Typed links between subjects/events |
| Inference | Deductive, inductive, abductive, analogical, causal, counterfactual | Evidence evaluation (supports/contradicts) |
| Provenance | Source tracing, gap detection | Chain of custody, transfer records |

A research project may `relatedInvestigation` zero or more investigations. Research findings may inform investigation hypotheses; investigation evidence may enrich research corpora.

---

## 4. Loose coupling

| Tool | `research-board` | `graph` | `doc` | `map` | `case-board` |
|:-----|:------------------|:--------|:------|:------|:-------------|
| `add-corpus-item` | ✅ primary | — | ✅ from document | ✅ from spatial data | ✅ from evidence |
| `infer-dark-link` | ✅ primary | ✅ | — | — | ✅ |
| `make-inference` | ✅ primary | — | — | — | ✅ |
| `define-social-dynamics` | ✅ | ✅ | — | ✅ | — |
| `detect-provenance-gaps` | ✅ | ✅ | ✅ | — | ✅ |
| `create-synthesis` | ✅ primary | — | ✅ output as document | — | — |

---

## 5. Relationship to existing specs

| Document | Relationship |
|:---------|:-------------|
| [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md) | Parent spec — hierarchy, core traits, ontology layer |
| [`qualia-ui/ontologies/research.n3`](../ontologies/research.n3) | Research ontology — projects, scope, constraints, questions, corpus, dynamics, dark links, inferences, findings, synthesis |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — now includes research containers |
| [`qualia-ui/ontologies/provenance.n3`](../ontologies/provenance.n3) | Provenance — source tracing, derivative chains, provenance gaps |
| [`qualia-ui/ontologies/agency.n3`](../ontologies/agency.n3) | Agency — research actors, contributors, claims |
| [`qualia-ui/ontologies/investigation.n3`](../ontologies/investigation.n3) | Investigation ontology — related investigations, shared evidence |
| [`TOOLBOX_INVESTIGATION_SPEC.md`](TOOLBOX_INVESTIGATION_SPEC.md) | Investigation toolboxes — shared link analysis, provenance tracing |
| [`TOOLBOX_CODE_SPEC.md`](TOOLBOX_CODE_SPEC.md) | AI toolbox — NLP extraction for corpus, neural inference for dark links |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation |
