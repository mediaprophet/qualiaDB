# Q1/Q2 incorporation wave 36 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Complete (pending parent verify)**  
**Prior:** wave 35 Complete — GPU leftover Q2 exhausted  
**Derived inventory after w35:** helper-aware Q2 ≈ 81

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-SOCIAL` | Social×4 · Forensic×2 | `social:live` in econ toolbox |
| B `Q2-FIN` | Finance×3 | `finance:live` in econ toolbox |
| C `Q2-GRAPH` | Corpus×2 · ChatGraph×2 · Interactive×2 · SecondScreen×1 | `comm:graph_live` |
| D `Q1-HOST` | none | Live-first |

## Parent verify

```
cargo test -p poet --lib wave36
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
```

Expected after this wave: helper-aware Q2 ≈ 65.

## Next

GraphMatch/GraphReasoning/Optimization/sampler/Capability, then Medical/Manifold/crypto and remaining singles.
