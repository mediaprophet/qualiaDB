//! Flat-scalar argument marshaling and convenience kernel evaluation for interactive
//! VibeScript 0.03 programs (`VS03-08`).
//!
//! Keeps hot-path marshaling zero-heap using caller-owned stack scratch buffers while
//! allowing interactive scripts to pass either slice lists (`[a, b, c]`) or flat scalar
//! arguments (`a, b, c`) to extension functions.

use crate::budget::Budget;
use crate::error::Diagnostic;
use crate::span::Span;
use crate::value::Value;
use vibescript_abi::{FuncDescriptor, ParamSpec, PrimitiveKind, VibeTrap};

/// Marshals `effective_args` into `input_scratch` and records `(start, end, kind, elems)`
/// in `param_offsets` for each parameter in `func.params`.
pub fn marshal_args_into_scratch(
    func: &FuncDescriptor,
    effective_args: &[Value],
    input_scratch: &mut [u8; 512],
    param_offsets: &mut [(usize, usize, PrimitiveKind, usize); 8],
) -> Result<(), VibeTrap> {
    if func.params.len() > param_offsets.len() {
        return Err(VibeTrap::ArityMismatch);
    }

    let mut cursor = 0usize;

    // Case 1: Exact 1-to-1 parameter count match
    if effective_args.len() == func.params.len() {
        for (idx, (spec, val)) in func.params.iter().zip(effective_args.iter()).enumerate() {
            let start = cursor;
            let elems = write_value_into_scratch(spec, val, input_scratch, &mut cursor)?;
            param_offsets[idx] = (start, cursor, spec.kind, elems);
        }
        return Ok(());
    }

    // Case 2: `project_10d_to_3d` called with 10 flat scalar coordinates (default 7D weights)
    if func.name == "project_10d_to_3d" && effective_args.len() == 10 && func.params.len() == 2 {
        let start0 = cursor;
        for val in effective_args {
            write_scalar_into_scratch(func.params[0].kind, val, input_scratch, &mut cursor)?;
        }
        param_offsets[0] = (start0, cursor, func.params[0].kind, 10);

        let start1 = cursor;
        const DEFAULT_WEIGHTS_7D: [f32; 7] = [1.0, 0.5, 0.35, 0.25, 0.2, 0.15, 0.1];
        for w in DEFAULT_WEIGHTS_7D {
            if cursor + 4 > input_scratch.len() {
                return Err(VibeTrap::SliceBoundsOverflow);
            }
            input_scratch[cursor..cursor + 4].copy_from_slice(&w.to_le_bytes());
            cursor += 4;
        }
        param_offsets[1] = (start1, cursor, func.params[1].kind, 7);
        return Ok(());
    }

    // Case 3: Single-slice parameter called with flat scalar arguments
    if func.params.len() == 1 {
        let spec = &func.params[0];
        let start = cursor;
        let mut elems = 0usize;
        for val in effective_args {
            elems += write_value_into_scratch(spec, val, input_scratch, &mut cursor)?;
        }
        if elems < spec.min_elems as usize || elems > spec.max_elems as usize {
            return Err(VibeTrap::ArityMismatch);
        }
        param_offsets[0] = (start, cursor, spec.kind, elems);
        return Ok(());
    }

    // Case 4: Multi-parameter function where flat scalar count equals sum of `min_elems`
    let total_min: usize = func.params.iter().map(|p| p.min_elems as usize).sum();
    if total_min == effective_args.len() {
        let mut arg_idx = 0usize;
        for (param_idx, spec) in func.params.iter().enumerate() {
            let start = cursor;
            let count = spec.min_elems as usize;
            for val in &effective_args[arg_idx..arg_idx + count] {
                write_scalar_into_scratch(spec.kind, val, input_scratch, &mut cursor)?;
            }
            param_offsets[param_idx] = (start, cursor, spec.kind, count);
            arg_idx += count;
        }
        return Ok(());
    }

    Err(VibeTrap::ArityMismatch)
}

pub fn write_value_into_scratch(
    spec: &ParamSpec,
    val: &Value,
    scratch: &mut [u8; 512],
    cursor: &mut usize,
) -> Result<usize, VibeTrap> {
    match val {
        Value::List(items) => {
            for item in items {
                write_scalar_into_scratch(spec.kind, item, scratch, cursor)?;
            }
            Ok(items.len())
        }
        scalar => {
            write_scalar_into_scratch(spec.kind, scalar, scratch, cursor)?;
            Ok(1)
        }
    }
}

pub fn write_scalar_into_scratch(
    kind: PrimitiveKind,
    val: &Value,
    scratch: &mut [u8; 512],
    cursor: &mut usize,
) -> Result<(), VibeTrap> {
    let need = kind.elem_size();
    if *cursor + need > scratch.len() {
        return Err(VibeTrap::SliceBoundsOverflow);
    }
    match kind {
        PrimitiveKind::F64 => {
            let n = val.as_f64().ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::F32 => {
            let n = val.as_f64().ok_or(VibeTrap::TypeMismatch)? as f32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::I64 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::U64 => {
            let n = value_to_u64(val).ok_or(VibeTrap::TypeMismatch)?;
            scratch[*cursor..*cursor + 8].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::U32 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)? as u32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::I32 => {
            let n = val.as_i64().ok_or(VibeTrap::TypeMismatch)? as i32;
            scratch[*cursor..*cursor + 4].copy_from_slice(&n.to_le_bytes());
        }
        PrimitiveKind::Bool | PrimitiveKind::U8 => {
            let b = match val {
                Value::Bool(b) => u8::from(*b),
                Value::U64(u) => *u as u8,
                Value::I64(i) => *i as u8,
                _ => return Err(VibeTrap::TypeMismatch),
            };
            scratch[*cursor] = b;
        }
        PrimitiveKind::NQuin48
        | PrimitiveKind::ResourceHandle
        | PrimitiveKind::Utf8Bytes => return Err(VibeTrap::TypeMismatch),
    }
    *cursor += need;
    Ok(())
}

fn value_to_u64(val: &Value) -> Option<u64> {
    match val {
        Value::U64(u) => Some(*u),
        Value::I64(i) if *i >= 0 => Some(*i as u64),
        Value::F64(f) if *f >= 0.0 => Some(*f as u64),
        Value::Bool(b) => Some(u64::from(*b)),
        Value::List(xs) => xs.first().and_then(value_to_u64),
        _ => None,
    }
}

/// Evaluates convenience extension helper functions used by interactive browser workbenches
/// (`wgsl_workgroup_threads`, `workgroup_occupancy`, `rms_energy`, `quant_row_bytes`,
/// `kv_cache_bytes`, `norm_opcode`, `verify_sealed_quin`, `verify_wire_parity`, etc.).
pub fn try_eval_convenience_kernel(
    path: &str,
    pos_args: &[Value],
    budget: &mut Budget,
    span: Span,
) -> Result<Option<Value>, Diagnostic> {
    let func_name = path.rsplit('.').next().unwrap_or(path);
    let val = match func_name {
        "wgsl_workgroup_threads" => {
            let wx = pos_args.first().and_then(value_to_u64).unwrap_or(1);
            let wy = pos_args.get(1).and_then(value_to_u64).unwrap_or(1);
            let wz = pos_args.get(2).and_then(value_to_u64).unwrap_or(1);
            Value::U64(wx.saturating_mul(wy).saturating_mul(wz))
        }
        "tile_schedule_valid" => {
            let tm = pos_args.first().and_then(value_to_u64).unwrap_or(16);
            let tn = pos_args.get(1).and_then(value_to_u64).unwrap_or(16);
            let tk = pos_args.get(2).and_then(value_to_u64).unwrap_or(16);
            let max_shmem = pos_args.get(3).and_then(value_to_u64).unwrap_or(32768);
            let bytes = (tm.saturating_mul(tk).saturating_add(tk.saturating_mul(tn))).saturating_mul(4);
            Value::Bool(bytes <= max_shmem)
        }
        "estimate_arithmetic_intensity" => {
            let flops = pos_args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let bytes = pos_args.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0).max(1e-9);
            Value::F64(flops / bytes)
        }
        "workgroup_occupancy" => {
            let threads = pos_args.first().and_then(|v| v.as_f64()).unwrap_or(256.0);
            let max_threads = pos_args.get(1).and_then(|v| v.as_f64()).unwrap_or(1024.0).max(1.0);
            Value::F64((threads / max_threads).clamp(0.0, 1.0))
        }
        "dispatch_grid_1d" => {
            let total = pos_args.first().and_then(value_to_u64).unwrap_or(0);
            let wg = pos_args.get(1).and_then(value_to_u64).unwrap_or(64).max(1);
            Value::U64((total + wg - 1) / wg)
        }
        "rms_energy" => {
            let mut sum_sq = 0.0f64;
            let mut count = 0usize;
            for a in pos_args {
                match a {
                    Value::List(xs) => {
                        for x in xs {
                            if let Some(f) = x.as_f64() {
                                sum_sq += f * f;
                                count += 1;
                            }
                        }
                    }
                    other => {
                        if let Some(f) = other.as_f64() {
                            sum_sq += f * f;
                            count += 1;
                        }
                    }
                }
            }
            let rms = if count > 0 {
                (sum_sq / (count as f64)).sqrt()
            } else {
                0.0
            };
            Value::F64(rms)
        }
        "zero_crossing_rate" => {
            let mut samples = [0.0f64; 64];
            let mut n = 0usize;
            for a in pos_args {
                match a {
                    Value::List(xs) => {
                        for x in xs {
                            if n < samples.len() {
                                samples[n] = x.as_f64().unwrap_or(0.0);
                                n += 1;
                            }
                        }
                    }
                    other => {
                        if n < samples.len() {
                            samples[n] = other.as_f64().unwrap_or(0.0);
                            n += 1;
                        }
                    }
                }
            }
            if n <= 1 {
                Value::F64(0.0)
            } else {
                let mut crossings = 0usize;
                for i in 1..n {
                    if (samples[i] >= 0.0) != (samples[i - 1] >= 0.0) {
                        crossings += 1;
                    }
                }
                Value::F64(crossings as f64 / ((n - 1) as f64))
            }
        }
        "fft_bin_freq" => {
            let bin = pos_args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sr = pos_args.get(1).and_then(|v| v.as_f64()).unwrap_or(48000.0);
            let n_fft = pos_args.get(2).and_then(|v| v.as_f64()).unwrap_or(1024.0).max(1.0);
            Value::F64(bin * sr / n_fft)
        }
        "db_to_linear" => {
            let db = pos_args.first().and_then(|v| v.as_f64()).unwrap_or(0.0);
            Value::F64(10.0_f64.powf(db / 20.0))
        }
        "quant_row_bytes" => {
            let cols = pos_args.first().and_then(value_to_u64).unwrap_or(0);
            let bits = pos_args.get(1).and_then(value_to_u64).unwrap_or(4);
            Value::U64((cols.saturating_mul(bits) + 7) / 8)
        }
        "kv_cache_bytes" => {
            let mut prod = 2u64;
            for a in pos_args {
                prod = prod.saturating_mul(value_to_u64(a).unwrap_or(1));
            }
            Value::U64(prod)
        }
        "norm_opcode" => {
            let pred = pos_args.first().and_then(value_to_u64).unwrap_or(0);
            Value::U64(pred & 0xFF)
        }
        "verify_sealed_quin" | "verify_wire_parity" => {
            let mut words = [0u64; 6];
            if pos_args.len() == 1 {
                if let Value::List(xs) = &pos_args[0] {
                    for (i, slot) in words.iter_mut().enumerate() {
                        *slot = xs.get(i).and_then(value_to_u64).unwrap_or(0);
                    }
                }
            } else {
                for i in 0..5 {
                    words[i] = pos_args.get(i).and_then(value_to_u64).unwrap_or(0);
                }
                if let Some(last) = pos_args.get(5) {
                    words[5] = match last {
                        Value::List(xs) if xs.len() >= 6 => value_to_u64(&xs[5]).unwrap_or(0),
                        other => value_to_u64(other).unwrap_or(0),
                    };
                }
            }
            let expected = words[0] ^ words[1] ^ words[2] ^ words[3] ^ words[4];
            Value::U64(u64::from(words[5] == expected))
        }
        _ => return Ok(None),
    };

    budget.charge(8, span)?;
    Ok(Some(val))
}
