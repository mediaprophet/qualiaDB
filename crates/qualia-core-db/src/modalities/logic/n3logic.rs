use crate::q_hash;
use crate::NQuin;

#[derive(Debug)]
pub enum N3LogicError {
    ParseError,
    HardwareUnavailable(&'static str),
}

/// N3Logic is the foundational rule-based inference engine.
/// It reads the raw triples/rules submitted in the Agent Intent
/// and dynamically infers which specialized logic modalities must be invoked.
pub fn infer_logic_bindings(quins: &[NQuin]) -> Result<Vec<&'static str>, N3LogicError> {
    let mut bindings = Vec::new();

    // We strictly enforce Epistemic Logic as the Sandbox Isolator
    bindings.push("modality:epistemic");

    // We strictly enforce Deontic Logic for human-rights alignment
    bindings.push("modality:deontic");

    let lock_predicate = q_hash("OP_INTENT_LOCK");
    let action_payload = q_hash("q42:actionPayload");
    let cost_predicate = q_hash("q42:burnsTokens");
    let structural_refactor = q_hash("q42:structuralRefactor");

    // Hardware acceleration and solver routing triggers
    let uses_pga = q_hash("q42:usesPGA");
    let quantum_opt = q_hash("q42:quantumOptimize");
    let solve_calculus = q_hash("q42:solveCalculus");
    let check_paraconsistent = q_hash("q42:paraconsistentCheck");
    let spatial_region = q_hash("q42:spatialRegion");

    // Specialized Libraries routing triggers
    let chem_model = q_hash("q42:chemistryModeling");
    let phys_sim = q_hash("q42:physicsSimulation");
    let med_comp = q_hash("q42:medicalComputing");
    let crypto_lib = q_hash("q42:cryptography");

    for q in quins {
        let opcode = (q.predicate & 0xFF) as u8;

        // If the agent submits an Intent Lock or modifies the active payload, enforce Temporal Leases
        if opcode == 0x23 || q.predicate == lock_predicate || q.predicate == action_payload {
            if !bindings.contains(&"modality:temporal") {
                bindings.push("modality:temporal");
            }
        }

        // If the agent requests a structural refactor, lock the namespace
        if q.predicate == structural_refactor {
            if !bindings.contains(&"modality:epistemic:namespace_lock") {
                bindings.push("modality:epistemic:namespace_lock");
            }
        }

        // If the agent burns compute tokens, enforce Defeasible Logic for cost-overrun exceptions
        if q.predicate == cost_predicate {
            if !bindings.contains(&"modality:defeasible") {
                bindings.push("modality:defeasible");
            }
        }

        // Geometric Algebra / PGA routing
        if q.predicate == uses_pga {
            if !bindings.contains(&"modality:geometric_algebra") {
                bindings.push("modality:geometric_algebra");
            }
        }

        // Hardware gating: the QPU is strictly assumed offline at this stage.
        if q.predicate == quantum_opt {
            return Err(N3LogicError::HardwareUnavailable(
                "QPU offline. Intent requested q42:quantumOptimize.",
            ));
        }
        if q.predicate == solve_calculus {
            if !bindings.contains(&"modality:solver:calculus") {
                bindings.push("modality:solver:calculus");
            }
        }

        // Advanced Modalities
        if q.predicate == check_paraconsistent {
            if !bindings.contains(&"modality:paraconsistent") {
                bindings.push("modality:paraconsistent");
            }
        }
        if q.predicate == spatial_region {
            if !bindings.contains(&"modality:spatio_temporal") {
                bindings.push("modality:spatio_temporal");
            }
        }

        // Specialized Libraries
        if q.predicate == chem_model {
            if !bindings.contains(&"modality:specialized:chemistry") {
                bindings.push("modality:specialized:chemistry");
            }
        }
        if q.predicate == phys_sim {
            if !bindings.contains(&"modality:specialized:physics") {
                bindings.push("modality:specialized:physics");
            }
        }
        if q.predicate == med_comp {
            if !bindings.contains(&"modality:specialized:medical") {
                bindings.push("modality:specialized:medical");
            }
        }
        if q.predicate == crypto_lib {
            if !bindings.contains(&"modality:specialized:crypto") {
                bindings.push("modality:specialized:crypto");
            }
        }
    }

    Ok(bindings)
}
