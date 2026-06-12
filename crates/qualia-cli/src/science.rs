// science.rs — CLI runners for domain-science modules:
//   chem, bio, geo, thermo, geometric algebra, clinical, economics

// ── Chemistry ─────────────────────────────────────────────────────────────────

pub fn run_chem_smiles(smiles: &str) {
    use qualia_core_db::domains::chemical::organic_chemistry::{
        parse_smiles, compute_descriptors, compute_logp, compute_tpsa,
        evaluate_lipinski, formula_string, exact_molecular_weight,
    };
    let mol  = parse_smiles(smiles);
    let desc = compute_descriptors(&mol);
    println!("SMILES: {smiles}");
    println!("  Formula  : {}", formula_string(&mol));
    println!("  Exact MW : {:.4} Da", exact_molecular_weight(&mol));
    println!("  LogP     : {:.4}", compute_logp(&mol));
    println!("  TPSA     : {:.4} Å²", compute_tpsa(&mol));
    println!("  HBA      : {}", desc.hb_acceptors);
    println!("  HBD      : {}", desc.hb_donors);
    println!("  Rot bonds: {}", desc.rotatable_bonds);
    let lip  = evaluate_lipinski(&desc);
    println!("  Lipinski : {} (violations: {})", if lip.passes { "PASS" } else { "FAIL" }, lip.violations);
}

pub fn run_chem_thermo(reaction: &str, a: f64, b: f64, c: f64) {
    use qualia_core_db::domains::chemical::organic_chemistry::{
        arrhenius_rate, gibbs_free_energy, henderson_hasselbalch,
    };
    match reaction.to_ascii_lowercase().as_str() {
        "arrhenius" => {
            // a = pre-exponential factor, b = activation energy (J/mol), c = temp (K)
            let rate = arrhenius_rate(a, b, c);
            println!("Arrhenius rate: k = {rate:.6e} s⁻¹");
            println!("  A={a:.3e}, Ea={b:.3e} J/mol, T={c} K");
        }
        "gibbs" => {
            // a = ΔH (kJ/mol), b = ΔS (J/mol·K), c = temp (K)
            let dg = gibbs_free_energy(a * 1000.0, b, c);
            println!("Gibbs free energy: ΔG = {dg:.4} J/mol ({:.4} kJ/mol)", dg / 1000.0);
            println!("  ΔH={a} kJ/mol, ΔS={b} J/mol·K, T={c} K");
            println!("  Spontaneous: {}", if dg < 0.0 { "yes (ΔG < 0)" } else { "no (ΔG ≥ 0)" });
        }
        "henderson-hasselbalch" | "hh" => {
            // a = pKa, b = [base] (M), c = [acid] (M)
            let ph = henderson_hasselbalch(a, b, c);
            println!("Henderson-Hasselbalch: pH = {ph:.4}");
            println!("  pKa={a}, [base]={b} M, [acid]={c} M");
        }
        other => eprintln!("Unknown reaction '{other}'. Use: arrhenius | gibbs | henderson-hasselbalch"),
    }
}

pub fn run_chem_druglike(smiles: &str) {
    use qualia_core_db::domains::chemical::organic_chemistry::{
        parse_smiles, compute_descriptors,
        evaluate_lipinski, evaluate_veber, evaluate_ghose, evaluate_egan,
        predict_bbb_permeation, compute_logp, compute_tpsa,
    };
    let mol  = parse_smiles(smiles);
    let desc = compute_descriptors(&mol);
    let logp = compute_logp(&mol);
    let tpsa = compute_tpsa(&mol);
    let lip  = evaluate_lipinski(&desc);
    let veb  = evaluate_veber(&desc);
    let gho  = evaluate_ghose(&desc);
    let ega  = evaluate_egan(&desc);
    let bbb  = predict_bbb_permeation(desc.molecular_weight, logp, tpsa, desc.hb_donors);
    println!("Drug-likeness for SMILES: {smiles}");
    println!("  Lipinski : {} (violations: {})", if lip.passes { "PASS" } else { "FAIL" }, lip.violations);
    println!("  Veber    : {}", if veb.passes { "PASS" } else { "FAIL" });
    println!("  Ghose    : {}", if gho.passes { "PASS" } else { "FAIL" });
    println!("  Egan     : {}", if ega.passes { "PASS" } else { "FAIL" });
    println!("  BBB      : {} (MPO score: {})", if bbb.is_cns_penetrant { "CNS-penetrant" } else { "non-penetrant" }, bbb.clark_score);
}

pub fn run_chem_pka(pka: f64, conc_base: f64, conc_acid: f64) {
    use qualia_core_db::domains::chemical::organic_chemistry::{
        henderson_hasselbalch, ionisation_fraction,
    };
    let ph  = henderson_hasselbalch(pka, conc_base, conc_acid);
    let frac = ionisation_fraction(ph, pka);
    println!("Henderson-Hasselbalch:");
    println!("  pKa={pka}, [A⁻]={conc_base} M, [HA]={conc_acid} M");
    println!("  pH              = {ph:.4}");
    println!("  Ionised fraction= {frac:.4}");
}

// ── Biology ───────────────────────────────────────────────────────────────────

pub fn run_bio_align(query: &str, target: &str, mode: &str) {
    use qualia_core_db::domains::biological::bioinformatics::{
        align_nucleotide, align_protein,
    };
    let q = query.as_bytes();
    let t = target.as_bytes();
    let result = match mode.to_ascii_lowercase().as_str() {
        "protein" | "aa" => align_protein(q, t),
        _                => align_nucleotide(q, t),
    };
    println!("Sequence alignment ({mode}):");
    println!("  Query  : {query}");
    println!("  Target : {target}");
    println!("  Score  : {}", result.score);
    println!("  Aligned: {}", result.aligned_query.len());
    println!("  Gaps   : {}", result.num_gaps);
    println!("  Matches: {}", result.num_matches);
}

pub fn run_bio_kmer(sequence: &str, k: usize) {
    use qualia_core_db::domains::biological::bioinformatics::kmer_frequencies;
    let freqs = kmer_frequencies(sequence.as_bytes(), k);
    println!("k-mer frequencies (k={k}) for sequence len={}:", sequence.len());
    println!("  Distinct k-mers: {}", freqs.len());
    for (hash, count) in freqs.iter().take(8) {
        println!("  0x{hash:016x} → {count}");
    }
    if freqs.len() > 8 { println!("  … ({} total)", freqs.len()); }
}

pub fn run_bio_translate(dna: &str) {
    use qualia_core_db::domains::biological::bioinformatics::translate_dna_to_protein;
    let dna_bytes = dna.as_bytes();
    let mut protein_buf = vec![0u8; dna_bytes.len() / 3 + 1];
    let n = translate_dna_to_protein(dna_bytes, &mut protein_buf);
    let protein = std::str::from_utf8(&protein_buf[..n]).unwrap_or("(invalid UTF-8)");
    println!("DNA → protein translation:");
    println!("  DNA    : {}", &dna[..dna.len().min(60)]);
    println!("  Protein: {protein}  ({n} aa)");
}

pub fn run_bio_isoelectric(protein: &str) {
    use qualia_core_db::domains::biological::bioinformatics::calculate_isoelectric_point;
    let pi = calculate_isoelectric_point(protein.as_bytes());
    println!("Isoelectric point for protein '{}':", &protein[..protein.len().min(20)]);
    println!("  pI = {pi:.4}");
}

pub fn run_bio_jaccard(sketch_a: &str, sketch_b: &str) {
    use qualia_core_db::domains::biological::bioinformatics::jaccard_similarity;
    let parse_hashes = |s: &str| -> Vec<u64> {
        s.split(',').filter_map(|t| {
            let t = t.trim();
            if t.starts_with("0x") || t.starts_with("0X") {
                u64::from_str_radix(&t[2..], 16).ok()
            } else {
                t.parse::<u64>().ok()
            }
        }).collect()
    };
    let a = parse_hashes(sketch_a);
    let b = parse_hashes(sketch_b);
    let j = jaccard_similarity(&a, &b);
    println!("Jaccard similarity:");
    println!("  |sketch A| = {}, |sketch B| = {}", a.len(), b.len());
    println!("  J(A,B)     = {j:.6}");
}

pub fn run_bio_minhash(sequence: &str, k: usize, sketch_size: usize) {
    use qualia_core_db::domains::biological::bioinformatics::minhash_sketch;
    let sketch = minhash_sketch(sequence.as_bytes(), k, sketch_size);
    println!("MinHash sketch (k={k}, size={sketch_size}):");
    println!("  Sequence len: {}", sequence.len());
    for h in sketch.iter().take(5) {
        println!("  0x{h:016x}");
    }
    if sketch.len() > 5 { println!("  … ({} total)", sketch.len()); }
}

// ── Geospatial ────────────────────────────────────────────────────────────────

pub fn run_geo_embed_h3(index: u64) {
    use qualia_core_db::domains::geospatial::spatial::embed_h3_context;
    let context = embed_h3_context(index);
    println!("H3 index embedding:");
    println!("  Input  : 0x{index:016x}");
    println!("  Context: 0x{context:016x}");
}

// ── Thermodynamics ────────────────────────────────────────────────────────────

pub fn run_thermo_gibbs(enthalpy: f64, entropy: f64, temp: f64) {
    use qualia_core_db::domains::physical::thermodynamics::ThermodynamicSampler;
    let sampler = ThermodynamicSampler::new(temp, 1);
    let dg = sampler.calculate_gibbs_free_energy(enthalpy, entropy);
    println!("Gibbs free energy:");
    println!("  H={enthalpy:.4} J, S={entropy:.4} J/K, T={temp:.1} K");
    println!("  ΔG = H - TS = {dg:.6} J");
    println!("  Spontaneous: {}", if dg < 0.0 { "yes" } else { "no" });
}

pub fn run_thermo_anneal(initial_temp: f64, particles: usize, proposed_energy: f64, random: f64) {
    use qualia_core_db::domains::physical::thermodynamics::ThermodynamicSampler;
    let mut sampler = ThermodynamicSampler::new(initial_temp, particles);
    let accepted = sampler.metropolis_step(proposed_energy, random);
    println!("Metropolis-Hastings step:");
    println!("  T_init={initial_temp} K,  particles={particles}");
    println!("  Proposed ΔE = {proposed_energy:.6}");
    println!("  Uniform u   = {random:.6}");
    println!("  Accepted    : {accepted}");
}

// ── Geometric algebra ─────────────────────────────────────────────────────────

fn parse_vec3(s: &str) -> Option<[f32; 3]> {
    let v: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
    if v.len() >= 3 { Some([v[0], v[1], v[2]]) } else {
        eprintln!("Need 3 comma-separated values for a vector, got {}.", v.len()); None
    }
}

pub fn run_geometric_cross(a_str: &str, b_str: &str) {
    use qualia_core_db::geometric_algebra::utils::{cross_product, dot_product};
    let (Some(a), Some(b)) = (parse_vec3(a_str), parse_vec3(b_str)) else { return; };
    let cross = cross_product(&a, &b);
    let dot   = dot_product(&a, &b);
    println!("Geometric algebra:");
    println!("  a = [{:.4}, {:.4}, {:.4}]", a[0], a[1], a[2]);
    println!("  b = [{:.4}, {:.4}, {:.4}]", b[0], b[1], b[2]);
    println!("  a×b = [{:.6}, {:.6}, {:.6}]", cross[0], cross[1], cross[2]);
    println!("  a·b = {dot:.6}");
}

pub fn run_geometric_angle(a_str: &str, b_str: &str) {
    use qualia_core_db::geometric_algebra::utils::{angle_between_vectors, rad_to_deg};
    let (Some(a), Some(b)) = (parse_vec3(a_str), parse_vec3(b_str)) else { return; };
    let angle_rad = angle_between_vectors(&a, &b);
    let angle_deg = rad_to_deg(angle_rad);
    println!("Angle between vectors:");
    println!("  a = [{:.4}, {:.4}, {:.4}]", a[0], a[1], a[2]);
    println!("  b = [{:.4}, {:.4}, {:.4}]", b[0], b[1], b[2]);
    println!("  θ = {angle_rad:.6} rad  ({angle_deg:.4}°)");
}

// ── Clinical ──────────────────────────────────────────────────────────────────

pub fn run_clinical_framingham(
    age: u8, sex_male: bool,
    total_chol: f64, hdl_chol: f64,
    systolic_bp: f64, bp_treated: bool,
    smoker: bool, diabetic: bool,
) {
    use qualia_core_db::clinical_engine::{FraminghamInput, framingham_10yr_risk};
    let input = FraminghamInput {
        age, sex_male,
        total_cholesterol_mmol: total_chol,
        hdl_cholesterol_mmol: hdl_chol,
        systolic_bp,
        bp_treated,
        current_smoker: smoker,
        diabetic,
    };
    let r = framingham_10yr_risk(&input);
    println!("Framingham 10-year CVD risk:");
    println!("  Age={age}, sex={}, TC={total_chol:.2} mmol/L, HDL={hdl_chol:.2}, SBP={systolic_bp:.0}",
             if sex_male { "M" } else { "F" });
    println!("  Treated={bp_treated}, Smoker={smoker}, Diabetic={diabetic}");
    println!("  Risk     : {:.1}%", r.risk_10yr * 100.0);
    println!("  Category : {:?}", r.category);
    println!("  Log score: {:.4}", r.log_score);
}

pub fn run_clinical_sofa(
    pao2_fio2: f64, platelets: f64, bilirubin: f64,
    map: f64, gcs: u8, creatinine: f64,
) {
    use qualia_core_db::clinical_engine::{SofaInput, sofa_score};
    let input = SofaInput {
        pao2_fio2_ratio: pao2_fio2,
        platelets_10_9_l: platelets,
        bilirubin_mg_dl: bilirubin,
        map_mmhg: map,
        dopamine_dose: 0.0,
        epinephrine_dose: 0.0,
        norepinephrine_dose: 0.0,
        glasgow_coma_scale: gcs,
        creatinine_mg_dl: creatinine,
        urine_output_ml_d: 500.0,
    };
    let score = sofa_score(&input);
    println!("SOFA score (Sequential Organ Failure Assessment):");
    println!("  PaO₂/FiO₂={pao2_fio2}, Platelets={platelets}×10⁹/L");
    println!("  Bilirubin={bilirubin} mg/dL, MAP={map} mmHg, GCS={gcs}");
    println!("  Creatinine={creatinine} mg/dL");
    println!("  SOFA score: {score}/24");
    let mortality = match score {
        0..=1  => "<10%",
        2..=3  => "~10%",
        4..=5  => "~20%",
        6..=7  => "~20–30%",
        8..=9  => "~40%",
        10..=11 => "~40–50%",
        12..=14 => ">50%",
        _       => ">80%",
    };
    println!("  Est. mortality: {mortality}");
}

pub fn run_clinical_ckd(age: u8, sex_male: bool, weight_kg: f64, creatinine: f64) {
    use qualia_core_db::clinical_engine::{RenalInput, cockcroft_gault_crcl, ckd_epi_egfr};
    let input = RenalInput { age, sex_male, weight_kg, serum_creatinine: creatinine };
    let crcl = cockcroft_gault_crcl(&input);
    let egfr = ckd_epi_egfr(&input);
    println!("Renal function:");
    println!("  Age={age}, sex={}, weight={weight_kg} kg, Cr={creatinine} mg/dL",
             if sex_male { "M" } else { "F" });
    println!("  CrCl (Cockcroft-Gault) : {crcl:.1} mL/min");
    println!("  eGFR (CKD-EPI 2021)    : {egfr:.1} mL/min/1.73m²");
    let stage = match egfr as u32 {
        90..=u32::MAX => "G1 (normal)",
        60..=89       => "G2 (mildly decreased)",
        45..=59       => "G3a (mild-moderate)",
        30..=44       => "G3b (moderate-severe)",
        15..=29       => "G4 (severe)",
        _             => "G5 (kidney failure)",
    };
    println!("  CKD stage              : {stage}");
}

pub fn run_clinical_pk(dose_mg: f64, vd_l: f64, cl_l_hr: f64, time_hr: f64) {
    use qualia_core_db::clinical_engine::{PkOneCompartmentInput, one_compartment_pk_model};
    let input = PkOneCompartmentInput {
        dose_mg,
        volume_distribution_l: vd_l,
        clearance_l_hr: cl_l_hr,
        time_hr,
    };
    let r = one_compartment_pk_model(&input);
    println!("1-compartment PK model (IV bolus):");
    println!("  Dose={dose_mg} mg, Vd={vd_l} L, CL={cl_l_hr} L/hr");
    println!("  C(t={time_hr}h) = {:.4} mg/L", r.concentration);
    println!("  Half-life      = {:.4} hr", r.half_life_hr);
}

pub fn run_clinical_drug_interactions(drug_names: &str) {
    use qualia_core_db::clinical_engine::check_drug_interactions;
    use qualia_core_db::mini_parser::hash_token;
    let hashes: Vec<u64> = drug_names.split(',').map(|s| hash_token(s.trim())).collect();
    let interactions = check_drug_interactions(&hashes);
    println!("Drug interaction screening for: {drug_names}");
    if interactions.is_empty() {
        println!("  No known interactions found.");
    } else {
        for ix in &interactions {
            println!("  [{:?}] 0x{:x} ↔ 0x{:x}: {}", ix.severity, ix.drug_a, ix.drug_b, ix.mechanism);
        }
    }
}

// ── Economics ─────────────────────────────────────────────────────────────────

pub fn run_economics_gbm(price: f64, drift: f64, vol: f64, horizon: f64, steps: usize) {
    use qualia_core_db::domains::financial::economics::simulate_gbm_path;
    let final_price = simulate_gbm_path(price, drift, vol, horizon, steps);
    println!("Geometric Brownian Motion path:");
    println!("  S₀={price}, μ={drift}, σ={vol}, T={horizon}, steps={steps}");
    println!("  S(T) ≈ {final_price:.4}");
    let expected = price * (drift * horizon).exp();
    println!("  E[S(T)] = {expected:.4}  (drift-only estimate)");
}

pub fn run_economics_var(price: f64, drift: f64, vol: f64, horizon: f64, steps: usize, paths: usize) {
    use qualia_core_db::domains::financial::economics::run_monte_carlo_var;
    println!("Monte Carlo VaR (paths={paths}, steps={steps})…");
    let (mean, var95) = run_monte_carlo_var(price, drift, vol, horizon, steps, paths);
    println!("  S₀={price}, μ={drift}, σ={vol}, T={horizon}");
    println!("  Mean end value : {mean:.4}");
    println!("  95% VaR (loss) : {var95:.4}");
}

pub fn run_economics_macro(m0: f64, p0: f64, velocity: f64, real_gdp: f64, horizon: f64, steps: usize) {
    use qualia_core_db::domains::financial::economics::simulate_macroeconomic_flow;
    let state = simulate_macroeconomic_flow(m0, p0, velocity, real_gdp, horizon, steps);
    println!("Macroeconomic flow (M×V = P×Q):");
    println!("  M₀={m0}, P₀={p0}, V={velocity}, Q={real_gdp}, T={horizon}");
    if state.values.len() >= 2 {
        println!("  M(T) = {:.4}", state.values[0]);
        println!("  P(T) = {:.4}", state.values[1]);
        println!("  Implied Q = {:.4}", (state.values[0] * velocity) / state.values[1]);
    }
}
