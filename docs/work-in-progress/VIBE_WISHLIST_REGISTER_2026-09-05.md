# Vibe wishlist register

**Status:** Work in progress  
**Date:** 2026-09-05  
**Source:** `docs/manuals/standards/vibescript-complete-wishlist.md`  
**Plan:** `docs/manuals/standards/impl-plan-vibe-sprint-B.md`  
**Deltas:** `docs/manuals/standards/vibescript-sprint-deltas.md`  
**Frozen host:** `vibe-host-0.1` at `6dc2b8b8`

This absorbs the grok-bot wishlist into the work-in-progress set without replacing
the normative spec or the staged impl plans. Execution tracker:
[`VIBE_WISHLIST_IMPLEMENTATION_PLAN_2026-09-05.md`](VIBE_WISHLIST_IMPLEMENTATION_PLAN_2026-09-05.md).

## Evidence vocabulary

- **Implemented:** code + focused test or live bind.
- **Document landed:** agents can follow it; not a Host change.
- **Parked:** named gate or other lane owns it.
- **Blocked:** missing bind, shape, or Capt. unpark.

## A — Language / DevRel (Vibe)

| # | Item | State | Evidence |
|---|---|---|---|
| A1 | Diagnose-first DevRel (`suggested_fix` + evidential on contradiction) | Implemented | `diagnose.rs` JSON now includes `errors[]` on failure; evidential on E100/E200/E700; `docs/vibe/devrel-frozen-host.md` |
| A2 | Fixture pack graph · volume · infer · render · diagnose | Implemented (graph/volume/diagnose + existing Render) | `crates/vibe/fixtures/FIXTURE_PACK.md`; `sprint_b_fixtures` |
| A3 | Span fidelity (UTF-8 byte span for cell/token glow) | Implemented (byte span) | core §9; `Span { start, end }`; cross-frame Timeline still B-007 |
| A4 | Human ↔ agent dialect guide | Document landed | `docs/vibe/devrel-frozen-host.md` §2 |
| A5 | G-COORD dialect | Remap bind landed | `Cosmic.*` in-process + SPARQL; UTF-8 labels; QDNF wait |
| A6 | Preview still/clip/scene on `Render.*` | Implemented (remap) | `vibe-catalog-honesty.md`; `render_preview_handles.vibe` |
| A7 | Catalog honesty (dual-VC, QISP, ledger vs showcase, aspirational bridges) | Implemented (honesty notes) | `vibe-catalog-honesty.md`; ALL_BOUND ⊂ Vibe catalog |
| A8 | Empty typography + custom unicode API | Implemented (session overlay, not Host) | `icon_session.rs` 32 PUA slots; B-009 done |
| A9 | Webizen Desktop language checklist | Implemented (four-op host) | `webizen-vibe-host-parity.md`; chrome extract still later |
| A10 | G-SOLID-IDP DevRel | Parked | Solid is a Qualia **exit adapter**, not the IdP/architecture |
| A11 | InvokeId annotation pack | Document landed | `poet-ontology-join-contract.md` + `poet-surface.shacl.ttl` |
| A12 | Position on vibe cells/modules | Document landed; bind gated | G-COORD shapes; live CRS invoke still owner-gated |

## B — Seams / Rust (Neo)

Thin `poet::vibe_host` + wasm parity landed. No Host widen. Next toolchains and G-COORD **bind** stay Neo/Capt. QDNF native bearer is a separate programme (`qualia-decentralized-network-fabric/`).

## C / D — Poet chrome / visual

Davinci/Monet chrome delta landed 2026-09-05 (`15-studio-chrome.css`). Next toolchain Capt. pick landed (`office:shapes`). Remaining: G-COORD extra invoke gate, browser UAT.

## E — Ontology (Marvin) — standing constraints only

E12–E14 (persons / living-natural / life-science OWL uplift) are already locked in sprint deltas as `B-OWL-*`. Shape publish is Marvin, not Vibe.

## Sprint B stages

| Stage | State | Notes |
|-------|-------|-------|
| 0 Hygiene | Implemented this packet | crate `0.0.36-dev`; EBNF ↔ core §3 test; B-006 closed |
| 1 Diagnose / DevRel pack | Implemented this packet | contract doc, dialect guide, fixture pack |
| 2 Toolchest language joins | Implemented (scopes live or gated) | Next *new* toolchain still Capt. pick |
| 3 G-COORD dialect | Shapes landed; bind gated | Spatial/realm only — DNS/IP is QDNF |
| 4 Preview / temporal | Implemented (remap + chrome) | B-007; still/clip/scene on live `Render.*` |
| 5 Catalog honesty backlog | Implemented | `vibe-catalog-honesty.md`; lockstep test in `ids.rs` |
| 6 G-SOLID-IDP | Parked | Qualia-first; Solid is exit only |
| 7 Webizen Desktop language | Four-op host live; chrome extract later | `vibe_host.rs` |

## Non-goals (unchanged)

Host widen · dotted `qualia.*` · fake durable save/preview · free tweens · G-COORD claiming DNS/IP (QDNF owns that) · Solid as the architecture · mid-flight API churn
