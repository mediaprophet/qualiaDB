# Work graph — Beat B MVP indexer

Customer name: **Work graph**. Working id: Implementation Graph.

Thin presence indexer for a QualiaDB checkout. It does **not** invent Host ids
(`ImplGraph.*` is forbidden in MVP). Complementary to ChatGraph.

Honesty vocabulary: **Present** / **Live** / **Planned**. This CLI emits
**Present** only (found in the tree). Presence is not Live. No held / not-yet
amber theatre.

## Run

From the QualiaDB repo root:

```bash
cargo run -p work-graph -- index
```

Or after `cargo build -p work-graph`:

```bash
./target/debug/work-graph index
./target/debug/work-graph index /path/to/qualiaDB --out target/work-graph
```

Defaults:

- root: current directory (must contain `crates/qualia-core-db/src/poet_host/invoke/ids.rs`)
- out: `<root>/target/work-graph`

Writes:

| File | What |
|------|------|
| `impl-graph.nq` | N-Quads: project tip, files, crates, ALL_BOUND invoke ids, families, doc cites |
| `impl-graph.json` | Compact summary Capt / Vibe can load later |

## What is indexed

1. **Git tip** — SHA, branch, dirty flag.
2. **File list** — `git ls-files` (Present artifacts).
3. **Crates** — package names from tracked `Cargo.toml` (skips `vendor/`).
4. **ALL_BOUND** — resolved from `poet_host/invoke/ids.rs` (same SoT Capt uses).
5. **Vibe catalog** — `crates/vibe/src/catalog/ids.rs` `ALL_INVOKE_IDS` when present.
6. **Doc cites** — `docs/**/*.md` / `docs/**/*.html` (plus README / skills) for exact invoke ids and `Family.` prefixes.

No rust-analyzer symbols, notes, Live stamps, or Host widen in this beat.

## Query later

Load `impl-graph.nq` or `impl-graph.json`. Do not add `ImplGraph.*` to
`ALL_BOUND`. Prefer `CapabilityDiscovery.catalog` on the Host side.

See `docs/work-in-progress/IMPLEMENTATION_GRAPH_SCOPE_WIP.md` and
`docs/work-in-progress/WORK_GRAPH_INDEXER.md`.
