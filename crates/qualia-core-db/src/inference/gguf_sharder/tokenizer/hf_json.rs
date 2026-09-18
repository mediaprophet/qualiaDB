//! Hugging Face `tokenizer.json` parser.
//!
//! Reconstructs a full BPE/ByteLevel `GgufTokenizer` from sibling `tokenizer.json` files,
//! preventing fallback to 256-byte dummy tokenizers when loading Safetensors models.

use serde_json::Value;
use super::GgufTokenizer;

/// Parse Hugging Face `tokenizer.json` into a `GgufTokenizer`.
pub fn parse_hf_tokenizer_json(json_str: &str) -> Option<GgufTokenizer> {
    let root: Value = serde_json::from_str(json_str).ok()?;
    let obj = root.as_object()?;

    // 1. Locate vocabulary
    // Usually under root["model"]["vocab"] or root["vocab"]
    let model_obj = obj.get("model").and_then(|m| m.as_object());
    let vocab_val = model_obj
        .and_then(|m| m.get("vocab"))
        .or_else(|| obj.get("vocab"))?;

    let mut token_id_pairs: Vec<(String, usize)> = Vec::new();
    if let Some(vocab_map) = vocab_val.as_object() {
        for (token, id_val) in vocab_map {
            if let Some(id) = id_val.as_u64() {
                token_id_pairs.push((token.clone(), id as usize));
            }
        }
    } else if let Some(vocab_arr) = vocab_val.as_array() {
        for (i, v) in vocab_arr.iter().enumerate() {
            if let Some(token) = v.as_str() {
                token_id_pairs.push((token.to_string(), i));
            }
        }
    }

    if token_id_pairs.is_empty() {
        return None;
    }

    // 2. Added / special tokens
    let mut special_bos: Option<u32> = None;
    let mut special_eos: Option<u32> = None;

    if let Some(added) = obj.get("added_tokens").and_then(|a| a.as_array()) {
        for item in added {
            let id = item.get("id").and_then(|v| v.as_u64()).map(|v| v as usize);
            let content = item.get("content").and_then(|v| v.as_str());
            if let (Some(id), Some(content)) = (id, content) {
                token_id_pairs.push((content.to_string(), id));
                match content {
                    "<s>" | "<bos>" | "<|im_start|>" | "<|begin_of_text|>" | "[BOS]" => {
                        special_bos = Some(id as u32);
                    }
                    "</s>" | "<eos>" | "<|im_end|>" | "<|endoftext|>" | "<|eot_id|>" | "<end_of_turn>" | "[EOS]" => {
                        special_eos = Some(id as u32);
                    }
                    _ => {}
                }
            }
        }
    }

    // 3. Merges
    let merges_val = model_obj
        .and_then(|m| m.get("merges"))
        .or_else(|| obj.get("merges"));

    let merges_raw: Option<Vec<String>> = merges_val.and_then(|m| {
        m.as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
    });

    // 4. Pre-tokenizer type
    let pre_type = obj
        .get("pre_tokenizer")
        .and_then(|p| p.get("type"))
        .and_then(|t| t.as_str())
        .map(String::from);

    // 5. Build dense vocab vector
    let max_id = token_id_pairs.iter().map(|(_, id)| *id).max().unwrap_or(0);
    let mut vocab = vec![String::new(); max_id + 1];
    for (token, id) in token_id_pairs {
        if id < vocab.len() {
            vocab[id] = token;
        }
    }

    // Fill any gaps with byte-level fallback tokens
    for (i, item) in vocab.iter_mut().enumerate() {
        if item.is_empty() {
            *item = format!("<0x{:02X}>", (i % 256) as u8);
        }
    }

    let bos_id = special_bos.unwrap_or(1);
    let eos_id = special_eos.unwrap_or(2);

    Some(GgufTokenizer::from_raw_components(
        vocab,
        merges_raw.as_deref(),
        Some(bos_id),
        Some(eos_id),
        Some(true),
        pre_type,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hf_tokenizer_json() {
        let json_sample = r#"{
            "version": "1.0",
            "model": {
                "type": "BPE",
                "vocab": {
                    "<unk>": 0,
                    "hello": 1,
                    "world": 2,
                    "!": 3
                },
                "merges": [
                    "h ello"
                ]
            },
            "added_tokens": [
                {
                    "id": 4,
                    "content": "<|im_start|>",
                    "special": true
                },
                {
                    "id": 5,
                    "content": "<|im_end|>",
                    "special": true
                }
            ]
        }"#;

        let tok = parse_hf_tokenizer_json(json_sample).expect("parse hf json");
        assert_eq!(tok.vocab.len(), 6);
        assert_eq!(tok.vocab[1], "hello");
        assert_eq!(tok.vocab[2], "world");
        assert_eq!(tok.vocab[4], "<|im_start|>");
        assert_eq!(tok.vocab[5], "<|im_end|>");
        assert_eq!(tok.bos_token_id, 4);
        assert_eq!(tok.eos_token_id, 5);
        assert!(tok.is_stop_token(5));
    }
}
