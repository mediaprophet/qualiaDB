# Skill: VibeScript (`vibe-0.1`)

You are authoring **Vibe**, Qualia's typed graph/document language. Poet is the engine. Do not invent JavaScript or Python APIs.

## Read first

1. `docs/manuals/standards/vibescript-core.md` — grammar, types, effects, §12/§13 fixtures.
2. `capability.invoke("CapabilityDiscovery.coverage", null)` — what is actually bound.
3. `capability.invoke("CapabilityDiscovery.catalog", null)` — Turtle of live names.

## Hard rules

- Quin construction is `quin.statement(...)` only. `<<[ s p o g prov ]>>` is illegal.
- RDF 1.2 only: `<<( s p o )>>` and `<< s p o ~ reifier >>`.
- `pulse.publish` / `aura.validate`. Not `pulse.broadcast` / `aura.apply_schema`.
- `graph.query` always has `take: N`. Cells (`= …`) are Pure — no pulse, write, or time.
- Diagnostics are `E001` parse, `E100` type, `E200` effect, `E300` capability, `E400` budget, `E500` policy, `E600` eval.
- Call `vibe::diagnose(src)` (or the desktop Poet harness) and repair from `suggested_fix`. Do not execute invalid source.
- Engine families beyond 0.1 bindings: `capability.invoke("Family.op", args)` — do not add keywords.
- Canonical 10D extension is `.10d`. Classic UI stays default.

## Grammar artifacts

- `crates/vibe/grammar/vibe-0.1.ebnf`
- `crates/vibe/grammar/vibe-0.1.gbnf` (in-process constrained decode, not Ollama)
- `crates/vibe/grammar/source.schema.json`

## Fixtures that already pass

- `crates/vibe/fixtures/12_1_cell.vibe` — clamp a score
- `crates/vibe/fixtures/12_2_clinic.vibe` — graph stage + SHACL + pulse
- `crates/vibe/fixtures/12_3_count.vibe` — query with `take`
- Reject: `n1_nospace_lt.vibe`, `n3_quin_overlay.vibe`

NL pairs: `crates/vibe/fixtures/nl/`.

Related (not the language spec): `docs/plans/native-presentation-and-vibe-beyond-webview-2026-08-16.md` — HID / WebView / WASM presentation. Destination pair: **CBOR-LD on the wire**, **Vibe instead of JS**; Dioxus/JSON IPC is temporary host.
