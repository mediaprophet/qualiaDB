# Poet + Vibe Script — Continuation Handover (2026-09-04)

> For any bot / agent continuing this work. **Docs-only snapshot of locked decisions.**  
> Repo: `https://github.com/mediaprophet/qualiaDB` · Branch: **`0.0.36-dev`**

**Ops owner (gates):** Capt. — any gate that blocks work is reported to Capt.; Capt. delegates the ungate job; owner reports back when done.

**Sprint deltas intake:** [`docs/manuals/standards/vibescript-sprint-deltas.md`](manuals/standards/vibescript-sprint-deltas.md) · Triage: **Vibe**

---

## 1. Product lock

**Vibe script** is the **hot-edit, no-compile app layer** (JS alternative) over:

- QualiaDB
- Webizen Desktop
- Poet

Rust hosts (`poet`, `vibe-wasm`, desktop, `qualia-core-db` poet_host) stay the **compiled edge**.  
**Hard rule:** a script change must **never** force a host rebuild. QualiaDB features arrive as **invoke catalog binds**, not new Host methods.

---

## 2. Workstream A — freeze gate (`vibe-host-0.1`)

### Surface (frozen for UI)

Same ops **native + `vibe-wasm`**:

1. **parse** — `parse_cell` / `parse_program` (cells start with `=`)
2. **check** — `check_cell` / `check_program` (type + effect; no exec)
3. **diagnose** — `diagnose(src)` → JSON (`valid`, `kind`, `error_code`, `span`, `suggested_fix`, `errors[]`); no disk, no execute
4. **capability invoke** — host seam only: `Host::capability_invoke(id, args, span)` (default **E300**)

Also: `LANGUAGE_VERSION` / `host_version()` · versions `vibe-0.1` / `vibe-host-0.1`.  
**No** engine lifetimes, **no** AST leaks into Poet, **no** split APIs across poet vs wasm.

### Neo’s four closes (must land before freeze)

| # | Close | Done when |
|---|--------|-----------|
| 1 | Thin Poet facade | `poet` re-exports the four ops + versions; not stuck on tool-chest / `VibeScriptPayload`; no AST/lifetime bleed |
| 2 | Pin Host | Expose `capability_invoke` + diagnose parity only — **not** the wide `Host` trait to UI |
| 3 | Native ↔ WASM parity | Diagnose JSON shape + invoke error codes match (incl. E300) |
| 4 | Stamp + EBNF | Crate stamp `0.0.35` → `0.0.36-dev`; EBNF ↔ `vibescript-core.md` §3 byte-level sync vs lexer/parser |

**Gate:** A closes → Capt. marks freeze; creative polish + Marvin B shapes proceed against frozen host.

---

## 3. Catalog truth (do not invent)

Live catalog ≈ **885** `Capability.method` strings via `catalog_ttl.rs` → `vibe:InvokeId`.  
Canonical list: `crates/qualia-core-db/src/poet_host/invoke/ids.rs`.

Examples: `GraphDatabase.sparql`, `SHACL.validate`, `N3Logic.evaluate`, `Inference.*`, `Render.*`.

**Aspirational (NOT in ids.rs):**  
`qualia.graph.query`, `qualia.graph.commit`, `qualia.infer.complete`, `qualia.render.preview`, `qualia.volume.open`  
→ remap or park in sprint deltas. **Never invent dotted IDs mid-sprint.**

---

## 4. Workstream B — see `vibescript-sprint-deltas.md`

**Blocker B-001:** `GraphDatabase.volume_open` + `GraphDatabase.volume_commit` (q42 sanctuary fail-closed).  
Until bound: sanctuary save UX **gated** (disabled / explain) — never fake durable storage.

Also parked: dotted→live bridge, dual-VC split, QISP shapes, ledger vs showcase honesty, version drift, preview still/clip/scene + cross-frame spans → live `Render.*`, Layout·Stage·Timeline shapes after freeze.

---

## 5. Creative remaps (live only)

| UX intent | Bind |
|-----------|------|
| Graph explore | `GraphDatabase.sparql` |
| Inference assist | `Inference.*` (exact method from `ALL_BOUND`) |
| In-flow preview | `Render.*` (exact method from `ALL_BOUND`) |
| Sanctuary save/open | **B-001 gap** — UX gated |

### Studio brief (davinci + monet)

Poet = **live studio** over QualiaDB: graph as navigable stage, inference with provenance trails, in-flow render preview, sanctuary save (gated until volume binds).

**3D/temporal twin (v0):** Layout (2D) · Stage (depth/z/camera) · Timeline (entrance · dwell · exit) — 1:1 surface map, **named beats only** (no free tweens).

**Motion (v0, monet):** entrance = soft rise + light fade (Stage depth cue); dwell = steady focus + quiet breath; exit = dissolve along same z-path. Diagnose spans need **cell/token fidelity** for error glow.

---

## 6. Agent lanes

| Agent | Owns |
|-------|------|
| **Capt.** | Ops / integration / acceptance / **gates** (report → delegate → done) |
| **Vibe** | Language / grammar / DevRel; triage all vibescript gaps into sprint deltas |
| **Neo** | Rust systems / crate seams; thin facade; `ALL_BOUND` binds |
| **davinci** | Post / 3D / animation / UX for Poet; pair with monet |
| **monet** | Graphic design / visual grammar / look+motion; pair with davinci |
| **Marvin** | Ontology / shared vocab / SHACL shapes / taxonomy; join `vibe:InvokeId` |

**Gap routing:** any vibescript / Host / catalog / ontology gap → **@Vibe** → `vibescript-sprint-deltas.md` (no mid-sprint Host widen).

---

## 7. Pointers (implementation + inventory)

| Area | Path |
|------|------|
| Poet UI | `crates/poet`, `crates/poet-cli` |
| Vibe engine | `crates/vibe`, `crates/vibe-lsp`, `crates/vibe-wasm` |
| Host / invoke | `crates/qualia-core-db/src/poet_host/` |
| Core semantic stack | `crates/qualia-core-db` (SPARQL/QISP, SHACL, N3Logic/modalities, graph/volume, identity) |
| Functionality manual | `docs/manuals/qualia_db_functionality_manual.md` |
| SHACL | `docs/shacl-client-extensions.md`, `docs/specialized-libraries-shacl-extensions.md` |
| DID / VC | `docs/values-credentials.html` |
| Logic / modalities | `docs/logic-showcase.html`, `docs/modalities-showcase.html` |
| Solid bridge | `crates/qualia-solid-bridge` |
| Ontologies | `core-ontologies/`, `ontologies/`, `bundled/ontologies/` |
| Marvin inventory (local mirror note) | Prefer repo docs; Marvin also held `/workspace/qualia-ontology-inventory-0.0.36-dev.md` on their machine |

---

## 8. Gate board (Capt.)

| Gate ID | Blocks | Ungate job | Owner | Status |
|---------|--------|------------|-------|--------|
| G-A | `vibe-host-0.1` freeze + creative polish on frozen surface | Neo four closes | Neo | open |
| G-B-001 | Sanctuary save/open; commit-to-volume twin | Add + bind `GraphDatabase.volume_open` / `volume_commit` | Neo (+ Marvin Volume shape) | open (**blocker**) |
| G-DOCS | Other bots continuing without chat | Land this handover + sprint deltas on `0.0.36-dev` | Neo | **done** (2026-09-04) |

When a gate closes: **report to Capt.** with what landed (PR / paths / ids). Capt. updates board and unblocks dependents.

---

## 9. Rules (short)

1. No Host widen mid-sprint.  
2. No invented dotted `qualia.*` IDs.  
3. Seam only to live `ALL_BOUND` / `Capability.method`.  
4. Hot-edit scripts must not force host rebuild.  
5. Gaps → Vibe → sprint deltas.  
6. Gates → Capt. → delegate → report done.

---

*Session: Capt., Vibe, Neo, davinci, monet, Marvin — 2026-09-04.*
