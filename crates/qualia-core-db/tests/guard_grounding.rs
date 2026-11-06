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

use qualia_core_db::foundation::plane_feature_guard::{
    validate_feature_bag, FeatureGuardError, KeyRoleTag, PlaneTag, TaggedFeature,
};
use qualia_core_db::modalities::logic::n3_parser::{N3Event, N3Parser};
use qualia_core_db::webizen::SlgArena;
use qualia_core_db::{q_hash, NQuin};

/// Parse N3 text and register every logic rule into the arena. Returns rule count.
fn register_rules(arena: &mut SlgArena, n3: &str) -> usize {
    let mut parser = N3Parser::new(n3);
    let mut count = 0usize;
    parser
        .parse_all(|event| {
            if let N3Event::LogicRule(rule) = event {
                arena.register_rule(&rule);
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
    assert!(!is_flagged(
        &mut arena,
        "ex:AcmeCorp",
        "values:PersonhoodCategoryError"
    ));
}

// ---------------------------------------------------------------------------
// Packet IDF-05: Plane Feature Guard & F6 §§8–15 Adversarial Pressure Tests
// ---------------------------------------------------------------------------

#[test]
fn idf05_rejects_missing_plane_tag_and_owl_sameas_on_who_plane() {
    let untagged = [TaggedFeature::new("living_agent_feature", None)];
    assert_eq!(
        validate_feature_bag(&untagged),
        Err(FeatureGuardError::MissingPlaneTag)
    );

    let same_as_who = [TaggedFeature::new("owl:sameAs", Some(PlaneTag::Who)).with_same_as()];
    assert_eq!(
        validate_feature_bag(&same_as_who),
        Err(FeatureGuardError::SameAsOnWhoPlane)
    );

    let qrc_to_who = [
        TaggedFeature::new("did:q42:z6MkpTHR8VNs", Some(PlaneTag::SpatiotemporalHandle))
            .with_target_plane(PlaneTag::Who),
    ];
    assert_eq!(
        validate_feature_bag(&qrc_to_who),
        Err(FeatureGuardError::QrcPromotedToWho)
    );
}

#[test]
fn idf05_enforces_closed_key_roles_and_blocks_dni_rar_qsession_collapse() {
    // Unknown key role must fail closed to HeldUnknownKeyRole.
    let unknown_role = [
        TaggedFeature::new("RendezvousAuthorizationRecord", Some(PlaneTag::Instrument))
            .with_key_role("InventedMasterKeyRole", None),
    ];
    assert_eq!(
        validate_feature_bag(&unknown_role),
        Err(FeatureGuardError::HeldUnknownKeyRole)
    );

    // QLinkEphemeralDh must never substitute for TransportAead; RouteUpdate != SessionAuthentication.
    let dh_as_aead = [TaggedFeature::new("QSessionProof", Some(PlaneTag::Instrument))
        .with_key_role("QLinkEphemeralDh", Some(KeyRoleTag::TransportAead))];
    assert_eq!(
        validate_feature_bag(&dh_as_aead),
        Err(FeatureGuardError::KeyRoleMismatch {
            actual: KeyRoleTag::QLinkEphemeralDh,
            expected: KeyRoleTag::TransportAead,
        })
    );

    let route_as_auth = [
        TaggedFeature::new("RendezvousAuthorizationRecord", Some(PlaneTag::Instrument))
            .with_key_role("RouteUpdate", Some(KeyRoleTag::SessionAuthentication)),
    ];
    assert_eq!(
        validate_feature_bag(&route_as_auth),
        Err(FeatureGuardError::KeyRoleMismatch {
            actual: KeyRoleTag::RouteUpdate,
            expected: KeyRoleTag::SessionAuthentication,
        })
    );

    // Collapsing DNI + RAR + QSessionProof into one record must fail closed.
    let collapsed = [
        TaggedFeature::new("DNI", Some(PlaneTag::SpatiotemporalHandle))
            .with_collapsed_dni_rar_qsession(),
    ];
    assert_eq!(
        validate_feature_bag(&collapsed),
        Err(FeatureGuardError::DniRarQSessionCollapsed)
    );
}

#[test]
fn idf05_f6_sections_8_to_15_adversarial_cross_plane_who_fusion_blocked() {
    // F6 §8: Relation-scoped locators (`jane@bob.tld`, `jane@carol.tld`) are SpatiotemporalHandles, never Who.
    for locator in ["jane@bob.tld", "jane@carol.tld", "RelationScopedLocator"] {
        let bag = [TaggedFeature::new(locator, Some(PlaneTag::Who))];
        assert_eq!(
            validate_feature_bag(&bag),
            Err(FeatureGuardError::CrossPlaneWhoFusion {
                source_plane: PlaneTag::SpatiotemporalHandle
            }),
            "locator {locator} must not fuse into Who"
        );
    }

    // F6 §9: Wallets, passwords, online accounts, biometrics are Instruments, never Who.
    for inst in [
        "Wallet",
        "PasswordVerifier",
        "OnlineAccount",
        "BiometricFamily",
        "BiometricInstance",
    ] {
        let bag = [TaggedFeature::new(inst, Some(PlaneTag::Instrument))
            .with_target_plane(PlaneTag::Who)];
        assert_eq!(
            validate_feature_bag(&bag),
            Err(FeatureGuardError::CrossPlaneWhoFusion {
                source_plane: PlaneTag::Instrument
            }),
            "instrument {inst} must not fuse into Who"
        );
    }

    // F6 §10: BLE proximity, ATM presence, geocache place-bound secrets are Instrument / Handle, never Who.
    assert_eq!(
        validate_feature_bag(&[TaggedFeature::new("BleProximitySensor", Some(PlaneTag::Who))]),
        Err(FeatureGuardError::CrossPlaneWhoFusion {
            source_plane: PlaneTag::Instrument
        })
    );
    assert_eq!(
        validate_feature_bag(&[TaggedFeature::new("AtmPresenceSensor", Some(PlaneTag::Who))]),
        Err(FeatureGuardError::CrossPlaneWhoFusion {
            source_plane: PlaneTag::Instrument
        })
    );
    assert_eq!(
        validate_feature_bag(&[TaggedFeature::new(
            "GeocachePlaceSecret",
            Some(PlaneTag::Who)
        )]),
        Err(FeatureGuardError::CrossPlaneWhoFusion {
            source_plane: PlaneTag::SpatiotemporalHandle
        })
    );

    // F6 §§11–15: RoleCapacityGrant, OrgRepresentativeBinding, RuleBreachClaim,
    // RelationshipAssessmentClaim, RelianceRecord, AssumptionChain, RootCauseBundle,
    // BlastRadiusMap, PseudonymAlias, and dense CounterpartNQuinGraph are Claims, never Who.
    for claim_artifact in [
        "RoleCapacityGrant",
        "OrgRepresentativeBinding",
        "RuleBreachClaim",
        "RelationshipAssessmentClaim",
        "RelianceRecord",
        "AssumptionChain",
        "RootCauseBundle",
        "BlastRadiusMap",
        "PseudonymAlias",
        "CounterpartNQuinGraph",
    ] {
        let mislabeled = [TaggedFeature::new(claim_artifact, Some(PlaneTag::Who))];
        assert_eq!(
            validate_feature_bag(&mislabeled),
            Err(FeatureGuardError::CrossPlaneWhoFusion {
                source_plane: PlaneTag::Claim
            }),
            "claim artifact {claim_artifact} must never be classified as Who"
        );
    }

    // Positive control: properly plane-separated feature bag passes.
    let valid_bag = [
        TaggedFeature::new("NaturalAgentSelfhood", Some(PlaneTag::Who)),
        TaggedFeature::new("jane@bob.tld", Some(PlaneTag::SpatiotemporalHandle)),
        TaggedFeature::new("Wallet", Some(PlaneTag::Instrument)).with_key_role(
            "CapabilityPresentation",
            Some(KeyRoleTag::CapabilityPresentation),
        ),
        TaggedFeature::new("RelianceRecord", Some(PlaneTag::Claim)),
    ];
    assert!(validate_feature_bag(&valid_bag).is_ok());
}
