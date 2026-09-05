# Poet ontology join contract

**Date:** 2026-09-05 · **Packet:** W2  
**Rule:** every Poet surface that claims a capability cites a live `vibe:InvokeId`
from `poet_host/invoke/ids.rs` / `catalog_ttl.rs`. Aspirational dotted IRIs are
not joins.

## Index (reuse before invent)

| Pack | Path | Use |
|------|------|-----|
| Agency / Principal | `crates/qualia-core-db/shapes/qualia-agency.shacl.ttl` | Persons SHACL-first; `sh:not owl:Thing` |
| Values / jural | `core-ontologies/{agency,values,jural,selfhood}.n3` | Rights and custody vocab |
| CML concepts | `core-ontologies/cml.n3` | Text → concept → logic; given world is RDFS+SHACL |
| Rights instruments | `ontologies/*.ttl`, `core-ontologies/concepts/` | Human-rights corpora |
| Volume | `docs/manuals/standards/q42-volume-shape-G-B-001.md` | `q42:Volume` + live volume ids |
| Surface (this programme) | `crates/qualia-core-db/shapes/poet-surface.shacl.ttl` | Container · Manifold · Link · aspects |
| Bundled SHACL | `bundled/ontologies/shacl.ttl` | Startup seed |

## SHACL-first vs OWL-ok

| Kind | Metamodel | Examples |
|------|-----------|----------|
| SHACL-first | RDFS class + SHACL; **not** `owl:Thing` | Principal, personhood, kinship, living/natural, country, creatures |
| OWL-ok | technical artifact | Volume, InvokeId, Container-as-software, CRS, Layout/Stage/Timeline aspects |

Incoming OWL (GO/OBO, RadLex) is an **input format**. Uplift living terms into
SHACL-first shapes; keep instruments/datasets as artifacts.

## Join table (chrome → class → invoke)

| Chrome | Class | Live `vibe:InvokeId` |
|--------|-------|----------------------|
| Graph explore | `q42:Container` (graph view) | `GraphDatabase.sparql` |
| Sanctuary save | `q42:Volume` | `GraphDatabase.volume_open` / `volume_commit` |
| Inference trail | Claim/Provenance on a Container | `Inference.grounding` · `Inference.verify_turn` |
| Preview dock | still/clip/scene handle | `Render.scene` / `Render.gpu_*` / `Render.animation_*` |
| SHACL panel | shape report | `SHACL.validate` |
| N3 rules | module | `N3Logic.evaluate` |

Missing bind → gated chrome + Vibe delta row. Never invent `qualia.*`.
