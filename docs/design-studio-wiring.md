# Design Studio — wiring and requirements

Natural-language design → `qualia.design` graph → `Tensor10D` SOA → **full Qualia Portal WASM** (same T2 stack as `spatial.html`), plus device-aware asset recommendations for the native runtime.

> **Not Webizen Studio.** Design Studio is the **docs / :8080 tech demo**. `crates/webizen-studio/` is out of scope for the current release target.

## Surfaces

| Surface | URL | Role |
|---------|-----|------|
| Docs (GitHub Pages) | `docs/design-studio.html` | **Primary demo** — `QualiaPortal` + `design_encode_wasm` / `export_tensor_buffer_wasm` |
| Settings portal | `http://127.0.0.1:8080/design-studio.html` | Same UI (sync via `scripts/sync-portal-design-kit.ps1`) + SPARQL + native asset APIs |

## Data contract

Design documents use `type: qualia.design` (see `crates/qualia-core-db/src/design_encode.rs`).

Portal bake (required):

1. `design_encode_wasm(json)` — pins U1 substrate + provenance (when export present in `docs/pkg/qualia/`)
2. `export_tensor_buffer_wasm(json)` — binary SOA with `"parts"` in JSON
3. `QualiaPortal.upload_tensor_buffer(bytes)` — phenomenal T0/T1/T2 render

Rebuild WASM: `scripts/package-qualia-wasm.ps1` or GitHub Pages CI (`pages.yml`).

## Asset recommendations

### Browser (offline-first)

1. `navigator.deviceMemory` + optional WebGPU probe → tier (`edge` / `mainstream` / `high_performance`).
2. Prompt text → inferred domains (`product`, `electrical`, `iot`, …).
3. Score against `docs/resources/asset-catalog.json` (trimmed mirror of `resources/*.yaml`).

### Desktop (authoritative)

`POST http://127.0.0.1:8080/api/assets/recommend`

```json
{
  "device": { "ram_gb": 16, "has_webgpu": true, "cpu_cores": 8 },
  "design": { "prompt": "two part smart switch…", "domains": [], "keywords": [] }
}
```

Implementation: `qualia-client-core/src/asset_recommendations.rs` loads `resources/llms.yaml` + `resources/ontologies.yaml`, checks installed manifests under `{storage}/`.

### Install actions

| Asset | Native path | Portal API |
|-------|-------------|------------|
| Ontology | `qualia resources import ontology {id}` | `POST /api/assets/enqueue` `{ "kind": "ontology_catalog_import", "ontology_id": "shacl" }` |
| LLM (GGUF) | `qualia resources import llm {id}` | Tray **Manage Models** / `POST /api/assets/enqueue` `{ "kind": "llm_catalog_import", "llm_id": "…" }` (planned — see sprint plan) |
| Bundled seed | — | `POST /api/assets/enqueue` `{ "kind": "bundled_ontology_seed" }` |

## SPARQL enrichment

When portal is live:

- `GET /api/sparql/endpoints`
- `POST /api/sparql/query` → local daemon `:4242/query` or federated endpoint

Requires graph daemon for local graph queries (`graph_daemon_reachable` in `/api/status`).

## End-to-end native stack (functional requirements)

```
User NL prompt
    → [optional] Remote/Hybrid LLM emits qualia.design JSON
    → design_encode_wasm / design_to_tensors
    → Qualia Portal (tensor SOA + pick/navigate)
    → validate_output (≥1 provenance Quin) on commit paths

Parallel:
    → asset_recommendations (RAM + domains)
    → user installs LLM + ontologies
    → ontology import jobs → Index/ + daemon reload
    → SPARQL enrichment + chat grounding improve
```

### Checklist for “fully functional” native creation loop

- [x] `design_encode.rs` + WASM export
- [x] Design Studio UI (docs + :8080)
- [x] Full Qualia Portal WASM preview (not canvas2d fallback)
- [x] SPARQL proxy on :8080
- [x] Asset recommend API + client-side fallback
- [x] Ontology enqueue via `/api/assets/enqueue`
- [ ] LLM job enqueue from Design Studio → tray / `:8080` portal (`local_job_scheduler`, not Flutter)
- [ ] Inference Handover Protocol (IHP) — browser WASM → native lease ([`plans/sprint-inference-handover-native-viewport.md`](plans/sprint-inference-handover-native-viewport.md))
- [ ] Online LLM → `qualia.design` tool in `online-llm-demo.html`
- [ ] `spatial.html` tab importing saved jobs from `localStorage`
- [ ] Native 10D preview — WASM embed interim or PR-C10 desktop parity (Flutter FRB **deprecated**)

## Desktop control surface (2026-06-17)

**Flutter LLM Hub is deprecated.** Active paths:

| Control | Entry |
|---------|--------|
| System tray | `webizen-desktop` — Open Studio, Settings Portal `:8080`, Ambient toggle |
| Settings portal | `http://127.0.0.1:8080/` — Design Studio, SPARQL, asset APIs |
| Webizen Studio | `llm_harness`, `model_lifecycle` (wire to real orchestrator — sprint) |

Full sprint breakdown: [`plans/sprint-inference-handover-native-viewport.md`](plans/sprint-inference-handover-native-viewport.md).

## Regenerating docs catalog

When `resources/llms.yaml` or `resources/ontologies.yaml` change, refresh `docs/resources/asset-catalog.json` (trimmed fields only) or add a small export script in `scripts/`.