# G-COORD — CoordinateSystem · Realm · Position (v0)

**Packet:** W5 · **Status:** shapes + dialect landed; **bind gated**  
**Rule:** G-COORD does not replace DNS/IP. Network identity, adjacency, and routing without DNS/IP live in **QDNF** (`qualia-decentralized-network-fabric/`). `did:q42` here is a storage/QRC locus, not a DNI. Locus is not a person.

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

## Bind (thinnest)

| Want | Live fit | Action |
|------|----------|--------|
| Geo query | `GraphDatabase.sparql` | remap |
| Celestial math | `Cosmic.*` | remap |
| GPU camera | `Render.gpu_set_camera` | remap |
| Dedicated CRS invoke | none | **gated** — do not add `qualia.coord.*` |

A new `ALL_BOUND` id is only justified if none of the remaps can carry realm +
position args. That add is a Capt. catalog decision, not this packet.

## Chrome

Map containers: one geo path + one non-geo realm skin, **gated** until a live
query/bind is selected. Temporal scrubber is Timeline twin, not a Host clock.
