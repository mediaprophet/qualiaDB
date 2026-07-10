pub mod abductive;
pub mod argumentation;
pub use argumentation::{
    Argument, ArgumentationFramework, Attack, AttackType, ARGUMENT_BIT, ATTACK_BIT, DEFENSE_BIT,
};
pub mod asp;
pub mod ctl;
pub mod fuzzy;
pub mod fuzzy_quantifiers;
pub mod fuzzy_rdf_schema;
pub mod fuzzy_type2;
pub mod modal;
pub use asp::{compute_answer_sets, enumerate_stable_models, AspRule, MAX_STABLE_MODELS};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod calculus;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use calculus::{
    detect_simd_width, pack_f32_pair, resolve_aligned_byte_offset, unpack_f32_pair, AlignmentError,
    CalculusError, ContinuousGrid, SimdWidth, OP_ADAPTIVE_STEP, OP_GPU_INTEGRATION, OP_RK4_STEP,
    OP_SIMPSONS_INTEGRATION, OP_TRAPEZOIDAL_INTEGRATION,
};
pub mod control_feedback;
pub use control_feedback::{ControlState, CONTROL_BIT, FEEDBACK_BIT, STABILIZATION_BIT};
pub mod dialectical;
pub use dialectical::{do_intervention, COUNTERFACTUAL_BIT, DO_INTERVENTION_BIT, SYNTHESIZED_BIT};
pub mod defeasible;
pub use defeasible::{
    evaluate_defeasible_frame, DefeasibleError, DefeasibleStatus, DefeasibleVerdict, DEFEATER_BIT,
    OP_DEFEASIBLE_OVERRIDE,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod diffusion;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use diffusion::{execute_diffusion_pass, trigger_diffusion};
pub mod dl;
pub use dl::check_subsumption_quin;
pub mod epistemic;
pub use epistemic::{
    evaluate_epistemic_frame, EpistemicError, EpistemicStatus, EpistemicVerdict,
    CERTAINTY_BIT_SHIFT, NESTING_BIT_SHIFT, OP_BELIEVES, OP_COMMON_KNOWLEDGE, OP_KNOWS,
};
pub mod epistemic_boundaries;
pub use epistemic_boundaries::{
    degrade_claim_to_socratic, detect_referral_by_severity, detect_referral_trigger,
    forbids_definitive_classification, identify_degradation_vector,
    requires_physiological_quarantine, DegradationVector, ReferralDomain, ReferralTrigger,
    SocraticDegradation, BIO_DISCLAIMER, EMERGENCY_PROMPT, LEGAL_DISCLAIMER, LEGAL_JEOPARDY_PROMPT,
};
pub mod graph_theory;
pub use graph_theory::{
    BoundedGraphAnalysisSummary, CommunitySpan, GraphAnalysisError, MotifRecord, TopNodeScore,
    MAX_BOUNDED_GRAPH_ANALYSIS_NODES, MAX_HEAP_GRAPH_ANALYSIS_QUINS,
};
pub mod interval_reasoning;
pub use interval_reasoning::TemporalInterval;
pub mod jural;
pub use jural::{
    compile_jural_quin, correlative, correlative_quin, find_unmet_correlatives,
    jural_correlativity_holds, jural_opposite, personhood_category_error, JURAL_CLAIM,
    JURAL_DISABILITY, JURAL_DUTY, JURAL_IMMUNITY, JURAL_LIABILITY, JURAL_NO_RIGHT, JURAL_POWER,
    JURAL_PRIVILEGE,
};
pub mod likeliness;
pub use likeliness::Likeliness;
pub mod linear;
pub use linear::{consume_quin, is_consumed, CONSUMED_BIT};
pub mod logic;
pub mod paraconsistent;
pub use paraconsistent::{
    route_paraconsistent, ContradictionStatus, ParaconsistentError, ISOLATED_CONTEXT_PREFIX,
    OP_CONTRADICTION_SCORE, OP_ISOLATE, OP_PARACONSISTENT_MERGE,
};
pub mod probabilistic;
pub use probabilistic::{evaluate_threshold, BayesianNetwork, BayesianNode, MAX_BAYESIAN_NODES};
pub mod spatio_temporal;
pub use spatio_temporal::{Rcc8Relation, SpatialRegion, TemporalOp};
pub mod stit;
pub use stit::{
    agentive_status, brought_about, is_duty_bearer, joint_discharged, joint_liable_members,
};
pub mod deontic_compose;
pub use deontic_compose::{
    agent_knows, classify_mens_rea, discharge_obligation, obligation_applies_in,
    obligation_globally, obligation_until, MensRea,
};
pub mod meta_deontic;
pub use meta_deontic::{
    breach_predicate, breach_provenance, build_breach_record, endorsement_credential,
    record_breach_to_wal,
};
pub mod interaction_governance;
pub use interaction_governance::{
    govern_verdict, map_policy, permits_execution, policy_action, Governance, PolicyMode,
};
pub mod causal;
pub use causal::{but_for_cause, caused, dependents_voided, is_overdetermined, is_voided_by};
pub mod responsibility;
pub use responsibility::{
    accountability_vacuum, adjudicate, enforcer_overreach, is_enforceable_fact,
    rule_of_law_asymmetry, ResponsibilityStatus,
};
pub mod capacity;
pub use capacity::{
    effective_principal, posthumous_standing, stipulation_binding, stipulation_voidable,
    CapacityStatus,
};
pub mod delegation;
pub use delegation::{authority_after_revocation, has_delegated_authority, revoked_descendants};
pub mod contract;
pub use contract::{
    formation_stage, incorporates_by_reference, is_binding_contract, FormationStage,
};
pub mod value_flow;
pub use value_flow::{commons_cost, is_commons_discharged, outstanding, pool_after, royalty};
pub mod capability_gap;
pub use capability_gap::{capability_gap, requirements_met};
pub mod identity_fabric;
pub use identity_fabric::{
    identifier_is_not_identity, identity_survives_loss, recompute_fabric, surviving_anchors,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod legal_compose;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use legal_compose::{
    selective_disclosure, translation_status, zk_eligibility, Eligibility, MatchStatus,
};
// §26 proportionality composes the native-only CAS — gated to native (see legal_compose.rs).
#[cfg(not(target_arch = "wasm32"))]
pub use legal_compose::{marginal_harm, proportionality_met};
pub mod consensus;
pub use consensus::{
    can_form_joint_during_partition, is_globally_valid, survives_partition, transaction_status,
    TxStatus,
};
pub mod manifold_logic;
pub use manifold_logic::{continuous_to_fact, integrate_abs, wave_eval, WaveCoord};
pub mod carrier;
pub use carrier::{extract_payload, media_tag, verify_binding};
pub mod temporal_ltl;
pub use temporal_ltl::{
    evaluate_ltl_trace, LtlFormula, OP_LTL_FINALLY, OP_LTL_GLOBALLY, OP_LTL_NEXT, OP_LTL_RELEASE,
    OP_LTL_UNTIL,
};
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod manifold;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub use manifold::project_10d_to_quaternion;
