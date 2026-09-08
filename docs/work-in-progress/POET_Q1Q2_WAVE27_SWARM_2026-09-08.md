# Q1/Q2 incorporation wave 27 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Complete (integrated)**  
**Prior:** wave 26 Complete — Asset persist_*×7 · Pulse live×9 · Portal/Avatar×5 Live; Host math-geometry×8  
**Derived inventory after w26:** `ALL_BOUND=1102` · `PoetLive≈798` · Q2 ≈ 304

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-INF` | Inference remainder (5) | extend `ai:inf` + `inference_remainder_chain_actions.rs` |
| B `Q2-RESEARCH` | Research enquiry/corpus/dark-link/inference first 20 | `research_live_chain_actions.rs` + chain `research:live` |
| C `Q1-HOST` | none this wave | Live-first to close Q2; no new Host binds |

**Never `Proficiency::Advanced`.** Dual-path. Exact Host scopes.

Chain id `research:live` (spec hyphenated `research:*` already occupies that family).

## Parent verify

```
cargo test -p poet --lib wave27
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
cargo test -p qualia-core-db --lib vibe_catalog_contains_every_bound_invoke_id
```

Expected after this wave: `ALL_BOUND=1102` · `PoetLive≈823` · `Q2≈279`.

## Verification (parent)

- `cargo test -p poet --lib wave27`: **4 passed** (research defaults, remainder parse, research:live binds, ai:inf remainder binds)
- `cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy`: **1 passed**
- `cargo test -p poet --test product_integrity`: **11 passed**
- `cargo test -p qualia-core-db --lib vibe_catalog_contains_every_bound_invoke_id`: **1 passed**
- Derived counts: `ALL_BOUND=1102` · `PoetLive≈823` · `Q2≈279`

## Next

Research remainder (~53) on `research:live`, then Render CPU, CG Host leftovers Live, Image/Animation/long-tail.
