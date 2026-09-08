# Q1/Q2 incorporation wave 37 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Complete (pending parent verify)**  
**Prior:** wave 36 Complete — Social/Finance/ChatGraph leftovers  
**Derived inventory after w36:** helper-aware Q2 ≈ 65

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-GRAPH` | GraphMatch×3 · GraphReasoning×3 · Optimization×3 | `scientific:graph_reason` |
| B `Q2-SAMP` | sampler×5 · Capability×5 | `ai:sampler_live` |
| C `Q1-HOST` | none | Live-first |

## Parent verify

```
cargo test -p poet --lib wave37
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
```

Expected after this wave: helper-aware Q2 ≈ 46.

## Next

Medical/Manifold/crypto, then remaining singles (`LinearAlgebra.gemm` Live only — no Host-widen).
