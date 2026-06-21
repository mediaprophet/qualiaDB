//! Guard-rule grounding — forward-chaining the real agency.n3 variable, multi-triple
//! guards (G1 corporate-capture, G1' AI-capture) end-to-end through the live engine.
//!
//! This is the wiring that the ground-instance `deontic_smoke` test could not reach:
//! the *abstract* G1 is a variable, multi-line, 4-triple-premise rule that the MVP
//! parser previously could not read and `compile_n3_rule_to_norm` could not ground.
//! Here the upgraded `n3_parser` reads it and `SlgArena::fire_guard_rules` joins the
//! premise over live facts and asserts the bound conclusion — the PersonhoodCategoryError
//! flag (a Deny) — observable via `has_quin`. No mocks: the real rule text, the real
//! parse → register → fire path.

use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser};
use qualia_core_db::webizen::SlgArena;
use qualia_core_db::{q_hash, NQuin};

/// Parse N3 text and register every logic rule into the arena. Returns rule count.
fn register_rules(arena: &mut SlgArena, n3: &str) -> usize {
    let cursor = std::io::Cursor::new(n3.as_bytes());
    let mut parser = N3Parser::new(cursor);
    let mut count = 0usize;
    parser
        .parse_all(|event| {
            if let N3Event::LogicRule(rule) = event {
                arena.register_rule(rule);
                count += 1;
            }
            Ok(())
        })
        .expect("N3 must parse");
    count
}

/// Write a ground fact `(s, p, o)` into the arena, hashed the same way the engine
/// hashes ingested triples (uniform `q_hash`; `a` is the literal type token).
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

fn is_flagged(arena: &SlgArena, subject: &str, flag: &str) -> bool {
    arena.has_quin(q_hash(subject), q_hash("values:flag"), q_hash(flag))
}

// The verbatim agency.n3 G1 corporate-capture guard — multi-line, `;`-list premise.
const G1: &str = "\
# (G1) Corporate-capture guard
{ ?c a values:CorporatePerson ; values:claims ?r .
  ?r a values:Right ; values:heldBy values:NaturalPerson
} => { ?c values:flag values:PersonhoodCategoryError } .
";

// G1' — same guard for any ArtificialAgent (blocks 'AI-rights' capture).
const G1_PRIME: &str = "\
{ ?x a values:ArtificialAgent ; values:claims ?r .
  ?r a values:Right ; values:heldBy values:NaturalPerson
} => { ?x values:flag values:PersonhoodCategoryError } .
";

#[test]
fn g1_grounds_and_denies_corporate_capture() {
    let mut arena = SlgArena::new();
    let rules = register_rules(&mut arena, G1);
    assert_eq!(
        rules, 1,
        "the multi-line, 4-triple-premise G1 must parse as exactly one rule"
    );

    // Facts: AcmeCorp (a CorporatePerson) claims R1, a natural-person-held Right.
    assert_fact(&mut arena, "ex:AcmeCorp", "a", "values:CorporatePerson");
    assert_fact(&mut arena, "ex:AcmeCorp", "values:claims", "ex:R1");
    assert_fact(&mut arena, "ex:R1", "a", "values:Right");
    assert_fact(&mut arena, "ex:R1", "values:heldBy", "values:NaturalPerson");

    let asserted = arena.fire_guard_rules();
    assert!(
        asserted >= 1,
        "G1 must forward-chain its conclusion (got {asserted} assertions)"
    );

    // THE DENY: AcmeCorp is flagged with a personhood category error.
    assert!(
        is_flagged(&mut arena, "ex:AcmeCorp", "values:PersonhoodCategoryError"),
        "G1 must flag the corporate person with PersonhoodCategoryError"
    );

    // Idempotent: a second pass asserts nothing new.
    assert_eq!(
        arena.fire_guard_rules(),
        0,
        "re-firing must not duplicate the already-asserted conclusion"
    );
}

#[test]
fn g1_does_not_flag_a_natural_person() {
    // Negative control — the guard is specific to CorporatePerson, not blanket.
    let mut arena = SlgArena::new();
    register_rules(&mut arena, G1);

    // A NATURAL person claiming the very same right must NOT trip the guard.
    assert_fact(&mut arena, "ex:Alice", "a", "values:NaturalPerson");
    assert_fact(&mut arena, "ex:Alice", "values:claims", "ex:R1");
    assert_fact(&mut arena, "ex:R1", "a", "values:Right");
    assert_fact(&mut arena, "ex:R1", "values:heldBy", "values:NaturalPerson");

    arena.fire_guard_rules();

    assert!(
        !is_flagged(&mut arena, "ex:Alice", "values:PersonhoodCategoryError"),
        "a natural person legitimately holding a right must not be flagged"
    );
}

#[test]
fn g1_prime_grounds_ai_capture() {
    let mut arena = SlgArena::new();
    assert_eq!(register_rules(&mut arena, G1_PRIME), 1);

    // An autonomous software agent claiming a natural-person right.
    assert_fact(&mut arena, "ex:ChatBot", "a", "values:ArtificialAgent");
    assert_fact(&mut arena, "ex:ChatBot", "values:claims", "ex:R1");
    assert_fact(&mut arena, "ex:R1", "a", "values:Right");
    assert_fact(&mut arena, "ex:R1", "values:heldBy", "values:NaturalPerson");

    assert!(arena.fire_guard_rules() >= 1);
    assert!(
        is_flagged(&mut arena, "ex:ChatBot", "values:PersonhoodCategoryError"),
        "G1' must flag the artificial agent with PersonhoodCategoryError"
    );
}

#[test]
fn partial_premise_match_does_not_fire() {
    // Falsifiability: drop the `heldBy NaturalPerson` fact — the 4th premise triple
    // is unsatisfied, so the guard must NOT fire. Proves the join is a real
    // conjunction, not a one-triple shortcut.
    let mut arena = SlgArena::new();
    register_rules(&mut arena, G1);

    assert_fact(&mut arena, "ex:AcmeCorp", "a", "values:CorporatePerson");
    assert_fact(&mut arena, "ex:AcmeCorp", "values:claims", "ex:R1");
    assert_fact(&mut arena, "ex:R1", "a", "values:Right");
    // (no `ex:R1 values:heldBy values:NaturalPerson`)

    assert_eq!(
        arena.fire_guard_rules(),
        0,
        "an unsatisfied premise conjunct must block the conclusion"
    );
    assert!(!is_flagged(&mut arena, "ex:AcmeCorp", "values:PersonhoodCategoryError"));
}
