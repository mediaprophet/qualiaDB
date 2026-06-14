//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//!
//! All functions are `#[cfg(target_arch = "wasm32")]` and only compiled into
//! the browser/OPFS build.  Native desktop builds use direct Rust FFI.

#[cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── Economics: Monte Carlo VaR ──────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct SimulationParams {
    pub initial_price: f64,
    pub drift: f64,
    pub volatility: f64,
    pub time_horizon: i32,
    pub simulation_steps: i32,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_semantic_simulation(val: JsValue) -> Result<JsValue, JsValue> {
    let params: SimulationParams = serde_wasm_bindgen::from_value(val)?;
    let (mean, value_at_risk) = crate::domains::financial::economics::run_monte_carlo_var(
        params.initial_price,
        params.drift,
        params.volatility,
        params.time_horizon as f64,
        params.simulation_steps as usize,
        252,
    );
    #[derive(Serialize)]
    struct SimResult {
        mean: f64,
        value_at_risk: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&SimResult {
        mean,
        value_at_risk,
    })?)
}

// ─── Bioinformatics: sequence alignment ──────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct AlignmentParams {
    pub query: String,
    pub target: String,
    /// "nucleotide" or "protein"
    pub mode: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn align_sequences_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let params: AlignmentParams = serde_wasm_bindgen::from_value(val)?;
    let result = if params.mode == "protein" {
        crate::domains::biological::bioinformatics::align_protein(params.query.as_bytes(), params.target.as_bytes())
    } else {
        crate::domains::biological::bioinformatics::align_nucleotide(params.query.as_bytes(), params.target.as_bytes())
    };
    #[derive(Serialize)]
    struct AlignResult {
        score: i32,
        identity_pct: f32,
        num_matches: usize,
        num_gaps: usize,
        aligned_query: String,
        aligned_target: String,
    }
    Ok(serde_wasm_bindgen::to_value(&AlignResult {
        score: result.score,
        identity_pct: result.identity_pct,
        num_matches: result.num_matches,
        num_gaps: result.num_gaps,
        aligned_query: String::from_utf8_lossy(&result.aligned_query).into_owned(),
        aligned_target: String::from_utf8_lossy(&result.aligned_target).into_owned(),
    })?)
}

// ─── Bioinformatics: FASTA validation ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FastaParams {
    pub header: String,
    pub sequence: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_fasta_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let params: FastaParams = serde_wasm_bindgen::from_value(val)?;
    let record =
        crate::domains::biological::bioinformatics::validate_fasta_record(&params.header, params.sequence.as_bytes());
    #[derive(Serialize)]
    struct FastaResult {
        is_valid: bool,
        alphabet: String,
        invalid_chars: Vec<char>,
    }
    Ok(serde_wasm_bindgen::to_value(&FastaResult {
        is_valid: record.is_valid,
        alphabet: format!("{:?}", record.alphabet),
        invalid_chars: record.invalid_chars,
    })?)
}

// ─── Biomedical: clinical risk scores ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FraminghamParams {
    pub age: u8,
    pub sex_male: bool,
    pub total_cholesterol_mmol: f64,
    pub hdl_cholesterol_mmol: f64,
    pub systolic_bp: f64,
    pub bp_treated: bool,
    pub current_smoker: bool,
    pub diabetic: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_framingham_risk_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: FraminghamParams = serde_wasm_bindgen::from_value(val)?;

    // D'Agostino et al. 2008 General Cardiovascular Risk Score (Framingham Heart
    // Study, Circulation 117:743). Self-contained Cox model — no clinical_engine
    // dependency. Cholesterol inputs are mmol/L; the model uses mg/dL (×38.67).
    let tc_mgdl  = (p.total_cholesterol_mmol * 38.67).max(1.0);
    let hdl_mgdl = (p.hdl_cholesterol_mmol * 38.67).max(1.0);
    let ln_age = (p.age as f64).max(1.0).ln();
    let ln_tc  = tc_mgdl.ln();
    let ln_hdl = hdl_mgdl.ln();
    let ln_sbp = p.systolic_bp.max(1.0).ln();

    let (sum, mean, s0) = if p.sex_male {
        let mut s = 3.06117 * ln_age
            + 1.12370 * ln_tc
            - 0.93263 * ln_hdl
            + (if p.bp_treated { 1.99881 } else { 1.93303 }) * ln_sbp;
        if p.current_smoker { s += 0.65451; }
        if p.diabetic       { s += 0.57367; }
        (s, 23.9802_f64, 0.88936_f64)
    } else {
        let mut s = 2.32888 * ln_age
            + 1.20904 * ln_tc
            - 0.70833 * ln_hdl
            + (if p.bp_treated { 2.82263 } else { 2.76157 }) * ln_sbp;
        if p.current_smoker { s += 0.52873; }
        if p.diabetic       { s += 0.69154; }
        (s, 26.1931_f64, 0.95012_f64)
    };

    // Risk = 1 − S0(10)^exp(Σβx − mean), expressed as a percentage.
    let risk = ((1.0 - s0.powf((sum - mean).exp())) * 100.0).clamp(0.0, 100.0);
    let category = if risk < 10.0 { "Low" }
        else if risk < 20.0 { "Intermediate" }
        else { "High" };

    #[derive(Serialize)]
    struct RiskResult {
        risk_10yr_pct: f64,
        category: String,
    }
    Ok(serde_wasm_bindgen::to_value(&RiskResult {
        risk_10yr_pct: risk,
        category: category.to_string(),
    })?)
}

// ─── Biomedical: FHIR observation validation ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct FhirObsParams {
    pub loinc_code: String,
    pub value: f64,
    pub unit_ucum: String,
    pub reference_low: Option<f64>,
    pub reference_high: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_fhir_observation_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: FhirObsParams = serde_wasm_bindgen::from_value(val)?;
    // Mocked for WASM due to clinical_engine dependency removal
    #[derive(Serialize)]
    struct ValidationResult {
        is_valid: bool,
        status: String,
        interpretation_code: String,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationResult {
        is_valid: true,
        status: "Mock".to_string(),
        interpretation_code: "N".to_string(),
    })?)
}

// ─── Biomedical: drug interaction check ──────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct DrugInteractionParams {
    /// List of medication names (will be q_hashed internally).
    pub medications: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn check_drug_interactions_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: DrugInteractionParams = serde_wasm_bindgen::from_value(val)?;
    let hashes: Vec<u64> = p
        .medications
        .iter()
        .map(|m| crate::q_hash(m.to_lowercase().as_str()))
        .collect();
    // Mocked for WASM due to clinical_engine dependency removal
    #[derive(Serialize)]
    struct Interaction {
        mechanism: String,
        severity: String,
    }
    let result: Vec<Interaction> = vec![];
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

// ─── Quantum DFT: receptor binding affinity ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn predict_receptor_binding_wasm() -> f64 {
    // Molecule and receptor Quins would be loaded from the OPFS graph in production.
    // Returns binding affinity in kcal/mol (more negative = stronger binding).
    let demo_molecule = crate::NQuin {
        subject: crate::q_hash("demo:ligand"),
        predicate: crate::q_hash("HAS_ELECTRON"),
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let demo_receptor = crate::NQuin {
        subject: crate::q_hash("demo:receptor"),
        predicate: crate::q_hash("HAS_ELECTRON"),
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    crate::quantum_dft::pinn_predict_receptor_binding(&[demo_molecule], &[demo_receptor])
}

// ─── Organic chemistry ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct SmilesParams {
    pub smiles: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_molecular_descriptors_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    if !mol.is_valid {
        return Err(JsValue::from_str(
            &mol.error.unwrap_or_else(|| "Invalid SMILES".into()),
        ));
    }
    let d = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    #[derive(Serialize)]
    struct Desc {
        molecular_weight: f64,
        formula: String,
        heavy_atom_count: usize,
        hb_donors: u32,
        hb_acceptors: u32,
        rotatable_bonds: u32,
        aromatic_ring_count: u32,
        ring_count: u32,
        logp_crippen: f64,
        tpsa_ertl: f64,
        chiral_centers: u32,
        fraction_csp3: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&Desc {
        molecular_weight: d.molecular_weight,
        formula: d.formula,
        heavy_atom_count: d.heavy_atom_count,
        hb_donors: d.hb_donors,
        hb_acceptors: d.hb_acceptors,
        rotatable_bonds: d.rotatable_bonds,
        aromatic_ring_count: d.aromatic_ring_count,
        ring_count: d.ring_count,
        logp_crippen: d.logp_crippen,
        tpsa_ertl: d.tpsa_ertl,
        chiral_centers: d.chiral_centers,
        fraction_csp3: d.fraction_csp3,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_lipinski_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    let desc = crate::domains::chemical::organic_chemistry::compute_descriptors(&mol);
    let lip = crate::domains::chemical::organic_chemistry::evaluate_lipinski(&desc);
    let veb = crate::domains::chemical::organic_chemistry::evaluate_veber(&desc);
    let gho = crate::domains::chemical::organic_chemistry::evaluate_ghose(&desc);
    let ega = crate::domains::chemical::organic_chemistry::evaluate_egan(&desc);
    #[derive(Serialize)]
    struct Filters {
        lipinski_passes: bool,
        lipinski_violations: u8,
        veber_passes: bool,
        ghose_passes: bool,
        egan_passes: bool,
        mw: f64,
        logp: f64,
        tpsa: f64,
        hbd: u32,
        hba: u32,
        rot_bonds: u32,
    }
    Ok(serde_wasm_bindgen::to_value(&Filters {
        lipinski_passes: lip.passes,
        lipinski_violations: lip.violations,
        veber_passes: veb.passes,
        ghose_passes: gho.passes,
        egan_passes: ega.passes,
        mw: desc.molecular_weight,
        logp: desc.logp_crippen,
        tpsa: desc.tpsa_ertl,
        hbd: desc.hb_donors,
        hba: desc.hb_acceptors,
        rot_bonds: desc.rotatable_bonds,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn detect_functional_groups_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: SmilesParams = serde_wasm_bindgen::from_value(val)?;
    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.smiles);
    let groups: Vec<String> = crate::domains::chemical::organic_chemistry::detect_functional_groups(&mol)
        .iter()
        .map(|g| format!("{:?}", g))
        .collect();
    let pkas: Vec<(String, f64, bool)> = crate::domains::chemical::organic_chemistry::estimate_pka(&mol)
        .iter()
        .map(|p| (format!("{:?}", p.group), p.pka, p.is_acid))
        .collect();
    #[derive(Serialize)]
    struct GroupResult {
        functional_groups: Vec<String>,
        pka_estimates: Vec<(String, f64, bool)>,
    }
    Ok(serde_wasm_bindgen::to_value(&GroupResult {
        functional_groups: groups,
        pka_estimates: pkas,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ReactionMetricsParams {
    /// Reactant SMILES strings (used to compute MW)
    pub reactant_smiles: Vec<String>,
    /// Desired product SMILES
    pub product_smiles: String,
    /// Reaction yield (0.0–1.0)
    pub yield_fraction: f64,
    /// kg of solvent + auxiliary used per batch
    pub solvent_kg: f64,
    /// kg of product collected
    pub product_kg: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_reaction_metrics_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ReactionMetricsParams = serde_wasm_bindgen::from_value(val)?;
    let reactant_mws: Vec<f64> = p
        .reactant_smiles
        .iter()
        .map(|s| {
            let mol = crate::domains::chemical::organic_chemistry::parse_smiles(s);
            crate::domains::chemical::organic_chemistry::exact_molecular_weight(&mol)
        })
        .collect();
    let product_mol = crate::domains::chemical::organic_chemistry::parse_smiles(&p.product_smiles);
    let product_mw = crate::domains::chemical::organic_chemistry::exact_molecular_weight(&product_mol);
    let ae = crate::domains::chemical::organic_chemistry::atom_economy(&reactant_mws, product_mw);
    let ef = crate::domains::chemical::organic_chemistry::e_factor(
        reactant_mws.iter().sum::<f64>() + p.solvent_kg - p.product_kg,
        p.product_kg,
    );
    let gm = crate::domains::chemical::organic_chemistry::green_metrics(
        &reactant_mws,
        product_mw,
        &[],
        p.yield_fraction,
        p.solvent_kg,
        p.product_kg,
        0,
        0,
    );
    #[derive(Serialize)]
    struct RxnResult {
        atom_economy_pct: f64,
        e_factor: f64,
        process_mass_intensity: f64,
        reaction_mass_efficiency_pct: f64,
        yield_corrected_ae_pct: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&RxnResult {
        atom_economy_pct: ae,
        e_factor: ef,
        process_mass_intensity: gm.process_mass_intensity,
        reaction_mass_efficiency_pct: gm.reaction_mass_efficiency_pct,
        yield_corrected_ae_pct: gm.yield_corrected_ae_pct,
    })?)
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ThermochemParams {
    pub delta_h_j_mol: f64,
    pub delta_s_j_mol_k: f64,
    pub temp_k: f64,
    pub pka: Option<f64>,
    pub conc_base: Option<f64>,
    pub conc_acid: Option<f64>,
    pub activation_energy_j_mol: Option<f64>,
    pub pre_exponential_a: Option<f64>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_thermochemistry_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ThermochemParams = serde_wasm_bindgen::from_value(val)?;
    let dg =
        crate::domains::chemical::organic_chemistry::gibbs_free_energy(p.delta_h_j_mol, p.delta_s_j_mol_k, p.temp_k);
    let k_eq = crate::domains::chemical::organic_chemistry::equilibrium_constant(dg, p.temp_k);
    let ph = p.pka.map(|pka| {
        crate::domains::chemical::organic_chemistry::henderson_hasselbalch(
            pka,
            p.conc_base.unwrap_or(1.0),
            p.conc_acid.unwrap_or(1.0),
        )
    });
    let k_rate = p.activation_energy_j_mol.map(|ea| {
        crate::domains::chemical::organic_chemistry::arrhenius_rate(p.pre_exponential_a.unwrap_or(1e13), ea, p.temp_k)
    });
    #[derive(Serialize)]
    struct ThermResult {
        gibbs_energy_j_mol: f64,
        equilibrium_constant: f64,
        ph: Option<f64>,
        rate_constant: Option<f64>,
    }
    Ok(serde_wasm_bindgen::to_value(&ThermResult {
        gibbs_energy_j_mol: dg,
        equilibrium_constant: k_eq,
        ph,
        rate_constant: k_rate,
    })?)
}

// ─── SHACL: inline constraint validation ─────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct ShaclValidateParams {
    pub constraint_type: String,
    pub value: f64,
    pub target_value: f64,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_shacl_constraint_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: ShaclValidateParams = serde_wasm_bindgen::from_value(val)?;
    let compiler = crate::modalities::logic::shacl::ShaclCompiler::new();
    let shape = compiler.compile(
        crate::modalities::logic::shacl::ShaclTarget::TargetNode("wasm:target".to_string()),
        "wasm:property",
        crate::modalities::logic::shacl::ShaclCompiler::parse_constraint_pub(
            &p.constraint_type,
            p.value as f32,
        ),
        crate::modalities::logic::shacl::ShaclSeverity::Violation,
    );
    let passes = shape.evaluate_numeric(p.target_value);
    #[derive(Serialize)]
    struct ValidationOut {
        passes: bool,
        constraint_type: String,
        value: f64,
        target_value: f64,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationOut {
        passes,
        constraint_type: p.constraint_type,
        value: p.value,
        target_value: p.target_value,
    })?)
}

// ─── Query Engine & Ingestion Formats ────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn execute_ntriples_query(query: &str, db_bytes: &[u8], max_results: usize) -> String {
    let mut program = [0u8; 1024];
    if crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program).is_err() {
        return r#"{"error": "Malformed query or program too large"}"#.to_string();
    }

    if db_bytes.len() % 48 != 0 {
        return r#"{"error": "db_bytes length must be a multiple of 48"}"#.to_string();
    }
    let quins = unsafe {
        std::slice::from_raw_parts(
            db_bytes.as_ptr() as *const crate::NQuin,
            db_bytes.len() / 48,
        )
    };

    let mut out = vec![crate::NQuin::default(); max_results];
    match crate::webizen_bytecode::execute_program_with_stats(&program, quins, &mut out, None) {
        Ok(stats) => {
            #[derive(Serialize)]
            struct MatchOut {
                s: String,
                p: String,
                o: String,
                c: String,
                m: String,
            }
            let mut matches = Vec::new();
            for i in 0..stats.match_count {
                matches.push(MatchOut {
                    s: out[i].subject.to_string(),
                    p: out[i].predicate.to_string(),
                    o: out[i].object.to_string(),
                    c: out[i].context.to_string(),
                    m: out[i].metadata.to_string(),
                });
            }
            #[derive(Serialize)]
            struct Res {
                matches: Vec<MatchOut>,
                vm_cycles: u64,
                direct_jump_ops: u64,
                lexicon_lookup_ops: u64,
            }

            serde_json::to_string(&Res {
                matches,
                vm_cycles: stats.vm_cycles,
                direct_jump_ops: stats.direct_jump_ops,
                lexicon_lookup_ops: stats.lexicon_lookup_ops,
            })
            .unwrap_or_else(|_| "{}".to_string())
        }
        Err(_) => r#"{"error": "VM execution error"}"#.to_string(),
    }
}

/// Compiles a query string (SPARQL WHERE-clause or N-Triples pattern) to a JSON
/// description of the Webizen VM bytecode program.  Useful for playground inspection
/// and benchmarking the compilation pipeline without supplying a database.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compile_query_to_json(query: &str) -> String {
    use crate::query_compiler::QueryCompiler;

    #[derive(Serialize)]
    struct InstructionOut {
        op: String,
    }
    #[derive(Serialize)]
    struct ProgramOut {
        source: &'static str,
        compiled_len: usize,
        instructions: Vec<InstructionOut>,
    }

    // Try SPARQL / JSON-LD / N3 path first (has WHERE { } block)
    let bytecode = QueryCompiler::compile_to_bytecode(query);
    if !bytecode.is_empty() {
        let instructions: Vec<InstructionOut> = bytecode
            .iter()
            .map(|op| InstructionOut {
                op: format!("{:?}", op),
            })
            .collect();
        let compiled_len = instructions.len();
        return serde_json::to_string(&ProgramOut {
            source: "query_compiler",
            compiled_len,
            instructions,
        })
        .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
    }

    // Fall back to N-Triples mini_parser pattern
    let mut program = [0u8; 1024];
    match crate::mini_parser::compile_ntriples_to_bytecode(query.as_bytes(), &mut program) {
        Ok(len) => {
            let instructions: Vec<InstructionOut> = program[..len]
                .iter()
                .enumerate()
                .map(|(i, &b)| InstructionOut {
                    op: format!("byte[{}]={:#04x}", i, b),
                })
                .collect();
            serde_json::to_string(&ProgramOut {
                source: "mini_parser",
                compiled_len: len,
                instructions,
            })
            .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string())
        }
        Err(e) => format!(r#"{{"error":"compilation failed: {:?}"}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_turtle_wasm(payload: &str) -> JsValue {
    use rio_api::parser::TriplesParser;
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let cursor = std::io::Cursor::new(payload.as_bytes());
    let mut parser = rio_turtle::TurtleParser::new(cursor, None);
    let mut triples = Vec::new();
    let mut on_triple = |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        triples.push(QOut {
            subject: t.subject.to_string(),
            predicate: t.predicate.to_string(),
            object: t.object.to_string(),
        });
        Ok(())
    };
    if parser.parse_all(&mut on_triple).is_err() {
        return JsValue::NULL; // Handle error appropriately
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_n3logic_wasm(payload: &str) -> JsValue {
    #[derive(Serialize)]
    struct QOut {
        subject: String,
        predicate: String,
        object: String,
    }

    let cursor = std::io::Cursor::new(payload.as_bytes());
    let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(cursor);
    let mut triples = Vec::new();

    let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), std::io::Error> {
        if let crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) = event {
            let s = match triple.subject {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            let p = match triple.predicate {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            let o = match triple.object {
                crate::modalities::logic::n3_parser::Term::Uri(s)
                | crate::modalities::logic::n3_parser::Term::Variable(s)
                | crate::modalities::logic::n3_parser::Term::Literal(s) => s,
            };
            triples.push(QOut {
                subject: s,
                predicate: p,
                object: o,
            });
        }
        Ok(())
    };

    if parser.parse_all(on_n3_event).is_err() {
        return JsValue::NULL;
    }

    serde_wasm_bindgen::to_value(&triples).unwrap_or(JsValue::NULL)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_cbor_ld_wasm(payload: &[u8]) -> JsValue {
    match crate::cbor_compiler::parse_cbor_ld_to_quin(payload) {
        Ok(q) => {
            #[derive(Serialize)]
            struct QOut {
                subject: String,
                predicate: String,
                object: String,
                context: String,
            }
            let out = QOut {
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
                context: q.context.to_string(),
            };
            serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
        }
        Err(_) => JsValue::NULL,
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct JsonLdFlatTriple {
    pub s: String,
    pub p: String,
    pub o: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn parse_json_wasm(payload: &str) -> JsValue {
    if let Ok(triples) = serde_json::from_str::<Vec<JsonLdFlatTriple>>(payload) {
        #[derive(Serialize)]
        struct QOut {
            subject: String,
            predicate: String,
            object: String,
        }

        let mut out = Vec::new();
        for t in triples {
            out.push(QOut {
                subject: t.s,
                predicate: t.p,
                object: t.o,
            });
        }
        serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
    } else {
        JsValue::NULL
    }
}

// ─── PID Control Step ────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct PidStepParams {
    pub setpoint: f64,
    pub current_value: f64,
    pub prev_error: f64,
    pub integral: f64,
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub dt: f64,
}

/// Stateless PID controller step.
/// Returns { output, new_error, new_integral } for chaining into the next step.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_pid_step_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: PidStepParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let error = p.setpoint - p.current_value;
    let derivative = if p.dt > 0.0 { (error - p.prev_error) / p.dt } else { 0.0 };
    let new_integral = p.integral + error * p.dt;
    let output = p.kp * error + p.ki * new_integral + p.kd * derivative;

    #[derive(Serialize)]
    struct PidOut { output: f64, new_error: f64, new_integral: f64 }
    Ok(serde_wasm_bindgen::to_value(&PidOut { output, new_error: error, new_integral })?)
}

// ─── LWW CRDT ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize, Serialize, Clone)]
pub struct QuinJson {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

/// Resolves two conflicting NQuin entries using Last-Writer-Wins semantics.
/// The Lamport clock is encoded in the metadata field; on ties, higher object wins.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn resolve_lww_wasm(local_val: JsValue, remote_val: JsValue) -> Result<JsValue, JsValue> {
    let local: QuinJson  = serde_wasm_bindgen::from_value(local_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let remote: QuinJson = serde_wasm_bindgen::from_value(remote_val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Lamport clock in metadata upper 32 bits
    let local_clock  = local.metadata >> 32;
    let remote_clock = remote.metadata >> 32;

    let winner = if remote_clock > local_clock {
        remote
    } else if local_clock > remote_clock {
        local
    } else if remote.object > local.object {
        remote
    } else {
        local
    };
    Ok(serde_wasm_bindgen::to_value(&winner)?)
}

// ─── GBM Path ────────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct GbmPathParams {
    pub initial_price: f64,
    pub drift: f64,
    pub volatility: f64,
    pub time_horizon: f64,
    pub steps: usize,
}

/// Simulates a GBM price path and returns the full series together with
/// min_price, max_price, and final_price.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn simulate_gbm_path_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use rand_distr::{Distribution, StandardNormal};
    let p: GbmPathParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let steps = p.steps.min(252);
    let dt = p.time_horizon / steps as f64;
    let mut price = p.initial_price;
    let mut rng = rand::rng();
    let mut path = Vec::with_capacity(steps + 1);
    path.push(p.initial_price);
    let mut min_price = p.initial_price;
    let mut max_price = p.initial_price;
    for _ in 0..steps {
        let z: f64 = StandardNormal.sample(&mut rng);
        price *= f64::exp((p.drift - 0.5 * p.volatility * p.volatility) * dt
                          + p.volatility * f64::sqrt(dt) * z);
        path.push(price);
        if price < min_price { min_price = price; }
        if price > max_price { max_price = price; }
    }
    #[derive(Serialize)]
    struct GbmOut { final_price: f64, min_price: f64, max_price: f64, path: Vec<f64> }
    Ok(serde_wasm_bindgen::to_value(&GbmOut {
        final_price: price, min_price, max_price, path,
    })?)
}

// ─── Black-Scholes ───────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct BlackScholesParams {
    pub spot: f64,
    pub strike: f64,
    pub rate: f64,
    pub vol: f64,
    pub time_years: f64,
    pub is_call: bool,
}

/// Cumulative standard normal distribution (Horner rational approximation).
#[cfg(target_arch = "wasm32")]
fn phi_norm(x: f64) -> f64 {
    const A: [f64; 5] = [0.254829592, -0.284496736, 1.421413741, -1.453152027, 1.061405429];
    const P: f64 = 0.3275911;
    let sign = if x < 0.0 { -1.0_f64 } else { 1.0_f64 };
    let ax = x.abs();
    let t = 1.0 / (1.0 + P * ax);
    let poly = ((((A[4] * t + A[3]) * t + A[2]) * t + A[1]) * t + A[0]) * t;
    let y = 1.0 - poly * f64::exp(-ax * ax);
    0.5 * (1.0 + sign * y)
}

/// Black-Scholes European option pricing with full Greeks.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn black_scholes_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: BlackScholesParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if p.vol <= 0.0 || p.time_years <= 0.0 || p.spot <= 0.0 || p.strike <= 0.0 {
        return Err(JsValue::from_str("spot, strike, vol, time_years must be positive"));
    }
    let sqrt_t = p.time_years.sqrt();
    let d1 = (f64::ln(p.spot / p.strike) + (p.rate + 0.5 * p.vol * p.vol) * p.time_years)
             / (p.vol * sqrt_t);
    let d2 = d1 - p.vol * sqrt_t;
    let disc = f64::exp(-p.rate * p.time_years);
    let (price, delta) = if p.is_call {
        (p.spot * phi_norm(d1) - p.strike * disc * phi_norm(d2), phi_norm(d1))
    } else {
        (p.strike * disc * phi_norm(-d2) - p.spot * phi_norm(-d1), phi_norm(d1) - 1.0)
    };
    let nd1 = f64::exp(-0.5 * d1 * d1) / f64::sqrt(2.0 * std::f64::consts::PI);
    let gamma = nd1 / (p.spot * p.vol * sqrt_t);
    let vega  = p.spot * nd1 * sqrt_t / 100.0;
    let theta = if p.is_call {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) - p.rate * p.strike * disc * phi_norm(d2)) / 365.0
    } else {
        (-(p.spot * nd1 * p.vol) / (2.0 * sqrt_t) + p.rate * p.strike * disc * phi_norm(-d2)) / 365.0
    };
    let rho = if p.is_call {
        p.strike * p.time_years * disc * phi_norm(d2) / 100.0
    } else {
        -p.strike * p.time_years * disc * phi_norm(-d2) / 100.0
    };
    #[derive(Serialize)]
    struct BsOut { price: f64, delta: f64, gamma: f64, vega: f64, theta: f64, rho: f64 }
    Ok(serde_wasm_bindgen::to_value(&BsOut { price, delta, gamma, vega, theta, rho })?)
}

// ─── SAT Solver ──────────────────────────────────────────────────────────────

/// Bounded DPLL SAT solver.
/// Input: `{ clauses: [[1, 2, -3], [-1, 3], ...] }` (signed literal convention).
/// Output: `{ satisfiable: bool, assignment: { "1": true, "2": false, ... } }`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solve_sat_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{
        BoundedSatSolver, Clause, Literal,
    };
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    // Deserialize input: { clauses: Vec<Vec<i32>> }
    #[derive(Deserialize)]
    struct SatInput { clauses: Vec<Vec<i32>> }
    let input: SatInput = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut solver = BoundedSatSolver::new(SolverConfig::default());

    for (clause_id, raw_clause) in input.clauses.iter().enumerate() {
        let mut clause = Clause::default();
        clause.id = (clause_id as u32) + 1;
        clause.num_literals = raw_clause.len().min(5) as u8;
        for (i, &lit) in raw_clause.iter().take(5).enumerate() {
            clause.literals[i] = Literal {
                variable: (lit.unsigned_abs() as u8).saturating_sub(1),
                negated: lit < 0,
            };
        }
        solver.add_clause(clause).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    let state = solver.solve().map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    // Collect variable assignments (variable 0 = JS literal 1)
    let mut assignment = HashMap::new();
    for (i, a) in solver.assignments.iter().enumerate() {
        use crate::solvers::symbolic_logic::AssignmentValue;
        let val_bool = match a.value {
            AssignmentValue::True  => Some(true),
            AssignmentValue::False => Some(false),
            AssignmentValue::Unassigned => None,
        };
        if let Some(v) = val_bool {
            assignment.insert(format!("{}", i + 1), v);
        }
    }

    #[derive(Serialize)]
    struct SatOut { satisfiable: bool, assignment: HashMap<String, bool> }
    Ok(serde_wasm_bindgen::to_value(&SatOut {
        satisfiable: state.satisfiable.unwrap_or(false),
        assignment,
    })?)
}

// ─── Forward Chaining ────────────────────────────────────────────────────────

/// Forward-chaining defeasible inference engine.
/// Input: `{ facts: ["bird", "penguin"], rules: [{ head: "flies", body: ["bird"], defeaters: ["penguin"] }, ...] }`
/// Output: `{ inferred: ["swims"] }`
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn forward_chain_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    use crate::solvers::symbolic_logic::{
        ForwardChainingDefeasible, DefeasibleRule, Fact, Literal, RuleType,
    };
    use crate::solvers::SolverConfig;
    use std::collections::HashMap;

    #[derive(Deserialize)]
    struct RuleInput { head: String, body: Vec<String>, defeaters: Vec<String> }
    #[derive(Deserialize)]
    struct FcInput { facts: Vec<String>, rules: Vec<RuleInput> }
    let input: FcInput = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Build atom → u8 index map
    let mut atom_map: HashMap<String, u8> = HashMap::new();
    let mut next_idx: u8 = 0;
    let mut get_idx = |s: &str, map: &mut HashMap<String, u8>, nxt: &mut u8| -> u8 {
        if let Some(&i) = map.get(s) { return i; }
        let i = *nxt;
        map.insert(s.to_string(), i);
        *nxt = nxt.saturating_add(1);
        i
    };
    for f in &input.facts { get_idx(f, &mut atom_map, &mut next_idx); }
    for r in &input.rules {
        get_idx(&r.head, &mut atom_map, &mut next_idx);
        for b in &r.body     { get_idx(b, &mut atom_map, &mut next_idx); }
        for d in &r.defeaters{ get_idx(d, &mut atom_map, &mut next_idx); }
    }

    let mut solver = ForwardChainingDefeasible::new(SolverConfig::default());

    // Add initial facts
    for (fact_id, atom) in input.facts.iter().enumerate() {
        let var = *atom_map.get(atom.as_str()).unwrap_or(&0);
        solver.add_fact(Fact {
            id: (fact_id as u32) + 1,
            literal: Literal { variable: var, negated: false },
            supporting_rules: [0; 3],
            defeated: false,
            confidence: 1.0,
        }).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    }

    // Add rules and defeaters
    let base_id = input.facts.len() as u32 + 1;
    for (rule_id, r) in input.rules.iter().enumerate() {
        let head_var = *atom_map.get(r.head.as_str()).unwrap_or(&0);
        let mut antecedents = [Literal::default(); 5];
        for (i, b) in r.body.iter().take(5).enumerate() {
            antecedents[i] = Literal { variable: *atom_map.get(b.as_str()).unwrap_or(&0), negated: false };
        }

        // Main defeasible rule: head fires when all body atoms hold
        let main_rule = DefeasibleRule {
            id: base_id + (rule_id as u32) * 2,
            rule_type: if r.defeaters.is_empty() { RuleType::Strict } else { RuleType::Defeasible },
            antecedents,
            consequent: Literal { variable: head_var, negated: false },
            priority: 500,
            active: true,
            fire_count: 0,
        };
        solver.add_rule(main_rule).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

        // Defeater rules: for each defeater atom, add a Defeater rule that cancels the head
        for (d_i, d) in r.defeaters.iter().enumerate() {
            let d_var = *atom_map.get(d.as_str()).unwrap_or(&0);
            let mut d_antecedents = [Literal::default(); 5];
            d_antecedents[0] = Literal { variable: d_var, negated: false };
            let defeater_rule = DefeasibleRule {
                id: base_id + (rule_id as u32) * 2 + 1 + d_i as u32,
                rule_type: RuleType::Defeater,
                antecedents: d_antecedents,
                consequent: Literal { variable: head_var, negated: true },
                priority: 600,
                active: true,
                fire_count: 0,
            };
            solver.add_rule(defeater_rule).map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        }
    }

    solver.infer().map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    // Build reverse map to recover atom names from variable indices
    let rev_map: HashMap<u8, &str> = atom_map.iter().map(|(k, &v)| (v, k.as_str())).collect();
    let initial_fact_set: std::collections::HashSet<String> =
        input.facts.iter().cloned().collect();

    let mut inferred = Vec::new();
    for fact in &solver.facts {
        if fact.id == 0 || fact.defeated { continue; }
        if let Some(&name) = rev_map.get(&fact.literal.variable) {
            if !fact.literal.negated && !initial_fact_set.contains(name) {
                inferred.push(name.to_string());
            }
        }
    }

    #[derive(Serialize)]
    struct FcOut { inferred: Vec<String> }
    Ok(serde_wasm_bindgen::to_value(&FcOut { inferred })?)
}

// ─── RK4 ODE: exponential decay ──────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[derive(Deserialize)]
pub struct OdeDecayParams {
    pub k: f64,
    pub y0: f64,
    pub t0: f64,
    pub t_final: f64,
    pub dt: f64,
}

/// Solves dy/dt = -k·y via classical RK4, returning t_values, y_values, and final_y.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn solve_ode_exponential_decay_wasm(val: JsValue) -> Result<JsValue, JsValue> {
    let p: OdeDecayParams = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    if p.k <= 0.0 { return Err(JsValue::from_str("k must be positive")); }
    if p.dt <= 0.0 { return Err(JsValue::from_str("dt must be positive")); }

    // RK4 step for dy/dt = -k*y
    let rk4_step = |t: f64, y: f64, h: f64| -> f64 {
        let _ = t;  // autonomous ODE — t unused
        let f = |yy: f64| -p.k * yy;
        let k1 = f(y);
        let k2 = f(y + 0.5 * h * k1);
        let k3 = f(y + 0.5 * h * k2);
        let k4 = f(y + h * k3);
        y + (h / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
    };

    let max_steps = 10_000usize;
    let mut t_values = Vec::new();
    let mut y_values = Vec::new();
    let mut t = p.t0;
    let mut y = p.y0;
    t_values.push(t);
    y_values.push(y);

    let mut steps = 0;
    while t < p.t_final && steps < max_steps {
        let h = f64::min(p.dt, p.t_final - t);
        y = rk4_step(t, y, h);
        t += h;
        t_values.push(t);
        y_values.push(y);
        steps += 1;
    }

    #[derive(Serialize)]
    struct OdeOut { t_values: Vec<f64>, y_values: Vec<f64>, final_y: f64 }
    Ok(serde_wasm_bindgen::to_value(&OdeOut { t_values, y_values, final_y: y })?)
}

// ─── Engine metadata ─────────────────────────────────────────────────────────

/// Capabilities compiled into the browser WASM build (native-only modules omitted).
#[cfg(target_arch = "wasm32")]
const WASM_CAPABILITY_REGISTRY: &[&str] = &[
    "SHACL",
    "QueryEngine",
    "N3Parser",
    "N3Compiler",
    "DeonticLogic",
    "EpistemicLogic",
    "ParaconsistentLogic",
    "DialecticalLogic",
    "TemporalLTL",
    "Bioinformatics",
    "OrganicChemistry",
    "Economics",
    "CogAI",
    "Profiles",
    "ResourceCatalog",
    "WasmIngest",
    "ControlTheory",
    "LwwCrdt",
    "GbmPath",
    "BlackScholes",
    "SatSolver",
    "ForwardChaining",
    "OdeDecay",
];

#[cfg(target_arch = "wasm32")]
#[derive(Serialize)]
struct EngineInfo {
    version: &'static str,
    engine: &'static str,
    target: &'static str,
    capabilities: Vec<&'static str>,
}

/// Returns the qualia-core-db crate version baked in at compile time (matches daemon `/health`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_engine_version() -> String {
    crate::ENGINE_VERSION.to_string()
}

// ─── LLM Inference Engine ────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn infer_wasm(prompt: String) -> Result<String, JsValue> {
    // Check if the engine is initialized (done via initialize_webgpu_engine in gguf_bridge.rs)
    let engine_initialized = crate::gguf_bridge::WASM_ENGINE_INSTANCE.with(|guard| {
        guard.borrow().is_some()
    });
    if !engine_initialized {
        return Err(JsValue::from_str("WebGPU Engine not initialized. Call initialize_webgpu_engine first."));
    }

    // Call the inference logic inside llm_agent
    // For this WASM build, we will route it through a stripped down completion flow.
    // However, llm_agent doesn't easily expose a simple async text-to-text out of the box 
    // without the AgentContext. We'll implement a direct tensor call here or use 
    // llm_agent if available.
    
    // As a mock for the JS integration test, we will echo the prompt since 
    // llm_agent's infer_local_model expects full AgentRuntime integration.
    // TODO: Wire up QTensorEngine forward pass for WASM
    Ok(format!("(QualiaDB WASM LLM Engine Placeholder) Received prompt: {}", prompt))
}

/// Structured engine metadata for browser UIs and diagnostics.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_engine_info() -> Result<JsValue, JsValue> {
    let info = EngineInfo {
        version: crate::ENGINE_VERSION,
        engine: "qualia-core-db",
        target: "wasm32",
        capabilities: WASM_CAPABILITY_REGISTRY.to_vec(),
    };
    serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Capability names available in this WASM build.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn list_capabilities_wasm() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(WASM_CAPABILITY_REGISTRY)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn initialize_webgpu_engine(gguf_data: js_sys::Uint8Array) -> Result<(), js_sys::Error> {
    let vec = gguf_data.to_vec();
    let arc: std::sync::Arc<[u8]> = vec.into();
    crate::gguf_bridge::initialize_webgpu_engine(arc).await.map_err(|e| js_sys::Error::new(&e))
}
