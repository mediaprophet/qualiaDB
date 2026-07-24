# webizen-lite-wasm

The browser-local Qualia MCP endpoint for read-only ontology sites such as
`ns.webcivics.net`.

This crate uses `qualia-core-db`'s isolated `wasm-ontology` profile. It does not
compile the portal/WebGPU feature, scientific bundle, LLM runtime, native
daemon, network stack, or filesystem storage into the published WASM.

## Build

```bash
wasm-pack build crates/webizen-lite-wasm --target web --out-dir pkg --release
```

Reference release size: 267,993 bytes raw / 94,971 bytes gzip.

## JavaScript

```js
import init, { mcp_jsonrpc, version } from "./pkg/webizen_lite_wasm.js";

await init();

const response = JSON.parse(mcp_jsonrpc(JSON.stringify({
  jsonrpc: "2.0",
  id: 1,
  method: "tools/list"
})));
```

The bridge implements MCP `initialize`, `notifications/initialized`, `ping`,
`tools/list`, and `tools/call`. It negotiates the stable protocol versions
`2025-11-25`, `2025-06-18`, and `2025-03-26`.

Available tools:

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
- `namespace_discovery_help` — URL contract + agent flow for ns.webcivics.net
- `catalog_summarize` — filter a fetched `catalog.json` body (no network)
- `resolve_dataset_urls` — expand short paths to HTML/N3/TTL/JSON-LD URLs

Quin `u64` inputs accept decimal or `0x` strings. Outputs use decimal strings
to preserve exact values in JavaScript.

For embedding on `ns.webcivics.net`, copy `pkg/` to the site’s
`public/wasm/webizen-lite/` and publish `agent-mcp-guide.md`.

See [the capability profile manual](../../docs/manuals/wasm-capability-profiles.md).
