# G-LEXICON-0 — slice 1 (Neo)

**Status:** landed (code) · chrome waits on this SHA · Vibe diagnose fixtures next  
**Gate:** additive under `vibe-host-0.1` freeze · **no Host widen** · **no in-binary WordNet**

## What landed

| Bind | Purpose |
|------|---------|
| `GraphDatabase.lexicon_manifest` | Read a volume-backed lexicon **pack manifest** (JSON). Missing/unknown → diagnose **held / not yet** (`E300` + `suggested_fix`). |

### Args

- `path` (string) — one of:
  - `*.lexicon.json` / `*.json` manifest file
  - `.q42` volume → requires sidecar `<stem>.lexicon.json` and successful `Q42Volume::open`
  - directory containing `lexicon.manifest.json`

### Manifest fields (read)

| Field | Required | Notes |
|-------|----------|--------|
| `packSemVer` | yes | SemVer of the pack |
| `framing` | yes | `living-SHACL` \| `artifact-OWL` \| `mixed` |
| `packId` / `id` | no | pack identity |
| `upliftFrom` | no | prior pack id@SemVer for upgrade recipe |
| `conceptIds` | no | list of concept ids (no sense engine yet) |

### Return record

`pack_id`, `packSemVer`, `framing`, `upliftFrom`, `conceptIds`, `manifest_path`, `volume_path`, `volume_ok`, `gate` (`"open"` on success).

### Diagnose voice

- Never say “broken”.
- Prefer `suggested_fix`: `held / not yet — open lexicon pack` (or framing/JSON variants).
- WASM: native-only (same as `volume_open`).

## Example fixture (fixture only — not shipped WordNet)

```json
{
  "packId": "en-core@0.1.0",
  "packSemVer": "0.1.0",
  "framing": "mixed",
  "upliftFrom": "",
  "conceptIds": ["concept:arrive", "concept:hold", "concept:leave"]
}
```

## Next (out of slice 1)

2. @Vibe — `lexicon:` pin + diagnose alias-map / `lexicon.upgrade` hooks  
3. @Marvin — pack framing shape doc (`packSemVer` · framing · `upliftFrom`)  
4. @davinci @monet — bay chips + held-gate chrome (after this SHA)

Hold: full WordNet engine · locale packs beyond en · next toolchain · Solid · G-COORD (Grok).

## Shape (Marvin)

Promoted: [`docs/manuals/standards/lexicon-pack-shape-G-LEXICON-0.md`](../manuals/standards/lexicon-pack-shape-G-LEXICON-0.md).
