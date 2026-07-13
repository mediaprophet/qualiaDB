//! Human-centric identity & data-rights SHACL extensions.
//!
//! Four structural enforcements the Webizen Sentinel needs, each grounded in the
//! project's identity principles (`identifiers-not-identity`,
//! `out-of-band-remainder-is-freedom`, `governance-topology-relational`):
//!
//! 1. **Identity as an enumerated state** — an identity is validated as a *bounded
//!    set of cryptographically-attested identifiers* with a confidence *relation*,
//!    never collapsed to one definitive identifier. A binding asserting certainty
//!    (`confidence >= 1.0`) is a `DefinitiveCollapse` and is rejected: the
//!    out-of-band remainder (the un-resolvable link to the natural person) is what
//!    keeps the person free, so it is a hard invariant here, not an afterthought.
//! 2. **Decentralized shape-target routing** — shapes are bound to storage *loci*
//!    (personal data stores / peers); validation is dispatched to where the data
//!    lives (local-first) instead of pulling everything into a central index.
//! 3. **Real-time severity degradation** — off-grid, non-critical violations degrade
//!    to non-blocking so a *partial* subgraph stays usable; `Critical` violations
//!    (identity / consent / safety) never degrade — they fail closed. This mirrors
//!    the deontic non-derogable rule.
//! 4. **Verifiable-Credential-gated targets** — a SHACL target applies to a focus
//!    node only when a *verified* W3C VC is presented about it (origin-authenticated
//!    data-rights property validation; the VC layer checks the signature/expiry first).
//!
//! All four runtime predicates are **zero-heap**: bounded slices in, scalars / enums
//! / caller `out` buffers out. The TTL/opcode emitters write constants or into a
//! caller slice.

use crate::verifiable_credential::Credential;
use crate::webizen::SlgOpcode;

// ── 1. Identity as an enumerated, cryptographically-attested state ──────────────

/// Bounded number of identifier bindings an enumerated identity may carry.
pub const MAX_IDENTITY_BINDINGS: usize = 32;

/// The cryptographic scheme that attests an identifier binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoScheme {
    Ed25519,
    MlDsa65,
    Blake3Commitment,
    X25519,
    /// No real crypto backing — does NOT count toward the attestation requirement.
    Unknown,
}

/// One identifier in an enumerated identity: a handle, the crypto scheme attesting
/// it, whether an attestation is actually present, and the confidence (strictly in
/// `(0,1)`) that this identifier picks out the natural person.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentifierBinding {
    /// `q_hash` of the identifier (DID, key id, handle, …).
    pub identifier: u64,
    pub scheme: CryptoScheme,
    pub attested: bool,
    pub confidence: f32,
}

/// The verdict of [`validate_enumerated_identity`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IdentityValidation {
    /// Enough distinct, crypto-attested identifiers; identity stands as an enumerated
    /// state. `aggregate_confidence` is the noisy-OR combination (always `< 1.0`).
    Valid {
        distinct: u16,
        attested: u16,
        aggregate_confidence: f32,
    },
    /// Too few distinct identifiers and/or crypto attestations.
    Underdetermined { distinct: u16, attested: u16 },
    /// A binding claims certainty (`confidence >= 1.0`) — REJECTED. Identity must
    /// remain a confidence-relation; collapsing it to a definitive identifier
    /// destroys the out-of-band remainder.
    DefinitiveCollapse,
}

/// Validate an identity as an enumerated state over crypto-attested identifiers.
///
/// `min_distinct` distinct identifiers and `min_attested` crypto-attested bindings
/// are required. A binding with `confidence >= 1.0` short-circuits to
/// [`IdentityValidation::DefinitiveCollapse`]. Zero-heap (scans the bounded slice).
pub fn validate_enumerated_identity(
    bindings: &[IdentifierBinding],
    min_distinct: u16,
    min_attested: u16,
) -> IdentityValidation {
    let mut distinct = 0u16;
    let mut attested = 0u16;
    let mut not_prob = 1.0f32; // running product of (1 - confidence) for noisy-OR

    for (i, b) in bindings.iter().enumerate() {
        // certainty is a definitive collapse — reject outright.
        if b.confidence >= 1.0 {
            return IdentityValidation::DefinitiveCollapse;
        }
        // count distinct identifiers (first occurrence only).
        let first_seen = !bindings[..i].iter().any(|p| p.identifier == b.identifier);
        if first_seen {
            distinct += 1;
        }
        // a binding counts as attested only with real crypto backing.
        if b.attested && b.scheme != CryptoScheme::Unknown {
            attested += 1;
            not_prob *= 1.0 - b.confidence.clamp(0.0, 1.0);
        }
    }

    if distinct < min_distinct || attested < min_attested {
        return IdentityValidation::Underdetermined { distinct, attested };
    }
    IdentityValidation::Valid {
        distinct,
        attested,
        aggregate_confidence: 1.0 - not_prob,
    }
}

/// Emit the SHACL opcodes for the enumerated-identity shape into `out` (zero-heap);
/// returns the count written. The richer semantic enforcement (collapse rejection,
/// noisy-OR confidence) lives in [`validate_enumerated_identity`].
pub fn enumerated_identity_opcodes(
    min_distinct: u32,
    min_attested: u32,
    out: &mut [SlgOpcode],
) -> usize {
    let ops = [
        SlgOpcode::CheckMinCount(min_distinct),
        SlgOpcode::CheckMinCount(min_attested),
    ];
    let n = ops.len().min(out.len());
    out[..n].copy_from_slice(&ops[..n]);
    n
}

// ── 2. Decentralized shape-target routing ──────────────────────────────────────

/// Bounded number of shape→locus routes the router holds.
pub const MAX_SHAPE_ROUTES: usize = 256;

/// A binding of a SHACL `shape` to a storage `locus` (a personal data store / peer)
/// where its target nodes live. Validation is dispatched to the locus; data is never
/// aggregated centrally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeRoute {
    pub shape: u64,
    pub locus: u64,
}

/// Enumerate the distinct shapes that apply at `locus` (what a local store validates).
/// Writes shape ids into `out`; returns the count. Zero-heap.
pub fn shapes_for_locus(routes: &[ShapeRoute], locus: u64, out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for r in routes {
        if r.locus != locus || out[..n].contains(&r.shape) {
            continue;
        }
        if n >= out.len() {
            break;
        }
        out[n] = r.shape;
        n += 1;
    }
    n
}

/// Enumerate the distinct loci a `shape` must be routed to (fan-out without pulling
/// the data together). Writes locus ids into `out`; returns the count. Zero-heap.
pub fn loci_for_shape(routes: &[ShapeRoute], shape: u64, out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for r in routes {
        if r.shape != shape || out[..n].contains(&r.locus) {
            continue;
        }
        if n >= out.len() {
            break;
        }
        out[n] = r.locus;
        n += 1;
    }
    n
}

/// Whether `shape` is validated locally at `self_locus` (local-first dispatch).
pub fn route_is_local(routes: &[ShapeRoute], shape: u64, self_locus: u64) -> bool {
    routes
        .iter()
        .any(|r| r.shape == shape && r.locus == self_locus)
}

// ── 3. Real-time severity degradation ──────────────────────────────────────────

/// SHACL result severity, ordered so `Critical` is the maximum (never degrades).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaclSeverity {
    Info,
    Warning,
    Violation,
    /// Non-derogable: identity / consent / safety. Fails closed even off-grid.
    Critical,
}

/// Whether the engine is online (strict) or off-grid (partial-utilization tolerant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationMode {
    Online,
    OffGrid,
}

/// A SHACL shape violation against a focus node, with its severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeViolation {
    pub shape: u64,
    pub focus_node: u64,
    pub severity: ShaclSeverity,
}

/// The result of [`degrade_violations`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DegradationOutcome {
    /// Violations that still block (always `Critical`; off-grid, *only* `Critical`).
    pub blocking: u16,
    /// Violations downgraded to non-blocking for off-grid partial utilization.
    pub degraded: u16,
    /// `true` when nothing blocks — the (partial) subgraph may be used.
    pub subgraph_usable: bool,
}

/// Apply real-time severity degradation. `Online` blocks on every `Violation`/
/// `Critical` (nothing degrades). `OffGrid` degrades non-`Critical` violations to
/// non-blocking so a partial subgraph stays usable; `Critical` never degrades.
///
/// Writes the post-degradation violations (with adjusted severity) into `out` and
/// returns the outcome. Zero-heap.
pub fn degrade_violations(
    violations: &[ShapeViolation],
    mode: OperationMode,
    out: &mut [ShapeViolation],
) -> DegradationOutcome {
    let mut blocking = 0u16;
    let mut degraded = 0u16;
    let count = violations.len().min(out.len());

    for (i, v) in violations.iter().take(count).enumerate() {
        let mut adjusted = *v;
        let blocks = match mode {
            OperationMode::Online => v.severity >= ShaclSeverity::Violation,
            OperationMode::OffGrid => {
                if v.severity == ShaclSeverity::Critical {
                    true
                } else {
                    // degrade a blocking violation down to a non-blocking warning.
                    if v.severity >= ShaclSeverity::Violation {
                        adjusted.severity = ShaclSeverity::Warning;
                        degraded += 1;
                    }
                    false
                }
            }
        };
        if blocks {
            blocking += 1;
        }
        out[i] = adjusted;
    }

    DegradationOutcome {
        blocking,
        degraded,
        subgraph_usable: blocking == 0,
    }
}

// ── 4. Verifiable-Credential-gated SHACL targets ───────────────────────────────

/// A credential gate on a SHACL target: the shape applies to a focus node only when
/// a verified VC about that node carries the required claim from an accepted issuer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialGate {
    /// The SHACL shape this gate guards.
    pub shape: u64,
    /// The claim predicate the VC must assert about the subject.
    pub required_claim_predicate: u64,
    /// The required object value (`0` = any value of the predicate).
    pub required_claim_object: u64,
    /// The accepted issuer (`0` = any grounded issuer — the VC layer enforces grounding).
    pub accepted_issuer: u64,
}

/// Decide whether a credential-gated SHACL target applies to `focus_node`, given an
/// **already cryptographically-verified** credential (call
/// [`crate::verifiable_credential::verify`] / `verify_grounded` first — this gates on
/// a *verified* VC, it does not re-check the signature).
///
/// Requires: the credential's subject is the focus node; the issuer is accepted; and
/// the credential carries a claim matching the required predicate/object. Zero-heap
/// (scans the credential's caller-owned claim list).
pub fn credential_gates_target(gate: &CredentialGate, focus_node: u64, vc: &Credential) -> bool {
    if vc.subject != focus_node {
        return false;
    }
    if gate.accepted_issuer != 0 && vc.issuer != gate.accepted_issuer {
        return false;
    }
    vc.claims.iter().any(|q| {
        q.predicate == gate.required_claim_predicate
            && (gate.required_claim_object == 0 || q.object == gate.required_claim_object)
    })
}

// ── SHACL TTL vocabulary for the identity / data-rights shapes ──────────────────

/// SHACL shapes for human-centric identity & data rights.
pub fn get_identity_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://webizen.org/q42#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# Identity is an ENUMERATED state over multiple cryptographically-attested
# identifiers — never a single definitive identifier. The out-of-band remainder is
# preserved: identifier confidence is a RELATION strictly in (0,1), never certainty.
q42:EnumeratedIdentityShape a sh:NodeShape ;
    sh:targetClass q42:Principal ;
    sh:property [
        sh:path q42:hasIdentifier ;
        sh:minCount 2 ;
        sh:message "An identity must enumerate at least two distinct identifiers; a single definitive identifier collapses the out-of-band remainder." ;
    ] ;
    sh:property [
        sh:path q42:identifierAttestation ;
        sh:minCount 1 ;
        sh:nodeKind sh:BlankNodeOrIRI ;
        sh:message "Each identifier must carry a cryptographic attestation (Ed25519 / ML-DSA-65 / BLAKE3 commitment)." ;
    ] ;
    sh:property [
        sh:path q42:identifierConfidence ;
        sh:datatype xsd:decimal ;
        sh:minExclusive 0 ;
        sh:maxExclusive 1 ;
        sh:message "Identifier confidence is strictly in (0,1); certainty (1.0) is a definitive-collapse and is rejected." ;
    ] .

# Decentralized routing: a shape is bound to the locus where its targets live, so
# validation is dispatched locally rather than aggregating personal data centrally.
q42:ShapeRouteShape a sh:NodeShape ;
    sh:targetClass q42:ShapeRoute ;
    sh:property [
        sh:path q42:routedToLocus ;
        sh:minCount 1 ;
        sh:message "Every shape route must name a storage locus; validation goes to the data, not the data to a central index." ;
    ] .

# A SHACL target gated by a presented, verified Verifiable Credential.
q42:CredentialGatedTargetShape a sh:NodeShape ;
    sh:targetSubjectsOf q42:presentsCredential ;
    sh:property [
        sh:path q42:presentsCredential ;
        sh:minCount 1 ;
        sh:message "This target requires a presented, verified Verifiable Credential." ;
    ] .
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verifiable_credential::Credential;
    use crate::{q_hash, NQuin};

    fn binding(
        id: &str,
        scheme: CryptoScheme,
        attested: bool,
        confidence: f32,
    ) -> IdentifierBinding {
        IdentifierBinding {
            identifier: q_hash(id),
            scheme,
            attested,
            confidence,
        }
    }

    #[test]
    fn enumerated_identity_valid_stays_below_certainty() {
        let bindings = [
            binding("did:key:a", CryptoScheme::Ed25519, true, 0.6),
            binding("did:key:b", CryptoScheme::MlDsa65, true, 0.7),
            binding("handle:c", CryptoScheme::Blake3Commitment, true, 0.5),
        ];
        match validate_enumerated_identity(&bindings, 2, 2) {
            IdentityValidation::Valid {
                distinct,
                attested,
                aggregate_confidence,
            } => {
                assert_eq!(distinct, 3);
                assert_eq!(attested, 3);
                // noisy-OR of 0.6/0.7/0.5 = 1 - 0.4*0.3*0.5 = 0.94, strictly < 1.
                assert!(aggregate_confidence > 0.9 && aggregate_confidence < 1.0);
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn enumerated_identity_underdetermined_when_too_few() {
        // Only one attested identifier; an Unknown-scheme binding does not count.
        let bindings = [
            binding("did:key:a", CryptoScheme::Ed25519, true, 0.6),
            binding("guess:b", CryptoScheme::Unknown, true, 0.4),
        ];
        assert_eq!(
            validate_enumerated_identity(&bindings, 2, 2),
            IdentityValidation::Underdetermined {
                distinct: 2,
                attested: 1
            }
        );
    }

    #[test]
    fn certainty_is_a_definitive_collapse() {
        // A binding asserting certainty must be rejected — the out-of-band remainder
        // is a hard invariant.
        let bindings = [
            binding("did:key:a", CryptoScheme::Ed25519, true, 0.6),
            binding("gov:id", CryptoScheme::MlDsa65, true, 1.0),
        ];
        assert_eq!(
            validate_enumerated_identity(&bindings, 1, 1),
            IdentityValidation::DefinitiveCollapse
        );
    }

    #[test]
    fn decentralized_routing_dispatches_to_loci() {
        let routes = [
            ShapeRoute {
                shape: 1,
                locus: 100,
            },
            ShapeRoute {
                shape: 2,
                locus: 100,
            },
            ShapeRoute {
                shape: 1,
                locus: 200,
            },
            ShapeRoute {
                shape: 1,
                locus: 100,
            }, // duplicate, must dedup
        ];
        let mut shapes = [0u64; 8];
        let n = shapes_for_locus(&routes, 100, &mut shapes);
        assert_eq!(n, 2);
        assert!(shapes[..n].contains(&1) && shapes[..n].contains(&2));

        let mut loci = [0u64; 8];
        let m = loci_for_shape(&routes, 1, &mut loci);
        assert_eq!(m, 2); // loci 100 and 200, deduped
        assert!(loci[..m].contains(&100) && loci[..m].contains(&200));

        assert!(route_is_local(&routes, 1, 100));
        assert!(!route_is_local(&routes, 2, 200));
    }

    #[test]
    fn severity_degradation_offgrid_keeps_partial_subgraph_usable() {
        let violations = [
            ShapeViolation {
                shape: 1,
                focus_node: 10,
                severity: ShaclSeverity::Violation,
            },
            ShapeViolation {
                shape: 2,
                focus_node: 11,
                severity: ShaclSeverity::Warning,
            },
        ];
        let mut out = [violations[0]; 2];

        // Online: the Violation blocks → subgraph not usable.
        let online = degrade_violations(&violations, OperationMode::Online, &mut out);
        assert_eq!(online.blocking, 1);
        assert!(!online.subgraph_usable);

        // Off-grid: the Violation degrades to Warning → nothing blocks → usable.
        let offgrid = degrade_violations(&violations, OperationMode::OffGrid, &mut out);
        assert_eq!(offgrid.blocking, 0);
        assert_eq!(offgrid.degraded, 1);
        assert!(offgrid.subgraph_usable);
        assert_eq!(out[0].severity, ShaclSeverity::Warning); // downgraded in place
    }

    #[test]
    fn critical_never_degrades_even_offgrid() {
        let violations = [ShapeViolation {
            shape: 9,
            focus_node: 99,
            severity: ShaclSeverity::Critical,
        }];
        let mut out = [violations[0]; 1];
        let outcome = degrade_violations(&violations, OperationMode::OffGrid, &mut out);
        assert_eq!(outcome.blocking, 1);
        assert_eq!(outcome.degraded, 0);
        assert!(!outcome.subgraph_usable); // fails closed
        assert_eq!(out[0].severity, ShaclSeverity::Critical);
    }

    #[test]
    fn verifiable_credential_gates_the_target() {
        let alice = q_hash("did:example:alice");
        let issuer = q_hash("did:example:gov");
        let cap_pred = q_hash("https://ns.webcivics.net/capability/heldBy");
        let cap_obj = q_hash("cap:LicensedPractitioner");

        let claim = NQuin {
            subject: alice,
            predicate: cap_pred,
            object: cap_obj,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let vc = Credential {
            issuer,
            subject: alice,
            issued_at: 1000,
            valid_until: 0,
            claims: vec![claim],
        };
        let gate = CredentialGate {
            shape: 1,
            required_claim_predicate: cap_pred,
            required_claim_object: cap_obj,
            accepted_issuer: issuer,
        };

        // Matches: right subject, issuer, and claim.
        assert!(credential_gates_target(&gate, alice, &vc));
        // Wrong focus node → does not apply.
        assert!(!credential_gates_target(
            &gate,
            q_hash("did:example:bob"),
            &vc
        ));
        // Wrong issuer → rejected.
        let gate_other_issuer = CredentialGate {
            accepted_issuer: q_hash("did:example:rogue"),
            ..gate
        };
        assert!(!credential_gates_target(&gate_other_issuer, alice, &vc));
        // Missing the required claim object → does not apply.
        let gate_other_claim = CredentialGate {
            required_claim_object: q_hash("cap:Something_Else"),
            ..gate
        };
        assert!(!credential_gates_target(&gate_other_claim, alice, &vc));
        // accepted_issuer = 0 means "any grounded issuer".
        let gate_any_issuer = CredentialGate {
            accepted_issuer: 0,
            ..gate
        };
        assert!(credential_gates_target(&gate_any_issuer, alice, &vc));
    }
}
