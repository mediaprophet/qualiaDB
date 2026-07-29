use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokenMetadata {
    pub token_ticker: String,
    pub token_name: String,
    pub token_document_url: String,
    pub token_document_hash: String,
    pub decimals: u8,
}

pub fn generate_slp_op_return(metadata: &SemanticTokenMetadata) -> Vec<u8> {
    let mut payload = Vec::new();

    payload.push(0x6a); // OP_RETURN
    payload.extend_from_slice(b"\x04SLP\x00"); // LOKAD ID
    payload.extend_from_slice(b"\x01\x01"); // Token Type
    payload.extend_from_slice(b"\x07GENESIS");

    let ticker_bytes = metadata.token_ticker.as_bytes();
    payload.push(ticker_bytes.len() as u8);
    payload.extend_from_slice(ticker_bytes);

    let name_bytes = metadata.token_name.as_bytes();
    payload.push(name_bytes.len() as u8);
    payload.extend_from_slice(name_bytes);

    let url_bytes = metadata.token_document_url.as_bytes();
    payload.push(url_bytes.len() as u8);
    payload.extend_from_slice(url_bytes);

    let hash_bytes = hex::decode(&metadata.token_document_hash).unwrap_or_else(|_| vec![0; 32]);
    payload.push(hash_bytes.len() as u8);
    payload.extend_from_slice(&hash_bytes);

    payload.extend_from_slice(&[0x01, metadata.decimals]);

    payload
}

pub fn generate_slp_send_op_return(token_id: &str, amounts: &[u64]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x6a); // OP_RETURN
    payload.extend_from_slice(b"\x04SLP\x00"); // LOKAD ID
    payload.extend_from_slice(b"\x01\x01"); // Token Type
    payload.extend_from_slice(b"\x04SEND");

    let token_id_bytes = hex::decode(token_id).unwrap_or_else(|_| vec![0; 32]);
    payload.push(token_id_bytes.len() as u8);
    payload.extend_from_slice(&token_id_bytes);

    for &amount in amounts {
        payload.push(0x08);
        payload.extend_from_slice(&amount.to_be_bytes());
    }

    payload
}

pub fn generate_alp_send_op_return(token_id: &str, amounts: &[u64]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x6a); // OP_RETURN
    payload.extend_from_slice(b"\x04ALP\x00"); // LOKAD ID
    payload.extend_from_slice(b"\x01\x01"); // Token Type
    payload.extend_from_slice(b"\x04SEND");

    let token_id_bytes = hex::decode(token_id).unwrap_or_else(|_| vec![0; 32]);
    payload.push(token_id_bytes.len() as u8);
    payload.extend_from_slice(&token_id_bytes);

    for &amount in amounts {
        payload.push(0x08);
        payload.extend_from_slice(&amount.to_be_bytes());
    }

    payload
}

pub fn generate_semantic_metadata_cbor(
    subject_did: &str,
    properties: serde_json::Value,
) -> Result<Vec<u8>, String> {
    let doc = serde_json::json!({
        "@context": "https://qualia.io/contexts/semantic-tokens-v1.jsonld",
        "@id": subject_did,
        "@type": "SemanticToken",
        "properties": properties
    });

    let mut buffer = Vec::new();
    ciborium::into_writer(&doc, &mut buffer).map_err(|e| e.to_string())?;
    Ok(buffer)
}
