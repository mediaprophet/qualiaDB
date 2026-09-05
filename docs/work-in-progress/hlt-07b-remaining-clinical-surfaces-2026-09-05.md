# HLT-07b — Remaining clinical-risk surfaces fail closed (2026-09-05)

**Status:** implemented · focused tests pending in this revision  
**Branch:** `cursor/poet-grok-handover-ac52` off `0.0.36-dev`  
**Freeze:** `vibe-host-0.1` — no Host widen  
**Does not close:** Review Gate A (`D5`)

## Outcome

The remaining calculator paths that still invented patient values now fail
closed, matching native `ClinicalRisk.*` invoke:

1. **MCP `clinical_risk`** — extracted from `medical.rs` into
   `mcp_tool_impls/clinical_risk.rs`. Missing `score`, omitted booleans,
   incomplete Framingham/SCORE2 lipids, CHA₂DS₂-VASc without
   `atrial_fibrillation: true`, missing SCORE2 `risk_region`, and incomplete
   SOFA/eGFR inputs return `InvalidParameters`. Success includes algorithm,
   version, citation, and `not_diagnosis`. Unknown `score` is not Framingham.
2. **WebizenVM `NativeClinicalRisk`** — a `VmFrame` cannot carry a complete
   clinical input. The opcode **holds** (logs under `vm_tracing`) instead of
   hardcoding lipids, SBP, `Score2Region::Moderate`, or `Default` CHA₂DS₂
   booleans. Logic lives in `governance/webizen/clinical_native.rs`.
3. **WASM playground** — `clinical_playground.rs` (native-testable). Incomplete
   JSON cannot calculate. Playground HTML presets are complete **labeled
   reference profiles**, not partial patient-looking defaults.

## How to run

```bash
cargo +stable test -p qualia-core-db --lib clinical_risk
cargo +stable test -p qualia-core-db --lib clinical_playground
cargo +stable test -p qualia-core-db --lib clinical_native
cargo +stable test -p qualia-core-db --lib invoke::clinical
```

## Not claimed

- Review Gate A (`D5`)
- Live daemon Poet Framingham / grant / ingest UAT
- Poet persist still does not call `ConsentLedger::issue` / `revoke`
- `wasm_bridge/medical.rs` D’Agostino 2008 path (serde already requires all
  Framingham fields; algorithm provenance still differs from Wilson ATP-III
  invoke)
- Closing Gate A or starting `AST-*`
