# Container · Manifold · Link · aspects (v0)

**Packet:** W2 / W4 · **Join contract:** `poet-ontology-join-contract.md`  
**SHACL:** `crates/qualia-core-db/shapes/poet-surface.shacl.ttl`  
**Metamodel:** these are **OWL-ok technical artifacts** (studio software objects).
They may *hold* SHACL-first content (a person record, a living-country map) without
becoming that content.

Do **not** overload legal `FormationStage`. Layout / Stage / Timeline here are
Poet surface **aspects** — three readings of one surface, not identical copies.
Not twins. Not planes. Not credential digital twins.

## Classes

### `q42:Container`

Content-shaped work surface (document, graph, sheet, map, cell host, …).

| Property | Notes |
|----------|--------|
| `q42:containerType` | string; chrome kind (`doc`, `graph`, `map`, …) |
| `q42:backedBy` | optional `q42:Volume` |
| `q42:hasPosition` | optional `q42:Position` (W5); language cells may carry this too |
| `q42:layout` / `q42:stage` / `q42:timeline` | 1:1 aspects |
| `q42:openedVia` | `vibe:InvokeId` when live |

### `q42:Manifold`

Nests containers and other manifolds. Optional shared `q42:CoordinateSystem`.

| Property | Notes |
|----------|--------|
| `q42:nests` | Container or Manifold |
| `q42:coordinateSystem` | optional; gated until G-COORD bind |

### `q42:Link`

Typed semantic relation between two ends (containers, manifolds, or named
nodes). Not a wire-only visual.

| Property | Notes |
|----------|--------|
| `q42:linkFrom` / `q42:linkTo` | required ends |
| `q42:linkType` | IRI or prefixed name |

## Aspects (named beats only)

| Aspect | Role | Beat |
|--------|------|------|
| `q42:Layout` | 2D structure | — |
| `q42:Stage` | depth / z / camera | entrance = soft rise |
| `q42:Timeline` | time | dwell · exit; no free tweens |

Every shipped surface has all three **aspects**. They are not identical to each other. Reduced-motion: same beats, shorter/crossfade.

## Volume backing

When a Container sits on a Volume: `q42:backedBy` + Volume `q42:state`
(`closed` · `open` · `committed` · `denied` · `fault`). Commit beat only on
successful `GraphDatabase.volume_commit`. wasm E300 ⇒ `fault` or `denied`, never
`committed`.

## Language cells

A Vibe cell/module is content inside a Container. It MAY carry `q42:hasPosition`
and optional `q42:viewpointRealm` without being a map. Spatiotemporal is a
property of language, not only of GIS chrome. Labels are UTF-8.

## Chrome notes (davinci / monet)

Human chrome names: Container, Manifold, Link. Machine ids stay under the hood.
Gated if the invoke in the join contract is unbound.
