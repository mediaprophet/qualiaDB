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

/// SPARQL Federated Query Handler — a DID-registry front over the URL-based
/// [`FederatedQueryEngine`]. It maps a DID to a concrete endpoint URL string and
/// dispatches remote SPARQL requests through the same real HTTP transport.
pub struct FederatedQueryHandler<'a> {
    pub local_quins: &'a [NQuin],
    pub endpoints: [Option<ServiceEndpoint>; 16],
    pub endpoint_count: u8,
    pub cache_enabled: bool,
    /// DID → concrete endpoint URL string (a `u64` hash cannot be un-hashed to a URL,
    /// so the resolvable URL is stored alongside for real remote execution).
    endpoint_urls: HashMap<u64, String>,
}

impl<'a> FederatedQueryHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            local_quins: quins,
            endpoints: [None; 16],
            endpoint_count: 0,
            cache_enabled: true,
            endpoint_urls: HashMap::new(),
        }
    }

    /// Associate a DID with a concrete endpoint URL so remote queries can actually be
    /// dispatched over HTTP (the `ServiceEndpoint.endpoint_url` hash is not reversible).
    pub fn set_endpoint_url(&mut self, did: u64, url: &str) {
        self.endpoint_urls.insert(did, url.to_string());
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

    /// Execute a federated query against the endpoint registered for `did`, over the
    /// real HTTP SPARQL 1.1 Protocol transport.
    ///
    /// Requires a resolvable endpoint URL registered via
    /// [`set_endpoint_url`](Self::set_endpoint_url) (the `ServiceEndpoint.endpoint_url`
    /// hash is not reversible to a URL). Authenticated (DID-LD/JWT) endpoints must be
    /// signed by the identity layer; only the **no-auth** path is dispatched here — no
    /// fabricated auth header. Network egress: makes a real outbound request.
    pub fn execute_service(
        &self,
        did: u64,
        query: &str,
        _format: &str,
    ) -> Result<FederatedResult, String> {
        let endpoint = (0..self.endpoint_count as usize)
            .filter_map(|i| self.endpoints[i])
            .find(|e| e.did == did)
            .ok_or("DID not found in endpoint registry")?;
        if endpoint.auth_method != 0 {
            return Err(
                "authenticated (DID-LD/JWT) federation must be signed by the \
                        identity layer; only no-auth remote endpoints are dispatched here"
                    .to_string(),
            );
        }
        let url = self.endpoint_urls.get(&did).ok_or(
            "no resolvable endpoint URL registered for this DID (call set_endpoint_url); \
             the endpoint_url hash is not reversible",
        )?;
        let body = fetch_remote_sparql(url, query, endpoint.timeout_ms, None)?;
        let (_vars, rows, _lexicon) = parse_sparql_results_json(&body)?;
        Ok(FederatedResult {
            endpoint_did: did,
            row_count: rows.len() as u16,
            success: true,
        })
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
    /// Result bindings — real rows for both local execution and successful remote
    /// (HTTP) execution. Each slot holds a term hash resolvable via [`Self::lexicon`]
    /// (remote) or the local graph lexicon (local).
    pub results: Vec<BindingRow>,
    /// Result variable names in slot order (populated for remote SPARQL-results
    /// responses; empty for a bare local execution that carries its own ctx).
    pub variables: Vec<String>,
    /// Hash → text (+ lang/datatype) for the terms in `results`, so remote string
    /// terms are resolvable in the local u64-term model. Empty for local execution
    /// (whose terms resolve via the local graph lexicon).
    pub lexicon: LiteralTable,
    /// Whether execution succeeded.
    pub success: bool,
    /// Error message when `success` is false.
    pub error: Option<String>,
    /// For remote endpoints, the fully-constructed query URL that was fetched (kept
    /// for provenance/debugging). `None` for local execution.
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
/// SPARQL parser → planner → executor pipeline. Remote HTTP(S) services are executed
/// over the **real SPARQL 1.1 Protocol** ([`fetch_remote_sparql`] +
/// [`parse_sparql_results_json`]): a `application/sparql-query` POST, the
/// `application/sparql-results+json` response parsed into binding rows + a resolvable
/// lexicon. Remote execution is real network egress and runs only for endpoints the
/// caller explicitly targets.
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
                    return Ok(Self::failed(
                        service,
                        query,
                        format!("Parse error: {e}"),
                        None,
                    ))
                }
            };
            let plan = match QueryPlanner::plan(&sparql_query, &ctx) {
                Ok(plan) => plan,
                Err(e) => {
                    return Ok(Self::failed(
                        service,
                        query,
                        format!("Planning error: {e}"),
                        None,
                    ))
                }
            };
            let executor = QueryExecutor::new(self.local_quins);
            match executor.execute(&plan, &ctx) {
                Ok(results) => Ok(FederatedQueryResult {
                    service_endpoint: service.endpoint.clone(),
                    query: query.to_string(),
                    results,
                    variables: Vec::new(),
                    lexicon: LiteralTable::new(),
                    success: true,
                    error: None,
                    remote_query_url: None,
                }),
                Err(e) => Ok(Self::failed(
                    service,
                    query,
                    format!("Execution error: {e}"),
                    None,
                )),
            }
        } else {
            // Remote HTTP SPARQL endpoint: perform a real SPARQL 1.1 Protocol request and
            // parse the `application/sparql-results+json` response into binding rows.
            //
            // Network egress: this makes a real outbound HTTP request, and runs only when
            // a caller explicitly dispatches to a remote `FederatedService`. Authenticated
            // (DID-LD/JWT) endpoints need the identity layer to sign the request; the
            // no-auth path is supported here directly (no synthetic/fake auth header).
            let query_url = build_remote_query_url(&service.endpoint, query);
            match fetch_remote_sparql(&service.endpoint, query, service.timeout_ms, None) {
                Ok(body) => match parse_sparql_results_json(&body) {
                    Ok((variables, results, lexicon)) => Ok(FederatedQueryResult {
                        service_endpoint: service.endpoint.clone(),
                        query: query.to_string(),
                        results,
                        variables,
                        lexicon,
                        success: true,
                        error: None,
                        remote_query_url: Some(query_url),
                    }),
                    Err(e) => Ok(Self::failed(
                        service,
                        query,
                        format!("Remote result parse error: {e}"),
                        Some(query_url),
                    )),
                },
                Err(e) => Ok(Self::failed(service, query, e, Some(query_url))),
            }
        }
    }

    /// Construct a failed [`FederatedQueryResult`] carrying an error message.
    fn failed(
        service: &FederatedService,
        query: &str,
        error: String,
        remote_query_url: Option<String>,
    ) -> FederatedQueryResult {
        FederatedQueryResult {
            service_endpoint: service.endpoint.clone(),
            query: query.to_string(),
            results: Vec::new(),
            variables: Vec::new(),
            lexicon: LiteralTable::new(),
            success: false,
            error: Some(error),
            remote_query_url,
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

/// Execute a SPARQL query against a remote HTTP endpoint (SPARQL 1.1 Protocol) and
/// return the raw `application/sparql-results+json` response body.
///
/// Uses a **blocking** HTTP client run on a dedicated thread, so it is safe to call from
/// within a Tokio runtime (a blocking client cannot be created on an async worker
/// thread). `timeout_ms` bounds the whole request. `auth`, when present, is sent as the
/// `Authorization` header value.
///
/// **Network egress.** This performs a real outbound HTTP request to `endpoint`.
#[cfg(not(target_arch = "wasm32"))]
fn fetch_remote_sparql(
    endpoint: &str,
    query: &str,
    timeout_ms: u32,
    auth: Option<&str>,
) -> Result<String, String> {
    let endpoint = endpoint.to_string();
    let query = query.to_string();
    let auth = auth.map(|s| s.to_string());
    std::thread::spawn(move || -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms.max(1) as u64))
            .build()
            .map_err(|e| format!("http client build failed: {e}"))?;
        let mut req = client
            .post(&endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/sparql-query")
            .header(reqwest::header::ACCEPT, "application/sparql-results+json")
            .body(query);
        if let Some(a) = auth {
            req = req.header(reqwest::header::AUTHORIZATION, a);
        }
        let resp = req
            .send()
            .map_err(|e| format!("remote request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| format!("reading remote response failed: {e}"))?;
        if !status.is_success() {
            let snippet: String = text.chars().take(200).collect();
            return Err(format!("remote endpoint returned HTTP {status}: {snippet}"));
        }
        Ok(text)
    })
    .join()
    .map_err(|_| "remote fetch thread panicked".to_string())?
}

/// wasm has no synchronous HTTP transport — remote federation is a native capability.
#[cfg(target_arch = "wasm32")]
fn fetch_remote_sparql(
    _endpoint: &str,
    _query: &str,
    _timeout_ms: u32,
    _auth: Option<&str>,
) -> Result<String, String> {
    Err("remote SPARQL federation is not available on the wasm target".to_string())
}

/// Parse an `application/sparql-results+json` document into `(variables, rows, lexicon)`:
/// the result variable names in order, one [`BindingRow`] per solution (each slot a term
/// hash), and a [`LiteralTable`] mapping those hashes back to text (with lang/datatype),
/// so remote string terms are resolvable in the local u64-term model. Also handles the
/// ASK form `{ "boolean": … }` (a single row with slot 0 = 1/0).
fn parse_sparql_results_json(
    body: &str,
) -> Result<(Vec<String>, Vec<BindingRow>, LiteralTable), String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("invalid SPARQL results JSON: {e}"))?;
    let mut lexicon = LiteralTable::new();

    // ASK result → a single row, slot 0 = 1/0.
    if let Some(b) = v.get("boolean").and_then(|b| b.as_bool()) {
        let mut row = BindingRow::new();
        row.slots[0] = Some(if b { 1 } else { 0 });
        return Ok((Vec::new(), vec![row], lexicon));
    }

    let vars: Vec<String> = v
        .get("head")
        .and_then(|h| h.get("vars"))
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let bindings = v
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::with_capacity(bindings.len());
    for binding in &bindings {
        let mut row = BindingRow::new();
        for (i, var) in vars.iter().enumerate() {
            if i >= MAX_BINDINGS {
                break; // fixed-capacity binding row (§6)
            }
            if let Some(term) = binding.get(var) {
                let value = term.get("value").and_then(|x| x.as_str()).unwrap_or("");
                let lang = term.get("xml:lang").and_then(|x| x.as_str());
                let datatype = term.get("datatype").and_then(|x| x.as_str());
                // Hash consistently with the local term model; intern text + any tags so
                // the remote term round-trips through the resolver and STR/LANG/DATATYPE.
                let hash = crate::sparql_ast::literal_term_hash(value, lang, datatype);
                lexicon.intern_tagged(hash, value, lang, datatype);
                row.slots[i] = Some(hash);
            }
        }
        rows.push(row);
    }
    Ok((vars, rows, lexicon))
}

/// DID-based CORS helper
pub struct DidCorsHelper;

impl DidCorsHelper {
    /// Verify a DID signature for a CORS preflight.
    ///
    /// The SPARQL federation layer holds **no key material** and receives only opaque
    /// `u64` hashes (not the actual signature/challenge/public-key bytes), so it cannot
    /// perform a real cryptographic verification here. It therefore **fails closed** with
    /// a named error rather than returning a fabricated `true` (which would let any origin
    /// pass). Real verification must go through the identity/key-vault layer, which holds
    /// the resolved DID public key and the raw bytes.
    pub fn verify_did_signature(
        _did: u64,
        _signature: u64,
        _challenge: u64,
    ) -> Result<bool, String> {
        Err(
            "DID signature verification must be performed by the identity/key-vault layer \
             (the SPARQL federation layer holds no keys); refusing to fabricate a result"
                .to_string(),
        )
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
    fn test_did_signature_verification_fails_closed() {
        // The SPARQL layer holds no keys → verification is an honest error, NOT a
        // fabricated `true` (which would let any origin pass a CORS preflight).
        assert!(DidCorsHelper::verify_did_signature(123, 456, 789).is_err());
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
        assert_eq!(
            FederatedService::new("local:default").local_graph(),
            Some("default")
        );
        assert_eq!(
            FederatedService::new("qualia:graph1").local_graph(),
            Some("graph1")
        );
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
        assert!(
            result.success,
            "local execution should succeed: {:?}",
            result.error
        );
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
    #[ignore = "makes a real (localhost) network connection — integration test"]
    fn test_execute_service_remote_unreachable_fails_gracefully() {
        let quins = vec![];
        let engine = FederatedQueryEngine::new(&quins);
        let mut service = FederatedService::new("http://127.0.0.1:1/sparql");
        service.timeout_ms = 300;
        let result = engine
            .execute_service(&service, "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        // A real fetch to an unreachable endpoint fails GRACEFULLY: success=false with a
        // named error, and the constructed query URL is retained for provenance.
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.remote_query_url.is_some());
        assert_eq!(result.row_count(), 0);
    }

    #[test]
    fn test_execute_federated_across_local_services() {
        // Federation orchestration across multiple services, network-free (both local).
        let quins = vec![];
        let mut engine = FederatedQueryEngine::new(&quins);
        engine.initialize().unwrap();

        let query = FederatedQuery::new("SELECT ?s WHERE { ?s ?p ?o }")
            .with_service(FederatedService::new("local:default"))
            .with_service(FederatedService::new("qualia:graph1"));

        let results = engine.execute_federated(&query).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].service_endpoint, "local:default");
        assert!(results[0].success);
        assert!(results[0].remote_query_url.is_none());
        assert_eq!(results[1].service_endpoint, "qualia:graph1");
        assert!(results[1].success);
    }

    #[test]
    fn test_parse_sparql_results_json_select() {
        // A standard application/sparql-results+json SELECT response → real binding rows
        // + a resolvable lexicon (incl. a language-tagged and a datatyped literal).
        let body = r#"{
          "head": { "vars": ["s", "name", "age"] },
          "results": { "bindings": [
            { "s": {"type":"uri","value":"http://ex/a"},
              "name": {"type":"literal","xml:lang":"en","value":"Alice"},
              "age": {"type":"literal","datatype":"http://www.w3.org/2001/XMLSchema#integer","value":"30"} }
          ] }
        }"#;
        let (vars, rows, lexicon) = parse_sparql_results_json(body).unwrap();
        assert_eq!(vars, vec!["s", "name", "age"]);
        assert_eq!(rows.len(), 1);
        let name_hash = rows[0].slots[1].unwrap();
        assert_eq!(lexicon.resolve(name_hash), Some("Alice"));
        assert_eq!(lexicon.lang(name_hash), Some("en"));
        let age_hash = rows[0].slots[2].unwrap();
        assert_eq!(
            lexicon.datatype(age_hash),
            Some("http://www.w3.org/2001/XMLSchema#integer")
        );
    }

    #[test]
    fn test_parse_sparql_results_json_ask() {
        let (vars, rows, _lex) =
            parse_sparql_results_json(r#"{ "head": {}, "boolean": true }"#).unwrap();
        assert!(vars.is_empty());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].slots[0], Some(1));
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
    fn test_build_remote_query_url_joins_with_ampersand_when_query_present() {
        // Endpoint already carrying a query string joins with '&', not a second '?'.
        let url = build_remote_query_url("https://example.org/sparql?dataset=foo", "SELECT ?s");
        assert!(url.contains("&query="));
        assert!(!url.contains(' '));
    }

    #[test]
    fn test_build_remote_query_url() {
        let url = build_remote_query_url("https://example.org/sparql", "SELECT ?s");
        assert!(url.starts_with("https://example.org/sparql?query="));
        assert!(!url.contains(' '));
    }
}
