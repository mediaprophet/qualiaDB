# WASM Lite agent conformance checklist

Companion to site file `https://ns.webcivics.net/agent-conformance.md` and plan
`docs/plans/wasm-lite-agent-query-plan.md`.

## Maintainer verification

```powershell
cd C:\Projects\qualia-27062026
cargo test -p webizen-lite-wasm --lib
wasm-pack build crates/webizen-lite-wasm --target web --out-dir crates/webizen-lite-wasm/pkg --release
# copy pkg/* → ns public/wasm/webizen-lite/
```

```powershell
cd C:\Projects\webcivics\ns\ns
node scripts/generate-search-indexes.js
# deploy public/ so live llms + agent-* + search/ + wasm update
```

## Checklist

| Check | Pass? |
|-------|-------|
| `tools/list` includes discovery + session + deontic bridge tools | unit surface |
| `catalog_summarize` + `titleContains` | unit test |
| `corpus_summarize` + Consumer Data Right | unit test |
| `export_graph` jsonld + rdfjson | unit test |
| `load_graph` n3 → `query_graph` / `query_sparql` | unit test |
| Q42L round-trip `export_q42lite` → `load_q42` | unit test |
| `compile_deontic_norms` + Active/Expired/Defeated | unit test |
| `search/title-index.json` generated | ns script |
| Live deploy of llms + agent-* + wasm | **operator** |

## Status

**Code complete** for plan phases P1–P4 (2026-07-24).  
`cargo test -p webizen-lite-wasm --lib` → 13 passed.  
Live Cloudflare deploy of `ns` remains the remaining operator step (P0).
