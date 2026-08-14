# Webizen Lite / ns.webcivics.net — Agent Query Pipeline

**Status:** Active implementation plan  
**Created:** 2026-07-24  
**Repos:** 2026-07-24  
**Repos:** Qualia `webizen-lite-wasm` + ns site static surface  
**Repos:** `0.0.28` / ns `main` as applicable  

This document is the durable source of truth for the workstream. Session memory may be lost; **follow this file**.

---

## 1. Goal

Enable LLM/agent bots to:

1. **Discover** instruments on `https://ns.webcivics.net` without inventing URLs  
2. **Load** graphs (prefer compiled **`.q42`** volumes; fallback N3)  
3. **Query** for specific sections (SPARQL or honest bounded substitute)  
4. **Evaluate** modalities (**deontic**, epistemic, LTL, paraconsistent, SHACL)  
5. **Export** results as bot-readable RDF: **JSON-LD** (default), **RDF/JSON** ([W3C Note](https://www.w3.org/TR/rdf-json/)), Turtle, N3, optional YAML-LD  
6. Follow **site-published instructions** only — no ad-hoc per-question Python  

WASM remains **network-free**. The host performs all HTTP GETs.

---

## 2. Repositories

| Path | Role |
|------|------|
| `C:\Projects\qualia-27062026` | `crates/webizen-lite-wasm`, core helpers under `wasm-ontology`, this plan, conformance tests |
| `C:\Projects\webcivics\ns\ns` | Static indexes, deploy assets, `llms.txt`, agent guides, packaged `.q42` when published |

Canonical Qualia tree only for Qualia code (no worktrees).

---

## 3. Architecture (target)

```text
Build / curation (offline)
  N3 (source of truth for rules) ──► .q42 parts + manifest
  Catalog / corpus ──► /search/*-index.json

Runtime
  Agent ──HTTP──► ns.webcivics.net (catalog, indexes, .n3, .q42)
  Agent ──load──► webizen-lite-wasm (MCP mcp_jsonrpc)
       │
       ├─ discovery tools (catalog_summarize, corpus_summarize, …)
       ├─ load_q42 / parse_n3
       ├─ query_sparql (or query_labels interim)
       ├─ evaluate_deontic / deontic_govern / other modalities
       └─ export_graph (jsonld | rdfjson | turtle | n3 | yamlld)
```

### Three logic layers (do not collapse)

| Layer | Content | Serialisation |
|-------|---------|----------------|
| **A — Ground graph** | Facts, labels, provenance | N3 subset, Turtle, JSON-LD, RDF/JSON, YAML-LD |
| **B — Logic-as-data** | CML norms, LogicApplication, obligations as RDF | Same as A + `logicMode=as-data` |
| **C — Executable logic** | N3 rules/variables/formulae; Quin opcodes | **N3** and/or **`.q42` + MCP evaluate** — not claimed as full fidelity in pure RDF/JSON |

Always set `queryMeta.projectedFromN3` / `dropped[]` when projecting away N3-only constructs.

---

## 4. Phases

### Phase 0 — Deploy what already exists (ns)

| ID | Task | Acceptance |
|----|------|------------|
| P0.1 | Deploy `llms.txt`, `agent-mcp-guide.md`, `agent-legislation-guide.md`, `wasm/webizen-lite/*` | Live 200; llms mentions MCP |
| P0.2 | Fix `_headers` for nested `/institutions/**/*.{n3,ttl,jsonld}` | Correct Content-Type + CORS |
| P0.3 | Smoke WASM `version` + `tools/list` in browser | JSON-RPC tools array non-empty |

### Phase 1 — Discovery + multi-format export

| ID | Task | Repo | Acceptance |
|----|------|------|------------|
| P1.1 | Generate `/search/title-index.json` from catalog | ns | Small JSON; filterable by title |
| P1.2 | Generate `/search/au-title-index.json` from AU corpus | ns | Privacy / CDR findable by title |
| P1.3 | `catalog_summarize`: `titleContains`, `idPrefix` | qualia | Unit tests |
| P1.4 | `corpus_summarize` for AU corpus body | qualia | Unit tests |
| P1.5 | `export_graph` formats: `jsonld`, `rdfjson`, `turtle`, `n3`, `yamlld` | qualia | Round-trip tests |
| P1.6 | Result envelope `queryMeta` on exports/queries | qualia | Documented fields present |
| P1.7 | Update llms + agent-mcp-guide formats section | ns | Formats + RDF/JSON Note status |

### Phase 2 — Compile-to-`.q42`

| ID | Task | Acceptance |
|----|------|------------|
| P2.1 | Package layout: `manifest.json` + `q42/part-NNNN.q42` | Matches legislation guide |
| P2.2 | Catalog/corpus optional `q42Urls[]`, `manifestUrl` | Fields documented |
| P2.3 | Gold packages (min 3): Privacy, CDR Act, CDR Rules — or test fixtures if CDN size blocks full acts | Native CLI SPARQL works |
| P2.4 | MCP `load_q42`, `list_graphs`, `unload_graph` | Session-scoped; no network in WASM |
| P2.5 | Size/multi-part honesty in docs | No whole-act-in-browser false claim |

### Phase 3 — Bounded query

| ID | Task | Acceptance |
|----|------|------------|
| P3.1 | MCP `query_sparql` SELECT-only, fail closed | Unsupported features → error |
| P3.2 | If SPARQL too heavy for wasm size gate: ship `query_labels` interim + **honest docs** | Label CONTAINS works on gold graph |
| P3.3 | Wire results through `export_graph` | JSON-LD + RDF/JSON samples |
| P3.4 | Gold queries: APP 12, CDR 56AA / consumer data request | ≥1 hit each on fixtures |

### Phase 4 — Deontic bridge

| ID | Task | Acceptance |
|----|------|------------|
| P4.1 | Compile/select path: section/norm rows → deontic Quins (or document exact layout + helper) | Documented + tested |
| P4.2 | Recipe: query → compile → `evaluate_deontic` → `deontic_govern` | One gold path green |
| P4.3 | `logicMode`: `none` \| `as-data` \| `evaluate` | Envelope fields |
| P4.4 | Tests: Active / Expired / Defeated | Unit tests |

**Already exposed (do not re-stub):** `evaluate_deontic`, `deontic_govern`, `evaluate_epistemic`, `route_paraconsistent`, `evaluate_ltl`, `check_subsumption`, `validate_shacl`.

### Phase 5 — Test harness

| ID | Task | Acceptance |
|----|------|------------|
| P5.1 | `cargo test -p webizen-lite-wasm` covers MCP tools added | All pass |
| P5.2 | Optional `scripts/agent-smoke` against fixtures or local serve | Documented how-to |
| P5.3 | `docs/manuals/wasm-lite-agent-conformance.md` checklist | Pass/fail reportable |
| P5.4 | Size gate: wasm gzip ~order of prior ~95 KiB unless budget explicitly raised | Measured |

### Phase 6 — Bot instructions (survive without us)

| Artifact | Purpose |
|----------|---------|
| `ns` `/llms.txt` | Short start-here contract |
| `ns` `/agent-mcp-guide.md` | Full MCP tools, formats, examples |
| `ns` `/agent-legislation-guide.md` | Packages, FRL rights, `.q42` |
| `ns` `/agent-conformance.md` (new) | Must / should / must-not; gold queries |
| Qualia `docs/manuals/wasm-capability-profiles.md` | Profile alignment |
| Qualia `crates/webizen-lite-wasm/README.md` | Build + embed |

Bot instruction **must** cover: policy first; host HTTP; prefer q42; format defaults; logic layers; cite Register + ns URL; `cml:Proposed` ≠ attested; bounds; no invented IDs.

---

## 5. Milestone cuts

### MVP (implement first)

**P0 + P1 + P5 (tests for P1) + P6 draft instructions.**

Unlocks: deployable discovery, title search, multi-format export, bot docs — without waiting on full SPARQL/q42 CDN.

### Full product

MVP + **P2 + P3 + P4** + final conformance.

### Defaults if not re-specified

- Gold `.q42`: **three packages/fixtures** for tests; not all 222 AU acts on CDN in v1  
- SPARQL: attempt bounded SELECT; if size gate fails, **document `query_labels` interim**  

---

## 6. Format policy (export)

| Format | Media type / note | When |
|--------|-------------------|------|
| **jsonld** | `application/ld+json` | **Default** for agents |
| **rdfjson** | `application/rdf+json` ([RDF/JSON Note](https://www.w3.org/TR/rdf-json/)) | Bots that refuse JSON-LD context |
| **turtle** | `text/turtle` | RDF tools |
| **n3** | `text/n3` | Rules / variables / formula fidelity |
| **yamlld** | YAML encoding of same JSON-LD model | Optional human/agent preference |

JSON-LD is W3C-preferred over RDF/JSON; RDF/JSON is offered as an **explicit alternate**.

---

## 7. Non-goals

- Training permission via robots Content-Signal  
- In-WASM network/filesystem  
- Silent `cml:Proposed` → Attested  
- Full remote SPARQL endpoint on Cloudflare as v1 requirement  
- Private non-IRI “bot JSON” that drops linked data identity  

---

## 8. Implementation progress log

Append dated entries here as phases complete.

### 2026-07-24 — Plan written

- Created this document.  
- Next: MVP implementation (P1 wasm tools + ns search indexes + docs; P0 deploy when ns changes land).

### 2026-07-24 — MVP implementation (in progress → code complete pending deploy)

**Qualia (`webizen-lite-wasm`):**

- `catalog_summarize`: `titleContains`, `idPrefix` + `queryMeta`
- `corpus_summarize`: AU corpus / datasets array title-id filter
- `export_graph`: `jsonld` | `rdfjson` | `turtle` | `n3` | `yamlld` + logicMode honesty
- `namespace_discovery_help`: search indexes, conformance URL, export/logic layers, extended flow
- Unit tests: **11 passed** (`cargo test -p webizen-lite-wasm --lib`)

**ns (`public/`):**

- `scripts/generate-search-indexes.js` → `search/title-index.json` (547), `search/au-title-index.json` (222)
- Updated `llms.txt`, `agent-mcp-guide.md`, new `agent-conformance.md`
- `_headers`: nested institutions RDF + `/search/*` + agent md CORS

**Still open (not MVP):**

- P0 live deploy (Cloudflare) so production 404s clear  
- Rebuild/copy WASM pkg to `ns/public/wasm/webizen-lite/` for production  
- P2–P4: load_q42, SPARQL, deontic compile bridge  

**Next:** **deploy ns** (Cloudflare) so live assets update.

### 2026-07-24 — P2–P4 session/query/deontic (code complete)

**Qualia `webizen-lite-wasm`:**

- New module `session.rs`: thread-local session graphs (max 8)
- `load_graph` (`n3` \| `quins` \| `q42lite`), `load_q42`, `list_graphs`, `unload_graph`, `export_q42lite`
- `query_graph` (hashes + label/object CONTAINS via lexicon)
- `query_sparql` SELECT-only subset (`?s ?p ?o` + optional FILTER CONTAINS on `?o`); fail closed otherwise
- Q42L format (magic `Q42L` v1): wasm-safe; **native Q42 v3 rejected** with guidance
- `compile_deontic_norms` + `evaluate_deontic_session` (Active / Expired / Defeated tested)
- Unit tests: **13 passed**

**ns docs:** agent-mcp-guide, agent-conformance, llms updated for full tool surface.

**Still open:** live Cloudflare deploy; optional CDN Q42L packages for gold AU acts.

**Measured:** release `webizen_lite_wasm_bg.wasm` ≈ **520 KiB** raw after P2–P4 (session/query/deontic).

---

## 9. Verification commands

```powershell
# Qualia
cd C:\Projects\qualia-27062026
cargo test -p webizen-lite-wasm
wasm-pack build crates/webizen-lite-wasm --target web --out-dir pkg --release

# Size (approximate gzip)
# Measure pkg/webizen_lite_wasm_bg.wasm

# ns (after copy pkg → public/wasm/webizen-lite/)
cd C:\Projects\webcivics\ns\ns
# generate search indexes (script TBD)
# deploy / wrangler / CF as project practice
```

---

## 10. Related live endpoints (target)

- https://ns.webcivics.net/llms.txt  
- https://ns.webcivics.net/agent-mcp-guide.md  
- https://ns.webcivics.net/agent-legislation-guide.md  
- https://ns.webcivics.net/agent-conformance.md  
- https://ns.webcivics.net/catalog.json  
- https://ns.webcivics.net/au-legislation-corpus.json  
- https://ns.webcivics.net/search/title-index.json  
- https://ns.webcivics.net/search/au-title-index.json  
- https://ns.webcivics.net/wasm/webizen-lite/  

---

## 11. Session handoff

Any new agent session:

1. Read **this file**  
2. Read `coordination/NOTICES.md` and CLAIM if editing  
3. Continue from **§8 progress log** last incomplete phase  
4. Do not re-litigate architecture unless Timothy redirects  
