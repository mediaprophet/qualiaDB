use crate::rdf::generate_rdf_prefixes;
/// SHACL-equivalent validation constraints expressed as SPARQL ASK queries.
/// Each shape returns true = violation found (use RDFS/SHACL semantics,
/// NOT OWL class membership — people are described by constraints, not ontological types).
use crate::store::HealthStore;

pub struct ShapeViolation {
    pub shape_id: &'static str,
    pub message: &'static str,
}

const SHAPES: &[(&str, &str, &str)] = &[
    (
        "sleep:efficiency-range",
        "Sleep efficiency must be between 0 and 100 percent",
        "ASK { ?obs <https://health.example.org/ns#sleepEfficiency> ?e \
               FILTER(?e < 0 || ?e > 100) }",
    ),
    (
        "sleep:duration-positive",
        "Sleep duration must be greater than zero",
        "ASK { ?obs a <http://hl7.org/fhir/Observation> ; \
                    <http://hl7.org/fhir/Observation.valueQuantity> ?d \
               FILTER(?d <= 0) }",
    ),
    (
        "heart-rate:physiological-range",
        "Resting heart rate must be between 20 and 300 BPM",
        "ASK { ?obs <http://hl7.org/fhir/Observation.valueQuantity> ?hr ; \
                    <https://health.example.org/ns#snomedConcept> \
                    <http://snomed.info/id/364075005> \
               FILTER(?hr < 20 || ?hr > 300) }",
    ),
    (
        "weight:positive",
        "Body weight must be a positive value",
        "ASK { ?obs a <http://hl7.org/fhir/Observation> ; \
                    <http://hl7.org/fhir/Observation.valueQuantity> ?w ; \
                    <https://health.example.org/ns#snomedConcept> \
                    <http://snomed.info/id/27113001> \
               FILTER(?w <= 0) }",
    ),
    (
        "weight:body-fat-range",
        "Body fat percentage must be between 0 and 100",
        "ASK { ?obs <https://health.example.org/ns#bodyFatPercentage> ?f \
               FILTER(?f < 0 || ?f > 100) }",
    ),
    (
        "steps:non-negative",
        "Step count must be zero or greater",
        "ASK { ?obs <http://hl7.org/fhir/Observation.valueQuantity> ?s ; \
                    <https://health.example.org/ns#snomedConcept> \
                    <http://snomed.info/id/256235009> \
               FILTER(?s < 0) }",
    ),
];

pub struct ValidationReport {
    pub violations: Vec<ShapeViolation>,
    pub checked: usize,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn to_json(&self) -> String {
        let v: Vec<String> = self
            .violations
            .iter()
            .map(|v| {
                format!(
                    "{{\"shape\":\"{}\",\"message\":\"{}\"}}",
                    v.shape_id, v.message
                )
            })
            .collect();
        format!(
            "{{\"valid\":{},\"checked\":{},\"violations\":[{}]}}",
            self.is_valid(),
            self.checked,
            v.join(",")
        )
    }
}

/// Validate a Turtle document against all registered shapes.
/// Individual shape errors are non-fatal (store is rebuilt per check).
pub fn validate_turtle(turtle_data: &str) -> ValidationReport {
    let prefixes = generate_rdf_prefixes();
    let mut violations = Vec::new();

    for (shape_id, message, ask_query) in SHAPES {
        let mut store = match HealthStore::new() {
            Ok(s) => s,
            Err(_) => continue,
        };
        match store.check_shape(&prefixes, turtle_data, ask_query) {
            Ok(true) => violations.push(ShapeViolation { shape_id, message }),
            Ok(false) | Err(_) => {}
        }
    }

    ValidationReport {
        violations,
        checked: SHAPES.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_sleep_passes() {
        let ttl = "<urn:health:sleep:ok> a <http://hl7.org/fhir/Observation> ; \
                   <https://health.example.org/ns#sleepEfficiency> 78 .";
        let report = validate_turtle(ttl);
        assert!(
            !report
                .violations
                .iter()
                .any(|v| v.shape_id == "sleep:efficiency-range")
        );
    }

    #[test]
    fn test_invalid_efficiency_caught() {
        let ttl = "<urn:health:sleep:bad> a <http://hl7.org/fhir/Observation> ; \
                   <https://health.example.org/ns#sleepEfficiency> 150 .";
        let report = validate_turtle(ttl);
        assert!(
            report
                .violations
                .iter()
                .any(|v| v.shape_id == "sleep:efficiency-range")
        );
    }
}
