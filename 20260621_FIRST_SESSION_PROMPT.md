# First-session prompt — namespace remediation + Webizen values-credential smoke test

Two phases, in order. Spec + decision log: **`core-ontologies/PLAN.md`** (esp. §18, §17, §11.3, §0).
Entry pointer: `20260621_HANDOVER_ontologies.md`. Discipline below is non-negotiable.

Domain trinity (§13): `ns.webcivics.org` = vocab/standards (✅ done) · `trustfactory.org` = trust
(future) · `webizen.org` = engine.

---

## PHASE 0 — namespace remediation (engine `qualia.*` → `webizen.org`); PLAN §18

The values layer is already on `ns.webcivics.org`. Remediate the remaining **engine** namespaces.
**Each is CODE-COUPLED** — `.rs` constructs/compares the URI — so move the `.ttl`/`.json`/`.js`
**and** `.rs` TOGETHER, then `cargo build` + full `cargo test` must pass (the owl.rs lesson:
data + code move together or runtime tag-matching breaks).

Per PLAN §18 table:
1. `qualia.network/q42#` → `webizen.org/q42#` — 4 `shapes/*.shacl.ttl` + 4 `src/modalities/logic/*_shacl.rs`. (low-med)
2. **`qualia.org/ld/{vault,context,vocab}` → `webizen.org/ld/...`** — `daemon.rs`, `p2p/protocol.rs`,
   `q42_lexicon.rs`, `vault_manifest.rs`. **HIGH RISK — CHECK FIRST:** does `C:\Users\Admin\qualia-vault`
   (or any on-disk `.q42`/vault manifest) contain `qualia.org/ld` URIs? If YES → read-alias (accept
   old+new) or migrate the data; if NO deployed vaults/peers → hard-swap. Don't blind-replace.
3. `qualia-db.org/vocab#` + demo URIs → `webizen.org/vocab#` — `resolver.rs`, `webizen_server.rs`,
   `tests/resolver_tests.rs`, `docs/src/qualia-worker.js`, `docs/tests/suites/*.js`,
   `docs/data/science-constants.json`. (med — tests catch breakage)
4. `qualia.social/ns/` → `webizen.org/ns/` — `docs/tests/suites/wasm-profiles.js`. (low)

LEAVE AS-IS: `qualia.anatomy.example` (reserved `.example` — correct test data) and `qualia.db/*`
(playground demo). **Phase 0 done =** `cargo build` + full `cargo test` green; no residual `qualia.*`
repo-wide except `.example`/demo.

---

## PHASE 1 — Webizen values-credential smoke test (the keystone; PLAN §11.3, §17.1)

The first executable, falsifiable proof the values corpus runs the engine's native deontic evaluator.

**Read first** (verify signatures against source — don't trust memory): `webizen.rs`
(`execute_vm_frame`, `register_rule`, `fire_registered_rules`, `NativeDeonticEval` ~L1140),
`modalities/logic/deontic.rs` (`evaluate_deontic_contract`, `compile_n3_rule_to_norm`, the 64-byte
`DeonticVerdict`), `n3_parser.rs` (`Rule`), call sites `ingest.rs:370/384` + `mcp_server.rs:486`,
and `values.n3`/`agency.n3`.

**Goal — a real `cargo test test_deontic_smoke` in `qualia-core-db`:** parse a tiny N3 fixture
(R3 + agency.n3 **G1**: a `CorporatePerson` claiming a `NaturalPerson`-only dignity right ⇒
`PersonhoodCategoryError`) via `n3_parser` → `register_rule` → `fire_registered_rules` →
`NativeDeonticEval`/`execute_vm_frame`; inject a malicious `AgentIntent` quin; pass a **stack
`[DeonticVerdict; N]`** buffer; **assert a Deny verdict** for the violated norm; confirm
`n3logic.rs` is NOT on the path.

**Done =** `cargo test -p qualia-core-db test_deontic_smoke` compiles AND passes; paste the real
output. If the lane doesn't connect, diagnose and fix the wiring — that's the deliverable — or
report the exact blocker honestly.

---

## Discipline (non-negotiable — this repo's hard-won lesson)
- **NO mocks, NO stubbed pass. Compile-green ≠ works.** Antigravity reported the ZK work "ready"
  because it compiled — its prove/verify test FAILED. Run the real round-trip.
- Verify every API against actual source. Move code + data together. Never fake success; if
  something's missing, build it for real or add a `PendingImplementation` marker.

## Already settled (don't re-litigate)
- Webizen (the governance VM, `webizen.rs`, formerly "Sentinel") is the evaluator — this is
  **wiring, not engine-building**.
- Values namespace = `https://ns.webcivics.org/` (migrated). Engine namespace target = `webizen.org`
  (Phase 0).
- Predicate-packing convention: NOT needed for the smoke test (deontic R1/R3, not sense/temporal).

## After both phases
PLAN §17.1: deontic wiring (ingest `values.n3`/`agency.n3` so guards register + fire) → build
`validate_core_ontologies` + `build_index.py` gap report (the safety gates) → only THEN parallelise
the backlog with agents whose output the gate checks.

Make it phenomenal.
