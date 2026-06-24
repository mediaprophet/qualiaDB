use qualia_core_db::modalities::logic::n3_parser::{N3Parser, N3Event, RuleType, Term};
use qualia_core_db::modalities::logic::n3_compiler::{
    compile_rule_to_opcodes, compile_rule_to_quin, compile_rule_to_zero_heap, compile_rules_with_shacl_gate,
    default_observation_shape, MAX_COMPILED_OPCODES, MAX_COMPILED_QUINS,
};
use qualia_core_db::webizen::SlgOpcode;
use qualia_core_db::{q_hash, NQuin};

fn parse_n3_string<'a>(content: &'a str) -> Vec<N3Event<'a>> {
    let mut parser = N3Parser::new(content);
    let mut events = Vec::new();
    parser
        .parse_all(|event| {
            events.push(event);
            Ok(())
        })
        .unwrap();
    events
}

#[test]
fn test_n3_parser_comprehensive_rules() {
    let n3_input = r#"
        # Strict Rule
        [rule_strict] { ?x a <ex:Person> } => { ?x <ex:hasRights> true } .

        # Defeasible Rule with Weight
        [rule_def] (0.75) { ?x <ex:age> ?age } ~> { ?x <ex:canVote> true } .

        # Defeater Rule
        [rule_defeater] { ?x <ex:isFelon> true } ^> { ?x <ex:canVote> false } .

        # Linear Rule
        [rule_linear] { ?x <ex:hasToken> true } -o { ?x <ex:usedToken> true } .
    "#;

    let events = parse_n3_string(n3_input);
    assert_eq!(events.len(), 4, "Should parse exactly 4 rules");

    let mut rules = Vec::new();
    for event in events {
        if let N3Event::LogicRule(rule) = event {
            rules.push(rule);
        } else {
            panic!("Expected only LogicRule events");
        }
    }

    assert_eq!(rules[0].id.as_deref(), Some("rule_strict"));
    assert_eq!(rules[0].rule_type, RuleType::Strict);

    assert_eq!(rules[1].id.as_deref(), Some("rule_def"));
    assert_eq!(rules[1].weight, Some(0.75));
    assert_eq!(rules[1].rule_type, RuleType::Defeasible);

    assert_eq!(rules[2].id.as_deref(), Some("rule_defeater"));
    assert_eq!(rules[2].rule_type, RuleType::Defeater);

    assert_eq!(rules[3].id.as_deref(), Some("rule_linear"));
    assert_eq!(rules[3].rule_type, RuleType::Linear);
}

#[test]
fn test_n3_parser_complex_multiline_and_abbreviations() {
    let n3_input = r#"
        # Complex multi-line rule with semicolon and comma lists
        { 
            ?company a <values:CorporatePerson> ;
                     <values:claims> ?right .
            ?right a <values:Right> , <values:Privilege>
        } => { 
            ?company <values:flag> <values:PersonhoodCategoryError> 
        } .
    "#;

    let events = parse_n3_string(n3_input);
    assert_eq!(events.len(), 1);

    if let N3Event::LogicRule(rule) = &events[0] {
        // Triples expected:
        // 1: ?company a <values:CorporatePerson>
        // 2: ?company <values:claims> ?right
        // 3: ?right a <values:Right>
        // 4: ?right a <values:Privilege>
        assert_eq!(rule.premise.triples.len(), 4, "Semicolon and comma abbreviations should expand to 4 triples");
        assert_eq!(rule.conclusion.triples.len(), 1);

        assert!(rule.premise.triples.iter().any(|t| 
            matches!(&t.predicate, Term::Uri(u) if *u == "values:claims")
        ));
    } else {
        panic!("Expected LogicRule event");
    }
}

#[test]
fn test_n3_compiler_opcodes_generation() {
    let n3_input = "{ ?x <ex:p> ?y } ~> { ?x <ex:q> ?y } .";
    let events = parse_n3_string(n3_input);
    let rule = match &events[0] {
        N3Event::LogicRule(r) => r,
        _ => panic!("Expected rule"),
    };

    let mut opcodes = [SlgOpcode::Halt; MAX_COMPILED_OPCODES];
    let count = compile_rule_to_opcodes(&compile_rule_to_zero_heap(rule), &mut opcodes).unwrap();

    // Defeasible rules compile to CheckDefeaters -> Unify -> Call -> WarnOnly
    assert_eq!(count, 4);
    assert_eq!(opcodes[0], SlgOpcode::CheckDefeaters);
    assert_eq!(opcodes[1], SlgOpcode::Unify);
    assert_eq!(opcodes[2], SlgOpcode::Call);
    assert_eq!(opcodes[3], SlgOpcode::WarnOnly);
}

#[test]
fn test_n3_compiler_quin_generation() {
    let n3_input = "{ <ex:S> <ex:P> <ex:O> } => { <ex:S> <ex:Q> <ex:O> } .";
    let events = parse_n3_string(n3_input);
    let rule = match &events[0] {
        N3Event::LogicRule(r) => r,
        _ => panic!("Expected rule"),
    };

    let contract_hash = q_hash("did:test:contract");
    let mut quins = [NQuin::default(); MAX_COMPILED_QUINS];
    
    let count = compile_rule_to_quin(&compile_rule_to_zero_heap(rule), contract_hash, &mut quins).unwrap();
    assert!(count > 0, "Should generate at least 1 Quin from rule");

    // Context field should be the contract hash
    assert_eq!(quins[0].context, contract_hash);
}

#[test]
fn test_n3_shacl_gate_validation() {
    // Valid: restingHeartRate is 72 (>= 20.0)
    let valid_input = "{ ex:P ex:p ex:o } => { ex:Patient1 health:restingHeartRate \"72\" } .";
    // Invalid: restingHeartRate is 12 (< 20.0)
    let invalid_input = "{ ex:P ex:p ex:o } => { ex:Patient1 health:restingHeartRate \"12\" } .";

    let valid_rule = match &parse_n3_string(valid_input)[0] {
        N3Event::LogicRule(r) => r.clone(),
        _ => panic!(),
    };
    let invalid_rule = match &parse_n3_string(invalid_input)[0] {
        N3Event::LogicRule(r) => r.clone(),
        _ => panic!(),
    };

    let shape = default_observation_shape();
    let shapes = [&shape];

    let mut opcodes = [SlgOpcode::Halt; MAX_COMPILED_OPCODES];
    let mut quins = [NQuin::default(); MAX_COMPILED_QUINS];
    let contract_hash = q_hash("did:test:contract");

    let result_valid = compile_rules_with_shacl_gate(&[valid_rule], &shapes, &mut opcodes, &mut quins, contract_hash);
    assert!(result_valid.is_ok(), "Valid rule should pass SHACL gate");

    let result_invalid = compile_rules_with_shacl_gate(&[invalid_rule], &shapes, &mut opcodes, &mut quins, contract_hash);
    assert!(result_invalid.is_err(), "Invalid rule should fail SHACL gate");
}
