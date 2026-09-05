# HLT-07 — Clinical calculator workflow integrity (2026-09-05)

**Status:** implementation complete · native + Poet tests passed · browser offline UAT partial  
**Branch:** `cursor/poet-grok-handover-ac52` off `0.0.36-dev`  
**Freeze:** `vibe-host-0.1` — four ops / live `ALL_BOUND` only  
**Live IDs:** `ClinicalRisk.framingham` · `ClinicalRisk.cha2ds2_vasc` · `ClinicalRisk.score2`

## Outcome

Native invoke no longer fabricates patient values. Missing fields, omitted
booleans, out-of-band ages, missing SCORE2 region, CHA₂DS₂-VASc without
atrial fibrillation, and HDL ≥ total cholesterol fail closed. Success
records name algorithm, version, citation, applicability, units, and
`not_diagnosis`.

Poet places an empty calculator form. Calculate stays disabled until the
form is complete and the local QualiaDB daemon is connected. Offline, no
score is invented.

## How to run

```bash
cargo +stable test -p qualia-core-db --lib invoke::clinical
cargo +stable test -p qualia-core-db --lib health_is_not_a_named_person
cargo +stable test -p poet --lib health_views
cargo +stable test -p poet --test product_integrity --test surface_inventory
```

Measured here (rustc 1.98.1): **16** `invoke::clinical` passed; **1** health
scene passed; **33** `health_views` passed; product integrity **10**; surface
inventory **1**; capability-scope and non-placement policy tests passed.

## Browser UAT (offline; daemon not running)

`NO_COLOR=true RUSTUP_TOOLCHAIN=stable trunk serve --port 8081`  
Help → Command Palette → Open Construct: Health.

Observed on the Health construct: Clinical calculators container **Risk estimates from entered values**; Calculate disabled; copy that empty fields are not patient values and the result is not a diagnosis; status bar Graph/Merkle/Gas **unavailable**. No numeric risk was shown. A complete Framingham fixture against a live daemon was not run (no `:4242` daemon in this environment).

## Not claimed

- Review Gate A (`D5`)
- Live daemon invoke of a complete Framingham / CHA₂DS₂-VASc / SCORE2 fixture
- MCP `mcp_tool_impls/medical.rs` Framingham `unwrap_or` defaults (out of Poet invoke)
- `governance/webizen/vm.rs` SCORE2 Moderate hardcode (different path)
