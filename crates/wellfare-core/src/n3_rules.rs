/// N3Logic clinical rule engine for WellFair.
///
/// Translates the four N3 rule files from legacy_pwa/extensions/n3_reasoner/rules/
/// into SPARQL SELECT + aggregation queries executed over the health Turtle graph.
/// Each rule fires if its aggregated metric thresholds are met.
///
/// Routing lanes (per qualiaDB IngestionPipeline):
///   N3Logic implication (=>) → BilateralMicroCommons (lane 2) — requires identity context
///   Standard threshold SHACL → PassthroughStandard (lane 0) or Permissive (lane 1)
///
/// The "person" is implicit: we aggregate over all observations in the loaded Turtle.
/// Aggregation strategy: AVG over available records. Rules only fire when data is present.
use crate::store::HealthStore;
use crate::webizen::{LANE_BILATERAL, LANE_PASSTHROUGH};

/// One triggered clinical pattern.
#[derive(Debug)]
pub struct N3RuleMatch {
    pub pattern: &'static str,
    pub confidence: &'static str,
    pub routing_lane: u8,
    /// The original N3 source file this came from.
    pub n3_source: &'static str,
    /// Optional recommended action (present for safety-critical rules).
    pub recommended_action: Option<&'static str>,
}

impl N3RuleMatch {
    fn to_json(&self) -> String {
        let action = self
            .recommended_action
            .map(|a| format!(",\"recommendedAction\":\"{}\"", a))
            .unwrap_or_default();
        format!(
            "{{\"pattern\":\"{}\",\"confidence\":\"{}\",\"routingLane\":{},\"n3Source\":\"{}\"{}}}",
            self.pattern, self.confidence, self.routing_lane, self.n3_source, action
        )
    }
}

/// Extract a single f64 average from a SPARQL query returning `?avg`.
fn query_avg(store: &HealthStore, sparql: &str) -> Option<f64> {
    match store.query(sparql) {
        Ok(json) => {
            // SPARQL JSON results: {"results":{"bindings":[{"avg":{"value":"..."}}]}}
            // Simple extraction without a full JSON parser.
            if let Some(start) = json.find("\"value\":\"") {
                let after = &json[start + 9..];
                if let Some(end) = after.find('"') {
                    return after[..end].parse::<f64>().ok();
                }
            }
            None
        }
        Err(_) => None,
    }
}

const HEALTH: &str = "https://health.example.org/ns#";
const FHIR: &str = "http://hl7.org/fhir/";
const SNOMED: &str = "http://snomed.info/id/";

// SNOMED codes used in the Turtle serialisers (rdf.rs).
const SNOMED_HEART_RATE: &str = "364075005";
const SNOMED_STEPS: &str = "256235009";

/// Evaluate all 7 N3 clinical rules against a pre-loaded `HealthStore`.
/// Returns only the rules that fire (empty vec = no concerns found).
pub fn evaluate_n3_rules(store: &HealthStore) -> Vec<N3RuleMatch> {
    let mut matches = Vec::new();

    // ── Aggregate health metrics ───────────────────────────────────────────────

    // Average sleep efficiency (health:sleepEfficiency)
    let avg_sleep_eff = query_avg(
        store,
        &format!(
            "SELECT (AVG(?e) AS ?avg) WHERE {{ ?obs <{}sleepEfficiency> ?e }}",
            HEALTH
        ),
    );

    // Average sleep hours (health:sleepHours — added by rdf.rs update)
    let avg_sleep_hrs = query_avg(
        store,
        &format!(
            "SELECT (AVG(?h) AS ?avg) WHERE {{ ?obs <{}sleepHours> ?h }}",
            HEALTH
        ),
    );

    // Average resting heart rate (fhir:Observation.valueQuantity on HR records)
    let avg_hr = query_avg(
        store,
        &format!(
            "SELECT (AVG(?hr) AS ?avg) WHERE {{ \
           ?obs <{}snomedConcept> <{}{}>  ; \
                <{}Observation.valueQuantity> ?hr \
         }}",
            HEALTH, SNOMED, SNOMED_HEART_RATE, FHIR
        ),
    );

    // Average daily steps (fhir:Observation.valueQuantity on steps records)
    let avg_steps = query_avg(
        store,
        &format!(
            "SELECT (AVG(?s) AS ?avg) WHERE {{ \
           ?obs <{}snomedConcept> <{}{}>  ; \
                <{}Observation.valueQuantity> ?s \
         }}",
            HEALTH, SNOMED, SNOMED_STEPS, FHIR
        ),
    );

    // Stress score and Maslow safety score (vault-level data, may not be present).
    let avg_stress = query_avg(
        store,
        &format!(
            "SELECT (AVG(?s) AS ?avg) WHERE {{ ?obs <{}stressScore> ?s }}",
            HEALTH
        ),
    );
    let avg_maslow = query_avg(
        store,
        &format!(
            "SELECT (AVG(?m) AS ?avg) WHERE {{ ?obs <{}maslowSafetyScore> ?m }}",
            HEALTH
        ),
    );

    // ── sleep_debt.n3 ─────────────────────────────────────────────────────────
    // { ?person health:sleepEfficiency ?e . ?e math:lessThan 70 .
    //   ?person health:sleepHours ?h . ?h math:lessThan 6 . }
    // => { ?person health:pattern health:ChronicSleepDebt . }
    if let (Some(eff), Some(hrs)) = (avg_sleep_eff, avg_sleep_hrs) {
        if eff < 70.0 && hrs < 6.0 {
            matches.push(N3RuleMatch {
                pattern: "ChronicSleepDebt",
                confidence: "high",
                routing_lane: LANE_BILATERAL,
                n3_source: "sleep_debt.n3",
                recommended_action: None,
            });
        }
    }

    // ── cardiovascular_risk.n3 — Tachycardia ─────────────────────────────────
    // { ?person health:restingHR ?hr . ?hr math:greaterThan 100 . }
    // => { ?person health:pattern health:TachycardiaFlag . }
    if let Some(hr) = avg_hr {
        if hr > 100.0 {
            matches.push(N3RuleMatch {
                pattern: "TachycardiaFlag",
                confidence: "high",
                routing_lane: LANE_BILATERAL,
                n3_source: "cardiovascular_risk.n3",
                recommended_action: None,
            });
        }

        // ── cardiovascular_risk.n3 — Deconditioning ───────────────────────────
        // { ?person health:dailySteps ?steps . ?steps math:lessThan 3000 .
        //   ?person health:restingHR ?hr . ?hr math:greaterThan 80 . }
        // => { ?person health:pattern health:DeconditioningRisk . }
        if let Some(steps) = avg_steps {
            if steps < 3000.0 && hr > 80.0 {
                matches.push(N3RuleMatch {
                    pattern: "DeconditioningRisk",
                    confidence: "moderate",
                    routing_lane: LANE_BILATERAL,
                    n3_source: "cardiovascular_risk.n3",
                    recommended_action: None,
                });
            }
        }
    }

    // ── adrenal_fatigue.n3 ────────────────────────────────────────────────────
    // { ?person health:sleepHours ?h . ?h math:lessThan 5 .
    //   ?person health:restingHR ?hr . ?hr math:greaterThan 90 . }
    // => { ?person health:pattern health:AdrenalFatigueSuspected . }
    if let (Some(hrs), Some(hr)) = (avg_sleep_hrs, avg_hr) {
        if hrs < 5.0 && hr > 90.0 {
            matches.push(N3RuleMatch {
                pattern: "AdrenalFatigueSuspected",
                confidence: "moderate",
                routing_lane: LANE_BILATERAL,
                n3_source: "adrenal_fatigue.n3",
                recommended_action: None,
            });
        }
    }

    // ── trauma_cascade.n3 — TraumaCascadeActive ───────────────────────────────
    // { ?person health:stressScore ?s . ?s math:greaterThan 75 .
    //   ?person health:maslowSafetyScore ?m . ?m math:lessThan 40 .
    //   ?person health:sleepHours ?h . ?h math:lessThan 5 . }
    // => { ?person health:pattern health:TraumaCascadeActive .
    //      ?person health:recommendedAction "urgent-welfare-review" . }
    if let (Some(stress), Some(maslow), Some(hrs)) = (avg_stress, avg_maslow, avg_sleep_hrs) {
        if stress > 75.0 && maslow < 40.0 && hrs < 5.0 {
            matches.push(N3RuleMatch {
                pattern: "TraumaCascadeActive",
                confidence: "high",
                routing_lane: LANE_BILATERAL,
                n3_source: "trauma_cascade.n3",
                recommended_action: Some("urgent-welfare-review"),
            });
        }
    }

    // ── trauma_cascade.n3 — SystemicFrailty ──────────────────────────────────
    // { ?person health:dailySteps ?steps . ?steps math:lessThan 2000 .
    //   ?person health:restingHR ?hr . ?hr math:greaterThan 85 . }
    // => { ?person health:pattern health:SystemicFrailty . }
    if let (Some(steps), Some(hr)) = (avg_steps, avg_hr) {
        if steps < 2000.0 && hr > 85.0 {
            matches.push(N3RuleMatch {
                pattern: "SystemicFrailty",
                confidence: "moderate",
                routing_lane: LANE_BILATERAL,
                n3_source: "trauma_cascade.n3",
                recommended_action: None,
            });
        }
    }

    // Rules that use only PassthroughStandard data (no identity context needed):
    // These re-use the metric values already computed above.

    // Low daily activity flag (standalone, lane 0)
    if let Some(steps) = avg_steps {
        if steps < 5000.0 {
            matches.push(N3RuleMatch {
                pattern: "LowActivityFlag",
                confidence: "low",
                routing_lane: LANE_PASSTHROUGH,
                n3_source: "cardiovascular_risk.n3",
                recommended_action: None,
            });
        }
    }

    matches
}

/// Convenience: build a HealthStore from Turtle, run all N3 rules, return JSON.
pub fn evaluate_n3_rules_turtle(turtle: &str) -> String {
    let prefixes = crate::rdf::generate_rdf_prefixes();
    let full = format!("{}\n{}", prefixes, turtle);
    let mut store = match HealthStore::new() {
        Ok(s) => s,
        Err(e) => return format!("{{\"error\":\"{}\"}}", e),
    };
    if let Err(e) = store.load_turtle(&full) {
        return format!("{{\"error\":\"{}\"}}", e);
    }
    let results = evaluate_n3_rules(&store);
    let json_items: Vec<String> = results.iter().map(|r| r.to_json()).collect();
    format!("[{}]", json_items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf::generate_rdf_prefixes;

    fn store_with(data: &str) -> HealthStore {
        let prefixes = generate_rdf_prefixes();
        let full = format!("{}\n{}", prefixes, data);
        let mut s = HealthStore::new().unwrap();
        s.load_turtle(&full).unwrap();
        s
    }

    #[test]
    fn test_tachycardia_fires() {
        // Heart rate 110 BPM > 100 threshold
        let data = format!(
            "<urn:health:hr:1> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#snomedConcept> \
             <http://snomed.info/id/364075005> ; \
             <http://hl7.org/fhir/Observation.valueQuantity> 110 ."
        );
        let store = store_with(&data);
        let matches = evaluate_n3_rules(&store);
        assert!(
            matches.iter().any(|m| m.pattern == "TachycardiaFlag"),
            "TachycardiaFlag should fire for HR 110"
        );
    }

    #[test]
    fn test_sleep_debt_fires() {
        // Sleep efficiency 60% < 70 AND sleep hours 5.0 < 6
        let data = format!(
            "<urn:health:sleep:1> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#sleepEfficiency> 60 ; \
             <https://health.example.org/ns#sleepHours> 5.0 ."
        );
        let store = store_with(&data);
        let matches = evaluate_n3_rules(&store);
        assert!(
            matches.iter().any(|m| m.pattern == "ChronicSleepDebt"),
            "ChronicSleepDebt should fire for eff=60, hrs=5.0"
        );
    }

    #[test]
    fn test_normal_data_no_flags() {
        // Normal HR 65 BPM, good sleep efficiency 85%, 8 hours, 8000 steps
        let data = format!(
            "<urn:health:hr:ok> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#snomedConcept> <http://snomed.info/id/364075005> ; \
             <http://hl7.org/fhir/Observation.valueQuantity> 65 . \
             <urn:health:sleep:ok> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#sleepEfficiency> 85 ; \
             <https://health.example.org/ns#sleepHours> 8.0 . \
             <urn:health:steps:ok> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#snomedConcept> <http://snomed.info/id/256235009> ; \
             <http://hl7.org/fhir/Observation.valueQuantity> 8000 ."
        );
        let store = store_with(&data);
        let matches = evaluate_n3_rules(&store);
        let critical: Vec<_> = matches.iter().filter(|m| m.routing_lane == 2).collect();
        assert!(
            critical.is_empty(),
            "No BilateralMicroCommons rules should fire for normal data. Got: {:?}",
            critical.iter().map(|m| m.pattern).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_adrenal_fatigue_fires() {
        // Sleep 4.5h < 5 AND HR 95 > 90
        let data = format!(
            "<urn:health:sleep:1> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#sleepHours> 4.5 . \
             <urn:health:hr:1> a <http://hl7.org/fhir/Observation> ; \
             <https://health.example.org/ns#snomedConcept> <http://snomed.info/id/364075005> ; \
             <http://hl7.org/fhir/Observation.valueQuantity> 95 ."
        );
        let store = store_with(&data);
        let matches = evaluate_n3_rules(&store);
        assert!(
            matches
                .iter()
                .any(|m| m.pattern == "AdrenalFatigueSuspected"),
            "AdrenalFatigueSuspected should fire for sleep=4.5h, HR=95"
        );
    }
}
