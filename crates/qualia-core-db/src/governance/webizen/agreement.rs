use super::*;

#[cfg(feature = "alloc_buffers")]
extern crate alloc;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementState {
    Proposed = 0x00,
    PartiallySigned = 0x01,
    Ratified = 0x02,
}

#[derive(Debug, Clone)]
pub struct AgreementDomain {
    #[cfg(feature = "alloc_buffers")]
    pub name: alloc::string::String,
    #[cfg(not(feature = "alloc_buffers"))]
    pub name: std::string::String,
    pub domain_id: u64,
}

#[derive(Debug, Clone)]
pub struct AgreementConstraint {
    pub required_signatures: u8,
}

pub struct AgreementDID {
    pub agreement_id: u64,
    pub principal: u64,
    pub agents: [u64; 8],
    pub num_agents: u8,
    pub domain_id: u64,
    pub threshold: u8,
    pub current_state: AgreementState,
}

impl AgreementDID {
    /// Compiles a ratified agreement into hardware-aligned Super-Quins.
    pub fn compile_to_super_quins(&self) -> [NQuin; 16] {
        let mut buffer = [NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 16];
        if self.current_state != AgreementState::Ratified {
            return buffer;
        }

        let mut idx = 0;
        let has_guardian = crate::q_hash("q42:hasGuardian");
        let has_domain_scope = crate::q_hash("q42:hasDomainScope");
        let requires_consensus = crate::q_hash("q42:requiresConsensus");

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = NQuin {
                    subject: self.principal,
                    predicate: has_guardian,
                    object: self.agents[i],
                    context: self.agreement_id,
                    // Embed routing lane (Bilateral Micro-Commons) and the State
                    metadata: 0x4000_0000_0000_0002 | ((self.current_state as u64) << 48),
                    parity: 0,
                };
                idx += 1;
            }
        }

        for i in 0..self.num_agents as usize {
            if idx < 16 {
                buffer[idx] = NQuin {
                    subject: self.agreement_id,
                    predicate: has_domain_scope,
                    object: self.domain_id,
                    context: self.agents[i],
                    metadata: 0x4000_0000_0000_0002,
                    parity: 0,
                };
                idx += 1;
            }
        }

        if idx < 16 {
            buffer[idx] = NQuin {
                subject: self.agreement_id,
                predicate: requires_consensus,
                object: self.threshold as u64,
                context: self.domain_id,
                metadata: 0x4000_0000_0000_0002,
                parity: 0,
            };
        }

        buffer
    }
}

/// Values abuse-check (the engine side of the MCP `values_check` tool).
///
/// Runs the REAL inverse rights-guard lane (agency.n3 G1 + its software-agent twin G1')
/// in a fresh arena: a non–natural-person agent that *claims* a natural-person-only dignity
/// right trips `values:PersonhoodCategoryError`. This is the anti-capture invariant — a
/// `CorporatePerson` or an `ArtificialAgent` cannot wear a human's dignity right as its own.
///
/// `agent_type` is `q_hash("https://ns.webcivics.net/values/<Class>")`. Returns `true` iff the
/// guard fires. A `NaturalPerson` (or a non-claiming agent) is never flagged. Cold path — uses
/// the same `Rule`/`Formula` machinery as `register_rule`, never a hot-path allocation.
pub fn check_personhood_category_error(agent_type: u64, claims_dignity_right: bool) -> bool {
    use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Term, Triple};
    const B: &str = "https://ns.webcivics.net/values/";
    let vh = |s: &str| crate::q_hash(s);
    let u = |s: &'static str| Term::Uri(s);
    let var = |n: &'static str| Term::Variable(n);

    // G-guard for a given non-natural-person class: claiming a NaturalPerson-held Right → flag.
    // `class_uri` is the FULL values: IRI of the guarded class, so its `q_hash`
    // matches the `agent_type` fact below. Full-IRI `&'static str` literals keep
    // this zero-heap (the predecessor leaked `format!` Strings via `Box::leak`).
    let guard = |id: &'static str, class_uri: &'static str| Rule {
        id: Some(id),
        rule_type: RuleType::Strict,
        weight: None,
        premise: Formula {
            triples: vec![
                Triple {
                    subject: var("c"),
                    predicate: u("a"),
                    object: u(class_uri),
                },
                Triple {
                    subject: var("c"),
                    predicate: u("https://ns.webcivics.net/values/claims"),
                    object: var("r"),
                },
                Triple {
                    subject: var("r"),
                    predicate: u("a"),
                    object: u("https://ns.webcivics.net/values/Right"),
                },
                Triple {
                    subject: var("r"),
                    predicate: u("https://ns.webcivics.net/values/heldBy"),
                    object: u("https://ns.webcivics.net/values/NaturalPerson"),
                },
            ],
        },
        conclusion: Formula {
            triples: vec![Triple {
                subject: var("c"),
                predicate: u("https://ns.webcivics.net/values/flag"),
                object: u("https://ns.webcivics.net/values/PersonhoodCategoryError"),
            }],
        },
    };

    let mut arena = SlgArena::new();
    let r1 = guard(
        "agency-G1",
        "https://ns.webcivics.net/values/CorporatePerson",
    );
    arena.register_rule(&r1);
    let r2 = guard(
        "agency-G1-prime",
        "https://ns.webcivics.net/values/ArtificialAgent",
    );
    arena.register_rule(&r2);

    let fact = |a: &mut SlgArena, s: u64, p: u64, o: u64| {
        a.write_table(NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: s ^ p ^ o,
        });
    };
    let agent = vh("urn:webcivics:values-check:agent");
    let right = vh("urn:webcivics:values-check:right");
    fact(&mut arena, agent, vh("a"), agent_type);
    if claims_dignity_right {
        fact(&mut arena, agent, vh(&format!("{B}claims")), right);
        fact(&mut arena, right, vh("a"), vh(&format!("{B}Right")));
        fact(
            &mut arena,
            right,
            vh(&format!("{B}heldBy")),
            vh(&format!("{B}NaturalPerson")),
        );
    }
    let _ = arena.fire_registered_rules(crate::q_hash("contract:values-check"));
    arena.has_quin(
        agent,
        vh(&format!("{B}flag")),
        vh(&format!("{B}PersonhoodCategoryError")),
    )
}
