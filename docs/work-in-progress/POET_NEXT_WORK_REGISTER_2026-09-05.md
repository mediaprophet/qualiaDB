# POET Reconciled Next-Work Register

**Status:** Work in progress  
**Date:** 2026-09-05

This register orders remaining work without assigning exclusive ownership to a
model or agent. Named lanes describe responsibility and review context; any
agent may contribute when it follows the governing contracts and records useful
evidence.

## Immediate sequence

| Order | Work | State | Required evidence | Gate/owner decision |
|---|---|---|---|---|
| 1 | Re-baseline after concurrent changes (`RBL-01`) | Audited; compile blocker found | Baseline `4eade061`; both focused suites fail at the same 16 missing registration modules before tests run | None |
| 2 | Restore registration library ownership (`FIX-REG-01` / `RM-01`) | Complete | Directory-backed module tree; product integrity 9/9; surface inventory 1/1; `trunk build` passed | None |
| 3 | Decompose POET style asset (`RM-02`) | Complete | 14 assets at no more than 421 lines; normalized CSS hash preserved; focused tests, build, desktop/mobile UAT passed | None |
| 4 | Review `HLT-03` consent contract (`HLT-R1`) | Instrument review complete; **D5 Gate A still open** | 12 `consent_contract` tests pass; Poet share projection fail-closed; `clinical_notes` removed from grantable UI flags | Project owner/expert (`D5`) |

| 5 | Complete `HLT-07` clinical calculator integrity | Not started in ledger | Required inputs and units, applicability, boundary tests, algorithm/version provenance, non-advice UI | Higher-assurance implementation/review |
| 6 | Complete `HLT-08` Health UAT pack | Blocked by 4-5 | Executable/manual evidence for add, reload, inspect, correct, grant, revoke, ingest, and offline recovery | Review Gate A |
| 7 | Close Review Gate A | Blocked by 4-6 | Architecture, data contract, security, visual, browser, and status review | Project owner/expert reviewer |
| 8 | Audit standalone Tool Chest semantics | Ready after structural packets | Live vs local labels, provenance, gated states, daemon rejection/error behavior | Project owner accepts findings |
| 9 | Select next Tool Chest chain | Awaiting selection | Inventory row, live `ALL_BOUND` ID or explicit gated shell, acceptance task | Captain/project owner |

## Latest execution evidence

### `RBL-01` - 2026-09-05

- Platform routing: platform-neutral `D3` coordination/specification/QA audit.
- Baseline: `0.0.36-dev` at `4eade061`; fetched remote is synchronized.
- Preserved state: pre-existing modified `Cargo.lock`; no product files edited.
- `cargo test -p poet --test product_integrity`: compile failed with 16
  `E0583` missing-module errors; no tests ran.
- `cargo test -p poet --test surface_inventory`: same compile gate; no tests ran.
- Cause: `crates/poet/src/browser/registration.rs` declares child modules, but
  the tracked `register_*_toolbox.rs` files are siblings under `browser/`.
- Provenance: the declarations entered in commit `43e759fa`; this identifies
  the change boundary, not agent ownership or intent.
- Broader build: not run because the same POET library compilation gate must be
  repaired first.
- Audit recommendation at that point: execute `FIX-REG-01` / `RM-01`, then
  `RM-02` before new styling. `RM-01` is now completed below.

### `RM-01` - 2026-09-05

- Filesystem structure: `browser/registration/mod.rs` plus 16 purpose-specific
  child modules; source contents were moved unchanged.
- `cargo test -p poet --test product_integrity`: 9 passed.
- `cargo test -p poet --test surface_inventory`: 1 passed.
- `trunk build`: passed; the first invocation was rejected by Trunk because the
  inherited `NO_COLOR=1` is not a valid Boolean for Trunk 0.21.14, then passed
  with process-local `NO_COLOR=true`.
- No capability, host, ABI, or product behavior change.
- Pre-existing `Cargo.lock` remained byte-for-byte unchanged.
- Next packet: `RM-02` (`D2`, `FE`, `UX`, `QA`).

### `RM-02` - 2026-09-05

- Structure: 43-line `browser/css.rs` composition module plus 14 named CSS
  assets; maximum asset size 421 lines.
- Preservation: assembled normalized CSS body retains SHA-256
  `BFF95C324960484900865245E6133D2D702A8AF96D5373220EC238EEC84786AB`.
- Focused CSS composition/order test: 1 passed.
- Product integrity: 9 passed; surface inventory: 1 passed.
- `trunk build`: passed.
- Browser UAT: 1280x720 and 390x844 rendered with the complete stylesheet;
  mobile body width matched its 390-pixel viewport.
- Repository policy: under 500 is the target; 500-1,199 requires ownership
  review; 1,200 triggers decomposition before new behavior; 1,400 escalates.
- Exceptions are documented for prose, generated artifacts, fixtures/tables,
  static registries, and genuinely cohesive algorithms.
- Next packet: `HLT-R1` (`D4`, `SEC`, `RUST`, `SPEC`, then `D5` review).

### `RM-03` - 2026-09-05

- Structure: 2,734-line `browser/topbar.rs` replaced by a 33-line stable
  router and eight purpose-specific child modules, all below 500 lines.
- API: all former public topbar functions and `MenuItemDef` remain re-exported.
- Links: browser-parent references were updated for the new module depth; no
  action, persistence, capability, or host semantics changed.
- Verification: `cargo check`, product integrity 9/9, surface inventory 1/1,
  `trunk build`, and scoped rustfmt passed.
- Browser UAT: File menu and Strata tray worked at desktop and mobile widths;
  browser logs contained no warnings or errors.
- Current decomposition counts: 67 implementation-focused files over 1,400
  lines and 12 over 2,000.
- Next decomposition candidate: `RM-04`, `browser/interactions.rs` (`D3`).
- Next programme assurance packet remains `HLT-R1` (`D4` plus independent
  review); either lane must remain one packet per session.

### `RM-04` - 2026-09-05

- Structure: 2,148-line `browser/interactions.rs` replaced by a 92-line stable
  router and seven purpose-specific child modules, all below 500 lines.
- API: every former public interaction function and `ContainerRect` remains
  re-exported; shared pointer state remains in the router.
- Verification: `cargo check`, five geometry tests, product integrity 9/9,
  surface inventory 1/1, `trunk build`, and scoped rustfmt passed.
- Browser UAT: selection, canvas zoom, Tool Chest flyout, right/left docking,
  and mobile layout passed; browser logs contained no warnings or errors.
- Current decomposition counts: 66 implementation-focused files over 1,400
  lines and 11 over 2,000.
- Next decomposition candidate: `RM-05`, `browser/search_workbench.rs` (`D3`).
- Next programme assurance packet remains `HLT-R1`; the project owner may
  continue either the POET decomposition lane or the Health assurance lane.

### `RM-05` - 2026-09-05

- Structure: 2,033-line `browser/search_workbench.rs` replaced by a 34-line
  stable router and eight purpose-specific child modules, all below 500 lines.
- API: `build_search_workbench`, `toggle_search_workbench`, `open_to_mode`,
  and `wire_search_workbench_shortcut` remain re-exported; no caller paths
  changed.
- Honesty: SPARQL execution still requires the QualiaDB daemon; the
  unavailable path still refuses to fabricate results.
- Verification: `cargo check` (no new warnings), five persist tests,
  product integrity 9/9, surface inventory 1/1, `trunk build`, and scoped
  rustfmt passed. Fresh wasm contains the workbench ids and the daemon
  unavailable string.
- Interactive browser click-UAT was not re-run (no click driver this
  session); `trunk` is serving `127.0.0.1:8080` from the new dist.
- Current decomposition counts: 66 crates-`src` files over 1,400 lines and
  10 over 2,000. POET has no remaining file over 2,000.
- Next decomposition candidate: `RM-06`, `browser/containers.rs` (`D3`),
  coordinated with the container-view cluster.
- Next programme assurance packet is `HLT-07` (`D4` CLIN); `HLT-R1` instrument
  review is recorded below. Review Gate A remains a `D5` owner close.

### `HLT-R1` - 2026-09-05

- Read: `governance/consent_contract.rs`, Poet `disclosure_model.rs`,
  `project_shares` / revocation payload, HLT-03 playbook acceptance.
- Held without change: signed digest immutability of principal/scope; expiry
  `now >= expires_at`; principal-only revoke; no private key on the grant
  struct (verifying key + 64-byte signature).
- Repaired: unused `ReplayDetected`; stateless omit-receipt reactivation;
  unknown/empty scope labels; Poet "All categories" / missing-expiry Active
  defaults; grantable `clinical_notes` UI flag (not a contract bit).
- Verification: 12 `consent_contract` tests passed; 27 `health_views` tests
  passed (includes 3 share-projection tests).
- Not claimed: Review Gate A / `D5` clinical-authorization close; live daemon
  signing of grants still goes through record upsert, not `ConsentLedger`
  issue/revoke (seam for Neo if Health persist should remember revocations).
- Next packet: `HLT-07`.

## Post-gate programme

The earlier programme remains the dependency backbone after Gate A:

| Programme | Higher-assurance contract work | Bounded product/design work | Review point |
|---|---|---|---|
| Governed Q42 health assets | `AST-01` to `AST-04` | `AST-05` to `AST-07` | Licence, provenance, mapping, and ABI issues escalate |
| Portable application contract | `APP-01` to `APP-03` | `APP-04` to `APP-06` | Manifest and projection parity review |
| Webizen Desktop host | `WD-02`, `WD-03`, `WD-05` | `WD-01`, `WD-04`, `WD-06` to `WD-08` | Review Gate B |
| Wider restoration | `GOV-01`, `GOV-02`, `DEV-01`, `SOC-01` | `PRJ-01` to `PRJ-03`, `KNOW-01` to `KNOW-02`, `STU-01` to `STU-02` | Domain-specific review as required |

## Post-freeze lane alignment

| Lane plan | First reconciled action | Dependency |
|---|---|---|
| Davinci / POET chrome | Compare Stage 0-1 acceptance against completed `UX-01` to `UX-04`; record only real deltas | Current UX implementation and Tool Chest inventory |
| Monet / visual grammar | Publish the motion contract, then verify existing reduced-motion and container states before new motion | Davinci delta audit |
| Marvin / ontology | Publish Container/Manifold/Link joins with precise SHACL-first wording | Existing vocabularies and live Invoke IDs |
| Neo / seams | Audit remaining `capability_scope` values; add no bind before next chain is selected | `ALL_BOUND`, project-owner selection |
| Vibe / language | Stage 0–1 landed 2026-09-05; remaining stages stay parked on Marvin/Neo/Capt. gates | Frozen host facade and Neo findings |

## Evidence vocabulary

- **Implemented:** code is present and linked to focused verification.
- **UAT verified:** the named user task was exercised in the relevant host.
- **Document landed:** a plan/specification exists; implementation is not implied.
- **Ready:** dependencies are present and work may begin.
- **Blocked:** a named dependency or authority decision is missing.
- **Parked:** intentionally excluded until a project gate opens.
- **Review required:** implementation evidence exists but the risk boundary has
  not received the required independent assessment.

## Change discipline

1. Start from the first ready item whose dependencies are met.
2. Preserve unrelated agent changes and record the baseline commit/status.
3. Make product behavior distinguish live, local, unavailable, denied, and fault.
4. Do not infer capability completion from a plan, label, or decorative shell.
5. Update this register by appending evidence; historical source ledgers remain
   append-only.
6. Stop at Review Gate A and Review Gate B until the project owner accepts the
   review outcome.
