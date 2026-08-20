# Multi-Agent Orchestration Strategy

**Date:** 2026-08-19
**Status:** Strategy + requirements analysis (awaiting Timothy's review)
**Principal:** Timothy Charles Holborn
**Repository:** `C:\Projects\qualia-27062026` (no worktrees, no sibling repos)

---

## 0. Purpose

This document defines the multi-agent orchestration strategy for QualiaDB /
VibeScript, analyzes the requirements to fully implement it (including
refactors), and establishes priority and dependency ordering.

It synthesizes three layers of existing work:

1. **VibeScript agent primitives** (A1–A9 in `poet-vibe`): CBOR-LD AST codec,
   DOMINO constrained decoding, 3-stage reflection, AST query engine, semantic
   blackboard, multi-agent DAGs, paraconsistent Eτ evidential logic, hardware
   deontic interrupts, semantic skills.
2. **Engine-level coordination** (`governance/coordination.rs`): The
   `MULTI_AGENT_PROTOCOL.md` opcodes (0x70–0x72), resource contracts, usury
   circuit breakers, Darwinian priority routing.
3. **Desktop agent roster** (`qualia-client-core/agent_registry.rs`): Durable
   agent definitions, backend specs, context policies, execution policies,
   @mention dispatch.

The problem: **these three layers are not wired together.** Each is
implemented and tested in isolation, but no path exists from "Timothy types
@researcher @reviewer in chat" through "the DAG pipeline runs both agents with
deontic-gated capability leases and blackboard-mediated state sharing" to "the
results are sealed as VCs with performance ratings."

---

## 1. Current state audit

### 1.1 What exists and is tested

| Layer | Module | Status | Tests | What it does |
|-------|--------|--------|-------|--------------|
| **A1** CBOR-LD AST | `poet-vibe/src/cbor_ast.rs` | ✅ Done | Tag 4200 bidirectional codec | Lossless AST serialization for agent manipulation |
| **A2** DOMINO | `inference/speculative_decode.rs` | ✅ Done | Prefix-trie token masking | Constrained decoding for in-process LLM |
| **A3** Reflection | `poet-vibe/src/reflection.rs` | ✅ Done | 3-stage self-healing | Search match → semantic lint → dry-run injection |
| **A4** AST query | `poet-vibe/src/ast_query.rs` | ✅ Done | S-expression policy engine | Static architectural policy enforcement |
| **A5** Blackboard | `modalities/blackboard.rs` | ✅ Done | Q42 CRDT channels | Observable state channels with hard/soft constraints |
| **A6** DAGs | `poet-vibe/src/dag.rs` | ✅ Done | DagPipeline, ControlUnit, JudgeFrame | Native DAG pipeline definitions + autonomous routers |
| **A7** Eτ evidential | `modalities/evidential_etau.rs` | ✅ Done | (μ, λ) packing + W3C VCs | Paraconsistent evidence + verifiable credentials |
| **A8** Deontic interrupts | `poet-vibe/src/deontic_interrupt.rs` | ✅ Done | PhaseLeaser, AgentSandbox, F(φ) | seL4-style capability revocation + phase leasing |
| **A9** Semantic skills | `inference/semantic_skills.rs` | ✅ Done | Vector cosine, embeddings, scratchpad | First-class RAG, semantic search, ephemeral memory |
| **Coord ISA** | `governance/coordination.rs` | ✅ Done | 0x70–0x72 opcodes | Authorization grants, resource contracts, performance VCs |
| **Agent roster** | `qualia-client-core/agent_registry.rs` | ✅ Done | AgentDefinition, backend specs | Durable agent definitions with context/execution policies |
| **Chat agents** | `qualia-client-core/chat_agents.rs` | ✅ Partial | Sub-agent DID binding | Display name from human profile, not roster agent |
| **Inference orchestrator** | `inference/orchestrator.rs` | ✅ Done | validate_intent → infer → validate_output | Mandatory governance gate for every LLM call |
| **Local job scheduler** | `qualia-client-core/local_job_scheduler.rs` | ✅ Partial | 5 fixed variants, no agent kind | One-shot, HTTP-only, no cron, no agent turns |
| **MCP tool surface** | `qualia-core-db/mcp/mcp_server.rs` | ✅ Done | 60 tools, in-process | Graph/SPARQL, llm_infer, SHACL, math/science/medical |
| **NOTICES.md** | `coordination/NOTICES.md` | ✅ Active | CLAIM/PROGRESS/RELEASE | Multi-instrument collision avoidance |

### 1.2 What is NOT wired (the gaps)

| Gap | Layers involved | Impact |
|-----|-----------------|--------|
| **G1: DAG → eval not wired** | A6 (`dag.rs`) ↔ `eval.rs` | DAGs are defined but don't drive execution. `lib.rs` doesn't `pub use` `dag`. |
| **G2: Deontic → capability_invoke not wired** | A8 (`deontic_interrupt.rs`) ↔ `eval.rs` / `capability_invoke` | Prohibitions don't gate eval. Phase leases don't gate capability dispatch. |
| **G3: Reflection → isolated PoetSnapshot** | A3 (`reflection.rs`) ↔ `poet_host/mod.rs` | Stage 3 dry-run may write the live graph. No isolated snapshot fork. |
| **G4: Blackboard → DAG node I/O** | A5 (`blackboard.rs`) ↔ A6 (`dag.rs`) | DAG nodes declare inputs/outputs but don't read/write blackboard channels. |
| **G5: Coord ISA → host seams** | `governance/coordination.rs` ↔ key-vault, daemon_swarm, VC minting | `verify_root_delegation` seam, `SuspendedTransactionQueue` yield, VC minting, Darwinian routing all unwired. |
| **G6: Agent roster → chat dispatch** | `agent_registry.rs` ↔ `chat_agents.rs` / `chat_inference.rs` | @mention doesn't resolve to roster agent. Display name is wrong. Backend selection not honoured per-turn. |
| **G7: Agent roster → DAG pipeline** | `agent_registry.rs` ↔ `dag.rs` | No path from "user @mentions 3 agents" to "DAG pipeline with 3 nodes, each node bound to one agent's definition." |
| **G8: Job scheduler → agent turns** | `local_job_scheduler.rs` ↔ `agent_registry.rs` | No `LocalJobKind::AgentTurn`. No cron. No bounded agent job groups. |
| **G9: Eτ evidential → diagnostic loop** | A7 (`evidential_etau.rs`) ↔ `diagnose.rs` | Diagnostics don't carry (μ, λ). Contradictions don't route to quarantine. |
| **G10: Semantic skills → agent context** | A9 (`semantic_skills.rs`) ↔ `chat_inference.rs` | Embeddings/scratchpad not injected into agent context windows. |
| **G11: DOMINO → in-process sampler** | A2 (`speculative_decode.rs`) ↔ `QTensorEngine` | GBNF artifacts exist; inference doesn't consume them. Logit mask not wired. |
| **G12: Performance VCs → reputation routing** | `coordination.rs` ↔ `daemon_swarm.rs` | `compute_priority` exists but daemon_swarm doesn't call it. No Darwinian routing. |
| **G13: Instrument traces (Kind B)** | bylines doc ↔ `AGENT_INTENT_LOGGING_SPEC` | Production notes not shipped. No customer-readable ledger of instrument acts. |
| **G14: DisclosureDenied as value** | disclosure doc ↔ `Value` / capability_invoke | No credentialed refusal value. Deny is an error, not a first-class value. |

---

## 2. Orchestration strategy

### 2.1 Architectural principle: one textual surface, layered lowering

The orchestration strategy follows the same principle as the rest of
VibeScript: **one textual surface for humans and agents, with layered
lowering into existing machinery.** Agents are not a separate language; they
are VibeScript programs (or DAG definitions) executed under capability
leases with deontic gating.

```
Timothy types: @researcher @reviewer please check the EMF interference code
                              │
                              ▼
    ┌─── Agent Roster Resolution ───┐
    │  researcher → AgentDefinition  │
    │  reviewer  → AgentDefinition   │
    └────────────┬───────────────────┘
                 │
                 ▼
    ┌─── DAG Pipeline Construction ───────────────────┐
    │  Node 0: researcher (effect: Cold, caps: [...])  │
    │  Node 1: reviewer  (effect: Cold, caps: [...])   │
    │  Edge: 0 → 1 (reviewer sees researcher output)    │
    │  Blackboard channels: "draft", "review", "verdict"│
    └────────────┬─────────────────────────────────────┘
                 │
                 ▼
    ┌─── Phase Lease + Deontic Gate ───────────────────┐
    │  Phase: "execute"                                 │
    │  Allowed caps: graph.read, capability.invoke      │
    │  Forbidden caps: graph.write (reviewer is read-only)│
    │  Resource contract: token_ceiling=4096, cycles=... │
    └────────────┬─────────────────────────────────────┘
                 │
                 ▼
    ┌─── Per-Node Execution ────────────────────────────┐
    │  1. validate_intent (orchestrator.rs)              │
    │  2. Read blackboard inputs                         │
    │  3. Build context from semantic skills (A9)        │
    │  4. infer() with DOMINO-constrained decoding (A2)  │
    │  5. validate_output (provenance-or-reject)         │
    │  6. Write blackboard outputs                       │
    │  7. If deontic breach → F(φ) interrupt → halt      │
    │  8. If contradiction → (μ,λ) quarantine (A7)       │
    │  9. Performance VC minted (0x72)                   │
    └────────────┬───────────────────────────────────────┘
                 │
                 ▼
    ┌─── Convergence / Judge Frame ─────────────────────┐
    │  JudgeFrame: isolated PoetSnapshot                 │
    │  Sheaf glue: reviewer verdict + researcher draft   │
    │  If glue fails → topological tear diagnostic       │
    │  If glue succeeds → commit to graph                │
    └────────────┬───────────────────────────────────────┘
                 │
                 ▼
    ┌─── Sealed Receipts ───────────────────────────────┐
    │  W3C VC per agent turn (A7)                        │
    │  Performance rating (0x72)                         │
    │  Instrument trace (Kind B)                         │
    └────────────────────────────────────────────────────┘
```

### 2.2 The five orchestration patterns

| Pattern | When | Mechanism | Lowering |
|---------|------|-----------|----------|
| **Single agent turn** | User @mentions one agent | Roster → job scheduler → inference orchestrator | Existing `run_chat_inference_full` + roster agent definition |
| **Parallel independent** | User @mentions N agents, no dependencies | N independent jobs, same approved context | Job scheduler with bounded job group |
| **Sequential pipeline** | User defines or implies a chain | DAG with linear edges | A6 DAG + A5 blackboard for handoffs |
| **Judge/implementer** | One agent generates, another validates | DAG + JudgeFrame (isolated PoetSnapshot) | A6 JudgeFrame + A3 reflection stage 3 on isolated snapshot |
| **Swarm with dynamic routing** | Complex, ill-structured domain | DAG with ControlUnit (autonomous router) | A6 ControlUnit reads blackboard, selects next node |

### 2.3 State sharing: the blackboard is the bus

Agents never see each other's raw context. They read from and write to
**blackboard channels** (A5). This is the MCP replacement: instead of
flooding context windows with JSON-RPC tool descriptions, agents receive
compact topological pointers (`QuinRef` / `did:q42:…`) to blackboard entries.

```
Agent A writes → blackboard["draft"] = QuinRef{...}
Agent B reads  ← blackboard["draft"]
Agent B writes → blackboard["review"] = QuinRef{...}
```

Hard constraints (pinned by the principal) and soft constraints (negotiable)
are preserved across agent iterations. A budget constraint pinned by the
principal is visible to all downstream agents and cannot be hallucinated away.

### 2.4 Governance: deontic gates at every boundary

Every agent turn passes through three gates (already implemented in
`orchestrator.rs`, extended by A8):

1. **validate_intent** — pre-flight: N3Logic rights ontology, profile
   constraints, phase lease check, deontic prohibition check.
2. **infer** — the actual LLM call, with DOMINO-constrained decoding (A2),
   resource contract enforcement (0x71), and mid-generation DenyRollback
   (Phase 8 bifurcated compute).
3. **validate_output** — post-flight: provenance-or-reject, semantic
   validation, sheaf glue check (if part of a DAG), performance VC minting
   (0x72).

A deontic breach at any gate triggers an F(φ) interrupt (A8) that
immediately revokes all capabilities and halts the agent. The breach is
recorded as a sealed, signed receipt (A7).

### 2.5 Convergence: sheaf gluing, not Promise.all

When multiple agents contribute to a shared result, convergence is a **sheaf
gluing operation** (per the grok recommendations and the vibe-design docs):

- **Success** → the contributions compose into a higher-dimensional simplex,
  committed to the graph.
- **Failure** → a **topological tear diagnostic** is emitted, with evidential
  (μ, λ) annotations on the conflicting claims. The tear is routed to a
  quarantine context (`q42:isolated`), not thrown as an exception.

This is stronger than `Promise.all` because it preserves the *reason* for
failure as a first-class geometric object, not just an error string.

### 2.6 Agent context: token-efficient stalks

When an agent is spawned into a phase or branch, it receives:

1. A **compact topological pointer** to the relevant blackboard channels
   (not the full conversation history).
2. The **sheaf rules** (hard/soft constraints) for its branch.
3. A **capability lease** scoped to its phase.
4. A **semantic context** built from A9 (embeddings of relevant graph
   entries, not raw text dumps).

This is the practical realization of "zero-copy context" from the grok
recommendations. An agent's context window contains pointers and shapes,
not 10,000 lines of dumped text.

---

## 3. Implementation requirements

### 3.1 Refactors required

| # | Refactor | Reason | Blast radius | Risk |
|---|----------|--------|--------------|------|
| **R1** | `pub use dag, deontic_interrupt, reflection` from `poet-vibe/src/lib.rs` | A6/A8/A3 are internal modules not on the public surface. They must be exportable for the host to call. | Low — adding `pub use` is additive. | Low |
| **R2** | Wire `deontic_interrupt::PhaseLeaser` into `eval.rs` capability dispatch | G2: prohibitions must gate eval. The eval loop's `capability_invoke` path needs to check the active phase lease before dispatching. | Medium — touches the eval hot path. Must be zero-heap (Tier 1). | Medium |
| **R3** | Wire `dag::DagPipeline` execution into `poet_host` | G1: DAGs need an executor that runs nodes in topological order, reading/writing blackboard channels between nodes. | Medium — new `dag_executor` module in `poet_host`. | Medium |
| **R4** | Isolate `reflection::Stage3` on a `PoetSnapshot` fork | G3: reflection stage 3 must not write the live graph. Need `PoetSnapshot::fork()` or equivalent. | Low — additive method on PoetSnapshot. | Low |
| **R5** | Connect `blackboard` channels to `dag` node I/O | G4: DAG nodes declare inputs/outputs as channel names but don't actually read/write. Need a `BlackboardBus` that nodes reference. | Low — the blackboard API already exists; this is wiring. | Low |
| **R6** | Add `LocalJobKind::AgentTurn` to job scheduler | G8: scheduler needs to accept agent turn jobs with immutable agent/runtime/context snapshots. | Low — new enum variant + handler. | Low |
| **R7** | Resolve @mentions to roster agents in `chat_agents.rs` | G6: @mention should resolve to `AgentDefinition` by slug, use its display name, and honour its backend spec per-turn. | Medium — touches chat dispatch. | Medium |
| **R8** | Wire `governance/coordination.rs` host seams | G5: `verify_root_delegation`, `SuspendedTransactionQueue` yield, VC minting, Darwinian routing. | Medium — multiple seam points. | Medium |
| **R9** | Wire DOMINO logit mask into `QTensorEngine` | G11: GBNF artifacts exist but inference doesn't consume them. Need to apply the mask during sampling. | Medium — touches inference hot path. | High |
| **R10** | Add `DisclosureDenied` value type | G14: credentialed refusal as a first-class value. | Low — new Value variant (post-0.1 grammar). | Low |
| **R11** | Wire `compute_priority` into `daemon_swarm.rs` | G12: Darwinian routing exists but isn't called. | Low — one function call site. | Low |
| **R12** | Instrument trace ledger (Kind B) | G13: production notes for customer audit. Needs a local ledger structure + write path. | Medium — new module. | Low |

### 3.2 New modules required

| Module | Location | Purpose | Depends on |
|--------|----------|---------|------------|
| `dag_executor.rs` | `poet_host/invoke/agent/` | Executes a `DagPipeline` node-by-node, wiring blackboard I/O, phase leases, and deontic gates. | R1, R3, R5, A6, A5, A8 |
| `agent_context_builder.rs` | `poet_host/invoke/agent/` | Builds token-efficient agent context from blackboard pointers + semantic skills (A9). | A5, A9, R5 |
| `stalk.rs` | `poet_host/invoke/agent/` | Isolated agent context (PoetSnapshot fork + capability lease + pulse topic prefix). | R4, A8 |
| `instrument_trace.rs` | `governance/` | Customer-readable ledger of instrument acts (Kind B). | G13, A7 |
| `agent_turn_handler.rs` | `qualia-client-core/` | Connects job scheduler → agent roster → inference orchestrator → DAG executor. | R6, R7, R3 |

### 3.3 Existing modules to extend (no rewrite)

| Module | Extension | Reason |
|--------|-----------|--------|
| `eval.rs` | Phase lease check before `capability_invoke` dispatch | R2 / G2 |
| `lib.rs` (poet-vibe) | `pub use dag, deontic_interrupt, reflection` | R1 / G1 |
| `chat_agents.rs` | @mention → roster resolution, display name, backend selection | R7 / G6 |
| `chat_inference.rs` | Agent context from A9 semantic skills, not raw text | G10 |
| `local_job_scheduler.rs` | `AgentTurn` job kind + bounded job groups + cron | R6 / G8 |
| `orchestrator.rs` | Deontic phase lease check in `validate_intent` | R2 / G2 |
| `daemon_swarm.rs` | Call `compute_priority` for Darwinian routing | R11 / G12 |
| `diagnose.rs` | Optional (μ, λ) evidential annotation on diagnostics | G9 |
| `QTensorEngine` | DOMINO logit mask application during sampling | R9 / G11 |

---

## 4. Priority and dependency analysis

### 4.1 Dependency graph

```
R1 (pub use) ─────────────────────────────────────────────────┐
                                                               │
R2 (deontic → eval) ──────────────────────────────────────────┤
                                                               │
R3 (DAG executor) ────── R5 (blackboard → DAG) ───────────────┤
                          A5 (blackboard) ─────────────────────┤
                          A6 (DAG) ────────────────────────────┤
                                                               │
R4 (reflection isolation) ────────────────────────────────────┤
                                                               │
R6 (job scheduler) ───────────────────────────────────────────┤
                                                               ├──► Agent Turn Handler ──► Chat @mention dispatch
R7 (@mention → roster) ───────────────────────────────────────┤
                                                               │
R8 (coord ISA seams) ─────────────────────────────────────────┤
                                                               │
R9 (DOMINO → sampler) ────────────────────────────────────────┤
                                                               │
R10 (DisclosureDenied) ───────────────────────────────────────┤
                                                               │
R11 (Darwinian routing) ──────────────────────────────────────┤
                                                               │
R12 (instrument trace) ───────────────────────────────────────┘
```

### 4.2 Priority tiers

| Tier | Items | Rationale | Can start now? |
|------|-------|-----------|----------------|
| **P0 — Foundation** | R1, R4, R5 | These are additive, low-risk, and unblock everything else. `pub use` the internal modules, isolate reflection, wire blackboard to DAG. | ✅ Yes |
| **P1 — Governance wiring** | R2, R8, R11 | Wire deontic gates into eval, connect coordination ISA seams, enable Darwinian routing. These make the existing governance machinery actually enforce. | ✅ Yes (after P0) |
| **P2 — Agent dispatch** | R6, R7, R3 | Job scheduler agent turns, @mention roster resolution, DAG executor. These connect the desktop UI to the agent primitives. | ✅ Yes (after P1) |
| **P3 — Inference quality** | R9, R10 | DOMINO logit mask in sampler, DisclosureDenied value. These improve agent output quality and rights enforcement. | ✅ Yes (after P2) |
| **P4 — Audit and traces** | R12, G9 | Instrument trace ledger, evidential annotations on diagnostics. These are the accountability layer. | ✅ Yes (after P2) |
| **P5 — Ecosystem** | LSP, CLI, playground | From the vibe-design to-do list (T60–T65). These are post-orchestration. | After P3 |

### 4.3 Critical path

The critical path to "Timothy can @mention two agents and they run as a
governed DAG pipeline with blackboard-mediated state sharing" is:

```
R1 (pub use) → R5 (blackboard→DAG) → R3 (DAG executor) → R6 (job scheduler) → R7 (@mention) → agent_turn_handler
                                                                      ↑
R2 (deontic→eval) ──────────────────────────────────────────────────────┘
```

Six steps. R1 and R2 can run in parallel. R5 depends on R1. R3 depends on R5.
R6 and R7 can run in parallel after R3. The agent turn handler ties it all
together.

### 4.4 What can be parallelized

| Workstream | Items | Can run in parallel with |
|------------|-------|-------------------------|
| **A: Core wiring** | R1, R5, R3, R6, R7 | Workstream B, C |
| **B: Governance** | R2, R8, R11 | Workstream A, C |
| **C: Inference quality** | R9, R10 | Workstream A, B |
| **D: Audit** | R12, G9 | After A + B |

Workstreams A, B, and C are independent and can be assigned to different
instruments (per the NOTICES.md coordination protocol).

---

## 5. Tracking

### 5.1 Progress log

Progress will be tracked in:
`docs/plans/multi-agent-orchestration-PROGRESS-LOG.md`

Each step must follow the CLAUDE.md §9 per-step progress logging rule:
step + status, what was built, measured results, what Timothy needs to
decide, next step.

### 5.2 NOTICES.md coordination

Before starting any workstream, the instrument must:
1. Read `coordination/NOTICES.md` for existing CLAIMs.
2. Append a CLAIM line with the files being touched.
3. Append PROGRESS / BLOCKED / RELEASE lines as work proceeds.

### 5.3 Phase tracker

| Phase | Items | Status | Started | Completed | Tests added |
|-------|-------|--------|---------|-----------|-------------|
| P0: Foundation | R1, R4, R5 | Not started | — | — | — |
| P1: Governance | R2, R8, R11 | Not started | — | — | — |
| P2: Agent dispatch | R6, R7, R3 | Not started | — | — | — |
| P3: Inference quality | R9, R10 | Not started | — | — | — |
| P4: Audit | R12, G9 | Not started | — | — | — |

---

## 6. Decisions needing Timothy

| # | Decision | Default if you want speed | Status |
|---|----------|--------------------------|--------|
| D1 | Should the DAG executor live in `poet_host/invoke/agent/` or as a standalone `poet_host/dag_executor.rs`? | `poet_host/invoke/agent/dag_executor.rs` (follows the §11 library pattern) | **Resolved by implementation** — lives at `poet_host/invoke/agent/dag_executor.rs` |
| D2 | Should @mention dispatch support agent-to-agent chaining (agent A can @mention agent B) or only human-to-agent? | Human-to-agent first; agent-to-agent requires explicit principal scheduling (per roster plan §3.3) | **Resolved by Timothy 2026-08-20** — see §6.1 below: mention taxonomy with FOAF Agent alignment |
| D3 | Should the initial DAG executor support ControlUnit (autonomous routing) or only static topological order? | Static topological order first; ControlUnit is P2.5 (after basic pipeline works) | **Resolved by implementation** — static topological order |
| D4 | Should DOMINO logit masking be applied to all inference or only when an agent explicitly requests constrained decoding? | Explicit request first (capability-gated); always-on is a future default | **Resolved by implementation** — explicit/capability-gated |
| D5 | Should the instrument trace ledger (Kind B) be local-only or also publish sealed summaries to the graph? | Local-only first; graph publication is a principal decision | **Resolved by implementation** — local-only (documented in `instrument_trace.rs`) |
| D6 | Should DisclosureDenied be a `Value` variant (post-0.1 grammar) or a `Result` error kind (0.1 compatible)? | Result error kind first (0.1 compatible); Value variant is post-0.1 | **Resolved by implementation** — implemented as `Value::DisclosureDenied` (the post-0.1 option). Needs Timothy confirmation. |

### 6.1 D2 Resolution — Mention Taxonomy with FOAF Agent Alignment (Timothy, 2026-08-20)

The binary "human-to-agent vs agent-to-agent" framing is too narrow.
Mentions should be broken into a **taxonomy** so different security
settings (deontic gates, capability leases, disclosure boundaries) can
be applied per mention class.

**FOAF alignment:** In FOAF, `foaf:Agent` is the broad class — a person,
group, software, or physical artifact. "Agent" in the mention system
should mean any FOAF Agent, not just "AI agent". The existing
`AgentType` enum in `front_door.rs` already maps to FOAF types:

| AgentType | FOAF / RDF type |
|-----------|-----------------|
| `NaturalPerson` | `foaf:Person` |
| `Organization` | `schema:Organization` |
| `Group` | `foaf:Group` |
| `AiAgent` | `QDP:AIAgent` |
| `HumanitarianService` | `QDP:EssentialService` |
| `ContentProvider` | `QDP:ContentProvider` |

**Mention taxonomy (source × target):**

| Mention class | Source type | Target type | Security posture |
|---------------|------------|-------------|------------------|
| **Principal → AI agent** | `foaf:Person` (principal) | `QDP:AIAgent` | Highest trust — principal's direct command. Full capability lease. |
| **Principal → Group** | `foaf:Person` (principal) | `foaf:Group` | High trust — principal dispatches to a team. Group resolution + per-member policy intersection. |
| **AI agent → AI agent (same principal)** | `QDP:AIAgent` | `QDP:AIAgent` | Medium trust — requires explicit principal scheduling or pre-authorized DAG edge. Capability lease is scoped to the DAG edge, not the agent's full capabilities. |
| **AI agent → Group (same principal)** | `QDP:AIAgent` | `foaf:Group` | Medium trust — agent dispatches to a team within its authorised scope. Group resolution + policy intersection + DAG-edge scoping. |
| **AI agent → AI agent (cross-principal)** | `QDP:AIAgent` | `QDP:AIAgent` | Low trust — requires explicit delegation (`DelegatedAccess` in CRDT). Capability lease is minimal; disclosure boundary is fail-closed unless explicitly permitted. |
| **Software → AI agent** | `foaf:Agent` (software, non-LLM) | `QDP:AIAgent` | Medium trust — a software artifact (sensor, pipeline, scheduler) triggers an agent. Requires a registered software identity + capability lease. |
| **Physical artifact → AI agent** | `foaf:Agent` (physical) | `QDP:AIAgent` | Medium trust — a device/sensor triggers an agent. Requires a registered device identity + capability lease. |
| **AI agent → Human** | `QDP:AIAgent` | `foaf:Person` | **Special case** — an agent requesting human review. Not a "dispatch" but a "request for attention". Goes through the deontic interrupt path, not the DAG executor. |

**Design principles:**

1. **Every mention has a source and target, both typed as `foaf:Agent`
   subtypes.** The source type determines what security posture applies.
2. **Different mention classes get different deontic gates.** A
   principal→agent mention gets `OBLIGATE` (the agent must act). An
   agent→agent mention gets `PERMIT` (the agent may act, if the DAG
   edge authorises it). A cross-principal mention gets `FORBID` unless
   explicit delegation exists.
3. **Group mentions resolve to per-member dispatch with policy
   intersection.** `@team` expands to the group's members, but each
   member's `AgentContextPolicy` and `AgentDataPolicy` are intersected
   with the group-level policy — never widened.
4. **Agent→Human is not a dispatch — it's a request for review.** It
   goes through the deontic interrupt path (`deontic_interrupt.rs`),
   not the DAG executor. The human sees the request and can approve,
   modify, or deny.
5. **Software and physical artifacts are first-class FOAF Agents.** A
   sensor reading can trigger an agent the same way a human can — but
   with a different security posture (registered identity + capability
   lease, not principal command).
6. **The mention taxonomy is extensible.** New source/target types can
   be added without changing the dispatch machinery — only the policy
   table changes.

**Implementation note:** The existing `parse_mention` / `resolve_mention`
/ `PromptDispatch` pipeline in `chat_agents.rs` handles the flat
`@slug` → `AgentDefinition` case. The taxonomy extends this with:
- A `MentionClass` enum (source type × target type).
- A `MentionPolicy` table mapping each class to its deontic gate,
  capability lease scope, and disclosure boundary defaults.
- Group resolution (`foaf:Group` → member list with policy intersection).

---

## 7. What NOT to do

- Do not build a second agent language beside VibeScript. Agents author `.vibe`
  or DAG definitions; the binary surface is Tag 4200 CBOR-LD.
- Do not flood agent context windows with raw text. Use topological pointers
  (`QuinRef` / `did:q42:…`) and blackboard channels.
- Do not bypass the inference orchestrator (`validate_intent → infer →
  validate_output`). It is mandatory infrastructure, not optional middleware.
- Do not wire deontic interrupts as deferred cleanup. They are immediate
  (seL4-style) revocation.
- Do not let agents see each other's raw context. The blackboard is the bus.
- Do not create a sibling repo or worktree. All work in
  `C:\Projects\qualia-27062026`.
- Do not add Ollama or external model servers. In-process inference only.
- Do not skip the NOTICES.md coordination protocol when multiple instruments
  are working.

---

## 8. Related documents

- [`docs/manuals/standards/MULTI_AGENT_PROTOCOL.md`](../manuals/standards/MULTI_AGENT_PROTOCOL.md) — Coordination ISA (0x70–0x72)
- [`docs/plans/agent-roster-multi-model-context-plan-2026-08-13.md`](agent-roster-multi-model-context-plan-2026-08-13.md) — Agent roster plan
- [`docs/plans/agentic-chat-workspace.md`](agentic-chat-workspace.md) — Chat workspace plan
- [`docs/plans/Extending Vibescript for LLM Agents.md`](Extending%20Vibescript%20for%20LLM%20Agents.md) — Agent extension consult
- [`docs/plans/vibe-design/20260819_grok.md`](vibe-design/20260819_grok.md) — Progressive layering, token-efficient stalks
- [`docs/plans/vibe-design/20260819_excellence-first.md`](vibe-design/20260819_excellence-first.md) — Type lattice, wire-or-delete
- [`docs/vibescript-full-impl-PLAN.md`](../vibescript-full-impl-PLAN.md) §8 — Vibe-design to-do list (T24–T27 wire shadow runtime, T47–T50 sheaves/stalks, T51–T54 MCP replacement)
- [`AGENTS.md`](../../AGENTS.md) — Multi-agent collaboration ecosystem rules
- [`CLAUDE.md`](../../CLAUDE.md) §10 — Announce before you act; §15 — Fidelity to the principal
