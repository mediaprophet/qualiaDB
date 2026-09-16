# Prompt precision — repository evidence and reuse map

Inspected: 2026-09-15. Branch `0.0.38`; HEAD `994f417c8`; working tree contains substantial pre-existing edits. Findings refer to inspected working-tree source, not a clean release or executed test result. Existing changes were preserved.

## 1. Product call paths

### Local chat and named agents

[`chat_inference.rs`](../../../crates/qualia-client-core/src/chat_inference.rs): `run_chat_inference_for_agent` selects/binds a roster agent and model; `run_chat_inference_full` orchestrates the turn. `build_augmented_packet` joins capability briefing, semantic profile, ontology routing, cooperative context, attachments, retrieved facts, inforg context and user text.

[`context_binding.rs`](../../../crates/qualia-client-core/src/context_binding.rs): `build_inference_packet` creates `InferenceContextPacket` with an augmented string, graph JSON, scopes, namespaces, model/profile and axiom bounds. Despite its “stack-allocated” comment, its `String` and `Vec` fields are heap-backed cold construction.

[`decode.rs`](../../../crates/qualia-core-db/src/inference/inference_agent/decode.rs) calls `GgufTokenizer::encode_chat_prompt(&prompt_owned)`. [`tokenizer.rs`](../../../crates/qualia-core-db/src/inference/gguf_sharder/tokenizer.rs) implements that through `apply_chat_template(None, user)`.

**Implication:** reuse session/context selection, but introduce structured request parts before the concatenation. In the inspected named-local route, `AgentDefinition.system_prompt` is not applied; the semantic profile briefing is applied. The direct system-prompt dispatch found in client-core is the remote path. Add an integration test for this discrepancy before migration.

### Remote MCP agent

[`api/agents.rs`](../../../crates/qualia-client-core/src/api/agents.rs): `run_remote_agent_turn` checks agent enablement and remote consent/routing, then passes the roster system prompt and current prompt to `remote_mcp_infer`.

[`remote_mcp.rs`](../../../crates/qualia-client-core/src/remote_mcp.rs): `build_infer_request` concatenates system text and prompt into a single `prompt` argument to MCP `tools/call`; model is optional. It is a real transport path, not a native latent-input interface.

**Implication:** preserve consent and connection ownership; replace inference-tool assumptions with a schema/capability mapping. The inspected remote response returns `committed: true`, empty citations/provenance and zero timing/token counts after attempting a chat append. This does not establish native-equivalent validation, measured zero usage, or verified graph commit. Introduce distinct response/validation/persistence outcomes and nullable or coverage-labelled measurements.

### HTTP API and asynchronous native jobs

[`poet_llm_api.rs`](../../../crates/qualia-core-db/src/services/poet_llm_api.rs): `PoetLlmRequest`, input validation, and `run_local_turn` create a local agent, validate intent, infer, validate output and optionally post-verify.

[`poet_llm_jobs.rs`](../../../crates/qualia-core-db/src/services/poet_llm_jobs.rs) owns the job execution route, including controlled inference.

**Implication:** compile in core so both API variants can use the same contract without importing client-core. Preserve cancellation and synchronous/job result consistency.

### Additional entry points and compatibility

- [`llm_lifecycle.rs`](../../../crates/qualia-cli/src/llm_lifecycle.rs) and [`llm_testing.rs`](../../../crates/qualia-cli/src/llm_testing.rs) call native streaming inference. Distinguish product requests from deliberately raw benchmark inputs.
- [`studio_pane_llm.rs`](../../../crates/qualia-client-core/src/studio_pane_llm.rs) constructs a layout prompt and delegates into chat inference. It should inherit the shared compiler without a second implementation.
- [`inference_backend.rs`](../../../crates/qualia-client-core/src/inference_backend.rs) contains persisted Local/Remote/Hybrid/Ollama preferences. [`context_binding.rs`](../../../crates/qualia-client-core/src/context_binding.rs) explicitly supports the opt-in Ollama harness. Older root orientation statements claiming no such harness are stale for this checkout; native Qualia remains the primary engine.
- No general direct-provider message API abstraction was identified in the inspected dispatch paths. Provider credentials and MCP HTTP transport are not proof that a direct model API adapter already exists. Inventory remaining entry points mechanically during P0.

## 2. Reusable components and their limits

| Component and source | Observed implementation | Reuse and required extension |
|---|---|---|
| [`agent_registry.rs`](../../../crates/qualia-client-core/src/agent_registry.rs) | `AgentSemanticProfile` has ontology-addressed tags across classification, specialisation, geography, language, method, dataset, tool and constraint. Roster also holds context/data/execution policies and `system_prompt`. | Extend with a versioned conditioning reference; preserve legacy text and semantic IDs. Existing 32-tag briefing bounds are not a complete prompt budget. |
| [`ontology_router.rs`](../../../crates/qualia-client-core/src/ontology_router.rs) | `route_prompt_with_focus_and_allowlist` narrows routing using focus and allowed ontologies. | Preserve relevance/authority separation; attach a selection receipt and deterministic tie policy. |
| [`chat_retrieval.rs`](../../../crates/qualia-client-core/src/chat_retrieval.rs) | Retrieval bundles, graph citations, keyword selection and daemon query integration. | Feed source-labelled evidence parts, include qualifier/dependency closure, measure actual relevance and citation support. |
| [`nlp/graphrag.rs`](../../../crates/qualia-core-db/src/nlp/graphrag.rs) | Deterministic term-overlap index over string triples; heap-backed. | Cold retrieval candidate/baseline. It is not neural graph encoding or trained GraphRAG. |
| [`semantic_skills.rs`](../../../crates/qualia-core-db/src/inference/semantic_skills.rs) | Hash-based n-gram text embeddings and vector/scratchpad helpers. | Useful lightweight retrieval baseline. These vectors are not aligned model embeddings or valid learned soft prompts. |
| [`runtime/graph_assist/query.rs`](../../../crates/qualia-core-db/src/inference/runtime/graph_assist/query.rs) | `query_graph_into` selects into caller buffers with sensitivity, parity, fact-count checks and selection receipt. | Strong core seam; supply already authorised graph scope. Wildcard selection and sensitivity filtering alone are not ACL enforcement. |
| [`runtime/graph_assist/identity.rs`](../../../crates/qualia-core-db/src/inference/runtime/graph_assist/identity.rs) | Prefix identity mixes model, tokenizer, graph context/revision and ordered Quins into four words. | Add full conditioning/token/template/adaptation/authority identity. Current mixer is not a cryptographic attestation. |
| [`runtime/kv/prefix/store.rs`](../../../crates/qualia-core-db/src/inference/runtime/kv/prefix/store.rs) | Reference-counted prefix page store with registry ownership and attach/eviction operations. | Reuse actual page lifecycle; verify end-to-end cache admission and all backend consumers. Presence of this library does not prove product chat uses it. |
| [`runtime/scheduler/request_table.rs`](../../../crates/qualia-core-db/src/inference/runtime/scheduler/request_table.rs) | Request management integrates prefix KV attachment. | Carry immutable conditioning/cache handles through request ownership and cancellation. |
| [`decode_helpers.rs`](../../../crates/qualia-core-db/src/inference/inference_agent/decode_helpers.rs) | Legacy `PREFIX_CACHE` is a process-global `HashMap<u64, Box<[f32]>>`. | Audit separately from paged KV. Do not describe both as one uniform cache or assume bounded retention from the newer registry. |
| [`runtime/prepared/decode_plan.rs`](../../../crates/qualia-core-db/src/inference/runtime/prepared/decode_plan.rs) | `PreparedDecodePlan` defines caller-buffered decode with no allocation, compilation, discovery or backend switching. | Target contract for prepared conditioning. Capability claims require tests of concrete backend implementations. |
| [`tokenizer.rs`](../../../crates/qualia-core-db/src/inference/gguf_sharder/tokenizer.rs) | Family-specific chat templates and raw/chat encoders; some families fold system into user, unknown family returns user unchanged. | Add role-preserving input and explicit template capability/fallback receipt. Test BOS/EOS and special-token injection handling. |
| [`lora/adapter_manager.rs`](../../../crates/qualia-core-db/src/lora/adapter_manager.rs) | Real A/B matrix application, metadata, loading/saving and context switching; `apply_cpu` allocates rank scratch. | Reuse artifact/math foundations after caller-buffered scratch, output-shape checks, target mapping and compatibility work. No trained-domain quality claim. |
| [`inference_agent/decode.rs`](../../../crates/qualia-core-db/src/inference/inference_agent/decode.rs) | Optional context-detected adapter adds a delta to embeddings. Selected branch clones embedding data per step and ignores apply errors; dimension mismatch skips adaptation. | A correctness/performance prerequisite before learned-profile rollout: request-scoped validated adapter, no silent no-op, no hot allocation, verify that every selected resident/legacy backend consumes adaptation. |
| [`inference_agent/runtime.rs`](../../../crates/qualia-core-db/src/inference/inference_agent/runtime.rs) | Intent profile check, output provenance-presence and token-budget checks. | Preserve gates; add actual evidence-support verification. Non-empty provenance is weaker than a supported claim. |
| [`inference_agent/decode.rs`](../../../crates/qualia-core-db/src/inference/inference_agent/decode.rs) | Initial `prov_hash` derives from the first eight graph-context bytes. | Do not mistake this marker for a cryptographic source citation. Bind results to real source records and validator outcomes. |
| [`mcp_tool_loop.rs`](../../../crates/qualia-client-core/src/mcp_tool_loop.rs) | Gated MCP tool invocation; raw dispatcher documents that caller must have passed principal/allowlist checks. | Keep gating executable and carry immutable authority through every tool round. Model instructions are not tool permissions. |
| [`domino_gbnf.rs`](../../../crates/qualia-core-db/src/inference/domino_gbnf.rs), [`sampler.rs`](../../../crates/qualia-core-db/src/inference/sampler.rs) | Grammar-state/logit masking and constrained sampling support. | Reuse as a syntax-control seam. Audit supported grammar and fallback behaviour; do not advertise arbitrary schema correctness or factual validation. |
| [`inference_eval.rs`](../../../crates/qualia-core-db/src/inference/inference_eval.rs) | NLL, perplexity, KL and engineering-fidelity thresholds; explicitly distinguishes fidelity from truth. | Retain for local adaptation regressions. Add task-success metrics; a prompt can improve task accuracy while changing these distributions. |
| [`lab/auto_improve.rs`](../../../crates/qualia-core-db/src/inference/lab/auto_improve.rs) | Bounded config search, resampling/plateau tracking and output package for engine tuning. | Reuse experiment discipline; build a separate prompt/task search space. This is not an existing prompt optimiser. |
| [`runtime/receipt/execution.rs`](../../../crates/qualia-core-db/src/inference/runtime/receipt/execution.rs) | Execution counters with coverage bits distinguishing unknown from measured zero. | Add a linked conditioning/evaluation receipt without inflating the hot ABI. Apply the same unknown-vs-zero convention to remote tokens/cost. |
| [`runtime/artifacts/run_dir.rs`](../../../crates/qualia-core-db/src/inference/runtime/artifacts/run_dir.rs) | `RunArtifactDir` uses `TempDir`, marker ownership, byte budget and explicit retention. | Use for optimisation candidates/evidence; verify all producers charge the same budget and cleanup succeeds under cancellation/unwind. |

## 3. CBOR/semantic integration is useful but needs a specified profile

[`query/cbor_compiler.rs`](../../../crates/qualia-core-db/src/query/cbor_compiler.rs) exposes `parse_cbor_ld_to_quin`, but its inspected implementation assumes four dictionary integers. Its non-integer branch returns zero, and it is not a general strict decoder for conditioning documents.

Other implementations are more capable: [`sparql_library/parsers/cbor_parser.rs`](../../../crates/qualia-core-db/src/sparql_library/parsers/cbor_parser.rs) uses `minicbor`, supports RDF-Star tags and canonical term hashing; [`rdf_serializers.rs`](../../../crates/qualia-core-db/src/sparql_library/serialisers/rdf_serializers.rs) serialises CBOR graph data.

**Design consequence:** do not select a parser solely because its name says CBOR-LD. Define the conditioning interchange subset, dictionary semantics and canonical vectors, then select/extend the appropriate library. Require strict round trips, unsupported-input errors, full bounds and identity tests. CBOR encoding remains separate from model-facing text rendering and learned tensors.

## 4. Priority implementation gaps

1. **Uniform request semantics:** system/task/evidence boundaries and roster defaults must survive local, MCP and HTTP dispatch.
2. **Requirement accounting:** no shared requirement-ID-to-rendering-to-validator trace was found.
3. **Truthful outcomes:** normalise validation, persistence and measurement status across routes; do not infer verification from chat append.
4. **Model-aware budget and identity:** include actual templates/tokens, evidence, tools, output reserve and adaptation in a request-scoped prepared plan.
5. **Strict graph interchange:** choose a versioned codec with rejection and round-trip evidence.
6. **Portable task evaluation:** add domain tasks and independent scoring; reuse engine-fidelity measures only for their stated purpose.
7. **Learned-conditioning readiness:** eliminate observed hot allocations/silent adapter skipping and prove target/backend consumption before evaluating new tensors.

## 5. Inspection limits

This was a targeted architecture review, not an exhaustive security or runtime audit. Source and selected existing tests were inspected; no builds, models, external inference calls, training or performance tests were run. Statements about missing shared functionality mean it was not found in the searched call paths, not that no related code can exist elsewhere.

Root documentation references historical plans including `native-inference-runtime-renewal-2026-07-26.md`. Those files were absent from `docs/plans/` during this review. This design follows available code and the supplied invariants; it does not claim to have reconciled unavailable plans.

## 6. Wave 0 mechanical inventory (PP-001, 2026-09-16)

`rg` over `crates/` for product/diagnostic inference entry points (working tree). Classification:

### Product routes (conditioning target)

| Site | Path | Notes |
|---|---|---|
| Named local chat | `qualia-client-core/src/chat_inference.rs` — `run_chat_inference_for_agent`, `run_chat_inference_full`, `build_augmented_packet` | Passes `semantic_profile`; **does not** reference `system_prompt`. Concatenates capability/semantic/routing/…/user into one string. |
| Chat API | `qualia-client-core/src/api/chat.rs` | Delegates into chat inference. |
| Agent turn | `qualia-client-core/src/agent_turn_handler.rs` | Local vs remote dispatch. |
| Studio pane | `qualia-client-core/src/studio_pane_llm.rs` | Builds layout prompt → chat inference. |
| Remote MCP | `qualia-client-core/src/remote_mcp.rs` — `build_infer_request`, `remote_mcp_infer` | Flattens system+user into single `prompt` arg. |
| Remote roster turn | `qualia-client-core/src/api/agents.rs` — `run_remote_agent_turn` | Uses roster `system_prompt`; returns `committed:true`, empty citations, zero tokens/ms (unknown coverage). |
| HTTP sync | `qualia-core-db/src/services/poet_llm_api.rs` — `PoetLlmRequest`, `run_local_turn` | Legacy `prompt` (+ graph_context); no conditioning object. |
| HTTP jobs | `qualia-core-db/src/services/poet_llm_jobs.rs` | Same request type; cooperative cancel via `cancel_handler`. |
| Tokenisation | `gguf_sharder/tokenizer.rs` — `apply_chat_template`, `encode_chat_prompt` | `encode_chat_prompt` always passes `system=None`. Families: ChatMl, Llama3, Gemma, Gemma4, None. |
| Decode | `inference_agent/decode.rs` | Calls `encode_chat_prompt`; uses legacy `get_prefix_cache` / `PREFIX_CACHE`. |
| Orchestrator | `inference/orchestrator.rs` — `orchestrate_inference` | Intent/output gates. |

### Diagnostic / CLI / compatibility (raw vs product)

| Site | Path | Class |
|---|---|---|
| CLI lifecycle/testing | `qualia-cli/src/llm_lifecycle.rs`, `llm_testing.rs` | Diagnostic / streaming — treat as raw/bench unless they use chat template. |
| Local job scheduler | `qualia-client-core/src/local_job_scheduler.rs` | Product-adjacent job wrapper. |
| Ollama harness | `inference_backend.rs` + `context_binding.rs` | Opt-in compatibility; not primary engine. |
| Desktop QA/social | `webizen-desktop/src/commands/agent_qa.rs`, `social.rs` | Host UI entry — inherits client routes. |

### Prefix / KV reachability (PP-006)

| Mechanism | Reached by product chat decode? | Evidence |
|---|---|---|
| Legacy `PREFIX_CACHE` (`decode_helpers.rs`) | **Yes** | `decode.rs` calls `get_prefix_cache`. |
| Paged KV prefix store (`runtime/kv/paged`, `runtime/kv/prefix`) | **Not on product chat path** | No `PagedKv` / prefix-page attach in `decode.rs`; library used by runtime/scheduler tests and CUDA lanes. |

### Fixtures landed

- `crates/qualia-client-core/tests/prompt_precision_p0.rs` — PP-002/003/004
- `crates/qualia-core-db/tests/prompt_precision_p0.rs` — PP-002/005/006
