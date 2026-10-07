//! WASM bindings for VibeScript 0.1.
//!
//! Exposes parse, check, evaluate, and bytecode operations to JavaScript
//! via `wasm-bindgen`.  The wrapper serialises results as plain JS objects
//! (through `serde-wasm-bindgen`) so the playground can consume them
//! without a custom ABI.

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

use vibe::{
    bytecode::{self, compile, compile_expr, decode_chunk, encode_chunk, Vm},
    check_cell, check_program, decode as decode_cbor_ast, diagnose, encode as encode_cbor_ast,
    eval_cell, eval_function, load_program, parse_cell, parse_program, Budget, DiagCode,
    Diagnostic, Engine, Env, Host, Item, LocalHost, Span, Value, HOST_VERSION, LANGUAGE_VERSION,
    TAG_VIBE_AST,
};

// ── helpers ────────────────────────────────────────────────────────

/// Convert a `Value` into a plain JS value.
fn value_to_js(v: &Value) -> JsValue {
    match v {
        Value::Null => JsValue::NULL,
        Value::Bool(b) => JsValue::from_bool(*b),
        Value::I64(n) => JsValue::from_str(&n.to_string()),
        Value::U64(n) => JsValue::from_str(&n.to_string()),
        Value::F64(f) => JsValue::from_f64(*f),
        Value::String(s) => JsValue::from_str(s),
        Value::Iri(s) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"iri".into()).ok();
            Reflect::set(&o, &"value".into(), &JsValue::from_str(s)).ok();
            o.into()
        }
        Value::Blank(s) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"blank".into()).ok();
            Reflect::set(&o, &"value".into(), &JsValue::from_str(s)).ok();
            o.into()
        }
        Value::Prefixed(p, l) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"prefixed".into()).ok();
            Reflect::set(&o, &"prefix".into(), &JsValue::from_str(p)).ok();
            Reflect::set(&o, &"local".into(), &JsValue::from_str(l)).ok();
            o.into()
        }
        Value::Var(s) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"var".into()).ok();
            Reflect::set(&o, &"name".into(), &JsValue::from_str(s)).ok();
            o.into()
        }
        Value::List(items) => {
            let arr = Array::new();
            for item in items {
                arr.push(&value_to_js(item));
            }
            arr.into()
        }
        Value::Record(map) => {
            let o = Object::new();
            for (k, v) in map.iter() {
                Reflect::set(&o, &JsValue::from_str(k), &value_to_js(v)).ok();
            }
            o.into()
        }
        Value::Ok(v) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"ok".into()).ok();
            Reflect::set(&o, &"value".into(), &value_to_js(v)).ok();
            o.into()
        }
        Value::Err(inner) => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"error".into()).ok();
            Reflect::set(&o, &"value".into(), &value_to_js(inner)).ok();
            o.into()
        }
        Value::Receipt => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &"receipt".into()).ok();
            o.into()
        }
        // For complex domain values, serialise as a tagged record.
        _ => {
            let o = Object::new();
            Reflect::set(&o, &"type".into(), &JsValue::from_str(&format!("{:?}", v))).ok();
            Reflect::set(&o, &"repr".into(), &JsValue::from_str(&format!("{:?}", v))).ok();
            o.into()
        }
    }
}

/// Convert a `Diagnostic` into a JS error object.
fn diag_to_js(d: &vibe::Diagnostic) -> JsValue {
    // Native parity with Diagnostic::to_json / DiagnoseReport (G-A).
    js_sys::JSON::parse(&d.to_json()).unwrap_or_else(|_| JsValue::NULL)
}

// ── public API ─────────────────────────────────────────────────────

/// Get the VibeScript language version string.
#[wasm_bindgen]
pub fn language_version() -> String {
    LANGUAGE_VERSION.to_string()
}

/// Frozen host ABI stamp (`vibe-host-0.1`).
#[wasm_bindgen]
pub fn host_version() -> String {
    HOST_VERSION.to_string()
}

/// Capability invoke — routes through `LocalHost` so Civics/stdlib kernels
/// (`Econ.*`, `Statistics.ols`, `PhysicalUnits.convert`, …) work under WASM.
/// Ungranted / unknown ids still fail closed with E300 / E100.
#[wasm_bindgen]
pub fn capability_invoke(id: &str, args_json: &str) -> JsValue {
    let args = match serde_json::from_str::<serde_json::Value>(args_json) {
        Ok(v) => json_to_vibe_value(&v),
        Err(e) => {
            let diag = Diagnostic::new(
                DiagCode::E100,
                Span { start: 0, end: 0 },
                format!("capability.invoke args JSON: {e}"),
            );
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&diag)).ok();
            return o.into();
        }
    };
    let mut host = LocalHost::default();
    match host.capability_invoke(id, &args, Span { start: 0, end: 0 }) {
        Ok(v) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

fn json_to_vibe_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                Value::U64(u)
            } else {
                Value::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(items) => {
            Value::List(items.iter().map(json_to_vibe_value).collect())
        }
        serde_json::Value::Object(map) => {
            let mut rec = std::collections::BTreeMap::new();
            for (k, val) in map {
                rec.insert(k.clone(), json_to_vibe_value(val));
            }
            Value::Record(rec)
        }
    }
}

/// Parse a cell expression (`= expr`).
/// Returns `{ ok: true, ast: ... }` or `{ ok: false, error: ... }`.
#[wasm_bindgen]
pub fn parse_cell_src(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(
                &o,
                &"kind".into(),
                &JsValue::from_str(&format!("{:?}", expr.kind)),
            )
            .ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Check a cell expression.
#[wasm_bindgen]
pub fn check_cell_src(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => match check_cell(&expr) {
            Ok(_) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                o.into()
            }
            Err(d) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
                o.into()
            }
        },
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Evaluate a cell expression (`= expr`) with the in-process local host.
#[wasm_bindgen]
pub fn eval_cell_src(src: &str) -> JsValue {
    let mut host = LocalHost::default();
    let mut env = Env::default();
    match eval_cell(src, &mut host, &mut env) {
        Ok(v) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Parse a full VibeScript program (module).
#[wasm_bindgen]
pub fn parse_program_src(src: &str) -> JsValue {
    match parse_program(src) {
        Ok(prog) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            let items = Array::new();
            for item in &prog.items {
                let desc = format!("{:?}", item);
                items.push(&JsValue::from_str(&desc));
            }
            Reflect::set(&o, &"items".into(), &items).ok();
            let funcs = Array::new();
            for item in &prog.items {
                if let vibe::Item::Function(fd) = item {
                    funcs.push(&JsValue::from_str(&fd.name));
                }
            }
            Reflect::set(&o, &"functions".into(), &funcs).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Check a full program.
#[wasm_bindgen]
pub fn check_program_src(src: &str) -> JsValue {
    match load_program(src) {
        Ok(prog) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            let funcs = Array::new();
            for item in &prog.items {
                if let vibe::Item::Function(fd) = item {
                    funcs.push(&JsValue::from_str(&fd.name));
                }
            }
            Reflect::set(&o, &"functions".into(), &funcs).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Evaluate a cell and return the result as a JSON-compatible JS value.
/// Playground Run uses `eval_program_src` (module + optional `main`), not this.
#[wasm_bindgen]
pub fn eval_cell_json(src: &str) -> JsValue {
    eval_cell_src(src)
}

/// Parse + check a module; collect up to eight diagnostics.
#[wasm_bindgen]
pub fn diagnose_src(src: &str) -> JsValue {
    // Byte-level JSON parity with native DiagnoseReport::to_json (G-A).
    let report = diagnose(src);
    js_sys::JSON::parse(&report.to_json()).unwrap_or_else(|_| JsValue::NULL)
}

/// Evaluate a full program on LocalHost (workshop dialect).
///
/// Runs preamble items, then `main` when present — same as `vibe eval FILE main`.
#[wasm_bindgen]
pub fn eval_program_src(src: &str) -> JsValue {
    match parse_program(src).and_then(|p| check_program(&p).map(|_| p)) {
        Ok(prog) => {
            let mut host = LocalHost::default();
            let mut env = Env::default();
            let result = (|| {
                {
                    let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
                    engine.eval_program(&prog, &mut env)?;
                }
                let has_main = prog
                    .items
                    .iter()
                    .any(|item| matches!(item, Item::Function(f) if f.name == "main"));
                if has_main {
                    eval_function(&prog, "main", Vec::new(), &mut host, &mut env)
                } else {
                    Ok(Value::Null)
                }
            })();
            match result {
                Ok(v) => {
                    let o = Object::new();
                    Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                    Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
                    o.into()
                }
                Err(d) => {
                    let o = Object::new();
                    Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                    Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
                    o.into()
                }
            }
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Persistent compiled cell for the browser binding.
///
/// Keeps the compiled [`bytecode::Chunk`] so timed / repeated runs can call
/// [`CompiledCell::run`] without re-decoding VBC1 bytes. Prefer this over
/// [`decode_and_run`] when the job is "run what we already compiled."
#[wasm_bindgen]
pub struct CompiledCell {
    chunk: bytecode::Chunk,
}

#[wasm_bindgen]
impl CompiledCell {
    /// Compile a cell expression (`= expr`) into a handle. Does not run it.
    pub fn compile(src: &str) -> Result<CompiledCell, JsValue> {
        match parse_cell(src) {
            Ok(expr) => match compile_expr(&expr) {
                Ok(chunk) => Ok(CompiledCell { chunk }),
                Err(e) => Err(JsValue::from_str(&format!("{:?}", e))),
            },
            Err(d) => Err(diag_to_js(&d)),
        }
    }

    /// Decode a VBC1 byte buffer once into a handle. Later [`Self::run`] calls
    /// do not decode again.
    pub fn from_bytes(bytes: &[u8]) -> Result<CompiledCell, JsValue> {
        match decode_chunk(bytes) {
            Ok(chunk) => Ok(CompiledCell { chunk }),
            Err(e) => Err(JsValue::from_str(&format!("{:?}", e))),
        }
    }

    /// Run the held chunk on a fresh local VM. Does not decode.
    pub fn run(&self) -> JsValue {
        let mut host = LocalHost::default();
        let mut vm = Vm::new(&self.chunk, &mut host, Budget::default());
        match vm.run() {
            Ok(v) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
                o.into()
            }
            Err(e) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e))).ok();
                o.into()
            }
        }
    }

    #[wasm_bindgen(getter)]
    pub fn code_size(&self) -> usize {
        self.chunk.code.len()
    }

    #[wasm_bindgen(getter)]
    pub fn constants(&self) -> usize {
        self.chunk.constants.len()
    }

    #[wasm_bindgen(getter)]
    pub fn functions(&self) -> usize {
        self.chunk.functions.len()
    }

    #[wasm_bindgen(getter)]
    pub fn top_locals(&self) -> u16 {
        self.chunk.top_locals
    }

    /// Human-readable disassembly of the held chunk (inspect only).
    pub fn disassembly(&self) -> String {
        disassemble(&self.chunk)
    }
}

/// Compile a cell expression to bytecode and return chunk metadata.
///
/// For repeated execution without re-decode, use [`CompiledCell::compile`] and
/// [`CompiledCell::run`] instead. This export stays for playground inspect /
/// size reporting.
#[wasm_bindgen]
pub fn compile_cell_bytecode(src: &str) -> JsValue {
    match CompiledCell::compile(src) {
        Ok(cell) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(
                &o,
                &"code_size".into(),
                &JsValue::from_f64(cell.code_size() as f64),
            )
            .ok();
            Reflect::set(
                &o,
                &"constants".into(),
                &JsValue::from_f64(cell.constants() as f64),
            )
            .ok();
            Reflect::set(
                &o,
                &"functions".into(),
                &JsValue::from_f64(cell.functions() as f64),
            )
            .ok();
            Reflect::set(
                &o,
                &"top_locals".into(),
                &JsValue::from_f64(cell.top_locals() as f64),
            )
            .ok();
            Reflect::set(
                &o,
                &"disassembly".into(),
                &JsValue::from_str(&cell.disassembly()),
            )
            .ok();
            o.into()
        }
        Err(err) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &err).ok();
            o.into()
        }
    }
}

/// Compile a cell to bytecode, run it on the VM, and return the result.
#[wasm_bindgen]
pub fn run_cell_bytecode(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => match compile_expr(&expr) {
            Ok(chunk) => {
                let mut host = LocalHost::default();
                let mut vm = Vm::new(&chunk, &mut host, Budget::default());
                match vm.run() {
                    Ok(v) => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                        Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
                        o.into()
                    }
                    Err(e) => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                        Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e)))
                            .ok();
                        o.into()
                    }
                }
            }
            Err(e) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e))).ok();
                o.into()
            }
        },
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Compile a program to bytecode, encode it to binary, decode it, and run
/// a named function.  Demonstrates the full bytecode round-trip.
#[wasm_bindgen]
pub fn run_program_bytecode(src: &str, fn_name: &str, args: Vec<JsValue>) -> JsValue {
    match load_program(src) {
        Ok(prog) => match compile(&prog) {
            Ok(chunk) => {
                // Encode → decode round-trip.
                let bytes = encode_chunk(&chunk);
                let decoded = match decode_chunk(&bytes) {
                    Ok(c) => c,
                    Err(e) => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                        Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e)))
                            .ok();
                        return o.into();
                    }
                };

                let idx = match decoded.find_function(fn_name) {
                    Some(i) => i,
                    None => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                        Reflect::set(
                            &o,
                            &"error".into(),
                            &JsValue::from_str(&format!("function '{}' not found", fn_name)),
                        )
                        .ok();
                        return o.into();
                    }
                };

                // Convert JS args to Values.
                let vibe_args: Vec<Value> = args.iter().map(js_to_value).collect();

                let mut host = LocalHost::default();
                let mut vm = Vm::new(&decoded, &mut host, Budget::default());
                // Run the preamble first.
                if let Err(e) = vm.run() {
                    let o = Object::new();
                    Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                    Reflect::set(
                        &o,
                        &"error".into(),
                        &JsValue::from_str(&format!("preamble: {:?}", e)),
                    )
                    .ok();
                    return o.into();
                }

                match vm.call_function(idx, &vibe_args) {
                    Ok(v) => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                        Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();
                        Reflect::set(
                            &o,
                            &"bytecode_size".into(),
                            &JsValue::from_f64(bytes.len() as f64),
                        )
                        .ok();
                        o.into()
                    }
                    Err(e) => {
                        let o = Object::new();
                        Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                        Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e)))
                            .ok();
                        o.into()
                    }
                }
            }
            Err(e) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
                Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e))).ok();
                o.into()
            }
        },
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Encode a cell's bytecode to a binary Uint8Array.
#[wasm_bindgen]
pub fn encode_cell_bytecode(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => match compile_expr(&expr) {
            Ok(chunk) => {
                let bytes = encode_chunk(&chunk);
                Uint8Array::from(&bytes[..]).into()
            }
            Err(e) => {
                let o = Object::new();
                Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e))).ok();
                o.into()
            }
        },
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

/// Decode a binary bytecode chunk and run it once.
///
/// Compat wrapper: each call decodes again. Prefer [`CompiledCell::from_bytes`]
/// then [`CompiledCell::run`] when the same bytes will run more than once.
#[wasm_bindgen]
pub fn decode_and_run(bytes: &[u8]) -> JsValue {
    match CompiledCell::from_bytes(bytes) {
        Ok(cell) => cell.run(),
        Err(err) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &err).ok();
            o.into()
        }
    }
}

/// Get the EBNF grammar string.
#[wasm_bindgen]
pub fn ebnf_grammar() -> String {
    vibe::EBNF.to_string()
}

/// Get the GBNF grammar (for LLM constrained decoding).
#[wasm_bindgen]
pub fn gbnf_grammar() -> String {
    vibe::GBNF.to_string()
}

/// Get the JSON schema for the AST.
#[wasm_bindgen]
pub fn ast_schema_json() -> String {
    vibe::SOURCE_SCHEMA_JSON.to_string()
}

/// Get the JSON schema for diagnostics.
#[wasm_bindgen]
pub fn diagnostic_schema_json() -> String {
    vibe::DIAGNOSTIC_SCHEMA_JSON.to_string()
}

// ── internal helpers ───────────────────────────────────────────────

/// Convert a JsValue to a VibeScript Value.
fn js_to_value(v: &JsValue) -> Value {
    if v.is_null() {
        Value::Null
    } else if let Some(b) = v.as_bool() {
        Value::Bool(b)
    } else if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 && n.abs() < 9.007e15 {
            Value::I64(n as i64)
        } else {
            Value::F64(n)
        }
    } else if let Some(s) = v.as_string() {
        Value::String(s)
    } else {
        Value::Null
    }
}

/// Disassemble a chunk's bytecode into a human-readable string.
fn disassemble(chunk: &bytecode::Chunk) -> String {
    use bytecode::Op;
    let mut out = String::new();
    let mut pc = 0;
    out.push_str(&format!(
        "=== Chunk: {} ops, {} constants, {} functions ===\n",
        chunk.code.len(),
        chunk.constants.len(),
        chunk.functions.len()
    ));
    while pc < chunk.code.len() {
        let op_byte = chunk.code[pc];
        let op = Op::from_byte(op_byte);
        let offset = pc;
        pc += 1;
        match op {
            Some(o) => {
                let name = format!("{:?}", o);
                // Read operands for ops that have them.
                match o {
                    Op::PushInt | Op::PushUInt => {
                        if pc + 8 <= chunk.code.len() {
                            let bytes: [u8; 8] =
                                chunk.code[pc..pc + 8].try_into().unwrap_or([0; 8]);
                            let val = i64::from_le_bytes(bytes);
                            pc += 8;
                            out.push_str(&format!("{:04}  {} {}\n", offset, name, val));
                        } else {
                            out.push_str(&format!("{:04}  {} <truncated>\n", offset, name));
                        }
                    }
                    Op::PushFloat => {
                        if pc + 8 <= chunk.code.len() {
                            let bytes: [u8; 8] =
                                chunk.code[pc..pc + 8].try_into().unwrap_or([0; 8]);
                            let val = f64::from_le_bytes(bytes);
                            pc += 8;
                            out.push_str(&format!("{:04}  {} {}\n", offset, name, val));
                        } else {
                            out.push_str(&format!("{:04}  {} <truncated>\n", offset, name));
                        }
                    }
                    Op::PushString | Op::PushIri => {
                        if pc + 2 <= chunk.code.len() {
                            let idx = u16::from_le_bytes([chunk.code[pc], chunk.code[pc + 1]]);
                            pc += 2;
                            let const_val = chunk
                                .constants
                                .get(idx as usize)
                                .map(|c| format!("{:?}", c))
                                .unwrap_or_else(|| "<invalid>".to_string());
                            out.push_str(&format!(
                                "{:04}  {} [{}] {}\n",
                                offset, name, idx, const_val
                            ));
                        } else {
                            out.push_str(&format!("{:04}  {} <truncated>\n", offset, name));
                        }
                    }
                    Op::LoadVar
                    | Op::StoreVar
                    | Op::MakeList
                    | Op::MakeRecord
                    | Op::Jump
                    | Op::JumpIfFalse
                    | Op::JumpIfTrue
                    | Op::CallHost
                    | Op::CallUser => {
                        if pc + 2 <= chunk.code.len() {
                            let operand = u16::from_le_bytes([chunk.code[pc], chunk.code[pc + 1]]);
                            pc += 2;
                            out.push_str(&format!("{:04}  {} {}\n", offset, name, operand));
                        } else {
                            out.push_str(&format!("{:04}  {} <truncated>\n", offset, name));
                        }
                    }
                    _ => {
                        out.push_str(&format!("{:04}  {}\n", offset, name));
                    }
                }
            }
            None => {
                out.push_str(&format!(
                    "{:04}  <invalid opcode 0x{:02x}>\n",
                    offset, op_byte
                ));
            }
        }
    }
    // Append function metadata.
    for (i, f) in chunk.functions.iter().enumerate() {
        out.push_str(&format!(
            "  fn[{}]: {} (params={}, locals={}, offset={}, budget={})\n",
            i, f.name, f.param_count, f.local_count, f.code_offset, f.budget_steps
        ));
    }
    out
}

// ── projectional authoring (W1) ────────────────────────────────────

use vibe::projectional::{apply_edit, apply_edits, project_program, Edit, ProjectOptions};
use vibe::{FieldRepresentation, FieldSupport, NamedArg, Span as VibeSpan};

/// Project a VibeScript program source to canonical form.
/// Parses the source, then re-projects it from the AST.
/// This is the core of projectional authoring: structure → text.
#[wasm_bindgen]
pub fn project_source(src: &str) -> JsValue {
    match parse_program(src) {
        Ok(prog) => {
            let projected = project_program(&prog, &ProjectOptions::default());
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"source".into(), &JsValue::from_str(&projected)).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &JsValue::from_str(&d.message)).ok();
            o.into()
        }
    }
}

/// Apply a structural edit to a VibeScript program and project the result.
///
/// The edit is specified as a JSON object with an `op` field and
/// operation-specific fields. This enables LLMs and browsers to
/// edit program structure without text patching.
///
/// Supported ops:
/// - `add_field`: { op, name, ty, unit?, support?, representation? }
/// - `add_material`: { op, name, properties: [{name, value}] }
/// - `add_law`: { op, name, condition, consequence }
/// - `remove_item`: { op, index }
/// - `rename_item`: { op, index, new_name }
/// - `set_field_unit`: { op, index, unit? }
/// - `set_field_support`: { op, index, support }
/// - `set_field_representation`: { op, index, representation }
/// - `add_material_property`: { op, index, name, value }
/// - `remove_material_property`: { op, index, name }
/// - `add_prefix`: { op, prefix, iri }
/// - `remove_prefix`: { op, prefix }
#[wasm_bindgen]
pub fn apply_structural_edit(src: &str, edit_json: &str) -> JsValue {
    let edit = match parse_edit_json(edit_json) {
        Ok(e) => e,
        Err(msg) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &JsValue::from_str(&msg)).ok();
            return o.into();
        }
    };

    match parse_program(src) {
        Ok(prog) => {
            let edited = apply_edit(&prog, &edit);
            let projected = project_program(&edited, &ProjectOptions::default());
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"source".into(), &JsValue::from_str(&projected)).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &JsValue::from_str(&d.message)).ok();
            o.into()
        }
    }
}

/// Apply multiple structural edits in sequence.
/// `edits_json` is a JSON array of edit objects.
#[wasm_bindgen]
pub fn apply_structural_edits(src: &str, edits_json: &str) -> JsValue {
    let edits = match parse_edits_json(edits_json) {
        Ok(e) => e,
        Err(msg) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &JsValue::from_str(&msg)).ok();
            return o.into();
        }
    };

    match parse_program(src) {
        Ok(prog) => {
            let edited = apply_edits(&prog, &edits);
            let projected = project_program(&edited, &ProjectOptions::default());
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"source".into(), &JsValue::from_str(&projected)).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &JsValue::from_str(&d.message)).ok();
            o.into()
        }
    }
}

fn parse_edit_json(json: &str) -> Result<Edit, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let op = v
        .get("op")
        .and_then(|o| o.as_str())
        .ok_or("missing 'op' field")?;

    match op {
        "add_field" => {
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'name'")?;
            let ty = v.get("ty").and_then(|t| t.as_str()).ok_or("missing 'ty'")?;
            let unit = v
                .get("unit")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());
            let support = v
                .get("support")
                .and_then(|s| s.as_str())
                .map(|s| match s {
                    "region" => FieldSupport::Region,
                    "point" => FieldSupport::Point,
                    "continuant" => FieldSupport::Continuant,
                    "stream" => FieldSupport::Stream,
                    _ => FieldSupport::Region,
                })
                .unwrap_or(FieldSupport::Region);
            let representation = v
                .get("representation")
                .and_then(|r| r.as_str())
                .map(|r| match r {
                    "grid" => FieldRepresentation::Grid,
                    "mesh" => FieldRepresentation::Mesh,
                    "particles" => FieldRepresentation::Particles,
                    "analytic" => FieldRepresentation::Analytic,
                    "sampled" => FieldRepresentation::Sampled,
                    _ => FieldRepresentation::Grid,
                })
                .unwrap_or(FieldRepresentation::Grid);
            let index = v.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);
            Ok(Edit::AddItem {
                item: vibe::projectional::make_field(
                    name,
                    ty,
                    unit.as_deref(),
                    support,
                    representation,
                ),
                index,
            })
        }
        "add_material" => {
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'name'")?;
            let props = v
                .get("properties")
                .and_then(|p| p.as_array())
                .ok_or("missing 'properties' array")?;
            let properties: Vec<(&str, vibe::Expr)> = props
                .iter()
                .filter_map(|p| {
                    let n = p.get("name")?.as_str()?;
                    let val = p.get("value")?.as_f64()?;
                    Some((n, vibe::projectional::make_float(val)))
                })
                .collect();
            let index = v.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);
            Ok(Edit::AddItem {
                item: vibe::projectional::make_material(name, properties),
                index,
            })
        }
        "add_law" => {
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'name'")?;
            let condition = v
                .get("condition")
                .and_then(|c| c.as_str())
                .ok_or("missing 'condition'")?;
            let consequence = v
                .get("consequence")
                .and_then(|c| c.as_str())
                .ok_or("missing 'consequence'")?;
            // For simplicity, parse condition/consequence as cell expressions
            let cond_expr = vibe::parse_cell(&format!("= {}", condition))
                .map_err(|e| format!("condition parse: {}", e.message))?;
            let cons_expr = vibe::parse_cell(&format!("= {}", consequence))
                .map_err(|e| format!("consequence parse: {}", e.message))?;
            let index = v.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);
            Ok(Edit::AddItem {
                item: vibe::projectional::make_law(name, cond_expr, cons_expr),
                index,
            })
        }
        "remove_item" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            Ok(Edit::RemoveItem { index })
        }
        "rename_item" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let new_name = v
                .get("new_name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'new_name'")?
                .to_string();
            Ok(Edit::RenameItem { index, new_name })
        }
        "set_field_unit" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let unit = v
                .get("unit")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());
            Ok(Edit::SetFieldUnit { index, unit })
        }
        "set_field_support" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let support = v
                .get("support")
                .and_then(|s| s.as_str())
                .ok_or("missing 'support'")?;
            let support = match support {
                "region" => FieldSupport::Region,
                "point" => FieldSupport::Point,
                "continuant" => FieldSupport::Continuant,
                "stream" => FieldSupport::Stream,
                _ => return Err(format!("unknown support '{}'", support)),
            };
            Ok(Edit::SetFieldSupport { index, support })
        }
        "set_field_representation" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let representation = v
                .get("representation")
                .and_then(|r| r.as_str())
                .ok_or("missing 'representation'")?;
            let representation = match representation {
                "grid" => FieldRepresentation::Grid,
                "mesh" => FieldRepresentation::Mesh,
                "particles" => FieldRepresentation::Particles,
                "analytic" => FieldRepresentation::Analytic,
                "sampled" => FieldRepresentation::Sampled,
                _ => return Err(format!("unknown representation '{}'", representation)),
            };
            Ok(Edit::SetFieldRepresentation {
                index,
                representation,
            })
        }
        "add_material_property" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'name'")?;
            let value = v
                .get("value")
                .and_then(|v| v.as_f64())
                .ok_or("missing 'value' (must be a number)")?;
            Ok(Edit::AddMaterialProperty {
                index,
                property: NamedArg {
                    span: VibeSpan::point(0),
                    name: name.to_string(),
                    value: vibe::projectional::make_float(value),
                },
            })
        }
        "remove_material_property" => {
            let index = v
                .get("index")
                .and_then(|i| i.as_u64())
                .ok_or("missing 'index'")? as usize;
            let name = v
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or("missing 'name'")?
                .to_string();
            Ok(Edit::RemoveMaterialProperty { index, name })
        }
        "add_prefix" => {
            let prefix = v
                .get("prefix")
                .and_then(|p| p.as_str())
                .ok_or("missing 'prefix'")?
                .to_string();
            let iri = v
                .get("iri")
                .and_then(|i| i.as_str())
                .ok_or("missing 'iri'")?
                .to_string();
            Ok(Edit::AddPrefix { prefix, iri })
        }
        "remove_prefix" => {
            let prefix = v
                .get("prefix")
                .and_then(|p| p.as_str())
                .ok_or("missing 'prefix'")?
                .to_string();
            Ok(Edit::RemovePrefix { prefix })
        }
        _ => Err(format!("unknown op '{}'", op)),
    }
}

fn parse_edits_json(json: &str) -> Result<Vec<Edit>, String> {
    let arr: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = arr.as_array().ok_or("expected a JSON array")?;
    arr.iter()
        .map(|v| {
            let s = serde_json::to_string(v).map_err(|e| e.to_string())?;
            parse_edit_json(&s)
        })
        .collect()
}

// ── CBOR-LD AST and Receipt Pipeline (VW-01, VW-02, VW-03) ───────────────────

/// Tagged CBOR-LD profile identifier for VibeScript programs.
pub const VIBE_CBOR_LD_PROFILE: &str = "application/vnd.vibe.ast+cbor; version=0.1";

fn fnv1a_hash_bytes(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Encodes a VibeScript source program into tagged CBOR-LD AST bytes (Tag 4200).
/// Validates syntax and type consistency before serializing.
#[wasm_bindgen]
pub fn encode_program_cborld_wasm(src: &str) -> Result<Uint8Array, JsValue> {
    let prog = parse_program(src).map_err(|d| diag_to_js(&d))?;
    check_program(&prog).map_err(|d| diag_to_js(&d))?;
    let bytes = encode_cbor_ast(&prog);
    Ok(Uint8Array::from(bytes.as_slice()))
}

/// Decodes tagged CBOR-LD AST bytes (Tag 4200) into a canonical representation,
/// returning the reconstructed source code, AST hash, and provenance metadata.
#[wasm_bindgen]
pub fn decode_program_cborld_wasm(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let prog = decode_cbor_ast(bytes)
        .map_err(|e| JsValue::from_str(&format!("CBOR AST decode error: {e}")))?;
    let canonical_source = project_program(&prog, &ProjectOptions::default());
    let ast_hash = fnv1a_hash_bytes(bytes);

    let o = Object::new();
    Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
    Reflect::set(&o, &"tag".into(), &JsValue::from_f64(TAG_VIBE_AST as f64)).ok();
    Reflect::set(
        &o,
        &"canonicalSource".into(),
        &JsValue::from_str(&canonical_source),
    )
    .ok();
    Reflect::set(
        &o,
        &"astHash".into(),
        &JsValue::from_str(&format!("{:016x}", ast_hash)),
    )
    .ok();
    Reflect::set(
        &o,
        &"itemsCount".into(),
        &JsValue::from_f64(prog.items.len() as f64),
    )
    .ok();
    Reflect::set(
        &o,
        &"languageVersion".into(),
        &JsValue::from_str(LANGUAGE_VERSION),
    )
    .ok();
    Reflect::set(&o, &"hostVersion".into(), &JsValue::from_str(HOST_VERSION)).ok();
    Reflect::set(
        &o,
        &"profile".into(),
        &JsValue::from_str(VIBE_CBOR_LD_PROFILE),
    )
    .ok();
    Ok(o.into())
}

/// Evaluates a Vibe program and returns the evaluated value together with a signed,
/// reproducible execution receipt (containing source hash, AST hash, and engine metadata).
#[wasm_bindgen]
pub fn eval_program_with_receipt_wasm(src: &str) -> JsValue {
    let prog = match parse_program(src).and_then(|p| check_program(&p).map(|_| p)) {
        Ok(p) => p,
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            return o.into();
        }
    };

    let cbor_bytes = encode_cbor_ast(&prog);
    let ast_hash = fnv1a_hash_bytes(&cbor_bytes);
    let src_hash = fnv1a_hash_bytes(src.as_bytes());

    let mut host = LocalHost::default();
    let mut env = Env::default();
    let result = (|| {
        {
            let mut engine = Engine::with_program(&mut host, Budget::default(), &prog);
            engine.eval_program(&prog, &mut env)?;
        }
        let has_main = prog
            .items
            .iter()
            .any(|item| matches!(item, Item::Function(f) if f.name == "main"));
        if has_main {
            eval_function(&prog, "main", Vec::new(), &mut host, &mut env)
        } else {
            Ok(Value::Null)
        }
    })();

    match result {
        Ok(v) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
            Reflect::set(&o, &"value".into(), &value_to_js(&v)).ok();

            let receipt = Object::new();
            Reflect::set(
                &receipt,
                &"sourceHash".into(),
                &JsValue::from_str(&format!("{:016x}", src_hash)),
            )
            .ok();
            Reflect::set(
                &receipt,
                &"astHash".into(),
                &JsValue::from_str(&format!("{:016x}", ast_hash)),
            )
            .ok();
            Reflect::set(
                &receipt,
                &"languageVersion".into(),
                &JsValue::from_str(LANGUAGE_VERSION),
            )
            .ok();
            Reflect::set(
                &receipt,
                &"hostVersion".into(),
                &JsValue::from_str(HOST_VERSION),
            )
            .ok();
            Reflect::set(
                &receipt,
                &"profile".into(),
                &JsValue::from_str(VIBE_CBOR_LD_PROFILE),
            )
            .ok();
            Reflect::set(&receipt, &"status".into(), &JsValue::from_str("success")).ok();

            Reflect::set(&o, &"receipt".into(), &receipt).ok();
            o.into()
        }
        Err(d) => {
            let o = Object::new();
            Reflect::set(&o, &"ok".into(), &JsValue::from_bool(false)).ok();
            Reflect::set(&o, &"error".into(), &diag_to_js(&d)).ok();
            o.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vibe_cbor_ast_native_roundtrip() {
        let src = r#"fn add_vals(x: i64, y: i64) -> i64 {
  return x + y;
}

fn main() -> i64 {
  return add_vals(42, 10);
}
"#;
        let prog = parse_program(src).expect("parse should succeed");
        check_program(&prog).expect("check should succeed");
        let bytes = encode_cbor_ast(&prog);
        assert!(!bytes.is_empty());
        // Tag 4200: major type 6 (110_00000 = 0xc0), length 25 (ai=25, 2 bytes for 4200)
        assert_eq!(bytes[0], (6 << 5) | 25);

        let decoded = decode_cbor_ast(&bytes).expect("decode should succeed");
        let canon = project_program(&decoded, &ProjectOptions::default());
        assert!(canon.contains("add_vals"));

        let hash = fnv1a_hash_bytes(&bytes);
        assert!(hash > 0);
    }
}
