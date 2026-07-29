//! Semantic logic rules, SPARQL, SHACL

#![allow(non_snake_case)]

// ── Handler registration ──────────────────────────────────────────────────────

#[tauri::command]
pub fn fetch_domain_ontology(domain_id: String) -> Result<String, String> {
    let compiler = qualia_semantic_library::ontology::OntologyCompiler::new(
        std::path::PathBuf::from("c:/Projects/qualia-27062026/cache/ontologies"),
    );
    compiler.fetch_domain_ontology(&domain_id)
}

#[tauri::command]
pub fn execute_sparql_query(query: String) -> Result<Vec<(String, String, String)>, String> {
    qualia_client_core::engine::semantic::execute_local_sparql(&query)
}

#[tauri::command]
pub fn validate_shacl_shape(node: u64, shape_uri: u64) -> Result<bool, String> {
    qualia_client_core::engine::semantic::validate_local_shacl(node, shape_uri)
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EvaluateLogicRulesInput {
    pub n3_source: String,
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    #[serde(default)]
    pub context: u64,
    #[serde(default = "default_ruleset_name")]
    pub ruleset_name: String,
    #[serde(default)]
    pub contract_hash: u64,
}

fn default_ruleset_name() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LogicRuleResultDto {
    pub ruleset_name: String,
    pub rule_name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvaluateLogicRulesOutput {
    pub rules_loaded: usize,
    pub ruleset_name: String,
    pub contract_hash: u64,
    pub results: Vec<LogicRuleResultDto>,
    pub passed_count: usize,
    pub failed_count: usize,
}

#[tauri::command]
pub fn evaluate_logic_rules(
    input: EvaluateLogicRulesInput,
) -> Result<EvaluateLogicRulesOutput, String> {
    use qualia_core_db::modalities::logic::rules::RuleEngine;
    use qualia_core_db::NQuin;

    let mut engine = RuleEngine::with_contract(input.contract_hash);
    let rules_loaded = engine.load_n3(&input.ruleset_name, &input.n3_source);

    let quin = NQuin {
        subject: input.subject,
        predicate: input.predicate,
        object: input.object,
        context: input.context,
        metadata: 0,
        parity: input.subject ^ input.predicate ^ input.object ^ input.context,
    };

    let results = engine.evaluate(&quin);
    let passed_count = results.iter().filter(|r| r.passed).count();
    let dto_results: Vec<LogicRuleResultDto> = results
        .iter()
        .map(|r| LogicRuleResultDto {
            ruleset_name: r.ruleset_name.clone(),
            rule_name: r.rule_name.clone(),
            passed: r.passed,
            message: r.message.clone(),
        })
        .collect();

    Ok(EvaluateLogicRulesOutput {
        rules_loaded,
        ruleset_name: input.ruleset_name,
        contract_hash: input.contract_hash,
        passed_count,
        failed_count: dto_results.len() - passed_count,
        results: dto_results,
    })
}
