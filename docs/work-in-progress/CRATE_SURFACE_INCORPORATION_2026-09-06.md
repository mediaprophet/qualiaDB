# Crate → Vibe / Poet incorporation inventory (EXP-C1)

**Date:** 2026-09-06  
**Branch:** `0.0.36-dev`  
**Status:** **SUPERSEDED for freeze wording** — see
`docs/work-in-progress/VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`.  
**Correct methodology:** **VibeScript / Host first** (bind built APIs) → Poet Live
consumes those ids. `vibe-host-0.1` is the **outcome** of incorporation, not a ban
on Host widen.  
**Prior (wrong) line:** classify-only / no Host widen — do not follow.

**Lane:** Poet Gate B swarm — Lane A (inventory still useful as a *priority list*)  
**Companion scan:** `docs/work-in-progress/VIBE_SURFACE_GAP_REVIEW.md`

---

## 1. Method

### Script

1. Extended `scripts/vibe_surface_gap_review.py`:
   - `PRIORITY_GLOBS` now includes full `qualia-client-core/src/**/*.rs`, `qualia-cooperative-core/src/**/*.rs`, `qualia-vision/src/**/*.rs` (chat-only globs removed as redundant).
   - `expand_globs` always `rglob`s those three sibling trees plus `specialized_libs`.
   - Keyword hints added for cooperative / agency / wellfair / qapp / biosense / anatomy (maps to **existing** bound family names only — never invents IDs).
2. Ran: `python scripts/vibe_surface_gap_review.py` → rewrote gap review markdown.

### Manual spot checks

- Crate `lib.rs` module maps and representative `pub` APIs.
- Grep of `ALL_BOUND` / Poet Live `capability_scope` / desktop `generate_handler` for citations.
- Poet `cooperative_economics` vs `qualia-cooperative-core` / `wellfair` / `Econ.*` (duplicate-risk).
- Honesty check: keyword “match” to `Agency` / `ComputerVision` ≠ those crates are fully bound (see §1.1).

### 1.1 Heuristic caveat (important)

Gap-score ≥ 12 rows **under-report** cooperative-core and vision product layers when path keywords hit an existing ALL_BOUND family (`Agency`, `Econ`, `ComputerVision`). Those families exist but cover **different** semantics:

| Keyword match | What ALL_BOUND actually is | What the crate actually is |
|---|---|---|
| `Agency` | **1** id: `Agency.evaluate` = Ed25519 frame/signature verify in `poet_host/invoke/governance` | Cooperative **delegation ABAC** / work items / taxonomy |
| `ComputerVision` | **10** kernel ops over `specialized_libs::computer_vision` | Product biosense, recipes, weights, media store, taxonomies |

This inventory therefore classifies by **manual seam judgment**, not gap-score alone.

---

## 2. Measured (script exit evidence)

```
Wrote C:\github\qualiaDB\docs\work-in-progress\VIBE_SURFACE_GAP_REVIEW.md
ALL_BOUND=892 ALL_INVOKE_IDS=892 modules_scanned=1481
```

From regenerated report header (2026-09-06 04:03 UTC):

| Metric | Count |
|---|---:|
| Workspace members declared | 25 |
| `ALL_BOUND` / `ALL_INVOKE_IDS` | **892 / 892** (0 drift) |
| Families | 99 |
| Gap-score ≥ 12 modules | 93 |
| MCP stable tools | 62 |
| Desktop command names | 542 |
| Poet Live / capability_scope strings | 39 |
| Poet Live not in ALL_BOUND | **0** |

Sibling-crate pub-surface spot count (manual one-shot, excl. `*test*` stems):

| Crate | `.rs` files | ≈ pub fns | ≈ pub types |
|---|---:|---:|---:|
| `qualia-cooperative-core` | 12 | 92 | 37 |
| `qualia-vision` | 110 | 229 | 85 |
| `qualia-client-core` | 224 | 1783 | 503 |

---

## 3. Per-crate tables

Seam legend (freeze): **Vibe (existing ALL_BOUND)** · **MCP** · **Desktop-FRB** · **Poet Live consume** · **cold-only** · **duplicate-risk**.

### 3.1 `qualia-cooperative-core`

Transport-neutral cooperative domain for WellFair desktop + forthcoming Cooperative Qapp. Re-exports `wellfare_core::{finance, projects, record}`.

| Module / area | Pub surface summary | Vibe / MCP / Desktop / Poet citations | Recommended seam | Duplicate-risk |
|---|---|---|---|---|
| `work_item` | WorkItem + status events + board projection (~13 fns, 7 types) | **Desktop:** `wellfair_add_work_item`, `wellfair_add_work_item_status`, `wellfair_work_item_board`. **Client:** `wellfair/api/welfare_work.rs`. **Vibe:** none. **Poet:** none (only copy mentions of wellfair work boards in plans) | **Desktop-FRB** (default); Poet honesty “open in Desktop” if UI ever needs boards | **No** vs Poet `cooperative_economics` (different domain) |
| `agency_delegation` + `agency_domain` + `trigger` + `authority_type` + `taxonomy` + `provenance` | Delegation ABAC (`delegation_permits`), consent, spheres, provenance DAG | **Desktop:** `wellfair_*_agency_*`. **Client:** `wellfair/api/agency.rs`. **Vibe:** name collision only — `Agency.evaluate` is **not** this evaluator. **MCP:** none. **Poet:** none | **Desktop-FRB**; do **not** bind new `Agency.*` under freeze | **Yes (name)** — `Agency.evaluate` ≠ cooperative ABAC; do not conflate in Tool Chest copy |
| `qapp_package` (`manifest`, `pwa`, `remote_controller`) | PWA / remote-controller package generation | **Client:** `wellfair/qapp_publish.rs`. **Desktop:** settings_server PWA path. **Vibe:** none | **Desktop-FRB** / APP registry lanes (Gate B WD/APP) — not Vibe | **No** (Lane B/D own app_manifest/registry) |
| Re-exports `wellfare_core` finance/projects | Shared envelopes / project membership | Desktop wellfair finance/welfare commands; client wellfair host | **Desktop-FRB** | **Partial** vs Poet true-cost UI — product narrative overlap, **no shared types** |

### 3.2 `qualia-vision`

Product surface: re-exports `specialized_libs::computer_vision` kernels; owns biosense, recipes, weights, capability registry, semantic quins, media store.

| Module / area | Pub surface summary | Citations | Recommended seam | Duplicate-risk |
|---|---|---|---|---|
| Kernel re-exports (`cv`, `ops`, `sr`, `bio`, `embeddings`, `gpu`, …) | Thin re-export of core-db CV | **Vibe:** `ComputerVision.*` (**10** ids) invoke `poet_host/invoke/vision/*` → **core-db** `specialized_libs`, not this crate’s product layer. **Poet Live:** `ahash`, `histogram`, `gaussian_blur`, `canny_edges`, `sobel_magnitude` (5 of 10). Uncited bound: `equalize_hist`, `rgb_to_gray`, `dhash`, `hamming_distance`, `cosine_similarity` | **Poet Live consume** remaining bound ids; no new Host binds | **Yes (intentional split)** — product crate vs engine kernels; do not re-bind same ops under a second family |
| `biosense/**` (~93 pub fns) | Consent-gated rPPG, EVM, PAD/liveness, respiration, face mesh helpers | **Not** in ALL_BOUND. No MCP tool name for biosense. Desktop/client via `vision_pipeline` / 10d browse paths | **cold-only** or **Desktop-FRB** (+ consent); **do not bind** under freeze | **No** vs `ComputerVision.*` (kernels ≠ biosignals) |
| `recipes/**` | Pulse / respiration / SPARQL observation recipes | Product orchestration; client `vision_pipeline` | **Desktop-FRB** / cold | **No** |
| `weights`, `media_store`, `capability` | ONNX/session, retention, honesty registry | Client pipeline | **Desktop-FRB** / cold | **No** |
| Dataset taxonomies (COCO, ADE20K, …) | Class lookup tables | Cold authoring | **cold-only** | **No** |
| `semantic`, `detector`, `tracker`, `generator`, `spatial` | Observation quins, synthetic detect/track, mesh handoff | **Client:** `vision_pipeline.rs`, `vision_10d_*`, desktop `browse_vision_10d` / `load_vision_10d` | **Desktop-FRB** (10d browser); Poet only via existing Live CV kernels | **No** |

### 3.3 `qualia-client-core` (broader than chat*)

Largest incorporation surface. Top directories by ≈ pub fns: `wellfair` (513), `api` (324), then chat*, vision_*, wallet, qapp_*, identity, etc.

| Module / area | Pub surface summary | Citations | Recommended seam | Duplicate-risk |
|---|---|---|---|---|
| Chat stack (`chat_graph`, `chat_session`, `chat_relay`, `chat_agents`, `chat_inference`, `chat_mesh*`, `api/chat`) | Session/fragment DAG, relay, agents, inference glue | **Desktop:** `get_chat_graph`, many social/chat cmds. **Vibe:** **no** `Chat.*` / `ChatGraph.*`. **Poet:** EXP-B0 desktop-only default; Inference Live for grounding only | **Desktop-FRB** (owner default); optional later `ChatGraph.*` = **owner gate** | **No** vs `Inference.*` if chat stays desktop |
| `wellfair/**` | Host API: agency, work items, welfare, anatomy, sync, encryption, … | Desktop `wellfair::*` commands; cooperative-core + wellfare-core | **Desktop-FRB** | **Partial** vs Poet cooperative_economics (UI narrative only) |
| `vision_pipeline`, `vision_ingest`, `vision_10d_*` | End-to-end vision + 10d load/rights | Uses **`qualia_vision`**; desktop `browser_10d::*` | **Desktop-FRB**; Poet via `ComputerVision.*` kernels only | **Low** — orchestrates product crate, does not reimplement kernels |
| `qapp_*`, `qapps_protocol`, `bundled_qapps` | Install/registry/MCP for Qapps | Desktop + Gate B APP/WD lanes | **Desktop-FRB** / app registry — **not** Vibe under freeze | Coordinate with Lane B/D; don’t invent Host ids |
| `wallet/**` | Coin select, ledger, chronik | Desktop wallet cmds | **Desktop-FRB**; thin overlap with bound `Finance.*` (3 ids) | **Yes (thin)** — don’t invent Wallet.*; map UX to Desktop or existing `Finance.*` only where true |
| `consent_credential`, `guardianship`, `duty_of_inquiry` | Consent / guardianship flows | Desktop + governance modalities | Prefer existing **Governance / Deontic** Live; else Desktop | **Name** risk with ConsentLedger programmes |
| `inference_backend`, `model_lifecycle`, `mcp_tool_loop` | Local inference orchestration | Desktop / MCP loop; Poet uses `Inference.*` Live | **Poet Live consume** `Inference.*`; Desktop for lifecycle chrome | **No** if paths stay dual-honest |
| `chora/**`, `canvas_*`, `view_host` | Spatial worlds / studio host | Desktop / studio | **Desktop-FRB** / cold | **No** |
| `accountability_store`, `qpu_oracle` | High pub-fn modules with **no** keyword→family match in gap scan | Desktop/internal | **cold-only** or Desktop until owner names a seam | Unknown — triage before any bind ask |
| `ollama_harness` | Legacy naming | Must not become a Vibe backend | **cold-only** / delete later — Qualia local GGUF path is authoritative | **N/A** |

### 3.4 Spot-check: Poet `cooperative_economics` vs cooperative-core / wellfare

| Claim | Evidence |
|---|---|
| Does Poet import `qualia_cooperative_core`? | **No** (grep of `crates/poet` — only wellfair **strings** / library labels). |
| What does Poet implement? | Local `TrueCostModel` + `OntologicalPricingEngine` + Live **`Econ.gini`** on user-supplied incomes (`live_welfare.rs`). |
| Overlap with cooperative-core work items / agency? | **None in code.** Cooperative-core is Kanban + delegation ABAC for WellFair. |
| Overlap with wellfare finance/welfare? | **Product narrative only** (“cooperative”, true-cost). Numbers: Live `Econ.*` / local sketch — not wellfare ledger. |
| Duplicate-risk verdict | **No code duplicate.** Risk is **naming confusion** if Tool Chest copy says “Agency” or “WellFair work board” when the viewport is SDN/true-cost + `Econ.gini`. |

---

## 4. Top 15 incorporation priorities

### A. Poet consumption of **already-bound** Vibe (no Host widen) — prefer these

1. **Finish `ComputerVision.*` Live consumption** — bind remaining 5 of 10 (`equalize_hist`, `rgb_to_gray`, `dhash`, `hamming_distance`, `cosine_similarity`) in image Tool Chest / specialist sessions (media buffer required; honest offline).
2. **Widen Poet Live `Econ.*` beyond A1 slice** — cooperative_economics + Tool Chest already cite `Econ.gini` / curated five; next user-facing math from EXP-A0 gap list (still no new families).
3. **`Inference.verify_turn` / `detect_ungrounded` / `grounding`** — keep chat chrome on Live paths (EXP-B2); do not port chat-graph to Vibe.
4. **`ClinicalRisk.*` Health toolbox** — already partially Live; deepen only with consent-honest fields (existing ids).
5. **`Finance.*` (3) + selective `FinancialModeling.*`** — only where product copy still names them; prefer `Econ.*` when stronger (EXP-A3).
6. **`Statistics.*` sheet toolbox** — already scoped; expand only with dual-path tests.
7. **`Agency.evaluate` honesty in Poet** — if cited, label as **signature/frame verify**, never as WellFair delegation ABAC.

### B. Desktop-only (correct under freeze)

8. **Chat-graph / relay / sessions** — FRB + client-core; EXP-B0 recommended desktop-only.
9. **WellFair agency + work-item board** — desktop commands over cooperative-core.
10. **Vision 10d browse/load/scrub** + full `vision_pipeline` / biosense — desktop (+ consent); not Vibe.
11. **Qapp install / PWA package / registry** — desktop + Gate B APP/WD; not Host.
12. **Wallet / mail / canvas / chora** — desktop product chrome.

### C. Later **owner gate** (would require freeze exception or non-Vibe seam)

13. **`ChatGraph.*` minimal bind set** — only if owner overrides EXP-B0.
14. **Biosense / PAD / rPPG as Vibe family** — high sensitivity; needs consent architecture + owner approval; default reject under freeze.
15. **Cooperative ABAC / work-item as Vibe family** — would collide with / confuse `Agency.evaluate`; prefer Desktop or a **new** owner-approved family name later — **not** silent widen of `Agency.*`.

---

## 5. Explicit “do not bind on Vibe under freeze”

Do **not** add ALL_BOUND / ALL_INVOKE_IDS entries for:

- Any `Chat.*` / `ChatGraph.*` (pending owner EXP-B0 exception).
- Cooperative-core surfaces: `WorkItem*`, `AgencyDelegation`, `delegation_permits`, taxonomy/trigger/provenance, qapp_package generators.
- WellFair host API methods (`wellfair_*`) as Host ids.
- `qualia-vision` biosense / recipes / weights / media_store / taxonomies.
- Duplicate CV kernels under a second family (`Vision.*`, `QualiaVision.*`, etc.).
- Wallet / mail / canvas / chora as new Vibe families.
- Invented dotted `qualia.*` Host widen ids.
- Mapping cooperative ABAC onto existing `Agency.evaluate` (wrong semantics).
- `ollama_harness` as an inference backend family.

Allowed under freeze: **Poet Live dual-path consume** of ids already in ALL_BOUND; Desktop/MCP honesty labels; cold leave.

---

## 6. Seam matrix (sibling crates → freeze)

```
qualia-cooperative-core ──► Desktop-FRB (wellfair_*) ──► client wellfair API
                         └──✗ Vibe (do not bind)

qualia-vision (product) ──► Desktop-FRB (vision_pipeline, 10d)
                         ├── biosense/recipes ──► cold / Desktop + consent
                         └── kernels live in core-db ──► Vibe ComputerVision.* (10)
                                      └── Poet Live consumes subset (5 today)

qualia-client-core ──────► Desktop-FRB primary (chat, wellfair, qapp, wallet, …)
                         ├── Inference / Clinical / Econ via existing Live (Poet)
                         └──✗ new ChatGraph / Agency-ABAC / Biosense Host ids
```

---

## 7. Files changed (this packet)

| Path | Change |
|---|---|
| `scripts/vibe_surface_gap_review.py` | Broader `PRIORITY_GLOBS`, expand extras, keyword hints |
| `docs/work-in-progress/CRATE_SURFACE_INCORPORATION_2026-09-06.md` | This deliverable |
| `docs/work-in-progress/VIBE_SURFACE_GAP_REVIEW.md` | Regenerated by script |

**Not touched:** `q42/app_manifest/`, `q42/app_registry/`, poet product UI, webizen-desktop shell, Host `ids.rs` / vibe catalog.  
**Not committed.**

---

## 8. ⚑ Owner asks

1. Confirm EXP-B0 remains **desktop-only chat-graph** (recommended) so Lane A does not pressure a ChatGraph bind.
2. Confirm biosense stays **off Vibe** until a consent-first programme (not Gate B EXP-C1).
3. Optional: whether remaining 5 `ComputerVision.*` ids should be the next Neo/PFT consume packet (no Host widen).
