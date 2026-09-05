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
| U1 | Add measurement | `overview_workspace.rs` save requires time + sys > dia; empty placeholders | pending | |
| U2 | Reload | `data-health-refresh` re-queries COP families | pending | |
| U3 | Inspect trend / table | `vitals_chart.rs` metric tabs + chart/table toggle | pending | |
| U4 | Correct record | append-only `health_correction` receipt; original not erased | pending | |
| U5 | Grant access | named contact + five `ConsentScope` flags; no `clinical_notes` | pending | |
| U6 | Revoke access | `data-revoke-grant` one-action | pending | |
| U7 | Ingest report text | paste extract; binary upload disabled | pending | |
| U8 | Offline recovery | mutation held; calculators invent no score | pending | |

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
