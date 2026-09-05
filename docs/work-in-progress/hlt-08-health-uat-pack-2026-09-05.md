# HLT-08 — Health completion UAT pack (2026-09-05)

**Status:** executable source contracts landed · browser evidence in progress  
**Dependency:** `HLT-R1`, `HLT-07`  
**Freeze:** `vibe-host-0.1`  
**Branch:** `cursor/poet-grok-handover-ac52`

Each row must have exact evidence. No completion claim may exceed observed
host behavior. Review Gate A (`D5`) is **not** closed by this pack.

## Workflows

| ID | Task | Source contract | Browser evidence | Result |
|---|---|---|---|---|
| U1 | Add measurement | `overview_workspace.rs` save requires time + sys > dia; empty placeholders | Health overview opened offline; inner form not fully zoomed for field-level clicks | Source PASS; browser partial |
| U2 | Reload | `data-health-refresh` re-queries COP families | Refresh chrome present on health surfaces; live reload not exercised (no daemon) | Source PASS; browser held |
| U3 | Inspect trend / table | `vitals_chart.rs` metric tabs + chart/table toggle | Empty timeline expected offline; tabs need saved vitals | Source PASS; browser held |
| U4 | Correct record | append-only `health_correction` receipt; original not erased | Correction modal needs an existing record | Source PASS; browser held |
| U5 | Grant access | named contact + five `ConsentScope` flags; no `clinical_notes` | Disclosure workspace present; Grant not live-signed (no daemon, no ConsentLedger persist) | Source PASS; browser held |
| U6 | Revoke access | `data-revoke-grant` one-action | No active grant to revoke offline | Source PASS; browser held |
| U7 | Ingest report text | paste extract; binary upload disabled | Documents container visible with PDF-extract / provenance chrome | Source PASS; browser partial |
| U8 | Offline recovery | mutation held; calculators invent no score | Graph/Merkle/Gas unavailable; Calculate disabled; no invented score | PASS |

## How to run

```bash
cargo +stable test -p poet --test health_uat_pack
```

## Known remaining seams (not defects of this pack unless UAT shows a lie)

- Poet grant persist still upserts JSON; it does not call `ConsentLedger::issue` / `revoke`.
- Live add/grant/ingest require a running QualiaDB daemon on `:4242`.
- MCP medical Framingham defaults and WebizenVM SCORE2 Moderate hardcode are out of this Poet path.

## Out of scope

G-COORD bind · Solid IdP · WordNet · EBNF `lexicon:` · QDNF · Host widen · Gate A close
