//! SPARQL HTTP Endpoint
//!
//! Provides HTTP endpoint for SPARQL queries (/sparql)

use crate::sparql_ast::*;
use crate::sparql_parser;
use crate::sparql_planner::*;
use crate::sparql_executor::*;
use crate::sparql_results::ResultFormatter;
use crate::NQuin;

/// SPARQL HTTP endpoint handler
pub struct SparqlEndpoint {
    quins: Vec<NQuin>,
}

impl SparqlEndpoint {
    pub fn new(quins: Vec<NQuin>) -> Self {
        Self { quins }
    }

    /// Handle SPARQL query via HTTP
    pub fn handle_query(&self, query: &str, format: &str) -> Result<String, String> {
        // Parse query
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;
        
        // Plan query
        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        
        // Execute query
        let executor = QueryExecutor::new(&self.quins);
        let results = executor.execute(&plan, &ctx)?;
        
        // Format results
        match format.to_lowercase().as_str() {
            "xml" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_xml(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "json" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_json(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "tsv" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_tsv(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "csv" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_csv(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format. Use: xml, json, tsv, or csv".to_string()),
        }
    }

    /// Handle ASK query
    pub fn handle_ask(&self, query: &str, format: &str) -> Result<String, String> {
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;

        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        let executor = QueryExecutor::new(&self.quins);
        let has_results = executor.execute_ask(&plan, &ctx)?;

        match format.to_lowercase().as_str() {
            "xml" => {
                let mut output = Vec::new();
                ResultFormatter::format_ask_xml(&mut output, has_results).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "json" => {
                let mut output = Vec::new();
                ResultFormatter::format_ask_json(&mut output, has_results).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format for ASK. Use: xml or json".to_string()),
        }
    }

    /// Handle CONSTRUCT query
    pub fn handle_construct(&self, query: &str, format: &str) -> Result<String, String> {
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;

        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        let executor = QueryExecutor::new(&self.quins);
        let results = executor.execute_construct(&plan, &ctx)?;

        // For now, format as SELECT results (simplified - no template variables)
        match format.to_lowercase().as_str() {
            "xml" => {
                let mut output = Vec::new();
                // Use default variables for CONSTRUCT
                let vars = vec![0u8, 1u8, 2u8]; // subject, predicate, object
                ResultFormatter::format_xml(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "json" => {
                let mut output = Vec::new();
                let vars = vec![0u8, 1u8, 2u8];
                ResultFormatter::format_json(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format. Use: xml or json".to_string()),
        }
    }

    /// Handle DESCRIBE query
    pub fn handle_describe(&self, query: &str, format: &str) -> Result<String, String> {
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;

        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        let executor = QueryExecutor::new(&self.quins);
        let results = executor.execute_describe(&plan, &ctx)?;

        // For now, format as SELECT results (simplified)
        match format.to_lowercase().as_str() {
            "xml" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Describe(describe) => {
                        // Use first variable
                        if describe.var_count > 0 {
                            vec![describe.vars_or_ids[0] as VariableId]
                        } else {
                            vec![0]
                        }
                    }
                    _ => vec![],
                };
                ResultFormatter::format_xml(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "json" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Describe(describe) => {
                        if describe.var_count > 0 {
                            vec![describe.vars_or_ids[0] as VariableId]
                        } else {
                            vec![0]
                        }
                    }
                    _ => vec![],
                };
                ResultFormatter::format_json(&mut output, &vars, &results, &ctx).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format. Use: xml or json".to_string()),
        }
    }
}

/// SPARQL protocol handler
pub struct SparqlProtocolHandler {
    endpoint: SparqlEndpoint,
}

impl SparqlProtocolHandler {
    pub fn new(quins: Vec<NQuin>) -> Self {
        Self {
            endpoint: SparqlEndpoint::new(quins),
        }
    }

    /// Parse Content-Type header to determine format
    pub fn parse_accept_header(accept: &str) -> String {
        if accept.contains("application/sparql-results+xml") {
            "xml".to_string()
        } else if accept.contains("application/sparql-results+json") {
            "json".to_string()
        } else if accept.contains("text/tab-separated-values") {
            "tsv".to_string()
        } else if accept.contains("text/csv") {
            "csv".to_string()
        } else {
            "json".to_string() // Default
        }
    }

    /// Handle SPARQL protocol request
    pub fn handle_request(&self, query: Option<&str>, accept: Option<&str>) -> Result<String, String> {
        let query = query.ok_or("No query provided")?;
        let format = accept.map(|a| Self::parse_accept_header(a)).unwrap_or_else(|| "json".to_string());
        
        // Check query type and dispatch
        let query_upper = query.trim().to_uppercase();
        
        if query_upper.starts_with("ASK") {
            self.endpoint.handle_ask(query, &format)
        } else if query_upper.starts_with("CONSTRUCT") {
            self.endpoint.handle_construct(query, &format)
        } else if query_upper.starts_with("DESCRIBE") {
            self.endpoint.handle_describe(query, &format)
        } else {
            // Default to SELECT
            self.endpoint.handle_query(query, &format)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_header() {
        assert_eq!(SparqlProtocolHandler::parse_accept_header("application/sparql-results+xml"), "xml");
        assert_eq!(SparqlProtocolHandler::parse_accept_header("application/sparql-results+json"), "json");
        assert_eq!(SparqlProtocolHandler::parse_accept_header("text/tab-separated-values"), "tsv");
        assert_eq!(SparqlProtocolHandler::parse_accept_header("text/csv"), "csv");
    }
}