use wasm_bindgen::prelude::*;
use crate::query_compiler::QueryCompiler;
use crate::QualiaQuin;

#[wasm_bindgen]
pub fn compile_query_to_json(query: &str) -> String {
    let quin_opt = QueryCompiler::compile_to_quin(query);
    
    match quin_opt {
        Some(quin) => {
            let routing_tier = (quin.metadata >> 61) & 0b11;
            let validation_mask = quin.metadata & 0xFFFF;
            
            format!(
                r#"{{
  "quin": {{
    "subject": "{}",
    "predicate": "{}",
    "object": "{}",
    "context": "{}",
    "metadata": "{}",
    "parity": "{}"
  }},
  "routing_tier": {},
  "validation_mask": {}
}}"#,
                quin.subject,
                quin.predicate,
                quin.object,
                quin.context,
                quin.metadata,
                quin.parity,
                routing_tier,
                validation_mask
            )
        },
        None => String::from(r#"{"error": "Compilation failed or empty query"}"#)
    }
}
