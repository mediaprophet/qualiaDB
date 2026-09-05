# G-COORD — CoordinateSystem · Realm · Position (v0)

**Packet:** W5 / COORD-01 · **Status:** shapes + dialect + **remap bind landed** (2026-09-05)  
**Rule:** G-COORD does not replace DNS/IP. Network identity, adjacency, and routing without DNS/IP live in **QDNF** (`qualia-decentralized-network-fabric/`) — **wait: that spec is still in design; do not implement QLink/QRoute yet.** `did:q42` here is a storage/QRC locus, not a DNI. Locus is not a person.  
**Script:** all labels, aliases, and authored place names are **UTF-8**. Do not ASCII-fold language. Machine tokens (`earth` / `cosmos` / `fictional`) are identifiers, not display text.

## Classes (OWL-ok technical CRS records)

### `q42:CoordinateSystem`

A named CRS or fictional frame. Grounding, in order:

1. GeoSPARQL / WGS84-class geo when the realm is Earth
2. Existing `did:q42` topological locus (identifier.rs) for native pointers
3. Temporal axis via Allen/LTL already in the engine — not a new Host clock

### `q42:Realm`

| Value | Meaning |
|-------|---------|
| `earth` | Geospatial; OSM-class layers |
| `cosmos` | Celestial / FLRW-class; live `Cosmic.*` if used |
| `fictional` | Authored worlds |
| `speculative` | Counterfactual / hypothetical |
| `viewpoint` | Observer-relative; not a network address |

### `q42:Position`

Realm-scoped coordinate. Properties: `q42:inRealm`, `q42:inSystem`,
`q42:time` (optional). A Vibe cell MAY carry `q42:hasPosition` without being a
map container.

## Vibe dialect (no new keywords)

Human:

```vibe
using GraphDatabase;

effect fn place(query: string) -> List {
    return GraphDatabase.sparql({
        query: query,
        take: 64
    });
}
```

Position as data, not syntax:

```vibe
let here = {
    realm: "earth",
    system: "wgs84",
    lat: -37.8,
    lon: 144.9
};
```

Unknown realm names are ordinary record fields. Diagnose stays E100/E001 for
bad types; there is no `E9xx` coord family until a live bind exists.

## Bind (thinnest) — landed as remap, no new Host id

| Want | Live fit | Action |
|------|----------|--------|
| Geo query | `GraphDatabase.sparql` | remap (empty local kernel; daemon for live graph) |
| Earth position | `Cosmic.geodetic_to_ecef` / `geodetic_distance` | **live in-process** (`vibe::catalog::cosmic`) |
| Celestial math | `Cosmic.body_profile` / `flrw_distance` | **live in-process** |
| Fiction time | `Cosmic.stardate_to_gregorian` | **live in-process** |
| GPU camera | `Render.gpu_set_camera` | remap (preview dock) |
| Dedicated CRS invoke | none | **do not add** `qualia.coord.*` |

## Chrome

Map containers: Earth (geo) + Cosmos + Fiction skins on the remaps above.
Temporal scrubber is Timeline twin, not a Host clock. UTF-8 place labels.
