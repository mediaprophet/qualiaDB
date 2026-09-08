# Q1/Q2 incorporation wave 28 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Complete (integrated)**  
**Prior:** wave 27 Complete — Inference remainder×5 · Research live first×20  
**Derived inventory after w27:** `ALL_BOUND=1102` · `PoetLive≈823` · Q2 ≈ 279

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-RESEARCH` | Research investigation/hypothesis/assessment ×20 | `research_live2_chain_actions.rs` + `register_research_live.rs` |
| B `Q1-HOST` | none | Live-first |

**Never `Proficiency::Advanced`.** Dual-path. Exact Host scopes.

## Parent verify

```
cargo test -p poet --lib wave28
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
```

Expected after this wave: `ALL_BOUND=1102` · `PoetLive≈843` · `Q2≈259`.

## Verification (parent)

- `cargo test -p poet --lib wave28`: **2 passed**
- `cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy`: **1 passed**
- `cargo test -p poet --test product_integrity`: **11 passed**
- Derived counts: `ALL_BOUND=1102` · `PoetLive≈843` · `Q2≈259`

## Next

Research remainder (~33) on `research:live`, then Render CPU, CG leftovers, long-tail.
