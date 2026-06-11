use async_trait::async_trait;
use libp2p::futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use crate::q42_lexicon::{Q42Context, Q42CborLdParser, SemanticPayload, CborLdError};
#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::Q42Volume;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NQuin {
    pub subject: [u8; 8],
    pub predicate: [u8; 8],
    pub object: [u8; 8],
    pub context: [u8; 8],
    pub clock_sig: [u8; 16],
}
const _: () = assert!(std::mem::size_of::<NQuin>() == 48);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualiaRequest {
    Handshake {
        // CBOR-LD semantic payload with Q42 lexicon resolution
        #[serde(rename = "@context")]
        context: String,
        #[serde(rename = "type")]
        request_type: String,
        #[serde(rename = "did_q42")]
        did_q42: u64,
        #[serde(rename = "semantic_context")]
        semantic_context: u64,
        // Flattened buffer containing sequences of (48-byte Quin + 64-byte Ed25519 Signature)
        credentials: Vec<u8>,
    },
    Sync {
        // CBOR-LD semantic payload
        #[serde(rename = "@context")]
        context: String,
        #[serde(rename = "type")]
        request_type: String,
        #[serde(rename = "did_q42")]
        did_q42: u64,
        hop_count: u8,
        gatekeeper_token: Option<String>,
        #[serde(rename = "target_shapes")]
        target_shapes: Vec<String>,
        #[serde(rename = "routing_constraints")]
        routing_constraints: u8,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl QualiaRequest {
    /// Convert semantic payload to QualiaRequest
    pub fn from_semantic_payload(payload: SemanticPayload) -> Self {
        let did_q42 = match payload.did_q42 {
            Some(d) => crate::q_hash(&d),
            None => 0,
        };
        
        // Extract semantic context hash from HashMap
        let semantic_context = payload.semantic_context
            .get("context")
            .map(|s| crate::q_hash(s))
            .unwrap_or(0);
        
        Self::Handshake {
            context: "https://qualia.org/ld/context/v1".to_string(),
            request_type: "Handshake".to_string(),
            did_q42,
            semantic_context,
            credentials: Vec::new(), // TODO: Extract from payload
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualiaResponse {
    HandshakeAck {
        // CBOR-LD semantic response
        #[serde(rename = "@context")]
        context: String,
        #[serde(rename = "type")]
        response_type: String,
        success: bool,
        #[serde(rename = "did_q42")]
        did_q42: u64,
        #[serde(rename = "semantic_context")]
        semantic_context: u64,
    },
    SyncAck {
        // CBOR-LD semantic response
        #[serde(rename = "@context")]
        context: String,
        #[serde(rename = "type")]
        response_type: String,
        success: bool,
        message: String,
        blocks_sent: u64,
        #[serde(rename = "did_q42")]
        did_q42: u64,
        #[serde(rename = "routing_constraints")]
        routing_constraints: u8,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl QualiaResponse {
    /// Convert semantic payload to QualiaResponse
    pub fn from_semantic_payload(payload: SemanticPayload) -> Self {
        let did_q42 = match payload.did_q42 {
            Some(d) => crate::q_hash(&d),
            None => 0,
        };
        
        let semantic_context = payload.semantic_context
            .get("context")
            .map(|s| crate::q_hash(s))
            .unwrap_or(0);
        
        Self::HandshakeAck {
            context: "https://qualia.org/ld/context/v1".to_string(),
            response_type: "HandshakeAck".to_string(),
            success: true,
            did_q42,
            semantic_context,
        }
    }
}

#[derive(Clone)]
pub struct QualiaSyncCodec {
    #[cfg(not(target_arch = "wasm32"))]
    q42_context: Option<Arc<Q42Context>>,
    #[cfg(not(target_arch = "wasm32"))]
    cbor_ld_parser: Option<Arc<Q42CborLdParser>>,
}

impl Default for QualiaSyncCodec {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            q42_context: None,
            #[cfg(not(target_arch = "wasm32"))]
            cbor_ld_parser: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl QualiaSyncCodec {
    /// Initialize codec with Q42 volume for CBOR-LD support
    pub fn with_q42_volume(volume: &Q42Volume) -> Result<Self, CborLdError> {
        let context = Arc::new(Q42Context::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
        let parser = Arc::new(Q42CborLdParser::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
        
        Ok(Self {
            q42_context: Some(context),
            cbor_ld_parser: Some(parser),
        })
    }
    
    /// Get Q42 context reference
    pub fn q42_context(&self) -> Option<&Arc<Q42Context>> {
        self.q42_context.as_ref()
    }
    
    /// Get CBOR-LD parser reference
    pub fn cbor_ld_parser(&self) -> Option<&Arc<Q42CborLdParser>> {
        self.cbor_ld_parser.as_ref()
    }
}

#[async_trait]
impl Codec for QualiaSyncCodec {
    type Protocol = StreamProtocol;
    type Request = QualiaRequest;
    type Response = QualiaResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        // Try CBOR-LD parsing first if parser is available
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(semantic_payload) = parser.parse_semantic_payload(&buf) {
                // Convert semantic payload to QualiaRequest
                return Ok(Self::Request::from_semantic_payload(semantic_payload));
            }
        }
        
        // Fallback to regular CBOR parsing
        ciborium::from_reader(&buf[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        // Try CBOR-LD parsing first if parser is available
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(semantic_payload) = parser.parse_semantic_payload(&buf) {
                // Convert semantic payload to QualiaResponse
                return Ok(Self::Response::from_semantic_payload(semantic_payload));
            }
        }
        
        // Fallback to regular CBOR parsing
        ciborium::from_reader(&buf[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let mut buf = Vec::new();
        ciborium::into_writer(&req, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let len = buf.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&buf).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let mut buf = Vec::new();
        ciborium::into_writer(&res, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let len = buf.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&buf).await?;
        Ok(())
    }
}
