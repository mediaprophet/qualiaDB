//! Minimal WASM exports for the Qualia portal hot path (`.q42` / JSON scene load).

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_cbor_ld_wasm(payload: &[u8]) -> JsValue {
    match crate::cbor_compiler::parse_cbor_ld_to_quin(payload) {
        Ok(q) => {
            #[derive(Serialize)]
            struct QOut {
                subject: String,
                predicate: String,
                object: String,
                context: String,
            }
            let out = QOut {
                subject: q.subject.to_string(),
                predicate: q.predicate.to_string(),
                object: q.object.to_string(),
                context: q.context.to_string(),
            };
            serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
        }
        Err(_) => JsValue::NULL,
    }
}

#[derive(Deserialize)]
struct JsonLdFlatTriple {
    s: String,
    p: String,
    o: String,
}

#[wasm_bindgen]
pub fn parse_json_wasm(payload: &str) -> JsValue {
    if let Ok(triples) = serde_json::from_str::<Vec<JsonLdFlatTriple>>(payload) {
        #[derive(Serialize)]
        struct QOut {
            subject: String,
            predicate: String,
            object: String,
        }

        let mut out = Vec::new();
        for t in triples {
            out.push(QOut {
                subject: t.s,
                predicate: t.p,
                object: t.o,
            });
        }
        serde_wasm_bindgen::to_value(&out).unwrap_or(JsValue::NULL)
    } else {
        JsValue::NULL
    }
}