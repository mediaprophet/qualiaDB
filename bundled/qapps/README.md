# Bundled qapps (tracked release source)

Desktop installers copy qapps from here into `{dist}/bundled/qapps/{Name}/` via
`scripts/copy-bundled-qapps.ps1` / `copy-bundled-qapps.sh`.

The Flutter shell seeds these into user storage on first launch (`bundled_qapps.rs`).

| Qapp | Purpose |
|------|---------|
| `Anatomy/` | Health visualization, chat handoff, DICOM overlay, knowledge catalog |

Legacy dev copies may still exist under gitignored `app-development/Anatomy/`; the
tracked canonical tree is `bundled/qapps/Anatomy/`.
