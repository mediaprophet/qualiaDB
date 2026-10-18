//! Deontic logic engine — ODRL-based policy evaluation for credential-gated subgraph access.
//!
//! The policy hierarchy maps agent Verifiable Credentials to `SubgraphLayer` access rights.
//! Policy evaluation is:
//!   1. Extract `VcAttributes` from the agent's NQuin slice (credential claims).
//!   2. Evaluate against the `SubgraphPolicy` for the requested layer.
//!   3. If the policy passes, call `KeyVault::generate_layer_key()` and return it.
//!
//! The `deontic_logic` module is intentionally free of heap allocation in evaluation paths
//! — all structures fit in fixed-size arrays.

#[cfg(not(target_arch = "wasm32"))]
use crate::key_vault::{KeyVault, SubgraphKey, SubgraphLayer};
use crate::q_hash;

// ── VC role IRI hashes ────────────────────────────────────────────────────────

/// Compile-time hashes of canonical ODRL/Qualia role IRIs used in VC claims.
pub mod vc_roles {
    use crate::q_hash;

    /// Any authenticated principal — no further VC attributes required.
    pub const AUTHENTICATED: u64 = q_hash("urn:qualia:role:authenticated");
    /// Professional context (e.g., organisation employee with NDA).
    pub const PROFESSIONAL: u64 = q_hash("urn:qualia:role:professional");
    /// Legal practitioner with privileged access to legal subgraph.
    pub const LEGAL_PRACTITIONER: u64 = q_hash("urn:qualia:role:legal-practitioner");
    /// Registered medical professional.
    pub const MEDICAL_PROFESSIONAL: u64 = q_hash("urn:qualia:role:medical-professional");
    /// Fiduciary duty holder (financial advisor, trustee, etc.).
    pub const FIDUCIARY: u64 = q_hash("urn:qualia:role:fiduciary");
    /// Qualia node operator — administrative role.
    pub const NODE_OPERATOR: u64 = q_hash("urn:qualia:role:node-operator");
}

/// Predicate hash for VC role claims (`urn:qualia:vc:hasRole`).
const P_HAS_ROLE: u64 = q_hash("urn:qualia:vc:hasRole");
/// Predicate hash for VC clearance level (`urn:qualia:vc:clearanceLevel`).
const P_CLEARANCE_LEVEL: u64 = q_hash("urn:qualia:vc:clearanceLevel");
/// Predicate hash for VC issuer (`urn:qualia:vc:issuedBy`).
const P_ISSUED_BY: u64 = q_hash("urn:qualia:vc:issuedBy");

/// Maximum number of roles stored in `VcAttributes`.
const MAX_ROLES: usize = 8;

/// Parsed Verifiable Credential attributes for a single agent.
///
/// Extracted from the agent's NQuin slice via `VcAttributes::from_quins()`.
#[derive(Debug, Clone, Copy)]
pub struct VcAttributes {
    pub did_hash: u64,
    pub roles: [u64; MAX_ROLES],
    pub role_count: u8,
    /// Numeric clearance level (0=public … 4=fiduciary). Sourced from `vc:clearanceLevel`.
    pub clearance_level: u8,
    /// Hash of the VC issuer's DID — must be in the trusted-issuer set.
    pub credential_issuer: u64,
}

impl VcAttributes {
    /// Create a minimal `VcAttributes` for `did_hash` with no claims.
    pub fn unauthenticated(did_hash: u64) -> Self {
        Self {
            did_hash,
            roles: [0; MAX_ROLES],
            role_count: 0,
            clearance_level: 0,
            credential_issuer: 0,
        }
    }

    /// Parse VC claims for `agent_did` from a slice of NQuins.
    ///
    /// Scans for quins where `subject == agent_did` and predicate is one of
    /// `P_HAS_ROLE`, `P_CLEARANCE_LEVEL`, or `P_ISSUED_BY`.
    pub fn from_quins(agent_did: u64, quins: &[crate::NQuin]) -> Self {
        let mut attrs = Self::unauthenticated(agent_did);
        for q in quins {
            if q.subject != agent_did {
                continue;
            }
            if q.predicate == P_HAS_ROLE && (attrs.role_count as usize) < MAX_ROLES {
                attrs.roles[attrs.role_count as usize] = q.object;
                attrs.role_count += 1;
            } else if q.predicate == P_CLEARANCE_LEVEL {
                attrs.clearance_level = (q.object & 0x0F) as u8;
            } else if q.predicate == P_ISSUED_BY {
                attrs.credential_issuer = q.object;
            }
        }
        attrs
    }

    /// Returns `true` if this VC includes `role_hash` in its role claims.
    #[inline]
    pub fn has_role(self, role_hash: u64) -> bool {
        self.roles[..self.role_count as usize].contains(&role_hash)
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// The result of a deontic policy evaluation for subgraph key release.
#[derive(Debug)]
pub enum DeonticResult {
    /// Policy passed — the derived `SubgraphKey` can be released to the agent.
    KeyRelease(SubgraphKey),
    /// Policy denied. The key is not returned.
    AccessDenied {
        layer: SubgraphLayer,
        reason: &'static str,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl DeonticResult {
    /// `true` when permitted.
    pub fn is_permitted(&self) -> bool {
        matches!(self, Self::KeyRelease(_))
    }
}

/// Evaluate an agent's VCs against the ODRL policy for `layer` and, if permitted,
/// return the derived `SubgraphKey`.
///
/// # Policy table
///
/// | Layer        | Minimum clearance | Accepted roles                         |
/// |-------------|-------------------|-----------------------------------------|
/// | Public      | 0 (any)           | (none required)                         |
/// | Professional| 1                 | `role:professional`, `role:node-operator` |
/// | Legal       | 2                 | `role:legal-practitioner`               |
/// | Medical     | 3                 | `role:medical-professional`             |
/// | Fiduciary   | 4                 | `role:fiduciary`                        |
///
/// Clearance level OR role match is sufficient; both are not required.
///
/// # Trusted issuers
/// The `trusted_issuers` slice lists DID hashes of credential issuers the node
/// accepts.  An empty slice disables issuer-trust enforcement (dev mode only).
#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_vc_for_subgraph_key_release(
    vault: &KeyVault,
    vc: &VcAttributes,
    layer: SubgraphLayer,
    trusted_issuers: &[u64],
) -> DeonticResult {
    // 1. Issuer trust check (skip if no trusted issuers configured).
    if !trusted_issuers.is_empty() && !trusted_issuers.contains(&vc.credential_issuer) {
        return DeonticResult::AccessDenied {
            layer,
            reason: "credential issuer not in trusted-issuer set",
        };
    }

    // 2. Layer-specific policy.
    let policy_passed = match layer {
        SubgraphLayer::Public => true,

        SubgraphLayer::Professional => {
            vc.clearance_level >= 1
                || vc.has_role(vc_roles::PROFESSIONAL)
                || vc.has_role(vc_roles::NODE_OPERATOR)
        }

        SubgraphLayer::Legal => {
            vc.clearance_level >= 2 || vc.has_role(vc_roles::LEGAL_PRACTITIONER)
        }

        SubgraphLayer::Medical => {
            vc.clearance_level >= 3 || vc.has_role(vc_roles::MEDICAL_PROFESSIONAL)
        }

        SubgraphLayer::Fiduciary => vc.clearance_level >= 4 || vc.has_role(vc_roles::FIDUCIARY),
    };

    if policy_passed {
        DeonticResult::KeyRelease(vault.generate_layer_key(layer))
    } else {
        DeonticResult::AccessDenied {
            layer,
            reason: "insufficient VC clearance or role",
        }
    }
}

/// Convenience: evaluate which layers the agent can access and write the permitted
/// keys into `out`, in ascending layer order.
///
/// Zero-heap: writes at most `out.len().min(5)` `(SubgraphLayer, SubgraphKey)` entries
/// (one per layer) into the caller-supplied slice and returns the count written.
/// `SubgraphKey` is move-only (it zeroizes on drop), so `out` is a slice of `Option<…>`;
/// written entries are `Some`. This function is not on a hot path.
#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_accessible_layers(
    vault: &KeyVault,
    vc: &VcAttributes,
    trusted_issuers: &[u64],
    out: &mut [Option<(SubgraphLayer, SubgraphKey)>],
) -> usize {
    let layers = [
        SubgraphLayer::Public,
        SubgraphLayer::Professional,
        SubgraphLayer::Legal,
        SubgraphLayer::Medical,
        SubgraphLayer::Fiduciary,
    ];
    let mut n = 0usize;
    for layer in layers {
        if let DeonticResult::KeyRelease(key) =
            evaluate_vc_for_subgraph_key_release(vault, vc, layer, trusted_issuers)
        {
            if n >= out.len() {
                break;
            }
            out[n] = Some((layer, key));
            n += 1;
        }
    }
    n
}

/// Build the NQuins that record a VC credential claim for an agent, for insertion
/// into the daemon graph.
///
/// Each `role_hash` becomes a `P_HAS_ROLE` quin in `CONTEXT`.
/// `clearance` and `issuer_did` emit one quin each.
pub fn write_vc_claim_quins(
    agent_did: u64,
    roles: &[u64],
    clearance: u8,
    issuer_did: u64,
    ts: u64,
) -> [crate::NQuin; 3] {
    const VC_CONTEXT: u64 = q_hash("urn:qualia:context:vc");

    let role_hash = if roles.is_empty() {
        vc_roles::AUTHENTICATED
    } else {
        roles[0]
    };

    [
        crate::NQuin {
            subject: agent_did,
            predicate: P_HAS_ROLE,
            object: role_hash,
            context: VC_CONTEXT,
            metadata: ts & 0xFFFF_FFFF,
            parity: agent_did ^ P_HAS_ROLE ^ role_hash ^ VC_CONTEXT,
        },
        crate::NQuin {
            subject: agent_did,
            predicate: P_CLEARANCE_LEVEL,
            object: clearance as u64,
            context: VC_CONTEXT,
            metadata: ts & 0xFFFF_FFFF,
            parity: agent_did ^ P_CLEARANCE_LEVEL ^ (clearance as u64) ^ VC_CONTEXT,
        },
        crate::NQuin {
            subject: agent_did,
            predicate: P_ISSUED_BY,
            object: issuer_did,
            context: VC_CONTEXT,
            metadata: ts & 0xFFFF_FFFF,
            parity: agent_did ^ P_ISSUED_BY ^ issuer_did ^ VC_CONTEXT,
        },
    ]
}

use crate::modalities::logic::deontic::{DEFEATER_BIT, OP_FORBID, OP_OBLIGATE, OP_PERMIT};
use crate::modalities::logic::n3_compiler::CompiledRule;
use crate::modalities::logic::n3_parser::RuleType;

/// Compile an N3 rule into a norm Quin (or a defeater Quin if rule_type is Defeater).
///
/// Mapping:
///   premise.triples\[0\].subject  → party_did_hash  (who is bound)
///   premise.triples\[0\].predicate → property_path_hash  (what action/property)
///   premise.triples\[0\].object   → action_object_hash  (target entity)
///   rule.rule_type              → opcode + is_defeater flag
///   conclusion.triples\[0\].subject → contract context hash
///
/// Returns None if the rule does not have the expected triple structure.
pub fn compile_n3_rule_to_norm(
    rule: &CompiledRule,
    contract_hash: u64,
    expiry_unix32: u32,
) -> Option<crate::NQuin> {
    if rule.premise.len == 0 || rule.conclusion.len == 0 {
        return None;
    }

    let premise_triple = &rule.premise.triples[0];
    let conclusion_triple = &rule.conclusion.triples[0];

    let party_did_hash = premise_triple.subject.as_u64();
    let property_path_hash = premise_triple.predicate.as_u64();
    let action_object_hash = premise_triple.object.as_u64();

    let mapped_contract_hash = if contract_hash == 0 {
        conclusion_triple.subject.as_u64()
    } else {
        contract_hash
    };

    let mut opcode = OP_PERMIT;
    let mut is_defeater = false;

    let p_hash = property_path_hash;

    let is_obligate = [
        crate::q_hash("q42:obligate"),
        crate::q_hash("q42:must"),
        crate::q_hash("q42:shall"),
    ]
    .contains(&p_hash);
    let is_permit = [
        crate::q_hash("q42:permit"),
        crate::q_hash("q42:may"),
        crate::q_hash("q42:can"),
    ]
    .contains(&p_hash);
    let is_forbid = [
        crate::q_hash("q42:forbid"),
        crate::q_hash("q42:not"),
        crate::q_hash("q42:prohibit"),
    ]
    .contains(&p_hash);

    match rule.rule_type {
        RuleType::Strict => {
            if is_obligate {
                opcode = OP_OBLIGATE;
            }
        }
        RuleType::Defeasible => {
            if is_permit {
                opcode = OP_PERMIT;
            } else if is_forbid {
                opcode = OP_FORBID;
            }
        }
        RuleType::Defeater => {
            opcode = OP_PERMIT;
            is_defeater = true;
        }
        RuleType::Linear => {
            if is_obligate {
                opcode = OP_OBLIGATE;
            }
        }
    }

    let mut quin = crate::NQuin::default();
    quin.subject = party_did_hash;

    // Predicate: opcode in lower 8 bits, property hash shifted left 8 bits (masked to 55 bits), and DEFEATER_BIT if needed
    let mut predicate_packed =
        ((property_path_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater {
        predicate_packed |= DEFEATER_BIT;
    }
    quin.predicate = predicate_packed;

    quin.object = action_object_hash;
    quin.context = mapped_contract_hash;
    quin.metadata = expiry_unix32 as u64;
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;

    Some(quin)
}

use crate::governance::webizen::agreement::{AgreementDID, AgreementState};
use crate::modalities::logic::n3_parser::{
    Rule as N3Rule, StackRule as N3StackRule, Term as N3Term,
};

fn hash_n3_term(term: &N3Term<'_>) -> u64 {
    match term {
        N3Term::Uri(s) | N3Term::Variable(s) | N3Term::Literal(s) | N3Term::Formula(s) => {
            crate::q_hash(s)
        }
    }
}

/// Compile an uncompiled/borrowed N3 AST `Rule` directly into a norm Quin.
pub fn compile_n3_ast_rule_to_norm(
    rule: &N3Rule<'_>,
    contract_hash: u64,
    expiry_unix32: u32,
) -> Option<crate::NQuin> {
    if rule.premise.triples.is_empty() || rule.conclusion.triples.is_empty() {
        return None;
    }
    let p_triple = &rule.premise.triples[0];
    let c_triple = &rule.conclusion.triples[0];

    let party_did_hash = hash_n3_term(&p_triple.subject);
    let property_path_hash = hash_n3_term(&p_triple.predicate);
    let action_object_hash = hash_n3_term(&p_triple.object);
    let mapped_contract_hash = if contract_hash == 0 {
        hash_n3_term(&c_triple.subject)
    } else {
        contract_hash
    };

    let p_str = match p_triple.predicate {
        N3Term::Uri(s) | N3Term::Variable(s) | N3Term::Literal(s) | N3Term::Formula(s) => {
            s.to_ascii_lowercase()
        }
    };

    let is_obligate =
        p_str.contains("obligate") || p_str.contains("must") || p_str.contains("shall");
    let is_permit = p_str.contains("permit") || p_str.contains("may") || p_str.contains("can");
    let is_forbid = p_str.contains("forbid") || p_str.contains("not") || p_str.contains("prohibit");

    let mut opcode = OP_PERMIT;
    let mut is_defeater = false;

    match rule.rule_type {
        RuleType::Strict => {
            opcode = if is_obligate { OP_OBLIGATE } else { OP_PERMIT };
        }
        RuleType::Defeasible => {
            if is_forbid {
                opcode = OP_FORBID;
            } else if is_permit {
                opcode = OP_PERMIT;
            }
        }
        RuleType::Defeater => {
            opcode = OP_PERMIT;
            is_defeater = true;
        }
        RuleType::Linear => {
            opcode = if is_obligate { OP_OBLIGATE } else { OP_PERMIT };
        }
    }

    let mut predicate_packed =
        ((property_path_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater {
        predicate_packed |= DEFEATER_BIT;
    }

    let parity = party_did_hash ^ predicate_packed ^ action_object_hash ^ mapped_contract_hash;

    Some(crate::NQuin {
        subject: party_did_hash,
        predicate: predicate_packed,
        object: action_object_hash,
        context: mapped_contract_hash,
        metadata: expiry_unix32 as u64,
        parity,
    })
}

/// Compile a zero-allocation `StackRule` directly into a norm Quin.
pub fn compile_stack_rule_to_norm(
    rule: &N3StackRule<'_>,
    contract_hash: u64,
    expiry_unix32: u32,
) -> Option<crate::NQuin> {
    if rule.premise.len == 0 || rule.conclusion.len == 0 {
        return None;
    }
    let p_triple = &rule.premise.triples[0];
    let c_triple = &rule.conclusion.triples[0];

    let party_did_hash = hash_n3_term(&p_triple.subject);
    let property_path_hash = hash_n3_term(&p_triple.predicate);
    let action_object_hash = hash_n3_term(&p_triple.object);
    let mapped_contract_hash = if contract_hash == 0 {
        hash_n3_term(&c_triple.subject)
    } else {
        contract_hash
    };

    let p_str = match p_triple.predicate {
        N3Term::Uri(s) | N3Term::Variable(s) | N3Term::Literal(s) | N3Term::Formula(s) => {
            s.to_ascii_lowercase()
        }
    };

    let is_obligate =
        p_str.contains("obligate") || p_str.contains("must") || p_str.contains("shall");
    let is_permit = p_str.contains("permit") || p_str.contains("may") || p_str.contains("can");
    let is_forbid = p_str.contains("forbid") || p_str.contains("not") || p_str.contains("prohibit");

    let mut opcode = OP_PERMIT;
    let mut is_defeater = false;

    match rule.rule_type {
        RuleType::Strict => {
            opcode = if is_obligate { OP_OBLIGATE } else { OP_PERMIT };
        }
        RuleType::Defeasible => {
            if is_forbid {
                opcode = OP_FORBID;
            } else if is_permit {
                opcode = OP_PERMIT;
            }
        }
        RuleType::Defeater => {
            opcode = OP_PERMIT;
            is_defeater = true;
        }
        RuleType::Linear => {
            opcode = if is_obligate { OP_OBLIGATE } else { OP_PERMIT };
        }
    }

    let mut predicate_packed =
        ((property_path_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater {
        predicate_packed |= DEFEATER_BIT;
    }

    let parity = party_did_hash ^ predicate_packed ^ action_object_hash ^ mapped_contract_hash;

    Some(crate::NQuin {
        subject: party_did_hash,
        predicate: predicate_packed,
        object: action_object_hash,
        context: mapped_contract_hash,
        metadata: expiry_unix32 as u64,
        parity,
    })
}

/// Compile a ratified `AgreementDID` into authorized deontic norm Quins.
pub fn compile_agreement_to_deontic_norms(
    agreement: &AgreementDID,
    expiry_unix32: u32,
    out: &mut [crate::NQuin],
) -> usize {
    if agreement.current_state != AgreementState::Ratified || out.is_empty() {
        return 0;
    }
    let mut written = 0;
    let has_domain_scope = crate::q_hash("q42:hasDomainScope");

    for i in 0..agreement.num_agents as usize {
        if written >= out.len() {
            break;
        }
        let agent = agreement.agents[i];
        let predicate = ((has_domain_scope & 0x007F_FFFF_FFFF_FFFF) << 8) | (OP_PERMIT as u64);
        let object = agreement.domain_id;
        let context = agreement.agreement_id;
        let metadata = (expiry_unix32 as u64) | 0x4000_0000_0000_0002;
        let parity = agent ^ predicate ^ object ^ context;

        out[written] = crate::NQuin {
            subject: agent,
            predicate,
            object,
            context,
            metadata,
            parity,
        };
        written += 1;
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> KeyVault {
        let tmp = tempfile::tempdir().expect("tmpdir");
        KeyVault::load_or_generate(tmp.path().to_str().unwrap()).expect("vault")
    }

    const ISSUER: u64 = 0x1550_0000_0000_CAFE;
    const AGENT: u64 = 0xA6E4_7777_0000_0001;

    fn vc_with_role(role: u64, clearance: u8) -> VcAttributes {
        let mut vc = VcAttributes::unauthenticated(AGENT);
        vc.roles[0] = role;
        vc.role_count = 1;
        vc.clearance_level = clearance;
        vc.credential_issuer = ISSUER;
        vc
    }

    #[test]
    fn public_layer_always_accessible() {
        let vault = test_vault();
        let vc = VcAttributes::unauthenticated(AGENT);
        let result = evaluate_vc_for_subgraph_key_release(&vault, &vc, SubgraphLayer::Public, &[]);
        assert!(result.is_permitted());
    }

    #[test]
    fn professional_layer_requires_role_or_clearance() {
        let vault = test_vault();

        // No role, no clearance → deny.
        let no_vc = VcAttributes::unauthenticated(AGENT);
        assert!(!evaluate_vc_for_subgraph_key_release(
            &vault,
            &no_vc,
            SubgraphLayer::Professional,
            &[]
        )
        .is_permitted());

        // Professional role → permit.
        let pro = vc_with_role(vc_roles::PROFESSIONAL, 0);
        assert!(evaluate_vc_for_subgraph_key_release(
            &vault,
            &pro,
            SubgraphLayer::Professional,
            &[]
        )
        .is_permitted());

        // Clearance 1 → permit.
        let cleared = vc_with_role(0, 1);
        assert!(evaluate_vc_for_subgraph_key_release(
            &vault,
            &cleared,
            SubgraphLayer::Professional,
            &[]
        )
        .is_permitted());
    }

    #[test]
    fn medical_layer_requires_role_or_clearance_3() {
        let vault = test_vault();

        let low = vc_with_role(vc_roles::PROFESSIONAL, 0);
        assert!(
            !evaluate_vc_for_subgraph_key_release(&vault, &low, SubgraphLayer::Medical, &[])
                .is_permitted()
        );

        let med_role = vc_with_role(vc_roles::MEDICAL_PROFESSIONAL, 0);
        assert!(evaluate_vc_for_subgraph_key_release(
            &vault,
            &med_role,
            SubgraphLayer::Medical,
            &[]
        )
        .is_permitted());

        let cleared = vc_with_role(0, 3);
        assert!(evaluate_vc_for_subgraph_key_release(
            &vault,
            &cleared,
            SubgraphLayer::Medical,
            &[]
        )
        .is_permitted());
    }

    #[test]
    fn fiduciary_layer_strictest() {
        let vault = test_vault();

        let med = vc_with_role(vc_roles::MEDICAL_PROFESSIONAL, 3);
        assert!(
            !evaluate_vc_for_subgraph_key_release(&vault, &med, SubgraphLayer::Fiduciary, &[])
                .is_permitted()
        );

        let fid = vc_with_role(vc_roles::FIDUCIARY, 0);
        assert!(
            evaluate_vc_for_subgraph_key_release(&vault, &fid, SubgraphLayer::Fiduciary, &[])
                .is_permitted()
        );
    }

    #[test]
    fn trusted_issuer_check_blocks_unknown_issuer() {
        let vault = test_vault();
        let trusted = [ISSUER];

        let mut vc = vc_with_role(vc_roles::FIDUCIARY, 4);
        vc.credential_issuer = 0xBAD_CAFE; // wrong issuer
        assert!(!evaluate_vc_for_subgraph_key_release(
            &vault,
            &vc,
            SubgraphLayer::Fiduciary,
            &trusted
        )
        .is_permitted());

        vc.credential_issuer = ISSUER; // correct issuer
        assert!(evaluate_vc_for_subgraph_key_release(
            &vault,
            &vc,
            SubgraphLayer::Fiduciary,
            &trusted
        )
        .is_permitted());
    }

    #[test]
    fn evaluate_accessible_layers_returns_permitted_set() {
        let vault = test_vault();
        // clearance 2 → Public + Professional + Legal, not Medical or Fiduciary.
        let vc = vc_with_role(0, 2);
        let mut out: [Option<(SubgraphLayer, SubgraphKey)>; 5] = [None, None, None, None, None];
        let count = evaluate_accessible_layers(&vault, &vc, &[], &mut out);
        assert_eq!(count, 3);
        let layers = [
            out[0].as_ref().unwrap().0,
            out[1].as_ref().unwrap().0,
            out[2].as_ref().unwrap().0,
        ];
        assert_eq!(
            layers,
            [
                SubgraphLayer::Public,
                SubgraphLayer::Professional,
                SubgraphLayer::Legal
            ]
        );
    }

    #[test]
    fn vc_attributes_from_quins_extracts_role_and_clearance() {
        let quins = write_vc_claim_quins(AGENT, &[vc_roles::MEDICAL_PROFESSIONAL], 3, ISSUER, 1000);
        let attrs = VcAttributes::from_quins(AGENT, &quins);
        assert_eq!(attrs.role_count, 1);
        assert!(attrs.has_role(vc_roles::MEDICAL_PROFESSIONAL));
        assert_eq!(attrs.clearance_level, 3);
        assert_eq!(attrs.credential_issuer, ISSUER);
    }

    #[test]
    fn test_child_medical_data_egress_violation() {
        use crate::mini_parser::{OP_END, OP_EVAL_PERMIT};
        use crate::webizen_bytecode::{execute_program, GuardianshipContext, VmError};

        let child_did = crate::q_hash("did:q42:child");
        let medical_data = crate::q_hash("MedicalData");
        let share_data = crate::q_hash("q42:shareData");

        let mut intent_quin = crate::NQuin::default();
        intent_quin.subject = child_did;
        intent_quin.predicate = share_data;
        intent_quin.object = medical_data;

        let db = [intent_quin];
        let mut prog = [0u8; 1024];
        // Program just has OP_EVAL_PERMIT then OP_END
        prog[0] = OP_EVAL_PERMIT;
        prog[1] = OP_END;

        let mut out = [crate::NQuin::default(); 10];

        let context = GuardianshipContext {
            principal_did: child_did,
            guardian_did: None, // No active guardian signature
        };

        let result = execute_program(&prog, &db, &mut out, Some(&context));
        assert_eq!(result, Err(VmError::HaltViolation));
    }

    #[test]
    fn test_compile_n3_rule_to_norm() {
        use crate::modalities::logic::n3_parser::RuleType;

        use crate::modalities::logic::n3_compiler::{
            CompiledFormula, CompiledTerm, CompiledTriple,
        };
        let make_rule = |rt: RuleType, pred: &str| -> CompiledRule {
            let mut premise_triples = [CompiledTriple {
                subject: CompiledTerm::Uri(0),
                predicate: CompiledTerm::Uri(0),
                object: CompiledTerm::Uri(0),
            }; 8];
            premise_triples[0] = CompiledTriple {
                subject: CompiledTerm::Uri(crate::q_hash("did:party")),
                predicate: CompiledTerm::Uri(crate::q_hash(pred)),
                object: CompiledTerm::Uri(crate::q_hash("did:target")),
            };
            let mut conclusion_triples = [CompiledTriple {
                subject: CompiledTerm::Uri(0),
                predicate: CompiledTerm::Uri(0),
                object: CompiledTerm::Uri(0),
            }; 8];
            conclusion_triples[0] = CompiledTriple {
                subject: CompiledTerm::Uri(crate::q_hash("urn:contract")),
                predicate: CompiledTerm::Uri(crate::q_hash("q42:boundBy")),
                object: CompiledTerm::Uri(crate::q_hash("did:party")),
            };
            CompiledRule {
                id_hash: None,
                rule_type: rt,
                weight: None,
                premise: CompiledFormula {
                    triples: premise_triples,
                    len: 1,
                },
                conclusion: CompiledFormula {
                    triples: conclusion_triples,
                    len: 1,
                },
            }
        };

        let contract_hash = 12345;

        // 1. Defeater
        let defeater_rule = make_rule(RuleType::Defeater, "q42:permit");
        let q = compile_n3_rule_to_norm(&defeater_rule, contract_hash, 0).unwrap();
        assert_eq!(
            q.predicate & 0xFF,
            crate::modalities::logic::deontic::OP_PERMIT as u64
        );
        assert_ne!(
            q.predicate & crate::modalities::logic::deontic::DEFEATER_BIT,
            0
        );

        // 2. Defeasible Permit
        let permit_rule = make_rule(RuleType::Defeasible, "q42:permit");
        let q2 = compile_n3_rule_to_norm(&permit_rule, contract_hash, 0).unwrap();
        assert_eq!(
            q2.predicate & 0xFF,
            crate::modalities::logic::deontic::OP_PERMIT as u64
        );
        assert_eq!(
            q2.predicate & crate::modalities::logic::deontic::DEFEATER_BIT,
            0
        );

        // 3. Strict Obligate
        let obligate_rule = make_rule(RuleType::Strict, "q42:obligate");
        let q3 = compile_n3_rule_to_norm(&obligate_rule, contract_hash, 0).unwrap();
        assert_eq!(
            q3.predicate & 0xFF,
            crate::modalities::logic::deontic::OP_OBLIGATE as u64
        );
        assert_eq!(
            q3.predicate & crate::modalities::logic::deontic::DEFEATER_BIT,
            0
        );

        // 4. Malformed
        let mut malformed = make_rule(RuleType::Strict, "q42:obligate");
        malformed.premise.len = 0;
        assert!(compile_n3_rule_to_norm(&malformed, contract_hash, 0).is_none());
    }

    #[test]
    fn test_compile_n3_ast_and_stack_rules_and_agreements() {
        use crate::modalities::logic::n3_parser::{
            Formula, Rule as N3Rule, StackFormula, StackRule as N3StackRule, Term as N3Term,
            Triple as N3Triple,
        };

        let t1 = N3Triple {
            subject: N3Term::Uri("did:agent:alice"),
            predicate: N3Term::Uri("q42:mayEnter"),
            object: N3Term::Uri("zone:park"),
        };
        let t2 = N3Triple {
            subject: N3Term::Uri("urn:contract:sanctuary"),
            predicate: N3Term::Uri("q42:appliesTo"),
            object: N3Term::Uri("zone:park"),
        };

        // AST Defeater rule
        let ast_defeater = N3Rule {
            id: Some("r1"),
            rule_type: RuleType::Defeater,
            weight: None,
            premise: Formula { triples: vec![t1] },
            conclusion: Formula { triples: vec![t2] },
        };
        let norm1 = compile_n3_ast_rule_to_norm(&ast_defeater, 0, 1000).expect("ast norm compiles");
        assert_eq!(norm1.predicate & 0xFF, OP_PERMIT as u64);
        assert_ne!(norm1.predicate & DEFEATER_BIT, 0);

        // Stack strict obligate rule
        let mut s_premise = StackFormula::new();
        s_premise.triples[0] = N3Triple {
            subject: N3Term::Uri("did:agent:bob"),
            predicate: N3Term::Uri("q42:mustPayFee"),
            object: N3Term::Literal("10"),
        };
        s_premise.len = 1;
        let mut s_conclusion = StackFormula::new();
        s_conclusion.triples[0] = t2;
        s_conclusion.len = 1;

        let stack_rule = N3StackRule {
            id: None,
            rule_type: RuleType::Strict,
            weight: None,
            premise: s_premise,
            conclusion: s_conclusion,
        };
        let norm2 = compile_stack_rule_to_norm(&stack_rule, 0, 2000).expect("stack norm compiles");
        assert_eq!(norm2.predicate & 0xFF, OP_OBLIGATE as u64);
        assert_eq!(norm2.predicate & DEFEATER_BIT, 0);

        // AgreementDID compile to deontic norms
        let mut agreement = AgreementDID {
            agreement_id: 0x9999,
            principal: 0x1111,
            agents: [0x2222, 0x3333, 0, 0, 0, 0, 0, 0],
            num_agents: 2,
            domain_id: 0x4444,
            threshold: 2,
            current_state: AgreementState::Ratified,
        };
        let mut agreement_norms = [crate::NQuin::default(); 4];
        let count = compile_agreement_to_deontic_norms(&agreement, 3000, &mut agreement_norms);
        assert_eq!(count, 2);
        assert_eq!(agreement_norms[0].subject, 0x2222);
        assert_eq!(agreement_norms[0].predicate & 0xFF, OP_PERMIT as u64);
        assert_eq!(agreement_norms[1].subject, 0x3333);

        agreement.current_state = AgreementState::Proposed;
        assert_eq!(
            compile_agreement_to_deontic_norms(&agreement, 3000, &mut agreement_norms),
            0
        );
    }
}
