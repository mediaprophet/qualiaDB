# Tool-Chest Spec — Epistemics Toolbox

**Copyright © 2026 Timothy Charles Holborn.** All rights reserved.
**Parent spec:** [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md)
**Ontology:** [`qualia-ui/ontologies/epistemics.n3`](../ontologies/epistemics.n3) (N3 authoring → CBOR-LD runtime)
**Split modules:** [`agent-nomenclature.n3`](../ontologies/agent-nomenclature.n3), [`ungrounded-generation.n3`](../ontologies/ungrounded-generation.n3)
**Nomenclature rules:** [`qualia-db-standards/agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md)

The `epistemics` toolbox evaluates the epistemic status of claims, observations, behaviours, and expressions — whether they are grounded in objective reality, subjective perspective, intersubjective consensus, or fictional construction. It is **agent-centric**: any entity (human, software agent, sensor, organisation, AI model) can be an epistemic agent with a perspective. It considers spatio-temporal and social relations as context that shapes what is subjective vs objective.

**Nomenclature note:** This toolbox follows the agent nomenclature isolation rules (see [`agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md)). Terms are scoped to the agent type whose STEM mechanism they accurately describe. Mind-dependent terms (cognitive, perception, belief, emotion, hallucination, deliberate) apply only to natural agents. Intent-dependent terms (fabricated, deceptive) apply to natural agents and legal persons, never to software agents. Software agent errors are "enumeration failures" or "ungrounded generation" — statistical process outcomes, not mental phenomena.

This toolbox is **loosely coupled** to the investigation and research toolboxes — epistemic assessments can inform hypothesis evaluation, evidence assessment, and research findings, but each can exist independently.

---

## 1. Toolbox: `epistemics` (Subjective vs Objective Reality Evaluation)

### 1.1 Containers placed by this toolbox

| Container | Kind | Honesty | Notes |
|:----------|:-----|:--------|:------|
| `reality-board` | content | missing | Primary workspace — claims, behaviours, expressions with epistemic status, reality classification, confidence |
| `sentiment-map` | content | missing | Sentiment across agents, targets, and time — type, intensity, authenticity, multi-dimensional scores |
| `perspective-view` | content | missing | Compares multiple agent perspectives on the same target |
| `intentionality-panel` | panel | missing | Intentionality assessment and mistake classification |
| `grounding-panel` | panel | missing | Behaviour grounding — what basis the agent acted on |
| `fiction-classifier` | panel | missing | Fiction/non-fiction/blended classification with sub-categories |
| `social-context-panel` | panel | missing | Social relation context — power, trust, consensus, incentive, pressure |
| `inspector` | panel | missing | Inspects selected assessment, perspective, or sentiment |

### 1.2 Tool-chains

#### `evaluate` — epistemic status assessment

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-assessment` | Mutate | `target: iri`, `agent: iri` | Creates a new epistemic assessment for a claim, behaviour, or expression |
| `set-epistemic-mode` | Mutate | `assessment_iri: iri`, `mode: EpistemicMode`, `confidence: float`, `reasoning: string` | Sets the epistemic mode (objective, subjective, intersubjective, contested, ambiguous, fictional) with reasoning |
| `set-reality-category` | Mutate | `assessment_iri: iri`, `category: RealityCategory`, `confidence: float`, `reasoning: string` | Sets the reality classification (non-fiction sub-type, fiction sub-type, or blended sub-type) |
| `mark-disputed` | Mutate | `assessment_iri: iri`, `disputing_agent: iri`, `reason: string` | Marks the reality classification as disputed by another agent |
| `set-spatio-temporal-context` | Mutate | `assessment_iri: iri`, `spatial_scope: string`, `temporal_scope: string`, `resolution: string`, `agent_position: string`, `mediation: MediationType` | Sets the spatio-temporal context — where, when, at what resolution, and how the agent's access is mediated |
| `set-social-context` | Mutate | `assessment_iri: iri`, `relation_type: SocialRelationType`, `power_asymmetry: float`, `trust_level: float`, `community_consensus: float`, `incentive_misrepresent: float`, `social_pressure: float` | Sets the social relation context |
| `query-assessments` | Query | `filter: string` | Queries assessments by epistemic mode, reality category, confidence, agent, or target |
| `compare-assessments` | Query | `assessment_a: iri`, `assessment_b: iri` | Compares two assessments of the same target by different agents |
| `assess-recursive` | Mutate | `target_assessment: iri`, `agent: iri` | Creates a recursive assessment — an assessment of an assessment (for multi-agent disagreement) |
| `export-assessment` | Mutate | `assessment_iri: iri`, `format: string` | Exports the assessment (CBOR-LD, RDF, document) |
| `link-to-investigation` | Mutate | `assessment_iri: iri`, `hypothesis_iri: iri` | Links the assessment to an investigation hypothesis |
| `link-to-research` | Mutate | `assessment_iri: iri`, `finding_iri: iri` | Links the assessment to a research finding |

#### `perspective` — agent perspective analysis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `register-perspective` | Mutate | `agent: iri`, `type: PerspectiveType`, `access_level: string` | Registers an agent's perspective — type (human, software-agent, sensor, organisation, ai-model, collective) and access level |
| `add-bias` | Mutate | `perspective_iri: iri`, `bias: Bias`, `description: string` | Adds a bias to an agent's perspective (cognitive, algorithmic, institutional, sensor) |
| `query-perspectives` | Query | `target: iri` | Queries all perspectives on a given target |
| `compare-perspectives` | Query | `target: iri` | Compares all agent perspectives on the same target — where they agree, disagree, and why |
| `detect-perspective-conflict` | Query | `target: iri` | Detects conflicts between agent perspectives — different epistemic modes, different reality classifications |
| `assess-perspective-coverage` | Query | `target: iri` | Assesses what perspectives are missing — which agent types have not yet weighed in |
| `reconcile-perspectives` | Mutate | `target: iri`, `agent: iri`, `strategy: string` | Attempts to reconcile conflicting perspectives (consensus, weighted average, adjudication, preserve disagreement) |

#### `intentionality` — intentional vs unintentional

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `assess-intentionality` | Mutate | `target: iri`, `agent: iri`, `type: IntentionalityType`, `confidence: float`, `evidence: [iri]`, `reasoning: string` | Assesses whether an action or outcome was deliberate, intentional, negligent, reckless, accidental, mistaken, systematic, emergent, coerced, or habitual |
| `classify-mistake` | Mutate | `target: iri`, `type: MistakeType`, `severity: string`, `correctable: bool`, `recurrence: string`, `reasoning: string` | Classifies a mistake — false-belief, skill-deficit, attention-failure, perception-error, judgement-error, system-design, deliberate-deviation, ungrounded-generation, simulation-artefact |
| `query-intentionality` | Query | `filter: string` | Queries intentionality assessments by type, confidence, or agent |
| `query-mistakes` | Query | `filter: string` | Queries mistake classifications by type, severity, or recurrence |
| `detect-mistake-patterns` | Query | `agent: iri` | Detects patterns in an agent's mistakes — recurring types, systematic biases, skill gaps |
| `compare-intentionality` | Query | `target: iri` | Compares intentionality assessments of the same action by different agents |

#### `grounding` — behaviour grounding analysis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `assess-grounding` | Mutate | `target: iri`, `agent: iri`, `type: GroundingType`, `basis: string`, `verifiable: bool`, `confidence: float`, `reasoning: string` | Assesses what grounds an agent's behaviour — observation, hypothesis, interpretation, belief, assumption, inference, instruction, emotion, instinct, habit, simulation, fiction, norm, values |
| `query-grounding` | Query | `filter: string` | Queries grounding assessments by type, verifiability, or agent |
| `detect-ungrounded-behaviour` | Query | `agent: iri` | Detects behaviours that lack clear grounding — actions without stated basis |
| `verify-grounding` | Query | `grounding_iri: iri` | Attempts to verify the grounding — can the basis be confirmed by other agents? |
| `compare-grounding` | Query | `target: iri` | Compares grounding assessments of the same behaviour by different agents |

#### `fiction` — reality classification

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `classify-reality` | Mutate | `target: iri`, `agent: iri`, `category: RealityCategory`, `confidence: float`, `reasoning: string` | Classifies a document, claim, or expression as fiction, non-fiction, or blended, with sub-category |
| `query-classifications` | Query | `filter: string` | Queries reality classifications by category, sub-type, confidence, or disputed status |
| `detect-deceptive-fiction` | Query | `target: iri` | Detects fiction presented as non-fiction with intent to deceive (deceptive-fiction category) |
| `detect-blended-content` | Query | `target: iri` | Detects content that blends fiction and non-fiction (mythological, legendary, propagandistic, marketing) |
| `compare-classifications` | Query | `target: iri` | Compares reality classifications of the same content by different agents |
| `trace-fiction-to-reality` | Query | `target: iri` | Traces connections from fictional content to real entities, events, or situations it references (allegorical, satirical, historical fiction) |

#### `sentiment` — sentiment evaluation and analysis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `assess-sentiment` | Mutate | `target: iri`, `agent: iri`, `type: SentimentType`, `intensity: SentimentIntensity`, `score: float`, `authenticity: SentimentAuthenticity`, `context: string` | Assesses an agent's sentiment towards a target — type, intensity, score, authenticity (genuine, performed, manipulated, simulated) |
| `set-sentiment-dimension` | Mutate | `sentiment_iri: iri`, `dimension: SentimentDimension`, `score: float` | Sets a multi-dimensional sentiment score (valence, arousal, dominance, certainty, moral, aesthetic, irony, sarcasm) |
| `link-sentiment-to-reality` | Mutate | `sentiment_iri: iri`, `reality_category: RealityCategory` | Links sentiment to the reality category of its target — sentiment towards fiction vs non-fiction |
| `query-sentiment` | Query | `filter: string` | Queries sentiment by type, intensity, authenticity, agent, or target |
| `analyse-sentiment-trends` | Query | `target: iri`, `time_range: string` | Analyses how sentiment towards a target has changed over time |
| `detect-sentiment-manipulation` | Query | `target: iri` | Detects patterns suggesting sentiment manipulation — coordinated sentiment, artificial amplification, performed sentiment |
| `detect-performed-sentiment` | Query | `agent: iri` | Detects patterns where an agent's expressed sentiment differs from likely felt sentiment (irony, sarcasm, social performance) |
| `map-sentiment-network` | Query | `target: iri` | Maps sentiment flow through social networks — who influences whose sentiment |
| `compare-sentiment` | Query | `target: iri` | Compares sentiment towards the same target across agents |
| `detect-sentiment-reality-mismatch` | Query | `target: iri` | Detects cases where sentiment intensity is disproportionate to the reality category (e.g. extreme anger at a fictional character) |

#### `ungrounded-generation` — AI model output diagnosis

| Tool | Action | Parameters | Notes |
|:-----|:-------|:-----------|:------|
| `create-ug-instance` | Mutate | `target: iri`, `model_id: string`, `sampling_params: string`, `context_length: int`, `retrieval_used: bool` | Creates an ungrounded generation instance — records the model, sampling parameters, context length, and whether RAG was used |
| `set-ug-cause` | Mutate | `ug_iri: iri`, `cause: UGCause`, `reasoning: string` | Sets the cause of the ungrounded generation — training-data-gap, context-window-overflow, attention-misallocation, sampling-artefact, retrieval-failure, alignment-tax, sycophancy, fine-tuning-drift, tokenization-artefact, quantisation-artefact, etc. (18 causes) |
| `set-ug-consequence` | Mutate | `ug_iri: iri`, `consequence: UGConsequence`, `reasoning: string` | Sets the consequence — enumerated-false-fact, enumerated-false-citation, enumerated-false-entity, enumerated-false-relationship, enumerated-false-quote, enumerated-false-data, context-drift, self-reinforcing-enumeration, confident-ungrounded-output, plausible-ungrounded-output, omission-by-ungrounded-output, style-mimicry (12 consequences). All consequences are software-agent-scoped — no intent implied |
| `set-ug-detection` | Mutate | `ug_iri: iri`, `method: string`, `detected_by: iri` | Records how the ungrounded generation was detected — reference checking, source verification, internal consistency analysis, external fact-check, grounding verifier, human review |
| `set-ug-mitigation` | Mutate | `ug_iri: iri`, `mitigation: string`, `applied: bool` | Records mitigation applied or recommended — constrained decoding, retrieval improvement, context window expansion, temperature reduction, alignment adjustment, quantisation upgrade, prompt clarification |
| `set-ug-calibration` | Mutate | `ug_iri: iri`, `calibration: float` | Sets confidence calibration gap — difference between model's expressed confidence and actual grounding quality (0.0 = well-calibrated, 1.0 = maximally miscalibrated) |
| `query-ug-instances` | Query | `filter: string` | Queries ungrounded generation instances by cause, consequence, model, detection method, or calibration |
| `query-ug-causes` | Query | `ug_iri: iri` | Queries all causes for an instance — multiple causes may apply (e.g. training-data-gap + sampling-artefact) |
| `query-ug-consequences` | Query | `ug_iri: iri` | Queries all consequences for an instance — multiple consequences may apply (e.g. enumerated-false-citation + confident-ungrounded-output) |
| `detect-ug-patterns` | Query | `model_id: string` | Detects patterns in a model's ungrounded generations — recurring causes, common consequences, calibration gaps |
| `compare-ug-instances` | Query | `instance_a: iri`, `instance_b: iri` | Compares two ungrounded generation instances — same cause? same consequence? same model? |
| `query-cause-consequence-matrix` | Query | `cause: iri` | Queries known cause-consequence pairings for a given cause — likelihood and recommended mitigation |

### 1.3 Epistemic assessment workflow

```
    ┌──────────────────┐
    │  identify target  │  ← claim, behaviour, expression, document
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  register         │  ← who is assessing? what is their perspective?
    │  perspective      │    what biases? what access level?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  set epistemic    │  ← objective? subjective? intersubjective?
    │  mode             │    contested? ambiguous? fictional?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  classify         │  ← fiction? non-fiction? blended?
    │  reality          │    which sub-category?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  assess           │  ← deliberate? accidental? negligent?
    │  intentionality   │    what type of mistake (if any)?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  assess           │  ← observation? belief? hypothesis?
    │  grounding        │    fiction? emotion? instruction?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  assess           │  ← positive/negative? intensity?
    │  sentiment        │    genuine/performed/manipulated?
    │                   │    multi-dimensional scores
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  set context      │  ← spatio-temporal: where, when, mediation?
    │                   │    social: power, trust, consensus, incentive?
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  compose          │  ← combine all assessments into conclusion
    │  assessment       │    confidence score, contested flag
    └────────┬─────────┘
             │
             ▼
    ┌──────────────────┐
    │  link & export    │  ← link to investigation/research
    │                   │    recursive assessment if disputed
    └──────────────────┘
```

### 1.4 Fiction / non-fiction category reference

| Category | Sub-types | Asserted as real? |
|:---------|:----------|:------------------|
| **Non-fiction** | empirical, testimonial, documentary, analytical, theoretical, procedural, normative, journalistic, administrative | Yes |
| **Fiction** | speculative, historical-fiction, realistic, fantasy, science-fiction, allegorical, satirical, hypothetical, counterfactual, simulated, roleplay, deceptive | No (except deceptive — presented as yes) |
| **Blended** | mythological, legendary, propagandistic, marketing | Ambiguous / agent-relative |

### 1.5 Intentionality × Mistake cross-reference

| Intentionality | Mistake type | Example |
|:---------------|:-------------|:--------|
| deliberate | deliberate-deviation | Agent knowingly breaks a rule |
| intentional | (not a mistake) | Agent achieves intended outcome |
| negligent | system-design, attention-failure | Agent fails to prevent foreseeable harm |
| reckless | judgement-error | Agent disregards known risk |
| accidental | perception-error, attention-failure | Unforeseeable accident |
| mistaken | false-belief, ungrounded-generation | Agent acts on wrong information |
| systematic | system-design | Structural issue causes repeated errors |
| emergent | simulation-artefact | Interaction of agents produces unexpected outcome |
| coerced | (not a mistake) | Agent forced to act |
| habitual | skill-deficit, attention-failure | Routine behaviour produces error |

### 1.6 Directory structure

```
toolboxes/epistemics/
├── mod.rs
├── ontology.n3                  ← imports epistemics.n3
├── ontology.cbor
├── manifest.cbor
├── chains/
│   ├── mod.rs
│   ├── evaluate/
│   │   ├── mod.rs
│   │   └── tools/               ← 12 tools
│   ├── perspective/
│   │   ├── mod.rs
│   │   └── tools/               ← 7 tools
│   ├── intentionality/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   ├── grounding/
│   │   ├── mod.rs
│   │   └── tools/               ← 5 tools
│   ├── fiction/
│   │   ├── mod.rs
│   │   └── tools/               ← 6 tools
│   ├── sentiment/
│   │   ├── mod.rs
│   │   └── tools/               ← 10 tools
│   └── ungrounded_generation/
│       ├── mod.rs
│       └── tools/               ← 12 tools
```

### 1.7 Capability scope

- `graph:read`, `graph:mutate` — reading/writing epistemic assessment graph
- `provenance:read`, `provenance:write` — provenance for assessments
- `agency:read` — reading agent profiles, perspectives, and biases
- `nlp:analyze` — running NLP for sentiment extraction, fiction detection, irony/sarcasm detection
- `neural:infer` — running neural models for sentiment classification, intentionality inference (optional)
- `epistemics:read`, `epistemics:mutate` — epistemics-specific operations

---

## 2. Epistemics manifold seed

| Container | Dock | Notes |
|:----------|:-----|:------|
| `reality-board` | centre | Primary workspace |
| `perspective-view` | left | Agent perspectives |
| `sentiment-map` | right | Sentiment visualisation |
| `intentionality-panel` | bottom (panel) | Intentionality & mistakes |
| `grounding-panel` | bottom (panel) | Behaviour grounding |
| `fiction-classifier` | right (panel) | Fiction/non-fiction classification |
| `social-context-panel` | left (panel) | Social relation context |

---

## 3. Relationship to existing toolboxes

| Aspect | Epistemics | Investigation | Research |
|:-------|:-----------|:--------------|:---------|
| Focus | Is this real? Is this subjective? Is this fiction? | What happened? Who did it? | What do we know? What does it mean? |
| Agent role | Any agent can assess or be assessed | Agents are investigators, witnesses, subjects | Agents are researchers, contributors |
| Intentionality | Classifies deliberate vs accidental | Evaluates evidence for hypotheses | Not directly addressed |
| Fiction | Classifies fiction vs non-fiction | Not directly addressed | May classify corpus items |
| Sentiment | Full sentiment analysis | Not directly addressed | Not directly addressed |
| Social context | Power, trust, consensus, incentive | Constituency, access levels | Not directly addressed |
| Spatio-temporal | Mediation type, resolution | Timeline, spatial scope | Spatio-temporal dynamics |
| Output | Epistemic assessment | Case conclusion | Findings, synthesis |

Epistemic assessments can be linked to investigation hypotheses (`link-to-investigation`) and research findings (`link-to-research`). An epistemic assessment may determine that an investigation hypothesis is based on subjective interpretation rather than objective evidence, or that a research finding relies on a fictional premise.

---

## 4. Recursive epistemics

Assessments can themselves be assessed. When two agents disagree on the epistemic status of a claim:

1. Agent A assesses the claim as **objective**
2. Agent B assesses the claim as **subjective**
3. Agent C creates a **recursive assessment** — assessing A's and B's assessments
4. Agent C may determine that A's assessment is biased (confirmation bias) or that B's assessment is motivated (adversarial relation)
5. The recursive assessment is itself assessable by other agents

This prevents epistemic gridlock — disagreements can be adjudicated, and the adjudication can itself be evaluated.

---

## 5. Loose coupling

| Tool | `reality-board` | `graph` | `doc` | `case-board` | `research-board` |
|:-----|:-----------------|:--------|:------|:-------------|:-----------------|
| `create-assessment` | ✅ primary | ✅ | ✅ assess document | ✅ assess evidence | ✅ assess finding |
| `assess-intentionality` | ✅ | ✅ | — | ✅ assess action | — |
| `classify-reality` | ✅ | — | ✅ classify document | ✅ classify evidence | ✅ classify corpus item |
| `assess-sentiment` | ✅ | ✅ | ✅ | — | ✅ |
| `register-perspective` | ✅ | ✅ | — | ✅ | ✅ |

---

## 6. Relationship to existing specs

| Document | Relationship |
|:---------|:-------------|
| [`TOOL_CHEST_SPEC.md`](TOOL_CHEST_SPEC.md) | Parent spec — hierarchy, core traits, ontology layer |
| [`qualia-ui/ontologies/epistemics.n3`](../ontologies/epistemics.n3) | Epistemics ontology — epistemic status, reality classification, intentionality, grounding, sentiment, perspective, context |
| [`qualia-ui/ontologies/agent-nomenclature.n3`](../ontologies/agent-nomenclature.n3) | Agent nomenclature ontology — agent-type classification, applicableAgentType scoping, nomenclature mapping, appendable record support |
| [`qualia-ui/ontologies/ungrounded-generation.n3`](../ontologies/ungrounded-generation.n3) | Ungrounded generation ontology — 18 causes, 12 consequences (software-agent-scoped), cause-consequence matrix |
| [`qualia-db-standards/agent-nomenclature-rules.md`](../../qualia-db-standards/agent-nomenclature-rules.md) | Project rule — agent nomenclature isolation table (natural vs legal vs software) |
| [`qualia-ui/ontologies/container.n3`](../ontologies/container.n3) | Container ontology — now includes epistemics containers |
| [`qualia-ui/ontologies/agency.n3`](../ontologies/agency.n3) | Agency ontology — agents, actors, claims, delegation |
| [`qualia-ui/ontologies/provenance.n3`](../ontologies/provenance.n3) | Provenance — assessment provenance chains |
| [`qualia-ui/ontologies/investigation.n3`](../ontologies/investigation.n3) | Investigation ontology — hypotheses, evidence, links |
| [`qualia-ui/ontologies/research.n3`](../ontologies/research.n3) | Research ontology — findings, inferences, corpus items |
| [`TOOLBOX_INVESTIGATION_SPEC.md`](TOOLBOX_INVESTIGATION_SPEC.md) | Investigation toolboxes — hypothesis graphs, evidence evaluation |
| [`TOOLBOX_RESEARCH_SPEC.md`](TOOLBOX_RESEARCH_SPEC.md) | Research toolbox — findings, inferences, dark links |
| [`TOOLBOX_CODE_SPEC.md`](TOOLBOX_CODE_SPEC.md) | AI toolbox — NLP for sentiment, fiction detection, irony |
| [`qualia-db-standards/poet-ui-concepts.md`](../../qualia-db-standards/poet-ui-concepts.md) | UI concepts — manifolds, containers, presentation |
