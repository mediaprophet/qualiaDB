# G-B-001 — Volume shape (v0)

**Tip:** `fdbcbfd` · **Binds:** `GraphDatabase.volume_open` · `GraphDatabase.volume_commit`
**Owner:** Marvin · Seam: Neo · Chrome: davinci/monet

## Class: `q42:Volume`
Grounded in `crates/qualia-core-db/src/q42/volume/`.

| Property | Notes |
|----------|--------|
| `q42:pathOrHandle` | path or opaque handle |
| `q42:sanctuary` | fail-closed; default on commit |
| `q42:openedVia` | `vibe:InvokeId` = `GraphDatabase.volume_open` |
| `q42:committedVia` | `vibe:InvokeId` = `GraphDatabase.volume_commit` |
| `q42:state` | closed · open · committed · denied · fault |

Join: Container/Manifold backing store; Layout→Stage→Timeline commit twin; wasm E300 honest/gated.
Non-goals: no dotted `qualia.volume.*`, no Host widen.
