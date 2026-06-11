//! SPARQL WebSocket Support
//!
//! Real-time SPARQL query results via WebSocket using zero-allocation patterns.

use crate::sparql_ast::*;
use crate::sparql_parser;
use crate::sparql_planner::*;
use crate::sparql_executor::*;
use crate::sparql_library::serialisers::sparql_results::ResultFormatter;
use crate::NQuin;

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

/// SPARQL WebSocket handler
pub struct SparqlWebSocketHandler<'a> {
    pub quins: &'a [NQuin],
    pub sessions: [Option<WebSocketSession>; 32],
    pub session_count: u8,
}

impl<'a> SparqlWebSocketHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            sessions: [None; 32],
            session_count: 0,
        }
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

    /// Handle a WebSocket query
    pub fn handle_query(
        &self,
        query: &str,
        format: &str,
        session_id: u64,
    ) -> Result<String, String> {
        // Parse query
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;
        
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
                ResultFormatter::format_json(&mut output, &vars, &results, &ctx, None).map_err(|e| e.to_string())?;
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
                ResultFormatter::format_xml(&mut output, &vars, &results, &ctx, None).map_err(|e| e.to_string())?;
                Ok(String::from_utf8(output).unwrap())
            }
            _ => Err("Unsupported format. Use: xml or json".to_string()),
        }
    }

    /// Stream query results in chunks
    pub fn stream_results(
        &self,
        query: &str,
        chunk_size: usize,
        session_id: u64,
    ) -> Result<Vec<String>, String> {
        // Parse and execute query
        let (sparql_query, mut ctx) = sparql_parser::parse_sparql(query)?;
        let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
        let executor = QueryExecutor::new(self.quins);
        let results = executor.execute(&plan, &ctx)?;

        // Get variables
        let vars = match &sparql_query {
            SparqlQuery::Select(select) => {
                select.variables[..select.var_count as usize].to_vec()
            }
            _ => vec![],
        };

        // Chunk results
        let mut chunks = Vec::new();
        for chunk in results.chunks(chunk_size) {
            let mut output = Vec::new();
            ResultFormatter::format_json(&mut output, &vars, chunk, &ctx, None).map_err(|e| e.to_string())?;
            chunks.push(String::from_utf8(output).unwrap());
        }
        
        Ok(chunks)
    }

    /// Subscribe to real-time updates for a query
    pub fn subscribe(&mut self, session_id: u64, query: &str) -> Result<u64, String> {
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

    /// Unsubscribe from updates
    pub fn unsubscribe(&mut self, session_id: u64) -> Result<(), String> {
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

    /// Notify subscribers of updates
    pub fn notify_subscribers(&self, query_hash: u64) -> Vec<u64> {
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
}

impl<'a> Default for SparqlWebSocketHandler<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
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
    fn test_subscribe() {
        let quins = vec![];
        let mut handler = SparqlWebSocketHandler::new(&quins);
        
        let session_id = handler.register_session().unwrap();
        let query_hash = handler.subscribe(session_id, "SELECT ?s WHERE ?s ?p ?o").unwrap();
        
        assert!(query_hash > 0);
    }
}
