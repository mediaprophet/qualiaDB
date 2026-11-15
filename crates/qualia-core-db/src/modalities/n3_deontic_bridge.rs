//! N3 Defeasible/Defeater & AgreementDID -> Deontic/Epistemic Quin Compiler (Task G & C2).
//!
//! Fully zero-allocation (Tier-1 Zero-Heap). Bridges N3 rules and ratified AgreementDID
//! structure Quins into parity-valid deontic and epistemic NQuins (<450 lines).

use crate::governance::webizen::agreement::AgreementDID;
use crate::modalities::epistemic::{CERTAINTY_BIT_SHIFT, OP_BELIEVES};
use crate::modalities::logic::deontic::{
    evaluate_deontic_contract, DeonticError, DeonticVerdict, DEFEATER_BIT,
    OP_FORBID, OP_OBLIGATE, OP_PERMIT,
};
#[cfg(test)]
use crate::governance::webizen::agreement::AgreementState;
#[cfg(test)]
use crate::modalities::epistemic::{EpistemicStatus, EpistemicVerdict, OP_KNOWS};
#[cfg(test)]
use crate::modalities::logic::deontic::DeonticStatus;
use crate::modalities::logic::n3_parser::{Rule, RuleType, StackRule, Term};
use crate::q_hash;
use crate::NQuin;

pub const HAS_GUARDIAN: u64 = q_hash("q42:hasGuardian");
pub const HAS_DOMAIN_SCOPE: u64 = q_hash("q42:hasDomainScope");
pub const REQUIRES_CONSENSUS: u64 = q_hash("q42:requiresConsensus");
pub const BILATERAL_MICROCOMMONS_LANE: u64 = 0x4000_0000_0000_0002;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeError {
    BufferOverflow,
    UnratifiedConsensusThreshold,
    ExpiredAgreement,
    MalformedRule,
    MissingStructureQuin,
    DeonticEvaluationError,
    EpistemicEvaluationError,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct N3RuleDescriptor {
    pub rule_type: RuleType,
    pub party_did_hash: u64,
    pub property_path_hash: u64,
    pub action_object_hash: u64,
    pub contract_hash: u64,
    pub weight: Option<f32>,
    pub expiry_unix32: u32,
}

#[inline(always)]
fn hash_term(term: &Term<'_>) -> u64 {
    match term {
        Term::Uri(s) | Term::Variable(s) | Term::Literal(s) | Term::Formula(s) => q_hash(s),
    }
}

impl N3RuleDescriptor {
    pub fn from_ast_rule(rule: &Rule<'_>, default_contract_hash: u64, default_expiry: u32) -> Result<Self, BridgeError> {
        if rule.premise.triples.is_empty() || rule.conclusion.triples.is_empty() { return Err(BridgeError::MalformedRule); }
        let (p, c) = (&rule.premise.triples[0], &rule.conclusion.triples[0]);
        let contract_hash = if default_contract_hash != 0 { default_contract_hash } else { hash_term(&c.subject) };
        Ok(Self {
            rule_type: rule.rule_type,
            party_did_hash: hash_term(&p.subject),
            property_path_hash: hash_term(&p.predicate),
            action_object_hash: hash_term(&p.object),
            contract_hash,
            weight: rule.weight,
            expiry_unix32: default_expiry,
        })
    }

    pub fn from_stack_rule(rule: &StackRule<'_>, default_contract_hash: u64, default_expiry: u32) -> Result<Self, BridgeError> {
        if rule.premise.len == 0 || rule.conclusion.len == 0 { return Err(BridgeError::MalformedRule); }
        let (p, c) = (&rule.premise.triples[0], &rule.conclusion.triples[0]);
        let contract_hash = if default_contract_hash != 0 { default_contract_hash } else { hash_term(&c.subject) };
        Ok(Self {
            rule_type: rule.rule_type,
            party_did_hash: hash_term(&p.subject),
            property_path_hash: hash_term(&p.predicate),
            action_object_hash: hash_term(&p.object),
            contract_hash,
            weight: rule.weight,
            expiry_unix32: default_expiry,
        })
    }
}

#[inline]
pub fn parse_deontic_modality(p_hash: u64, rule_type: RuleType) -> (u8, bool) {
    if rule_type == RuleType::Defeater { return (OP_PERMIT, true); }
    let is_forbid = [q_hash("q42:forbid"), q_hash("q42:not"), q_hash("q42:prohibit")].contains(&p_hash);
    let is_permit = [q_hash("q42:permit"), q_hash("q42:may"), q_hash("q42:can")].contains(&p_hash);
    let opcode = if is_forbid { OP_FORBID } else if is_permit { OP_PERMIT } else { OP_OBLIGATE };
    (opcode, false)
}

pub fn compile_descriptor_to_deontic_norm(desc: &N3RuleDescriptor) -> NQuin {
    let (opcode, is_defeater) = parse_deontic_modality(desc.property_path_hash, desc.rule_type);
    let mut pred = ((desc.property_path_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater || desc.rule_type == RuleType::Defeater { pred |= DEFEATER_BIT; }
    let parity = desc.party_did_hash ^ pred ^ desc.action_object_hash ^ desc.contract_hash;
    NQuin {
        subject: desc.party_did_hash,
        predicate: pred,
        object: desc.action_object_hash,
        context: desc.contract_hash,
        metadata: desc.expiry_unix32 as u64,
        parity,
    }
}

pub fn compile_descriptor_to_epistemic_quin(desc: &N3RuleDescriptor, opcode: u8) -> NQuin {
    let certainty = match desc.weight {
        Some(w) => (w.clamp(0.0, 1.0) * 255.0).round() as u8,
        None => if opcode == OP_BELIEVES { 200 } else { 255 },
    };
    let pred = (opcode as u64) | ((certainty as u64) << CERTAINTY_BIT_SHIFT);
    let parity = desc.party_did_hash ^ pred ^ desc.action_object_hash ^ desc.contract_hash;
    NQuin {
        subject: desc.party_did_hash,
        predicate: pred,
        object: desc.action_object_hash,
        context: desc.contract_hash,
        metadata: (desc.expiry_unix32 as u64) | (certainty as u64),
        parity,
    }
}

pub fn compile_agreement_structure_quins_to_deontic_norms(
    structure_quins: &[NQuin],
    collected_signatures: u8,
    current_epoch: u32,
    expiry_epoch: u32,
    out: &mut [NQuin],
) -> Result<usize, BridgeError> {
    if expiry_epoch != 0 && current_epoch > expiry_epoch { return Err(BridgeError::ExpiredAgreement); }
    let (mut agreement_id, mut domain_id, mut threshold) = (0u64, 0u64, 0u8);
    let mut agents = [0u64; 8];
    let (mut num_agents, mut has_consensus) = (0usize, false);

    for q in structure_quins {
        if q.predicate == HAS_GUARDIAN {
            agreement_id = q.context;
            if q.object != 0 && num_agents < 8 && !agents[..num_agents].contains(&q.object) {
                agents[num_agents] = q.object;
                num_agents += 1;
            }
        } else if q.predicate == HAS_DOMAIN_SCOPE {
            if agreement_id == 0 { agreement_id = q.subject; }
            domain_id = q.object;
        } else if q.predicate == REQUIRES_CONSENSUS {
            if agreement_id == 0 { agreement_id = q.subject; }
            if domain_id == 0 { domain_id = q.context; }
            threshold = q.object as u8;
            has_consensus = true;
        }
    }
    if !has_consensus || threshold == 0 { return Err(BridgeError::MissingStructureQuin); }
    if collected_signatures < threshold { return Err(BridgeError::UnratifiedConsensusThreshold); }
    if out.len() < num_agents { return Err(BridgeError::BufferOverflow); }

    for i in 0..num_agents {
        let agent = agents[i];
        let predicate = ((HAS_DOMAIN_SCOPE & 0x007F_FFFF_FFFF_FFFF) << 8) | (OP_OBLIGATE as u64);
        let parity = agent ^ predicate ^ domain_id ^ agreement_id;
        out[i] = NQuin {
            subject: agent,
            predicate,
            object: domain_id,
            context: agreement_id,
            metadata: (expiry_epoch as u64) | BILATERAL_MICROCOMMONS_LANE,
            parity,
        };
    }
    Ok(num_agents)
}

pub fn compile_agreement_did_to_deontic_norms(
    agreement: &AgreementDID,
    collected_signatures: u8,
    current_epoch: u32,
    expiry_epoch: u32,
    out: &mut [NQuin],
) -> Result<usize, BridgeError> {
    if expiry_epoch != 0 && current_epoch > expiry_epoch { return Err(BridgeError::ExpiredAgreement); }
    if collected_signatures < agreement.threshold { return Err(BridgeError::UnratifiedConsensusThreshold); }
    let n = agreement.num_agents as usize;
    if out.len() < n { return Err(BridgeError::BufferOverflow); }
    for i in 0..n {
        let agent = agreement.agents[i];
        let predicate = ((HAS_DOMAIN_SCOPE & 0x007F_FFFF_FFFF_FFFF) << 8) | (OP_OBLIGATE as u64);
        let parity = agent ^ predicate ^ agreement.domain_id ^ agreement.agreement_id;
        out[i] = NQuin {
            subject: agent,
            predicate,
            object: agreement.domain_id,
            context: agreement.agreement_id,
            metadata: (expiry_epoch as u64) | BILATERAL_MICROCOMMONS_LANE,
            parity,
        };
    }
    Ok(n)
}

#[inline]
pub fn evaluate_deontic_frame(quins: &[NQuin], now_unix: u32, out: &mut [DeonticVerdict]) -> Result<usize, DeonticError> {
    evaluate_deontic_contract(quins, now_unix, out)
}

pub fn compile_and_evaluate_n3_deontic_norms(
    descriptors: &[N3RuleDescriptor],
    agreement_structure_quins: &[NQuin],
    collected_signatures: u8,
    current_epoch: u32,
    expiry_epoch: u32,
    quin_buf: &mut [NQuin],
    out_verdicts: &mut [DeonticVerdict],
) -> Result<usize, BridgeError> {
    if quin_buf.len() < descriptors.len() { return Err(BridgeError::BufferOverflow); }
    let mut written = 0usize;
    for d in descriptors {
        quin_buf[written] = compile_descriptor_to_deontic_norm(d);
        written += 1;
    }
    if !agreement_structure_quins.is_empty() {
        let n_agr = compile_agreement_structure_quins_to_deontic_norms(
            agreement_structure_quins,
            collected_signatures,
            current_epoch,
            expiry_epoch,
            &mut quin_buf[written..],
        )?;
        written += n_agr;
    }
    evaluate_deontic_frame(&quin_buf[..written], current_epoch, out_verdicts)
        .map_err(|_| BridgeError::DeonticEvaluationError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::epistemic::evaluate_epistemic_frame;
    use crate::modalities::logic::deontic::DefeatKind;
    use crate::modalities::logic::n3_parser::{Formula, StackFormula, Triple};

    const PARTY_ALICE: u64 = 0xA11C_E000_0000_0001;
    const PARTY_BOB: u64 = 0xB0B0_0000_0000_0002;
    const PRED_DISCLOSE: u64 = q_hash("q42:disclose");
    const OBJ_PROJECT_X: u64 = q_hash("q42:projectX");
    const OBJ_AUDITOR: u64 = q_hash("q42:auditor");
    const CONTRACT_NDA: u64 = 0xCDA0_0000_0000_0001;

    fn sample_agreement(state: AgreementState, threshold: u8, num_agents: u8) -> AgreementDID {
        AgreementDID {
            agreement_id: 0x8888,
            principal: 0x1111,
            agents: [PARTY_ALICE, PARTY_BOB, 0, 0, 0, 0, 0, 0],
            num_agents,
            domain_id: 0x9999,
            threshold,
            current_state: state,
        }
    }

    #[test]
    fn test_defeasible_obligation_active_and_defeated_by_defeater() {
        let defeasible_rule = N3RuleDescriptor {
            rule_type: RuleType::Defeasible,
            party_did_hash: PARTY_ALICE,
            property_path_hash: PRED_DISCLOSE,
            action_object_hash: OBJ_PROJECT_X,
            contract_hash: CONTRACT_NDA,
            weight: None,
            expiry_unix32: 5000,
        };
        let mut quins = [NQuin::default(); 8];
        let mut verdicts = [DeonticVerdict::default(); 8];

        let count = compile_and_evaluate_n3_deontic_norms(
            &[defeasible_rule], &[], 0, 1000, 5000, &mut quins, &mut verdicts,
        ).expect("eval alone");
        assert_eq!(count, 1);
        assert_eq!(verdicts[0].status, DeonticStatus::Active);
        assert_eq!(verdicts[0].opcode, OP_OBLIGATE);
        assert_eq!(verdicts[0].defeat_kind, DefeatKind::None);

        let defeater_rule = N3RuleDescriptor {
            rule_type: RuleType::Defeater,
            party_did_hash: PARTY_ALICE,
            property_path_hash: PRED_DISCLOSE,
            action_object_hash: OBJ_AUDITOR,
            contract_hash: CONTRACT_NDA,
            weight: None,
            expiry_unix32: 5000,
        };
        let count2 = compile_and_evaluate_n3_deontic_norms(
            &[defeasible_rule, defeater_rule], &[], 0, 1000, 5000, &mut quins, &mut verdicts,
        ).expect("eval with defeater");
        assert_eq!(count2, 1);
        assert_eq!(verdicts[0].status, DeonticStatus::Defeated);
        assert_eq!(verdicts[0].defeat_kind, DefeatKind::Rebutting);
    }

    #[test]
    fn test_ratified_agreement_compiles_to_deontic_obligations() {
        let agreement = sample_agreement(AgreementState::Ratified, 2, 2);
        let structure_quins = agreement.compile_to_super_quins();
        let mut quins = [NQuin::default(); 16];
        let mut verdicts = [DeonticVerdict::default(); 8];

        let count = compile_and_evaluate_n3_deontic_norms(
            &[], &structure_quins, 2, 1000, 5000, &mut quins, &mut verdicts,
        ).expect("compile agreement norms");
        assert_eq!(count, 2);
        assert_eq!(verdicts[0].status, DeonticStatus::Active);
        assert_eq!(verdicts[0].opcode, OP_OBLIGATE);
        assert_eq!(verdicts[0].norm.subject, PARTY_ALICE);
        assert_eq!(verdicts[1].status, DeonticStatus::Active);
        assert_eq!(verdicts[1].opcode, OP_OBLIGATE);
        assert_eq!(verdicts[1].norm.subject, PARTY_BOB);
    }

    #[test]
    fn test_negative_unratified_consensus_fails_closed() {
        let agreement = sample_agreement(AgreementState::Ratified, 2, 2);
        let structure_quins = agreement.compile_to_super_quins();
        let mut out = [NQuin::default(); 4];
        let err = compile_agreement_structure_quins_to_deontic_norms(
            &structure_quins, 1, 1000, 5000, &mut out,
        ).unwrap_err();
        assert_eq!(err, BridgeError::UnratifiedConsensusThreshold);
        let err_did = compile_agreement_did_to_deontic_norms(&agreement, 1, 1000, 5000, &mut out).unwrap_err();
        assert_eq!(err_did, BridgeError::UnratifiedConsensusThreshold);
    }

    #[test]
    fn test_negative_expired_epoch_fails_closed() {
        let agreement = sample_agreement(AgreementState::Ratified, 1, 1);
        let structure_quins = agreement.compile_to_super_quins();
        let mut out = [NQuin::default(); 4];
        let err = compile_agreement_structure_quins_to_deontic_norms(
            &structure_quins, 1, 6000, 5000, &mut out,
        ).unwrap_err();
        assert_eq!(err, BridgeError::ExpiredAgreement);
    }

    #[test]
    fn test_negative_buffer_overflow_fails_closed_without_partial_write() {
        let agreement = sample_agreement(AgreementState::Ratified, 1, 2);
        let structure_quins = agreement.compile_to_super_quins();
        let mut out = [NQuin { subject: 0xDEAD_BEEF, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0 }; 1];
        let err = compile_agreement_structure_quins_to_deontic_norms(
            &structure_quins, 1, 1000, 5000, &mut out,
        ).unwrap_err();
        assert_eq!(err, BridgeError::BufferOverflow);
        assert_eq!(out[0].subject, 0xDEAD_BEEF);
    }

    #[test]
    fn test_epistemic_compilation_and_evaluation() {
        let known_rule = N3RuleDescriptor {
            rule_type: RuleType::Strict,
            party_did_hash: PARTY_ALICE,
            property_path_hash: q_hash("q42:knows"),
            action_object_hash: OBJ_PROJECT_X,
            contract_hash: CONTRACT_NDA,
            weight: Some(0.95),
            expiry_unix32: 5000,
        };
        let uncertain_rule = N3RuleDescriptor {
            rule_type: RuleType::Defeasible,
            party_did_hash: PARTY_BOB,
            property_path_hash: q_hash("q42:believes"),
            action_object_hash: OBJ_PROJECT_X,
            contract_hash: CONTRACT_NDA,
            weight: Some(0.20),
            expiry_unix32: 5000,
        };
        let q1 = compile_descriptor_to_epistemic_quin(&known_rule, OP_KNOWS);
        let q2 = compile_descriptor_to_epistemic_quin(&uncertain_rule, OP_BELIEVES);
        let mut verdicts = [EpistemicVerdict { claim: NQuin::default(), status: EpistemicStatus::Skipped, certainty: 0 }; 4];
        let count = evaluate_epistemic_frame(&[q1, q2], 0, 0, &mut verdicts).expect("epistemic eval");
        assert_eq!(count, 2);
        assert_eq!(verdicts[0].status, EpistemicStatus::Active);
        assert_eq!(verdicts[0].certainty, 242);
        assert_eq!(verdicts[1].status, EpistemicStatus::Uncertain);
        assert_eq!(verdicts[1].certainty, 51);
    }

    #[test]
    fn test_ast_and_stack_rule_conversion() {
        let mut s_premise = StackFormula::new();
        s_premise.triples[0] = Triple {
            subject: Term::Uri("did:web:alice"),
            predicate: Term::Uri("q42:mustDeliver"),
            object: Term::Uri("item:package"),
        };
        s_premise.len = 1;
        let mut s_conclusion = StackFormula::new();
        s_conclusion.triples[0] = Triple {
            subject: Term::Uri("urn:contract:shipping"),
            predicate: Term::Uri("q42:scope"),
            object: Term::Uri("item:package"),
        };
        s_conclusion.len = 1;
        let stack_rule = StackRule {
            id: Some("sr-1"),
            rule_type: RuleType::Strict,
            weight: Some(1.0),
            premise: s_premise,
            conclusion: s_conclusion,
        };

        let desc = N3RuleDescriptor::from_stack_rule(&stack_rule, 0, 1000).expect("from stack rule");
        assert_eq!(desc.party_did_hash, q_hash("did:web:alice"));
        assert_eq!(desc.property_path_hash, q_hash("q42:mustDeliver"));
        assert_eq!(desc.action_object_hash, q_hash("item:package"));
        assert_eq!(desc.contract_hash, q_hash("urn:contract:shipping"));

        let quin = compile_descriptor_to_deontic_norm(&desc);
        assert_eq!(quin.subject, q_hash("did:web:alice"));
        assert_eq!(quin.predicate & 0xFF, OP_OBLIGATE as u64);
        assert_eq!(quin.predicate & DEFEATER_BIT, 0);

        let empty_ast = Rule {
            id: None,
            rule_type: RuleType::Strict,
            weight: None,
            premise: Formula { triples: vec![] },
            conclusion: Formula { triples: vec![] },
        };
        assert_eq!(N3RuleDescriptor::from_ast_rule(&empty_ast, 0, 0), Err(BridgeError::MalformedRule));
    }
}
