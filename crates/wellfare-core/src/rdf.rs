use crate::models::{HeartRateRecord, SleepRecord, StepRecord, WeightRecord};
use serde_json::Value;

pub fn generate_rdf_prefixes() -> String {
    format!(
        "@prefix schema:   <http://schema.org/> .\n\
         @prefix health:   <https://health.example.org/ns#> .\n\
         @prefix wf:       <https://wellfare.social/ns/vault#> .\n\
         @prefix qudt:     <http://qudt.org/schema/qudt/> .\n\
         @prefix qudt-unit:<http://qudt.org/vocab/unit/> .\n\
         @prefix prov:     <http://www.w3.org/ns/prov#> .\n\
         @prefix xsd:      <http://www.w3.org/2001/XMLSchema#> .\n\
         @prefix rdf:      <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
         @prefix rdfs:     <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix sh:       <http://www.w3.org/ns/shacl#> .\n\
         @prefix snomed:   <http://snomed.info/id/> .\n\
         @prefix loinc:    <https://loinc.org/rdf/> .\n\
         @prefix fhir:     <http://hl7.org/fhir/> .\n\
         @prefix ccf:      <http://purl.org/ccf/> .\n\
         @prefix uberon:   <http://purl.obolibrary.org/obo/UBERON_> .\n\
         @prefix cl:       <http://purl.obolibrary.org/obo/CL_> .\n\
         @prefix hra:      <https://purl.humanatlas.io/> .\n\n"
    )
}

/// Serialize vault medication records (IDB wf-meds JSON array) to Turtle.
pub fn vault_meds_to_turtle(json: &str) -> Result<String, String> {
    let arr: Vec<Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = generate_rdf_prefixes();
    for rec in &arr {
        let id = rec["id"].as_str().unwrap_or("unknown");
        let name = rec["name"].as_str().unwrap_or("");
        let subj = format!("<urn:wf:med:{}>", id);
        out.push_str(&format!("{} a wf:MedicationRecord ;\n", subj));
        out.push_str(&format!("    schema:name {:?} ;\n", name));
        if let Some(v) = rec["dose"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:dose {:?} ;\n", v));
        }
        if let Some(v) = rec["route"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:route {:?} ;\n", v));
        }
        if let Some(v) = rec["frequency"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:frequency {:?} ;\n", v));
        }
        if let Some(v) = rec["started"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:startDate {:?}^^xsd:date ;\n", v));
        }
        if let Some(v) = rec["ceased"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:endDate {:?}^^xsd:date ;\n", v));
        }
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:vault> .\n\n");
    }
    Ok(out)
}

/// Serialize vault diet log entries (IDB wf-dl JSON array) to Turtle.
pub fn vault_diet_to_turtle(json: &str) -> Result<String, String> {
    let arr: Vec<Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = generate_rdf_prefixes();
    for rec in &arr {
        let id = rec["id"].as_str().unwrap_or("unknown");
        let name = rec["name"].as_str().unwrap_or("");
        let subj = format!("<urn:wf:diet:{}>", id);
        out.push_str(&format!("{} a wf:DietEntry ;\n", subj));
        out.push_str(&format!("    schema:name {:?} ;\n", name));
        if let Some(v) = rec["date"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:date {:?}^^xsd:date ;\n", v));
        }
        if let Some(v) = rec["meal"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("    wf:meal {:?} ;\n", v));
        }
        for (field, pred) in &[
            ("kcal", "wf:kcal"),
            ("protein_g", "wf:proteinG"),
            ("carbs_g", "wf:carbsG"),
            ("fat_g", "wf:fatG"),
            ("fiber_g", "wf:fiberG"),
            ("sugar_g", "wf:sugarG"),
            ("sodium_mg", "wf:sodiumMg"),
        ] {
            if let Some(v) = rec[*field].as_f64() {
                out.push_str(&format!("    {} {} ;\n", pred, v));
            }
        }
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:vault> .\n\n");
    }
    Ok(out)
}

/// Serialize vault biometric records (IDB wf-biometrics JSON array) to Turtle.
/// Each record carries { id, type: "weight"|"sleep"|"heart_rate"|"steps", date, value, ... }
pub fn vault_biometrics_to_turtle(json: &str) -> Result<String, String> {
    let arr: Vec<Value> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = generate_rdf_prefixes();
    for rec in &arr {
        let id = rec["id"].as_str().unwrap_or("unknown");
        let btype = rec["type"].as_str().unwrap_or("unknown");
        let subj = format!("<urn:wf:biometric:{}>", id);
        out.push_str(&format!(
            "{} a wf:BiometricRecord , fhir:Observation ;\n",
            subj
        ));
        out.push_str(&format!("    wf:biometricType {:?} ;\n", btype));
        if let Some(v) = rec["date"].as_str() {
            out.push_str(&format!(
                "    fhir:Observation.effectiveDateTime {:?}^^xsd:dateTime ;\n",
                v
            ));
        }
        if let Some(v) = rec["value"].as_f64() {
            out.push_str(&format!("    fhir:Observation.valueQuantity {} ;\n", v));
        }
        if let Some(v) = rec["unit"].as_str() {
            out.push_str(&format!("    wf:unit {:?} ;\n", v));
        }
        out.push_str("    prov:wasDerivedFrom <urn:health:source:samsung-health-export> ;\n");
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:vault> .\n\n");
    }
    Ok(out)
}

pub fn weight_to_turtle(records: &[WeightRecord]) -> String {
    let mut out = generate_rdf_prefixes();
    for rec in records {
        let subj = format!("<urn:health:weight:{}>", rec.uuid);
        out.push_str(&format!("{} a fhir:Observation ;\n", subj));
        out.push_str("    health:snomedConcept snomed:27113001 ;\n");
        out.push_str("    health:loincConcept loinc:29463-7 ;\n");
        out.push_str("    schema:name \"Body weight\" ;\n");
        out.push_str("    prov:wasDerivedFrom <urn:health:source:samsung-health-export> ;\n");
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> ;\n");
        out.push_str("    schema:description \"Samsung Health CSV export\" ;\n");
        out.push_str(&format!(
            "    fhir:Observation.effectiveDateTime \"{}\"^^xsd:dateTime ;\n",
            rec.start_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.valueQuantity {} ;\n",
            rec.weight
        ));
        out.push_str("    qudt:unit qudt-unit:Pound_Lb ;\n");
        out.push_str("    schema:unitCode \"Pound_Lb\" ;\n");
        out.push_str(
            "    health:semanticReference fhir:Observation.valueQuantity , snomed:27113001 .\n\n",
        );

        if let Some(body_fat) = rec.body_fat {
            out.push_str(&format!(
                "{} health:bodyFatPercentage {} ;\n",
                subj, body_fat
            ));
            out.push_str("    qudt:unit qudt-unit:Percent ;\n");
            out.push_str("    schema:unitCode \"Percent\" .\n\n");
        }
        if let Some(muscle_mass) = rec.muscle_mass {
            out.push_str(&format!("{} health:muscleMassKg {} ;\n", subj, muscle_mass));
            out.push_str("    qudt:unit qudt-unit:Kilogram ;\n");
            out.push_str("    schema:unitCode \"Kilogram\" .\n\n");
        }
        if let Some(body_water) = rec.body_water {
            out.push_str(&format!(
                "{} health:bodyWaterPercentage {} ;\n",
                subj, body_water
            ));
            out.push_str("    qudt:unit qudt-unit:Percent ;\n");
            out.push_str("    schema:unitCode \"Percent\" .\n\n");
        }
        if let Some(skeletal_muscle) = rec.skeletal_muscle {
            out.push_str(&format!(
                "{} health:skeletalMuscleMassKg {} ;\n",
                subj, skeletal_muscle
            ));
            out.push_str("    qudt:unit qudt-unit:Kilogram ;\n");
            out.push_str("    schema:unitCode \"Kilogram\" .\n\n");
        }

        // Add RDF-Star statement metadata
        out.push_str(&format!(
            "<< {} fhir:Observation.valueQuantity {} >> health:privacyMode \"MODE_B_PRIVILEGED\" ;\n\
             \tprov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> .\n\n",
            subj, rec.weight
        ));
    }
    out
}

pub fn sleep_to_turtle(records: &[SleepRecord]) -> String {
    let mut out = generate_rdf_prefixes();
    for rec in records {
        let subj = format!("<urn:health:sleep:{}>", rec.uuid);
        out.push_str(&format!("{} a fhir:Observation ;\n", subj));
        out.push_str("    health:snomedConcept snomed:248263006 ;\n");
        out.push_str("    schema:name \"Sleep session\" ;\n");
        out.push_str("    prov:wasDerivedFrom <urn:health:source:samsung-health-export> ;\n");
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> ;\n");
        out.push_str("    schema:description \"Samsung Health CSV export\" ;\n");
        out.push_str(&format!(
            "    fhir:Observation.effectivePeriod.start \"{}\"^^xsd:dateTime ;\n",
            rec.start_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.effectivePeriod.end \"{}\"^^xsd:dateTime ;\n",
            rec.end_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.valueQuantity \"PT{}M\"^^xsd:duration ;\n",
            rec.sleep_duration as u64
        ));
        out.push_str("    qudt:unit qudt-unit:Minute ;\n");
        out.push_str(&format!(
            "    health:sleepHours {} ;\n",
            rec.sleep_duration / 60.0
        ));
        out.push_str(&format!(
            "    health:sleepEfficiency {} ;\n",
            rec.efficiency
        ));
        out.push_str("    health:semanticReference fhir:Observation.valueQuantity .\n\n");

        if let Some(deep) = rec.deep_sleep {
            out.push_str(&format!("{} health:deepSleepMinutes {} ;\n", subj, deep));
            out.push_str("    qudt:unit qudt-unit:Minute .\n\n");
        }
        if let Some(rem) = rec.rem_sleep {
            out.push_str(&format!("{} health:remSleepMinutes {} ;\n", subj, rem));
            out.push_str("    qudt:unit qudt-unit:Minute .\n\n");
        }
        if let Some(light) = rec.light_sleep {
            out.push_str(&format!("{} health:lightSleepMinutes {} ;\n", subj, light));
            out.push_str("    qudt:unit qudt-unit:Minute .\n\n");
        }
        if let Some(wuc) = rec.wake_up_count {
            out.push_str(&format!("{} health:wakeUpCount {} .\n\n", subj, wuc));
        }

        // Add RDF-Star statement for sleep efficiency annotation
        out.push_str(&format!(
            "<< {} health:sleepEfficiency {} >> health:privacyMode \"MODE_B_PRIVILEGED\" ;\n\
             \tprov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> .\n\n",
            subj, rec.efficiency
        ));
    }
    out
}

pub fn heart_rate_to_turtle(records: &[HeartRateRecord]) -> String {
    let mut out = generate_rdf_prefixes();
    for rec in records {
        let subj = format!("<urn:health:heart_rate:{}>", rec.uuid);
        out.push_str(&format!("{} a fhir:Observation ;\n", subj));
        out.push_str("    health:snomedConcept snomed:364075005 ;\n");
        out.push_str("    health:loincConcept loinc:8867-4 ;\n");
        out.push_str("    schema:name \"Heart rate\" ;\n");
        out.push_str("    prov:wasDerivedFrom <urn:health:source:samsung-health-export> ;\n");
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> ;\n");
        out.push_str("    schema:description \"Samsung Health CSV export\" ;\n");
        out.push_str(&format!(
            "    fhir:Observation.effectiveDateTime \"{}\"^^xsd:dateTime ;\n",
            rec.start_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.valueQuantity {} ;\n",
            rec.heart_rate
        ));
        out.push_str("    qudt:unit qudt-unit:BeatsPerMinute ;\n");
        out.push_str("    health:semanticReference fhir:Observation.valueQuantity , snomed:364075005 , loinc:8867-4 .\n\n");

        if let Some(min) = rec.min {
            out.push_str(&format!("{} health:minHeartRate {} ;\n", subj, min));
            out.push_str("    qudt:unit qudt-unit:BeatsPerMinute .\n\n");
        }
        if let Some(max) = rec.max {
            out.push_str(&format!("{} health:maxHeartRate {} ;\n", subj, max));
            out.push_str("    qudt:unit qudt-unit:BeatsPerMinute .\n\n");
        }
        if let Some(ref zone) = rec.heart_rate_zone {
            out.push_str(&format!("{} health:heartRateZone \"{}\" .\n\n", subj, zone));
        }

        // Add RDF-Star statement for heart rate annotation
        out.push_str(&format!(
            "<< {} fhir:Observation.valueQuantity {} >> health:privacyMode \"MODE_B_PRIVILEGED\" ;\n\
             \tprov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> .\n\n",
            subj, rec.heart_rate
        ));
    }
    out
}

pub fn steps_to_turtle(records: &[StepRecord]) -> String {
    let mut out = generate_rdf_prefixes();
    for rec in records {
        let subj = format!("<urn:health:steps:{}>", rec.uuid);
        out.push_str(&format!("{} a fhir:Observation ;\n", subj));
        out.push_str("    health:snomedConcept snomed:256235009 ;\n");
        out.push_str("    health:loincConcept loinc:55423-8 ;\n");
        out.push_str("    schema:name \"Daily step count\" ;\n");
        out.push_str("    prov:wasDerivedFrom <urn:health:source:samsung-health-export> ;\n");
        out.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> ;\n");
        out.push_str("    schema:description \"Samsung Health CSV export\" ;\n");
        out.push_str(&format!(
            "    fhir:Observation.effectivePeriod.start \"{}\"^^xsd:dateTime ;\n",
            rec.start_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.effectivePeriod.end \"{}\"^^xsd:dateTime ;\n",
            rec.end_datetime.to_rfc3339()
        ));
        out.push_str(&format!(
            "    fhir:Observation.valueQuantity {} ;\n",
            rec.count
        ));
        out.push_str("    qudt:unit qudt-unit:Unitless ;\n");
        out.push_str("    health:semanticReference fhir:Observation.valueQuantity .\n\n");

        out.push_str(&format!(
            "{} health:caloriesBurned {} ;\n",
            subj, rec.calorie
        ));
        out.push_str("    qudt:unit qudt-unit:Kilocalorie .\n\n");

        out.push_str(&format!("{} health:distance {} ;\n", subj, rec.distance));
        out.push_str("    qudt:unit qudt-unit:Meter .\n\n");

        // Add RDF-Star statement for step count annotation
        out.push_str(&format!(
            "<< {} fhir:Observation.valueQuantity {} >> health:privacyMode \"MODE_B_PRIVILEGED\" ;\n\
             \tprov:wasGeneratedBy <urn:wellfair:agent:health-to-solid> .\n\n",
            subj, rec.count
        ));
    }
    out
}
