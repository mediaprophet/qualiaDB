# Q1/Q2 incorporation wave 24 swarm — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** **Wiring complete — verification pending**  
**Prior:** wave 23 Complete — Dmx 14 + Video 10 + HID 16 Live; Host CG +8  
**Derived inventory after w23:** `ALL_BOUND=1078` · `PoetLive≈728` · Q2 ≈ 350

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A `Q2-VC` | Exhaust `VectorCalculus.*` Q2 (7) | `vc_chain_actions.rs` + chain `scientific:vc` |
| B `Q2-INTERP` | Exhaust `Interpolation.*` Q2 (6) | `interp_chain_actions.rs` + chain `scientific:interp` |
| C `Q2-SPECTRAL` + `Q2-WORLD` | Exhaust `Spectral.*` (5) and `World.*` (7) | `spectral_chain_actions.rs` + `world_chain_actions.rs` |
| D `Q1-HOST` | 8 CG coreset/duality/boolean predicates | `geometry/wave24_host.rs` + paired catalogs |

### Shared-file rule (avoid merge collisions)

Lanes **must not** edit:

- `crates/poet/src/browser/tool_actions.rs`
- `crates/poet/src/browser/tool_copy.rs`
- `crates/poet/src/browser/registration/mod.rs`

Parent wires `has_live_invoke`, dispatch arms, `tool_copy`, and registration asserts after lanes land.

**Never `Proficiency::Advanced`.** Dual-path. Exact Host scopes. No Host invent on Live lanes.

## Parent verify

```
cargo test -p poet --lib wave24
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
cargo test -p qualia-core-db --lib wave24
cargo test -p qualia-core-db --lib vibe_catalog_contains_every_bound_invoke_id
```

Expected after this wave: `ALL_BOUND=1086` · `PoetLive≈753` · `Q2≈333`.
