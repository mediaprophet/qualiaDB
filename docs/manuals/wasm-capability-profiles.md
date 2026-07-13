# Qualia WebAssembly capability profiles

The WASM builds are deliberately separate products. A profile must not expose a
library merely because the native engine has it.

The compile-time source of truth is
`crates/qualia-core-db/src/wasm_capabilities.rs`.

| Product | Cargo selection | Intended use | Included | Explicitly excluded |
|---|---|---|---|---|
| Ontology MCP | `-p webizen-lite-wasm` | Read-only ontology sites such as `ns.webcivics.net` | MCP JSON-RPC, N3 inspection, bounded Quin query, SHACL property validation, deontic, epistemic, paraconsistent, LTL, DL, ASP/linear kernels, governance mapping | Portal, WebGPU, science, LLM, daemon, network, filesystem storage |
| Portal | `qualia-core-db --no-default-features --features portal` | Spatial/phenomenal pages | JSON/CBOR ingest, 10D tensor, spatial encoding, WebGPU viewport, AcousticPlane | Logic bridge, science, LLM |
| Logic | `qualia-core-db --no-default-features --features wasm-logic` | RDF/rule demos and the larger browser reasoning API | N3/Turtle, RDF serialization, bytecode query, numeric SHACL, modal logic, LWW CRDT | Scientific domain libraries, LLM |
| Scientific | `qualia-core-db --no-default-features --features wasm-scientific` | Browser scientific playground | Logic surface plus WASM-safe bioinformatics, clinical, chemistry, economics, symbolic/numerical solvers, control, GA and DFT | LLM |
| LLM | `qualia-core-db --no-default-features --features wasm-llm` | Browser model runtime | Logic + scientific prerequisites, GGUF/Q42 model loading, WebGPU inference, streaming decode | Portal |
| Full playground | `qualia-core-db --no-default-features --features wasm-full` | API explorer and local development | Portal + logic + scientific + LLM + playground exports | Native daemon/network/filesystem-only facilities |

## Ontology MCP contract

`webizen-lite-wasm` exports only `mcp_jsonrpc(message)` and `version()`. Its MCP
tool catalog currently contains:

- `ontology_capabilities`
- `hash_iri`
- `parse_n3`
- `query_quins`
- `validate_shacl`
- `evaluate_deontic`
- `evaluate_epistemic`
- `route_paraconsistent`
- `evaluate_ltl`
- `check_subsumption`
- `deontic_govern`

All Quin `u64` values may be supplied as decimal or `0x` strings. Results use
decimal strings so JavaScript never truncates values above `2^53 - 1`.

Inputs are bounded to 4,096 Quins, query responses to 512 Quins, and N3
responses to 512 events. The evaluator kernels retain caller-buffer APIs; heap
allocation occurs only at the JSON/MCP boundary.

## Build verification

```powershell
cargo check --target wasm32-unknown-unknown -p webizen-lite-wasm
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features portal
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-logic
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-scientific
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-llm
cargo check --target wasm32-unknown-unknown -p qualia-core-db --no-default-features --features wasm-full
```

Build the ontology package:

```powershell
wasm-pack build crates/webizen-lite-wasm --target web --out-dir pkg --release
```

The 2026-06-27 reference build is 267,993 bytes raw and 94,971 bytes gzip.
CI limits it to 512 KiB raw / 200 KiB gzip.
