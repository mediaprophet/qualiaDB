# Tool-Chest Spec — Investigation & Forecast Toolboxes

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Ontology:** [`qualia-ui/ontologies/investigation.n3`](../ontologies/investigation.n3) (N3 authoring → CBOR-LD runtime)

These two toolboxes serve investigation and forecasting work — forensic reconstruction, evidence-based hypothesis testing, link analysis, timeline construction, and forward-looking scenario modelling. They are **loosely coupled** to containers — the same tool works across different container and manifold types.

---

## 1. Toolbox: `investigate` (Forensic Investigation)

The `investigate` toolbox is for backward-looking, evidence-based inquiry. It covers evidence collection, chain of custody, hypothesis testing, timeline construction, link analysis, and case management. The toolbox is loosely coupled: tools work in a `case-board` container, a `graph` container (for link analysis), a `doc` container (for evidence documents), or embedded in a `map` container (for spatial analysis).

### 1.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `case-board` | content | missing | Primary investigation workspace — evidence, hypotheses, links, timelines on a spatial canvas. |
| `evidence-board` | content | missing | Evidence collection and verification view. |
| `timeline` | content | missing | Multi-track temporal sequence of events and evidence. |
| `hypothesis-tree` | content | missing | Hierarchical hypothesis view with evidence evaluations. |
| `dossier` | content | missing | Subject profile — biographical, organisational, or systemic. |
| `link-graph` | content | missing | Network graph of links between subjects, events, and evidence. |
| `constituency-panel` | panel | missing | Investigation constituencies — roles, access levels, contributions. |
| `custody-log` | panel | missing | Chain of custody log for selected evidence. |
| `inspector` | panel | missing | Inspects selected evidence, hypothesis, or link. |
| `property-sheet` | panel | missing | Edits investigation properties (status, jurisdiction, classification). |

### 1.2 Tool-chains

#### `case` — case management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `new-investigation` | Mutate | `name: string`, `mode: InvestigationMode`, `jurisdiction: string` | Creates a new investigation (forensic, forecasting, or hybrid) |
| `set-mode` | Mutate | `investigation_iri: iri`, `mode: InvestigationMode` | Sets or changes investigation mode |
| `set-status` | Mutate | `investigation_iri: iri`, `status: InvestigationStatus` | Sets investigation status (open, suspended, concluded, cold case, reopened, ongoing) |
| `set-jurisdiction` | Mutate | `investigation_iri: iri`, `jurisdiction: string` | Sets legal/regulatory jurisdiction scope |
| `add-subject` | Mutate | `investigation_iri: iri`, `subject: Subject` | Adds a subject (person, organisation, system, phenomenon, event) |
| `add-topic` | Mutate | `investigation_iri: iri`, `topic: Topic` | Adds a topic area with optional ontology binding |
| `add-event` | Mutate | `investigation_iri: iri`, `event: Event` | Adds an event with temporal extent, location, participants |
| `add-case` | Mutate | `investigation_iri: iri`, `case: Case` | Associates a formal case file (case number, classification, framework) |
| `add-constituency` | Mutate | `investigation_iri: iri`, `constituency: Constituency` | Adds a constituency with role, entity, and access level |
| `query-investigation` | Query | `investigation_iri: iri`, `filter: string` | Queries investigation contents (subjects, events, evidence, hypotheses) |
| `export-findings` | Mutate | `investigation_iri: iri`, `format: string` | Exports findings report (CBOR-LD, document, HCF) |

#### `evidence` — evidence collection and verification

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `collect-evidence` | Mutate | `investigation_iri: iri`, `evidence: Evidence` | Collects a new piece of evidence (documentary, testimonial, physical, digital, sensor, analytical, circumstantial) |
| `set-reliability` | Mutate | `evidence_iri: iri`, `reliability: ReliabilityRating` | Sets evidence reliability (confirmed, likely, possible, doubtful, disputed, fabricated) |
| `verify-evidence` | Query | `evidence_iri: iri` | Runs verification checks — source provenance, hash integrity, cross-reference |
| `link-evidence-source` | Mutate | `evidence_iri: iri`, `source_iri: iri` | Links evidence to its source artifact (document, sensor feed, testimony record) |
| `tag-evidence` | Mutate | `evidence_iri: iri`, `tags: [string]` | Tags evidence for categorisation and retrieval |
| `query-evidence` | Query | `investigation_iri: iri`, `filter: string` | Queries evidence by type, reliability, source, tags, or time range |
| `redact-evidence` | Mutate | `evidence_iri: iri`, `redaction: string` | Redacts sensitive content (requires constituency access level check) |
| `compare-evidence` | Query | `evidence_a: iri`, `evidence_b: iri` | Compares two evidence items — consistency, overlap, contradiction |

#### `custody` — chain of custody

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `init-custody` | Mutate | `evidence_iri: iri`, `holder: iri`, `reason: string` | Initialises chain of custody for a piece of evidence |
| `transfer-custody` | Mutate | `evidence_iri: iri`, `from: iri`, `to: iri`, `reason: string` | Records a custody transfer |
| `record-transform` | Mutate | `evidence_iri: iri`, `transform: string`, `actor: iri` | Records a transformation applied during custody (format conversion, redaction, hash) |
| `verify-custody` | Query | `evidence_iri: iri` | Verifies chain of custody integrity — no gaps, all transfers documented |
| `custody-history` | Query | `evidence_iri: iri` | Returns the full custody history for an evidence item |

#### `hypothesis` — hypothesis testing

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `propose-hypothesis` | Mutate | `investigation_iri: iri`, `statement: string`, `parent: iri?` | Proposes a new hypothesis (optionally as a sub-hypothesis) |
| `set-hypothesis-status` | Mutate | `hypothesis_iri: iri`, `status: HypothesisStatus` | Sets hypothesis status (proposed, testing, supported, confirmed, contradicted, disproven, inconclusive) |
| `evaluate-evidence` | Mutate | `hypothesis_iri: iri`, `evidence_iri: iri`, `verdict: EvaluationVerdict`, `reasoning: string`, `confidence: float` | Evaluates a piece of evidence against a hypothesis (supports, contradicts, neutral, conditional) |
| `set-mutually-exclusive` | Mutate | `hypothesis_a: iri`, `hypothesis_b: iri` | Marks two hypotheses as mutually exclusive |
| `query-hypotheses` | Query | `investigation_iri: iri`, `filter: string` | Queries hypotheses by status, evidence count, or confidence |
| `hypothesis-summary` | Query | `hypothesis_iri: iri` | Summarises a hypothesis — supporting evidence, contradicting evidence, confidence |
| `rank-hypotheses` | Query | `investigation_iri: iri` | Ranks hypotheses by evidence support (weighted by reliability and confidence) |

#### `timeline` — temporal reconstruction

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-timeline` | Mutate | `investigation_iri: iri`, `name: string` | Creates a new timeline (single or multi-track) |
| `add-entry` | Mutate | `timeline_iri: iri`, `entry: TimelineEntry` | Adds a timeline entry (event, evidence, or hypothesis evaluation) |
| `set-time-precision` | Mutate | `entry_iri: iri`, `precision: string` | Sets time precision (exact, approximate±N, estimated, unknown) |
| `set-track` | Mutate | `entry_iri: iri`, `track: string` | Sets the track name for multi-track timelines |
| `query-timeline` | Query | `timeline_iri: iri`, `range: [dateTime;2]?` | Queries timeline entries, optionally filtered by time range |
| `detect-sequences` | Query | `timeline_iri: iri` | Detects temporal sequences and patterns (before/after, simultaneous, periodic) |
| `reconcile-timelines` | Query | `timeline_a: iri`, `timeline_b: iri` | Reconciles two timelines — finds overlaps, conflicts, and gaps |

#### `link-analysis` — relationship mapping

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `add-link` | Mutate | `investigation_iri: iri`, `from: iri`, `to: iri`, `type: LinkType`, `strength: float` | Adds a typed link between subjects, events, or evidence |
| `link-evidence` | Mutate | `link_iri: iri`, `evidence_iri: iri` | Associates evidence supporting a link |
| `query-links` | Query | `investigation_iri: iri`, `filter: string` | Queries links by type, strength, or connected entity |
| `find-path` | Query | `investigation_iri: iri`, `from: iri`, `to: iri` | Finds connection paths between two entities through the link graph |
| `detect-clusters` | Query | `investigation_iri: iri` | Detects clusters/cliques in the link graph (groups of closely connected entities) |
| `compute-centrality` | Query | `investigation_iri: iri` | Computes centrality metrics (degree, betweenness, eigenvector) for the link graph |
| `visualise-links` | Query | `investigation_iri: iri`, `layout: string` | Generates a layout for link graph visualisation (force-directed, hierarchical, circular) |

#### `hypothesis-graph` — living, versioned, multi-agent hypothesis graph

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-hypothesis-graph` | Mutate | `investigation_iri: iri` | Creates a new living hypothesis graph for the investigation |
| `add-to-graph` | Mutate | `graph_iri: iri`, `hypothesis_iri: iri` | Adds a hypothesis to the graph (creates a contribution of type add-hypothesis) |
| `contribute-evaluation` | Mutate | `graph_iri: iri`, `hypothesis_iri: iri`, `evidence_iri: iri`, `verdict: EvaluationVerdict`, `reasoning: string`, `confidence: float`, `agent: iri` | Agent contributes an evidence evaluation to a hypothesis in the graph |
| `contribute-confidence-revision` | Mutate | `graph_iri: iri`, `hypothesis_iri: iri`, `new_confidence: float`, `reason: string`, `agent: iri` | Agent revises the confidence of a hypothesis with reasoning |
| `bridge-dark-link` | Mutate | `graph_iri: iri`, `dark_link_iri: iri`, `hypothesis_iri: iri`, `verdict: EvaluationVerdict`, `confidence: float`, `reasoning: string`, `agent: iri` | Bridges a dark link (from research ontology) to a hypothesis in the graph |
| `reframe-hypothesis` | Mutate | `graph_iri: iri`, `hypothesis_iri: iri`, `new_statement: string`, `reason: string`, `agent: iri` | Agent reframes a hypothesis with a better formulation |
| `merge-hypotheses` | Mutate | `graph_iri: iri`, `hypothesis_a: iri`, `hypothesis_b: iri`, `merged_statement: string`, `agent: iri` | Agent merges two hypotheses into one |
| `split-hypothesis` | Mutate | `graph_iri: iri`, `hypothesis_iri: iri`, `sub_hypotheses: [string]`, `agent: iri` | Agent splits a hypothesis into sub-hypotheses |
| `flag-gap` | Mutate | `graph_iri: iri`, `gap_type: GapType`, `description: string`, `priority: string`, `agent: iri` | Agent flags a gap in the hypothesis graph (no-hypothesis, no-evidence, conflicting, insufficient, stale, dark-link-unresolved) |
| `close-gap` | Mutate | `graph_iri: iri`, `gap_iri: iri`, `resolution: string`, `agent: iri` | Agent closes a gap with a resolution |
| `merge-contribution` | Mutate | `graph_iri: iri`, `contribution_iri: iri` | Merges a pending contribution into the graph (applies the contribution to the current revision) |
| `reject-contribution` | Mutate | `graph_iri: iri`, `contribution_iri: iri`, `reason: string` | Rejects a contribution (invalid, redundant, or out of scope) |
| `adjudicate-conflict` | Mutate | `graph_iri: iri`, `contribution_a: iri`, `contribution_b: iri`, `resolution: string`, `agent: iri` | Adjudicates a conflict between two contributions from different agents |
| `create-revision` | Mutate | `graph_iri: iri`, `agent: iri` | Creates a new immutable revision snapshot of the graph |
| `diff-revisions` | Query | `graph_iri: iri`, `from: iri`, `to: iri` | Diffs two revisions — what hypotheses, evaluations, and bridges changed |
| `rollback-revision` | Mutate | `graph_iri: iri`, `target: iri` | Rolls back the graph to a prior revision |
| `query-graph` | Query | `graph_iri: iri`, `filter: string` | Queries the hypothesis graph — by hypothesis status, confidence, gap type, contributor, or dark link bridge |
| `query-gaps` | Query | `graph_iri: iri`, `filter: string` | Queries gaps by type, priority, or age |
| `query-contributions` | Query | `graph_iri: iri`, `filter: string` | Queries contributions by agent, type, status, or time range |
| `query-dark-link-bridges` | Query | `graph_iri: iri`, `filter: string` | Queries dark link bridges by verdict, confidence, or dark link type |
| `compute-confidence` | Query | `graph_iri: iri`, `hypothesis_iri: iri` | Computes aggregate confidence and support score for a hypothesis from all evaluations and bridges |
| `rank-hypotheses` | Query | `graph_iri: iri` | Ranks hypotheses by aggregate confidence and support score |
| `visualise-graph` | Query | `graph_iri: iri`, `layout: string`, `highlight: string` | Generates a visualisation layout — supports confidence-weighted, gap-highlighted, agent-coloured, and dark-link-bridge layouts |
| `subscribe-updates` | Mutate | `graph_iri: iri`, `subscriber: iri`, `filter: string` | Subscribes an agent to live update stream for the graph |
| `unsubscribe-updates` | Mutate | `graph_iri: iri`, `subscriber: iri` | Unsubscribes an agent from the update stream |
| `export-graph` | Mutate | `graph_iri: iri`, `format: string` | Exports the hypothesis graph (CBOR-LD, RDF, document) |

### 1.3 Directory structure

```
toolboxes/investigate/
├── mod.rs
├── ontology.n3                  ← imports investigation.n3
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── case/
│   │   ├── mod.rs
│   │   └── tools/               ← 11 tools
│   ├── evidence/
│   │   ├── mod.rs
│   │   └── tools/               ← 8 tools
│   ├── custody/
│   │   ├── mod.rs
│   │   └── tools/               ← 5 tools
│   ├── hypothesis/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   ├── hypothesis_graph/
│   │   ├── mod.rs
│   │   └── tools/               ← 25 tools
│   ├── timeline/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   └── link_analysis/
│       ├── mod.rs
│       └── tools/               ← 7 tools
```

### 1.4 Capability scope

- `graph:read`, `graph:mutate` — reading/writing investigation graph (evidence, hypotheses, links)
- `provenance:write` — recording provenance for evidence collection and custody transfers
- `agency:evaluate` — evaluating agency claims for constituency roles and evidence actors
- `datasource:query` — querying external datasources (legislation, case law, public records)
- `nlp:analyze` — running NLP extraction on evidence documents
- `investigation:read`, `investigation:mutate` — investigation-specific operations (Sentinel-gated by constituency access level)
- `agency:act` — multi-agent contribution, adjudication, and subscription to hypothesis graph update streams

---

## 2. Toolbox: `forecast` (Scenario Modelling & Prediction)

The `forecast` toolbox is for forward-looking, predictive modelling. It covers scenario generation, probability estimation, trend analysis, projection, and risk assessment. The toolbox is loosely coupled: tools work in a `scenario-board` container, a `forecast-chart` container, a `graph` container (for causal models), or embedded in a `doc` container (for forecast reports).

### 2.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `scenario-board` | content | missing | Forecast scenario workspace — scenarios, probabilities, driving factors, outcomes. |
| `forecast-chart` | content | missing | Quantitative forecast visualisation — time-series, confidence intervals, comparisons. |
| `case-board` | content | missing | Shared with investigate — hybrid investigations use both. |
| `timeline` | content | missing | Shared with investigate — projected events on future timeline tracks. |
| `link-graph` | content | missing | Shared with investigate — causal link analysis for forecasting. |
| `constituency-panel` | panel | missing | Shared with investigate — forecast stakeholders. |
| `inspector` | panel | missing | Inspects selected scenario, outcome, or driving factor. |
| `property-sheet` | panel | missing | Edits forecast parameters (timeframe, confidence level, model settings). |

### 2.2 Tool-chains

#### `scenario` — scenario generation and management

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-scenario` | Mutate | `investigation_iri: iri`, `type: ScenarioType`, `timeframe: string` | Creates a new forecast scenario (best case, worst case, most likely, black swan, branching) |
| `set-probability` | Mutate | `scenario_iri: iri`, `probability: float` | Sets or revises scenario probability (0.0-1.0) |
| `add-driving-factor` | Mutate | `scenario_iri: iri`, `factor: string` | Adds a driving factor (event, trend, policy change, condition) |
| `add-outcome` | Mutate | `scenario_iri: iri`, `outcome: ProjectedOutcome` | Adds a projected outcome (description, metric, value, unit, range) |
| `set-scenario-base` | Mutate | `scenario_iri: iri`, `evidence_iri: iri` | Links scenario to evidence/patterns it's based on |
| `branch-scenario` | Mutate | `scenario_iri: iri`, `branch_point: string`, `sub_scenarios: [Scenario]` | Creates branching sub-scenarios at a decision point |
| `query-scenarios` | Query | `investigation_iri: iri`, `filter: string` | Queries scenarios by type, probability, timeframe |
| `compare-scenarios` | Query | `scenario_a: iri`, `scenario_b: iri` | Compares two scenarios — outcomes, probabilities, driving factors |
| `merge-scenarios` | Mutate | `scenarios: [iri]`, `name: string` | Merges multiple scenarios into a composite |

#### `trend` — trend analysis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `detect-trends` | Query | `datasource: iri`, `metric: string`, `range: [dateTime;2]` | Detects trends in a time-series datasource (linear, exponential, cyclical, seasonal) |
| `fit-model` | Mutate | `data: [float]`, `model_type: string` | Fits a statistical model (linear, polynomial, ARIMA, exponential, logistic) |
| `forecast-trend` | Query | `model_iri: iri`, `horizon: string` | Projects a fitted model forward by a time horizon |
| `detect-anomalies` | Query | `data: [float]`, `sensitivity: float` | Detects anomalies/outliers in time-series data |
| `seasonal-decompose` | Query | `data: [float]`, `period: int` | Decomposes time-series into trend, seasonal, and residual components |
| `correlate-trends` | Query | `trend_a: iri`, `trend_b: iri` | Correlates two trends — finds leading/lagging relationships |
| `import-timeseries` | Mutate | `datasource: iri`, `metric: string`, `range: [dateTime;2]` | Imports time-series data from a datasource for analysis |

#### `risk` — risk assessment

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `assess-risk` | Query | `scenario_iri: iri`, `impact_metric: string` | Assesses risk for a scenario (probability × impact) |
| `build-risk-matrix` | Query | `scenarios: [iri]`, `impact_metric: string` | Builds a risk matrix (probability vs impact) for multiple scenarios |
| `identify-tail-risks` | Query | `investigation_iri: iri` | Identifies tail-risk scenarios (low probability, high impact) |
| `set-risk-threshold` | Mutate | `investigation_iri: iri`, `threshold: float` | Sets a risk threshold for alerting |
| `monitor-risk` | Query | `investigation_iri: iri` | Monitors current risk levels against thresholds (for ongoing investigations) |
| `risk-report` | Query | `investigation_iri: iri`, `format: string` | Generates a risk assessment report |

#### `causal` — causal modelling

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `build-causal-model` | Mutate | `investigation_iri: iri`, `variables: [string]`, `relationships: [CausalLink]` | Builds a causal model (DAG of cause→effect relationships) |
| `query-causal-path` | Query | `model_iri: iri`, `from: string`, `to: string` | Finds causal paths between two variables |
| `intervene` | Query | `model_iri: iri`, `variable: string`, `value: float` | Simulates an intervention (do-calculus) — projects outcome if a variable is set to a value |
| `counterfactual` | Query | `model_iri: iri`, `observed: CBOR-LD`, `hypothetical: CBOR-LD` | Computes a counterfactual — what would have happened if conditions were different |
| `validate-causal-model` | Query | `model_iri: iri`, `evidence: [iri]` | Validates a causal model against available evidence |

### 2.3 Forecasting workflow

```
Evidence & Patterns (from investigate toolbox)
         │
         ▼
   ┌─────────────────┐
   │  detect-trends   │ ← identifies patterns in data
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │  fit-model       │ ← fits statistical model to trends
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │  create-scenario │ ← generates scenarios (best/worst/likely/black-swan)
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │  add-outcome     │ ← projects quantitative outcomes with ranges
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │  assess-risk     │ ← evaluates probability × impact
   └────────┬────────┘
            │
            ▼
   ┌─────────────────┐
   │  risk-report     │ ← generates findings for constituencies
   └─────────────────┘
```

### 2.4 Directory structure

```
toolboxes/forecast/
├── mod.rs
├── ontology.n3                  ← imports investigation.n3 (scenario section)
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── scenario/
│   │   ├── mod.rs
│   │   └── tools/               ← 9 tools
│   ├── trend/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   ├── risk/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   └── causal/
│       ├── mod.rs
│       └── tools/               ← 5 tools
```

### 2.5 Capability scope

- `graph:read`, `graph:mutate` — reading/writing scenario and causal model graphs
- `datasource:query` — querying external datasources for trend data
- `nlp:analyze` — extracting patterns from text-based evidence
- `neural:infer` — running neural models for trend prediction (optional)
- `forecast:read`, `forecast:mutate` — forecast-specific operations
- `provenance:write` — recording provenance for scenario generation and model fitting

---

## 3. Hybrid investigations

Investigations may be **hybrid** — both forensic and forecasting. In this case, both toolboxes are active on the same investigation manifold:

| Phase | Toolbox | Activity |
|:------|:--------|:---------|
| 1. Reconstruct | `investigate` | Collect evidence, build timeline, test hypotheses |
| 2. Understand | `investigate` | Link analysis, causal identification |
| 3. Project | `forecast` | Build scenarios from understood patterns |
| 4. Assess | `forecast` | Risk assessment, tail-risk identification |
| 5. Report | both | Export findings with both forensic and forecast components |

The investigation ontology's `inv:Hybrid` mode signals that both toolboxes should be available. The manifold seed for a hybrid investigation includes both `case-board` and `scenario-board` containers.

---

## 4. Investigation manifold seed

| Container | Dock | Toolbox |
|:----------|:-----|:--------|
| `case-board` | centre | investigate |
| `evidence-board` | left | investigate |
| `hypothesis-tree` | right | investigate |
| `timeline` | bottom | investigate |
| `link-graph` | centre (tab) | investigate |
| `scenario-board` | centre (tab) | forecast (hybrid only) |
| `forecast-chart` | bottom (tab) | forecast (hybrid only) |
| `constituency-panel` | right (panel) | both |
| `custody-log` | left (panel) | investigate |
| `inspector` | right (panel) | both |

---

## 5. Loose coupling

| Tool | Works in `case-board` | Works in `graph` | Works in `doc` | Works in `map` | Works in `scenario-board` |
|:-----|:----------------------|:-----------------|:---------------|:---------------|:--------------------------|
| `collect-evidence` | ✅ primary | — | ✅ from document | ✅ from spatial data | — |
| `add-link` | ✅ | ✅ primary | — | ✅ spatial links | — |
| `propose-hypothesis` | ✅ primary | — | ✅ from document | — | — |
| `create-scenario` | — | — | — | — | ✅ primary |
| `detect-trends` | — | — | — | — | ✅ |
| `build-causal-model` | ✅ | ✅ primary | — | — | ✅ |

---

## 6. Relationship to existing specs

| Document | Relationship |
|:---------|:-------------|
| [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md) | Parent spec — hierarchy, core traits, ontology layer |
| [`qualia-ui/ontologies/investigation.n3`](../ontologies/investigation.n3) | Investigation ontology — cases, evidence, hypotheses, timelines, scenarios, constituencies, links |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — now includes investigation containers |
| [`qualia-ui/ontologies/provenance.n3`](../ontologies/provenance.n3) | Provenance — evidence sources, custody transfers, derivative chains |
| [`qualia-ui/ontologies/agency.n3`](../ontologies/agency.n3) | Agency — investigation actors, constituency entities, claims |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation |
| [`TOOLBOX_CODE_SPEC.md`](TOOLBOX_CODE_SPEC.md) | Code/AI/Spatial toolboxes — NLP tools for evidence analysis, graph tools for link analysis |
