//! SPARQL WebSocket Support
//!
//! Real-time SPARQL query results via WebSocket using zero-allocation patterns.

use crate::sparql_ast::*;
use crate::sparql_executor::*;
use crate::sparql_library::serialisers::sparql_results::ResultFormatter;
use crate::sparql_parser;
use crate::sparql_planner::*;
use crate::NQuin;

use std::collections::HashMap;

/// WebSocket message type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketMessageType {
    Query,
    Subscribe,
    Unsubscribe,
    Result,
    Error,
    Close,
}

/// WebSocket message
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WebSocketMessage {
    pub msg_type: WebSocketMessageType,
    pub query_id: u64,
    pub payload_len: u16,
}

/// WebSocket session
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WebSocketSession {
    pub session_id: u64,
    pub active_query: Option<u64>,
    pub subscribed: bool,
}

/// A real-time subscription registered by a WebSocket client.
///
/// Each subscription binds a `client_id` (the WebSocket connection identifier)
/// to a SPARQL query whose results the client wants pushed to it whenever the
/// underlying data changes.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Unique subscription identifier (a query-derived token).
    pub subscription_id: String,
    /// The WebSocket client that owns this subscription.
    pub client_id: String,
    /// The SPARQL query the subscription is watching.
    pub query: String,
}

impl Subscription {
    /// Create a new subscription.
    pub fn new(subscription_id: String, client_id: String, query: String) -> Self {
        Self {
            subscription_id,
            client_id,
            query,
        }
    }
}

/// SPARQL WebSocket handler
pub struct SparqlWebSocketHandler<'a> {
    pub quins: &'a [NQuin],
    pub sessions: [Option<WebSocketSession>; 32],
    pub session_count: u8,
    /// Active subscriptions keyed by subscription id.
    pub subscriptions: HashMap<String, Subscription>,
    /// Whether [`initialize`](Self::initialize) has been called.
    pub initialized: bool,
}

impl<'a> SparqlWebSocketHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            sessions: [None; 32],
            session_count: 0,
            subscriptions: HashMap::new(),
            initialized: false,
        }
    }

    /// Initialize the handler for use. Resets subscription state and marks the
    /// handler ready. Must be called before subscribing/notifying so downstream
    /// code can rely on a deterministic start state.
    pub fn initialize(&mut self) -> Result<(), String> {
        self.subscriptions.clear();
        self.initialized = true;
        Ok(())
    }

    /// Register a new WebSocket session
    pub fn register_session(&mut self) -> Result<u64, String> {
        if self.session_count >= 32 {
            return Err("Session overflow".to_string());
        }

        let session_id = self.session_count as u64;
        self.sessions[self.session_count as usize] = Some(WebSocketSession {
            session_id,
            active_query: None,
            subscribed: false,
        });
        self.session_count += 1;

        Ok(session_id)
    }

    /// Unregister a WebSocket session
    pub fn unregister_session(&mut self, session_id: u64) -> Result<(), String> {
        for i in 0..self.session_count as usize {
            if let Some(session) = self.sessions[i] {
                if session.session_id == session_id {
                    self.sessions[i] = None;
                    return Ok(());
                }
            }
        }
        Err("Session not found".to_string())
    }

    /// Handle a WebSocket query, returning formatted output (json or xml).
    ///
    /// This is the format-aware, session-scoped variant. The simpler
    /// [`handle_query`](Self::handle_query) returns serialized JSON bytes.
    pub fn handle_query_formatted(
        &self,
        query: &str,
        format: &str,
        _session_id: u64,
    ) -> Result<String, String> {
        // Parse query
        let (sparql_query, ctx) = sparql_parser::parse_sparql(query)?;

        // Plan query
        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;

        // Execute query
        let executor = QueryExecutor::new(self.quins);
        let results = executor.execute(&plan, &ctx)?;

        // Format results
        match format.to_lowercase().as_str() {
            "json" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_json(&mut output, &vars, &results, &ctx, None)
                    .map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            "xml" => {
                let mut output = Vec::new();
                let vars = match &sparql_query {
                    SparqlQuery::Select(select) => {
                        select.variables[..select.var_count as usize].to_vec()
                    }
                    _ => vec![],
                };
                ResultFormatter::format_xml(&mut output, &vars, &results, &ctx, None)
                    .map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format. Use: xml or json".to_string()),
        }
    }

    /// Stream query results in chunks, executing the query and formatting each
    /// chunk as JSON. This is the query-executing variant. The simpler
    /// [`stream_results`](Self::stream_results) splits an already-serialized
    /// byte buffer.
    pub fn stream_query_results(
        &self,
        query: &str,
        chunk_size: usize,
        _session_id: u64,
    ) -> Result<Vec<String>, String> {
        // Parse and execute query
        let (sparql_query, ctx) = sparql_parser::parse_sparql(query)?;
        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        let executor = QueryExecutor::new(self.quins);
        let results = executor.execute(&plan, &ctx)?;

        // Get variables
        let vars = match &sparql_query {
            SparqlQuery::Select(select) => select.variables[..select.var_count as usize].to_vec(),
            _ => vec![],
        };

        // Chunk results
        let mut chunks = Vec::new();
        for chunk in results.chunks(chunk_size) {
            let mut output = Vec::new();
            ResultFormatter::format_json(&mut output, &vars, chunk, &ctx, None)
                .map_err(|e| e.to_string())?;
            chunks.push(String::from_utf8(output).unwrap());
        }

        Ok(chunks)
    }

    /// Subscribe a session to real-time updates for a query (session-based).
    ///
    /// This is the numeric-session variant. The string-keyed
    /// [`subscribe`](Self::subscribe) is the subscription notification API.
    pub fn subscribe_session(&mut self, session_id: u64, query: &str) -> Result<u64, String> {
        // Find session
        for i in 0..self.session_count as usize {
            if let Some(session) = self.sessions[i] {
                if session.session_id == session_id {
                    // Store query hash as subscription ID
                    let query_hash = crate::lexicon::generate_60bit_token(query.as_bytes());
                    self.sessions[i] = Some(WebSocketSession {
                        session_id,
                        active_query: Some(query_hash),
                        subscribed: true,
                    });
                    return Ok(query_hash);
                }
            }
        }
        Err("Session not found".to_string())
    }

    /// Unsubscribe a session from updates (session-based).
    pub fn unsubscribe_session(&mut self, session_id: u64) -> Result<(), String> {
        for i in 0..self.session_count as usize {
            if let Some(session) = self.sessions[i] {
                if session.session_id == session_id {
                    self.sessions[i] = Some(WebSocketSession {
                        session_id,
                        active_query: None,
                        subscribed: false,
                    });
                    return Ok(());
                }
            }
        }
        Err("Session not found".to_string())
    }

    /// Notify session subscribers of updates (session-based), returning the
    /// session ids whose active query matches `query_hash`.
    pub fn notify_session_subscribers(&self, query_hash: u64) -> Vec<u64> {
        let mut subscribers = Vec::new();

        for i in 0..self.session_count as usize {
            if let Some(session) = self.sessions[i] {
                if session.subscribed && session.active_query == Some(query_hash) {
                    subscribers.push(session.session_id);
                }
            }
        }

        subscribers
    }

    // ---- Subscription-based API (client_id string keyed) ----
    //
    // The methods below use a `client_id: &str` / `subscription_id: &str` model
    // that is independent of the numeric session slots above. This is the
    // notification system used by the WebSocket subscription feature.

    /// Register a subscription for a client and return the subscription id.
    ///
    /// The subscription id is derived from the query (via
    /// `generate_60bit_token`) so the same query re-subscribed by the same
    /// client yields a stable id. The query and client_id are stored in the
    /// subscriptions map for later notification.
    pub fn subscribe(&mut self, client_id: &str, query: &str) -> Result<String, String> {
        let token = crate::lexicon::generate_60bit_token(query.as_bytes());
        let subscription_id = format!("sub-{}-{}", client_id, token);
        let subscription = Subscription::new(
            subscription_id.clone(),
            client_id.to_string(),
            query.to_string(),
        );
        self.subscriptions
            .insert(subscription_id.clone(), subscription);
        Ok(subscription_id)
    }

    /// Remove a subscription by its id. Returns an error if the subscription
    /// does not exist.
    pub fn unsubscribe(&mut self, subscription_id: &str) -> Result<(), String> {
        if self.subscriptions.remove(subscription_id).is_some() {
            Ok(())
        } else {
            Err("Subscription not found".to_string())
        }
    }

    /// List all active subscriptions.
    pub fn get_subscriptions(&self) -> Vec<&Subscription> {
        self.subscriptions.values().collect()
    }

    /// Count active subscriptions.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Notify all active subscribers of an event.
    ///
    /// For each subscription, returns a `(client_id, notification_message)`
    /// pair. The notification message is a JSON-like string containing the
    /// subscription's query and the event data, so a transport layer can push
    /// it directly to the connected client.
    pub fn notify_subscribers(&self, event_data: &str) -> Vec<(String, String)> {
        let mut notifications = Vec::new();
        for sub in self.subscriptions.values() {
            let message = format_notification(&sub.subscription_id, &sub.query, event_data);
            notifications.push((sub.client_id.clone(), message));
        }
        notifications
    }

    /// Execute a SPARQL query and return the results as serialized bytes in a
    /// simple JSON format. This is the WebSocket query handler entry point.
    pub fn handle_query(&self, query: &str) -> Result<Vec<u8>, String> {
        // Parse query
        let (sparql_query, ctx) = sparql_parser::parse_sparql(query)?;

        // Plan query
        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;

        // Execute query
        let executor = QueryExecutor::new(self.quins);
        let results = executor.execute(&plan, &ctx)?;

        // Serialize results as JSON bytes.
        let vars = match &sparql_query {
            SparqlQuery::Select(select) => select.variables[..select.var_count as usize].to_vec(),
            _ => vec![],
        };
        let mut output = Vec::new();
        ResultFormatter::format_json(&mut output, &vars, &results, &ctx, None)
            .map_err(|e| e.to_string())?;
        Ok(output)
    }

    /// Split a serialized result buffer into chunks for streaming over
    /// WebSocket frames. Each chunk is at most `chunk_size` bytes (the final
    /// chunk may be smaller).
    pub fn stream_results(&self, results: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
        if chunk_size == 0 {
            // A zero chunk size would loop forever; return the whole buffer as
            // a single chunk instead.
            return vec![results.to_vec()];
        }
        results.chunks(chunk_size).map(|c| c.to_vec()).collect()
    }
}

impl<'a> Default for SparqlWebSocketHandler<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// Build a JSON-like notification message for a subscription event.
///
/// The message is a compact JSON object containing the subscription id, the
/// subscribed query, and the event payload. It is intentionally simple and
/// dependency-free so it can be pushed directly over a WebSocket frame.
fn format_notification(subscription_id: &str, query: &str, event_data: &str) -> String {
    format!(
        "{{\"subscriptionId\":\"{}\",\"query\":\"{}\",\"event\":{}}}",
        escape_json_string(subscription_id),
        escape_json_string(query),
        event_data
    )
}

/// Escape a string for inclusion inside a JSON string literal. Handles the
/// characters that are most likely to appear in subscription ids / queries.
fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_handler_creation() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        assert_eq!(handler.session_count, 0);
    }

    #[test]
    fn test_register_session() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let session_id = handler.register_session().unwrap();
        assert_eq!(session_id, 0);
        assert_eq!(handler.session_count, 1);
    }

    #[test]
    fn test_unregister_session() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let session_id = handler.register_session().unwrap();
        handler.unregister_session(session_id).unwrap();
    }

    #[test]
    fn test_subscribe_session() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let session_id = handler.register_session().unwrap();
        let query_hash = handler
            .subscribe_session(session_id, "SELECT ?s WHERE ?s ?p ?o")
            .unwrap();

        assert!(query_hash > 0);
    }

    // ---- Subscription-based API tests ----

    #[test]
    fn test_handler_initialize() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);
        assert!(!handler.initialized);
        handler.initialize().unwrap();
        assert!(handler.initialized);
        assert_eq!(handler.subscription_count(), 0);
    }

    #[test]
    fn test_subscribe_client_returns_id() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let sub_id = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert!(sub_id.starts_with("sub-client-1-"));
        assert_eq!(handler.subscription_count(), 1);
    }

    #[test]
    fn test_subscribe_client_stores_query_and_client() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let sub_id = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        let subs = handler.get_subscriptions();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].subscription_id, sub_id);
        assert_eq!(subs[0].client_id, "client-1");
        assert_eq!(subs[0].query, "SELECT ?s WHERE { ?s ?p ?o }");
    }

    #[test]
    fn test_subscribe_same_query_same_client_stable_id() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let id1 = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        let id2 = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        // Same client + same query → same id (overwrites the previous entry).
        assert_eq!(id1, id2);
        assert_eq!(handler.subscription_count(), 1);
    }

    #[test]
    fn test_subscribe_different_clients_separate_subscriptions() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        handler
            .subscribe("client-2", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert_eq!(handler.subscription_count(), 2);
    }

    #[test]
    fn test_unsubscribe_client_removes_subscription() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);

        let sub_id = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert_eq!(handler.subscription_count(), 1);

        handler.unsubscribe(&sub_id).unwrap();
        assert_eq!(handler.subscription_count(), 0);
        assert!(handler.get_subscriptions().is_empty());
    }

    #[test]
    fn test_unsubscribe_client_unknown_id_errors() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);
        let err = handler.unsubscribe("does-not-exist").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_subscription_lifecycle() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);
        handler.initialize().unwrap();

        // Subscribe two clients.
        let id1 = handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        let id2 = handler
            .subscribe("client-2", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        assert_eq!(handler.subscription_count(), 2);

        // Notify both.
        let notifications = handler.notify_subscribers("{\"type\":\"update\"}");
        assert_eq!(notifications.len(), 2);
        let client_ids: Vec<&str> = notifications.iter().map(|(c, _)| c.as_str()).collect();
        assert!(client_ids.contains(&"client-1"));
        assert!(client_ids.contains(&"client-2"));
        for (_, msg) in &notifications {
            assert!(msg.contains("\"query\":\"SELECT ?s WHERE { ?s ?p ?o }\""));
            assert!(msg.contains("\"event\":{\"type\":\"update\"}"));
        }

        // Unsubscribe client-1; only client-2 remains.
        handler.unsubscribe(&id1).unwrap();
        assert_eq!(handler.subscription_count(), 1);
        let notifications = handler.notify_subscribers("{\"type\":\"update\"}");
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].0, "client-2");

        // Unsubscribe client-2; none remain.
        handler.unsubscribe(&id2).unwrap();
        assert_eq!(handler.subscription_count(), 0);
        assert!(handler.notify_subscribers("event").is_empty());
    }

    #[test]
    fn test_notify_subscribers_message_format() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);
        handler
            .subscribe("client-1", "SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();

        let notifications = handler.notify_subscribers("{\"data\":42}");
        assert_eq!(notifications.len(), 1);
        let (_client, msg) = &notifications[0];
        assert!(msg.starts_with("{\"subscriptionId\":\""));
        assert!(msg.contains("\"query\":\"SELECT ?s WHERE { ?s ?p ?o }\""));
        assert!(msg.contains("\"event\":{\"data\":42}"));
    }

    #[test]
    fn test_handle_query_bytes_returns_json() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        let bytes = handler
            .handle_query("SELECT ?s WHERE { ?s ?p ?o }")
            .unwrap();
        // ResultFormatter produces a JSON document.
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("head") || text.contains("results") || text.contains('{'));
    }

    #[test]
    fn test_handle_query_bytes_parse_error() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        let err = handler.handle_query("not sparql").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn test_stream_results_bytes_chunks() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        let data: Vec<u8> = (0..25u8).collect();
        let chunks = handler.stream_results(&data, 10);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[1].len(), 10);
        assert_eq!(chunks[2].len(), 5);
        // Reassemble and verify round-trip.
        let reassembled: Vec<u8> = chunks.into_iter().flatten().collect();
        assert_eq!(reassembled, data);
    }

    #[test]
    fn test_stream_results_bytes_empty() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        let chunks = handler.stream_results(&[], 10);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_stream_results_bytes_zero_chunk_size() {
        let quins = vec![];
        let handler = SparqlWebSocketHandler::new(&quins);
        let data: Vec<u8> = (0..5u8).collect();
        // Zero chunk size must not panic; returns a single chunk.
        let chunks = handler.stream_results(&data, 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], data);
    }

    #[test]
    fn test_format_notification_escapes_quotes() {
        let msg = format_notification("sub-1", "SELECT \"x\"", "{}");
        assert!(msg.contains("\\\"x\\\""));
        assert!(msg.contains("\"event\":{}"));
    }

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("a\"b"), "a\\\"b");
        assert_eq!(escape_json_string("a\\b"), "a\\\\b");
        assert_eq!(escape_json_string("a\nb"), "a\\nb");
        assert_eq!(escape_json_string("plain"), "plain");
    }
}
