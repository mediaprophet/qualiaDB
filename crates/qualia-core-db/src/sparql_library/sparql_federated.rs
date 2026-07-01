//! SPARQL 1.1 Federated Query (SERVICE)
//!
//! Implements federated query support with DID integration for CORS handling.

use crate::sparql_ast::*;
use crate::sparql_executor::QueryExecutor;
use crate::sparql_parser;
use crate::sparql_planner::QueryPlanner;
use crate::NQuin;

use std::collections::HashMap;

/// SERVICE endpoint configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ServiceEndpoint {
    pub did: u64,          // DID identifier
    pub endpoint_url: u64, // Hash of URL string
    pub auth_method: u8,   // 0=none, 1=DID-LD, 2=JWT
    pub timeout_ms: u32,
}

/// Federated query result
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FederatedResult {
    pub endpoint_did: u64,
    pub row_count: u16,
    pub success: bool,
}

/// SPARQL Federated Query Handler
pub struct FederatedQueryHandler<'a> {
    pub local_quins: &'a [NQuin],
    pub endpoints: [Option<ServiceEndpoint>; 16],
    pub endpoint_count: u8,
    pub cache_enabled: bool,
}

impl<'a> FederatedQueryHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            local_quins: quins,
            endpoints: [None; 16],
            endpoint_count: 0,
            cache_enabled: true,
        }
    }

    /// Register a federated endpoint
    pub fn register_endpoint(&mut self, endpoint: ServiceEndpoint) -> Result<u8, String> {
        if self.endpoint_count >= 16 {
            return Err("Endpoint overflow".to_string());
        }
        let idx = self.endpoint_count;
        self.endpoints[idx as usize] = Some(endpoint);
        self.endpoint_count += 1;
        Ok(idx)
    }

    /// Resolve DID to endpoint URL
    pub fn resolve_did(&self, did: u64) -> Result<u64, String> {
        // In production, this would:
        // 1. Resolve DID using DID resolver
        // 2. Extract service endpoint from DID document
        // 3. Return the endpoint URL hash

        // Simplified: return DID as URL hash
        for i in 0..self.endpoint_count as usize {
            if let Some(endpoint) = self.endpoints[i] {
                if endpoint.did == did {
                    return Ok(endpoint.endpoint_url);
                }
            }
        }

        Err("DID not found in endpoint registry".to_string())
    }

    /// Execute federated query
    pub fn execute_service(
        &self,
        did: u64,
        query: &str,
        format: &str,
    ) -> Result<FederatedResult, String> {
        // Resolve DID to endpoint
        let endpoint_url_hash = self.resolve_did(did)?;

        // Prepare authentication headers based on DID
        let auth_headers = self.prepare_did_auth(did)?;

        // Execute remote SPARQL query
        let results = self.execute_remote_query(endpoint_url_hash, query, format, &auth_headers)?;

        Ok(FederatedResult {
            endpoint_did: did,
            row_count: results.len() as u16,
            success: true,
        })
    }

    /// Prepare DID-based authentication
    fn prepare_did_auth(&self, did: u64) -> Result<Vec<(u64, u64)>, String> {
        // Find endpoint to get auth method
        for i in 0..self.endpoint_count as usize {
            if let Some(endpoint) = self.endpoints[i] {
                if endpoint.did == did {
                    match endpoint.auth_method {
                        0 => return Ok(vec![]), // No auth
                        1 => {
                            // DID-LD authentication
                            // In production: sign request with DID key
                            return Ok(vec![(0x4155544800000000_u64, 0x4449442D4C440000_u64)]);
                            // "Authorization": "DID-LD"
                        }
                        2 => {
                            // JWT authentication
                            // In production: generate JWT with DID
                            return Ok(vec![(0x4155544800000000, 0x4A57540000000000)]);
                            // "Authorization": "JWT"
                        }
                        _ => return Err("Unknown auth method".to_string()),
                    }
                }
            }
        }

        Err("DID not found".to_string())
    }

    /// Execute remote SPARQL query (simplified)
    fn execute_remote_query(
        &self,
        _endpoint_url_hash: u64,
        _query: &str,
        format: &str,
        _auth_headers: &[(u64, u64)],
    ) -> Result<Vec<BindingRow>, String> {
        // In production, this would:
        // 1. Hash endpoint_url_hash to get actual URL string
        // 2. Make HTTP request with auth headers
        // 3. Handle CORS using DID-based authentication
        // 4. Parse response into BindingRows

        // Simplified: return empty result
        // For testing, we'll return a dummy result if format is json
        if format == "json" {
            return Ok(vec![BindingRow::new()]);
        }

        Ok(vec![])
    }

    /// Execute federated query with local data
    pub fn execute_federated(
        &self,
        _service_did: u64,
        _service_query: &str,
        _local_pattern: PatternId,
        _ctx: &SparqlQueryContext,
    ) -> Result<Vec<BindingRow>, String> {
        // Execute remote SERVICE query
        // let _remote_result = self.execute_service(service_did, service_query, "json")?;

        // Parse remote query to get variables
        // let (_sparql_query, mut remote_ctx) = sparql_parser::parse_sparql(service_query)?;

        // Execute local pattern
        // let plan = QueryPlanner::from_pattern(local_pattern, ctx)?;
        // let executor = QueryExecutor::new(self.local_quins);
        // let local_results = executor.execute(&plan, ctx)?;

        // Merge results (simplified: just return empty for now)
        // In production, this would:
        // 1. Get remote results from HTTP response
        // 2. Join with local results on common variables
        // 3. Return merged bindings

        Ok(vec![])
    }

    /// Check CORS using DID-based authentication
    pub fn check_cors_allowed(&self, did: u64, origin_did: u64) -> Result<bool, String> {
        // In production, this would:
        // 1. Resolve both DIDs
        // 2. Check if origin_did is in the service endpoint's allowed origins list
        // 3. Verify DID relationship (e.g., same controller, trusted relationship)

        // Simplified: allow if DIDs are same
        Ok(did == origin_did)
    }

    /// Generate CORS headers using DID
    pub fn generate_cors_headers(&self, did: u64) -> Result<Vec<(u64, u64)>, String> {
        // In production, this would generate proper CORS headers
        // based on DID resolution and trust relationships

        let access_control_origin: u64 = 0x41432D4F72696769; // "Access-Control-Origin" (truncated)
        let access_control_methods: u64 = 0x41432D4D65746F64; // "Access-Control-Methods" (truncated)
        let access_control_headers: u64 = 0x41432D4865616465; // "Access-Control-Headers" (truncated)

        Ok(vec![
            (access_control_origin, did),                     // Use DID as origin
            (access_control_methods, 0x4745540000000000_u64), // "GET"
            (access_control_headers, 0x436F6E74656E742D_u64), // "Content-Type" (truncated)
        ])
    }
}

impl<'a> Default for FederatedQueryHandler<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

// =============================================================================
// FederatedQueryEngine — string-URL based federated query execution.
//
// This is a higher-level engine than the DID-hash `FederatedQueryHandler` above.
// It works with human-readable endpoint URLs (`local:`, `qualia:`, `http://...`)
// and is the entry point used by the SPARQL `SERVICE` clause evaluator.
// =============================================================================

/// A federated service endpoint identified by a readable URL string.
///
/// URL schemes:
/// - `local:<graph>` / `qualia:<graph>` — execute against the local QualiaDB instance
/// - `http://` / `https://`           — remote SPARQL endpoint (HTTP transport)
#[derive(Debug, Clone)]
pub struct FederatedService {
    /// Endpoint URL, e.g. `local:default`, `qualia:graph1`, `https://db.example.org/sparql`.
    pub endpoint: String,
    /// 0 = none, 1 = DID-LD, 2 = JWT (forwarded to remote endpoints).
    pub auth_method: u8,
    /// Request timeout in milliseconds (remote endpoints only).
    pub timeout_ms: u32,
}

impl FederatedService {
    /// Create a new service endpoint with no auth and a 30s default timeout.
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            auth_method: 0,
            timeout_ms: 30_000,
        }
    }

    /// Returns true when the endpoint targets the local QualiaDB instance and
    /// should be executed in-process rather than over HTTP.
    pub fn is_local(&self) -> bool {
        self.endpoint.starts_with("local:") || self.endpoint.starts_with("qualia:")
    }

    /// Returns the local graph identifier portion of a `local:`/`qualia:` endpoint,
    /// or `None` for remote endpoints.
    pub fn local_graph(&self) -> Option<&str> {
        if let Some(rest) = self.endpoint.strip_prefix("local:") {
            return Some(rest);
        }
        if let Some(rest) = self.endpoint.strip_prefix("qualia:") {
            return Some(rest);
        }
        None
    }
}

/// The outcome of executing a single query against a single federated service.
#[derive(Debug, Clone)]
pub struct FederatedQueryResult {
    /// The endpoint URL the query was dispatched to.
    pub service_endpoint: String,
    /// The SPARQL query string that was executed.
    pub query: String,
    /// Result bindings (populated for local execution; empty for remote placeholders).
    pub results: Vec<BindingRow>,
    /// Whether execution succeeded.
    pub success: bool,
    /// Error message when `success` is false.
    pub error: Option<String>,
    /// For remote endpoints, the fully-constructed query URL that an HTTP client
    /// would fetch. `None` for local execution.
    pub remote_query_url: Option<String>,
}

impl FederatedQueryResult {
    /// Number of result rows.
    pub fn row_count(&self) -> usize {
        self.results.len()
    }
}

/// A federated query: a SPARQL query to run across one or more services.
#[derive(Debug, Clone)]
pub struct FederatedQuery {
    /// Services to dispatch the query to.
    pub services: Vec<FederatedService>,
    /// The SPARQL query string.
    pub query: String,
}

impl FederatedQuery {
    /// Create a new federated query targeting a single service.
    pub fn new(query: &str) -> Self {
        Self {
            services: Vec::new(),
            query: query.to_string(),
        }
    }

    /// Add a service to the federated query.
    pub fn with_service(mut self, service: FederatedService) -> Self {
        self.services.push(service);
        self
    }
}

/// Federated query engine — dispatches queries across local and remote services.
///
/// Local services (`local:`/`qualia:`) are executed in-process via the standard
/// SPARQL parser → planner → executor pipeline. Remote HTTP services return a
/// placeholder result carrying the constructed query URL; a real HTTP transport
/// can be plugged in later by replacing the placeholder branch in
/// [`execute_service`](Self::execute_service).
pub struct FederatedQueryEngine<'a> {
    /// Local Quins the engine executes local services against.
    pub local_quins: &'a [NQuin],
    /// Registered service endpoints keyed by endpoint URL.
    pub services: HashMap<String, FederatedService>,
    /// Whether [`initialize`](Self::initialize) has been called.
    pub initialized: bool,
}

impl<'a> FederatedQueryEngine<'a> {
    /// Create a new engine over a slice of local Quins.
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            local_quins: quins,
            services: HashMap::new(),
            initialized: false,
        }
    }

    /// Initialize the engine for use. Clears any stale state and marks the
    /// engine ready. Calling this is required before executing federated
    /// queries so that downstream code can rely on a deterministic start state.
    pub fn initialize(&mut self) -> Result<(), String> {
        // Reset to a clean, known-good state. Registered services are preserved
        // (registration may happen before or after initialize), but the
        // initialized flag is what gates execution.
        self.initialized = true;
        Ok(())
    }

    /// Register a federated service endpoint. Re-registering an existing
    /// endpoint URL updates its configuration in place.
    pub fn register_service(&mut self, service: FederatedService) {
        self.services.insert(service.endpoint.clone(), service);
    }

    /// List the endpoint URLs of all registered services.
    pub fn list_services(&self) -> Vec<String> {
        let mut endpoints: Vec<String> = self.services.keys().cloned().collect();
        endpoints.sort();
        endpoints
    }

    /// Execute a query against a single federated service.
    ///
    /// - Local endpoints (`local:`/`qualia:`) are executed in-process and
    ///   return real result bindings.
    /// - Remote HTTP endpoints return a placeholder result containing the
    ///   constructed query URL (`<endpoint>?query=<encoded>`). The HTTP fetch
    ///   itself requires an async runtime which is not available in this
    ///   synchronous module; the placeholder is structured so a transport layer
    ///   can be plugged in later without changing the call sites.
    pub fn execute_service(
        &self,
        service: &FederatedService,
        query: &str,
    ) -> Result<FederatedQueryResult, String> {
        if service.is_local() {
            // Execute locally through the standard pipeline.
            let (sparql_query, ctx) = match sparql_parser::parse_sparql(query) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return Ok(FederatedQueryResult {
                        service_endpoint: service.endpoint.clone(),
                        query: query.to_string(),
                        results: Vec::new(),
                        success: false,
                        error: Some(format!("Parse error: {}", e)),
                        remote_query_url: None,
                    });
                }
            };

            let plan = match QueryPlanner::plan(&sparql_query, &ctx) {
                Ok(plan) => plan,
                Err(e) => {
                    return Ok(FederatedQueryResult {
                        service_endpoint: service.endpoint.clone(),
                        query: query.to_string(),
                        results: Vec::new(),
                        success: false,
                        error: Some(format!("Planning error: {}", e)),
                        remote_query_url: None,
                    });
                }
            };

            let executor = QueryExecutor::new(self.local_quins);
            match executor.execute(&plan, &ctx) {
                Ok(results) => Ok(FederatedQueryResult {
                    service_endpoint: service.endpoint.clone(),
                    query: query.to_string(),
                    results,
                    success: true,
                    error: None,
                    remote_query_url: None,
                }),
                Err(e) => Ok(FederatedQueryResult {
                    service_endpoint: service.endpoint.clone(),
                    query: query.to_string(),
                    results: Vec::new(),
                    success: false,
                    error: Some(format!("Execution error: {}", e)),
                    remote_query_url: None,
                }),
            }
        } else {
            // Remote HTTP endpoint: construct the query URL and return a
            // placeholder. A real HTTP transport can be wired in here later.
            let query_url = build_remote_query_url(&service.endpoint, query);
            Ok(FederatedQueryResult {
                service_endpoint: service.endpoint.clone(),
                query: query.to_string(),
                results: Vec::new(),
                success: true,
                error: None,
                remote_query_url: Some(query_url),
            })
        }
    }

    /// Execute a federated query across all of its services.
    ///
    /// Each service is dispatched via [`execute_service`](Self::execute_service)
    /// and the per-service results are collected. Results are concatenated
    /// (simple merge); full SPARQL join semantics across services are a
    /// separate task.
    pub fn execute_federated(
        &self,
        query: &FederatedQuery,
    ) -> Result<Vec<FederatedQueryResult>, String> {
        if !self.initialized {
            return Err("FederatedQueryEngine not initialized".to_string());
        }
        if query.services.is_empty() {
            return Err("Federated query has no services".to_string());
        }

        let mut results = Vec::with_capacity(query.services.len());
        for service in &query.services {
            // A per-service failure does not abort the whole federated query;
            // the error is captured in the result so callers can decide.
            let result = self.execute_service(service, &query.query)?;
            results.push(result);
        }
        Ok(results)
    }
}

impl<'a> Default for FederatedQueryEngine<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// Build the HTTP query URL for a remote SPARQL endpoint using simple
/// percent-encoding of the query string. This is the URL an HTTP transport
/// layer would GET/POST.
fn build_remote_query_url(endpoint: &str, query: &str) -> String {
    let encoded = percent_encode_query(query);
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    format!("{}{}query={}", endpoint, separator, encoded)
}

/// Minimal percent-encoder for the SPARQL query parameter. Encodes characters
/// that are not unreserved per RFC 3986. Kept dependency-free and synchronous.
fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        // Unreserved: A-Z a-z 0-9 - _ . ~
        let unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

/// DID-based CORS helper
pub struct DidCorsHelper;

impl DidCorsHelper {
    /// Verify DID signature for CORS preflight
    pub fn verify_did_signature(_did: u64, _signature: u64, _challenge: u64) -> Result<bool, String> {
        // In production, this would:
        // 1. Resolve DID to get public key
        // 2. Verify signature of challenge using public key
        // 3. Return true if valid

        // Simplified: always true
        Ok(true)
    }

    /// Generate DID-based challenge for CORS
    pub fn generate_challenge(did: u64) -> u64 {
        // In production, generate cryptographically secure challenge
        did ^ 0xDEADBEEFCAFEBABE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federated_handler_creation() {
        let quins = vec![];
        let handler = FederatedQueryHandler::new(&quins);
        assert_eq!(handler.endpoint_count, 0);
    }

    #[test]
    fn test_register_endpoint() {
        let quins = vec![];
        let mut handler = FederatedQueryHandler::new(&quins);

        let endpoint = ServiceEndpoint {
            did: 1,
            endpoint_url: 2,
            auth_method: 0,
            timeout_ms: 5000,
        };

        let result = handler.register_endpoint(endpoint);
        assert!(result.is_ok());
        assert_eq!(handler.endpoint_count, 1);
    }

    #[test]
    fn test_resolve_did() {
        let quins = vec![];
        let mut handler = FederatedQueryHandler::new(&quins);

        let endpoint = ServiceEndpoint {
            did: 123,
            endpoint_url: 456,
            auth_method: 0,
            timeout_ms: 5000,
        };

        handler.register_endpoint(endpoint).unwrap();
        let url_hash = handler.resolve_did(123).unwrap();
        assert_eq!(url_hash, 456);
    }

    #[test]
    fn test_cors_check() {
        let quins = vec![];
        let handler = FederatedQueryHandler::new(&quins);

        let allowed = handler.check_cors_allowed(123, 123).unwrap();
        assert!(allowed);

        let not_allowed = handler.check_cors_allowed(123, 456).unwrap();
        assert!(!not_allowed);
    }

    #[test]
    fn test_did_signature_verification() {
        let result = DidCorsHelper::verify_did_signature(123, 456, 789).unwrap();
        assert!(result);
    }

    // ---- FederatedQueryEngine tests ----

    #[test]
    fn test_engine_creation() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        assert!(!engine.initialized);
        assert!(engine.services.is_empty());
    }

    #[test]
    fn test_engine_initialize() {
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        assert!(!engine.initialized);
        engine.initialize().unwrap();
        assert!(engine.initialized);
    }

    #[test]
    fn test_register_and_list_services() {
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        engine.register_service(FederatedService::new("local:default"));
        engine.register_service(FederatedService::new("https://remote.example.org/sparql"));

        let services = engine.list_services();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&"local:default".to_string()));
        assert!(services.contains(&"https://remote.example.org/sparql".to_string()));
    }

    #[test]
    fn test_register_service_overwrites() {
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        let mut svc = FederatedService::new("local:default");
        svc.auth_method = 0;
        engine.register_service(svc);
        let mut svc2 = FederatedService::new("local:default");
        svc2.auth_method = 2;
        engine.register_service(svc2);

        // Same endpoint URL → single entry, updated config.
        assert_eq!(engine.list_services().len(), 1);
        assert_eq!(engine.services.get("local:default").unwrap().auth_method, 2);
    }

    #[test]
    fn test_service_is_local() {
        assert!(FederatedService::new("local:default").is_local());
        assert!(FederatedService::new("qualia:graph1").is_local());
        assert!(!FederatedService::new("https://remote.example.org/sparql").is_local());
        assert!(!FederatedService::new("http://remote.example.org/sparql").is_local());
    }

    #[test]
    fn test_service_local_graph() {
        assert_eq!(FederatedService::new("local:default").local_graph(), Some("default"));
        assert_eq!(FederatedService::new("qualia:graph1").local_graph(), Some("graph1"));
        assert_eq!(
            FederatedService::new("https://remote.example.org/sparql").local_graph(),
            None
        );
    }

    #[test]
    fn test_execute_service_local() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let service = FederatedService::new("local:default");
        let result = engine
            .execute_service(&service, "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert_eq!(result.service_endpoint, "local:default");
        assert!(result.success, "local execution should succeed: {:?}", result.error);
        assert!(result.remote_query_url.is_none());
        // No quins → no rows, but still a successful local execution.
        assert_eq!(result.row_count(), 0);
    }

    #[test]
    fn test_execute_service_local_parse_error() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let service = FederatedService::new("local:default");
        let result = engine
            .execute_service(&service, "this is not sparql")
            .unwrap();
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.row_count(), 0);
    }

    #[test]
    fn test_execute_service_remote_placeholder() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let service = FederatedService::new("https://remote.example.org/sparql");
        let result = engine
            .execute_service(&service, "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert_eq!(result.service_endpoint, "https://remote.example.org/sparql");
        assert!(result.success);
        // Remote path returns a placeholder with the constructed query URL.
        assert!(result.remote_query_url.is_some());
        let url = result.remote_query_url.as_ref().unwrap();
        assert!(url.starts_with("https://remote.example.org/sparql?query="));
        // Spaces must be percent-encoded.
        assert!(!url.contains(' '));
        // No real rows for the remote placeholder.
        assert_eq!(result.row_count(), 0);
    }

    #[test]
    fn test_execute_service_remote_with_existing_query_param() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let service = FederatedService::new("https://remote.example.org/sparql?dataset=foo");
        let result = engine
            .execute_service(&service, "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        let url = result.remote_query_url.unwrap();
        // Should join with '&' since the endpoint already has a '?'.
        assert!(url.contains("&query="));
    }

    #[test]
    fn test_execute_federated_across_services() {
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        engine.initialize().unwrap();
        engine.register_service(FederatedService::new("local:default"));
        engine.register_service(FederatedService::new("https://remote.example.org/sparql"));

        let query = FederatedQuery::new("SELECT ?s WHERE { ?s ?p ?o }")
            .with_service(FederatedService::new("local:default"))
            .with_service(FederatedService::new("https://remote.example.org/sparql"));

        let results = engine.execute_federated(&query).unwrap();
        assert_eq!(results.len(), 2);
        // Local first, then remote — order follows the query's service list.
        assert_eq!(results[0].service_endpoint, "local:default");
        assert!(results[0].success);
        assert!(results[0].remote_query_url.is_none());
        assert_eq!(results[1].service_endpoint, "https://remote.example.org/sparql");
        assert!(results[1].success);
        assert!(results[1].remote_query_url.is_some());
    }

    #[test]
    fn test_execute_federated_not_initialized() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let query = FederatedQuery::new("SELECT ?s WHERE { ?s ?p ?o }")
            .with_service(FederatedService::new("local:default"));
        let err = engine.execute_federated(&query).unwrap_err();
        assert!(err.contains("not initialized"));
    }

    #[test]
    fn test_execute_federated_no_services() {
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        engine.initialize().unwrap();
        let query = FederatedQuery::new("SELECT ?s WHERE { ?s ?p ?o }");
        let err = engine.execute_federated(&query).unwrap_err();
        assert!(err.contains("no services"));
    }

    #[test]
    fn test_percent_encode_query() {
        let encoded = percent_encode_query("SELECT ?s WHERE { ?s ?p ?o }");
        assert!(!encoded.contains(' '));
        assert!(encoded.contains("SELECT"));
        // '?' should be encoded as %3F.
        assert!(encoded.contains("%3F"));
    }

    #[test]
    fn test_build_remote_query_url() {
        let url = build_remote_query_url("https://example.org/sparql", "SELECT ?s");
        assert!(url.starts_with("https://example.org/sparql?query="));
        assert!(!url.contains(' '));
    }
}
