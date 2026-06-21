//! SHACL extensions for ALL Webizen VM logic modalities.
//!
//! Every logic modality wired to a `Native*` opcode carries a SHACL
//! `q42:<Name>ConfigurationShape` constraining its parameters and the
//! predicate/opcode packing convention it relies on. Namespace
//! `https://webizen.org/q42#` (mirrors `core_modalities_shacl.rs`). The TTL is
//! also mirrored at `shapes/logic-modalities.shacl.ttl`.

/// The 20 logic modalities that MUST each have a configuration shape below.
/// Used by the completeness test so a new modality cannot land without its SHACL.
pub const LOGIC_MODALITY_SHAPES: [&str; 21] = [
    "DeonticConfigurationShape",
    "EpistemicConfigurationShape",
    "LinearConfigurationShape",
    "AspConfigurationShape",
    "ParaconsistentConfigurationShape",
    "DialecticalConfigurationShape",
    "DefeasibleConfigurationShape",
    "LtlConfigurationShape",
    "AllenIntervalConfigurationShape",
    "ProbabilisticConfigurationShape",
    "DlSubsumptionConfigurationShape",
    "ArgumentationConfigurationShape",
    "MetricTemporalConfigurationShape",
    "ContraryToDutyConfigurationShape",
    "CausalNecessityConfigurationShape",
    "AbductiveConfigurationShape",
    "ClosedWorldConfigurationShape",
    "FuzzyConfigurationShape",
    "CtlConfigurationShape",
    "ModalConfigurationShape",
    "Rcc8ConfigurationShape",
];

/// SHACL TTL covering every VM logic modality (configuration + structural shapes).
pub fn get_logic_modalities_shacl_ttl() -> &'static str {
    r#"
@prefix q42: <https://webizen.org/q42#> .
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

# ── Deontic (O/P/F) ─────────────────────────────────────────────────────────
q42:DeonticConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:deonticOpcode ; sh:datatype xsd:unsignedByte ;
        sh:minInclusive 16 ; sh:maxInclusive 18 ;
        sh:message "Deontic opcode must be OP_OBLIGATE(0x10) | OP_PERMIT(0x11) | OP_FORBID(0x12)." ] .

# ── Epistemic (K/B) ─────────────────────────────────────────────────────────
q42:EpistemicConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:certainty ; sh:datatype xsd:unsignedByte ;
        sh:minInclusive 0 ; sh:maxInclusive 255 ;
        sh:message "Certainty is an 8-bit value (0–255); ≥128 ⇒ Active." ] .

# ── Linear (resource consumption) ───────────────────────────────────────────
q42:LinearConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:linearResource ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "A linear gate names the resource quin it consumes." ] .

# ── ASP (stable models) ─────────────────────────────────────────────────────
q42:AspConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:maxStableModels ; sh:datatype xsd:integer ;
        sh:minInclusive 1 ; sh:maxInclusive 8 ;
        sh:message "Stable models enumerated ≤ MAX_STABLE_MODELS (8)." ] .

# ── Paraconsistent ──────────────────────────────────────────────────────────
q42:ParaconsistentConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:isolateContradictions ; sh:datatype xsd:boolean ;
        sh:message "Paraconsistent routing isolates contradictory quins." ] .

# ── Dialectical (thesis/antithesis → synthesis) ─────────────────────────────
q42:DialecticalConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:minFacts ; sh:datatype xsd:integer ; sh:minInclusive 2 ;
        sh:message "Dialectical synthesis requires ≥ 2 contradictory facts." ] .

# ── Defeasible (q42:unless) ─────────────────────────────────────────────────
q42:DefeasibleConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:defeaterBit ; sh:datatype xsd:boolean ;
        sh:message "A defeater node sets the DEFEATER_BIT (predicate bit 63)." ] .

# ── Temporal LTL ────────────────────────────────────────────────────────────
q42:LtlConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:ltlOpcode ; sh:datatype xsd:unsignedByte ;
        sh:minInclusive 64 ; sh:maxInclusive 68 ;
        sh:message "LTL opcode ∈ {Globally 0x40 … Release 0x44}; compares the FULL predicate (PLAN §9.2)." ] .

# ── Allen interval algebra ──────────────────────────────────────────────────
q42:AllenIntervalConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:allenRelation ; sh:datatype xsd:unsignedByte ;
        sh:minInclusive 0 ; sh:maxInclusive 6 ;
        sh:message "Allen relation ∈ {Before 0 … Equals 6}." ] .

# ── Probabilistic (Bayesian threshold) ──────────────────────────────────────
q42:ProbabilisticConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:weight ; sh:datatype xsd:decimal ;
        sh:minInclusive 0.0 ; sh:maxInclusive 1.0 ;
        sh:message "A probabilistic belief weight is a probability in [0,1]." ] .

# ── Description-logic subsumption ───────────────────────────────────────────
q42:DlSubsumptionConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:subClassPredicate ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "DL subsumption walks transitive rdfs:subClassOf edges." ] .

# ── Argumentation (Dung grounded) ───────────────────────────────────────────
q42:ArgumentationConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:maxArguments ; sh:datatype xsd:integer ;
        sh:minInclusive 1 ; sh:maxInclusive 128 ;
        sh:message "Grounded-extension membership over ≤ MAX_GROUNDED_ARGS (128) arguments." ] .

# ── Metric/timed temporal (deadlines) ───────────────────────────────────────
q42:MetricTemporalConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:window ; sh:datatype xsd:integer ; sh:minInclusive 0 ;
        sh:message "MTL 'within' window is a non-negative duration; event timestamps live in metadata." ] .

# ── Contrary-to-duty (dyadic deontic) ───────────────────────────────────────
q42:ContraryToDutyConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:primaryObligation ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "CTD names the primary (breached) obligation." ] ;
    sh:property [ sh:path q42:reparation ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "CTD names the secondary reparation obligation required after breach." ] .

# ── Causal necessity (but-for) ──────────────────────────────────────────────
q42:CausalNecessityConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:causalEdgePredicate ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "But-for necessity is computed over cause→effect edge quins." ] .

# ── Abductive (inference to best explanation) ───────────────────────────────
q42:AbductiveConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:explainsPredicate ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "Abduction walks backward over hypothesis→observation explanatory edges." ] .

# ── Closed-world / negation-as-failure ──────────────────────────────────────
q42:ClosedWorldConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:closedWorld ; sh:datatype xsd:boolean ;
        sh:message "NAF: a proposition holds by default exactly when it is absent (unprovable)." ] .

# ── Fuzzy / many-valued (t-norm) ────────────────────────────────────────────
q42:FuzzyConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:truthDegree ; sh:datatype xsd:decimal ;
        sh:minInclusive 0.0 ; sh:maxInclusive 1.0 ;
        sh:message "A fuzzy truth degree ∈ [0,1]; conjunction via the Gödel t-norm (min)." ] .

# ── CTL (branching-time) ────────────────────────────────────────────────────
q42:CtlConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:ctlOperator ; sh:in ( "EF" "AG" ) ;
        sh:message "Supported CTL operators: EF (exists-finally), AG (always-globally)." ] .

# ── General modal (Kripke □/◇) ──────────────────────────────────────────────
q42:ModalConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:modalOperator ; sh:in ( "box" "diamond" ) ;
        sh:message "Modal operator: box (□ necessary, all accessible worlds) | diamond (◇ possible, some)." ] ;
    sh:property [ sh:path q42:accessibilityPredicate ; sh:nodeKind sh:IRI ; sh:minCount 1 ;
        sh:message "□/◇ quantify over a Kripke accessibility relation (modal:accesses)." ] .

# ── RCC-8 spatial topology (full polygon, zero-heap) ────────────────────────
q42:Rcc8ConfigurationShape a sh:NodeShape ;
    sh:property [ sh:path q42:rcc8Relation ; sh:datatype xsd:unsignedByte ;
        sh:minInclusive 0 ; sh:maxInclusive 7 ;
        sh:message "RCC-8 relation ∈ {DC 0, EC 1, PO 2, TPP 3, TPPi 4, NTPP 5, NTPPi 6, EQ 7}." ] ;
    sh:property [ sh:path q42:maxBoundaryPoints ; sh:datatype xsd:integer ;
        sh:minInclusive 3 ; sh:maxInclusive 64 ;
        sh:message "A region is 3–MAX_BOUNDARY_POINTS (64) boundary-point quins (spatial:boundary; metadata = vertex sequence)." ] .
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_modality_has_a_configuration_shape() {
        let ttl = get_logic_modalities_shacl_ttl();
        for shape in LOGIC_MODALITY_SHAPES {
            assert!(
                ttl.contains(&format!("q42:{shape}")),
                "missing SHACL configuration shape for {shape}"
            );
        }
        // The new modalities specifically must be present.
        for s in ["MetricTemporal", "ContraryToDuty", "CausalNecessity", "Abductive",
                  "ClosedWorld", "Fuzzy", "Ctl", "Modal"] {
            assert!(ttl.contains(s), "new modality {s} missing its SHACL shape");
        }
        // Namespace migrated (no residual qualia.* namespace).
        assert!(ttl.contains("https://webizen.org/q42#"));
        assert!(!ttl.contains("qualia.network/q42"));
    }
}
