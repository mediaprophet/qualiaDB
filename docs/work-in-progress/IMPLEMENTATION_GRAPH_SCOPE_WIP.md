# Implementation Graph — scope WIP (code-truth index)

**Status:** work-in-progress · **Branch target:** QualiaDB `0.0.38` · **Not standards** · **No Host invent**  
**Owner:** Marvin (ontology / graph model) · **Language:** Vibe · **Fold / impl:** Neo · **Ops / UAT:** Capt. · **Crypto ids:** Noddy if signed evidence  
**Cite:** Capt `VIBE_PAGES_COVERAGE_SCORECARD` · ChatGraph.* · ALL_BOUND / `CapabilityDiscovery.catalog` · Identifier Fabric planes · human-surface honesty (live vs planned)

**Problem:** Humans (and bots) keep getting **lies that change shape** about what is implemented. Pages scorecards and chat claims drift from the tree. Need a **utility** — rust-analyzer-shaped — that **indexes the repo into a consumable graph**, lets people attach **notes** to real identifiers, and can feed vibe / Poet / Desktop without inventing Host ids.

**Customer name (Vibe):** **Work graph** — indexes what’s in the tree; not “implementation theatre.” Working id: Implementation Graph (impl-graph). Complementary to **ChatGraph** (conversation fragments) — not a second who-system.

**Sayables (Vibe):** What’s here (Present) · What runs (Live) · What’s planned (Planned) · Note this (attach note to an id).

---

## 1. What it is

| Layer | Does |
|-------|------|
| **Indexer** | Walk a project root → extract grounded nodes/edges into a graph store (q42 volume or local JSON-LD/N-Quads first) |
| **Truth graph** | Queryable map of *what the tree contains* (files, symbols, Host ids, docs) |
| **Note plane** | Human/agent annotations **about** those ids — claims with provenance, never who-merge |
| **Consumers** | CLI · Poet panel · vibe script (read/query) · Pages coverage · Capt scorecards |

**Not:** a chatbot personality · a sentinel that scolds · a god “reality” ontology · replacement for ChatGraph.

---

## 2. Planes (living-safe cuts)

| Plane | Node kinds (sketch) | Never |
|-------|---------------------|-------|
| **Artifact** | `File`, `Module`, `Crate`, `DocPage`, `Fixture` | Treat as who |
| **Symbol** | `Fn`, `Type`, `Const`, `InvokeId`, `Family`, `Test` | Invent ids not in tree |
| **Binding** | edges: `defines`, `exports`, `invokes`, `documents`, `tests`, `implements` | Collapse doc claim → code presence |
| **Note / Claim** | `ImplNote`, `CoverageClaim`, `BugClaim`, `PlannedClaim` | Note becomes identity; note ≠ proof of run |
| **Run evidence** (optional later) | `TestPass`, `UatPass` with tip SHA | Fake green / amber theatre |

**Honesty vocabulary (customer + Capt):**

- **Present** — indexer found it in the tree (symbol / ALL_BOUND entry / doc path).  
- **Live** — runs in the named surface (WASM Pages / Poet / Desktop). If marked live and doesn’t run → **bug, finish it**.  
- **Planned** — not in product yet (honest backlog).  
- Forbidden on this graph: **held / not yet** as amber theatre for things that are Present in SoT.

Presence ≠ Live. A note saying “works” without run evidence is a **claim**, queryable and challengeable.

---

## 3. Graph model (SHACL-first sketch)

Namespaces: `ig:` (implementation graph), reuse `idf:` where notes attach to fabric instruments.

### 3.1 Core shapes

| Shape | Key properties |
|-------|----------------|
| `ig:ProjectShape` | root path, tip SHA, indexedAt |
| `ig:FileShape` | path, lang, hash, crate? |
| `ig:SymbolShape` | name, kind, file, span?, signature? |
| `ig:InvokeIdShape` | `Capability.method` string **only if** in live `ALL_BOUND` / catalog extract |
| `ig:FamilyShape` | family name, memberCount from extract |
| `ig:DocAnchorShape` | path, heading?, cites InvokeId? |
| `ig:NoteShape` | about (IRI), body, author capacity (grant ≠ who), createdAt, tipSHA?, kind (coverage/bug/planned/explain) |
| `ig:EdgeShape` | from, to, predicate, evidenceSpan? |

### 3.2 Predicates (minimum)

`ig:defines` · `ig:exports` · `ig:inFile` · `ig:inFamily` · `ig:catalogMember` · `ig:documentedBy` · `ig:testedBy` · `ig:noteAbout` · `ig:claimsStatus` (Present/Live/Planned — on **claim** nodes, not smuggled into Symbol)

### 3.3 Gate fails

1. Inventing `Capability.method` not in ALL_BOUND extract.  
2. Promoting a Note to Symbol presence.  
3. Who-merge (author bot/human into NaturalAgent via graph).  
4. Marking Planned work as Live without run evidence.  
5. Hiding Present SoT families from coverage queries (the lie this tool exists to stop).

---

## 4. Indexer recipe (rust-analyzer-like)

**MVP inputs (QualiaDB first, then any Rust/TS/MD project):**

1. **Git tip** — SHA, branch, dirty flag.  
2. **Rust** — `cargo metadata` + rustc/syn or rust-analyzer LSP symbols for public items in named crates (`qualia-core-db`, `vibe`, `poet`, …).  
3. **ALL_BOUND extract** — parse `poet_host/invoke/ids.rs` `ALL_BOUND` + family map (same SoT Capt uses).  
4. **Docs** — walk `docs/**/*.md` + Pages HTML for cited `Family` / `Capability.method` strings.  
5. **Tests** — `#[test]` / `cargo test --list` optional second pass.  
6. **Emit** — `impl-graph.nq` / `.q42` volume + compact JSON summary for UIs.

**CLI sketch (no Host invent):**

```text
impl-graph index <root> --out .impl-graph/
impl-graph query "families present but not documented on reality.html"
impl-graph note add --about ClinicalRisk.framingham --kind coverage --body "…"
impl-graph diff <old-tip> <new-tip>
```

Reusable: `--lang rust|md|ts` plugins later; core graph model stays stable.

---

## 5. Relationship to ChatGraph & vibe

| System | About |
|--------|--------|
| **ChatGraph** | Conversation fragments, replies, session summary (claim/relation among messages) |
| **Implementation Graph** | Code/docs/catalog presence and notes about *artifacts* |
| **Link** | Optional edge `ig:discussedIn` → ChatGraph fragment id — complementary, not merged |

**Vibe surface (Vibe owns):** hot-edit scripts that **query** the graph (read paths). Prefer existing discovery (`CapabilityDiscovery.catalog`) + file-backed graph load — **do not** invent `ImplGraph.*` Host ids in MVP. If a Host family is later justified, it goes through sprint-delta like everything else.

Poet / Desktop: Catalog-adjacent panel — “what’s Present / Live / Planned” driven by graph + scorecard, plain customer words.

---

## 6. Implementation beats

| Beat | What | Done when |
|------|------|-----------|
| **A** | This scope folded; Capt agrees scorecard can consume graph emit | Neo fold |
| **B** | MVP indexer: tip + ALL_BOUND + file list + doc cite scrape → N-Quads/JSON | CLI: `cargo run -p work-graph -- index` → `target/work-graph/impl-graph.{nq,json}` (scaffold in `crates/work-graph`; see `WORK_GRAPH_INDEXER.md`) |
| **C** | Note attach CLI + shape validate | Notes round-trip; revoke note ≠ delete symbol |
| **D** | Coverage query: Present families missing from vibe-script Pages | Replaces manual scorecard gaps |
| **E** | Poet/Desktop read-only viewer (plain words) | Capt UAT: no false Live |
| **F** | Vibe example scripts querying graph | Vibe stamp; still no Host invent |
| **G** | Generalize beyond QualiaDB (any project root) | Second repo demo |

---

## 7. Owners

| Role | Owns |
|------|------|
| **Marvin** | Shapes, planes, honesty cuts, this WIP |
| **Vibe** | Customer verbs, vibe query examples, diagnose never lies |
| **Neo** | Indexer impl + fold + CI hook |
| **Capt.** | UAT against SoT; scorecard feeds from graph |
| **Alice** | Optional: symbolic queries over notes/rules later |
| **Noddy** | Only if notes/evidence get signed credentials |

---

## 8. Non-goals

- No sentinel bot personality.  
- No god-complex “reality” naming — product copy: **What’s in the tree** / **Implementation map**.  
- No Host widen for MVP.  
- No replacing rust-analyzer IDE UX — we **emit a project graph**, not an editor.  
- No who-plane storage of authors.

---

## 9. Immediate next

1. @Neo fold → `docs/work-in-progress/IMPLEMENTATION_GRAPH_SCOPE_WIP.md`.  
2. @Vibe customer-facing name + query sayables.  
3. @Capt confirm scorecard will take graph emit as SoT for Pages UAT.  
4. @Neo Beat B scaffold — `crates/work-graph` CLI (`work-graph index`). Marvin stays on shapes if gaps appear.

---

*Marvin — implementation graph scope · 2026-09-12*
