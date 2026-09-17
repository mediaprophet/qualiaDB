# Work graph indexer (Beat B)

**Status:** MVP scaffold · **Branch target:** `0.0.38` · **No Host invent**  
**Crate:** `crates/work-graph` · **Binary:** `work-graph`

Companion to [`IMPLEMENTATION_GRAPH_SCOPE_WIP.md`](IMPLEMENTATION_GRAPH_SCOPE_WIP.md).
Customer name: **Work graph**. Honesty: **Present / Live / Planned**. This beat
emits **Present** only.

## Run

From the QualiaDB checkout:

```bash
cargo run -p work-graph -- index
```

Optional:

```bash
cargo run -p work-graph -- index . --out target/work-graph
cargo run -p work-graph -- index /path/to/qualiaDB --out docs/_generated/work-graph
```

Default out: `target/work-graph/` (gitignored via `/target`).

| Artifact | Use |
|----------|-----|
| `impl-graph.nq` | N-Quads Capt / Vibe can query later |
| `impl-graph.json` | Compact counts, families, invoke ids, doc cites |

## Grounding (no invent)

- Tip from `git rev-parse` / `git status --porcelain`.
- Files from `git ls-files`.
- Invoke ids resolved from `ALL_BOUND` in `crates/qualia-core-db/src/poet_host/invoke/ids.rs`.
- Catalog ids from `crates/vibe/src/catalog/ids.rs` `ALL_INVOKE_IDS` when present.
- Doc cites only for ids/families already in that extract (`Family.` prefix for families).

Does **not** add `ImplGraph.*` Host ids. Prefer existing
`CapabilityDiscovery.catalog` on the Host side.
