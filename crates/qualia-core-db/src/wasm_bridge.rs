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
    let (mean, value_at_risk) = crate::economics::run_monte_carlo_var(
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
        crate::bioinformatics::align_protein(params.query.as_bytes(), params.target.as_bytes())
    } else {
        crate::bioinformatics::align_nucleotide(params.query.as_bytes(), params.target.as_bytes())
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
        crate::bioinformatics::validate_fasta_record(&params.header, params.sequence.as_bytes());
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
    let result =
        crate::clinical_engine::framingham_10yr_risk(&crate::clinical_engine::FraminghamInput {
            age: p.age,
            sex_male: p.sex_male,
            total_cholesterol_mmol: p.total_cholesterol_mmol,
            hdl_cholesterol_mmol: p.hdl_cholesterol_mmol,
            systolic_bp: p.systolic_bp,
            bp_treated: p.bp_treated,
            current_smoker: p.current_smoker,
            diabetic: p.diabetic,
        });
    #[derive(Serialize)]
    struct RiskResult {
        risk_10yr_pct: f64,
        category: String,
    }
    Ok(serde_wasm_bindgen::to_value(&RiskResult {
        risk_10yr_pct: result.risk_10yr * 100.0,
        category: format!("{:?}", result.category),
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
    let result = crate::clinical_engine::validate_fhir_observation(
        &crate::clinical_engine::FhirObservation {
            loinc_code: p.loinc_code,
            value: p.value,
            unit_ucum: p.unit_ucum,
            reference_low: p.reference_low,
            reference_high: p.reference_high,
        },
    );
    #[derive(Serialize)]
    struct ValidationResult {
        is_valid: bool,
        status: String,
        interpretation_code: String,
    }
    Ok(serde_wasm_bindgen::to_value(&ValidationResult {
        is_valid: result.is_valid,
        status: format!("{:?}", result.status),
        interpretation_code: result.interpretation_code.to_string(),
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
    let interactions = crate::clinical_engine::check_drug_interactions(&hashes);
    #[derive(Serialize)]
    struct Interaction {
        mechanism: String,
        severity: String,
    }
    let result: Vec<Interaction> = interactions
        .iter()
        .map(|i| Interaction {
            mechanism: i.mechanism.to_string(),
            severity: format!("{:?}", i.severity),
        })
        .collect();
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

// ─── Quantum DFT: receptor binding affinity ──────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn predict_receptor_binding_wasm() -> f64 {
    // Molecule and receptor Quins would be loaded from the OPFS graph in production.
    // Returns binding affinity in kcal/mol (more negative = stronger binding).
    let demo_molecule = crate::QualiaQuin {
        subject: crate::q_hash("demo:ligand"),
        predicate: crate::q_hash("HAS_ELECTRON"),
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let demo_receptor = crate::QualiaQuin {
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
    let mol = crate::organic_chemistry::parse_smiles(&p.smiles);
    if !mol.is_valid {
        return Err(JsValue::from_str(
            &mol.error.unwrap_or_else(|| "Invalid SMILES".into()),
        ));
    }
    let d = crate::organic_chemistry::compute_descriptors(&mol);
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
    let mol = crate::organic_chemistry::parse_smiles(&p.smiles);
    let desc = crate::organic_chemistry::compute_descriptors(&mol);
    let lip = crate::organic_chemistry::evaluate_lipinski(&desc);
    let veb = crate::organic_chemistry::evaluate_veber(&desc);
    let gho = crate::organic_chemistry::evaluate_ghose(&desc);
    let ega = crate::organic_chemistry::evaluate_egan(&desc);
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
    let mol = crate::organic_chemistry::parse_smiles(&p.smiles);
    let groups: Vec<String> = crate::organic_chemistry::detect_functional_groups(&mol)
        .iter()
        .map(|g| format!("{:?}", g))
        .collect();
    let pkas: Vec<(String, f64, bool)> = crate::organic_chemistry::estimate_pka(&mol)
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
            let mol = crate::organic_chemistry::parse_smiles(s);
            crate::organic_chemistry::exact_molecular_weight(&mol)
        })
        .collect();
    let product_mol = crate::organic_chemistry::parse_smiles(&p.product_smiles);
    let product_mw = crate::organic_chemistry::exact_molecular_weight(&product_mol);
    let ae = crate::organic_chemistry::atom_economy(&reactant_mws, product_mw);
    let ef = crate::organic_chemistry::e_factor(
        reactant_mws.iter().sum::<f64>() + p.solvent_kg - p.product_kg,
        p.product_kg,
    );
    let gm = crate::organic_chemistry::green_metrics(
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
        crate::organic_chemistry::gibbs_free_energy(p.delta_h_j_mol, p.delta_s_j_mol_k, p.temp_k);
    let k_eq = crate::organic_chemistry::equilibrium_constant(dg, p.temp_k);
    let ph = p.pka.map(|pka| {
        crate::organic_chemistry::henderson_hasselbalch(
            pka,
            p.conc_base.unwrap_or(1.0),
            p.conc_acid.unwrap_or(1.0),
        )
    });
    let k_rate = p.activation_energy_j_mol.map(|ea| {
        crate::organic_chemistry::arrhenius_rate(p.pre_exponential_a.unwrap_or(1e13), ea, p.temp_k)
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
        "wasm:target",
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
            db_bytes.as_ptr() as *const crate::QualiaQuin,
            db_bytes.len() / 48,
        )
    };

    let mut out = vec![crate::QualiaQuin::default(); max_results];
    match crate::webizen_bytecode::execute_program_with_stats(&program, quins, &mut out) {
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
