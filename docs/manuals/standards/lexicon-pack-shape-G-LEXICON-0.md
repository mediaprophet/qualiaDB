# G-LEXICON-0 — Lexicon pack shape (v0)

**Tip:** `720bd5a9` · **Bind:** `GraphDatabase.lexicon_manifest`
**Owner:** Marvin · Seam: Neo · Language: Vibe · Chrome: davinci/monet

## Class: `vibe:LexiconPack` (artifact machinery — OWL-ok)
Resolved via volume path / `.q42` + `<stem>.lexicon.json` sidecar. Not in dialect binary.

| Property | Required | Notes |
|----------|----------|--------|
| `packId` | yes | stable pack name |
| `packSemVer` | yes | SemVer; pin as `lexicon: "packId@SemVer"` |
| `framing` | yes | `living-SHACL` \| `artifact-OWL` \| `mixed` |
| `upliftFrom` | no | source lexicon IRI + provenance (WordNet/GO/OBO/…) |
| `conceptIds` | yes | list of stable concept ids |
| `aliases` | no | `{ from, to: conceptId, framing }` for migrate/`suggested_fix` |
| `localeSurfaces` | no | concept → locale string map (en first) |

## Framing rules
- **living-SHACL:** senses for person/kin/country/life-scale — never under `owl:Thing`
- **artifact-OWL:** tool/volume/file/catalog senses OK
- **mixed:** each conceptId carries its own framing; upgrades must not Thing-wash living senses

## Diagnose join
- Missing/unknown pack → **held / not yet** (`E300` + `suggested_fix`) via live bind
- Removed living sense on bump → held + alias row; never silent reinterpret as artifact

## Non-goals
Full WordNet engine · Host widen · dotted `qualia.*` · in-binary lexicon dump
