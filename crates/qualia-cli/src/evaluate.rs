use std::fs::File;
use std::path::Path;
use qualia_core_db::NQuin;
use qualia_core_db::deontic_logic::{VcAttributes, evaluate_accessible_layers};
use qualia_core_db::modalities::epistemic::{evaluate_epistemic_frame, EpistemicVerdict, EpistemicStatus};
use qualia_core_db::modalities::paraconsistent::route_paraconsistent;

pub fn map_quins(path: &Path) -> std::io::Result<(memmap2::Mmap, usize)> {
    let file = File::open(path)?;
    let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
    let len = mmap.len();
    Ok((mmap, len))
}

const EMPTY_QUIN: NQuin = NQuin {
    subject: 0, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0,
};

/// Try to cast a memory-mapped file to a slice of NQuins.
/// Prints a diagnostic and returns an empty slice on failure.
fn cast_quins(mmap: &memmap2::Mmap) -> &[NQuin] {
    match bytemuck::try_cast_slice(mmap.as_ref()) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("cast_slice failed ({e:?}) — file may have a volume header; try stripping with `qualia-cli inspect`");
            &[]
        }
    }
}

// ── Existing modalities ───────────────────────────────────────────────────────

pub fn run_deontic(dataset: &Path, contract_hash: u64) {
    let (mmap, len) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);
    println!("Mapped {} quins ({len} bytes).", quins.len());

    let vc = VcAttributes::from_quins(contract_hash, quins);
    let storage_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let vault = match qualia_core_db::key_vault::KeyVault::load_or_generate(&storage_dir) {
        Ok(v) => v,
        Err(e) => { eprintln!("KeyVault error: {e}"); return; }
    };
    let permitted_layers = evaluate_accessible_layers(&vault, &vc, &[]);
    println!("Deontic evaluation for contract 0x{contract_hash:016x}:");
    if permitted_layers.is_empty() {
        println!("  No permitted layers found.");
    }
    for (layer, _key) in permitted_layers {
        println!("  Permitted layer: {layer:?}");
    }
}

pub fn run_epistemic(dataset: &Path, agent_hash: u64) {
    let (mmap, len) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);
    println!("Mapped {} quins ({len} bytes).", quins.len());

    let empty = EpistemicVerdict { claim: EMPTY_QUIN, status: EpistemicStatus::Uncertain, certainty: 0 };
    let mut out = vec![empty; 128];
    match evaluate_epistemic_frame(quins, agent_hash, 0, &mut out) {
        Ok(n) => println!("Epistemic: {n} verdicts for agent 0x{agent_hash:016x}."),
        Err(e) => eprintln!("Epistemic error: {e:?}"),
    }
}

pub fn run_paraconsistent(dataset: &Path) {
    let (mmap, len) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);
    println!("Mapped {} quins ({len} bytes).", quins.len());

    let mut out_ok  = vec![EMPTY_QUIN; quins.len().max(1)];
    let mut out_iso = vec![EMPTY_QUIN; quins.len().max(1)];
    match route_paraconsistent(quins, &mut out_ok, &mut out_iso) {
        Ok((ok, iso)) => {
            println!("Paraconsistent routing:");
            println!("  Consistent   : {ok}");
            println!("  Isolated (⊥) : {iso}");
        }
        Err(e) => eprintln!("Paraconsistent error: {e:?}"),
    }
}

// ── New modalities ────────────────────────────────────────────────────────────

pub fn run_ltl(dataset: &Path, formula_type: &str, hash_a: u64, hash_b: u64) {
    use qualia_core_db::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};

    let formula = match formula_type.to_ascii_lowercase().as_str() {
        "globally" | "g" => LtlFormula::Globally(hash_a),
        "next"     | "x" => LtlFormula::Next(hash_a),
        "until"    | "u" => LtlFormula::Until { ante: hash_a, consequent: hash_b },
        other => {
            eprintln!("Unknown LTL operator '{other}'. Use: globally, next, until");
            return;
        }
    };

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let trace = cast_quins(&mmap);

    let result = evaluate_ltl_trace(trace, &formula);
    println!("LTL trace evaluation ({formula_type} 0x{hash_a:x}): {}", if result { "HOLDS" } else { "VIOLATED" });
}

pub fn run_asp(dataset: &Path, base_index: usize) {
    use qualia_core_db::modalities::asp::{enumerate_stable_models, MAX_STABLE_MODELS};

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);

    if quins.is_empty() {
        eprintln!("Empty dataset."); return;
    }
    let base = quins.get(base_index).copied().unwrap_or(EMPTY_QUIN);
    let mut out = [0u64; MAX_STABLE_MODELS];
    let count = enumerate_stable_models(&base, quins, &mut out);
    println!("ASP: {count} stable model(s) from base quin at index {base_index}:");
    for h in &out[..count] {
        println!("  0x{h:016x}");
    }
}

pub fn run_dl(dataset: &Path, sub_class: u64, super_class: u64) {
    use qualia_core_db::modalities::dl::check_subsumption_quin;

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let tbox = cast_quins(&mmap);

    let result = check_subsumption_quin(sub_class, super_class, tbox);
    println!(
        "DL subsumption: 0x{sub_class:x} ⊑ 0x{super_class:x}  →  {}",
        if result { "TRUE (subclass confirmed)" } else { "FALSE (not subsumed)" }
    );
}

pub fn run_probabilistic(weight: f32, threshold: f32) {
    use qualia_core_db::modalities::probabilistic::evaluate_threshold;
    let result = evaluate_threshold(weight, threshold);
    println!(
        "Probabilistic: weight={weight:.4}, threshold={threshold:.4}  →  {}",
        if result { "ABOVE THRESHOLD" } else { "BELOW THRESHOLD" }
    );
}

pub fn run_linear_logic(dataset: &Path, quin_index: usize) {
    use qualia_core_db::modalities::linear::{consume_quin, is_consumed};

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);

    match quins.get(quin_index) {
        None => eprintln!("Quin index {quin_index} out of range (dataset has {} quins).", quins.len()),
        Some(q) => {
            let consumed_before = is_consumed(q);
            let mut mutable_q = *q;
            consume_quin(&mut mutable_q);
            println!("Linear logic consume on quin[{quin_index}]:");
            println!("  Before consume: {}", if consumed_before { "already consumed" } else { "available" });
            println!("  After  consume: {}", if is_consumed(&mutable_q) { "consumed" } else { "still available (check bit encoding)" });
        }
    }
}

pub fn run_dialectical(dataset: &Path, var1: u64, var2: u64) {
    use qualia_core_db::modalities::dialectical::{are_confounded, do_intervention};

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let graph = cast_quins(&mmap);

    let confounded = are_confounded(graph, var1, var2);
    println!("Dialectical analysis:");
    println!("  var1=0x{var1:x}, var2=0x{var2:x}");
    println!("  Confounded: {confounded}");

    if let Some(effect) = do_intervention(graph, var1, 1, var2) {
        println!("  Intervention (set var1=1) → effect on var2: {effect:.4}");
    } else {
        println!("  Intervention: insufficient data for causal effect estimate.");
    }
}

pub fn run_diffusion(graph_id: &str) {
    use qualia_core_db::modalities::diffusion::trigger_diffusion;
    let triggered = trigger_diffusion(graph_id);
    println!("Diffusion trigger for graph '{}': {}", graph_id, if triggered { "initiated" } else { "already active or no-op" });
}

pub fn run_spatio_temporal(
    action: &str,
    ax1: f64, ay1: f64, ax2: f64, ay2: f64,
    bx1: f64, by1: f64, bx2: f64, by2: f64,
) {
    use qualia_core_db::modalities::spatio_temporal::{
        evaluate_rcc8, evaluate_temporal, SpatialRegion, TemporalOp,
    };

    match action.to_ascii_lowercase().as_str() {
        "rcc8" => {
            let region_a = SpatialRegion::new(1, vec![(ax1,ay1),(ax2,ay1),(ax2,ay2),(ax1,ay2)]);
            let region_b = SpatialRegion::new(2, vec![(bx1,by1),(bx2,by1),(bx2,by2),(bx1,by2)]);
            let relation = evaluate_rcc8(&region_a, &region_b);
            println!("RCC8 relation A({ax1},{ay1})-({ax2},{ay2}) vs B({bx1},{by1})-({bx2},{by2}): {relation:?}");
        }
        "temporal-before" => {
            let result = evaluate_temporal(TemporalOp::Before, ax1 as i64, ay1 as i64, bx1 as i64, by1 as i64);
            println!("Temporal BEFORE [{ax1}..{ay1}] vs [{bx1}..{by1}]: {result}");
        }
        "temporal-meets" => {
            let result = evaluate_temporal(TemporalOp::Meets, ax1 as i64, ay1 as i64, bx1 as i64, by1 as i64);
            println!("Temporal MEETS [{ax1}..{ay1}] vs [{bx1}..{by1}]: {result}");
        }
        "temporal-overlaps" => {
            let result = evaluate_temporal(TemporalOp::Overlaps, ax1 as i64, ay1 as i64, bx1 as i64, by1 as i64);
            println!("Temporal OVERLAPS [{ax1}..{ay1}] vs [{bx1}..{by1}]: {result}");
        }
        "temporal-during" => {
            let result = evaluate_temporal(TemporalOp::During, ax1 as i64, ay1 as i64, bx1 as i64, by1 as i64);
            println!("Temporal DURING [{ax1}..{ay1}] vs [{bx1}..{by1}]: {result}");
        }
        other => {
            eprintln!("Unknown spatio-temporal action '{other}'. Use: rcc8, temporal-before, temporal-meets, temporal-overlaps, temporal-during");
        }
    }
}

pub fn run_interval(action: &str, start1: i64, end1: i64, start2: i64, end2: i64, point: i64) {
    use qualia_core_db::modalities::interval_reasoning::TemporalInterval;

    let a = TemporalInterval::new(1, start1, end1);
    let b = TemporalInterval::new(2, start2, end2);

    match action.to_ascii_lowercase().as_str() {
        "contains" => println!("Interval [{start1}..{end1}].contains({point}): {}", a.contains(point)),
        "overlaps" => println!("Interval [{start1}..{end1}] overlaps [{start2}..{end2}]: {}", a.overlaps(&b)),
        "intersection" => {
            match a.intersection(&b) {
                Some(i) => println!("Intersection: [{}..{}]", i.start, i.end),
                None    => println!("Intervals do not intersect."),
            }
        }
        "union" => {
            let u = a.union(&b);
            println!("Union: [{}..{}]", u.start, u.end);
        }
        "gap" => {
            match a.gap(&b) {
                Some(g) => println!("Gap between intervals: {g} time units"),
                None    => println!("Intervals overlap — no gap."),
            }
        }
        other => {
            eprintln!("Unknown interval action '{other}'. Use: contains, overlaps, intersection, union, gap");
        }
    }
}

pub fn run_graph_topology(dataset: &Path, context: u64) {
    use qualia_core_db::modalities::graph_theory::analyze_graph_topology;

    let (mmap, _) = match map_quins(dataset) {
        Ok(r) => r,
        Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
    };
    let quins = cast_quins(&mmap);

    let result = analyze_graph_topology(quins, context);
    println!("Graph topology analysis (context=0x{context:x}):");
    println!("  Nodes        : {}", result.node_count);
    println!("  Edges        : {}", result.edge_count);
    println!("  Density      : {:.4}", result.density);
    println!("  Communities  : {}", result.communities.len());
    println!("  Motifs found : {}", result.motifs.len());
    if !result.top_nodes.is_empty() {
        println!("  Top nodes by centrality:");
        for (node_id, score) in result.top_nodes.iter().take(5) {
            println!("    0x{node_id:016x}  centrality={score:.4}");
        }
    }
}

pub fn run_argumentation(demo: bool, dataset: Option<&Path>) {
    use qualia_core_db::modalities::argumentation::{ArgumentationFramework, create_sanctuary_debate};

    let framework: ArgumentationFramework = if demo || dataset.is_none() {
        create_sanctuary_debate()
    } else {
        // Build a minimal framework from the dataset's quins:
        // each quin.subject attacks quin.object if predicate matches "attacks" hash
        let (mmap, _) = match dataset.map(|p| map_quins(p)).unwrap() {
            Ok(r) => r,
            Err(e) => { eprintln!("Cannot open dataset: {e}"); return; }
        };
        let quins = cast_quins(&mmap);
        let attack_pred = qualia_core_db::q_hash("attacks");
        let mut fw = ArgumentationFramework::new();
        for q in quins {
            if q.predicate == attack_pred {
                fw.add_attack(qualia_core_db::modalities::argumentation::Attack {
                    attacker:    q.subject,
                    target:      q.object,
                    attack_type: qualia_core_db::modalities::argumentation::AttackType::Rebuttal,
                    strength:    1.0,
                });
            }
        }
        fw
    };

    let grounded = framework.grounded_extension();
    println!("Argumentation framework analysis:");
    println!("  Arguments    : {}", framework.arguments.len());
    println!("  Attacks      : {}", framework.attacks.len());
    println!("  Grounded ext : {} argument(s)", grounded.len());
    if grounded.is_empty() {
        println!("  (empty grounded extension — all arguments attacked)");
    }
    for id in &grounded {
        if let Some(arg) = framework.arguments.get(id) {
            println!("    0x{id:016x}  \"{}\"", arg.content);
        } else {
            println!("    0x{id:016x}");
        }
    }
}

pub fn run_control_feedback(kp: f64, ki: f64, kd: f64, setpoint: f64, measurement: f64) {
    use qualia_core_db::modalities::control_feedback::{FeedbackController, PidParameters};

    let params = PidParameters { kp, ki, kd, output_min: f64::NEG_INFINITY, output_max: f64::INFINITY };
    let mut ctrl = FeedbackController::new("cli".to_string(), setpoint, measurement, params);
    let output = ctrl.compute_output();
    println!("PID controller:");
    println!("  Kp={kp}, Ki={ki}, Kd={kd}");
    println!("  Setpoint={setpoint}, Measurement={measurement}");
    println!("  Control output: {output:.6}");
}

pub fn run_neuro_symbolic() {
    use qualia_core_db::neuro_symbolic_sieve::{SieveLexSpec, SieveState};
    use qualia_core_db::q_hash;

    // Demonstrate the grammar sieve spec — shows which NQuin hashes constrain token generation
    let spec = SieveLexSpec::fever_observation();

    println!("Neuro-symbolic grammar sieve — fever observation demo");
    println!("  The sieve constrains LLM token generation to well-typed NQuins.");
    println!("  Token masks are built from the '.q42.lex' lexicon at runtime.");
    println!();
    println!("  State machine: ExpectSubject → ExpectPredicate → ExpectObject → Complete");
    println!();
    println!("  Allowed hashes:");
    for i in 0..spec.subjects_len as usize {
        println!("    Subject[{i}]   : 0x{:016x}  (q_hash(\"Patient\") = 0x{:016x})",
                 spec.subjects[i], q_hash("Patient"));
    }
    for i in 0..spec.predicates_len as usize {
        println!("    Predicate[{i}] : 0x{:016x}  (q_hash(\"fever\") = 0x{:016x})",
                 spec.predicates[i], q_hash("fever"));
    }
    for i in 0..spec.objects_len as usize {
        println!("    Object[{i}]    : 0x{:016x}  (q_hash(\"True\") = 0x{:016x})",
                 spec.objects[i], q_hash("True"));
    }
    println!();
    // Full clinical spec
    let full = SieveLexSpec::graph_mutation_default();
    println!("  graph_mutation_default spec ({} subjects, {} predicates, {} objects):",
             full.subjects_len, full.predicates_len, full.objects_len);
    for i in 0..full.subjects_len as usize {
        println!("    Subject[{i}]   : 0x{:016x}", full.subjects[i]);
    }
    for i in 0..full.predicates_len as usize {
        println!("    Predicate[{i}] : 0x{:016x}", full.predicates[i]);
    }
    for i in 0..full.objects_len as usize {
        println!("    Object[{i}]    : 0x{:016x}", full.objects[i]);
    }
    println!();
    println!("  At inference time, NeuroSymbolicSieve::from_lex_and_tokenizer() maps each");
    println!("  hash to allowed token IDs from the GGUF vocabulary.  Any token not in the");
    println!("  current mask has its logit set to -∞, enforcing SHACL-typed output.");
}
