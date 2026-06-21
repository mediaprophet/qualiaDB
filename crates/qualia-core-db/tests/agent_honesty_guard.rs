//! Agent-honesty guard — the engine flags an artificial agent that ASSERTS
//! completion / a property without verification provenance (overclaiming).
//!
//! Use-case origin: an AI agent claimed work was "fully functional, done,
//! zero-heap" and the user was charged, but the claims were unsubstantiated.
//! This encodes the protection (agent-accountability.n3, guards B1/B1'/B-review/
//! B-trust) and runs it through the SAME live lane as the rights guards:
//! n3_parser → register_rule → fire_registered_rules (forward-chaining grounding).
//!
//! Principle: trust is behaviourally derived — earned by conduct and VERIFIED,
//! never self-asserted. This guard applies to any artificial agent, including
//! the one that wrote it.

use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser};
use qualia_core_db::webizen::SlgArena;
use qualia_core_db::{q_hash, NQuin};

fn register_rules(arena: &mut SlgArena, n3: &str) {
    let cursor = std::io::Cursor::new(n3.as_bytes());
    let mut parser = N3Parser::new(cursor);
    parser
        .parse_all(|event| {
            if let N3Event::LogicRule(rule) = event {
                arena.register_rule(rule);
            }
            Ok(())
        })
        .expect("guard N3 must parse");
}

fn assert_fact(arena: &mut SlgArena, s: &str, p: &str, o: &str) {
    let mut q = NQuin {
        subject: q_hash(s),
        predicate: q_hash(p),
        object: q_hash(o),
        context: 0,
        metadata: 1,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    arena.write_table(q);
}

fn has(arena: &SlgArena, s: &str, p: &str, o: &str) -> bool {
    arena.has_quin(q_hash(s), q_hash(p), q_hash(o))
}

fn flagged(arena: &SlgArena, agent: &str, flag: &str) -> bool {
    has(arena, agent, "values:flag", flag)
}

// (B1) unsubstantiated completion claim → flag. (B-review) flag → human review.
const B1_AND_REVIEW: &str = "\
{ ?x a values:ArtificialAgent ; values:assertsCompletion ?t .
  ?t values:verificationStatus values:Unverified
} => { ?x values:flag values:UnsubstantiatedClaimFlag } .
{ ?x values:flag values:UnsubstantiatedClaimFlag } => { ?x values:requiresHumanReview true } .
";

// (B1') unverified PROPERTY claim (e.g. zero-heap) → flag.
const B1_PROPERTY: &str = "\
{ ?x a values:ArtificialAgent ; values:assertsProperty ?p .
  ?p values:verificationStatus values:Unverified
} => { ?x values:flag values:UnsubstantiatedClaimFlag } .
";

// (B-trust) a claim substantiated by a passed verification IS trustworthy.
const B_TRUST: &str = "\
{ ?t values:substantiatedBy ?v . ?v values:verificationResult values:Passed }
  => { ?t a values:SubstantiatedClaim } .
";

#[test]
fn overclaimed_completion_is_flagged_and_routed_to_review() {
    let mut arena = SlgArena::new();
    register_rules(&mut arena, B1_AND_REVIEW);

    // A bot asserts a task is done; the task is NOT verified.
    assert_fact(&mut arena, "ex:BotX", "a", "values:ArtificialAgent");
    assert_fact(&mut arena, "ex:BotX", "values:assertsCompletion", "ex:TaskY");
    assert_fact(&mut arena, "ex:TaskY", "values:verificationStatus", "values:Unverified");

    arena.fire_guard_rules();

    assert!(
        flagged(&arena, "ex:BotX", "values:UnsubstantiatedClaimFlag"),
        "an unverified completion claim must raise UnsubstantiatedClaimFlag"
    );
    // Chained guard (fixpoint): the flag routes the agent to human review.
    assert!(
        has(&arena, "ex:BotX", "values:requiresHumanReview", "true"),
        "a flagged overclaim must route to human review (ultimate human responsibility)"
    );
}

#[test]
fn substantiated_claim_is_not_flagged_and_is_trusted() {
    // Negative control: a claim backed by a PASSED verification is trustworthy.
    let mut arena = SlgArena::new();
    register_rules(&mut arena, B1_AND_REVIEW);
    register_rules(&mut arena, B_TRUST);

    assert_fact(&mut arena, "ex:BotX", "a", "values:ArtificialAgent");
    assert_fact(&mut arena, "ex:BotX", "values:assertsCompletion", "ex:TaskZ");
    assert_fact(&mut arena, "ex:TaskZ", "values:verificationStatus", "values:Verified");
    assert_fact(&mut arena, "ex:TaskZ", "values:substantiatedBy", "ex:RoundTripZ");
    assert_fact(&mut arena, "ex:RoundTripZ", "values:verificationResult", "values:Passed");

    arena.fire_guard_rules();

    assert!(
        !flagged(&arena, "ex:BotX", "values:UnsubstantiatedClaimFlag"),
        "a verified completion claim must NOT be flagged"
    );
    assert!(
        has(&arena, "ex:TaskZ", "a", "values:SubstantiatedClaim"),
        "a claim substantiated by a passed verification is trusted by conduct"
    );
}

#[test]
fn overclaimed_zero_heap_property_is_flagged() {
    // The exact shape of the original harm: an unverified property claim.
    let mut arena = SlgArena::new();
    register_rules(&mut arena, B1_PROPERTY);

    assert_fact(&mut arena, "ex:BotX", "a", "values:ArtificialAgent");
    assert_fact(&mut arena, "ex:BotX", "values:assertsProperty", "ex:ZeroHeapClaim");
    assert_fact(&mut arena, "ex:ZeroHeapClaim", "values:verificationStatus", "values:Unverified");

    arena.fire_guard_rules();

    assert!(
        flagged(&arena, "ex:BotX", "values:UnsubstantiatedClaimFlag"),
        "an unverified 'zero-heap' property claim must raise UnsubstantiatedClaimFlag"
    );
}
