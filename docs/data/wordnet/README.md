# Princeton WordNet dataset (release asset)

The full Princeton WordNet 3.1 graph (`princeton.q42`, ~127 MB) is **not** stored in git.
It is published as a [GitHub Release](https://github.com/mediaprophet/qualiaDB/releases) asset
for tag `v$(cat VERSION)`.

## Local setup

**Download from release (recommended):**

```bash
bash scripts/fetch_wordnet_release.sh
```

```powershell
.\scripts\fetch_wordnet_release.ps1
```

**Build from source RDF:**

```powershell
$env:QUALIA_PRINCETON_RDF = 'C:\path\to\wordnet.rdf'
.\scripts\ingest_princeton_wordnet.ps1
```

## CI

- **Release workflow** (`v*` tags): builds and uploads `princeton.q42` when `WORDNET_RDF_URL` is set.
- **Pages workflow**: downloads `princeton.q42` from the release named in `VERSION` before deploy.