# Prompt precision — implementation-agent brief

Date: 2026-09-15 · Swarm prep: 2026-09-16

Status: approved design; managed swarm board live; Wave 0 authorised; runtime implementation not started

**Swarm board:** [`docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md`](../../work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md) — package tracker, lane briefs, independent-DONE rule. Use that document for allocation; this brief remains the normative technical contract.

Primary objective: implement one portable, testable conditioning contract for Qualia's native, MCP and API inference paths, with VibeScript as its authoring/orchestration language and Aura/SHACL as its semantic validation layer.

This brief is intended to be handed directly to an implementation agent. Read [AGENTS.md](../../../AGENTS.md), [CLAUDE.md](../../../CLAUDE.md), the [architecture](README.md), and the [repository evidence map](CODEBASE-MAP.md) before editing. Work in the canonical checkout, read and update `coordination/NOTICES.md`, preserve unrelated working-tree changes, and record every phase in the required progress log.

## 1. Outcome and completion boundary

Deliver a conditioning pipeline in which:

1. A person or agent can define an inference profile in ordinary VibeScript records and modules.
2. The profile has a canonical semantic projection using the VibeScript and Conditioning ontologies.
3. Aura/SHACL validates structure and reports precise violations.
4. The core compiler resolves requirements, authority, evidence, backend capabilities and budgets into an immutable prepared plan.
5. Native, MCP and Qualia HTTP entry points lower the same plan without losing instruction boundaries.
6. Every requirement is traceable from its source through compilation, rendering, validation and outcome.
7. Evaluation compares current prompting, explicit text, graph-compiled text and later learned conditioning on held-out tasks.

Portable completion means phases P0–P6 pass for every declared supported product route. Learned prompts, layer prefixes, graph encoders and LoRA experiments are P7 and remain experimental until independently measured.

Do not claim the programme complete when only a Vibe module, ontology file, native backend or demo path works.

## 2. Architectural decision

### 2.1 Ownership

| Concern | Owner |
|---|---|
| Human and agent authoring, composition, campaign definitions | VibeScript |
| Language metamodel and profile vocabulary | Vibe and Conditioning RDFS vocabularies |
| Structural validation and violation reports | Aura using SHACL |
| Compact descriptive/interchange shapes when a concrete need exists | Derived ShExC; never a second hand-maintained source |
| Requirement conflict resolution, evidence selection, budgets and canonical identities | `qualia-core-db::inference::conditioning` |
| Roster, chat/session and ontology-routing adaptation | `qualia-client-core::conditioning` |
| Model-specific messages, tokenisation and transport arguments | Backend lowering adapters |
| KV ownership, learned prefixes, adapters and decode execution | Existing native inference runtime |
| Permission, disclosure, tool and graph-write enforcement | Existing Webizen/host gates |
| Evaluation, promotion evidence and rollback | Conditioning evaluation/registry modules |

VibeScript is not the enforcement boundary. A Vibe profile can request a capability but cannot grant it. Ontology tags can narrow relevance but cannot widen graph, attachment, tool or network authority.

### 2.2 Keep four representations distinct

```text
.vibe source
    ├─> typed Vibe AST ──> Tag-4200 CBOR syntax representation
    └─> semantic projection ──> Vibe + Conditioning graph ──> SHACL validation
                                      │
                                      v
                              core compiled plan
                                      │
                     ┌────────────────┼────────────────┐
                     v                v                v
                native request    MCP request      API request
```

- The AST and Tag-4200 encoding preserve program structure.
- The ontology graph describes meaning and relationships.
- The compiled plan is the bounded executable contract for one request.
- Rendered messages/tokens/tool arguments are backend-specific products.

The present Tag-4200 implementation is custom tagged CBOR using textual node/field keys. Do not call it linked data merely because it serialises an AST. Either retain the honest name “Tag-4200 canonical CBOR AST” or add a separate, tested semantic projection with stable IRIs. Do not break existing Tag-4200 fixtures while adding that projection.

## 3. Hard constraints

- Preserve the 48-byte `NQuin` ABI, canonical object tags, parity rules, sensitivity bits and `q_hash` conventions. Do not allocate new modality opcodes for this work.
- Tier-1 selection, validation, token masking and decode paths must allocate zero heap memory. Use caller-owned slices and bounded fixed storage.
- Cold Vibe authoring, graph projection and transport serialisation may allocate within explicit budgets. Public core compilation output remains caller-buffered.
- The Sentinel pass remains within 42 × 1024 × 1024 bytes. Model weights, KV residency and offline training are accounted separately and must not be described as fitting inside the Sentinel arena.
- Keep request policy, grammar, budgets and adapters request-scoped. Do not introduce process-global mutable conditioning state.
- Prompt or latent conditioning is probabilistic guidance. Permissions, disclosure, tool calls, output syntax and graph writes remain executable checks.
- Preserve instruction, task, evidence, history and tool-schema roles until backend lowering. A delimiter inside one string is not a security boundary.
- Reject unsupported required features. An optional feature may degrade only through an explicit fallback recorded in the receipt.
- Unknown usage is not zero usage. Follow the existing receipt coverage-bit pattern.
- Do not add an external Python, Ollama or optimisation-framework dependency. The existing opt-in Ollama harness is a compatibility route and keeps its current default behaviour.
- Do not add catalogue IDs before their host implementations exist. Any new `Capability.method` ID must land in the real host dispatch, `ALL_BOUND`, the Vibe catalogue and machine schema in the same phase with lockstep tests.
- Follow ADR 0009: SHACL enforces; ShEx describes only where recursion, publication or interchange requires it. When both exist, generate them from one source.
- No remote inference, paid API call, model download, training campaign, automation, deployment, commit or push is authorised by this brief alone.

## 4. Target module layout

Create focused directory-backed libraries. Keep implementation files below 500 lines unless a cohesive generated registry justifies an explicitly recorded exception.

```text
crates/vibe/src/metamodel/
  mod.rs              public descriptors and semantic projection API
  namespace.rs        stable Vibe IRIs and language-version identity
  node_kinds.rs       AST node-kind registry used by codec/projection tests
  fields.rs           field/property descriptors
  project.rs          typed AST -> semantic graph projection
  export.rs           deterministic RDFS/SHACL artifact writer
  validate.rs         registry/AST/codec coverage checks

crates/vibe/src/conditioning/
  mod.rs              Vibe-facing constructors and conversion API
  profile.rs          validated Vibe Value/record -> profile DTO
  project.rs          profile DTO -> Conditioning semantic graph
  tests.rs            parse/check/project fixtures; no inference runtime

crates/vibe/schema/
  vibescript-core.ttl             generated, committed RDFS vocabulary
  vibescript-core.shacl.ttl       generated, committed enforcement shapes
  conditioning-v1.ttl             generated, committed RDFS vocabulary
  conditioning-v1.shacl.ttl       generated, committed enforcement shapes

crates/qualia-core-db/src/inference/conditioning/
  mod.rs              public API and re-exports
  spec.rs             borrowed bounded input contract
  requirement.rs      requirement classes, priority and resolution
  capabilities.rs     endpoint/model capability evidence
  authority.rs        read/disclosure/tool/write view
  validate.rs         bounds, references and conflict validation
  select.rs           deterministic evidence selection/dependency closure
  budget.rs           tokens, bytes, tool rounds, time and cost
  compile.rs          immutable plan assembly
  render.rs           shared text/message parts
  identity.rs         canonical content/cache binding
  receipt.rs          applied/degraded/rejected requirement outcomes
  codec/
    mod.rs            versioned Conditioning CBOR profile
    encode.rs
    decode.rs
  tests/
    mod.rs
    conformance.rs
    authority.rs
    capacity.rs
    codec.rs
    allocation.rs

crates/qualia-client-core/src/conditioning/
  mod.rs              client integration facade
  roster.rs           legacy `system_prompt` import and profile references
  context.rs          session/retrieval/ontology adapters
  mcp.rs              tool schema/capability-aware lowering
  result.rs           validation/persistence/measurement status normalisation

crates/qualia-core-db/src/inference/conditioning_eval/
  mod.rs              cold, bounded evaluation API
  corpus.rs           versioned task manifests and split enforcement
  scoring.rs          independent validators and task metrics
  search.rs           finite candidate search and stopping
  report.rs           paired comparisons and evidence artifacts
```

Backend integration should be added through small modules beside existing owners rather than adding more responsibilities to large files such as `inference_agent/decode.rs` or `chat_inference.rs`. If touching a file at or above the repository thresholds, record and perform the appropriate decomposition in the same programme.

Dependency direction is fixed: `qualia-core-db` already depends on `vibe`, so `vibe` must not depend on `qualia-core-db`. Vibe metamodel/profile projection therefore emits neutral semantic statements, a caller-provided writer, or Vibe values. The core/host layer resolves those terms into Quins and seals parity. Do not introduce a cyclic crate dependency or duplicate `NQuin` inside Vibe.

## 5. Ontology contract

Use the existing namespace convention:

```text
vibe:  https://qualiadb.org/schema/vibe#
cond:  https://qualiadb.org/schema/conditioning#
q42:   https://qualiadb.org/schema/
```

Confirm these against the live lexicon before committing generated terms. If the existing lexicon pins a different canonical expansion, use it consistently and update this plan/progress log with the evidence. Never create two IRIs for the same term.

### 5.1 VibeScript metamodel

Minimum RDFS classes:

```text
vibe:LanguageVersion, vibe:Program, vibe:Module, vibe:AstNode,
vibe:Declaration, vibe:FunctionDeclaration, vibe:HookDeclaration,
vibe:CellDeclaration, vibe:FieldDeclaration, vibe:MaterialDeclaration,
vibe:LawDeclaration, vibe:Statement, vibe:Expression, vibe:Pattern,
vibe:Type, vibe:EffectClass, vibe:Capability, vibe:InvokeId,
vibe:CapabilityArgument, vibe:Budget, vibe:SourceSpan, vibe:Diagnostic,
vibe:Receipt
```

Minimum properties:

```text
vibe:languageVersion, vibe:nodeKind, vibe:contains, vibe:declares,
vibe:hasExpression, vibe:hasStatement, vibe:hasType, vibe:hasEffect,
vibe:requiresCapability, vibe:invokeId, vibe:hasArgument,
vibe:hasBudget, vibe:sourceStart, vibe:sourceEnd, vibe:diagnosticCode
```

Requirements:

- Every concrete `Item`, `Stmt`, `ExprKind`, `Pattern`, `ModalKind` and `Type` variant has one stable descriptor or an explicitly documented non-semantic representation.
- Language/version identity is explicit; Tag 4200 alone is not the language version.
- Source spans are provenance annotations and must not be confused with semantic identity.
- `vibe:InvokeId` instances are generated only from the live catalogue/host intersection. Availability, effect, honesty label, arguments, return type and host profile are properties of those instances.
- Do not duplicate the present Vibe and host capability schema tables again. Select one canonical descriptor source or generate a reconciled export with tests proving exact coverage and reporting disagreements.
- AST registry descriptors should be consumed by the semantic projection and tested against the Tag-4200 codec. A wholesale parser rewrite is outside the initial slice.

### 5.2 Conditioning ontology

Minimum classes:

```text
cond:ConditioningProfile, cond:Objective, cond:DomainScope,
cond:Requirement, cond:EnforcedRequirement, cond:EvidenceObligation,
cond:Guidance, cond:MethodConstraint, cond:EvidencePolicy,
cond:OutputContract, cond:ResourceBudget, cond:BackendCapability,
cond:CompiledPlan, cond:RequirementOutcome, cond:CompilationReceipt,
cond:EvaluationSuite, cond:EvaluationRun, cond:CandidateProfile,
cond:PromotionDecision
```

Minimum properties:

```text
cond:schemaVersion, cond:profileId, cond:contentDigest,
cond:hasObjective, cond:hasDomainScope, cond:hasRequirement,
cond:authoritySource, cond:priority, cond:isRequired, cond:validatorRef,
cond:hasEvidencePolicy, cond:allowedGraphScope, cond:disclosureCeiling,
cond:hasOutputContract, cond:hasBudget, cond:targetsBackend,
cond:targetsModel, cond:usesTokenizer, cond:usesTemplate,
cond:selectedEvidence, cond:compiledFrom, cond:expressedBy,
cond:outcomeStatus, cond:fallbackReason, cond:measuredBy,
cond:evaluatedBy, cond:promotes, cond:replaces
```

Required relationship rules:

- `cond:expressedBy` may point to a `vibe:Program`; a profile remains usable through other authoring clients.
- Each requirement has a stable ID, exactly one requirement class, an authority source, required/optional status and priority.
- An `EnforcedRequirement` requires a validator reference that resolves to an executable validator.
- A `CompiledPlan` identifies its profile digest, model, tokenizer, template, ordered evidence, capability snapshot and authority view.
- A receipt has one outcome for every input requirement: `applied`, `enforced`, `degraded`, `rejected` or `not-applicable` with a reason.
- A promotion decision links immutable evaluation evidence. Activation changes a registry pointer; it does not rewrite prior artifacts.

### 5.3 SHACL shapes

SHACL is authoritative for graph validation. At minimum test:

- required fields and `maxCount 1` for identity/version/digest fields;
- one valid requirement subtype per requirement;
- bounded requirement/domain/evidence lists;
- non-empty stable IDs and supported schema versions;
- validator requirement for enforced constraints;
- numeric budget ranges and checked cross-field constraints;
- explicit disclosure ceiling and graph-scope representation;
- complete requirement-outcome coverage on receipts;
- model/tokenizer/template identity on cacheable compiled plans;
- immutable evaluation and promotion references.

The existing SHACL engine may not express every cross-record invariant. Enforce what it supports in SHACL and perform the remainder in the core validator, returning the same stable requirement/path identifiers in both violation formats. Do not describe a Rust-only check as SHACL validation.

Do not create a hand-maintained ShEx copy in P1. Add a derived ShExC export only if a concrete recursive or publication requirement enters scope, following ADR 0009.

## 6. Core data and API contract

The following names are target APIs. Adjust only when current code makes a materially better shape; record deviations in the progress log and keep the semantic contract.

```rust
pub enum RequirementClass {
    Enforced,
    EvidenceObligation,
    Guidance,
}

pub enum SupportLevel {
    Supported,
    Unsupported,
    Unknown,
}

pub enum RequirementDisposition {
    Applied,
    Enforced,
    Degraded,
    Rejected,
    NotApplicable,
}

pub struct ConditioningSpec<'a> {
    pub schema_version: u16,
    pub profile_id: &'a str,
    pub objective: &'a str,
    pub requirements: &'a [RequirementRef<'a>],
    pub domain_refs: &'a [u64],
    pub output_contract: OutputContractRef<'a>,
    pub budget: ConditioningBudget,
}

pub fn compile_into(
    spec: &ConditioningSpec<'_>,
    evidence: &[EvidencePart<'_>],
    capabilities: &BackendCapabilities,
    authority: &AuthorityView<'_>,
    buffers: &mut CompileBuffers<'_>,
) -> Result<CompiledPlanSummary, ConditioningError>;

pub fn render_into(
    plan: &PreparedConditioning<'_>,
    target: &RenderTarget,
    output: &mut [u8],
) -> Result<RenderedRequestSummary, ConditioningError>;
```

`CompileBuffers` owns the storage for requirement outcomes, selected Quin/source references, message parts, rendered bytes and token IDs. Summaries contain lengths and canonical identities, never references to temporary local arrays. Capacity failure produces no dispatchable plan.

Errors include `UnsupportedVersion`, `InvalidReference`, `DuplicateRequirement`, `ConflictingRequirements`, `DisclosureDenied`, `RequiredCapabilityMissing`, `ContextBudgetExceeded`, `OutputBufferFull`, `IncompatibleArtifact`, `DeadlineExceeded` and `ValidationFailed`. Diagnostics identify the requirement/path and a concrete remedy.

`BackendCapabilities` records supported, unsupported or unknown for:

- system/developer/user role separation;
- media/message content types;
- tools and structured tool arguments;
- output grammar/schema;
- exact token counting;
- exact-prefix KV reuse;
- learned input prompts, layer prefixes and adapter targets;
- streaming, cancellation and usage/cost reporting.

Capabilities are endpoint/model/version scoped and carry evidence of how they were established. A configuration label supplied by a user is an expectation, not proof of support.

## 7. VibeScript authoring surface

Do not add new grammar in the first implementation. Use current modules, records, bounded lists, functions, effects, budgets and capability leases. Add first-class syntax only after the record form has real usage and a versioned grammar decision.

The target source form is conceptually:

```vibe
module <urn:qualia:conditioning:rust-review>;

using Conditioning;

requires [
    capability("Conditioning.compile"),
    capability("Inference.run_transformer")
];

const profile = {
    schema_version: 1,
    profile_id: "urn:qualia:profile:rust-systems-review:v1",
    objective: "Review a Rust change for correctness and bounded resource use",
    domains: ["urn:qualia:domain:rust", "urn:qualia:domain:systems"],
    requirements: [
        {
            id: "R1",
            class: "enforced",
            rule: "No allocation in declared Tier-1 paths",
            validator: "zero-alloc-suite",
            required: true,
            priority: 255
        }
    ],
    budget: {
        input_tokens: 4096,
        output_tokens: 1024,
        tool_rounds: 4
    }
};
```

This is a target fixture, not currently valid evidence that the named host calls exist. Before adding `Conditioning.*`:

1. Implement the underlying core operation.
2. Bind it in `poet_host::invoke` with real argument/result conversion and gating.
3. Add the exact ID to `ALL_BOUND` and `vibe::catalog::ALL_INVOKE_IDS`.
4. Add one machine schema generated from or reconciled with the bound descriptor.
5. Add lockstep, lease, effect, unsupported-host and end-to-end tests.

Recommended real host surface, introduced only when each operation works:

| Candidate ID | Effect | Purpose |
|---|---|---|
| `Conditioning.validate` | Pure or Read, depending on shape resolution | Validate one profile and return structured violations. |
| `Conditioning.compile` | Cold/Read | Compile against an explicit backend capability and authority snapshot. |
| `Conditioning.inspect` | Read | Return a redacted requirement/token/evidence trace. |
| `Conditioning.evaluate` | External/Async | Start a bounded evaluation campaign. |
| `Conditioning.activate` | Write/External | Move the active registry pointer after approval gates. |
| `Conditioning.rollback` | Write/External | Restore a prior approved version. |

Never expose raw learned tensors, unrestricted prompts, private evidence or cache pages as ordinary Vibe `Value` data. Use opaque refs and receipts.

## 8. Compilation algorithm

1. **Normalise:** validate versions, bounded UTF-8 lengths, IDs, references and duplicate policy. Attach source kind and authority reference to every part.
2. **Project/validate:** create or accept the canonical semantic projection; run SHACL/Aura and core invariants. Preserve the original authoring artifact and digest.
3. **Resolve:** expand only configured, version-pinned dictionaries and locally authorised references.
4. **Resolve conflicts:** executable policy constrains all requests; explicit current-task requirements override conflicting optional profile defaults. Incompatible required task constraints produce a diagnostic and no plan.
5. **Select evidence:** filter by authority before ranking; then perform deterministic relevance ranking and dependency closure. Preserve definitions, units, provenance, temporal qualifiers and contradictions.
6. **Plan capabilities:** compare required features with the actual endpoint/model capability snapshot. Reject missing required capabilities; record optional degradation.
7. **Budget:** count the target template/tokens where available. Include instructions, current task, evidence, history, tool schemas, media accounting and output reserve. Estimate only when exact accounting is impossible and label it.
8. **Render:** maintain typed message parts and stable order. Render text only at the final adapter boundary.
9. **Prepare:** bind immutable caller-owned buffers, validators and compatible cache/adapter handles. Cold allocation remains bounded.
10. **Execute/validate:** preserve intent gates, Sentinel/control mechanisms, output checks and tool allowlists. Recheck every tool action against the original authority and remaining request budget.
11. **Receipt:** record all requirement outcomes, capability omissions, evidence selections, exact/estimated counts, validation, cache/adaptation identity, route and measured cost coverage.

Compilation is deterministic for identical pinned inputs. Generation, provider scheduling and network execution are not promised deterministic.

## 9. Backend lowering

| Route | Required implementation | Required tests |
|---|---|---|
| Native GGUF/P64 | Preserve roles until `GgufTokenizer` template rendering; tokenise once; preflight memory/context; pass prepared request-scoped state into runtime. | Roster system prompt survives; exact token IDs and BOS/EOS; unknown template diagnostic; special-token injection; stream/cancel parity. |
| Remote MCP inference tool | Inspect configured/discovered input schema; use structured fields when supported; otherwise render a labelled text envelope. | Role-flattening receipt; required-role rejection; consent/disclosure before serialisation; remote partial/error parsing. |
| MCP sampling | Separate negotiated adapter using sampling messages and optional system prompt. | Capability negotiation; host-controlled model choice recorded; no assumption that sampling equals a tool call. |
| Qualia HTTP sync and jobs | Add optional conditioning profile/reference while preserving legacy prompt requests as compatibility specs. Compile in core before inference. | Sync/job semantic parity; cancellation; bounds; result status and measurement coverage. |
| Existing opt-in harness | Route through the same compiler while preserving selected behaviour and defaults. | No implicit activation; capability limitations and estimated counts recorded. |
| Future provider API | Implement only for a concrete authorised provider and verified schema. | Provider message/tool/schema fixtures and redacted receipts; no inferred latent-input support. |

Normalise result state across routes. At minimum distinguish inference success, output validation, persistence/commit, provenance verification, token/cost measurement coverage and partial streaming. A successful remote text response is not automatically a verified graph commit.

## 10. Cache and learned-conditioning identity

Exact-prefix reuse must include:

- model/weight revision;
- tokenizer and chat-template digest;
- ordered rendered token IDs;
- positional encoding and attention-layout compatibility;
- backend/KV representation;
- adapter or soft-prefix digest and target layout;
- authority/disclosure scope and relevant graph revision.

Do not attach pages by domain similarity or graph similarity. Exact reusable token prefix and compatible model state are required. Reuse existing reference-counted page ownership and request scheduler semantics. Treat the legacy global hidden-state map and paged KV store as separate mechanisms until an audit proves otherwise.

P7 progression:

1. best explicit text baseline;
2. learned input prompt on one pinned local model;
3. layer prefix with explicit per-layer K/V layout;
4. one target-specific LoRA adapter;
5. adapter composition and graph encoder only after interference/fidelity tests.

Before P7, remove observed embedding-copy and rank-scratch allocations, validate input/output shapes, stop silently skipping incompatible adapters, and prove every selected runtime path actually consumes the artifact. Keep dynamic facts and executable authority outside learned state.

## 11. Phased work programme

Each phase ends with tests, a progress-log entry and a `PROGRESS`/`RELEASE` notice before the next phase begins. Do not batch the evidence record until the end.

| Phase | Deliverable | Dependencies | Completion evidence |
|---|---|---|---|
| P0 | Entry-point inventory and golden current-behaviour fixtures for named local chat, MCP, HTTP sync/jobs, CLI and configured harness. | None | Fixtures capture role boundaries, exact/estimated counts and result semantics without paid inference. |
| P1A | Vibe metamodel registry and deterministic RDFS/SHACL exporter. | P0 | All declared AST/type/effect/capability categories covered; generated artifacts stable; Tag-4200 round trips unchanged. |
| P1B | Conditioning ontology, SHACL shapes, profile DTO and Vibe record-to-graph projection. | P1A | Valid/invalid fixtures, canonical graph/digest, bounded lists, no invented invoke IDs. |
| P1C | Core spec, strict CBOR codec, validators, compiler buffers, conflict resolution, identities and receipts. | P1B vocabulary freeze | Deterministic round trips; malformed/unknown/duplicate/overflow cases; every requirement accounted for. |
| P2 | Native chat and HTTP sync/job integration with request-scoped state. | P1C | Correct local roster system prompt, typed roles to template, tokenise once, stream/cancel parity, existing gates preserved. |
| P3 | MCP lowering and normalised remote result semantics. | P1C | Schema fixtures, role-degradation handling, consent/disclosure gates, unknown usage preserved, persistence separated from response. |
| P4 | Authorised graph selection, dependency closure and compatible exact-prefix reuse. | P2/P3 | Source/unit/qualifier preservation, budget admission, complete invalidation matrix, eviction/concurrency tests. |
| P5 | Held-out evaluation harness and finite text-profile search. | P2/P3; P4 for cache comparisons | Split isolation, reproducible campaign receipts, task-quality/resource comparisons with uncertainty, strict caps. |
| P6 | Vibe host bindings, profile registry, inspector, promotion and rollback. | P5 | Real `Conditioning.*` IDs in host/catalog/schema lockstep; lease/effect tests; reviewable activation and rollback. |
| P7 | Learned prompts/prefixes/adapters for one pinned local model. | P5/P6 plus readiness gates | Shape compatibility, zero-allocation application, held-out gain over best text baseline, complete memory/cost evidence and base fallback. |

P2 and P3 may run independently after P1C if lane allocation permits. P1A–P1C are sequential because the vocabulary and validation contract must stabilise before backend integration.

## 12. First implementation slice

The first agent should complete P0 and P1A only, unless explicitly allocated more. This establishes truth about current routes and the language metamodel without prematurely altering inference behaviour.

### P0 tasks

1. Enumerate every product and diagnostic inference call site with `rg`; classify raw benchmark paths separately.
2. Add non-network fixtures capturing input roles/parts immediately before native tokenisation, MCP serialisation and HTTP agent invocation.
3. Reproduce and test the inspected named-local roster system-prompt omission.
4. Reproduce and test MCP system/user flattening and remote result status/unknown usage semantics.
5. Record current template family fallback behaviour and whether each prefix/KV mechanism is reached by product chat.
6. Write the inventory and test evidence into `CODEBASE-MAP.md` and the progress log; do not “fix along the way” until the fixture protects the behaviour unless the fix is required to make observation possible.

### P1A tasks

1. Add `vibe::metamodel` as a directory-backed library and export it through `lib.rs`.
2. Define namespace/version constants and stable node/effect/type/capability descriptors.
3. Export deterministic RDFS plus SHACL through an `io::Write` or caller-buffered API; generation is cold and byte-budgeted.
4. Generate the committed `vibescript-core.ttl` and `vibescript-core.shacl.ttl` artifacts through the exporter; do not hand-edit generated output.
5. Add coverage tests against AST categories, language/effect/type registries and the live Vibe/host capability intersection.
6. Add semantic-projection tests for representative Program, Function, Requirement/CapSpec, GraphQuery, ModalLogic, Field, Material and Law nodes.
7. Prove existing Tag-4200 encode/decode fixture bytes and round trips remain unchanged unless a separately approved codec version is introduced.

### P0/P1A stop conditions

Stop and report a concrete blocker if:

- the canonical namespace conflicts with a pinned lexicon;
- another live claim covers a target file;
- complete capability reconciliation requires modifying a concurrently owned catalogue;
- a required generated artifact cannot be reproduced deterministically;
- preserving existing Tag-4200 compatibility is impossible without a codec version decision.

Do not proceed to P1B by silently choosing a conflicting namespace or changing the wire format.

## 13. Evaluation protocol for P5+

Initial domain: software development. Use bounded repository fixtures with a user task, permitted source files, expected properties and independent validators.

Task families:

1. Bug diagnosis with exact code location and concrete trigger.
2. Small changes with held-out behavioural/boundary tests.
3. Requirement extraction from documents that contain quoted or embedded instructions.
4. Structured-output and mock tool-selection tasks with permission boundaries.
5. Tier-1 allocation/capacity/ABI review that distinguishes permitted cold construction.
6. Missing/conflicting evidence cases that require an explicit uncertainty or conflict result.

Maintain search/train, development/selection and sealed test manifests, split by underlying task/repository family. Candidate generation and optimisation cannot see sealed labels. Retire a sealed test after it influences selection.

Required baselines:

| Variant | Isolates |
|---|---|
| B0 current route | Current product behaviour. |
| B1 explicit readable requirements | Precision without graph syntax. |
| B2 ontology graph compiled to readable text | Validation, conflict handling, evidence and traceability. |
| B3 compact graph/text representation | Actual token saving versus comprehension loss. |
| B4 B2 plus exact-prefix reuse | Cache/prefill effect independent of semantics. |
| B5 optimised text profile | Finite measured candidate search. |
| B6 each learned artifact separately | Incremental value over the best validated text baseline. |

Hold model revision, template, retrieval snapshot, tools and decoding settings constant for conditioning comparisons. Compare full allowed context and equal-token-budget conditions separately.

Primary metrics are held-out task success and required-constraint satisfaction. Also report evidence support, unsupported claims, valid/authorised tool use, omitted requirements, schema validity, cold/warm TTFT, total latency, actual versus estimated input/output tokens, cache hits, peak memory and billable cost when observable. Native KL/perplexity may measure fidelity for learned/quantised changes but does not establish truth or task quality.

Use paired cases, repeat stochastic outputs and report uncertainty and failure counts by task family. Persist campaign caps for requests, calls, tokens, money, time and storage. Describe the winner as the best measured candidate within that campaign.

## 14. Required cross-cutting tests

- Same profile requirements survive Vibe projection, core compile, native, HTTP and MCP fixtures.
- Evidence containing role-like text remains evidence and cannot manufacture a trusted instruction role.
- Unknown required fields, duplicate IDs, inconsistent dictionary versions, malformed CBOR and over-depth graphs fail closed.
- Long evidence/history/tool schemas cannot silently evict required instructions; the compiler returns a specific budget diagnostic.
- IRI/literal/unit/temporal/provenance meaning survives source, graph and rendered projections.
- Permission rejection happens before retrieval disclosure or tool serialisation. Profile defaults cannot widen authority.
- Remote response, output validation, persistence, provenance verification and measurement coverage remain distinct.
- Cache invalidates on every semantic/layout/authority change and never crosses disclosure scope.
- Concurrent requests cannot mutate one another's profile, grammar, budget, adapter or cache attachment.
- Streaming is provisional until validation. Failed validation cannot produce final success or graph mutation.
- Repair attempts share the original deadline/cost budget.
- Success, error, cancellation and unwind reclaim temporary artifacts and page references.
- New Tier-1 paths pass the real zero-allocation harness under parallel tests.
- Existing Vibe conformance, native Sentinel/control, graph mutation and supported WASM profile tests remain green.

## 15. Verification commands

Run the smallest relevant commands after each edit, then the phase gate. Use `--offline` when dependencies are already present; report unavailable dependencies honestly rather than changing scope.

```powershell
cargo test -p vibe --lib --offline
cargo test -p vibe --test conformance --offline
cargo test -p vibe --test vibe_catalog_surface --offline
cargo test -p qualia-core-db --lib --offline -- conditioning
cargo test -p qualia-core-db --lib --offline -- zero_heap
cargo test -p qualia-client-core --lib --offline -- conditioning
cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-ontology --offline
```

Before declaring a phase complete, run the repository-required broader suites appropriate to the touched crates. Do not run an expensive full suite repeatedly after an unchanged green result. Documentation checks include local links, balanced fences, generated-artifact cleanliness and `git diff --check` scoped to owned files.

## 16. Progress and handoff record

After every phase, append:

- status: done, partial or blocked;
- exact files and mechanisms changed;
- measured results and what they do not prove;
- where human input is required, or “none this phase”;
- next phase and newly discovered prerequisites;
- active/released file claims and unrelated concurrent changes preserved.

Do not claim generated ontology, host capability, cache reuse, zero allocation, provider role support or quality improvement without the corresponding test or measurement.

## 17. Decisions reserved for later

P0–P4 require no new product decision if the live lexicon resolves the proposed namespaces. Before P5/P6 production promotion, obtain the selected models/endpoints, representative task corpus, unacceptable failure classes and quality/cost/latency thresholds. Any remote evaluation also requires an explicit monetary/call cap and existing disclosure consent.

P7 requires a separately identified local model, compatible training/import method and artifact budget. Do not treat this design as authorisation to train, download or call a model.
