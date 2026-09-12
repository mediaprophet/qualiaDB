# Vibe catalog honesty (W1 / B-002–B-005 / B-007)

**Date:** 2026-09-12 · **Host ABI:** `vibe-host-0.1` (outcome, not an `ALL_BOUND` freeze)
**Live list:** `crates/qualia-core-db/src/poet_host/invoke/ids.rs` `ALL_BOUND`
**Language list:** `crates/vibe/src/catalog/ids.rs` `ALL_INVOKE_IDS`
**Tip count:** `ALL_BOUND` length **1121** on branch `0.0.38` at this edit (count the tip in `ids.rs`; do not hardcode ~885).

## Diff result (2026-09-12)

Every `ALL_BOUND` string is present in the Vibe catalog. The 2026-09-05 “885 unique host ids” figure is stale — cite the tip, not ~885.

`GraphDatabase.volume_open` / `volume_commit` were the last host ids missing from
Vibe and are now catalogued. Vibe may list additional local kernels
(`biosignal.dp_*`) that LocalHost can run without a Poet bind; those are not
Host methods and are not dotted `qualia.*` IRIs.

Customer chips: **live** = it runs (if not, it is a bug — finish it); **planned** = not built yet. `held` is an internal gate only when a bind is truly unbound — not customer theatre on shipped work.

## Aspirational → live remap (B-002)

| Do not write | Live bind |
|--------------|-----------|
| `qualia.graph.query` / `qualia.graph.commit` | `GraphDatabase.sparql` · `volume_commit` |
| `qualia.volume.open` | `GraphDatabase.volume_open` |
| `qualia.infer.complete` | `Inference.grounding` / `Inference.verify_turn` / `Inference.run_transformer` (pick the actual method) |
| `qualia.render.preview` | see preview handles below |

Never add the left column to `ALL_BOUND`.

## Preview handles still / clip / scene (B-007)

One `Render.*` family already carries the three kinds. No sibling Host op.

| Handle kind | Live methods | Notes |
|-------------|--------------|-------|
| still | `Render.gpu_render_frame`, `Render.gpu_read_pixels`, `Render.scene` | Single frame / RGBA8 readback |
| clip | `Render.animation_eval_curve`, `Render.animation_eval_preset`, `Render.css_animation` | Time-parameterised; named beats only |
| scene | `Render.gpu_init_surface`, `Render.gpu_upload_mesh`, `Render.gpu_set_camera`, `Render.scene` | Persistent surface + camera |

Cross-frame diagnose spans remain UTF-8 byte ranges on the **source cell**. Timeline
glow maps those bytes per frame; it does not need a new Host method.

## Dual-VC (B-003)

Two presentations of the same agency fact, not two Hosts:

| Class | Proof | Shape home |
|-------|-------|------------|
| W3C VC | VCDM + ML-DSA-65 | `core-ontologies/capability-credentials.n3`, `fiduciary_crypto` |
| Native quin | 48-byte NQuin + Ed25519 | `NQuin` + `agency.rs` |

Do not subclass a Principal under `owl:Thing` to “join” them. Cite both from a
Provenance/Claim shape (wishlist E9) when Marvin publishes it.

## QISP (B-004)

Typed values and tensor predicates live in `sparql_library/immersive/`. Vibe
surfaces them only as `GraphDatabase.sparql` results plus existing
`Manifold.*` / `Render.gpu_upload_tensor` binds. No `qualia.qisp.*` family.

## Ledger vs showcase (B-005)

| Surface | Honest label |
|---------|--------------|
| Native `GraphDatabase.volume_commit` | **live** when sanctuary permits a real write |
| wasm / LocalHost volume invoke | **planned** for durable `.q42` on that surface — or E300 if the bind is truly unbound |
| SPARQL showcase pages | recorded engine version in the page; **live** only when connected |
| Inference chrome | **live** `Inference.*` or **planned** if the chrome is not built |

A demo that cannot open a volume must say **planned** (path not built) or treat a failed live bind as a bug. Do not say “saved”. Do not dress a missing write as a customer “unavailable” chip.
