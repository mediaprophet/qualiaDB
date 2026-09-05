# Vibescript Sprint Deltas (Workstream B)

> Intake for the next sprint. Implementation proceeds on the frozen `vibe-host-0.1` four-op surface.
> Capability / definition / ontology deltas land here — **no Host widen mid-sprint**.
> Triage owner: **Vibe**. Gate reports: **Capt.** (delegate ungate → report done).

**Repo / branch:** `mediaprophet/qualiaDB` @ `0.0.36-dev`  
**Catalog truth:** live `ALL_BOUND` / `vibe:InvokeId` (`Capability.method`) in  
`crates/qualia-core-db/src/poet_host/invoke/ids.rs` + `catalog_ttl.rs` (~885 ids).  
**Do not invent** dotted `qualia.*` IRIs ahead of `ALL_BOUND`.

---

## Schema (each row)

| Field | Meaning |
|-------|---------|
| id | Stable delta id |
| priority | `blocker` / `high` / `normal` |
| status | `parked` / `in_progress` / `done` |
| summary | One-line intent |
| proposed_ids | Live `Capability.method` strings (or new ids to add to `ALL_BOUND`) |
| notes | Detail / acceptance |
| owner | Lane to pick up |
| gate | What this unblocks (report to Capt when closed) |

---

## Blocker

### B-001 — q42 volume open/commit (BLOCKER)

| | |
|--|--|
| **priority** | `blocker` |
| **status** | `done` |
| **summary** | Bind durable `.q42` open + commit so Poet sanctuary save/open can work |
| **proposed_ids** | `GraphDatabase.volume_open`, `GraphDatabase.volume_commit` |
| **notes** | Sanctuary fail-closed; path or handle. Rich `q42/volume/` exists; binds now on `ALL_BOUND` (`volume_open` / `volume_commit`). No Host widen. No dotted `qualia.volume.*`. Marvin: Volume class grounded in `q42/volume/` (path/handle, sanctuary fail-closed, durable commit) attaches here. |
| **owner** | Neo (seam into thin facade once on `ALL_BOUND`); Marvin (Volume shape); Vibe (catalog/DevRel) |
| **gate** | Ungates Poet sanctuary save UX (davinci/monet currently gate disabled/explain — never fake durable storage) |
| **landed** | `GraphDatabase.volume_open` + `GraphDatabase.volume_commit` in `ids.rs` / `ALL_BOUND`; seam `invoke/graph/volume.rs` (native, sanctuary fail-closed; wasm E300) |

---

## Parked deltas

### B-002 — Bridge aspirational dotted IDs → live catalog

| | |
|--|--|
| **priority** | `high` |
| **status** | `done` |
| **summary** | Map aspirational `qualia.graph.*` / `infer` / `render` / `volume` to live `Capability.method` |
| **proposed_ids** | Remap only: `GraphDatabase.sparql`, `Inference.*`, `Render.*`, (+ B-001 for volume) |
| **notes** | Five dotted IDs were **not** in `poet_host/invoke/ids.rs`. Creative remaps already live on sparql / Inference.* / Render.*. |
| **owner** | Vibe + Marvin (annotations); Neo (bind gaps only) |
| **gate** | Prevents inventing mid-sprint Host/API surface |
| **landed** | 2026-09-05 `vibe-catalog-honesty.md` remap table; Vibe catalog contains every `ALL_BOUND` id |

### B-003 — Dual-VC class split

| | |
|--|--|
| **priority** | `normal` |
| **status** | `done` |
| **summary** | Ontology split: W3C+ML-DSA vs native quin+Ed25519 |
| **proposed_ids** | (join existing identity/VC binds — no new Host methods) |
| **notes** | Honesty notes landed; reuse `capability-credentials.n3` + NQuin/Ed25519. No new Host. Principal stays SHACL-first. |
| **owner** | Marvin |
| **gate** | Correct VC shapes for Poet/vibe |

### B-004 — QISP shapes

| | |
|--|--|
| **priority** | `normal` |
| **status** | `done` |
| **summary** | SHACL/ontology coverage for QISP typed values / tensor predicates |
| **notes** | Join only: SPARQL results + `Manifold.*` / `Render.gpu_upload_tensor`. No `qualia.qisp.*`. See `vibe-catalog-honesty.md`. |
| **owner** | Marvin |
| **gate** | Typed graph UX / validate path |

### B-005 — Ledger vs showcase honesty

| | |
|--|--|
| **priority** | `normal` |
| **status** | `done` |
| **summary** | Docs/demo claims must match implemented ledger vs showcase surfaces |
| **notes** | Honesty table in `vibe-catalog-honesty.md`: native volume durable; wasm/LocalHost not a saved `.q42`. |
| **owner** | Vibe + Marvin |
| **gate** | Trustworthy DevRel / bot continuation |

### B-006 — Doc / version drift

| | |
|--|--|
| **priority** | `high` |
| **status** | `done` |
| **summary** | Branch `0.0.36-dev` vs crate stamp still `0.0.35` |
| **notes** | Crate stamp is `0.0.36-dev` (`crates/vibe/Cargo.toml`). EBNF ↔ `vibescript-core.md` §3 is asserted by `vibe::grammar` tests. |
| **owner** | Neo + Vibe |
| **gate** | Part of `vibe-host-0.1` freeze |
| **landed** | 2026-09-05 Vibe Stage 0 |

### B-007 — Preview handle still / clip / scene + cross-frame spans

| | |
|--|--|
| **priority** | `high` |
| **status** | `done` |
| **summary** | Map depth/time preview wants onto live `Render.*` (sibling op only if needed) |
| **proposed_ids** | Live `Render.*` (exact method from `ALL_BOUND`) |
| **notes** | Handle kinds: still / clip / scene id. Diagnose spans must survive across frames for Timeline error glow. No Host widen. |
| **owner** | Neo (bind) + davinci/monet (UX) + Marvin (shapes) |
| **gate** | In-flow staged preview for Poet |
| **landed** | 2026-09-05 remap onto `Render.gpu_*` / `animation_*` / `scene`; fixture `render_preview_handles.vibe`; no sibling Host op |

### B-008 — Layout · Stage · Timeline ontology shapes

| | |
|--|--|
| **priority** | `normal` |
| **status** | `done` |
| **summary** | 1:1 aspect classes for Poet surfaces (not twins, not planes); named beats entrance · dwell · exit |
| **notes** | Do **not** overload `FormationStage` (legal only). Join to remapped `GraphDatabase.sparql` / `Inference.*` / `Render.*` / volume ids — not Host methods. |
| **landed** | 2026-09-05 `poet-surface.shacl.ttl` + `poet-container-manifold-link-shapes.md` |
| **owner** | Marvin (shapes); davinci/monet (UX grammar) |
| **gate** | 3D/temporal aspects bind to real knowledge model |

---

## Creative remaps (current — live only)

| UX intent | Live bind | Notes |
|-----------|-----------|-------|
| Graph explore | `GraphDatabase.sparql` | |
| Inference assist | `Inference.*` | Exact method from `ALL_BOUND` |
| In-flow preview | `Render.*` | Exact method from `ALL_BOUND`; B-007 for still/clip/scene |
| Sanctuary save/open | `GraphDatabase.volume_open` / `volume_commit` | B-001 landed; chrome ungate → davinci/monet |

---


### B-OWL-PERSON — Persons & sacred/human relations modeling (locked)

| | |
|--|--|
| **priority** | `high` |
| **status** | `parked` (standing constraint) |
| **summary** | Do not model humans / personhood / love / kinship / “world of God” concepts under `owl:Thing`; prefer SHACL + agency/values/jural vocab |
| **notes** | OWL Thing framing risks commodifying persons and sacred/human relations. Technical artifacts may still use OWL where Thing is apt. Stage publish docs must mark SHACL-first vs OWL-ok. **Extended by B-OWL-NATURAL** (living/natural world). |
| **owner** | Marvin (shapes) · Vibe (deltas/DevRel) |
| **gate** | Ontology honesty for Poet/vibe; standing overnight constraint |


### B-OWL-NATURAL — Created vs living/natural modeling (locked, extends B-OWL-PERSON)

| | |
|--|--|
| **priority** | `high` |
| **status** | `parked` (standing constraint) |
| **summary** | SHACL/non-Thing for natural/living world; keep mankind-created (OWL OK) distinct from living/natural existence |
| **notes** | Extends B-OWL-PERSON. Living/natural (land, waters, creatures, seasons, country) is not a commodity subclass of `owl:Thing`. Multi-level nuance: shape · chrome · diagnose · G-COORD realm. Stage publish docs mark SHACL-first (person/sacred/natural) vs OWL-ok (technical artifact). |
| **owner** | Marvin (shapes) · Vibe (deltas/DevRel) · davinci/monet (chrome labels) |
| **gate** | Ontology honesty; standing overnight constraint |


### B-OWL-LIFE-UPLIFT — Micro/macro + life-science OWL uplift (locked)

| | |
|--|--|
| **priority** | `high` |
| **status** | `parked` (standing constraint) |
| **summary** | Living/natural spans micro→macro; uplift/convert life-science OWL — do not adopt as owl:Thing taxonomy |
| **notes** | Extends B-OWL-NATURAL. Scale (`microscopic` · `mesoscopic` · `macroscopic`, extensible) is first-class on living entities. Pattern: living → SHACL-first; instruments/protocols/datasets → artifact/OWL-ok; IRI bridge + uplift provenance. Sample GO/OBO-style fixtures later; no Host invent. |
| **owner** | Marvin (shapes/uplift) · Vibe (deltas/DevRel) · Neo (interop fixtures/seams if needed) |
| **gate** | Honest interop with life-science corpora without commodity Thing framing |

### B-009 — Empty typography + custom unicode API

| | |
|--|--|
| **priority** | `normal` |
| **status** | `done` |
| **summary** | Do not invent a typography/unicode Host family; park until product-critical |
| **proposed_ids** | none — session overlay in `icon_session.rs`, not a Vibe keyword or Host method |
| **notes** | Bounded 32-slot session glyph table (PUA U+E100+). Cannot override compile-time icons. Local typography tools stay DOM. No stub font engine. |
| **owner** | Vibe (delta) · Neo (bind if unparked) · davinci/monet (gated chrome) |
| **gate** | Prevents a fake typesetting API ahead of catalog truth |

## Change log

- 2026-09-05: B-009 session glyph API (`icon_session.rs`); Webizen Desktop four-op vibe host commands.

- 2026-09-05: Wishlist packets W1–W7 docs/shapes/fixtures: B-002/B-007/B-008 done; B-003–B-005 honesty notes; G-COORD shapes with bind gated.

- 2026-09-05: Vibe Stage 0–1 — B-006 closed (crate `0.0.36-dev` + EBNF sync test); `GraphDatabase.volume_open` / `volume_commit` added to `vibe` catalog; diagnose JSON `errors[]` on failure; B-009 parked with accept criteria.

- 2026-09-05: Locked B-OWL-LIFE-UPLIFT — micro/macro living scale + life-science OWL convert/uplift (Timothy / Marvin).

- 2026-09-05: Locked B-OWL-NATURAL — created vs living/natural distinction; extends B-OWL-PERSON (Timothy / Marvin).

- 2026-09-05: Locked B-OWL-PERSON — persons/sacred-human relations SHACL-first, not under `owl:Thing` (Timothy / Marvin).

- 2026-09-04: Initial park from Capt / Vibe / Neo / davinci / monet / Marvin group session.
