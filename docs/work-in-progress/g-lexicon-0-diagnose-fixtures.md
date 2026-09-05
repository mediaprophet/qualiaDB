# G-LEXICON-0 — diagnose / lexicon pin fixtures (Vibe)

**Status:** landed (fixtures + diagnose hooks) · **Freeze:** `vibe-host-0.1`  
**Parent tip:** `6df915a0` (G-LEXICON-0 accept `641c2460` / bind `720bd5a9`)  
**Gate:** additive · **no Host widen** · **no dotted `qualia.*`** · **no in-binary WordNet**  
**Owner:** Vibe · Seam: Neo (`GraphDatabase.lexicon_manifest`) · Shape: Marvin

`lexicon:` is **not** an EBNF production. Pin is package metadata: comment/header
`// lexicon: "packId@SemVer"` and/or `const lexicon = "packId@SemVer";`.
`vibe::diagnose` stays parse+check (no disk, no invoke). Missing-pack E300 is
documented by `vibe::diagnose_lexicon_pin` for Neo to map the live bind onto.

## Fixture paths

| Path | Covers |
|------|--------|
| `crates/vibe/fixtures/lexicon/missing_pack.vibe` | A — missing pack pin (`missing-core@0.0.0`) + `lexicon_manifest` call |
| `crates/vibe/fixtures/lexicon/missing_pack.diagnose.json` | A — expected DiagnoseReport JSON (E300 · held / not yet · open lexicon pack) |
| `crates/vibe/fixtures/lexicon/pin_ok.vibe` | B — recorded pin `en-core@0.1.0` |
| `crates/vibe/fixtures/lexicon/en-core.lexicon.json` | B — byte-match of `docs/manuals/standards/lexicon-pack-manifest-example.json` |
| `crates/vibe/fixtures/lexicon/alias_rows.json` | C — alias rows `{from, to, framing}` |
| `crates/vibe/fixtures/lexicon/upgrade_living.json` | D — living-SHACL must not become artifact-OWL on upgrade |

Hooks: `crates/vibe/src/lexicon_pin.rs` (`diagnose_lexicon_pin`, pin parse, alias
round-trip, upgrade framing). Catalog sync only: `GraphDatabase.lexicon_manifest`
added to `ALL_INVOKE_IDS` (already in `ALL_BOUND` — not a new Host method).

## Accept criteria

1. **A — Missing pack / E300.** `diagnose_lexicon_pin(missing_pack.vibe, false)` →
   `valid: false`, `kind: "module"`, `error_code: "E300"`, `suggested_fix` contains
   `held / not yet` and `open lexicon pack`. Never “broken”. `diagnose()` of the
   same file stays valid (parse+check does not invoke).
2. **B — Valid pin.** `parse_lexicon_pin_from_source(pin_ok.vibe)` records
   `en-core@0.1.0`. Example pack fields: `packSemVer: 0.1.0`, `framing: mixed`,
   concept ids arrive/hold/leave. Fixture JSON matches the standards example.
3. **C — Alias round-trip.** `suggested_fix` JSON array of
   `{from, to: conceptId, framing: living-SHACL\|artifact-OWL\|mixed}` parses
   back to the same rows.
4. **D — Living not Thing-washed.** Upgrade map requesting artifact-OWL on a
   living-SHACL row keeps `living-SHACL`.
5. **Freeze.** `LANGUAGE_VERSION == vibe-0.1`, `HOST_VERSION == vibe-host-0.1`.
   Four-ops unchanged. Workshop files have no `qualia.*` and no agent
   `capability.invoke`.

## How to run

```bash
cargo test -p vibe --lib lexicon
cargo test -p vibe --lib diagnose
cargo test -p vibe --test sprint_b_fixtures
```

`--lib` also needs `crates/vibe/fixtures/vocab/clinic.n3` (pre-existing `include_bytes` in `check.rs`; restored from the in-module clinic chunk so lib tests compile).

## Out of scope

Full WordNet engine · Host widen · EBNF `lexicon:` production · locale packs
beyond en · Solid · G-COORD bind
