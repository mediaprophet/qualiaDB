//! Agent-identifier model (#16): resolve an identifier's AGENT TYPE and check grounding.
//!
//! An agent's type is carried as a graph relation (`rdf:type` → a values agent class),
//! the same modal-predicate pattern as [`crate::modal_kind`], resolved via zero-alloc
//! `QuinIndex` point lookups (one identity space, #14). On top of resolution this
//! activates the agency.n3 grounding guard at runtime: an `ArtificialAgent` /
//! `PlatformAgent` acting with no Principal (`values:operatedBy`) is UngroundedAgency —
//! the G1' guard that keeps an AI agent accountable to a human principal rather than
//! free-floating. (Agent identity is still relational + never definitive — this resolves
//! a declared type, it does not *fix* the agent; see principle-identifiers-not-identity.)

use crate::indexing::QuinIndex;
use crate::q_hash;

pub const P_RDF_TYPE: u64 = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// The accountable principal behind an agent (agency.n3 `values:operatedBy`).
pub const P_OPERATED_BY: u64 = q_hash("https://ns.webcivics.net/values/operatedBy");

// The values agent lattice (agency.n3).
pub const A_NATURAL_PERSON: u64 = q_hash("https://ns.webcivics.net/values/NaturalPerson");
pub const A_LEGAL_PERSON: u64 = q_hash("https://ns.webcivics.net/values/LegalPerson");
pub const A_PUBLIC_AUTHORITY: u64 = q_hash("https://ns.webcivics.net/values/PublicAuthority");
pub const A_ARTIFICIAL_AGENT: u64 = q_hash("https://ns.webcivics.net/values/ArtificialAgent");
pub const A_PLATFORM_AGENT: u64 = q_hash("https://ns.webcivics.net/values/PlatformAgent");

/// The declared agent class of `agent` (its `rdf:type`), if any.
pub fn agent_type(index: &QuinIndex, agent: u64) -> Option<u64> {
    index.object_of(agent, P_RDF_TYPE)
}

/// The accountable principal behind `agent` (`values:operatedBy`), if declared.
pub fn principal_of(index: &QuinIndex, agent: u64) -> Option<u64> {
    index.object_of(agent, P_OPERATED_BY)
}

/// Whether an agent class is a non-personhood artificial agent (must be grounded).
#[inline]
pub fn is_artificial(agent_class: u64) -> bool {
    agent_class == A_ARTIFICIAL_AGENT || agent_class == A_PLATFORM_AGENT
}

/// agency.n3 G1' grounding guard: an `ArtificialAgent` / `PlatformAgent` acting with no
/// Principal is **UngroundedAgency**. Returns `true` when the agent trips the flag.
pub fn is_ungrounded_agency(index: &QuinIndex, agent: u64) -> bool {
    match agent_type(index, agent) {
        Some(class) if is_artificial(class) => principal_of(index, agent).is_none(),
        _ => false,
    }
}

/// Readable name for a known agent class.
pub fn agent_type_name(class: u64) -> Option<&'static str> {
    match class {
        A_NATURAL_PERSON => Some("NaturalPerson"),
        A_LEGAL_PERSON => Some("LegalPerson"),
        A_PUBLIC_AUTHORITY => Some("PublicAuthority"),
        A_ARTIFICIAL_AGENT => Some("ArtificialAgent"),
        A_PLATFORM_AGENT => Some("PlatformAgent"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    fn t(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn resolves_agent_type() {
        let alice = q_hash("https://example.org/alice");
        let idx = QuinIndex::from_slice(&[t(alice, P_RDF_TYPE, A_NATURAL_PERSON)]);
        assert_eq!(agent_type(&idx, alice), Some(A_NATURAL_PERSON));
        assert_eq!(agent_type_name(agent_type(&idx, alice).unwrap()), Some("NaturalPerson"));
    }

    #[test]
    fn natural_person_is_never_ungrounded() {
        let alice = q_hash("https://example.org/alice");
        let idx = QuinIndex::from_slice(&[t(alice, P_RDF_TYPE, A_NATURAL_PERSON)]);
        assert!(!is_ungrounded_agency(&idx, alice));
    }

    #[test]
    fn artificial_agent_without_principal_is_ungrounded() {
        let bot = q_hash("https://example.org/bot");
        let idx = QuinIndex::from_slice(&[t(bot, P_RDF_TYPE, A_ARTIFICIAL_AGENT)]);
        // No values:operatedBy → trips the agency.n3 G1' UngroundedAgency guard.
        assert!(is_ungrounded_agency(&idx, bot));
    }

    #[test]
    fn artificial_agent_with_principal_is_grounded() {
        let bot = q_hash("https://example.org/bot");
        let human = q_hash("https://example.org/alice");
        let idx = QuinIndex::from_slice(&[
            t(bot, P_RDF_TYPE, A_ARTIFICIAL_AGENT),
            t(bot, P_OPERATED_BY, human), // a Principal stands behind it
        ]);
        assert!(!is_ungrounded_agency(&idx, bot));
        assert_eq!(principal_of(&idx, bot), Some(human));
    }
}
