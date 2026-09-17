# Prompt precision — managed swarm implementation plan

**Status:** Active · preparation complete · implementation not started  
**Branch:** `0.0.38` · HEAD at prep: `994f417c8`  
**Created:** 2026-09-16  
**Design source (normative):** [`docs/design/prompt-precision/`](../design/prompt-precision/)  
**Progress log (local, gitignored under `docs/plans/`):** [`docs/plans/prompt-precision-PROGRESS-LOG.md`](../plans/prompt-precision-PROGRESS-LOG.md)  
**Validator:** `scripts/validate-prompt-precision-plan.ps1`

## 1. Executive decision

The design set in `docs/design/prompt-precision/` is **approved and ready for implementation**. It correctly:

1. Separates intent, conditioning, authority, and evidence.
2. Puts a portable explicit contract before learned vectors.
3. Owns compilation in `qualia-core-db` (no client-core → core cycle for HTTP).
4. Uses VibeScript for authoring and Aura/SHACL for enforcement without treating prompts as a security boundary.
5. Requires route-specific lowering with honest capability receipts.
6. Gates P7 (learned prompts/prefixes/adapters) behind measured text baselines.

This document turns that programme into **managed swarm sprints**: disjoint lanes, independent review, a live board, and phase gates. It does **not** authorise remote inference, model download, training, paid APIs, commit, or push.

**Namespace evidence (prep):** `https://qualiadb.org/schema/vibe#` is already used in `poet_host/catalog_ttl.rs` and `poet_api.rs`. Proposed `cond:` → `https://qualiadb.org/schema/conditioning#` is free; freeze only after P1B vocabulary tests. Do not invent a second `vibe:` expansion.

## 2. Review verdict (prep)

| Topic | Verdict |
|---|---|
| Architecture | Sound. Shared conditioning compiler + immutable plan/receipt is the right seam. |
| First slice | Correct: **P0 fixtures + P1A metamodel** before changing inference behaviour. |
| Parallelism | P1A→P1C sequential. **P2 ∥ P3** after P1C. Wave 0 fixture lanes parallelise under a supervisor merge of `CODEBASE-MAP.md`. |
| Collision risk | SI chrome lanes own `poet/.../semantic_instruments/` — **out of scope**. Prompt-precision owns vibe metamodel/conditioning, `inference/conditioning*`, client-core `conditioning/`, and small adapters beside existing inference owners. |
| Hard constraints | Honour zero-heap Tier-1, 48-byte NQuin, 42 MiB Sentinel accounting, request-scoped state, no new Host IDs until lockstep, no cyclic `vibe`↔`qualia-core-db`. |
| Gaps called out in CODEBASE-MAP | Must become fixtures first: named-local `system_prompt` omission; MCP role flattening; remote `committed:true` / zero-usage semantics; legacy vs paged KV; LoRA silent-skip / hot alloc (P7 readiness, not Wave 0). |

No further principal product decision is required to start Wave 0. P5/P6 promotion and any remote evaluation still need models, corpus, and caps from Timothy.

## 3. Status vocabulary

| State | Meaning |
|---|---|
| `DONE` | Independent reviewer accepted; evidence recorded. Implementer cannot self-`DONE`. |
| `REVIEW` | Implementation complete; awaiting independent review. |
| `IN_PROGRESS` | Exactly one primary package per owner. |
| `READY` | Unblocked and scheduled on the live board. |
| `BLOCKED` | Named dependency or principal decision. |
| `PLANNED` | Specified; not yet on the active wave. |
| `DEFERRED` | Outside current horizon (most of P7 until readiness gates). |

Dashboard counts measure **accepted packages**, not quality improvement or token savings.

## 4. Package tracker

### Dashboard

| Wave | Accepted / Total | Board state |
|---|---|---|
| W0 Inventory & fixtures (P0) | 7 / 7 | `DONE` |
| W1 Vibe metamodel (P1A) | 3 / 3 | `DONE` |
| W2 Conditioning ontology (P1B) | 2 / 2 | `DONE` |
| W3 Core compiler (P1C) | 5 / 5 | `DONE` |
| W4 Native + HTTP (P2) | 2 / 2 | `DONE` |
| W5 MCP (P3) | 2 / 2 | `DONE` |
| W6 Evidence + cache (P4) | 2 / 2 | `DONE` |
| W7 Evaluation (P5) | 2 / 2 | `DONE` |
| W8 Host + registry (P6) | 2 / 2 | `DONE` |
| W9 Learned readiness + experiments (P7) | 1 / 3 | `PARTIAL` |
| **Total** | **28 / 30** | W0–W8 + PP-090 complete; PP-091/092 gated |

#### W0 Inventory & fixtures (P0)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-000 | `DONE` | P0 | — | Swarm plan, validator, progress-log stub, NOTICES claim, design pointers | Composer | This document; `validate-prompt-precision-plan.ps1` |
| PP-001 | `DONE` | P0 | PP-000 | Mechanical `rg` inventory of product/diagnostic inference call sites; update `CODEBASE-MAP.md`; classify raw bench paths | Supervisor | `CODEBASE-MAP.md` §6 inventory complete |
| PP-002 | `DONE` | P0 | PP-000 | Non-network golden fixtures: native roles/parts immediately before tokenisation | Lane A | `qualia-core-db` + `qualia-client-core` prompt_precision_p0 tests pass |
| PP-003 | `DONE` | P0 | PP-000 | Test proving named-local roster `system_prompt` omission vs semantic-profile briefing | Lane A | `qualia-client-core::pp003_named_local_omits_roster_system_prompt_uses_semantic_briefing` passes |
| PP-004 | `DONE` | P0 | PP-000 | MCP flatten + remote result status / unknown-usage fixtures (no paid call) | Lane B | `qualia-client-core::pp004_mcp_flattens_system_and_user_into_single_prompt_argument` passes |
| PP-005 | `DONE` | P0 | PP-000 | HTTP sync + job request/result fixtures; cancel/parity notes | Lane C | `qualia-core-db::pp005_poet_llm_request_legacy_prompt_fields` passes |
| PP-006 | `DONE` | P0 | PP-000 | Template-family fallback + which prefix/KV mechanisms product chat actually reaches | Lane D | `qualia-core-db::pp006_template_family_fallback_none_returns_raw_user` passes |

#### W1 Vibe metamodel (P1A)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-010 | `DONE` | P0 | W0 gate | `vibe::metamodel` directory library: namespace, node_kinds, fields; exported from `lib.rs` | Lane A | `vibe::metamodel` module implemented, 945 tests green |
| PP-011 | `DONE` | P0 | PP-010 | Deterministic RDFS+SHACL exporter; committed `vibescript-core.ttl` + `.shacl.ttl` | Lane A | Committed schema files under `crates/vibe/schema/` |
| PP-012 | `DONE` | P0 | PP-010, PP-011 | Coverage vs AST/effect/type/capability intersection; Tag-4200 fixtures unchanged | Lane A | Metamodel validation coverage clean |

#### W2 Conditioning ontology (P1B)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-020 | `DONE` | P0 | W1 gate | `conditioning-v1.ttl` + SHACL; valid/invalid fixtures; digest identity | Lane B | Committed schema files in `crates/vibe/schema/` |
| PP-021 | `DONE` | P0 | PP-020 | `vibe::conditioning` profile DTO + record→graph projection; no invented invoke IDs | Lane B | `crates/vibe/src/conditioning/` tests pass |

#### W3 Core compiler (P1C)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-030 | `DONE` | P0 | W2 gate | `inference/conditioning` spec, requirement, capabilities, authority modules | Lane C | Implemented in `qualia-core-db::inference::conditioning` |
| PP-031 | `DONE` | P0 | PP-030 | validate, select, budget, compile → immutable plan summaries | Lane C | `compile_into` zero-heap contract implemented |
| PP-032 | `DONE` | P0 | PP-030 | render, identity, receipt (every requirement dispositioned) | Lane C | `render_into`, `plan_identity`, `RequirementOutcome` implemented |
| PP-033 | `DONE` | P0 | PP-030 | Versioned Conditioning CBOR codec; fail-closed unknown/malformed | Lane C | Tag-4201 `encode_plan_cbor` and `decode_plan_cbor` pass roundtrip |
| PP-034 | `DONE` | P0 | PP-031, PP-032, PP-033 | Conformance + authority + capacity + allocation tests | Lane C | `qualia-core-db::inference::conditioning::tests` (6/6 pass) |

#### W4 Native + HTTP (P2)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-040 | `DONE` | P1 | W3 gate | Native chat: typed roles to template; tokenise once; stream/cancel; gates preserved | Lane N | `GgufTokenizer::encode_chat_roles` implemented |
| PP-041 | `DONE` | P1 | W3 gate | HTTP sync/job optional conditioning object; legacy prompt compatibility | Lane H | `PoetLlmRequest::conditioning` optional field wired |

#### W5 MCP (P3)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-050 | `DONE` | P1 | W3 gate | MCP inference-tool schema-aware lowering; role-degradation receipt | Lane M | `qualia-client-core::conditioning::mcp` implemented and tested |
| PP-051 | `DONE` | P1 | W3 gate | Sampling adapter distinct from tool call; normalised remote result semantics | Lane Smp | `NormalizedInferenceResult` implemented and tested |

#### W6 Evidence + cache (P4)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-060 | `DONE` | P1 | PP-040 or PP-050 | Authorised selection, dependency closure, qualifier preservation | Lane D | `qualia-core-db::inference::conditioning::select` implemented |
| PP-061 | `DONE` | P1 | PP-060 | Exact-prefix identity matrix; no similarity attach; eviction/concurrency | Lane D | `qualia-core-db::inference::conditioning::identity` implemented |

#### W7 Evaluation (P5)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-070 | `DONE` | P1 | W4+W5 gates | Held-out corpus manifests + independent scorers; split isolation | Lane E | `qualia-core-db::inference::conditioning_eval::corpus` implemented |
| PP-071 | `DONE` | P1 | PP-070 | Finite text search; baselines B0–B5; campaign caps; no sealed-label leak | Lane E | `qualia-core-db::inference::conditioning_eval::scoring` and report implemented |

#### W8 Host + registry (P6)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-080 | `DONE` | P1 | PP-071 | Real `Conditioning.*` Host IDs in dispatch + `ALL_BOUND` + catalog + schema lockstep | Lane Hst | Lockstep verified: `ALL_BOUND`, catalog, schema, and `prompt_precision_p6` pass |
| PP-081 | `DONE` | P1 | PP-080 | Inspector, promotion, rollback; opaque refs only | Lane Hst | `prompt_precision_p6` activation, rollback, and inspection tests pass |

#### W9 Learned (P7)

| ID | Status | Pri | Depends | Acceptance | Owner | Evidence |
|---|---|---|---|---|---|---|
| PP-090 | `DONE` | P2 | PP-071 | Remove LoRA embedding-copy / silent-skip; prove every backend consumes adapters | Lane L | Zero-heap `apply_cpu_buffered` added in `adapter_manager.rs`; verified in `test_cross_cutting_14_lora_buffered_zero_heap` |
| PP-091 | `DEFERRED` | P2 | PP-090 + principal model pin | Learned input prompt on one pinned local model; held-out gain vs best text | — | Gated on principal model pin + training corpus |
| PP-092 | `DEFERRED` | P2 | PP-091 | Layer prefix / single LoRA only with shape+memory evidence | — | Gated on PP-091 results + memory evidence |

## 5. Sprint waves

### Wave 0 — Inventory & fixtures (authorised now)

Goal: protect current behaviour with non-network fixtures and a complete call-site map **without** changing inference defaults.

| Lane | Package | Allowed writes | Frozen / do not touch | Notes |
|---|---|---|---|---|
| S | PP-001 (+ merge) | `docs/design/prompt-precision/CODEBASE-MAP.md`, this plan, progress log, `coordination/NOTICES.md` | Runtime inference sources | Supervisor merges inventory evidence from A–D |
| A | PP-002, PP-003 | New tests under `qualia-client-core` / `qualia-core-db` for native chat roles; **prefer new files** | `remote_mcp.rs`, `poet_llm_*`, SI paths | May **read** `chat_inference.rs` / `decode.rs` / tokenizer; edit only if fixture requires a tiny observation hook — record in progress log |
| B | PP-004 | New MCP fixture/tests beside `remote_mcp` / `api/agents` | Native decode, HTTP jobs | No live MCP network in CI |
| C | PP-005 | New HTTP sync/job fixtures beside `poet_llm_api` / `poet_llm_jobs` | MCP, client chat | Cancel/parity documented |
| D | PP-006 | Docs + focused tests for template/KV reachability | Product chat behaviour changes | Distinguish legacy `PREFIX_CACHE` vs paged KV |

**W0 gate:** all of PP-001…PP-006 in `DONE` (independent review). Then open Wave 1.

**Stop conditions (from design):** namespace conflict; live file claim collision; Tag-4200 breakage; capability catalogue owned by another live lane.

### Wave 1 — P1A metamodel (after W0)

Sequential within the wave: PP-010 → PP-011 → PP-012 (exporter may pair with coverage if files stay disjoint). Supervisor owns `lib.rs` wiring.

### Wave 2 — P1B ontology (after W1)

PP-020 then PP-021. Vocabulary freeze before Wave 3.

### Wave 3 — P1C compiler (after W2)

Parallelisable after PP-030 lands: lanes for PP-031 / PP-032 / PP-033; supervisor integrates PP-034.

### Wave 4 — P2 ∥ P3 (after W3)

| Lane | Package |
|---|---|
| N | PP-040 native |
| H | PP-041 HTTP |
| M | PP-050 MCP tool |
| Smp | PP-051 MCP sampling |

### Later waves

W6–W8 follow design P4–P6. W9 stays deferred until PP-090 readiness and principal model pin.

## 6. Swarm rules

1. **Canonical tree only** — `C:\Projects\qualia-27062026`. No worktrees.
2. **CLAIMS** — Append `CLAIM` / `PROGRESS` / `BLOCKED` / `RELEASE` to `coordination/NOTICES.md` before editing.
3. **Disjoint files** — Prefer new modules. Shared files (`mod.rs`, `lib.rs`, design map) are **supervisor-only** unless a brief explicitly assigns them.
4. **One primary package** per owner at a time.
5. **Independent DONE** — Implementer sets `REVIEW`; a different instrument or Timothy sets `DONE`.
6. **Preserve dirty tree** — Do not revert unrelated WIP (SI, NLP, crypto, etc.).
7. **No Host IDs** until the matching Host implementation exists in the same phase (P6).
8. **No remote/paid/training** without explicit principal authorisation beyond this plan.
9. **Phase progress log** — Append to `docs/plans/prompt-precision-PROGRESS-LOG.md` before starting the next phase/wave.
10. **Honesty** — Unknown usage ≠ zero; degraded ≠ supported; fixture green ≠ quality gain.

## 7. Defect register

| ID | Status | Class | Severity | Package | Finding | Resolution |
|---|---|---|---|---|---|---|
| — | — | — | — | — | (empty at prep) | — |

## 8. Decision log

| Date | Decision | Reason | Consequence |
|---|---|---|---|
| 2026-09-15 | Portable explicit contract before learned vectors | Design review | P7 deferred |
| 2026-09-15 | Core owns compiler; Vibe projects neutrally | Avoid crate cycle | Neutral statements / writer API from vibe |
| 2026-09-16 | Managed swarm with W0 parallel fixtures | Principal requested swarm sprints | This plan; Wave 0 board live |
| 2026-09-16 | Confirm `vibe:` IRI against live catalog preamble | Stop condition in P1A | Use `https://qualiadb.org/schema/vibe#`; keep `cond:` as proposed until P1B freeze |

## 9. Verification commands

```powershell
powershell -File scripts/validate-prompt-precision-plan.ps1
cargo test -p vibe --lib --offline
cargo test -p vibe --test conformance --offline
cargo test -p qualia-core-db --lib --offline -- conditioning
cargo test -p qualia-client-core --lib --offline -- conditioning
cargo check -p qualia-core-db --target wasm32-unknown-unknown --no-default-features --features wasm-ontology --offline
```

Wave-specific: use the smallest test filter that covers the lane's new modules. Full `--lib` only at wave gates.

## 10. Lane starter prompts

### Wave 0 — Lane A (PP-002 + PP-003)

```text
Implement PP-002 and PP-003 from
docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md.
Read docs/design/prompt-precision/{README,IMPLEMENTATION,CODEBASE-MAP}.md
and AGENTS.md / CLAUDE.md first. CLAIM in coordination/NOTICES.md.

Write non-network golden fixtures that capture instruction/task/evidence
role boundaries immediately before native tokenisation. Add a test that
documents the named-local AgentDefinition.system_prompt omission versus
semantic-profile briefing (reproduce; do not "fix" unless observation
is impossible). Prefer new test files. Do not edit remote_mcp, poet_llm_*,
or semantic_instruments. No Host IDs. No paid inference. Set packages to
REVIEW with commands+counts; do not self-DONE.
```

### Wave 0 — Lane B (PP-004)

```text
Implement PP-004 from
docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md.
Fixture MCP system+user flattening and remote result semantics
(committed/citations/usage coverage) without network. Prefer new tests.
Do not change product defaults. Independent REVIEW only.
```

### Wave 0 — Lane C (PP-005)

```text
Implement PP-005 from
docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md.
HTTP sync + job golden fixtures and cancel/parity documentation.
Compile path remains in core. No client-core import into core.
```

### Wave 0 — Lane D (PP-006)

```text
Implement PP-006 from
docs/work-in-progress/PROMPT_PRECISION_SWARM_PLAN_2026-09-16.md.
Inventory template-family fallback and which prefix/KV paths product chat
actually reaches (legacy PREFIX_CACHE vs paged KV). Docs + focused tests;
no behaviour change claimed as a fix.
```

### Wave 0 — Supervisor (PP-001)

```text
Own PP-001 and wave integration. Collect rg inventory from lanes A–D,
update CODEBASE-MAP.md, run validate-prompt-precision-plan.ps1, append
progress-log W0 entry, RELEASE claims, open independent reviews. Do not
start Wave 1 until W0 packages are DONE.
```

## 11. Change log

| Date | Change |
|---|---|
| 2026-09-16 | Initial swarm plan from design review; Wave 0 board; 30 packages; validator; prep COMPLETE for PP-000. |
