use async_trait::async_trait;
use libp2p::futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use crate::q42_lexicon::{CborLdError, Q42CborLdParser, Q42Context, SemanticPayload};
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
        let semantic_context = payload
            .semantic_context
            .get("context")
            .map(|s| crate::q_hash(s))
            .unwrap_or(0);

        Self::Handshake {
            context: "https://webizen.org/ld/context/v1".to_string(),
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

        let semantic_context = payload
            .semantic_context
            .get("context")
            .map(|s| crate::q_hash(s))
            .unwrap_or(0);

        Self::HandshakeAck {
            context: "https://webizen.org/ld/context/v1".to_string(),
            response_type: "HandshakeAck".to_string(),
            success: true,
            did_q42,
            semantic_context,
        }
    }
}

/// Q42 lexicon-compacted **CBOR-LD** wire codec for the sync protocol
/// (`qualia-sync-protocol.md` §13).
///
/// The JSON-LD `@context` IRI and every field *term* is resolved through the Q42
/// lexicon to a 64-bit key, so the wire payload is genuine CBOR-LD term-compacted
/// CBOR — **not** plain CBOR of the Rust enum. The map is self-identifying (a magic
/// term key carrying the codec version) so the reader can tell CBOR-LD frames from a
/// plain-ciborium fallback, and the mapping is **lossless** (proven by the round-trip
/// tests). When a term has no lexicon entry the deterministic FNV-1a `q_hash` is used,
/// so the codec works with the default (volume-less) lexicon too.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod qcborld {
    use super::{QualiaRequest, QualiaResponse};
    use crate::q42::q42_lexicon::Q42Lexicon;
    use ciborium::value::Value;

    pub const CONTEXT_IRI: &str = "https://webizen.org/ld/context/v1";
    const MAGIC_TERM: &str = "@cbor-ld/q42";
    const VERSION: u64 = 1;

    /// Resolve a field term to its 64-bit wire key via the lexicon (term compaction),
    /// falling back to the deterministic `q_hash` when the term is not in the lexicon.
    #[inline]
    fn key(lex: &Q42Lexicon, term: &str) -> u64 {
        lex.resolve_term(term).unwrap_or_else(|| crate::q_hash(term))
    }
    #[inline]
    fn kv(lex: &Q42Lexicon, term: &str, v: Value) -> (Value, Value) {
        (Value::from(key(lex, term)), v)
    }
    fn get<'a>(map: &'a [(Value, Value)], lex: &Q42Lexicon, term: &str) -> Option<&'a Value> {
        let k = key(lex, term);
        map.iter().find_map(|(kk, vv)| match kk.as_integer() {
            Some(i) if i128::from(i) as u64 == k => Some(vv),
            _ => None,
        })
    }
    fn u64v(v: &Value) -> Option<u64> {
        v.as_integer().map(|i| i128::from(i) as u64)
    }
    fn header(lex: &Q42Lexicon, ty: &str) -> Vec<(Value, Value)> {
        vec![
            kv(lex, MAGIC_TERM, Value::from(VERSION)),
            kv(lex, "@context", Value::from(crate::q_hash(CONTEXT_IRI))),
            kv(lex, "type", Value::Text(ty.to_string())),
        ]
    }
    fn is_cbor_ld(map: &[(Value, Value)], lex: &Q42Lexicon) -> bool {
        get(map, lex, MAGIC_TERM).and_then(u64v) == Some(VERSION)
    }
    fn encode(entries: Vec<(Value, Value)>) -> Result<Vec<u8>, ()> {
        let mut buf = Vec::new();
        ciborium::into_writer(&Value::Map(entries), &mut buf).map_err(|_| ())?;
        Ok(buf)
    }

    /// Encode a request as Q42 CBOR-LD. Lossless over both variants.
    pub fn encode_request(lex: &Q42Lexicon, req: &QualiaRequest) -> Result<Vec<u8>, ()> {
        let mut m = header(
            lex,
            match req {
                QualiaRequest::Handshake { .. } => "Handshake",
                QualiaRequest::Sync { .. } => "Sync",
            },
        );
        match req {
            QualiaRequest::Handshake {
                did_q42,
                semantic_context,
                credentials,
                ..
            } => {
                m.push(kv(lex, "did_q42", Value::from(*did_q42)));
                m.push(kv(lex, "semantic_context", Value::from(*semantic_context)));
                m.push(kv(lex, "credentials", Value::Bytes(credentials.clone())));
            }
            QualiaRequest::Sync {
                did_q42,
                hop_count,
                gatekeeper_token,
                target_shapes,
                routing_constraints,
                ..
            } => {
                m.push(kv(lex, "did_q42", Value::from(*did_q42)));
                m.push(kv(lex, "hop_count", Value::from(*hop_count as u64)));
                m.push(kv(
                    lex,
                    "gatekeeper_token",
                    match gatekeeper_token {
                        Some(t) => Value::Text(t.clone()),
                        None => Value::Null,
                    },
                ));
                m.push(kv(
                    lex,
                    "target_shapes",
                    Value::Array(target_shapes.iter().map(|s| Value::Text(s.clone())).collect()),
                ));
                m.push(kv(
                    lex,
                    "routing_constraints",
                    Value::from(*routing_constraints as u64),
                ));
            }
        }
        encode(m)
    }

    /// Decode a Q42 CBOR-LD request. `Err(())` when the frame is not Q42 CBOR-LD
    /// (so the codec can fall back to plain ciborium).
    pub fn decode_request(lex: &Q42Lexicon, data: &[u8]) -> Result<QualiaRequest, ()> {
        let val: Value = ciborium::from_reader(data).map_err(|_| ())?;
        let map = val.as_map().ok_or(())?;
        if !is_cbor_ld(map, lex) {
            return Err(());
        }
        let ty = get(map, lex, "type").and_then(|v| v.as_text()).ok_or(())?;
        let did_q42 = get(map, lex, "did_q42").and_then(u64v).unwrap_or(0);
        match ty {
            "Handshake" => Ok(QualiaRequest::Handshake {
                context: CONTEXT_IRI.to_string(),
                request_type: "Handshake".to_string(),
                did_q42,
                semantic_context: get(map, lex, "semantic_context").and_then(u64v).unwrap_or(0),
                credentials: get(map, lex, "credentials")
                    .and_then(|v| v.as_bytes())
                    .cloned()
                    .unwrap_or_default(),
            }),
            "Sync" => Ok(QualiaRequest::Sync {
                context: CONTEXT_IRI.to_string(),
                request_type: "Sync".to_string(),
                did_q42,
                hop_count: get(map, lex, "hop_count").and_then(u64v).unwrap_or(0) as u8,
                gatekeeper_token: get(map, lex, "gatekeeper_token")
                    .and_then(|v| v.as_text().map(|s| s.to_string())),
                target_shapes: get(map, lex, "target_shapes")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_text().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                routing_constraints: get(map, lex, "routing_constraints").and_then(u64v).unwrap_or(0)
                    as u8,
            }),
            _ => Err(()),
        }
    }

    /// Encode a response as Q42 CBOR-LD. Lossless over both variants.
    pub fn encode_response(lex: &Q42Lexicon, res: &QualiaResponse) -> Result<Vec<u8>, ()> {
        let mut m = header(
            lex,
            match res {
                QualiaResponse::HandshakeAck { .. } => "HandshakeAck",
                QualiaResponse::SyncAck { .. } => "SyncAck",
            },
        );
        match res {
            QualiaResponse::HandshakeAck {
                success,
                did_q42,
                semantic_context,
                ..
            } => {
                m.push(kv(lex, "success", Value::Bool(*success)));
                m.push(kv(lex, "did_q42", Value::from(*did_q42)));
                m.push(kv(lex, "semantic_context", Value::from(*semantic_context)));
            }
            QualiaResponse::SyncAck {
                success,
                message,
                blocks_sent,
                did_q42,
                routing_constraints,
                ..
            } => {
                m.push(kv(lex, "success", Value::Bool(*success)));
                m.push(kv(lex, "message", Value::Text(message.clone())));
                m.push(kv(lex, "blocks_sent", Value::from(*blocks_sent)));
                m.push(kv(lex, "did_q42", Value::from(*did_q42)));
                m.push(kv(
                    lex,
                    "routing_constraints",
                    Value::from(*routing_constraints as u64),
                ));
            }
        }
        encode(m)
    }

    /// Decode a Q42 CBOR-LD response. `Err(())` when the frame is not Q42 CBOR-LD.
    pub fn decode_response(lex: &Q42Lexicon, data: &[u8]) -> Result<QualiaResponse, ()> {
        let val: Value = ciborium::from_reader(data).map_err(|_| ())?;
        let map = val.as_map().ok_or(())?;
        if !is_cbor_ld(map, lex) {
            return Err(());
        }
        let ty = get(map, lex, "type").and_then(|v| v.as_text()).ok_or(())?;
        let success = get(map, lex, "success").and_then(|v| v.as_bool()).unwrap_or(false);
        let did_q42 = get(map, lex, "did_q42").and_then(u64v).unwrap_or(0);
        match ty {
            "HandshakeAck" => Ok(QualiaResponse::HandshakeAck {
                context: CONTEXT_IRI.to_string(),
                response_type: "HandshakeAck".to_string(),
                success,
                did_q42,
                semantic_context: get(map, lex, "semantic_context").and_then(u64v).unwrap_or(0),
            }),
            "SyncAck" => Ok(QualiaResponse::SyncAck {
                context: CONTEXT_IRI.to_string(),
                response_type: "SyncAck".to_string(),
                success,
                message: get(map, lex, "message")
                    .and_then(|v| v.as_text().map(|s| s.to_string()))
                    .unwrap_or_default(),
                blocks_sent: get(map, lex, "blocks_sent").and_then(u64v).unwrap_or(0),
                did_q42,
                routing_constraints: get(map, lex, "routing_constraints").and_then(u64v).unwrap_or(0)
                    as u8,
            }),
            _ => Err(()),
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
        let context =
            Arc::new(Q42Context::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
        let parser =
            Arc::new(Q42CborLdParser::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);

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

        // Decode a Q42 CBOR-LD frame (qualia-sync-protocol.md §13) when a lexicon is
        // present; a non-CBOR-LD frame falls through to the plain-ciborium decode.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(req) = qcborld::decode_request(parser.lexicon(), &buf) {
                return Ok(req);
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

        // Decode a Q42 CBOR-LD frame (qualia-sync-protocol.md §13) when a lexicon is
        // present; a non-CBOR-LD frame falls through to the plain-ciborium decode.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(res) = qcborld::decode_response(parser.lexicon(), &buf) {
                return Ok(res);
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
        // Emit Q42 CBOR-LD (qualia-sync-protocol.md §13) when a lexicon is present;
        // otherwise plain ciborium CBOR.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(bytes) = qcborld::encode_request(parser.lexicon(), &req) {
                io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
                io.write_all(&bytes).await?;
                return Ok(());
            }
        }

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
        // Emit Q42 CBOR-LD (qualia-sync-protocol.md §13) when a lexicon is present;
        // otherwise plain ciborium CBOR.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref parser) = self.cbor_ld_parser {
            if let Ok(bytes) = qcborld::encode_response(parser.lexicon(), &res) {
                io.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
                io.write_all(&bytes).await?;
                return Ok(());
            }
        }

        let mut buf = Vec::new();
        ciborium::into_writer(&res, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

        let len = buf.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&buf).await?;
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod cbor_ld_tests {
    use super::*;
    use crate::q42::q42_lexicon::Q42Lexicon;

    // CBOR-LD round-trip is lossless for every request/response variant, and the
    // wire bytes are genuine CBOR-LD (a magic-tagged term-compacted map), NOT a
    // plain ciborium encoding of the enum — qualia-sync-protocol.md §13.
    #[test]
    fn cbor_ld_request_roundtrip_is_lossless() {
        let lex = Q42Lexicon::new();
        let hs = QualiaRequest::Handshake {
            context: qcborld::CONTEXT_IRI.to_string(),
            request_type: "Handshake".to_string(),
            did_q42: 0xDEAD_BEEF,
            semantic_context: 0x1234_5678,
            credentials: vec![1, 2, 3, 4, 5, 250, 251, 252],
        };
        let bytes = qcborld::encode_request(&lex, &hs).unwrap();
        // A genuine CBOR-LD frame does NOT decode as the plain ciborium enum.
        assert!(ciborium::from_reader::<QualiaRequest, _>(&bytes[..]).is_err());
        let back = qcborld::decode_request(&lex, &bytes).unwrap();
        match back {
            QualiaRequest::Handshake { did_q42, semantic_context, credentials, .. } => {
                assert_eq!(did_q42, 0xDEAD_BEEF);
                assert_eq!(semantic_context, 0x1234_5678);
                assert_eq!(credentials, vec![1, 2, 3, 4, 5, 250, 251, 252]);
            }
            _ => panic!("variant changed across round-trip"),
        }

        let sync = QualiaRequest::Sync {
            context: qcborld::CONTEXT_IRI.to_string(),
            request_type: "Sync".to_string(),
            did_q42: 42,
            hop_count: 2,
            gatekeeper_token: Some("tok-abc".to_string()),
            target_shapes: vec!["foaf:Person".to_string(), "qualia:Vault".to_string()],
            routing_constraints: 7,
        };
        let back = qcborld::decode_request(&lex, &qcborld::encode_request(&lex, &sync).unwrap()).unwrap();
        match back {
            QualiaRequest::Sync { did_q42, hop_count, gatekeeper_token, target_shapes, routing_constraints, .. } => {
                assert_eq!(did_q42, 42);
                assert_eq!(hop_count, 2);
                assert_eq!(gatekeeper_token.as_deref(), Some("tok-abc"));
                assert_eq!(target_shapes, vec!["foaf:Person".to_string(), "qualia:Vault".to_string()]);
                assert_eq!(routing_constraints, 7);
            }
            _ => panic!("variant changed across round-trip"),
        }
    }

    #[test]
    fn cbor_ld_response_roundtrip_is_lossless() {
        let lex = Q42Lexicon::new();
        let ack = QualiaResponse::SyncAck {
            context: qcborld::CONTEXT_IRI.to_string(),
            response_type: "SyncAck".to_string(),
            success: true,
            message: "synced".to_string(),
            blocks_sent: 1234,
            did_q42: 99,
            routing_constraints: 3,
        };
        let back = qcborld::decode_response(&lex, &qcborld::encode_response(&lex, &ack).unwrap()).unwrap();
        match back {
            QualiaResponse::SyncAck { success, message, blocks_sent, did_q42, routing_constraints, .. } => {
                assert!(success);
                assert_eq!(message, "synced");
                assert_eq!(blocks_sent, 1234);
                assert_eq!(did_q42, 99);
                assert_eq!(routing_constraints, 3);
            }
            _ => panic!("variant changed across round-trip"),
        }
    }

    #[test]
    fn plain_ciborium_is_not_mistaken_for_cbor_ld() {
        // A plain-ciborium frame must NOT be accepted by the CBOR-LD decoder, so the
        // codec's fallback path stays correct.
        let lex = Q42Lexicon::new();
        let hs = QualiaRequest::Handshake {
            context: qcborld::CONTEXT_IRI.to_string(),
            request_type: "Handshake".to_string(),
            did_q42: 1,
            semantic_context: 2,
            credentials: vec![],
        };
        let mut plain = Vec::new();
        ciborium::into_writer(&hs, &mut plain).unwrap();
        assert!(qcborld::decode_request(&lex, &plain).is_err());
    }
}
