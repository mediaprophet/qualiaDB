# QualiaDB MCP Surface Audit

**Date:** 2026-06-17 (rev 2026-06-21 — algebra/CAS tools)  
**Source of truth:** `crates/qualia-core-db/src/mcp_server.rs`  
**Server identity:** `qualia-core-db-mcp` (protocol `2025-03-26`)  
**CLI entry:** `qualia-cli mcp serve` (stdio or TCP `:4244`)

> **Update (2026-06-21):** three new **fully caller-driven** tools were added —
> `algebra_solve_polynomial`, `algebra_matrix_analyze` (determinant / eigenvalues /
> eigen_symmetric / svd), and `cas` (symbolic differentiate / simplify / expand / evaluate
> / solve_quadratic / factor). Unlike most existing public tools (key finding #3), these run
> on **caller-supplied** data, not fixed demo inputs, so they add to the "fully
> production-ready" set. The per-layer counts below are the **2026-06-17 snapshot** and have
> since drifted — re-derive from `mcp_server.rs` (the source of truth) before quoting exact
> numbers. See `ALGEBRA_MANIFOLD_PLAN.md` and `docs/manuals/standards/q42-symbolic-algebra-encoding.md`.

---

## Executive summary

The Qualia MCP server is **not** a full-fidelity API for QualiaDB. It is a **curated agent harness** with three layers that do not fully align (counts = 2026-06-17 snapshot):

| Layer | Count | Role |
|-------|------:|------|
| **`tools/list` (public)** | 24 (+3) | What IDEs and agents see when they discover capabilities |
| **Dispatch handlers** | 41 (+3) | What the server will accept if you call a tool by name |
| **Fully production-ready** | ~6 (+3) | Tools that perform real, caller-driven work end-to-end |

**Key findings:**

1. **17 dispatch-only tools** exist in `enforce_fiduciary_tool_dispatch` but are **omitted** from `stable_mcp_tools()` / `tools/list`.
2. **17 handlers** return `ToolNotReady` (`tool_not_ready()` stub) immediately — all are hidden dispatch-only tools.
3. **Most “working” public tools** are **diagnostic demos**: they call real libraries but with **fixed inputs** (2×2 matrices, demo DNA strings, default Framingham vitals, empty Quin slices) rather than caller-supplied problem data.
4. **Major engine surfaces** (daemon HTTP `/query`, Phase 8 LLM, WAL/governance, ingest CLI, 40+ `SlgOpcode::Native*` VM ops) have **no MCP tool**.
5. **`query_graph`** is advertised but **blocked by default** unless `sanctuary_override` is supplied in arguments (Sanctuary gate).

**Bottom line:** MCP is suitable for **health checks, modality smoke tests, specialized-library probes, and orchestrating `docs/tests`**. It is **not** a substitute for `qualia-cli`, the loopback daemon, or in-process Webizen VM execution.

---

## Root cause: two registries, one dispatcher

The mismatch is structural, not accidental. `mcp_server.rs` maintains **two separate registries**:

| Registry | Function | Lines (approx.) | Purpose |
|----------|----------|-----------------|---------|
| **Public catalog** | `stable_mcp_tools()` | 85–207 | Feeds `tools/list` — what IDEs discover |
| **Dispatch router** | `enforce_fiduciary_tool_dispatch()` | 211–351 | Accepts any known tool name on `tools/call` |

Public tools are a **curated subset** of dispatch handlers. The 17 hidden tools were wired into dispatch (likely for future CLI/IDE parity or legacy callers) but deliberately **excluded** from `stable_mcp_tools()` because they only call `tool_not_ready()`:

```356:361:crates/qualia-core-db/src/mcp_server.rs
unsafe fn execute_sparql_query(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<String, McpSystemError> {
    tool_not_ready()
}
```

Meanwhile, several **public** tools invoke real libraries but ignore most caller arguments — they are smoke-test harnesses, not production APIs:

```1056:1065:crates/qualia-core-db/src/mcp_server.rs
    let demo_q = b"ATCGATCG";
    let demo_t = b"ATCGATCC";
    let result = if mode == b"protein" {
        align_protein(demo_q, demo_t)
    } else {
        align_nucleotide(demo_q, demo_t)
    };
```

The static Qapp schema tool **advertises** hidden stub tools in its JSON payload, creating a second discovery path that contradicts `tools/list`:

```876:886:crates/qualia-core-db/src/mcp_server.rs
    let schema = r#"{
  ...
  "mcp_tools": ["list_qapps", "get_qapp_manifest", "inspect_qapp_readiness", "list_qapp_updates", "describe_qapp_surface_schema"]
}"#;
```

---

## Architecture

### Transport

| Mode | Command | Notes |
|------|---------|-------|
| Stdio | `qualia-cli mcp serve --transport stdio` | Default for IDE MCP clients |
| TCP | `qualia-cli mcp serve --transport tcp --bind 127.0.0.1:4244` | Line-delimited JSON-RPC |
| Background | `qualia-cli mcp start` / `qualia-cli service start` | PID file under `.qualia/run/` |

### JSON-RPC methods

| Method | Supported |
|--------|-----------|
| `initialize` | Yes |
| `ping` | Yes |
| `tools/list` | Yes — returns **only** `stable_mcp_tools()` |
| `tools/call` | Yes |
| `resources/list` | Yes — one static resource |
| `resources/read` | Yes — `qualia://qapp-surface-schema` only |
| `notifications/initialized` | Ignored (no response) |

No `prompts/*` surface. No dynamic `tools/listChanged` (capability flag is `false`).

### Intent frame (per `tools/call`)

Built in `build_intent_frame()` from JSON arguments and server flags:

| Field | Source |
|-------|--------|
| `purpose_hash` | Fixed: `q_hash("purpose:General")` |
| `sanctuary_override` | Optional JSON string `sanctuary_override` |
| `qpu_enabled` | `qualia-cli --enable-qpu` or TCP defaults |
| `llm_enabled` | `true` for stdio/TCP serve (configurable via `start_mcp_listener_with_flags`) |
| `active_deontic_constraints` | Always empty `Vec` |
| `active_profile_id` | Always `None` |

### Error codes (`McpSystemError`)

| Code | JSON-RPC | Meaning |
|------|----------|---------|
| `SanctuaryGateTriggered` | -32001 | `query_graph` without `sanctuary_override` |
| `IntentFrameViolation` | -32002 | Reserved |
| `FeatureNotEnabled` | -32003 | LLM/QPU tool when flag off |
| `ToolNotReady` | -32004 | Stub implementation |
| `ToolNotFound` | -32601 | Unknown tool name |
| `InvalidParameters` | -32602 | Bad/missing args |
| `ParseError` | -32700 | JSON/parse failure |

---

## Public tools (`tools/list`) — 24 entries

These are the **only** tools returned by `tools/list`. Status reflects **actual handler behavior**, not the marketing description.

### Graph & system

| Tool | Status | Behavior |
|------|--------|----------|
| `query_graph` | **Gated / partial** | Requires `sanctuary_override`. Runs `SlgArena::fire_registered_rules` and returns a rule-fire count string — **not** a SPARQL/N-Triples graph query against the daemon store. |
| `get_system_status` | **Real** | Returns JSON: server name, version, protocol, tool count, `qpuEnabled`, `llmEnabled`. |
| `describe_qapp_surface_schema` | **Static** | Returns hard-coded JSON schema text; does not introspect installed Qapps. |
| `inject_test_quin` | **Real** | Builds a test `NQuin`, routes through `paraconsistent::route_paraconsistent`, appends isolated quins to WAL. |
| `run_docs_tests` | **Real** | Spawns `node docs/tests/run-headless.mjs`; checks daemon `/health` for `native`/`both` modes. |

### Modalities & logic

| Tool | Status | Behavior |
|------|--------|----------|
| `evaluate_modality` | **Partial** | `ltl`, `asp`, `probabilistic`, `argumentation` run real evaluators on **empty/demo inputs**. All other modality strings return `"0"`. |
| `symbolic_logic_infer` | **Diagnostic** | Runs defeasible forward-chaining or bounded SAT with **built-in demo facts/rules**. |
| `geometric_algebra_op` | **Diagnostic** | Fixed vectors `[1,0,0]` × `[0,1,0]`; `cross` or `angle`. |

### Specialized libraries (9 tools)

All invoke real `specialized_libs::*` types but with **canned parameters** (not caller-supplied tensors/molecules/portfolios):

| Tool | Library | Demo behavior |
|------|---------|---------------|
| `matrix_operation` | `LinearAlgebraLibrary` | Fixed 2×2 `A`, `B`; `multiply` / `transpose` / `solve`. |
| `ode_solve` | `PhysicsSimulationLibrary` | 10×10 CFD Burgers step; ignores meaningful caller ODE. |
| `chemical_analysis` | `ChemistryModelingLibrary` | Empty `Molecule::new()` boiling-point prediction. |
| `statistical_analysis` | `StatisticalComputingLibrary` | Fixed 5-row dataset; `mean` / `variance` / `correlation`. |
| `ml_inference` | `MachineLearningLibrary` | Loads model from **`/dev/null`** (fails on Windows); dummy inference params. |
| `financial_model` | `FinancialModelingLibrary` | Default `OptionParameters` or empty portfolio risk. |
| `medical_score` | `MedicalComputingLibrary` | `analyze_clinical_data("patient_1", …)` with enum switch only. |
| `engineering_analysis_op` | `EngineeringAnalysisLibrary` | Default `EngineeringModel::new()` structural/thermal/dynamic. |

### Domain science (CLI-adjacent)

| Tool | Status | Behavior |
|------|--------|----------|
| `bioinformatics_align` | **Diagnostic** | **Ignores** `query`/`target` JSON fields; aligns hard-coded `ATCGATCG` vs `ATCGATCC`. |
| `chemical_descriptors` | **Partial** | Parses caller `smiles` (defaults to `"C"`); returns molecular weight as integer string. |
| `clinical_risk` | **Partial** | Only `framingham` branch is real (fixed 55yo male demo vitals). Other scores return `"0"`. |

### Data formats

| Tool | Status | Behavior |
|------|--------|----------|
| `parse_csv` | **Real (file I/O)** | Opens caller `file_path`; uses **empty** `CsvMappingProfile` (no column maps). |
| `parse_json` | **Real (file I/O)** | Same pattern with empty `JsonMappingProfile`. |
| `serialize_csv` | **Real (file I/O)** | Writes **empty** Quin slice `&[]` to caller path. |
| `serialize_json` | **Real (file I/O)** | Same — empty Quin slice. |
| `serialize_rdf` | **Real (file I/O)** | Serializes **empty** Quin slice to NT/Turtle/N-Quads/TriG/N3/JSON-LD. |

---

## Hidden dispatch tools (17 — not in `tools/list`)

Callable via raw `tools/call` if the client knows the name; **invisible** to standard MCP discovery.

| Tool | Status | Gating |
|------|--------|--------|
| `query_sparql` | Stub | — |
| `get_graph_stats` | Stub | — |
| `list_ontologies` | Stub | — |
| `llm_infer` | Stub | `llm_enabled` |
| `llm_chat` | Stub | `llm_enabled` |
| `list_models` | Stub | — |
| `qpu_optimize` | Stub | `qpu_enabled` |
| `qpu_dft` | Stub | `qpu_enabled` |
| `qpu_status` | Stub | — |
| `get_wallet_status` | Stub | — |
| `get_did_info` | Stub | — |
| `ingest_ontology` | Stub | — |
| `validate_shacl` | Stub | — |
| `list_qapps` | Stub | — |
| `get_qapp_manifest` | Stub* | Validates `qapp_name` then stub |
| `inspect_qapp_readiness` | Stub* | Validates `qapp_name` then stub |
| `list_qapp_updates` | Stub | — |

\* Parses parameters, then returns `ToolNotReady`.

**Note:** `describe_qapp_surface_schema`’s static JSON **mentions** the hidden Qapp tools (`list_qapps`, etc.) even though those tools are stubs and not in `tools/list`.

---

## Implementation taxonomy

| Category | Count | Tools |
|----------|------:|-------|
| **Stub** (`tool_not_ready`) | 17 | All hidden graph/LLM/QPU/wallet/ontology/Qapp tools |
| **Gated** | 1 public | `query_graph` (Sanctuary) |
| **Static JSON** | 1 | `describe_qapp_surface_schema` |
| **Diagnostic demo** | 14 | Specialized libs + bioinformatics + symbolic_logic + geometric_algebra + most `evaluate_modality` |
| **Partial real** | 3 | `chemical_descriptors`, `clinical_risk` (framingham only), `evaluate_modality` (subset) |
| **Real I/O** | 6 | parse/serialize × 4, `inject_test_quin`, `run_docs_tests` |
| **Real metadata** | 1 | `get_system_status` |

---

## MCP resources

| URI | Content |
|-----|---------|
| `qualia://qapp-surface-schema` | Same static JSON as `describe_qapp_surface_schema` tool |

No graph snapshots, ontology manifests, GGUF indices, or WAL excerpts are exposed as MCP resources.

---

## Coverage gap: QualiaDB vs MCP

### Surfaces with **no** MCP tool

| Capability | Primary API today |
|------------|-------------------|
| **Loopback daemon** | `qualia-cli daemon` → HTTP `:4242` `/query`, `/health`, WS `/qualia-bridge` |
| **N-Triples / JSON-LD query** | Daemon `POST /query` (not MCP `query_graph`) |
| **Phase 8 LLM inference** | `llm_agent.rs`, `orchestrator.rs` — not `llm_infer`/`llm_chat` stubs |
| **GGUF lifecycle** | `qualia-cli llm *` |
| **Ingest pipelines** | `qualia-cli ingest`, `ExternalSorter`, `.q42` writer |
| **SHACL compile/validate** | `shacl_compiler.rs`, `qualia-cli shacl` — stub `validate_shacl` only |
| **Deontic / epistemic eval** | `qualia-cli evaluate deontic|epistemic`, `evaluate_modality` partial |
| **Governance / WAL / CRDT** | `qualia-cli governance`, `wal.rs`, `crdt.rs` |
| **Vault / profiles / DID** | `qualia-cli vault`, `profile`, `key_vault.rs` — wallet/DID stubs |
| **QPU dispatch** | `qualia-cli qpu`, daemon oracle — QPU stubs |
| **Webizen `SlgOpcode::Native*`** | 40+ wired opcodes in `webizen.rs::execute_vm_frame` (bio, clinical, chem, physics, economics) — only loosely mirrored by diagnostic MCP lib tools |
| **Storage / mesh / torrent** | `storage_driver.rs`, `webtorrent_*`, `acoustic_ble_mesh.rs` |
| **Flutter FRB / client** | `qualia-flutter`, `qualia-client-core` |
| **MCP server (meta)** | No `daemon_query`, `bench_load`, or `service status` tools |

### `qualia-cli` commands without MCP equivalent

`extension`, `governance`, `compile`, `llm`, `vault`, `migrate`, `ingest`, `query`, `compress`, `resources`, `profile`, `qpu`, `solve`, `science`, `benchmark`, `webizen`, `export-solid`, `daemon`, `service`, and the full `evaluate` modality subcommand tree.

### Specialized libraries

Nine active libraries under `specialized_libs/` (79 tests per project docs). MCP exposes **7 diagnostic wrappers** (matrix, ODE, chemistry, stats, ML, finance, medical, engineering) — not the full API surface of each library (e.g. no sparse matrices, no caller-supplied portfolios, no HIPAA-grade clinical inputs).

---

## Security & governance model

1. **Sanctuary gate** on `query_graph` writes a conduct-violation Quin to WAL and returns `-32001` without override.
2. **Feature flags** disable LLM/QPU tool branches at dispatch (stubs would run even if enabled today).
3. **Fiduciary framing** (`McpIntentFrame`, `enforce_fiduciary_tool_dispatch`) is structural; most tools ignore intent constraints beyond sanctuary/QPU/LLM flags.
4. **No authentication** on MCP TCP bind (localhost only by default) — relies on network isolation.

---

## Descriptor drift (`mcps/qualia/`)

The workspace folder `mcps/qualia/tools/` contains **4** JSON descriptors (`get_system_status`, `run_docs_tests`, `evaluate_modality`, `query_graph`). This is a **subset** of the 24 advertised tools and does not include hidden dispatch tools.

---

## Recommendations

### Short term (discovery honesty)

1. **Unify lists** — Either add all 41 dispatch names to `tools/list` with `status: stub|beta|production`, or remove stub handlers from dispatch to avoid “secret” tools.
2. **Mark diagnostics** — Prefix descriptions with `[diagnostic]` where inputs are canned.
3. **Fix `ml_inference`** — Replace `/dev/null` model path with in-memory fixture; gate Windows.
4. **Document `query_graph`** — Clarify it is not daemon `/query`; requires `sanctuary_override`.

### Medium term (high-value MCP tools)

| Priority | Tool | Bridges to |
|----------|------|------------|
| P0 | `daemon_query` | `POST /query` JSON-LD / N-Triples |
| P0 | `daemon_health` | `/health` + graph quin count |
| P1 | `evaluate_deontic` / `evaluate_epistemic` | `deontic_logic.rs`, `modalities/epistemic.rs` |
| P1 | `validate_shacl` (real) | `shacl_compiler.rs` |
| P1 | `list_models` / `llm_infer` (real) | `llm_agent.rs` + resident GGUF |
| P2 | `ingest_ntriples` | Ingest pipeline |
| P2 | `sparql_query` | `daemon_query` / future SPARQL layer |

### Long term

Expose **Webizen VM opcode dispatch** as a structured MCP tool (`execute_native_opcode`) for scientific parity with WASM tests, or generate MCP tools from the `SlgOpcode` enum automatically.

---

## Appendix A — Full dispatch map

```
PUBLIC + HANDLER
  query_graph          → gated partial (SlgArena)
  get_system_status    → real
  describe_qapp_surface_schema → static JSON
  inject_test_quin     → real
  evaluate_modality    → partial
  matrix_operation     → diagnostic
  ode_solve            → diagnostic
  chemical_analysis    → diagnostic
  statistical_analysis → diagnostic
  ml_inference         → diagnostic (fragile)
  financial_model      → diagnostic
  medical_score        → diagnostic
  engineering_analysis_op → diagnostic
  bioinformatics_align → diagnostic
  chemical_descriptors → partial
  clinical_risk        → partial
  parse_csv            → real I/O
  parse_json           → real I/O
  serialize_csv        → real I/O
  serialize_json       → real I/O
  serialize_rdf        → real I/O
  symbolic_logic_infer → diagnostic
  geometric_algebra_op → diagnostic
  run_docs_tests       → real (subprocess)

HIDDEN + STUB
  query_sparql, get_graph_stats, list_ontologies
  llm_infer, llm_chat, list_models
  qpu_optimize, qpu_dft, qpu_status
  get_wallet_status, get_did_info
  ingest_ontology, validate_shacl
  list_qapps, get_qapp_manifest, inspect_qapp_readiness, list_qapp_updates
```

---

## Appendix B — How to verify locally

```powershell
# Unit tests (no TCP needed)
cargo test -p qualia-core-db mcp_server --lib

# Start MCP (TCP)
qualia-cli service start
qualia-cli mcp doctor

# List advertised tools (expect 24)
node scripts/mcp-call.mjs tools/list

# Probe a hidden stub — expect error code -32004
node -e "
const net=require('net');const s=net.connect(4244,'127.0.0.1',()=>{
  s.write(JSON.stringify({jsonrpc:'2.0',id:1,method:'tools/call',
    params:{name:'validate_shacl',arguments:{}}})+'\n');
});
let b='';s.on('data',c=>{b+=c;if(b.includes('\n')){console.log(b);s.end()}});
"

# Run test orchestration tool
$env:QUALIA_MCP_ARGS='{"mode":"logic"}'
node scripts/mcp-call.mjs tools/call run_docs_tests
```

### Verification performed (2026-06-17)

| Check | Result |
|-------|--------|
| `cargo test -p qualia-core-db mcp_server --lib` | 3/3 passed |
| `tool_not_ready()` call sites in `mcp_server.rs` | 17 (lines 360–869) |
| Dispatch match arms in `enforce_fiduciary_tool_dispatch` | 41 |
| Public entries in `stable_mcp_tools()` | 24 |
| Live TCP probe (`127.0.0.1:4244`) | Not run — service not listening at audit time |

---

*Generated from static analysis of `mcp_server.rs` (≈1728 lines). Re-run this audit when `stable_mcp_tools()` or `enforce_fiduciary_tool_dispatch` changes.*