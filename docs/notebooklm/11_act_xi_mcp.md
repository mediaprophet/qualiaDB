# Act XI — MCP

> *The seam between the engine and the outside world.*

---

## Thesis

> **The engine exposes its capabilities through the Model Context Protocol.
> The protocol is the seam. The seam is gated. The gate refuses
> unverified callers. The gate refuses ungrounded callers. The gate
> refuses non-derogable violations. The gate is the cooperation layer.**

---

## Voice-over script

### Shot 1 — A diagram: the engine in the center, surrounded by callers. [SLOW]

> The engine is in the center. [PAUSE]
> The callers are around it. [PAUSE]
> The seam is the Model Context Protocol. [PAUSE]

### Shot 2 — A caller without a verified DID approaches the seam. The seam denies. [SLOW]

> A caller without a verified DID approaches the seam. [PAUSE]
> The seam denies. [PAUSE]
> The denial is a Quin. [PAUSE]
> The denial is signed. [PAUSE]

### Shot 3 — A caller with a verified DID but no grounding principal approaches the seam. The seam denies. [SLOW]

> A caller with a verified DID but no grounding principal approaches
> the seam. [PAUSE]
> The seam denies. [PAUSE]
> The denial is a Quin. [PAUSE]
> The denial is signed. [PAUSE]

### Shot 4 — A caller with a verified, grounded DID approaches the seam. The seam permits. [SLOW]

> A caller with a verified, grounded DID approaches the seam. [PAUSE]
> The seam permits. [PAUSE]
> The permission is a Quin. [PAUSE]
> The permission is signed. [PAUSE]

### Shot 5 — The caller requests an action that violates a non-derogable norm. The seam blocks. [SLOW]

> The caller requests an action. [PAUSE]
> The action violates a non-derogable norm. [PAUSE]
> The seam blocks. [PAUSE]
> The block is a Quin. [PAUSE]
> The block is signed. [PAUSE]

### Shot 6 — The MCP server is shown. The stable tool catalog is listed. [ITEM]

> The MCP server has a stable tool catalog. [PAUSE] [ITEM]
> `system_resource` — telemetry, health. [PAUSE] [ITEM]
> `quin_*` — query, insert, update, delete Quins. [PAUSE] [ITEM]
> `matrix_*` — multiply, determinant, eigenvalues. [PAUSE] [ITEM]
> `algebra_*` — solve polynomial, analyze matrix. [PAUSE] [ITEM]
> `evaluate_*` — deontic, epistemic, paraconsistent, LTL, ASP, DL. [PAUSE] [ITEM]
> `solve_*` — QUBO, VQE, ODE, tensor. [PAUSE] [ITEM]
> `science_*` — chemistry, biology, thermodynamics. [PAUSE] [ITEM]
> `governance_*` — propose agreement, ratify, deny. [PAUSE] [ITEM]
> `qapp_*` — list, install, update, describe. [END LIST] [PAUSE]

### Shot 7 — The browser-local ontology MCP is shown. The eleven bounded tools are listed. [ITEM]

> The browser-local ontology MCP has eleven bounded tools. [PAUSE] [ITEM]
> `hash_iri` — FNV-one-a hash of an IRI. [PAUSE] [ITEM]
> `parse_n3` — parse an N3 rule. [PAUSE] [ITEM]
> `query_quins` — query the local Quin store. [PAUSE] [ITEM]
> `validate_shacl` — validate a SHACL shape. [PAUSE] [ITEM]
> `evaluate_deontic` — evaluate a deontic contract. [PAUSE] [ITEM]
> `evaluate_epistemic` — evaluate an epistemic claim. [PAUSE] [ITEM]
> `route_paraconsistent` — route a contradiction. [PAUSE] [ITEM]
> `evaluate_ltl` — evaluate an LTL formula. [PAUSE] [ITEM]
> `check_subsumption` — check DL subsumption. [PAUSE] [ITEM]
> `ontology_capabilities` — list the capabilities. [PAUSE] [ITEM]
> `mcp_jsonrpc` — the JSON-RPC entry point. [END LIST] [PAUSE]
> It runs in the browser. It does not phone home. [PAUSE]

### Shot 8 — The cooperation gate is shown. The four states: denied, denied, permitted, blocked. [SLOW]

> The cooperation gate has four states. [PAUSE]
> Denied — unverified caller. [PAUSE]
> Denied — ungrounded caller. [PAUSE]
> Permitted — verified, grounded caller. [PAUSE]
> Blocked — non-derogable violation. [PAUSE]

### Shot 9 — Title card: **The seam is gated.** [SLOW]

> The seam is gated. [PAUSE]
> The gate is the cooperation layer. [PAUSE]
> The cooperation layer is the engine. [PAUSE]

---

## On-screen notes

- **Shot 1:** A diagram. The engine is a circle in the center. The callers are smaller circles around it. The seam is the boundary.
- **Shot 2:** A caller approaches. The seam flashes red. The denial is a Quin.
- **Shot 3:** A caller approaches. The seam flashes red. The denial is a Quin.
- **Shot 4:** A caller approaches. The seam flashes green. The permission is a Quin.
- **Shot 5:** A caller approaches. The seam flashes red. The block is a Quin.
- **Shot 6:** The MCP server. The tool catalog is listed.
- **Shot 7:** The browser-local ontology MCP. The eleven tools are listed.
- **Shot 8:** The cooperation gate. The four states are shown.
- **Shot 9:** Title card.

---

## Source code anchors

- `crates/qualia-core-db/src/mcp/mcp_server.rs` — `McpRuntimeState`, `McpIntentFrame`, `RawToolPayload`, `McpToolDescriptor`, `stable_mcp_tools`, `tool_list_json`, `system_resource_json`.
- `crates/qualia-core-db/src/mcp/mcp_cooperation.rs` — `CallerStandpoint`, `CooperationVerdict`, `caller_grounded`, `authorize`, `authorize_call`, `enforcement_enabled`, `unverified_caller_is_denied`, `ungrounded_caller_is_denied`, `verified_grounded_ordinary_call_is_authorized`, `non_derogable_violation_request_is_blocked_by_policy`.
- `crates/qualia-core-db/src/mcp/mcp_tool_impls.rs` — `list_capabilities`, `matrix_operation`, `algebra_solve_polynomial`, `algebra_matrix_analyze`, `evaluate_*`, `solve_*`, `science_*`.
- `crates/qualia-core-db/src/mcp/mcp_format_impls.rs` — `quin_to_json`, `apply_context`, `resolve_predicate_hash`.
- `crates/qualia-core-db/src/mcp/mcp_stub_impls.rs` — `quins_to_ntriples`, `query_sparql`, `get_graph_stats`, `list_ontologies`, `llm_infer`, `llm_chat`.
- `crates/qualia-core-db/src/mcp/mod.rs` — the MCP module root.
- `crates/webizen-lite-wasm/src/lib.rs` — the browser-local ontology MCP, eleven bounded tools.
- `crates/qualia-cli/src/mcp.rs` — `McpTransport`, `McpAction`, `serve_tcp`, `handle_tcp_client`, `start_background`, `stop_background`, `print_status`, `print_doctor`, `ping_service`, `send_request`.
- `AGENTS.md §2-B` — the MCP cooperation layer is the gating mechanism.

---

## Duration

Approximately 90 seconds. This is the act where the viewer learns that the engine meets the world on its own terms.
