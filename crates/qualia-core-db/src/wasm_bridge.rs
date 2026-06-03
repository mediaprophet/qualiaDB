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

// ---------------------------------------------------------------------------
// Sentinel VM query execution — pre-fetch synchronous model
// ---------------------------------------------------------------------------
//
// The JS orchestrator (vfs.js) pre-fetches SuperBlock bytes via HTTP Range
// requests or OPFS and copies them into a Uint8Array before calling here.
// This avoids SharedArrayBuffer / Atomics.wait() which require COOP/COEP
// headers that GitHub Pages cannot serve.

/// Execute a single N-Triples pattern query against a pre-loaded SuperBlock.
///
/// # Arguments
/// * `query`    — N-Triples pattern, e.g. `?s <http://…/writtenRep> "dog"@en`
/// * `db_bytes` — raw bytes of one or more 40,960-byte SuperBlocks.  The JS
///               caller passes a `Uint8Array` view; wasm-bindgen copies it in.
/// * `max_results` — upper bound on the Quins returned (default: 256).
///
/// # Returns
/// A JSON string:
/// ```json
/// {
///   "matches": [{"s":"u64str","p":"u64str","o":"u64str","c":"u64str","m":"u64str"}, …],
///   "vm_cycles": 1234,
///   "direct_jump_ops": 0,
///   "lexicon_lookup_ops": 6
/// }
/// ```
/// All u64 field values are serialised as **decimal strings** so the JS side
/// can parse them losslessly with `BigInt(v)` without IEEE-754 truncation.
///
/// On error a `{"error":"..."}` object is returned instead.
#[wasm_bindgen]
pub fn execute_ntriples_query(query: &str, db_bytes: &[u8], max_results: u32) -> String {
    use crate::mini_parser::compile_ntriples_to_bytecode;
    use crate::sentinel_bytecode::execute_program_with_stats;
    use crate::QualiaQuin;

    const QUIN_SIZE: usize = 48;

    if db_bytes.len() % QUIN_SIZE != 0 {
        return format!(
            r#"{{"error":"db_bytes length {} is not a multiple of 48"}}"#,
            db_bytes.len()
        );
    }

    // Build a properly-aligned Vec<QualiaQuin> from the raw bytes using
    // unaligned reads (the Uint8Array from JS may start at any byte offset).
    let quin_count = db_bytes.len() / QUIN_SIZE;
    let mut db: Vec<QualiaQuin> = Vec::with_capacity(quin_count);
    for chunk in db_bytes.chunks_exact(QUIN_SIZE) {
        // Safety: `chunk` is exactly 48 bytes; read_unaligned handles any alignment.
        let q = unsafe { core::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
        db.push(q);
    }

    let mut prog = [0u8; 1024];
    if let Err(e) = compile_ntriples_to_bytecode(query.as_bytes(), &mut prog) {
        return format!(r#"{{"error":"parse error: {:?}"}}"#, e);
    }

    let max = (max_results as usize).min(4096);
    let mut out = vec![QualiaQuin::default(); max];

    match execute_program_with_stats(&prog, &db, &mut out) {
        Ok(stats) => {
            let matches: Vec<String> = out[..stats.match_count]
                .iter()
                .map(|q| {
                    format!(
                        r#"{{"s":"{}","p":"{}","o":"{}","c":"{}","m":"{}"}}"#,
                        q.subject, q.predicate, q.object, q.context, q.metadata
                    )
                })
                .collect();
            format!(
                r#"{{"matches":[{}],"vm_cycles":{},"direct_jump_ops":{},"lexicon_lookup_ops":{}}}"#,
                matches.join(","),
                stats.vm_cycles,
                stats.direct_jump_ops,
                stats.lexicon_lookup_ops,
            )
        }
        Err(e) => format!(r#"{{"error":"{:?}"}}"#, e),
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
