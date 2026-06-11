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
    pub const AUTHENTICATED:      u64 = q_hash("urn:qualia:role:authenticated");
    /// Professional context (e.g., organisation employee with NDA).
    pub const PROFESSIONAL:       u64 = q_hash("urn:qualia:role:professional");
    /// Legal practitioner with privileged access to legal subgraph.
    pub const LEGAL_PRACTITIONER: u64 = q_hash("urn:qualia:role:legal-practitioner");
    /// Registered medical professional.
    pub const MEDICAL_PROFESSIONAL: u64 = q_hash("urn:qualia:role:medical-professional");
    /// Fiduciary duty holder (financial advisor, trustee, etc.).
    pub const FIDUCIARY:          u64 = q_hash("urn:qualia:role:fiduciary");
    /// Qualia node operator — administrative role.
    pub const NODE_OPERATOR:      u64 = q_hash("urn:qualia:role:node-operator");
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
    pub did_hash:         u64,
    pub roles:            [u64; MAX_ROLES],
    pub role_count:       u8,
    /// Numeric clearance level (0=public … 4=fiduciary). Sourced from `vc:clearanceLevel`.
    pub clearance_level:  u8,
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

/// The result of a deontic policy evaluation for subgraph key release.
#[derive(Debug)]
pub enum DeonticResult {
    /// Policy passed — the derived `SubgraphKey` can be released to the agent.
    KeyRelease(SubgraphKey),
    /// Policy denied. The key is not returned.
    AccessDenied { layer: SubgraphLayer, reason: &'static str },
}

impl DeonticResult {
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

        SubgraphLayer::Fiduciary => {
            vc.clearance_level >= 4 || vc.has_role(vc_roles::FIDUCIARY)
        }
    };

    if policy_passed {
        DeonticResult::KeyRelease(vault.generate_layer_key(layer))
    } else {
        DeonticResult::AccessDenied { layer, reason: "insufficient VC clearance or role" }
    }
}

/// Convenience: evaluate which layers the agent can access and return all permitted keys.
///
/// Returns `(SubgraphLayer, SubgraphKey)` pairs in ascending layer order.
/// At most 5 entries (one per layer). This function is not on a hot path.
#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_accessible_layers(
    vault: &KeyVault,
    vc: &VcAttributes,
    trusted_issuers: &[u64],
) -> Vec<(SubgraphLayer, SubgraphKey)> {
    let layers = [
        SubgraphLayer::Public,
        SubgraphLayer::Professional,
        SubgraphLayer::Legal,
        SubgraphLayer::Medical,
        SubgraphLayer::Fiduciary,
    ];
    let mut result = Vec::with_capacity(layers.len());
    for layer in layers {
        if let DeonticResult::KeyRelease(key) =
            evaluate_vc_for_subgraph_key_release(vault, vc, layer, trusted_issuers)
        {
            result.push((layer, key));
        }
    }
    result
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

    let role_hash = if roles.is_empty() { vc_roles::AUTHENTICATED } else { roles[0] };

    [
        crate::NQuin {
            subject:   agent_did,
            predicate: P_HAS_ROLE,
            object:    role_hash,
            context:   VC_CONTEXT,
            metadata:  ts & 0xFFFF_FFFF,
            parity:    agent_did ^ P_HAS_ROLE ^ role_hash ^ VC_CONTEXT,
        },
        crate::NQuin {
            subject:   agent_did,
            predicate: P_CLEARANCE_LEVEL,
            object:    clearance as u64,
            context:   VC_CONTEXT,
            metadata:  ts & 0xFFFF_FFFF,
            parity:    agent_did ^ P_CLEARANCE_LEVEL ^ (clearance as u64) ^ VC_CONTEXT,
        },
        crate::NQuin {
            subject:   agent_did,
            predicate: P_ISSUED_BY,
            object:    issuer_did,
            context:   VC_CONTEXT,
            metadata:  ts & 0xFFFF_FFFF,
            parity:    agent_did ^ P_ISSUED_BY ^ issuer_did ^ VC_CONTEXT,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vault() -> KeyVault {
        let tmp = tempfile::tempdir().expect("tmpdir");
        KeyVault::load_or_generate(tmp.path().to_str().unwrap()).expect("vault")
    }

    const ISSUER: u64 = 0x1550_0000_0000_CAFE;
    const AGENT:  u64 = 0xA6E4_7777_0000_0001;

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
        assert!(!evaluate_vc_for_subgraph_key_release(&vault, &no_vc, SubgraphLayer::Professional, &[]).is_permitted());

        // Professional role → permit.
        let pro = vc_with_role(vc_roles::PROFESSIONAL, 0);
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &pro, SubgraphLayer::Professional, &[]).is_permitted());

        // Clearance 1 → permit.
        let cleared = vc_with_role(0, 1);
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &cleared, SubgraphLayer::Professional, &[]).is_permitted());
    }

    #[test]
    fn medical_layer_requires_role_or_clearance_3() {
        let vault = test_vault();

        let low = vc_with_role(vc_roles::PROFESSIONAL, 0);
        assert!(!evaluate_vc_for_subgraph_key_release(&vault, &low, SubgraphLayer::Medical, &[]).is_permitted());

        let med_role = vc_with_role(vc_roles::MEDICAL_PROFESSIONAL, 0);
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &med_role, SubgraphLayer::Medical, &[]).is_permitted());

        let cleared = vc_with_role(0, 3);
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &cleared, SubgraphLayer::Medical, &[]).is_permitted());
    }

    #[test]
    fn fiduciary_layer_strictest() {
        let vault = test_vault();

        let med = vc_with_role(vc_roles::MEDICAL_PROFESSIONAL, 3);
        assert!(!evaluate_vc_for_subgraph_key_release(&vault, &med, SubgraphLayer::Fiduciary, &[]).is_permitted());

        let fid = vc_with_role(vc_roles::FIDUCIARY, 0);
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &fid, SubgraphLayer::Fiduciary, &[]).is_permitted());
    }

    #[test]
    fn trusted_issuer_check_blocks_unknown_issuer() {
        let vault = test_vault();
        let trusted = [ISSUER];

        let mut vc = vc_with_role(vc_roles::FIDUCIARY, 4);
        vc.credential_issuer = 0xBAD_CAFE; // wrong issuer
        assert!(!evaluate_vc_for_subgraph_key_release(&vault, &vc, SubgraphLayer::Fiduciary, &trusted).is_permitted());

        vc.credential_issuer = ISSUER; // correct issuer
        assert!(evaluate_vc_for_subgraph_key_release(&vault, &vc, SubgraphLayer::Fiduciary, &trusted).is_permitted());
    }

    #[test]
    fn evaluate_accessible_layers_returns_permitted_set() {
        let vault = test_vault();
        // clearance 2 → Public + Professional + Legal, not Medical or Fiduciary.
        let vc = vc_with_role(0, 2);
        let accessible = evaluate_accessible_layers(&vault, &vc, &[]);
        let layers: Vec<SubgraphLayer> = accessible.iter().map(|(l, _)| *l).collect();
        assert_eq!(layers, vec![SubgraphLayer::Public, SubgraphLayer::Professional, SubgraphLayer::Legal]);
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
        use crate::webizen_bytecode::{execute_program, GuardianshipContext, VmError};
        use crate::mini_parser::{OP_EVAL_PERMIT, OP_END};
        
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
}
