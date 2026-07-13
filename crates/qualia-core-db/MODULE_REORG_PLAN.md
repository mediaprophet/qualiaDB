# qualia-core-db `src/` reorganisation plan

**Problem:** `src/` has **137 loose `.rs` files (~66k lines)** at the crate root with no
sub-directory structure, ~20 of them monolithic (>500 lines). This is the §11
library-ization pass.

**Method (category-theoretic).** Treat each module as an *object* and each `use crate::X`
as a *morphism*. A good grouping is a quotient of this dependency category in which most
arrows are **internal** to a group (cohesion) and the few cross-group arrows form thin,
named interfaces (the "natural transformations"). Foundational objects (high in-degree —
depended on by many: `frame_layout`, `crdt`, `resolver`) sit at the base; API surfaces
(high out-degree, low in-degree: `mcp_server`, `daemon`, `wasm_bridge`) sit at the top.

**Safety.** Import discipline is overwhelmingly absolute (`crate::` 1197× vs top-level
`super::` ~15×), so file moves survive. Each move preserves the public path with a
`pub use cat::foo;` re-export in `lib.rs`, so **no other file changes**. Execute one
category per commit, with a native + wasm build gate each time. `⚑` = monolith to split
(in a later sub-phase, *after* the move — don't block categorisation on splitting).

---

## Target categories (loose file → home)

Existing sub-dirs already set precedent: `audio domains geometric_algebra gguf_bridge
gpu_context lora modalities obfuscation p2p render shaders solvers sparql_library
specialized_libs storage tensor`. New categories below; some loose files fold into
existing dirs.

### `inference/` — weight-tensor / LLM runtime (neighbours gguf_bridge, gpu_context, lora)
llm_agent⚑ llm_awq llm_bench⚑ llm_eval llm_gpu_profiler llm_kernel_parity gguf_parser
gguf_sharder⚑ gguf_tensor_index gguf_tokenizer ggml_quants⚑ safetensor tensor_roles
ternary ternary_gpu⚑ topk topk_gpu directml_bridge⚑ metal_bridge resident_model
residency_planner semantic_culler⚑ neuro_symbolic_sieve spatial_sieve agent orchestrator⚑
ambient_orchestration⚑ compute_universe⚑

### `q42/` — the .q42 volume/weight format family
q42_weight⚑ q42_volume⚑ q42_reader q42_lex q42_lexicon yaml_ld_q42 design_encode

### `mcp/` — Model-Context-Protocol server surface
mcp_server⚑ mcp_tool_impls⚑ mcp_stub_impls⚑ mcp_format_impls⚑ mcp_cooperation

### `wasm/` — browser/OPFS bindgen surface (wasm_bridge/ already split)
wasm_bridge_core wasm_edge wasm_llm wasm_playground⚑ wasm_storage spatial_wasm

### `daemon/` — graph daemon + relays (port 4242)
daemon⚑ daemon_graph daemon_query daemon_swarm⚑ daemon_tensor chat_relay_daemon
webizen_server⚑ rpc webtorrent_routes webtorrent_seeder solid_ldp ilp_dispatcher

### `net/` — transport / sensing / filtering (neighbours p2p/)
nym_adapter ebpf_filter⚑ ebpf_firewall⚑ acoustic_ble_mesh⚑ sonic_token host_topology

### `crypto/`
zk_proofs⚑ fiduciary_crypto⚑ sanctuary_crypto pq_kem_shim deontic_circuit
verifiable_credential

### `identity/`
agency identifier profiles key_vault⚑ webizen_identifiers vault_manifest

### `governance/` — Webizen VM + deontic/epistemic core (neighbours modalities/)
webizen⚑(3028) webizen_bytecode webizen_sync webizen_validator web_civics deontic_logic⚑
deontic_mapping epistemic illocution modal_kind provenance

### `medical/` — (or fold under domains/)
clinical_engine⚑ comorbidity_eval dicom⚑ dicom_ingest⚑

### `query/` — RDF / SPARQL / parse / index (neighbours sparql_library/)
query_compiler query_engine rdf_star shacl_compiler cbor_compiler resolve resolver⚑
lexicon ontology_loader mini_parser ingest ingestion graph_index indexing temporal_graph

### `platform/` — HW / FFI / scheduling
jni_bridge npu_ffi tee_ffi git_bridge kml_bridge⚑ hardware_passport device_benchmark
platform_scheduler local_scheduler⚑

### → `storage/` (exists)
csd_storage⚑ zns_storage⚑ storage storage_driver⚑ wal archive sync

### → `solvers/` (exists)
ode_solver quantum_dft qubo_compiler qpu_ingress

### `extensions/`
extension_bus extension_manifest resource_catalog⚑

### `core/` — foundational ABI (base of the dependency order)
frame_layout crdt telemetry fuzz_testing topology_draft

---

## ⚑ Genuine ambiguities — Timothy's architectural call

These objects have arrows into two categories; the boundary is a design decision, not a
mechanical one:

1. **`deontic_logic` / `epistemic` / `modal_kind`** → `governance/` or the existing
   `modalities/`? (They are reasoning modalities *and* governance primitives.)
2. **`resolver` / `resolve`** → `query/` or `core/`? (Foundational, high in-degree.)
3. **`compute_universe` / `semantic_culler`** → `inference/` or `core/`?
4. Should existing `gguf_bridge/`, `gpu_context/`, `lora/` **nest under `inference/`**, or
   stay top-level siblings? (Nesting is cleaner but a larger path change.)
5. Naming: `inference/` — the de-buzzword plan flags "llm" as contested; `inference/` is my
   pick over `llm/`. Confirm or rename.

## Execution order (lowest-risk first)
1. `mcp/` (5 files, perfectly cohesive — proof of method)
2. `q42/`, `crypto/`, `extensions/`, `medical/` (small, clean)
3. fold into existing `storage/`, `solvers/`
4. `inference/`, `query/`, `daemon/`, `net/`, `platform/`, `identity/`, `governance/`
5. **Monolith-split sub-phase** (the ⚑ files), each `foo.rs → foo/{mod,…}.rs` in its new home.

Two monoliths already done this pass: `gguf_bridge/mc8_wasm.rs` (1253→4 modules),
`wasm_bridge.rs` (1541→8 domain modules).
