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

use js_sys::WebAssembly;

// WASM bindings for the Qualia-DB engine SharedArrayBuffer integration

#[wasm_bindgen]
pub struct QualiaWasmBridge {
    // We hold a reference to the strict 512MB pre-allocated WebAssembly.Memory object
    // that the WebWorker instantiated, ensuring zero dynamic memory growth.
    memory: WebAssembly::Memory,
}

#[wasm_bindgen]
impl QualiaWasmBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(memory: WebAssembly::Memory) -> Self {
        // Enforce strict memory bounds check (512MB = 8192 WASM pages, where 1 page = 64KB)
        // If the JS side doesn't allocate exactly 8192 pages, the engine panics.
        // For this phase, we accept the passed memory and assume JS correctly requested 512MB.
        Self { memory }
    }

    /// Initializes the Qualia-DB engine on the WebWorker thread.
    /// Expects the WebAssembly.Memory object to be pre-allocated to exactly 512MB.
    #[wasm_bindgen]
    pub fn qualia_init(memory: WebAssembly::Memory) -> QualiaWasmBridge {
        QualiaWasmBridge::new(memory)
    }

    /// Dispatches a query over the SharedArrayBuffer. 
    /// The offset points to the byte location in the SharedArrayBuffer where the engine should
    /// write the zero-allocation raw result pointers for the Main UI thread to read synchronously.
    #[wasm_bindgen]
    pub fn dispatch_query(&self, buffer_offset: u32, _query_id: u64) {
        // In a full implementation, this calls the Qualia Sieve and extracts the results.
        // Here, we simulate the Sieve completing and writing the raw result pointers 
        // into the SharedArrayBuffer at the provided offset.
        
        let mem_buffer = self.memory.buffer();
        // Create an Uint32Array view into the memory starting at `buffer_offset`
        let uint32_view = js_sys::Uint32Array::new_with_byte_offset_and_length(
            &mem_buffer,
            buffer_offset,
            4 // Mocking 4 return pointers
        );

        // Write the Sieve results (mock pointers to Quin arrays in memory)
        uint32_view.set_index(0, 0x1000);
        uint32_view.set_index(1, 0x1040);
        uint32_view.set_index(2, 0x1080);
        uint32_view.set_index(3, 0x10C0);

        // Telemetry update:
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }
}
