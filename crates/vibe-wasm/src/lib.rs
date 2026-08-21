//! WASM bindings for VibeScript 0.1.
//!
//! Exposes parse, check, evaluate, and bytecode operations to JavaScript
//! via `wasm-bindgen`.  The wrapper serialises results as plain JS objects
//! (through `serde-wasm-bindgen`) so the playground can consume them
//! without a custom ABI.

use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

use poet_vibe::{
    bytecode::{self, compile, compile_expr, decode_chunk, encode_chunk, Vm},
    check_cell, check_program, eval_cell, load_program, parse_cell, parse_program, Budget, Env,
    MockHost, Value,
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
fn diag_to_js(d: &poet_vibe::Diagnostic) -> JsValue {
    let o = Object::new();
    Reflect::set(
        &o,
        &"code".into(),
        &JsValue::from_str(&format!("{:?}", d.code)),
    )
    .ok();
    Reflect::set(&o, &"message".into(), &JsValue::from_str(&d.message)).ok();
    Reflect::set(
        &o,
        &"span_start".into(),
        &JsValue::from_f64(d.span.start as f64),
    )
    .ok();
    Reflect::set(
        &o,
        &"span_end".into(),
        &JsValue::from_f64(d.span.end as f64),
    )
    .ok();
    o.into()
}

// ── public API ─────────────────────────────────────────────────────

/// Get the VibeScript language version string.
#[wasm_bindgen]
pub fn language_version() -> String {
    poet_vibe::LANGUAGE_VERSION.to_string()
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

/// Evaluate a cell expression (`= expr`) with a mock host.
#[wasm_bindgen]
pub fn eval_cell_src(src: &str) -> JsValue {
    let mut host = MockHost::default();
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
                if let poet_vibe::Item::Function(fd) = item {
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
                if let poet_vibe::Item::Function(fd) = item {
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
/// This is the main entry point for the playground.
#[wasm_bindgen]
pub fn eval_cell_json(src: &str) -> JsValue {
    eval_cell_src(src)
}

/// Compile a cell expression to bytecode and return chunk metadata.
#[wasm_bindgen]
pub fn compile_cell_bytecode(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => match compile_expr(&expr) {
            Ok(chunk) => {
                let o = Object::new();
                Reflect::set(&o, &"ok".into(), &JsValue::from_bool(true)).ok();
                Reflect::set(
                    &o,
                    &"code_size".into(),
                    &JsValue::from_f64(chunk.code.len() as f64),
                )
                .ok();
                Reflect::set(
                    &o,
                    &"constants".into(),
                    &JsValue::from_f64(chunk.constants.len() as f64),
                )
                .ok();
                Reflect::set(
                    &o,
                    &"functions".into(),
                    &JsValue::from_f64(chunk.functions.len() as f64),
                )
                .ok();
                Reflect::set(
                    &o,
                    &"top_locals".into(),
                    &JsValue::from_f64(chunk.top_locals as f64),
                )
                .ok();

                // Disassemble the code into a human-readable string.
                let disasm = disassemble(&chunk);
                Reflect::set(&o, &"disassembly".into(), &JsValue::from_str(&disasm)).ok();
                o.into()
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

/// Compile a cell to bytecode, run it on the VM, and return the result.
#[wasm_bindgen]
pub fn run_cell_bytecode(src: &str) -> JsValue {
    match parse_cell(src) {
        Ok(expr) => match compile_expr(&expr) {
            Ok(chunk) => {
                let mut host = MockHost::default();
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

                let mut host = MockHost::default();
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

/// Decode a binary bytecode chunk and run it.
#[wasm_bindgen]
pub fn decode_and_run(bytes: &[u8]) -> JsValue {
    match decode_chunk(bytes) {
        Ok(chunk) => {
            let mut host = MockHost::default();
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
                    Reflect::set(&o, &"error".into(), &JsValue::from_str(&format!("{:?}", e))).ok();
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
    }
}

/// Get the EBNF grammar string.
#[wasm_bindgen]
pub fn ebnf_grammar() -> String {
    poet_vibe::EBNF.to_string()
}

/// Get the GBNF grammar (for LLM constrained decoding).
#[wasm_bindgen]
pub fn gbnf_grammar() -> String {
    poet_vibe::GBNF.to_string()
}

/// Get the JSON schema for the AST.
#[wasm_bindgen]
pub fn ast_schema_json() -> String {
    poet_vibe::SOURCE_SCHEMA_JSON.to_string()
}

/// Get the JSON schema for diagnostics.
#[wasm_bindgen]
pub fn diagnostic_schema_json() -> String {
    poet_vibe::DIAGNOSTIC_SCHEMA_JSON.to_string()
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
