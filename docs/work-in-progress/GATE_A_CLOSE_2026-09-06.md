# Review Gate A — D5 close (2026-09-06)

**Status:** **CLOSED** by project-owner instruction  
**Authority:** Timothy (project owner) — session instruction *“poet implementation is held up because Gate A (D5) is not closed. please fix it.”*  
**Tip at close:** `9909c1b4` on `0.0.36-dev` (PR #75 Health/Tool Chest/RM pack + env cherry-pick)  
**Freeze preserved:** `vibe-host-0.1` — no Host widen

## Decision

Review Gate A for the person-controlled Health programme is **accepted and closed**.
Governed asset work (`AST-*`), portable-app contract packets, and Tool Chest clinical
engine unparking may proceed under their own packet rules. This close does **not**
claim FIPS validation, medical-device clearance, or live-daemon clinician UAT beyond
the evidence listed below.

## Evidence accepted

| Packet | Evidence |
|---|---|
| `HLT-R1` | Consent contract review; replay ledger; fail-closed Poet share projection; grantable UI flags match five `ConsentScope` bits — `docs/work-in-progress/hlt-r1-consent-review-2026-09-05.md` |
| `HLT-07` | Native `ClinicalRisk.framingham` / `.cha2ds2_vasc` / `.score2` require complete inputs; Poet calculator form fail-closed offline — `hlt-07-clinical-calculators-2026-09-05.md` |
| `HLT-07b` | MCP `clinical_risk`, WebizenVM `NativeClinicalRisk`, and playground paths fail closed — `hlt-07b-remaining-clinical-surfaces-2026-09-05.md` |
| `HLT-08` | Eight workflow source contracts in `crates/poet/tests/health_uat_pack.rs`; offline recovery PASS — `hlt-08-health-uat-pack-2026-09-05.md` |
| Integrity | Product integrity / surface inventory carried by PR #75 tip |

## Residuals explicitly accepted (not Gate A reopeners)

1. **Poet grant persist** still upserts JSON (`health_share` / `health_safeguard`) and does **not** call `ConsentLedger::issue` / `revoke`. Cryptographic ledger remains the service contract; wiring persist is a post-gate hardening packet.
2. **Live daemon browser UAT** for add / grant / revoke / complete Framingham fixture was not re-run in this close session. Source contracts + offline UAT + native invoke tests stand until Capt/daemon UAT extends them.
3. **`wasm_bridge/medical.rs` D’Agostino path** provenance differs from Wilson ATP-III invoke; out of Poet clinical form path.
4. Status claims must continue to say **not a diagnosis**; incomplete inputs must not calculate.

## Unblocks

- Tool Chest clinical engines (`health:framingham`, `health:cha2ds2`, `health:score2`) — unparked to live place/invoke against `ClinicalRisk.*`
- Phase 2 programmes: `AST-01+`, portable-app (`APP-*`), post-gate Tool Chest selection (`PFT-03`)
- Review Gate B (Webizen Desktop host) remains a **separate** gate

## How to verify after close

```bash
cargo +stable test -p qualia-core-db --lib consent_contract
cargo +stable test -p qualia-core-db --lib invoke::clinical
cargo +stable test -p poet --test health_uat_pack
cargo +stable test -p poet --test product_integrity --test surface_inventory
```
