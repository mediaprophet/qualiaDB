# webizen-lite-wasm

The browser-local Qualia MCP endpoint for read-only ontology sites such as
`ns.webcivics.net`.

This crate uses `qualia-core-db`'s isolated `wasm-ontology` profile. It does not
compile the portal/WebGPU feature, scientific bundle, LLM runtime, native
daemon, network stack, or filesystem storage into the published WASM.

## Copyright and licence

Copyright (c) 2026 Timothy Charles Holborn  
https://www.linkedin.com/in/ubiquitous/ | timothy.holborn@gmail.com

Licensed under **Creative Commons Attribution-NonCommercial-NoDerivatives 4.0
International (CC BY-NC-ND 4.0)** — the same technical-work scope as the
Webizen / Web Civics namespace repository (`LICENSE` / `RIGHTS.md` there).

- Deed: https://creativecommons.org/licenses/by-nc-nd/4.0/  
- Legal code: https://creativecommons.org/licenses/by-nc-nd/4.0/legalcode  
- See [`LICENSE`](./LICENSE) in this crate.

This does not re-license source legislation or third-party material.

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
- `catalog_summarize` — filter a fetched `catalog.json` (`categoryPrefix`, `titleContains`, `idPrefix`)
- `corpus_summarize` — filter a fetched legislation corpus JSON (`titleContains`, `idPrefix`)
- `export_graph` — ground triples → `jsonld` | `rdfjson` | `turtle` | `n3` | `yamlld`
- `resolve_dataset_urls` — expand short paths to HTML/N3/TTL/JSON-LD URLs
- `load_graph` / `load_q42` / `list_graphs` / `unload_graph` / `export_q42lite` — session graphs (Q42L)
- `query_graph` / `query_sparql` — section-style filters (SPARQL SELECT subset)
- `compile_deontic_norms` / `evaluate_deontic_session` — deontic bridge

Quin `u64` inputs accept decimal or `0x` strings. Outputs use decimal strings
to preserve exact values in JavaScript.

For embedding on `ns.webcivics.net`, copy `pkg/` to the site’s
`public/wasm/webizen-lite/` and publish `agent-mcp-guide.md` + `agent-conformance.md`.

**Implementation plan:** [docs/plans/wasm-lite-agent-query-plan.md](../../docs/plans/wasm-lite-agent-query-plan.md)

See [the capability profile manual](../../docs/manuals/wasm-capability-profiles.md).
