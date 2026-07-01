/// In-memory RDF store backed by oxigraph.
/// Accepts Turtle serialised by rdf.rs, executes SPARQL 1.1 SELECT/ASK/CONSTRUCT.
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::results::{QueryResultsFormat, QueryResultsSerializer};
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

pub struct HealthStore {
    inner: Store,
}

impl HealthStore {
    pub fn new() -> Result<Self, String> {
        Store::new()
            .map(|inner| Self { inner })
            .map_err(|e| e.to_string())
    }

    /// Load a Turtle document into the default graph.
    pub fn load_turtle(&mut self, turtle: &str) -> Result<(), String> {
        let parser = RdfParser::from_format(RdfFormat::Turtle);
        self.inner
            .load_from_reader(parser, turtle.as_bytes())
            .map_err(|e| e.to_string())
    }

    /// Execute a SPARQL SELECT, ASK, or CONSTRUCT query; returns JSON SPARQL results.
    pub fn query(&self, sparql: &str) -> Result<String, String> {
        let results = SparqlEvaluator::new()
            .parse_query(sparql)
            .map_err(|e| e.to_string())?
            .on_store(&self.inner)
            .execute()
            .map_err(|e| e.to_string())?;

        match results {
            QueryResults::Solutions(solutions) => {
                let mut buf = Vec::new();
                let variables = solutions.variables().to_vec();
                let mut writer = QueryResultsSerializer::from_format(QueryResultsFormat::Json)
                    .serialize_solutions_to_writer(&mut buf, variables)
                    .map_err(|e| e.to_string())?;
                for solution in solutions {
                    writer
                        .serialize(solution.map_err(|e| e.to_string())?.iter())
                        .map_err(|e| e.to_string())?;
                }
                writer.finish().map_err(|e| e.to_string())?;
                String::from_utf8(buf).map_err(|e| e.to_string())
            }
            QueryResults::Boolean(b) => Ok(format!("{{\"boolean\":{}}}", b)),
            QueryResults::Graph(triples) => {
                let mut buf = Vec::new();
                for triple in triples {
                    let t = triple.map_err(|e| e.to_string())?;
                    buf.extend_from_slice(format!("{} .\n", t).as_bytes());
                }
                String::from_utf8(buf).map_err(|e| e.to_string())
            }
        }
    }

    /// Load prefixes + data, then run a SPARQL ASK shape check.
    /// Returns `true` if the constraint is violated (ASK returns true = violation found).
    pub fn check_shape(&mut self, prefixes: &str, turtle_data: &str, ask_query: &str) -> Result<bool, String> {
        let full_ttl = format!("{}\n{}", prefixes, turtle_data);
        self.load_turtle(&full_ttl)?;
        let result = self.query(ask_query)?;
        Ok(result.contains("true"))
    }
}

impl Default for HealthStore {
    fn default() -> Self {
        Self::new().expect("oxigraph store init failed")
    }
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
