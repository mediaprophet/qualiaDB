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
| A5 | G-COORD dialect | Shapes landed; bind gated | `g-coord-coordinate-system-shapes.md` |
| A6 | Preview still/clip/scene on `Render.*` | Implemented (remap) | `vibe-catalog-honesty.md`; `render_preview_handles.vibe` |
| A7 | Catalog honesty (dual-VC, QISP, ledger vs showcase, aspirational bridges) | Implemented (honesty notes) | `vibe-catalog-honesty.md`; ALL_BOUND ⊂ Vibe catalog |
| A8 | Empty typography + custom unicode API | Parked | `B-009` in sprint deltas with accept criteria |
| A9 | Webizen Desktop language checklist | Parked | After Poet / Webizen gate (Stage 7) |
| A10 | G-SOLID-IDP DevRel | Parked | Capt. unpark (Stage 6) |
| A11 | InvokeId annotation pack | Parked | Marvin E8; Vibe consumes after shapes exist |
| A12 | Position on vibe cells/modules | Parked | Marvin E10 + Stage 3 |

## B — Seams / Rust (Neo) — not this packet

Keep thin `poet::vibe_host` + wasm parity. No Host widen. Next toolchains and G-COORD bind stay Neo.

## C / D — Poet chrome / visual — not this packet

Davinci and Monet items remain on their impl plans. Do not start them as a substitute for the language pack.

## E — Ontology (Marvin) — standing constraints only

E12–E14 (persons / living-natural / life-science OWL uplift) are already locked in sprint deltas as `B-OWL-*`. Shape publish is Marvin, not Vibe.

## Sprint B stages

| Stage | State | Notes |
|-------|-------|-------|
| 0 Hygiene | Implemented this packet | crate `0.0.36-dev`; EBNF ↔ core §3 test; B-006 closed |
| 1 Diagnose / DevRel pack | Implemented this packet | contract doc, dialect guide, fixture pack |
| 2 Toolchest language joins | Parked | After Neo next toolchain |
| 3 G-COORD dialect | Parked | After Marvin + Neo |
| 4 Preview / temporal | Parked | B-007 |
| 5 Catalog honesty backlog | Partial | Volume catalog sync done; dual-VC/QISP/ledger remain |
| 6 G-SOLID-IDP | Parked | Capt. |
| 7 Webizen Desktop language | Parked | After Poet |

## Non-goals (unchanged)

Host widen · dotted `qualia.*` · fake durable save/preview · free tweens · DNS/IP replacement · Solid before Poet/Webizen · mid-flight API churn
