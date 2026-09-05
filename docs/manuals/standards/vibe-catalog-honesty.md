# Vibe catalog honesty (W1 / B-002–B-005 / B-007)

**Date:** 2026-09-05 · **Freeze:** `vibe-host-0.1`  
**Live list:** `crates/qualia-core-db/src/poet_host/invoke/ids.rs` `ALL_BOUND`  
**Language list:** `crates/vibe/src/catalog/ids.rs` `ALL_INVOKE_IDS`

## Diff result (2026-09-05)

Every `ALL_BOUND` string is present in the Vibe catalog (885 unique host ids).
`GraphDatabase.volume_open` / `volume_commit` were the last host ids missing from
Vibe and are now catalogued. Vibe may list additional local kernels
(`biosignal.dp_*`) that LocalHost can run without a Poet bind; those are not
Host methods and are not dotted `qualia.*` IRIs.

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
| Native `GraphDatabase.volume_commit` | durable when sanctuary permits |
| wasm / LocalHost volume invoke | `honesty: local` or E300 — not a saved `.q42` |
| SPARQL showcase pages | recorded engine version in the page; not live-unless-connected |
| Inference chrome | live `Inference.*` or gated |

A demo that cannot open a volume must say unavailable, not “saved”.
