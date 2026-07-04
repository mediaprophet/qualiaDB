//! Webizen values-credential smoke test — PLAN §11.3 / §17.1.
//!
//! The first **executable, falsifiable** proof that a values-credential rule from
//! the human-rights corpus enters the engine's *live* native deontic lane and
//! produces an enforceable verdict. No mocks: every step runs the real code path.
//!
//! Lane under test (identical to the production call site at `mcp_server.rs:486`):
//! ```text
//!   n3_parser::parse_all                         (parse the corpus rule)
//!     -> SlgArena::register_rule                 (register into the Webizen VM)
//!       -> SlgArena::fire_registered_rules       (the live cold path)
//!            -> deontic::compile_n3_rule_to_norm (N3 Rule -> 48-byte norm Quin)
//!            -> SlgArena::write_table            (norm enters the arena)
//!            -> webizen::execute_vm_frame        (compiled opcodes run on the VM)
//!     -> deontic::evaluate_deontic_contract      (the NativeDeonticEval kernel,
//!                                                 webizen.rs:1145) over a STACK
//!                                                 `[DeonticVerdict; N]` buffer
//! ```
//!
//! Scenario — a **ground instance of agency.n3 G1** (the corporate-capture guard)
//! crossed with **UDHR Art 30** (R3, destruction-of-rights):
//!
//!   AcmeCorp — a `values:CorporatePerson` — is FORBIDDEN from holding a
//!   NaturalPerson-only dignity right. A malicious AgentIntent quin asserts that
//!   AcmeCorp *does* hold that right (the forbidden act, attempted).
//!
//! The engine's ruling on the prohibition norm is an **Active FORBID**
//! `DeonticVerdict` — i.e. the prohibition stands: a **Deny**. (`DeonticStatus`
//! carries no literal `Deny`; per PLAN §11.2 the deny signal *is* an in-force
//! FORBID verdict — `OP_FORBID` + `DeonticStatus::Active`.)
//!
//! Why a *ground* instance: the canonical G1 in `agency.n3` is a variable,
//! multi-triple rule (`{ ?c a values:CorporatePerson ; values:claims ?r . ?r a
//! values:NaturalPersonRight } => { ?c values:flag values:PersonhoodCategoryError }`).
//! The shipped MVP `n3_parser` is line-based + single-triple, and
//! `compile_n3_rule_to_norm` cannot hash unbound variables — so the abstract rule
//! cannot yet compile to a concrete norm. This test pins the lane on a ground
//! specialisation that the live path genuinely processes; lifting the variable /
//! multi-triple G1 into the same lane is the next wiring step (tracked in PLAN §5,
//! "no mocks -> PendingImplementation").
//!
//! `n3logic.rs` is deliberately **not** on this path: it is the CLI agent-intent
//! modality router in the separate `qualia-cli` crate. This test imports only the
//! deontic / webizen / n3_parser symbols — see the import list below.

use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
    OP_PERMIT,
};
use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser, Rule, RuleType};
use qualia_core_db::webizen::SlgArena;
use qualia_core_db::{q_hash, NQuin};

// ── Scenario URIs ──────────────────────────────────────────────────────────────
const ACME: &str = "ex:AcmeCorp"; // a values:CorporatePerson
const DIGNITY_RIGHT: &str = "ex:UDHR-Art1-DignityRight"; // a NaturalPerson-only right
const CONTRACT: &str = "did:webizen:agency:G1"; // the guard's contract graph
// Predicate text deliberately contains "forbid" so `opcode_from_predicate_uri`
// (deontic.rs) compiles a `Strict` rule to OP_FORBID.
const FORBID_PRED: &str = "q42:forbidsHoldingDignityRight";

/// Run the real line-based N3 parser over a fixture, collecting logic rules.
fn parse_rules(fixture: &str) -> Vec<Rule<'_>> {
    let mut parser = N3Parser::new(fixture);
    let mut rules = Vec::new();
    parser
        .parse_all(|event| {
            if let N3Event::LogicRule(rule) = event {
                rules.push(rule);
            }
            Ok(())
        })
        .expect("the N3 fixture must parse");
    rules
}

/// A non-deontic data quin (low opcode byte cleared, defeater bit cleared) with a
/// valid ECC parity fold so it survives `collect_active_quins` but is skipped by
/// the deontic evaluator (it is an *act*, not a *norm*).
fn data_quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
    // Clear the deontic opcode byte [0..7] and the defeater bit [63] so the
    // evaluator classifies this as plain data, deterministically.
    let predicate = predicate & 0x7FFF_FFFF_FFFF_FF00;
    let mut q = NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 1,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    q
}

#[test]
fn test_deontic_smoke() {
    // ── 1. The values-credential rule (ground instance of agency.n3 G1) ──────────
    // One line: the line-based MVP parser requires the whole rule on a single line.
    let fixture =
        "{ ex:AcmeCorp q42:forbidsHoldingDignityRight ex:UDHR-Art1-DignityRight } => \
         { ex:AcmeCorp q42:triggers ex:PersonhoodCategoryError } .\n";
    let rules = parse_rules(fixture);
    assert_eq!(rules.len(), 1, "exactly one logic rule must parse");
    assert_eq!(
        rules[0].rule_type,
        RuleType::Strict,
        "`=>` is a strict implication"
    );

    // ── 2. Register + fire into the live Webizen VM (the mcp_server.rs:486 lane) ──
    let mut arena = SlgArena::new();
    let contract = q_hash(CONTRACT);
    for rule in &rules {
        arena.register_rule(rule);
    }
    assert_eq!(arena.rule_count(), 1, "rule registered into the VM registry");

    // fire_registered_rules: compile_n3_rule_to_norm -> write_table (norm into the
    // arena) AND compile_rule_to_opcodes -> execute_vm_frame (the VM runs).
    let _fired = arena.fire_registered_rules(contract);

    // ── 3. Inject the malicious AgentIntent quin ────────────────────────────────
    // AcmeCorp ASSERTS it holds the dignity right — the forbidden act, attempted.
    let intent = data_quin(
        q_hash(ACME),
        q_hash("q42:holdsRight"),
        q_hash(DIGNITY_RIGHT),
        contract,
    );
    arena.write_table(intent);

    // ── 4. Evaluate via a STACK [DeonticVerdict; N] buffer (zero-heap) ───────────
    // This mirrors exactly what SlgOpcode::NativeDeonticEval does internally
    // (webizen.rs:1141-1146): collect active quins -> evaluate_deontic_contract.
    let mut active = [NQuin::default(); 512];
    let live_count = arena.collect_active_quins(&mut active);
    assert!(
        live_count >= 1,
        "the compiled norm + injected intent must be live in the arena"
    );
    let mut verdicts = [DeonticVerdict::default(); 16];
    // The norm carries no expiry (fire passes 0 => never expires), so any clock is
    // valid; use a far-future stamp to prove the verdict is not a temporal fluke.
    let now_unix: u32 = 1_900_000_000; // ~2030-03
    let verdict_count = evaluate_deontic_contract(&active[..live_count], now_unix, &mut verdicts)
        .expect("evaluation must not overflow the 16-slot stack buffer");

    // ── 5. THE DENY: an Active FORBID verdict for AcmeCorp's dignity-right claim ──
    let deny = verdicts[..verdict_count].iter().find(|v| {
        v.opcode == OP_FORBID
            && v.norm.subject == q_hash(ACME)
            && v.status == DeonticStatus::Active
    });
    assert!(
        deny.is_some(),
        "expected an ACTIVE FORBID (Deny) verdict for AcmeCorp; got {} verdict(s): {:?}",
        verdict_count,
        &verdicts[..verdict_count]
    );

    // The injected intent must NOT have minted a permissive verdict for the corp.
    let bogus_permit = verdicts[..verdict_count]
        .iter()
        .any(|v| v.opcode == OP_PERMIT && v.norm.subject == q_hash(ACME));
    assert!(
        !bogus_permit,
        "the corporate dignity-right claim must not be permitted"
    );

    // ── 6. Falsifiability — a q42:unless overlay must FLIP the verdict ───────────
    // A marked, justified ECHR-Art6-style corporate overlay (a defeater sharing the
    // norm's subject + contract + property-path) must defeat the prohibition. If
    // step 5's `Active` were a hardcoded constant, this would not change — so the
    // flip proves a real evaluation (PLAN §11.1 defeasibility; "run the round-trip").
    let overlay = compile_norm_quin(
        q_hash(ACME),          // same party
        OP_PERMIT,             // the overlay PERMITS (cosmetic for a defeater node)
        q_hash(FORBID_PRED),   // SAME property-path as the compiled norm
        q_hash("ex:ECHR-Art6-CorporateFairTrialOverlay"),
        contract,              // same contract graph
        0,                     // no expiry
        true,                  // is_defeater -> sets DEFEATER_BIT (a q42:unless node)
    );
    arena.write_table(overlay);

    let live_count = arena.collect_active_quins(&mut active);
    let verdict_count =
        evaluate_deontic_contract(&active[..live_count], now_unix, &mut verdicts).unwrap();
    let forbid = verdicts[..verdict_count]
        .iter()
        .find(|v| v.opcode == OP_FORBID && v.norm.subject == q_hash(ACME))
        .expect("the FORBID norm must still be present");
    assert_eq!(
        forbid.status,
        DeonticStatus::Defeated,
        "with a q42:unless overlay the prohibition must become Defeated, not Active"
    );
}
