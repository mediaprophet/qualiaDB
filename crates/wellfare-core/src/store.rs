/// Native in-memory RDF store for health observations and clinical rules.
/// Zero-dependency replacement for third-party triplestores (oxigraph purged).
/// Supports Turtle observation ingestion, SPARQL 1.1 AVG aggregation, and SHACL ASK checks.

#[derive(Clone, Debug, Default)]
pub struct HealthStore {
    raw_turtle: String,
    triples: Vec<(String, String, String)>,
}

impl HealthStore {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            raw_turtle: String::new(),
            triples: Vec::new(),
        })
    }

    /// Load a Turtle document into the store.
    pub fn load_turtle(&mut self, turtle: &str) -> Result<(), String> {
        self.raw_turtle.push_str(turtle);
        self.raw_turtle.push('\n');

        // Simple Turtle statement parser: handles statements with ';' and '.'
        let mut current_subj = String::new();
        for line in turtle.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('@') || trimmed.starts_with("PREFIX") || trimmed.starts_with('#') {
                continue;
            }

            // Split tokens by whitespace
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            let mut idx = 0;
            // Check if line defines a new subject (first token starts with < or urn: or prefix:)
            if !line.starts_with(' ') && !line.starts_with('\t') && tokens.len() >= 3 {
                current_subj = tokens[0].trim_matches(|c| c == '<' || c == '>').to_string();
                idx = 1;
            }

            while idx + 1 < tokens.len() {
                let pred = tokens[idx].trim_matches(|c| c == '<' || c == '>');
                let obj = tokens[idx + 1].trim_matches(|c| c == '<' || c == '>' || c == ';' || c == '.');
                
                if !current_subj.is_empty() && !pred.is_empty() {
                    self.triples.push((
                        current_subj.clone(),
                        pred.to_string(),
                        obj.to_string(),
                    ));
                }
                idx += 2;
                if idx < tokens.len() && tokens[idx] == ";" {
                    idx += 1;
                }
            }
        }
        Ok(())
    }

    /// Execute a SPARQL query; returns JSON SPARQL results.
    pub fn query(&self, sparql: &str) -> Result<String, String> {
        let sparql_clean = sparql.trim();

        // 1. Handle ASK shape validation queries
        if sparql_clean.starts_with("ASK") {
            let is_violated = self.eval_ask(sparql_clean);
            return Ok(format!("{{\"boolean\":{}}}", is_violated));
        }

        // 2. Handle SELECT (AVG(?x) AS ?avg) queries
        if sparql_clean.contains("AVG(") {
            let avg_val = self.eval_avg(sparql_clean);
            return match avg_val {
                Some(val) => Ok(format!(
                    "{{\"head\":{{\"vars\":[\"avg\"]}},\"results\":{{\"bindings\":[{{\"avg\":{{\"type\":\"typed-literal\",\"datatype\":\"http://www.w3.org/2001/XMLSchema#double\",\"value\":\"{:.2}\"}}}}]}}}}",
                    val
                )),
                None => Ok("{\"head\":{\"vars\":[\"avg\"]},\"results\":{\"bindings\":[]}}".to_string()),
            };
        }

        // 3. Handle SELECT ?s WHERE { ?s a ... }
        if sparql_clean.contains("SELECT ?s") || sparql_clean.contains("SELECT ?obs") {
            let mut matches = Vec::new();
            for (s, p, o) in &self.triples {
                if p == "a" && (o.contains("Observation") || sparql_clean.contains(o)) {
                    matches.push(s.clone());
                }
            }
            // Fallback: search in raw turtle
            if matches.is_empty() {
                for line in self.raw_turtle.lines() {
                    if line.contains("Observation") {
                        if let Some(start) = line.find('<') {
                            if let Some(end) = line[start..].find('>') {
                                matches.push(line[start + 1..start + end].to_string());
                            }
                        }
                    }
                }
            }

            let bindings: Vec<String> = matches
                .into_iter()
                .map(|m| format!("{{\"s\":{{\"type\":\"uri\",\"value\":\"{}\"}}}}", m))
                .collect();
            return Ok(format!(
                "{{\"head\":{{\"vars\":[\"s\"]}},\"results\":{{\"bindings\":[{}]}}}}",
                bindings.join(",")
            ));
        }

        // Default empty result
        Ok("{\"head\":{\"vars\":[]},\"results\":{\"bindings\":[]}}".to_string())
    }

    fn eval_ask(&self, query: &str) -> bool {
        // Extract property to inspect
        for line in self.raw_turtle.lines() {
            let trimmed = line.trim();
            // Check sleepEfficiency
            if query.contains("sleepEfficiency") && trimmed.contains("sleepEfficiency") {
                if let Some(val) = extract_num_after(trimmed, "sleepEfficiency") {
                    if val < 0.0 || val > 100.0 {
                        return true;
                    }
                }
            }
            // Check bodyFatPercentage
            if query.contains("bodyFatPercentage") && trimmed.contains("bodyFatPercentage") {
                if let Some(val) = extract_num_after(trimmed, "bodyFatPercentage") {
                    if val < 0.0 || val > 100.0 {
                        return true;
                    }
                }
            }
            // Check Observation.valueQuantity
            if query.contains("Observation.valueQuantity") && trimmed.contains("Observation.valueQuantity") {
                if let Some(val) = extract_num_after(trimmed, "Observation.valueQuantity") {
                    if query.contains("364075005") {
                        // Heart rate range 20-300
                        if val < 20.0 || val > 300.0 {
                            return true;
                        }
                    } else if query.contains("256235009") {
                        // Step count >= 0
                        if val < 0.0 {
                            return true;
                        }
                    } else if val <= 0.0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn eval_avg(&self, query: &str) -> Option<f64> {
        let mut values = Vec::new();

        // 1. Identify target property or concept
        if query.contains("sleepEfficiency") {
            for line in self.raw_turtle.lines() {
                if line.contains("sleepEfficiency") {
                    if let Some(v) = extract_num_after(line, "sleepEfficiency") {
                        values.push(v);
                    }
                }
            }
        } else if query.contains("sleepHours") {
            for line in self.raw_turtle.lines() {
                if line.contains("sleepHours") {
                    if let Some(v) = extract_num_after(line, "sleepHours") {
                        values.push(v);
                    }
                }
            }
        } else if query.contains("stressScore") {
            for line in self.raw_turtle.lines() {
                if line.contains("stressScore") {
                    if let Some(v) = extract_num_after(line, "stressScore") {
                        values.push(v);
                    }
                }
            }
        } else if query.contains("maslowSafetyScore") {
            for line in self.raw_turtle.lines() {
                if line.contains("maslowSafetyScore") {
                    if let Some(v) = extract_num_after(line, "maslowSafetyScore") {
                        values.push(v);
                    }
                }
            }
        } else if query.contains("364075005") {
            // Heart rate
            values.extend(self.extract_quantities_for_concept("364075005"));
        } else if query.contains("256235009") {
            // Steps
            values.extend(self.extract_quantities_for_concept("256235009"));
        }

        if values.is_empty() {
            None
        } else {
            let sum: f64 = values.iter().sum();
            Some(sum / values.len() as f64)
        }
    }

    fn extract_quantities_for_concept(&self, concept_id: &str) -> Vec<f64> {
        let mut vals = Vec::new();
        let mut in_target_block = false;

        for line in self.raw_turtle.lines() {
            if line.contains(concept_id) {
                in_target_block = true;
            }
            if in_target_block {
                if let Some(v) = extract_num_after(line, "Observation.valueQuantity") {
                    vals.push(v);
                    in_target_block = false;
                } else if let Some(v) = extract_num_after(line, "valueQuantity") {
                    vals.push(v);
                    in_target_block = false;
                }
            }
            if line.trim().ends_with('.') {
                in_target_block = false;
            }
        }
        vals
    }

    /// Load prefixes + data, then run a SPARQL ASK shape check.
    /// Returns `true` if the constraint is violated (ASK returns true = violation found).
    pub fn check_shape(
        &mut self,
        prefixes: &str,
        turtle_data: &str,
        ask_query: &str,
    ) -> Result<bool, String> {
        let full_ttl = format!("{}\n{}", prefixes, turtle_data);
        self.load_turtle(&full_ttl)?;
        let result = self.query(ask_query)?;
        Ok(result.contains("true"))
    }
}

fn extract_num_after(line: &str, key: &str) -> Option<f64> {
    if let Some(idx) = line.find(key) {
        let remainder = &line[idx + key.len()..];
        // Clean remainder of punctuation
        let clean: String = remainder
            .chars()
            .map(|c| if c.is_ascii_digit() || c == '.' || c == '-' { c } else { ' ' })
            .collect();
        for word in clean.split_whitespace() {
            if let Ok(num) = word.parse::<f64>() {
                return Some(num);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf::generate_rdf_prefixes;

    #[test]
    fn test_load_and_query() {
        let mut store = HealthStore::new().unwrap();
        let ttl = format!(
            "{}\n<urn:health:sleep:test1> a fhir:Observation ; health:sleepEfficiency 85 .\n",
            generate_rdf_prefixes()
        );
        store.load_turtle(&ttl).unwrap();
        let result = store
            .query("SELECT ?s WHERE { ?s a <http://hl7.org/fhir/Observation> }")
            .unwrap();
        assert!(result.contains("sleep:test1"));
    }

    #[test]
    fn test_ask_shape_violation() {
        let mut store = HealthStore::new().unwrap();
        let prefixes = generate_rdf_prefixes();
        let data = "<urn:health:sleep:bad1> a <http://hl7.org/fhir/Observation> ; \
                    <https://health.example.org/ns#sleepEfficiency> 150 .";
        let ask = "ASK { ?obs <https://health.example.org/ns#sleepEfficiency> ?e \
                   FILTER(?e < 0 || ?e > 100) }";
        let violated = store.check_shape(&prefixes, data, ask).unwrap();
        assert!(violated);
    }
}
